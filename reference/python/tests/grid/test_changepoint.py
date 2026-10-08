"""Tests for detect_changepoints in statespace.py.

These tests drive TDD for the automatic changepoint detection function
that replaces the hardcoded `interventions={9}` in the pipeline.

Tests:
    test_detect_no_changepoint_quiet_series
    test_detect_flags_large_jump
    test_detect_ignores_missing_obs
    test_detect_threshold_monotone
    test_weekly_update_auto_interventions_recovers_demo
"""
from __future__ import annotations

import numpy as np
import pytest

from backend.grid.statespace import (
    KalmanState,
    SSParams,
    N_STATE,
    detect_changepoints,
    kalman_step,
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _make_settled_state(player_ids: list[str], talent_val: float = 0.3,
                        seed: int = 0) -> KalmanState:
    """Return a KalmanState with settled (low-variance) priors near talent_val.

    Simulates a player whose filter has converged after several weeks of
    consistent observations so that sigma is realistically small.
    """
    state = KalmanState.init(player_ids)
    # Give each player a modest initial talent estimate
    state.mu[:, 0] = talent_val  # talent component
    # Shrink sigma to simulate post-convergence state
    n = len(player_ids)
    state.sigma = np.stack([np.diag([0.01, 0.005, 0.002])] * n)
    return state


# ---------------------------------------------------------------------------
# Test 1: No changepoint for a quiet series
# ---------------------------------------------------------------------------

def test_detect_no_changepoint_quiet_series():
    """Small innovation (obs close to prediction) should never be flagged.

    We give three players observations very close to their predicted means.
    With tight sigma and obs within ~0.5 sigma of the prediction, |z| < 3.0
    so no player should be flagged.
    """
    player_ids = ["A", "B", "C"]
    state = _make_settled_state(player_ids, talent_val=0.3)
    params = SSParams.from_position("QB")

    # Observations very close to predicted means (all ~= talent component = 0.3,
    # form + scheme_fit ~= 0; total predicted ~= 0.3)
    obs = np.array([0.30, 0.31, 0.29])
    snaps = np.array([50.0, 50.0, 50.0])

    flagged = detect_changepoints(state, obs, snaps, params)

    assert isinstance(flagged, set), "detect_changepoints must return a set"
    assert len(flagged) == 0, (
        f"Expected no flags for quiet series, got: {flagged}"
    )


# ---------------------------------------------------------------------------
# Test 2: Large jump (5σ above prediction) should be flagged
# ---------------------------------------------------------------------------

def test_detect_flags_large_jump():
    """An observation ~5σ above the prediction should be flagged.

    We construct the scenario by computing expected S from the formulas in
    kalman_step, then setting obs = predicted_mean + 5 * sqrt(S) to guarantee
    |z| = 5 > z_thresh=3.0.  The large-jump player must appear in the result.
    """
    player_ids = ["quiet", "jumper"]
    state = _make_settled_state(player_ids, talent_val=0.3)
    params = SSParams.from_position("QB")

    # Compute predicted mean for player 1 ("jumper") manually:
    # mu_pred[1] = F @ state.mu[1], total_pred = H @ mu_pred[1]
    from backend.grid.statespace import _F, _H
    F = _F(params)
    H = _H
    mu_pred = state.mu @ F.T  # shape (2, N_STATE)
    sigma_pred_0 = F @ state.sigma[0] @ F.T
    sigma_pred_0[0, 0] /= params.d_steady
    sigma_pred_0[1, 1] += params.q_form
    sigma_pred_0[2, 2] += params.q_scheme

    sigma_pred_1 = F @ state.sigma[1] @ F.T
    sigma_pred_1[0, 0] /= params.d_steady
    sigma_pred_1[1, 1] += params.q_form
    sigma_pred_1[2, 2] += params.q_scheme

    snaps = np.array([50.0, 50.0])
    R_jumper = params.r_scale / max(snaps[1], 1.0)
    S_jumper = float(H @ sigma_pred_1 @ H) + R_jumper
    total_pred_jumper = float(H @ mu_pred[1])

    # obs_jumper = predicted + 5 * sqrt(S) → z = 5.0 > 3.0
    obs_jumper = total_pred_jumper + 5.0 * np.sqrt(S_jumper)

    # "quiet" player: obs right at predicted mean → z ≈ 0
    total_pred_quiet = float(H @ mu_pred[0])
    obs = np.array([total_pred_quiet, obs_jumper])

    flagged = detect_changepoints(state, obs, snaps, params)

    assert "jumper" in flagged, (
        f"'jumper' with 5σ deviation should be flagged, got: {flagged}"
    )
    assert "quiet" not in flagged, (
        f"'quiet' with obs at predicted mean should not be flagged, got: {flagged}"
    )


# ---------------------------------------------------------------------------
# Test 3: NaN observations are never flagged
# ---------------------------------------------------------------------------

def test_detect_ignores_missing_obs():
    """NaN observations (player did not play) must never be flagged.

    Even with a very low z_thresh, NaN obs should produce no flag.
    """
    player_ids = ["absent", "present"]
    state = _make_settled_state(player_ids, talent_val=0.3)
    params = SSParams()

    # Player "absent" has NaN observation — definitely should not be flagged
    # Player "present" has a realistic observation
    obs = np.array([np.nan, 0.3])
    snaps = np.array([0.0, 50.0])

    # Use z_thresh=0.01 to flag anything — the NaN player must still not appear
    flagged = detect_changepoints(state, obs, snaps, params, z_thresh=0.01)

    assert "absent" not in flagged, (
        f"NaN observation should never be flagged, got: {flagged}"
    )


# ---------------------------------------------------------------------------
# Test 4: Monotone threshold (higher z_thresh → fewer or equal flags)
# ---------------------------------------------------------------------------

def test_detect_threshold_monotone():
    """Raising z_thresh can only remove flags, never add new ones.

    With a moderately large deviation (2.5σ), z_thresh=2.0 should flag
    the player but z_thresh=3.0 should not.  Additionally verify that for
    any pair z1 < z2: flagged(z2) ⊆ flagged(z1).
    """
    player_ids = ["P0", "P1", "P2"]
    state = _make_settled_state(player_ids, talent_val=0.0)
    params = SSParams()

    from backend.grid.statespace import _F, _H
    F = _F(params)
    H = _H
    mu_pred = state.mu @ F.T

    snaps = np.array([50.0, 50.0, 50.0])
    # Compute S for P0 to plant a 2.5σ deviation
    sigma_pred_0 = F @ state.sigma[0] @ F.T
    sigma_pred_0[0, 0] /= params.d_steady
    sigma_pred_0[1, 1] += params.q_form
    sigma_pred_0[2, 2] += params.q_scheme
    R0 = params.r_scale / max(snaps[0], 1.0)
    S0 = float(H @ sigma_pred_0 @ H) + R0
    pred0 = float(H @ mu_pred[0])
    obs_2_5sigma = pred0 + 2.5 * np.sqrt(S0)  # z = 2.5

    obs = np.array([obs_2_5sigma, pred0, pred0])  # P0 has 2.5σ, others near 0

    flagged_low = detect_changepoints(state, obs, snaps, params, z_thresh=2.0)
    flagged_high = detect_changepoints(state, obs, snaps, params, z_thresh=3.0)

    # Higher threshold flags subset
    assert flagged_high.issubset(flagged_low), (
        f"Higher threshold should flag fewer/equal players: "
        f"z=2.0→{flagged_low}, z=3.0→{flagged_high}"
    )
    # 2.5σ > z_thresh=2.0 → should be flagged at low threshold
    assert "P0" in flagged_low, (
        f"P0 with 2.5σ deviation should be flagged at z_thresh=2.0, got {flagged_low}"
    )
    # 2.5σ < z_thresh=3.0 → should NOT be flagged at high threshold
    assert "P0" not in flagged_high, (
        f"P0 with 2.5σ deviation should NOT be flagged at z_thresh=3.0, got {flagged_high}"
    )


# ---------------------------------------------------------------------------
# Test 5: Integration — auto-interventions in the pipeline recovers demo
# ---------------------------------------------------------------------------

def test_weekly_update_auto_interventions_recovers_demo():
    """The pipeline auto-flags the injury-return week without hardcoded {9}.

    We build a synthetic focus-QB whose state has been trained on stable
    (small) values for weeks 1-8, then is absent week 9, and returns week 10
    with a large positive obs (talent recovered).  The auto-detection on
    week 10 should flag the returning QB without any manual intervention arg.

    This test calls kalman_step directly (not the full weekly_update pipeline)
    to keep it fast and self-contained, and verifies that detect_changepoints
    flags the returning player.
    """
    # Build a focused player that has been stable for 8 weeks then absent week 9
    player_ids = ["qb_focus"]
    stable_obs = 0.3
    params = SSParams.from_position("QB")

    # Warm up the filter with 8 stable weeks
    state = KalmanState.init(player_ids)
    rng = np.random.default_rng(42)
    for _ in range(8):
        obs_w = np.array([stable_obs + rng.normal(0, 0.02)])
        snaps_w = np.array([60.0])
        state = kalman_step(state, obs_w, snaps_w, params)

    # Week 9: absent (NaN) — predict-only, variance grows
    obs_absent = np.array([np.nan])
    snaps_absent = np.array([0.0])
    state = kalman_step(state, obs_absent, snaps_absent, params)

    # Week 10: return — player comes back with a large value (injury recovery)
    # We need an obs well above the predicted mean (which has regressed toward 0
    # during absence) — use a large value to guarantee z > 3.0
    from backend.grid.statespace import _F, _H
    F = _F(params)
    H = _H
    mu_pred_return = state.mu @ F.T  # predicted state for week 10
    sigma_pred_return = F @ state.sigma[0] @ F.T
    sigma_pred_return[0, 0] /= params.d_steady
    sigma_pred_return[1, 1] += params.q_form
    sigma_pred_return[2, 2] += params.q_scheme
    snaps_return = np.array([60.0])
    R_return = params.r_scale / max(snaps_return[0], 1.0)
    S_return = float(H @ sigma_pred_return @ H) + R_return
    total_pred_return = float(H @ mu_pred_return[0])

    # Force obs 6σ above prediction — well above z_thresh=3.0
    obs_return = np.array([total_pred_return + 6.0 * np.sqrt(S_return)])

    flagged = detect_changepoints(state, obs_return, snaps_return, params, z_thresh=3.0)

    assert "qb_focus" in flagged, (
        f"Returning QB with large jump should be auto-flagged as changepoint. "
        f"total_pred={total_pred_return:.4f}, obs={obs_return[0]:.4f}, "
        f"sqrt(S)={np.sqrt(S_return):.4f}, z={(obs_return[0]-total_pred_return)/np.sqrt(S_return):.2f}, "
        f"flagged={flagged}"
    )
