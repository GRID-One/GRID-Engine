"""Tests for the as-of accessor + cache namespace + tripwire (Phase 2a; §8).

Each leakage guard ships with a deliberate-leak canary ("test the test"): a
fixture that *would* leak, asserting the guard catches it.
"""
import numpy as np
import pandas as pd
import pytest

from backend.validation.asof import (
    AsOf,
    CacheNamespace,
    LeakageError,
    TripwireFrame,
)


def _plays():
    """Tiny plays-contract frame spanning weeks 1-4 with participation."""
    return pd.DataFrame({
        "week": [1, 2, 3, 4],
        "off_players": [("QB1", "WR1"), ("QB1", "WR2"), ("QB2", "WR1"), ("QB3", "WR9")],
        "def_players": [("CB1",), ("CB1",), ("CB2",), ("CB9",)],
        "drive_points": [7.0, 0.0, 3.0, 7.0],
    })


# --------------------------------------------------------------------------- #
# Cutoffs                                                                      #
# --------------------------------------------------------------------------- #
def test_cutoffs_split_outcome_vs_pregame():
    """Outcomes are known through W-1; pre-game signals are available for W."""
    a = AsOf(season=2023, week=5)
    assert a.outcome_cutoff == 4
    assert a.pregame_cutoff == 5


# --------------------------------------------------------------------------- #
# Slices (temporal + scope axes)                                              #
# --------------------------------------------------------------------------- #
def test_slice_plays_excludes_week_w_and_later():
    """Forecasting W=3 sees only outcome weeks 1-2 (no week 3+)."""
    sub = AsOf(2023, 3).slice_plays(_plays())
    assert set(sub["week"]) == {1, 2}


def test_slice_pool_is_as_of_w_no_later_callups():
    """The player universe through W-1 excludes a week-4 call-up (QB3/WR9/CB9)."""
    pool = AsOf(2023, 4).slice_pool(_plays())   # outcomes weeks 1-3
    assert {"QB1", "QB2", "WR1", "WR2", "CB1", "CB2"} <= pool
    assert "QB3" not in pool and "WR9" not in pool and "CB9" not in pool


def test_slice_plays_is_season_aware():
    """On a multi-season frame: keep all prior seasons + current season <= W-1."""
    multi = pd.DataFrame({
        "season": [2022, 2022, 2023, 2023, 2023],
        "week": [17, 18, 1, 3, 5],   # 2022 wk18 is prior-season (known); 2023 wk5 is future
        "off_players": [()] * 5,
        "def_players": [()] * 5,
        "drive_points": [0.0] * 5,
    })
    sub = AsOf(2023, 4).slice_plays(multi)   # forecasting 2023 wk4 -> outcomes <= wk3
    # prior season kept in full (incl. its high week numbers); current season <= 3
    assert list(zip(sub["season"], sub["week"])) == [(2022, 17), (2022, 18), (2023, 1), (2023, 3)]


def test_intervention_missing_week_is_rejected():
    """CANARY: an undated intervention would be visible to every fold -> rejected."""
    with pytest.raises(LeakageError, match="missing required"):
        AsOf(2023, 5).slice_interventions([{"player_id": "QB1"}])   # no 'week'


def test_slice_market_dict_passthrough_and_table_slice():
    """A dict anchor passes through; a per-week table is cut to ≤ W (pre-game)."""
    a = AsOf(2023, 3)
    assert a.slice_market({"A": 1.0, "B": -1.0}) == {"A": 1.0, "B": -1.0}
    tbl = pd.DataFrame({"week": [1, 2, 3, 4], "team": list("ABCD"), "line": [1, 2, 3, 4]})
    sliced = a.slice_market(tbl)
    assert set(sliced["week"]) == {1, 2, 3}   # includes W=3 (line published pre-kickoff)


def test_slice_interventions_excludes_future_weeks():
    """A week-5 forecast must not see a week-9 regime change (intervention foreknowledge)."""
    ivs = [{"player_id": "QB1", "week": 3}, {"player_id": "QB2", "week": 9}]
    kept = AsOf(2023, 5).slice_interventions(ivs)
    assert [iv["week"] for iv in kept] == [3]


# --------------------------------------------------------------------------- #
# Watermark guard (+ canary)                                                   #
# --------------------------------------------------------------------------- #
def test_watermark_ok_through_w_minus_1():
    """An accumulator that absorbed only weeks ≤ W-1 passes."""
    AsOf(2023, 5).assert_watermark(last_week=4)   # no raise


def test_watermark_canary_catches_future_absorption():
    """CANARY: an accumulator that absorbed week ≥ W is a temporal leak."""
    with pytest.raises(LeakageError, match="future outcomes leaked"):
        AsOf(2023, 5).assert_watermark(last_week=5)


# --------------------------------------------------------------------------- #
# Cache namespace (state axis) (+ canary)                                      #
# --------------------------------------------------------------------------- #
def test_cache_namespace_paths_and_isolation(tmp_path):
    """A scratch namespace exposes confined accum/kalman paths and is isolated."""
    ns = CacheNamespace(tmp_path / "fold0")
    assert ns.accum_base_path.parent == (tmp_path / "fold0")
    assert ns.kalman_path.name == "kalman_state.npz"
    assert ns.is_isolated()
    ns.assert_isolated()   # no raise


def test_cache_namespace_canary_rejects_production_dir():
    """CANARY: a production-dir namespace must raise at construction (before mkdir)."""
    with pytest.raises(LeakageError, match="clobber real state"):
        CacheNamespace("data/cache")


# --------------------------------------------------------------------------- #
# Tripwire proxy (temporal localization) (+ canary)                           #
# --------------------------------------------------------------------------- #
def test_tripwire_passes_clean_slice():
    """A correctly-sliced frame is transparently usable through the proxy."""
    clean = AsOf(2023, 3).slice_plays(_plays())
    tw = AsOf(2023, 3).tripwire(clean)
    assert len(tw) == 2
    assert set(tw["week"]) == {1, 2}          # __getitem__ works
    assert list(tw.columns) == list(clean.columns)   # __getattr__ forwards


def test_tripwire_canary_fires_on_future_row():
    """CANARY: wrapping an unsliced frame (contains week ≥ W) raises at use site."""
    with pytest.raises(LeakageError, match="future leak"):
        TripwireFrame(_plays(), cutoff=2, week_col="week", source="plays")


def test_tripwire_detects_post_construction_mutation():
    """A frame that passes at construction but is later given a future row trips on read."""
    df = _plays()[_plays()["week"] <= 2].copy()
    tw = TripwireFrame(df, cutoff=2)
    # mutate the underlying frame to inject a future row, then read through proxy
    df.loc[99] = {"week": 9, "off_players": (), "def_players": (), "drive_points": 0.0}
    with pytest.raises(LeakageError, match="future leak"):
        _ = tw["week"]
