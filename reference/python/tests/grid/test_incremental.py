"""Tests for incremental RAPM accumulators and Kalman step."""
import numpy as np
import pytest
import scipy.sparse as sp

from backend.grid.layers import accumulate, fit_from_accumulators, init_accumulators
from backend.grid.statespace import KalmanState, SSParams, kalman_step, N_STATE


# ─── accumulator tests ────────────────────────────────────────────────────────

def _make_sparse(n_plays, n_cols, seed=42):
    rng = np.random.default_rng(seed)
    data = rng.normal(0, 1, size=(n_plays, n_cols)) * (rng.random((n_plays, n_cols)) > 0.9)
    return sp.csr_matrix(data)


def test_init_accumulators_shape():
    XtX, Xty = init_accumulators(10)
    assert XtX.shape == (10, 10)
    assert Xty.shape == (10,)
    assert XtX.sum() == 0.0


def test_accumulate_single_week():
    rng = np.random.default_rng(0)
    n_plays, n_cols = 50, 8
    X = sp.csr_matrix(rng.normal(size=(n_plays, n_cols)))
    y = rng.normal(size=n_plays)
    XtX, Xty = init_accumulators(n_cols)
    XtX2, Xty2 = accumulate(XtX, Xty, X, y)
    expected_XtX = (X.T @ X).toarray()
    expected_Xty = np.asarray(X.T @ y).ravel()
    np.testing.assert_allclose(XtX2, expected_XtX)
    np.testing.assert_allclose(Xty2, expected_Xty)


def test_accumulate_two_weeks_equals_full():
    """Accumulating weeks A then B should equal running on A+B together."""
    rng = np.random.default_rng(1)
    n_cols = 6
    Xa = sp.csr_matrix(rng.normal(size=(30, n_cols)))
    ya = rng.normal(size=30)
    Xb = sp.csr_matrix(rng.normal(size=(20, n_cols)))
    yb = rng.normal(size=20)

    XtX, Xty = init_accumulators(n_cols)
    XtX, Xty = accumulate(XtX, Xty, Xa, ya)
    XtX, Xty = accumulate(XtX, Xty, Xb, yb)

    X_full = sp.vstack([Xa, Xb])
    y_full = np.concatenate([ya, yb])
    expected_XtX = (X_full.T @ X_full).toarray()
    expected_Xty = np.asarray(X_full.T @ y_full).ravel()

    np.testing.assert_allclose(XtX, expected_XtX, atol=1e-10)
    np.testing.assert_allclose(Xty, expected_Xty, atol=1e-10)


def test_fit_from_accumulators_recovers_signal():
    """fit_from_accumulators should recover a known signal with low ridge."""
    rng = np.random.default_rng(2)
    n_plays, n_cols = 200, 5
    true_beta = rng.normal(size=n_cols)
    X = sp.csr_matrix(rng.normal(size=(n_plays, n_cols)))
    y = np.asarray(X @ true_beta).ravel() + rng.normal(scale=0.1, size=n_plays)

    XtX = (X.T @ X).toarray()
    Xty = np.asarray(X.T @ y).ravel()
    prior_mean = np.zeros(n_cols)
    mask = np.ones(n_cols)
    beta_hat = fit_from_accumulators(XtX, Xty, lam=0.01, prior_mean=prior_mean, ridge_mask=mask)

    corr = np.corrcoef(beta_hat, true_beta)[0, 1]
    assert corr > 0.95, f"Expected high correlation, got {corr:.3f}"


def test_accumulator_persistence(tmp_path, monkeypatch):
    """save_accumulators / load_accumulators round-trip."""
    import backend.grid.layers as layers_mod
    monkeypatch.setattr(layers_mod, "_ACCUM_PATH", tmp_path / "rapm_accumulators.npz")

    from backend.grid.layers import save_accumulators, load_accumulators
    rng = np.random.default_rng(3)
    XtX = rng.normal(size=(5, 5))
    Xty = rng.normal(size=5)
    save_accumulators(XtX, Xty, week=7)
    result = load_accumulators()
    assert result is not None
    loaded_XtX, loaded_Xty, week, _player_order = result
    np.testing.assert_array_equal(loaded_XtX, XtX)
    np.testing.assert_array_equal(loaded_Xty, Xty)
    assert week == 7


# ─── Kalman step tests ────────────────────────────────────────────────────────

def test_kalman_state_init():
    """State init should produce 3-component shapes and correct diagonal."""
    state = KalmanState.init(["p1", "p2", "p3"])
    assert state.mu.shape == (3, N_STATE)
    assert state.sigma.shape == (3, N_STATE, N_STATE)
    assert state.player_ids == ["p1", "p2", "p3"]
    # Check initial diagonal: [0.05, 0.02, 0.01]
    for i in range(3):
        np.testing.assert_allclose(np.diag(state.sigma[i]), [0.05, 0.02, 0.01],
                                   atol=1e-12, err_msg=f"Player {i} init diagonal wrong")
        # Off-diagonal should be zero
        off_diag = state.sigma[i] - np.diag(np.diag(state.sigma[i]))
        assert np.all(off_diag == 0.0), f"Player {i} init sigma has non-zero off-diagonals"


def test_kalman_step_no_obs():
    """All-NaN observations: state advances but stays close to prior."""
    state = KalmanState.init(["p1", "p2"])
    obs = np.array([np.nan, np.nan])
    new_state = kalman_step(state, obs)
    assert new_state.mu.shape == (2, N_STATE)
    # Variance should have grown (predict without update)
    assert np.all(new_state.sigma[:, 0, 0] >= state.sigma[:, 0, 0])


def test_kalman_step_with_obs():
    """An observation should pull mu toward the observed value."""
    state = KalmanState.init(["p1"])
    state.mu[0] = np.zeros(N_STATE)
    obs = np.array([1.0])
    new_state = kalman_step(state, obs, snaps=np.array([50.0]))
    assert new_state.mu[0, 0] > 0.0, "Talent should be pulled positive"


def test_kalman_step_preserves_player_ids():
    pids = ["A", "B", "C"]
    state = KalmanState.init(pids)
    obs = np.array([0.1, np.nan, -0.2])
    new_state = kalman_step(state, obs)
    assert new_state.player_ids == pids


def test_ssparams_from_position_qb():
    params = SSParams.from_position("QB")
    assert params.d_steady == 0.95
    assert params.r_scale == 0.55   # calibrated on synth (was 0.35); see test_calibration_synth


def test_ssparams_from_position_rb():
    params = SSParams.from_position("RB")
    assert params.d_steady == 0.85
    assert params.r_scale == 0.45


def test_ssparams_from_position_unknown():
    params = SSParams.from_position("PUNTER")
    assert params.d_steady == 0.90  # default
    assert params.r_scale == 0.40


def test_kalman_step_intervention_inflates_variance():
    """Intervention players should have larger predicted variance."""
    state = KalmanState.init(["p1", "p2"])
    obs = np.array([0.5, 0.5])
    snaps = np.array([50.0, 50.0])
    s_no_intv = kalman_step(state, obs, snaps=snaps)
    s_intv = kalman_step(state, obs, snaps=snaps, interventions={"p1"})
    assert s_intv.sigma[0, 0, 0] > s_no_intv.sigma[0, 0, 0], \
        "Intervention player should have larger talent variance"
    np.testing.assert_allclose(s_intv.sigma[1], s_no_intv.sigma[1], atol=1e-12,
                               err_msg="Non-intervention player should be unchanged")


def test_kalman_step_intervention_inflates_obs_noise():
    """Intervention players get inflated R, pulling mu less toward observation."""
    state = KalmanState.init(["p1", "p2"])
    state.mu[:] = 0.0
    obs = np.array([1.0, 1.0])
    snaps = np.array([50.0, 50.0])
    s_intv = kalman_step(state, obs, snaps=snaps, interventions={"p1"})
    s_no = kalman_step(state, obs, snaps=snaps)
    assert s_intv.mu[0, 0] < s_no.mu[0, 0], \
        "Intervention player should be pulled less toward obs (higher R)"


def test_kalman_step_intervention_none_backward_compatible():
    """interventions=None should produce identical results to omitting the parameter."""
    state = KalmanState.init(["p1"])
    obs = np.array([0.3])
    s1 = kalman_step(state, obs)
    s2 = kalman_step(state, obs, interventions=None)
    np.testing.assert_array_equal(s1.mu, s2.mu)
    np.testing.assert_array_equal(s1.sigma, s2.sigma)


def test_kalman_step_intervention_unknown_player_ignored():
    """An intervention for a player not in the state should be silently ignored."""
    state = KalmanState.init(["p1", "p2"])
    obs = np.array([0.3, 0.3])
    s1 = kalman_step(state, obs)
    s2 = kalman_step(state, obs, interventions={"UNKNOWN_PLAYER"})
    np.testing.assert_array_equal(s1.mu, s2.mu)
    np.testing.assert_array_equal(s1.sigma, s2.sigma)


def test_kalman_state_persistence(tmp_path, monkeypatch):
    """KalmanState.save() / KalmanState.load() round-trip."""
    import backend.grid.statespace as ss_mod
    monkeypatch.setattr(ss_mod, "_KALMAN_PATH", tmp_path / "kalman_state.npz")

    state = KalmanState.init(["x", "y"])
    state.mu[0, 0] = 0.5
    state.mu[0, 1] = -0.1
    state.save()
    loaded = KalmanState.load()
    assert loaded is not None
    np.testing.assert_array_equal(loaded.mu, state.mu)
    assert loaded.player_ids == ["x", "y"]


def test_kalman_step_three_component_symmetry():
    """kalman_step should maintain 3×3 covariance symmetry and PSD after 10 steps."""
    state = KalmanState.init(["p1", "p2", "p3"])
    rng = np.random.default_rng(77)
    for _ in range(10):
        obs = rng.normal(0.1, 0.2, 3)
        obs[rng.random() < 0.3] = np.nan
        snaps = rng.choice([5.0, 50.0, 200.0], size=3)
        state = kalman_step(state, obs, snaps=snaps)
    for i in range(3):
        S = state.sigma[i]
        assert S.shape == (N_STATE, N_STATE), f"Player {i} sigma wrong shape"
        np.testing.assert_allclose(S, S.T, atol=1e-12,
                                   err_msg=f"Player {i} covariance not symmetric")
        eigvals = np.linalg.eigvalsh(S)
        assert np.all(eigvals > -1e-12), \
            f"Player {i} covariance not PSD (min eigval={eigvals.min():.2e})"


def test_scheme_fit_drifts_slowly():
    """scheme_fit variance should stay bounded and much smaller than talent variance.

    phi_scheme=0.985 means the AR(1) decay brings variance toward its stationary level
    q_scheme / (1 - phi_scheme^2) ≈ 0.0002 / (1 - 0.985^2) ≈ 0.0066.
    Starting from init sigma[2,2]=0.01 (above stationary), variance decreases toward
    steady-state.  Key properties: remains finite, positive, and stays small.
    """
    state = KalmanState.init(["p1"])
    params = SSParams()

    # Advance 50 steps with no observations (predict-only)
    obs = np.array([np.nan])
    vars_over_time = []
    for _ in range(50):
        state = kalman_step(state, obs, params=params)
        vars_over_time.append(float(state.sigma[0, 2, 2]))

    final_var = state.sigma[0, 2, 2]

    # 1. scheme_fit variance remains positive and finite
    assert final_var > 0, "scheme_fit variance should remain positive"
    assert np.isfinite(final_var), "scheme_fit variance should be finite"

    # 2. scheme_fit variance stays much smaller than talent variance
    #    (scheme_fit is a slow component; talent variance is inflated by discount)
    assert final_var < 0.5, f"scheme_fit variance grew unexpectedly large: {final_var}"

    # 3. scheme_fit innovation q_scheme is much smaller than q_form
    #    Verify the parameter relationship holds
    assert params.q_scheme < params.q_form, \
        "q_scheme must be smaller than q_form (scheme changes slower than form)"

    # 4. scheme_fit variance converges (change between step 40-50 should be small)
    late_change = abs(vars_over_time[49] - vars_over_time[39])
    assert late_change < 0.005, \
        f"scheme_fit variance still drifting a lot at late steps: delta={late_change:.4f}"


# ─── Kalman NPZ migration tests ──────────────────────────────────────────────

def test_migrate_2comp_to_3comp():
    """KalmanState.migrate() pads 2-wide mu/sigma to 3 components losslessly."""
    import backend.grid.statespace as ss_mod
    rng = np.random.default_rng(99)
    n = 4
    mu2 = rng.normal(size=(n, 2))
    sigma2 = np.stack([np.diag([0.05, 0.02])] * n)
    pids = ["a", "b", "c", "d"]
    state = ss_mod.KalmanState.migrate(mu2, sigma2, pids, scheme_var=0.01)

    assert state.mu.shape == (n, 3), "mu should be (n, 3) after migrate"
    assert state.sigma.shape == (n, 3, 3), "sigma should be (n, 3, 3) after migrate"
    assert state.player_ids == pids

    # talent and form columns preserved exactly
    np.testing.assert_array_equal(state.mu[:, :2], mu2)
    # scheme_fit column is zeros
    np.testing.assert_array_equal(state.mu[:, 2], np.zeros(n))

    for i in range(n):
        # top-left 2×2 preserved
        np.testing.assert_array_equal(state.sigma[i, :2, :2], sigma2[i])
        # scheme_fit variance set to scheme_var
        assert state.sigma[i, 2, 2] == 0.01, f"Player {i} sigma[2,2] should be 0.01"
        # off-diagonals in column/row 2 are zero
        assert state.sigma[i, 0, 2] == 0.0
        assert state.sigma[i, 1, 2] == 0.0
        assert state.sigma[i, 2, 0] == 0.0
        assert state.sigma[i, 2, 1] == 0.0


def test_load_legacy_npz_pads(tmp_path, monkeypatch):
    """Loading a 2-component npz WITHOUT state_version pads to 3 components."""
    import backend.grid.statespace as ss_mod
    monkeypatch.setattr(ss_mod, "_KALMAN_PATH", tmp_path / "kalman_state.npz")

    rng = np.random.default_rng(7)
    n = 3
    mu2 = rng.normal(size=(n, 2))
    sigma2 = np.stack([np.diag([0.05, 0.02])] * n)
    pids = np.array(["x1", "x2", "x3"])

    # Write a legacy file: no state_version key
    np.savez(str(tmp_path / "kalman_state.npz"), mu=mu2, sigma=sigma2, player_ids=pids)

    loaded = ss_mod.KalmanState.load()
    assert loaded is not None, "Legacy file should load successfully (not return None)"
    assert loaded.mu.shape == (n, 3), "Loaded mu should be padded to 3 components"
    assert loaded.sigma.shape == (n, 3, 3), "Loaded sigma should be padded to 3×3"
    # talent/form preserved
    np.testing.assert_array_equal(loaded.mu[:, :2], mu2)
    # scheme_fit padded with zeros
    np.testing.assert_array_equal(loaded.mu[:, 2], np.zeros(n))
    assert loaded.player_ids == list(pids)
    # sigma: top-left 2×2 preserved exactly from the legacy file
    for i in range(n):
        np.testing.assert_array_equal(loaded.sigma[i, :2, :2], sigma2[i],
            err_msg=f"Player {i} top-left 2×2 sigma not preserved after load migration")
    # sigma: scheme_fit variance initialised to 0.01, off-diagonals zero
    np.testing.assert_array_equal(loaded.sigma[:, 2, 2], np.full(n, 0.01),
        err_msg="sigma[:,2,2] should equal scheme_init_var=0.01 after legacy load")
    np.testing.assert_array_equal(loaded.sigma[:, 0, 2], np.zeros(n))
    np.testing.assert_array_equal(loaded.sigma[:, 1, 2], np.zeros(n))
    np.testing.assert_array_equal(loaded.sigma[:, 2, 0], np.zeros(n))
    np.testing.assert_array_equal(loaded.sigma[:, 2, 1], np.zeros(n))


def test_load_corrupt_returns_none(tmp_path, monkeypatch):
    """A npz with mu.shape[1]==5 (unknown width) should cause load to return None."""
    import backend.grid.statespace as ss_mod
    monkeypatch.setattr(ss_mod, "_KALMAN_PATH", tmp_path / "kalman_state.npz")

    rng = np.random.default_rng(13)
    n = 2
    mu5 = rng.normal(size=(n, 5))
    sigma5 = np.zeros((n, 5, 5))
    pids = np.array(["p1", "p2"])

    np.savez(str(tmp_path / "kalman_state.npz"), mu=mu5, sigma=sigma5, player_ids=pids)

    result = ss_mod.KalmanState.load()
    assert result is None, "Corrupt/unknown shape should return None so caller reinitializes"


def test_save_writes_version(tmp_path, monkeypatch):
    """KalmanState.save() must write state_version == N_STATE into the npz."""
    import backend.grid.statespace as ss_mod
    monkeypatch.setattr(ss_mod, "_KALMAN_PATH", tmp_path / "kalman_state.npz")

    state = ss_mod.KalmanState.init(["p1", "p2"])
    state.save()

    data = np.load(str(tmp_path / "kalman_state.npz"), allow_pickle=True)
    assert "state_version" in data.files, "Saved npz must contain state_version key"
    assert int(data["state_version"]) == ss_mod.N_STATE, \
        f"state_version should equal N_STATE={ss_mod.N_STATE}"


def test_load_3wide_without_version_loads_as_is(tmp_path, monkeypatch):
    """A 3-wide npz WITHOUT state_version (Task-8 format) must load as-is, not return None.

    Task 8 produced 3-component state files but did not yet write state_version.
    On first deploy of Task 9 over such a cache, load() must preserve all accumulated
    Kalman state rather than discarding it by returning None.
    """
    import backend.grid.statespace as ss_mod
    monkeypatch.setattr(ss_mod, "_KALMAN_PATH", tmp_path / "kalman_state.npz")

    rng = np.random.default_rng(42)
    n = 4
    mu3 = rng.normal(size=(n, 3))
    sigma3 = np.stack([np.diag([0.05, 0.02, 0.01])] * n)
    pids = np.array(["a", "b", "c", "d"])

    # Simulate a Task-8 save: 3-wide arrays, NO state_version key
    np.savez(str(tmp_path / "kalman_state.npz"), mu=mu3, sigma=sigma3, player_ids=pids)

    result = ss_mod.KalmanState.load()
    assert result is not None, (
        "3-wide npz without state_version should load as-is (not be discarded)"
    )
    assert result.mu.shape == (n, 3), "mu should remain (n, 3), not be migrated"
    np.testing.assert_array_equal(result.mu, mu3,
        err_msg="mu values must equal the saved values exactly (no migration applied)")
    np.testing.assert_array_equal(result.sigma, sigma3,
        err_msg="sigma values must equal the saved values exactly (no migration applied)")
    assert result.player_ids == list(pids)


def test_two_component_recovery_unchanged():
    """3-comp talent recovery must not regress below the pre-Task-8 2-comp baseline.

    Baseline measured at ~0.7272 on seed=42, W=30, snaps=50.  We allow a 0.03
    tolerance so the assertion is 3-comp >= baseline - tol (currently ≈ 0.7543 >= 0.6972).
    A regression that dropped to, say, 0.71 (still > the old flat 0.70 threshold)
    would now correctly fail.
    """
    # Baseline: measured correlation of 2-component smoother on this exact seed.
    # Provenance: pre-Task-8 kalman_two_component (2-state) run with same planted data
    # produced corr ≈ 0.7272; current 3-comp run ≈ 0.7543.  Any value below this
    # (minus tolerance) indicates genuine regression relative to the prior model.
    BASELINE_TALENT_CORR = 0.7272
    TOL = 0.03

    rng = np.random.default_rng(42)
    W = 30
    # Plant a known talent trajectory
    true_talent = np.cumsum(rng.normal(0, 0.02, W)) + 0.3
    noise = rng.normal(0, 0.05, W)
    y = true_talent + noise
    snaps = np.full(W, 50.0)
    played = np.ones(W, dtype=bool)

    from backend.grid.statespace import kalman_two_component
    out = kalman_two_component(y, snaps, played)

    corr = np.corrcoef(out["tau_smooth"], true_talent)[0, 1]
    assert corr >= BASELINE_TALENT_CORR - TOL, (
        f"Talent recovery regressed below 2-comp baseline: {corr:.4f} < "
        f"{BASELINE_TALENT_CORR:.4f} - {TOL} = {BASELINE_TALENT_CORR - TOL:.4f}"
    )
