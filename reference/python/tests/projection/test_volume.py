"""Tests for the volume/role model (Phase 2b; roadmap §4.5).

Small inline weekly usage frames; checks shrinkage behavior, leakage safety, the
override hook, and per-position means.
"""
import pandas as pd
import pytest

from backend.validation.asof import AsOf
from backend.projection.volume import VOLUME_COLS, project_volume


def _wk(pid, pos, season, week, **vols):
    """One player-week row with VOLUME_COLS (unspecified default to 0)."""
    row = {"player_id": pid, "position": pos, "season": season, "week": week}
    for c in VOLUME_COLS:
        row[c] = vols.get(c, 0)
    return row


def _history(rows):
    return pd.DataFrame(rows)


def test_full_season_player_keeps_own_rate():
    """A player with many games barely regresses to the position mean."""
    # WR_A: 16 games at 10 targets/game in 2022; a second WR drags the pos mean down
    rows = [_wk("A", "WR", 2022, w, targets=10, receptions=7) for w in range(1, 17)]
    rows += [_wk("B", "WR", 2022, w, targets=2, receptions=1) for w in range(1, 17)]
    proj = project_volume(_history(rows), AsOf(2023, 1), k_shrink=8.0)
    # g/(g+k) = 16/24 = 0.667; pos mean targets = (10+2)/2 = 6
    expected = (16 / 24) * 10 + (8 / 24) * 6
    assert proj.per_game["A"]["targets"] == pytest.approx(expected)
    assert proj.games_prior["A"] == 16


def test_small_sample_player_pulled_to_position_mean():
    """A 2-game player is shrunk hard toward the position mean."""
    rows = [_wk("A", "RB", 2022, w, rush_attempts=20) for w in range(1, 17)]   # workhorse
    rows += [_wk("B", "RB", 2022, w, rush_attempts=4) for w in (1, 2)]          # 2 games
    proj = project_volume(_history(rows), AsOf(2023, 1), k_shrink=8.0)
    pos_mean = (20 + 4) / 2
    # B: g/(g+k) = 2/10 = 0.2
    expected = 0.2 * 4 + 0.8 * pos_mean
    assert proj.per_game["B"]["rush_attempts"] == pytest.approx(expected)


def test_uses_most_recent_prior_season():
    """When a player has two prior seasons, the most recent one is the base."""
    rows = [_wk("A", "WR", 2021, w, targets=2) for w in range(1, 17)]
    rows += [_wk("A", "WR", 2022, w, targets=12) for w in range(1, 17)]
    proj = project_volume(_history(rows), AsOf(2023, 1), k_shrink=0.0)  # no shrink
    assert proj.per_game["A"]["targets"] == pytest.approx(12.0)   # 2022, not 2021


def test_leakage_future_season_excluded():
    """Forecasting season S ignores S's own (and later) outcomes."""
    rows = [_wk("A", "WR", 2022, w, targets=5) for w in range(1, 17)]
    rows += [_wk("A", "WR", 2023, w, targets=15) for w in range(1, 17)]  # the future
    proj = project_volume(_history(rows), AsOf(2023, 1), k_shrink=0.0)
    assert proj.per_game["A"]["targets"] == pytest.approx(5.0)   # 2022 only


def test_baseline_is_prior_season_not_current_to_date():
    """The baseline is the prior season, even mid-season (current-to-date is not
    yet blended in — that's a deferred extension)."""
    rows = [_wk("A", "RB", 2022, w, rush_attempts=10) for w in range(1, 17)]  # prior
    rows += [_wk("A", "RB", 2023, w, rush_attempts=20) for w in (1, 2, 3)]    # current-to-date
    proj = project_volume(_history(rows), AsOf(2023, 5), k_shrink=0.0)        # forecast W=5
    assert proj.per_game["A"]["rush_attempts"] == pytest.approx(10.0)         # 2022, not 2023


def test_no_prior_season_returns_empty():
    """A player with only current-season data has no prior baseline -> empty."""
    rows = [_wk("A", "RB", 2023, w, rush_attempts=12) for w in (1, 2, 3)]
    proj = project_volume(_history(rows), AsOf(2023, 5))
    assert proj.per_game == {} and proj.games_prior == {}


def test_override_rejects_unknown_columns():
    """An override with a non-VOLUME column is rejected (catches ADP typos)."""
    rows = [_wk("A", "RB", 2022, w, rush_attempts=5) for w in range(1, 17)]
    with pytest.raises(ValueError, match="unknown volume override columns"):
        project_volume(_history(rows), AsOf(2023, 1),
                       overrides={"A": {"rushing_yards": 800}})   # not a usage driver


def test_override_hook_replaces_usage():
    """An ADP/depth-chart override replaces the projected usage for that player."""
    rows = [_wk("A", "RB", 2022, w, rush_attempts=5) for w in range(1, 17)]
    proj = project_volume(_history(rows), AsOf(2023, 1),
                          overrides={"A": {"rush_attempts": 18.0}})
    assert proj.per_game["A"]["rush_attempts"] == 18.0


def test_position_means_are_separate():
    """Shrinkage targets are per position — a WR isn't pulled toward RB usage."""
    rows = [_wk("W", "WR", 2022, w, targets=8) for w in (1, 2)]
    rows += [_wk("R", "RB", 2022, w, targets=1) for w in (1, 2)]
    proj = project_volume(_history(rows), AsOf(2023, 1), k_shrink=8.0)
    # each position has a single player, so its mean == own rate -> no cross-pull
    assert proj.per_game["W"]["targets"] == pytest.approx(8.0)
    assert proj.per_game["R"]["targets"] == pytest.approx(1.0)


def test_empty_history_returns_empty():
    """A truly empty input frame -> empty projection (not an error)."""
    empty = pd.DataFrame(columns=["player_id", "position", "season", "week", *VOLUME_COLS])
    proj = project_volume(empty, AsOf(2023, 1))
    assert proj.per_game == {} and proj.games_prior == {}
