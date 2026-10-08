"""Leakage guards (roadmap Phase 2a; validation plan §8 — three axes, not one).

The four guards the trust layer rests on, each shipping a deliberate-leak
**canary** ("test the test") so the guard can't silently rot into a no-op:

1. Metamorphic future-poisoning (the headline, mechanism-agnostic) — forecasting
   week W twice with two *different* futures must give a bit-identical week-W
   result; any dependence on future content surfaces as a difference.
2. Temporal watermark — the accumulators behind a week-W forecast never absorb
   week >= W.
3. Tripwire localization — a frame that still carries future rows raises at the
   use site.
4. Two-path equivalence — the offline backtest's week-W forecast equals what the
   production ``weekly_update`` forward path produces after weeks 1..W-1 (also a
   ``weekly_update`` regression).

Single-threaded throughout: the value-model GBM diverges ~1e-2 multi-threaded
(see the golden master), and these guards assert exact / tight-tolerance equality.
"""
import sqlite3

import numpy as np
import pytest
from threadpoolctl import threadpool_limits

import backend.grid.layers as layers_mod
import backend.grid.statespace as ss_mod
from backend.db.connection import init_db
from backend.grid import load_synthetic
from backend.grid.layers import accumulate, build_design, init_accumulators, load_accumulators
from backend.grid.value import STATE_COLS, attach_dv, fit_value_model
from backend.pipeline.weekly_update import run as weekly_run
from backend.validation import backtest as BT
from backend.validation.asof import AsOf, LeakageError

SEASON = 2023
WARMUP = 4
W = 8   # the origin week these guards forecast (an origin: 5..14 with warmup=4)


@pytest.fixture(scope="module")
def synth():
    """A single synthetic season tagged with a season column."""
    b = load_synthetic()
    plays, players = b["plays"].copy(), b["players"]
    plays["season"] = SEASON
    return plays, players


def _poison_future(plays, seed: int):
    """Return a copy with the STATE columns of rows at week >= W shuffled.

    Shuffling state (down/distance/yardline) for future rows changes their dV
    under the frozen V(s), so any forecaster that *reads* week >= W produces a
    different answer — while a leakage-safe one (weeks < W only) does not.
    """
    rng = np.random.default_rng(seed)
    p = plays.copy()
    idx = p.index[p["week"] >= W]
    perm = rng.permutation(len(idx))
    for col in STATE_COLS:
        p.loc[idx, col] = p.loc[idx, col].to_numpy()[perm]
    return p


def _ratings_vec(ratings: dict) -> np.ndarray:
    """Ratings as a vector in a stable (sorted) key order."""
    return np.array([ratings[k] for k in sorted(ratings, key=str)])


def _leaky_forecast(plays, players, vmodel) -> dict:
    """A DELIBERATELY leaky forecaster: fits RAPM on ALL weeks, incl. the future."""
    allp = attach_dv(plays.copy(), vmodel)
    X, y, colidx = build_design(allp, players)
    XtX, Xty = init_accumulators(colidx["nCols"])
    XtX, Xty = accumulate(XtX, Xty, X, y)
    beta = BT.solve_rapm(XtX, Xty, colidx)
    return {pid: float(beta[i]) for pid, i in colidx["p_col"].items()}


# --------------------------------------------------------------------------- #
# 1. Metamorphic future-poisoning (headline) + canary                         #
# --------------------------------------------------------------------------- #
def test_future_poisoning_leaves_forecast_bit_identical(synth):
    """Two different futures (weeks >= W) -> identical week-W ratings."""
    plays, players = synth
    with threadpool_limits(limits=1):
        a = next(r for r in BT.walk_forward(_poison_future(plays, 1), players,
                                            warmup_weeks=WARMUP) if r.week == W)
        b = next(r for r in BT.walk_forward(_poison_future(plays, 2), players,
                                            warmup_weeks=WARMUP) if r.week == W)
    np.testing.assert_array_equal(_ratings_vec(a.ratings), _ratings_vec(b.ratings))


def test_poisoning_canary_detects_a_leak(synth):
    """CANARY: a forecaster that reads the future DOES differ between futures,
    proving the poisoning guard above is not vacuous."""
    plays, players = synth
    weeks = sorted(plays["week"].unique())
    with threadpool_limits(limits=1):
        vmodel = fit_value_model(plays[plays["week"].isin(weeks[:WARMUP])])
        la = _leaky_forecast(_poison_future(plays, 1), players, vmodel)
        lb = _leaky_forecast(_poison_future(plays, 2), players, vmodel)
    assert not np.allclose(_ratings_vec(la), _ratings_vec(lb))


# --------------------------------------------------------------------------- #
# 2. Temporal watermark + canary                                              #
# --------------------------------------------------------------------------- #
def test_watermark_holds_across_the_walk(synth):
    """The driver's per-origin watermark assertion holds for every origin (no raise)."""
    plays, players = synth
    with threadpool_limits(limits=1):
        results = BT.walk_forward(plays, players, warmup_weeks=WARMUP)
    assert [r.week for r in results] == sorted(plays["week"].unique())[WARMUP:]


def test_watermark_canary(synth):
    """CANARY: an accumulator that absorbed week W is flagged for a week-W forecast."""
    with pytest.raises(LeakageError, match="future outcomes leaked"):
        AsOf(SEASON, W).assert_watermark(W)


# --------------------------------------------------------------------------- #
# 3. Tripwire localization + canary                                           #
# --------------------------------------------------------------------------- #
def test_tripwire_passes_on_asof_slice(synth):
    """A frame sliced through AsOf is clean, so the tripwire is transparent."""
    plays, _ = synth
    asof = AsOf(SEASON, W)
    tw = asof.tripwire(asof.slice_plays(plays))
    assert max(tw["week"]) <= W - 1


def test_tripwire_canary_fires_at_read_site(synth):
    """CANARY: a frame clean when wrapped but later given a future row trips on
    read — exercising the access-site localization, not just construction."""
    plays, _ = synth
    asof = AsOf(SEASON, W)
    sub = asof.slice_plays(plays)          # weeks <= W-1: clean at construction
    tw = asof.tripwire(sub)
    sub.loc[sub.index.max() + 1, "week"] = W + 1   # inject a future row post-wrap
    with pytest.raises(LeakageError, match="future leak"):
        _ = tw["week"]


# --------------------------------------------------------------------------- #
# 4. Two-path equivalence (offline backtest == production weekly_update)       #
# --------------------------------------------------------------------------- #
def test_two_path_equivalence(synth, tmp_path, monkeypatch):
    """The offline week-W forecast equals what weekly_update yields after 1..W-1.

    Drives the production forward path (disk + DB confined to tmp) week by week
    with the SAME frozen V(s) the driver uses, then solves its accumulated state
    with the same solver and asserts the ratings match the offline origin-W
    result — validating the harness against production and doubling as a
    weekly_update regression.
    """
    plays, players = synth
    weeks = sorted(plays["week"].unique())

    with threadpool_limits(limits=1):
        # --- offline path ---
        offline = next(r for r in BT.walk_forward(plays, players, warmup_weeks=WARMUP)
                       if r.week == W)

        # --- production path (confined) ---
        monkeypatch.setattr(layers_mod, "_ACCUM_PATH", tmp_path / "acc.npz")
        monkeypatch.setattr(ss_mod, "_KALMAN_PATH", tmp_path / "kal.npz")
        monkeypatch.setattr("backend.pipeline.weekly_update.run_valuations", lambda **k: None)
        monkeypatch.setattr("backend.pipeline.weekly_update.write_health", lambda **k: None)
        conn = sqlite3.connect(str(tmp_path / "t.db"), check_same_thread=False)
        init_db(conn)

        vmodel = fit_value_model(plays[plays["week"].isin(weeks[:WARMUP])])
        colidx = build_design(attach_dv(plays[plays["week"] == weeks[0]].copy(), vmodel),
                              players)[2]
        for w in weeks:
            if w >= W:
                break
            wk = attach_dv(plays[plays["week"] == w].copy(), vmodel)
            weekly_run(week=int(w), season=SEASON, plays_df=wk, players_df=players,
                       market_strength=None, _conn=conn)

        loaded = load_accumulators(key="all")
        assert loaded is not None
        XtX, Xty, last_week, player_order = loaded
        # the persisted column order must match the layout we solve with, else we
        # would misinterpret the production accumulator columns
        assert player_order == [str(k) for k in colidx["p_col"]]
        beta = BT.solve_rapm(XtX, Xty, colidx)
        prod = {pid: float(beta[i]) for pid, i in colidx["p_col"].items()}

    assert last_week == W - 1   # production absorbed only weeks before W
    inc = np.array([offline.ratings[p] for p in colidx["p_col"]])
    ref = np.array([prod[p] for p in colidx["p_col"]])
    np.testing.assert_allclose(inc, ref, rtol=1e-6, atol=1e-6)
