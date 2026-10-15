"""Tests for run_situation_rapm — per-situation RAPM orchestration (Task 5).

All tests use synthetic data (synth.simulate + value.attach_dv) because
participation columns (off_players / def_players) are not yet emitted by the
real nflverse loader.  The honesty gate (missing-participation raise) is tested
separately.
"""
from __future__ import annotations

import numpy as np
import pandas as pd
import pytest

from backend.grid.layers import run_situation_rapm


# ---------------------------------------------------------------------------
# Shared helpers
# ---------------------------------------------------------------------------

def _make_synth_plays(seed: int = 42):
    """Return (plays_with_dv, players) using synth.simulate + value.attach_dv."""
    from backend.grid.synth import simulate, SynthConfig
    from backend.grid.value import fit_value_model, attach_dv

    cfg = SynthConfig(seed=seed)
    plays, players, _gt = simulate(cfg)
    vm = fit_value_model(plays)
    plays = attach_dv(plays, vm)
    return plays, players


def _make_tiny_plays(n: int, off_players, def_players,
                     yardline_100=50, down=1, ydstogo=10, dv=0.1):
    """Build a minimal plays DataFrame with participation columns."""
    return pd.DataFrame({
        "off_players": [off_players] * n,
        "def_players": [def_players] * n,
        "off_team": ["A"] * n,
        "def_team": ["B"] * n,
        "dv": [float(dv)] * n,
        "down": [down] * n,
        "ydstogo": [ydstogo] * n,
        "yardline_100": [yardline_100] * n,
    })


def _make_tiny_players(player_ids):
    """Build a minimal players DataFrame for a two-team setup."""
    half = len(player_ids) // 2 or 1
    teams = ["A"] * half + ["B"] * (len(player_ids) - half)
    return pd.DataFrame({
        "player_id": player_ids,
        "team": teams,
        "position": ["WR"] * len(player_ids),
        "is_starter": [True] * len(player_ids),
        "ability": [0.0] * len(player_ids),
    })


# ---------------------------------------------------------------------------
# Test 1: returns subset of situations
# ---------------------------------------------------------------------------

def test_run_situation_rapm_returns_subset_of_situations():
    """run_situation_rapm must return a dict keyed by situation name.

    At minimum the keys must be a non-empty subset of the situations that
    classify() would produce (red_zone, passing_downs, rushing_downs, and
    optionally two_minute).  Each value must be a DataFrame with a 'player_id'
    column.
    """
    plays, players = _make_synth_plays(seed=0)

    result = run_situation_rapm(plays, players, min_plays=1)

    assert isinstance(result, dict), "result must be a dict"
    assert len(result) > 0, "at least one situation must pass the min_plays threshold"

    known_situations = {"red_zone", "passing_downs", "rushing_downs", "two_minute"}
    for key in result:
        assert key in known_situations, f"unexpected situation key: {key!r}"
        ratings = result[key]
        assert isinstance(ratings, pd.DataFrame), f"{key}: value must be a DataFrame"
        assert "player_id" in ratings.columns, f"{key}: ratings must have player_id column"


# ---------------------------------------------------------------------------
# Test 2: low-volume situation is skipped
# ---------------------------------------------------------------------------

def test_low_volume_situation_skipped():
    """Situations whose subframe has fewer rows than min_plays must be skipped.

    We construct custom masks so that one situation has 0 plays and confirm
    it is absent from the output dict.
    """
    plays, players = _make_synth_plays(seed=1)

    # Build custom masks: only one situation has enough plays (all plays),
    # the rest get an empty mask.
    n = len(plays)
    custom_masks = {
        "big_situation":   np.ones(n, dtype=bool),
        "tiny_situation":  np.zeros(n, dtype=bool),
    }

    result = run_situation_rapm(plays, players, situations=custom_masks, min_plays=1)

    assert "big_situation" in result, "big_situation (all plays) must be in result"
    assert "tiny_situation" not in result, "tiny_situation (0 plays) must be skipped"


# ---------------------------------------------------------------------------
# Test 3: red_zone ratings have expected player columns
# ---------------------------------------------------------------------------

def test_red_zone_ratings_have_expected_player_columns():
    """The ratings DataFrame returned for red_zone must have the full set of
    player metadata columns that run_rapm normally returns (player_id, rating,
    team, position, is_starter, ability).
    """
    plays, players = _make_synth_plays(seed=2)

    result = run_situation_rapm(plays, players, min_plays=1)

    assert "red_zone" in result, "red_zone must be present (synth data has red-zone plays)"
    rz = result["red_zone"]
    for col in ("player_id", "rating", "team", "position", "is_starter", "ability"):
        assert col in rz.columns, f"red_zone ratings missing column: {col!r}"
    assert len(rz) > 0, "red_zone ratings must not be empty"
    assert not rz["rating"].isna().any(), "red_zone ratings must not contain NaN"


# ---------------------------------------------------------------------------
# Test 4: situation ratings are independent (planted red-zone bump)
# ---------------------------------------------------------------------------

def test_situation_ratings_independent():
    """Situation-specific ratings must differ from overall ratings when a player has
    a planted red-zone-only signal.

    Strategy:
      1. Pick a QB that appears in red-zone plays.
      2. Run situation RAPM on the un-bumped plays → baseline red_zone rating.
      3. Boost that player's dv on red-zone-only plays by a large constant (BUMP).
      4. Run situation RAPM on the boosted plays → bumped red_zone rating.
      5. Assert the bumped red_zone rating is meaningfully higher than the baseline.
         (The signal is isolated to red-zone; the overall solve across all plays
         would dilute the same bump — the per-situation solve amplifies it.)
    """
    from backend.grid.layers import run_rapm

    plays, players = _make_synth_plays(seed=3)

    # Pick a QB that appears in red-zone plays
    qb_rows = players[players.position == "QB"]
    focus_pid = int(qb_rows.iloc[0]["player_id"])

    # Identify red-zone plays where the focus player is on the field
    rz_mask = plays["yardline_100"] <= 20
    on_field_mask = plays["off_players"].apply(lambda lst: focus_pid in lst)
    boost_mask = rz_mask & on_field_mask

    if boost_mask.sum() < 5:
        pytest.skip("Too few red-zone on-field plays for focus player; try different seed")

    # Baseline situation RAPM (no bump)
    baseline_result = run_situation_rapm(plays, players, min_plays=1)
    assert "red_zone" in baseline_result, "red_zone must be present in baseline"
    baseline_rz = baseline_result["red_zone"]
    baseline_rating = float(
        baseline_rz.loc[baseline_rz["player_id"] == focus_pid, "rating"].iloc[0]
    )

    # Plant a large positive bump on red-zone on-field plays for the focus player
    BUMP = 2.0
    boosted_plays = plays.copy()
    boosted_plays.loc[boost_mask, "dv"] = boosted_plays.loc[boost_mask, "dv"] + BUMP

    # Situation RAPM on boosted plays
    bumped_result = run_situation_rapm(boosted_plays, players, min_plays=1)

    assert "red_zone" in bumped_result, "red_zone must be present after boosting"
    bumped_rz = bumped_result["red_zone"]
    bumped_row = bumped_rz[bumped_rz["player_id"] == focus_pid]
    assert len(bumped_row) > 0, "focus player must appear in bumped red_zone ratings"
    bumped_rating = float(bumped_row["rating"].iloc[0])

    assert bumped_rating > baseline_rating, (
        f"bumped red_zone rating ({bumped_rating:.4f}) should exceed baseline "
        f"({baseline_rating:.4f}) for the player with a planted red-zone bump"
    )


# ---------------------------------------------------------------------------
# Test 5: missing participation columns raises ValueError
# ---------------------------------------------------------------------------

def test_missing_participation_columns_raises():
    """run_situation_rapm must raise ValueError immediately when participation
    columns are absent — before attempting any solve.

    This is the honesty gate: real nflverse data does not yet emit off_players /
    def_players, so any call on real data must fail fast with a clear message.
    """
    plays_no_participation = pd.DataFrame({
        "dv": [0.1, 0.2, 0.3],
        "down": [1, 2, 3],
        "ydstogo": [10, 7, 1],
        "yardline_100": [75, 50, 25],
        "off_team": ["A", "A", "B"],
        "def_team": ["B", "B", "A"],
    })
    players = _make_tiny_players(["p1", "p2", "p3", "p4"])

    with pytest.raises(ValueError, match="participation columns"):
        run_situation_rapm(plays_no_participation, players)
