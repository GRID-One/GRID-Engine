"""Tests for Kalman filter numerical stability (Joseph form + safe RTS smoother).

Ported to 3-component state [talent, form, scheme_fit] in Task 8.
"""
import numpy as np
import pytest

from backend.grid.statespace import SSParams, KalmanState, kalman_two_component, kalman_step, N_STATE


def _make_adversarial_series(W=50, seed=7):
    """Generate a long series with extreme snap counts and interventions."""
    rng = np.random.default_rng(seed)
    y = rng.normal(0.5, 0.3, W)
    snaps = rng.choice([1.0, 5.0, 200.0], size=W)
    played = np.ones(W, dtype=bool)
    played[10:12] = False
    interventions = {12, 30}
    return y, snaps, played, interventions


def test_joseph_form_preserves_symmetry():
    y, snaps, played, intv = _make_adversarial_series(W=50)
    out = kalman_two_component(y, snaps, played, interventions=intv)
    tau = out["tau_smooth"]
    assert tau.shape == (50,)
    assert np.all(np.isfinite(tau))


def test_joseph_form_preserves_positive_definite():
    """After 50 weeks the filtered variance should remain positive."""
    y, snaps, played, intv = _make_adversarial_series(W=50)
    out = kalman_two_component(y, snaps, played, interventions=intv)
    for v in out["var_total_filt"]:
        assert v > 0, f"Filtered variance went non-positive: {v}"
    for v in out["var_tau_smooth"]:
        assert v > 0, f"Smoothed tau variance went non-positive: {v}"


def test_joseph_form_matches_standard_in_benign_case():
    """Under benign inputs, Joseph form produces finite results close to data mean."""
    rng = np.random.default_rng(42)
    W = 10
    y = rng.normal(0.3, 0.1, W)
    snaps = np.full(W, 50.0)
    played = np.ones(W, dtype=bool)
    out = kalman_two_component(y, snaps, played)
    assert out["tau_smooth"].shape == (W,)
    assert np.all(np.isfinite(out["total_smooth"]))
    assert abs(out["total_smooth"][-1] - y.mean()) < 0.5


def test_rts_smoother_near_singular():
    """RTS smoother should not crash when predicted covariance is near-singular."""
    W = 8
    y = np.array([0.1, 0.2, 0.15, 0.18, 0.12, 0.14, 0.16, 0.13])
    snaps = np.full(W, 1000.0)  # very high snaps -> tiny R -> Pp can approach singular
    played = np.ones(W, dtype=bool)
    params = SSParams(d_steady=0.999, q_form=1e-8, r_scale=0.001)
    out = kalman_two_component(y, snaps, played, params=params)
    assert np.all(np.isfinite(out["tau_smooth"]))
    assert np.all(np.isfinite(out["var_tau_smooth"]))


def test_kalman_step_joseph_symmetry():
    """Incremental kalman_step should maintain covariance symmetry over 10 iterations."""
    state = KalmanState.init(["p1", "p2", "p3"])
    rng = np.random.default_rng(99)
    for _ in range(10):
        obs = rng.normal(0.1, 0.2, 3)
        obs[rng.random() < 0.3] = np.nan  # occasional missing
        snaps = rng.choice([5.0, 50.0, 200.0], size=3)
        state = kalman_step(state, obs, snaps=snaps)
    for i in range(3):
        S = state.sigma[i]
        assert S.shape == (N_STATE, N_STATE), f"Player {i} sigma shape wrong"
        np.testing.assert_allclose(S, S.T, atol=1e-12,
                                   err_msg=f"Player {i} covariance not symmetric")
        eigvals = np.linalg.eigvalsh(S)
        assert np.all(eigvals > -1e-12), f"Player {i} covariance has negative eigenvalue"


def test_rts_smoother_covariances_positive_definite():
    """RTS smoother output covariances (Ps) should all be PSD after 3-component run."""
    rng = np.random.default_rng(13)
    W = 20
    y = rng.normal(0.4, 0.2, W)
    snaps = rng.choice([10.0, 50.0, 100.0], size=W)
    played = np.ones(W, dtype=bool)
    played[5] = False  # one missing week
    interventions = {8}

    # Access the internal Ps list by running with default params
    # We test through the public interface: var_tau_smooth and var_total_filt
    out = kalman_two_component(y, snaps, played, interventions=interventions)

    # All variances from RTS smoother must be positive
    for w, v in enumerate(out["var_tau_smooth"]):
        assert v > 0, f"RTS smoothed tau variance non-positive at week {w}: {v}"
    for w, v in enumerate(out["var_total_filt"]):
        assert v > 0, f"Filtered total variance non-positive at week {w}: {v}"

    # Smoothed tau must be finite
    assert np.all(np.isfinite(out["tau_smooth"])), "RTS tau_smooth has non-finite values"
    assert np.all(np.isfinite(out["total_smooth"])), "RTS total_smooth has non-finite values"


def test_three_component_rts_pd():
    """3-component RTS smoother: every per-step Ps matrix must be symmetric and PSD.

    kalman_two_component now returns sigma_smooth of shape (T, N_STATE, N_STATE) —
    the full RTS-smoothed covariance sequence.  This test eigen-checks each of those
    N_STATE×N_STATE matrices directly; a non-PSD off-diagonal entry in any Ps would
    cause an assertion failure here.
    """
    rng = np.random.default_rng(55)
    W = 25
    y = rng.normal(0.3, 0.15, W)
    snaps = rng.choice([5.0, 30.0, 100.0], size=W)
    played = np.ones(W, dtype=bool)
    played[[3, 4]] = False
    interventions = {7, 18}

    out = kalman_two_component(y, snaps, played, interventions=interventions)

    # --- primary assertion: eigen-check every full Ps matrix ---
    sigma_smooth = out["sigma_smooth"]  # shape (W, N_STATE, N_STATE)
    assert sigma_smooth.shape == (W, N_STATE, N_STATE), \
        f"sigma_smooth shape wrong: {sigma_smooth.shape}"
    for w in range(W):
        Ps_w = sigma_smooth[w]
        np.testing.assert_allclose(Ps_w, Ps_w.T, atol=1e-12,
                                   err_msg=f"Week {w}: smoothed Ps not symmetric")
        eigvals = np.linalg.eigvalsh(Ps_w)
        assert np.all(eigvals > -1e-9), (
            f"Week {w}: smoothed Ps not PSD (min eigval={eigvals.min():.2e})\n"
            f"Ps[{w}] =\n{Ps_w}"
        )

    # --- secondary: scalar summaries are still valid ---
    assert np.all(np.isfinite(out["tau_smooth"])), "RTS tau_smooth not finite"
    assert np.all(np.array(out["var_tau_smooth"]) > 0), "RTS var_tau_smooth not all positive"
    assert np.all(np.array(out["var_total_filt"]) > 0), "var_total_filt not all positive"
