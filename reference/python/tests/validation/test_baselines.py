"""Tests for the baseline forecasters (Phase 2a; validation plan §9).

Small inline realized-points history; checks each baseline's value + leakage
safety (outcomes strictly before week W).
"""
import numpy as np
import pytest

import pandas as pd

from backend.validation.asof import AsOf
from backend.validation import baselines as B


def _history():
    """Two seasons of weekly fantasy points for two players."""
    rows = []
    # 2022: A averages 10, B averages 20
    for wk, (a, b) in enumerate([(8, 18), (12, 22), (10, 20)], start=1):
        rows.append({"player_id": "A", "season": 2022, "week": wk, "points": a})
        rows.append({"player_id": "B", "season": 2022, "week": wk, "points": b})
    # 2023: A = [14, 16, 30 (future)], B = [24, 26, 0 (future)]
    for wk, (a, b) in enumerate([(14, 24), (16, 26), (30, 0)], start=1):
        rows.append({"player_id": "A", "season": 2023, "week": wk, "points": a})
        rows.append({"player_id": "B", "season": 2023, "week": wk, "points": b})
    return pd.DataFrame(rows)


# --------------------------------------------------------------------------- #
# persistence                                                                 #
# --------------------------------------------------------------------------- #
def test_persistence_uses_last_week_before_w():
    """Forecasting 2023 W=3 uses week-2 points (week 3 is the future target)."""
    f = B.persistence(_history(), AsOf(2023, 3))
    assert f.mean == {"A": 16.0, "B": 26.0}     # week-2 values, not week-3


def test_persistence_empty_at_week_1():
    """No in-season game exists before W=1, so persistence is empty."""
    f = B.persistence(_history(), AsOf(2023, 1))
    assert f.mean == {}


# --------------------------------------------------------------------------- #
# season-to-date mean (+ spread)                                              #
# --------------------------------------------------------------------------- #
def test_season_to_date_mean_and_spread():
    """Mean over current-season weeks < W; a real per-player std is emitted."""
    f = B.season_to_date_mean(_history(), AsOf(2023, 3))   # weeks 1-2
    assert f.mean == pytest.approx({"A": 15.0, "B": 25.0})
    # two games each -> sample std of [14,16] and [24,26] = sqrt(2)
    assert f.sd["A"] == pytest.approx(np.sqrt(2.0))
    assert f.sd["B"] == pytest.approx(np.sqrt(2.0))


def test_season_to_date_mean_single_game_uses_pooled_spread():
    """With one game each (W=2), per-player std is undefined -> pooled fallback, non-zero."""
    f = B.season_to_date_mean(_history(), AsOf(2023, 2))   # week 1 only
    assert f.mean == pytest.approx({"A": 14.0, "B": 24.0})
    # pooled = std([14, 24], ddof=1); both players get it, and it's > 0
    pooled = np.std([14.0, 24.0], ddof=1)
    assert f.sd["A"] == pytest.approx(pooled) and f.sd["A"] > 0


def test_season_to_date_mean_single_player_uses_tiny_floor():
    """One player with one game: no per-player and no pooled spread -> tiny floor,
    not an inflated 1.0 that would skew CRPS/PICP for sparse early folds."""
    hist = pd.DataFrame([{"player_id": "A", "season": 2023, "week": 1, "points": 12.0}])
    f = B.season_to_date_mean(hist, AsOf(2023, 2))   # week 1 only, single player
    assert f.mean == pytest.approx({"A": 12.0})
    assert f.sd["A"] == pytest.approx(1e-6)


# --------------------------------------------------------------------------- #
# last-season-actuals (the H1 kill-criterion bar)                             #
# --------------------------------------------------------------------------- #
def test_last_season_per_game_average():
    """Forecast = prior-season (2022) per-game average, independent of W."""
    f = B.last_season(_history(), AsOf(2023, 3))
    assert f.mean == pytest.approx({"A": 10.0, "B": 20.0})   # 2022 means
    assert f.sd is None


def test_last_season_ignores_current_season():
    """Even late in 2023, last-season uses only 2022 (no current-season bleed)."""
    f = B.last_season(_history(), AsOf(2023, 14))
    assert f.mean == pytest.approx({"A": 10.0, "B": 20.0})


# --------------------------------------------------------------------------- #
# market (report-only)                                                         #
# --------------------------------------------------------------------------- #
def test_market_from_dict_and_dataframe():
    """Market wraps an external projection mapping or table unchanged."""
    f1 = B.market({"A": 11.0, "B": 21.0})
    assert f1.mean == {"A": 11.0, "B": 21.0} and f1.sd is None
    df = pd.DataFrame({"player_id": ["A", "B"], "projected_points": [11, 21]})
    f2 = B.market(df)
    assert f2.mean == pytest.approx({"A": 11.0, "B": 21.0})
