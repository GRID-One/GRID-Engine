"""Tests for the rolling-origin walk-forward driver (Phase 2a; §11, §11.5).

Network-free on the synthetic contract.  Single-threaded for determinism (the
value-model GBM diverges ~1e-2 multi-threaded; see the golden master).
"""
import numpy as np
import pytest
from threadpoolctl import threadpool_limits

from backend.grid import load_synthetic
from backend.grid.layers import accumulate, build_design, init_accumulators
from backend.grid.value import attach_dv, fit_value_model
from backend.validation import backtest as BT


@pytest.fixture(scope="module")
def synth_one_season():
    """A single synthetic season tagged with a season column for the driver."""
    b = load_synthetic()
    plays, players = b["plays"].copy(), b["players"]
    plays["season"] = 2023
    return plays, players


def test_enumerate_origins_skips_warmup(synth_one_season):
    """Origins are every (season, week) slot after the warm-up pre-period."""
    plays, _ = synth_one_season
    n_weeks = plays["week"].nunique()
    origins = BT.enumerate_origins(plays, warmup_weeks=4)
    assert len(origins) == n_weeks - 4
    assert origins[0] == (2023, 5)            # first 4 weeks reserved
    assert all(s == 2023 for s, _ in origins)


def test_too_few_slots_raises(synth_one_season):
    """A warm-up that swallows every slot leaves no origin -> error."""
    plays, players = synth_one_season
    with pytest.raises(ValueError, match="warmup_weeks"):
        BT.walk_forward(plays, players, warmup_weeks=plays["week"].nunique())


def test_invalid_params_raise(synth_one_season):
    """Non-positive warmup_weeks / origin_stride are rejected up front."""
    plays, players = synth_one_season
    with pytest.raises(ValueError, match="warmup_weeks must be > 0"):
        BT.walk_forward(plays, players, warmup_weeks=0)
    with pytest.raises(ValueError, match="origin_stride must be > 0"):
        BT.walk_forward(plays, players, warmup_weeks=4, origin_stride=0)


def test_walk_forward_produces_finite_ratings(synth_one_season):
    """Each origin yields finite RAPM ratings over the full player set, and the
    training-play count grows monotonically as weeks accrue."""
    plays, players = synth_one_season
    with threadpool_limits(limits=1):
        results = BT.walk_forward(plays, players, warmup_weeks=4)
    assert len(results) == plays["week"].nunique() - 4
    assert results[0].week == 5 and results[-1].week == plays["week"].nunique()
    # ratings cover every player (keys are the raw player_id values) and are finite
    assert set(results[-1].ratings) == set(players["player_id"])
    assert all(np.isfinite(v) for v in results[-1].ratings.values())
    # training set grows each origin (incremental accumulation)
    counts = [r.n_train_plays for r in results]
    assert counts == sorted(counts) and counts[0] < counts[-1]


def test_incremental_equals_from_scratch(synth_one_season):
    """The incremental accumulator solve at the final origin equals a from-scratch
    fit over all pre-origin weeks — the core perf claim (and the two-path basis)."""
    plays, players = synth_one_season
    warmup = 4
    with threadpool_limits(limits=1):
        results = BT.walk_forward(plays, players, warmup_weeks=warmup)
        last = results[-1]

        # Reconstruct the final origin from scratch: freeze V(s) on the same
        # pre-period, build the design over ALL weeks before the final origin in
        # one shot, accumulate once, and solve with the same settings.
        weeks = sorted(plays["week"].unique())
        pre = plays[plays["week"].isin(weeks[:warmup])]
        vmodel = fit_value_model(pre)
        before_last = plays[plays["week"] < last.week].copy()
        before_last = attach_dv(before_last, vmodel)
        X, y, colidx = build_design(before_last, players)
        XtX, Xty = init_accumulators(colidx["nCols"])
        XtX, Xty = accumulate(XtX, Xty, X, y)
        beta = BT.solve_rapm(XtX, Xty, colidx)
        scratch = {pid: float(beta[i]) for pid, i in colidx["p_col"].items()}

    inc = np.array([last.ratings[p] for p in colidx["p_col"]])
    ref = np.array([scratch[p] for p in colidx["p_col"]])
    np.testing.assert_allclose(inc, ref, rtol=1e-8, atol=1e-8)


def test_origin_stride_subsets(synth_one_season):
    """origin_stride>1 forecasts a strided subset of the full origin list."""
    plays, players = synth_one_season
    with threadpool_limits(limits=1):
        full = BT.walk_forward(plays, players, warmup_weeks=4)
        strided = BT.walk_forward(plays, players, warmup_weeks=4, origin_stride=2)
    full_slots = [(r.season, r.week) for r in full]
    strided_slots = [(r.season, r.week) for r in strided]
    assert strided_slots == full_slots[::2]
    assert len(strided_slots) < len(full_slots)


def test_market_anchor_changes_ratings(synth_one_season):
    """Passing a market anchor shifts the solve (the anchor is actually applied)."""
    plays, players = synth_one_season
    teams = sorted(players["team"].unique())
    market = {t: 1.0 for t in teams}        # uniform nonzero anchor
    with threadpool_limits(limits=1):
        base = BT.walk_forward(plays, players, warmup_weeks=4)
        anchored = BT.walk_forward(plays, players, warmup_weeks=4, market=market)
    b = np.array([base[-1].ratings[p] for p in base[-1].ratings])
    a = np.array([anchored[-1].ratings[p] for p in base[-1].ratings])
    assert not np.allclose(a, b)
