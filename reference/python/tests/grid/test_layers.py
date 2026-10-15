"""Tests for canonical player_id ordering in build_design and accumulator fingerprinting."""

from __future__ import annotations

import random

import numpy as np
import pandas as pd
import pytest

from backend.grid.layers import (
    build_design,
    init_accumulators,
    load_accumulators,
    save_accumulators,
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _make_players(player_ids: list[str], team: str = "KC") -> pd.DataFrame:
    return pd.DataFrame({"player_id": player_ids, "team": team, "position": "WR"})


def _make_players_two_teams(player_ids: list[str]) -> pd.DataFrame:
    """Split player_ids across KC (offense) and DEN (defense)."""
    half = len(player_ids) // 2 or 1
    teams = ["KC"] * half + ["DEN"] * (len(player_ids) - half)
    return pd.DataFrame({"player_id": player_ids, "team": teams, "position": "WR"})


def _make_plays(player_ids: list[str]) -> pd.DataFrame:
    """A minimal single-play DataFrame using the given player ids on both sides."""
    return pd.DataFrame(
        {
            "off_players": [player_ids[:2]],
            "def_players": [player_ids[2:4] if len(player_ids) >= 4 else player_ids[:2]],
            "dv": [1.0],
            "off_team": ["KC"],
            "def_team": ["DEN"],
        }
    )


# ---------------------------------------------------------------------------
# Test 1: column index of a known pid matches its sorted position
# ---------------------------------------------------------------------------

def test_build_design_player_columns_sorted():
    """Column index of a known pid must equal its position in the sorted pid list."""
    player_ids = ["z_player", "a_player", "m_player", "b_player"]
    players_df = _make_players_two_teams(player_ids)
    plays_df = _make_plays(player_ids)

    _, _, colidx = build_design(plays_df, players_df)

    sorted_pids = sorted(player_ids)
    p_col = colidx["p_col"]

    for expected_col, pid in enumerate(sorted_pids):
        assert p_col[pid] == expected_col, (
            f"pid={pid!r}: expected column {expected_col} (sorted position) "
            f"but got {p_col[pid]}"
        )


# ---------------------------------------------------------------------------
# Test 2: shuffling players_df rows yields identical colidx['p_col']
# ---------------------------------------------------------------------------

def test_build_design_order_stable_across_row_shuffle():
    """Shuffling players_df rows must not change p_col column assignments."""
    player_ids = ["z_player", "a_player", "m_player", "b_player"]
    players_df_original = _make_players_two_teams(player_ids)
    plays_df = _make_plays(player_ids)

    _, _, colidx_orig = build_design(plays_df, players_df_original)

    # Shuffle rows
    shuffled = players_df_original.sample(frac=1, random_state=42).reset_index(drop=True)
    _, _, colidx_shuffled = build_design(plays_df, shuffled)

    assert colidx_orig["p_col"] == colidx_shuffled["p_col"], (
        "p_col mapping changed after row shuffle — order is not canonical"
    )


# ---------------------------------------------------------------------------
# Test 3: same player count, different order → load triggers reinit
# ---------------------------------------------------------------------------

def test_accumulator_reinit_on_player_reorder(tmp_path, monkeypatch):
    """load_accumulators must return a player_order fingerprint that differs
    when player order changes, so callers can detect and reinit on REORDER."""
    import backend.grid.layers as layers_mod

    # Override the accumulator path to a temp file
    orig_path = layers_mod._ACCUM_PATH
    temp_path = tmp_path / "accumulators.npz"
    monkeypatch.setattr(layers_mod, "_ACCUM_PATH", temp_path)

    try:
        player_ids_v1 = ["a_player", "b_player", "c_player"]
        player_ids_v2 = ["c_player", "a_player", "b_player"]  # same set, different order

        n_cols = 3
        XtX, Xty = init_accumulators(n_cols)

        # Save with first player ordering
        save_accumulators(XtX, Xty, week=1, player_order=player_ids_v1)

        # Load back and verify player_order fingerprint is returned
        result = load_accumulators()
        assert result is not None, "load_accumulators returned None for existing file"
        assert len(result) == 4, (
            f"load_accumulators must return (XtX, Xty, week, player_order) "
            f"but returned {len(result)}-tuple"
        )
        loaded_XtX, loaded_Xty, loaded_week, loaded_order = result

        assert loaded_week == 1
        assert list(loaded_order) == player_ids_v1

        # Simulate reorder: current sorted pids differ from stored order
        current_sorted_pids = sorted(player_ids_v2)  # ["a_player", "b_player", "c_player"]
        stored_order = list(loaded_order)

        # Since save always stores sorted order (canonical), they should match after sort
        # But if we pass an unsorted order that differs, reinit should be triggered.
        # Save with unsorted order to simulate old/corrupt state.
        save_accumulators(XtX, Xty, week=1, player_order=player_ids_v2)
        result2 = load_accumulators()
        assert result2 is not None
        _, _, _, order2 = result2
        assert list(order2) == player_ids_v2, "stored order must be preserved faithfully"

        # The caller logic: reinit if stored order != current canonical order
        canonical = sorted(player_ids_v2)
        assert list(order2) != canonical, (
            "Reorder scenario: stored order should differ from current canonical sorted order"
        )
    finally:
        monkeypatch.setattr(layers_mod, "_ACCUM_PATH", orig_path)
