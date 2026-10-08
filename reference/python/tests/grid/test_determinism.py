"""Tier 0.5 prerequisite — in-process determinism gate (roadmap Phase 0).

The golden master and every recovery threshold rest on the engine being
deterministic: the same synthetic input must produce bit-identical estimates
within a single process.  This test runs synth -> value -> RAPM fixed point
TWICE in the same process and asserts the outputs match < 1e-9.

This is an IN-PROCESS gate only.  It is NOT a cross-platform guarantee: floats
diverge ~1e-6 across OS/BLAS, which is why the (future) cross-platform golden
master leans on sign/label/ordering invariants plus loose numeric tolerances
rather than this bound.  Determinism hazards the engine already pins:
  * HistGradientBoostingRegressor random_state (value.py, layers.py Layer-1),
  * KFold(shuffle=True, random_state=...) in layer1_qb_weekly,
  * canonical sorted player/team column order in build_design,
  * all synth RNGs seeded (synth.py), market noise seeded in this harness.
Run with OMP_NUM_THREADS=1 to remove BLAS reduction-order jitter.
"""
import numpy as np
import pandas as pd
import pytest

from backend.grid import (
    load_synthetic, fit_value_model, attach_dv, fit, kalman_step, KalmanState,
)


def _run_pipeline():
    """Run synth -> V(s) -> dV -> RAPM fixed point once; return the estimates we
    pin for determinism (value-model dV, player ratings, team ratings, weekly
    QB credit), in canonical row order."""
    b = load_synthetic()
    plays, players, gt = b["plays"], b["players"], b["gt"]
    vm = fit_value_model(plays)
    plays = attach_dv(plays, vm)
    rng = np.random.default_rng(1)
    market = {t: v + rng.normal(0, 0.01) for t, v in gt["team_strength"].items()}
    res = fit(plays, players, market, gt["focus_qb"], n_iter=3, verbose=False)
    ratings = res["ratings"].sort_values("player_id").reset_index(drop=True)
    team_df = res["team_df"].sort_values("team").reset_index(drop=True)
    qb_weekly = res["qb_weekly"].sort_values("week").reset_index(drop=True)
    return {
        "dv": plays["dv"].to_numpy(),
        "rating": ratings["rating"].to_numpy(),
        "rating_pids": list(ratings["player_id"]),
        "team_rating": team_df["team_rating"].to_numpy(),
        "qb_credit": qb_weekly["qb_credit"].to_numpy(),
    }


def test_pipeline_is_deterministic_in_process():
    """Two full synth->fit runs in the same process match < 1e-9."""
    a = _run_pipeline()
    b = _run_pipeline()

    assert a["rating_pids"] == b["rating_pids"], "player column order is not stable"
    for key in ("dv", "rating", "team_rating", "qb_credit"):
        np.testing.assert_allclose(
            a[key], b[key], rtol=0, atol=1e-9,
            err_msg=f"{key!r} differs between two in-process runs — nondeterminism",
        )


def test_kalman_step_sequence_is_deterministic():
    """The incremental Kalman path is deterministic for a fixed observation
    sequence — the weekly engine state must not drift run-to-run."""
    def run():
        """Advance 12 weeks of seeded Kalman steps; return final state and the
        full predictive-variance trace (every week, not just the last)."""
        state = KalmanState.init(["p1", "p2", "p3", "p4"])
        rng = np.random.default_rng(123)
        snaps = np.array([30.0, 12.0, 45.0, 8.0])
        pred_trace = []
        for _ in range(12):
            obs = rng.normal(0.1, 0.25, 4)
            state, pred = kalman_step(state, obs, snaps, return_pred=True)
            pred_trace.append(pred.copy())
        return state.mu.copy(), state.sigma.copy(), np.stack(pred_trace)

    mu1, sig1, pred1 = run()
    mu2, sig2, pred2 = run()
    np.testing.assert_allclose(mu1, mu2, rtol=0, atol=1e-12)
    np.testing.assert_allclose(sig1, sig2, rtol=0, atol=1e-12)
    np.testing.assert_allclose(pred1, pred2, rtol=0, atol=1e-12)  # full trace, all 12 weeks
