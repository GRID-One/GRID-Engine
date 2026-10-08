"""Canonical synth snapshot for the Tier 0.5 golden master (roadmap Phase 0;
validation plan §6).

`compute_snapshot()` runs the engine on a FIXED synth seed and returns a
structured, canonically-ordered snapshot of every engine output the golden
master freezes (player ratings, team ratings, weekly QB credit, the focus-QB
Kalman trajectory) plus the planted ground truth the semantic invariants are
anchored to.  It has NO matplotlib / file-writing side effects (unlike
run_demo.py) so tests can import it directly.

Run ``python -m tests.grid.golden_master`` (from the repo root) to regenerate
the committed golden file ``tests/grid/golden/snapshot.npz`` after an intended
engine change — the golden diff in the PR is the feature (it shows reviewers
exactly which players/teams moved).
"""
from __future__ import annotations
from pathlib import Path

import numpy as np
from threadpoolctl import threadpool_limits

from backend.grid import (
    load_synthetic, fit_value_model, attach_dv, fit,
    kalman_two_component, SSParams, SynthConfig,
)

GOLDEN_PATH = Path(__file__).parent / "golden" / "snapshot.npz"

# Canonical run constants — changing any of these invalidates the golden file.
MARKET_SEED = 1
N_ITER = 3
FOCUS_INTERVENTION = 9   # 0-based week index of the planted injury return

# Pin the synth config EXPLICITLY rather than inheriting it: a future change to
# the defaults must not silently move this "fixed synth seed" snapshot.  These
# values MIRROR what bare load_synthetic() actually produces today — note it
# does NOT use the SynthConfig dataclass defaults: it overrides
# yards_noise_sd=3.2 and drives_per_team_per_game=12 (data_adapters.py).  This is
# the same dataset the rest of the synth suite (Tier 0, run_demo) runs on, so the
# golden stays consistent with them.  Changing any field here is an intentional
# golden regeneration.
CANONICAL_SYNTH = SynthConfig(
    n_teams=12, weeks=14, seed=7,
    yards_ability_gain=26.0, yards_noise_sd=3.2,        # bare-default override
    drives_per_team_per_game=12, max_plays_per_drive=12,  # bare-default override
    fg_range_yardline=35, league_factor=0.62, college_noise_sd=0.030,
    cb_split=False, n_cb_per_team=2,
)


def compute_snapshot() -> dict:
    """Run the canonical synth -> V(s) -> dV -> RAPM fixed point + focus-QB
    Kalman, and return a dict of canonically-ordered arrays + planted truth.

    Runs under ``threadpool_limits(1)`` so the result is reproducible regardless
    of the runner's OMP/BLAS thread count: multi-threaded the gradient-boosted
    V(s) / Layer-1 credit diverge at ~1e-2 (reduction-order + parallel-split
    nondeterminism), which the golden's 1e-5 tolerance could never absorb.  This
    makes the golden a single-thread freeze, matching the determinism gate.
    """
    with threadpool_limits(limits=1):
        return _compute_snapshot_inner()


def _compute_snapshot_inner() -> dict:
    b = load_synthetic(CANONICAL_SYNTH)
    plays, players, gt = b["plays"], b["players"], b["gt"]
    plays = attach_dv(plays, fit_value_model(plays))
    rng = np.random.default_rng(MARKET_SEED)
    market = {t: v + rng.normal(0, 0.01) for t, v in gt["team_strength"].items()}
    res = fit(plays, players, market, gt["focus_qb"], n_iter=N_ITER, verbose=False)

    ratings = res["ratings"].sort_values("player_id").reset_index(drop=True)
    team_df = res["team_df"].sort_values("team").reset_index(drop=True)
    qb_weekly = res["qb_weekly"].sort_values("week").reset_index(drop=True)

    # --- focus-QB Kalman trajectory (planted rise -> injury wk8-9 -> return) ---
    W = gt["focus_tau"].shape[0]
    y = np.full(W, np.nan)
    snaps = np.zeros(W)
    played = np.zeros(W, bool)
    wk = qb_weekly.set_index("week")
    for w in range(1, W + 1):
        if w in wk.index:
            y[w - 1] = wk.loc[w, "qb_credit"]
            snaps[w - 1] = wk.loc[w, "snaps"]
            played[w - 1] = True
    ss = kalman_two_component(
        y, snaps, played, interventions={FOCUS_INTERVENTION}, params=SSParams()
    )

    # team planted strength aligned to team_df order
    planted_team = np.array([gt["team_strength"][t] for t in team_df["team"]])

    return {
        # --- identity / ordering ---
        "player_id": ratings["player_id"].to_numpy().astype(str),
        "position": ratings["position"].to_numpy().astype(str),
        "team": team_df["team"].to_numpy().astype(str),
        "qb_week": qb_weekly["week"].to_numpy().astype(int),
        # --- numeric (Layer C) ---
        "rating": ratings["rating"].to_numpy(float),
        "ability": ratings["ability"].to_numpy(float),
        "team_rating": team_df["team_rating"].to_numpy(float),
        "planted_team": planted_team.astype(float),
        "qb_credit": qb_weekly["qb_credit"].to_numpy(float),
        "k_total_filt": ss["total_filt"],
        "k_total_smooth": ss["total_smooth"],
        "k_tau_smooth": ss["tau_smooth"],
        "k_var_total_filt": ss["var_total_filt"],
        "k_total_pred": ss["total_pred"],
        "k_var_total_pred": ss["var_total_pred"],
        # --- planted truth for semantic anchoring (not all frozen numerically) ---
        "focus_tau": gt["focus_tau"].astype(float),
        "focus_played": played,
    }


def _save(snapshot: dict) -> None:
    GOLDEN_PATH.parent.mkdir(parents=True, exist_ok=True)
    np.savez(str(GOLDEN_PATH), **snapshot)
    print(f"wrote golden snapshot -> {GOLDEN_PATH} ({len(snapshot)} arrays)")


def load_golden() -> dict:
    """Load the committed golden snapshot (pickle-disabled — all plain arrays)."""
    with np.load(str(GOLDEN_PATH), allow_pickle=False) as data:
        return {k: data[k] for k in data.files}


if __name__ == "__main__":
    _save(compute_snapshot())
