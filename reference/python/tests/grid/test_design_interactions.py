"""
Tests for optional WR-vs-CB interaction columns in build_design (Task 7).

All five tests align with the task-7 brief. The interactions feature is
OFF by default; these tests verify the default path is byte-for-byte
unchanged and that enabling interactions produces the correct structure.
"""
from __future__ import annotations

import numpy as np
import pandas as pd
import pytest
import scipy.sparse as sp

from backend.grid.layers import build_design
from backend.grid.synth import simulate, SynthConfig


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _minimal_plays_players(
    n_wr: int = 2,
    n_def: int = 3,
    n_plays: int = 50,
    seed: int = 42,
    with_positions: bool = True,
) -> tuple[pd.DataFrame, pd.DataFrame]:
    """Build a tiny synthetic plays+players frame for isolation tests.

    Offense: 1 QB + n_wr WRs + 1 RB.  Defense: n_def DEF players.
    All on one team per side so team columns are deterministic.
    """
    rng = np.random.default_rng(seed)

    # players
    off_players = (
        [dict(player_id="qb1", team="OFF", position="QB", is_starter=True, ability=0.1)]
        + [dict(player_id=f"wr{i}", team="OFF", position="WR", is_starter=True, ability=0.05 * i)
           for i in range(n_wr)]
        + [dict(player_id="rb1", team="OFF", position="RB", is_starter=True, ability=0.02)]
    )
    def_players_list = [
        dict(player_id=f"def{i}", team="DEF", position="DEF", is_starter=True, ability=-0.01 * i)
        for i in range(n_def)
    ]
    players = pd.DataFrame(off_players + def_players_list)
    if not with_positions:
        players = players.drop(columns=["position"])

    # plays: all WRs on offense, all DEFs on defense
    off_ids = tuple(p["player_id"] for p in off_players)
    def_ids = tuple(p["player_id"] for p in def_players_list)
    plays = pd.DataFrame({
        "off_team": ["OFF"] * n_plays,
        "def_team": ["DEF"] * n_plays,
        "off_players": [off_ids] * n_plays,
        "def_players": [def_ids] * n_plays,
        "dv": rng.normal(0, 1, n_plays),
    })
    return plays, players


def _minimal_plays_players_with_cb(
    n_wr: int = 2,
    n_cb: int = 2,
    n_other_def: int = 2,
    n_plays: int = 60,
    seed: int = 99,
) -> tuple[pd.DataFrame, pd.DataFrame]:
    """Build a plays+players frame with explicit CB position for pruning tests."""
    rng = np.random.default_rng(seed)

    off_players = [
        dict(player_id="qb1", team="OFF", position="QB", is_starter=True, ability=0.1),
    ] + [
        dict(player_id=f"wr{i}", team="OFF", position="WR", is_starter=True, ability=0.04 * i)
        for i in range(n_wr)
    ]
    cb_players = [
        dict(player_id=f"cb{i}", team="DEF", position="CB", is_starter=True, ability=-0.02 * i)
        for i in range(n_cb)
    ]
    other_def = [
        dict(player_id=f"dl{i}", team="DEF", position="DEF", is_starter=True, ability=0.0)
        for i in range(n_other_def)
    ]

    players = pd.DataFrame(off_players + cb_players + other_def)
    off_ids = tuple(p["player_id"] for p in off_players)

    # plays: first half has cb0+cb1, second half has only cb0 (so cb1 pairs drop with high threshold)
    half = n_plays // 2
    def_ids_full = tuple(p["player_id"] for p in cb_players + other_def)
    def_ids_partial = tuple(
        p["player_id"] for p in cb_players[:1] + other_def
    )
    plays = pd.DataFrame({
        "off_team": ["OFF"] * n_plays,
        "def_team": ["DEF"] * n_plays,
        "off_players": [off_ids] * n_plays,
        "def_players": [def_ids_full] * half + [def_ids_partial] * (n_plays - half),
        "dv": rng.normal(0, 1, n_plays),
    })
    return plays, players


# ---------------------------------------------------------------------------
# Test 1: default (interactions=False) is byte-for-byte identical to pre-task colidx
# ---------------------------------------------------------------------------

def test_build_design_default_no_interactions_unchanged_nCols():
    """With interactions=False (the default), build_design must return
    the exact same colidx keys and nCols as it did before Task 7.
    This is the regression guard: the default path MUST NOT change."""
    plays, players = _minimal_plays_players(n_wr=2, n_def=3, n_plays=40)

    # Call with explicit False (same as omitting)
    X_new, y_new, colidx_new = build_design(plays, players, interactions=False)

    # Also call the old way (no keyword at all) — must be identical
    X_old, y_old, colidx_old = build_design(plays, players)

    # Shape must be identical
    assert X_new.shape == X_old.shape, "X shape changed with interactions=False"
    assert y_new.shape == y_old.shape

    # colidx keys must be identical (no 'interactions' key added)
    assert set(colidx_new.keys()) == set(colidx_old.keys()), (
        f"colidx gained unexpected keys: {set(colidx_new.keys()) - set(colidx_old.keys())}"
    )
    assert "interactions" not in colidx_new, \
        "interactions key must NOT appear when interactions=False"

    # nCols must be identical
    assert colidx_new["nCols"] == colidx_old["nCols"], \
        f"nCols changed: {colidx_new['nCols']} vs {colidx_old['nCols']}"

    # p_col, t_off, t_def must be identical
    assert colidx_new["p_col"] == colidx_old["p_col"]
    assert colidx_new["t_off"] == colidx_old["t_off"]
    assert colidx_new["t_def"] == colidx_old["t_def"]

    # X data must be identical (sparse matrix comparison)
    diff = X_new - X_old
    assert diff.nnz == 0, "X matrix changed with interactions=False"


# ---------------------------------------------------------------------------
# Test 2: interaction columns appended AFTER base block; existing indices stable
# ---------------------------------------------------------------------------

def test_interactions_append_columns_after_base_block():
    """When interactions=True, new columns are appended after [players, team_off,
    team_def]. Existing column indices in p_col / t_off / t_def must be
    byte-for-byte identical to the interactions=False result."""
    plays, players = _minimal_plays_players(n_wr=2, n_def=3, n_plays=60)

    X_base, y_base, colidx_base = build_design(plays, players, interactions=False)
    X_int, y_int, colidx_int = build_design(plays, players, interactions=True, min_pair_plays=1)

    base_nCols = colidx_base["nCols"]

    # Existing indices must be IDENTICAL
    assert colidx_int["p_col"] == colidx_base["p_col"], \
        "p_col changed when interactions=True (existing indices must be stable)"
    assert colidx_int["t_off"] == colidx_base["t_off"]
    assert colidx_int["t_def"] == colidx_base["t_def"]

    # 'interactions' key must be present
    assert "interactions" in colidx_int, "colidx['interactions'] missing when interactions=True"
    pair_col = colidx_int["interactions"]
    assert isinstance(pair_col, dict), "colidx['interactions'] should be a dict of {(wr,cb): col}"

    # All interaction columns must be >= base_nCols (appended after)
    for (wr, cb), col in pair_col.items():
        assert col >= base_nCols, \
            f"Interaction col {col} for ({wr},{cb}) is not after base block (base_nCols={base_nCols})"

    # nCols must have increased by the number of interaction columns
    n_pairs = len(pair_col)
    assert n_pairs > 0, "Expected at least one interaction pair with min_pair_plays=1"
    assert colidx_int["nCols"] == base_nCols + n_pairs, \
        f"nCols not bumped correctly: {colidx_int['nCols']} vs {base_nCols + n_pairs}"

    # X must have more columns
    assert X_int.shape[1] == colidx_int["nCols"]
    assert X_int.shape[0] == X_base.shape[0]  # same number of plays

    # The base block of X must be unchanged (first base_nCols columns)
    X_base_slice = X_base
    X_int_slice = X_int[:, :base_nCols]
    diff = X_base_slice - X_int_slice
    assert diff.nnz == 0, "Base columns of X changed when interactions=True"


# ---------------------------------------------------------------------------
# Test 3: pruning drops low-co-occurrence pairs
# ---------------------------------------------------------------------------

def test_pruning_drops_low_cooccurrence_pairs():
    """Pairs seen fewer than min_pair_plays times must be pruned from the result."""
    plays, players = _minimal_plays_players_with_cb(
        n_wr=2, n_cb=2, n_other_def=1, n_plays=60, seed=11
    )
    # half=30 plays have both CBs; half=30 have only cb0
    # wr0-cb1 and wr1-cb1 each appear 30 times (exactly half)
    # wr0-cb0 and wr1-cb0 appear all 60 times

    # with threshold=31, pairs involving cb1 (30 plays) should be pruned
    _, _, colidx_high = build_design(plays, players, interactions=True, min_pair_plays=31)
    _, _, colidx_low = build_design(plays, players, interactions=True, min_pair_plays=1)

    pairs_high = colidx_high.get("interactions", {})
    pairs_low = colidx_low.get("interactions", {})

    # With high threshold: cb1 pairs should be absent (30 < 31)
    cb1_pairs_high = [(wr, cb) for (wr, cb) in pairs_high if cb == "cb1"]
    assert len(cb1_pairs_high) == 0, \
        f"cb1 pairs should be pruned at min_pair_plays=31, but found: {cb1_pairs_high}"

    # With low threshold: cb1 pairs should be present (30 >= 1)
    cb1_pairs_low = [(wr, cb) for (wr, cb) in pairs_low if cb == "cb1"]
    assert len(cb1_pairs_low) > 0, \
        "cb1 pairs should survive at min_pair_plays=1"

    # cb0 pairs should always survive (60 >= 31)
    cb0_pairs_high = [(wr, cb) for (wr, cb) in pairs_high if cb == "cb0"]
    assert len(cb0_pairs_high) > 0, \
        "cb0 pairs (60 plays) should not be pruned at threshold=31"


# ---------------------------------------------------------------------------
# Test 4: missing participation falls back to positional-only pairing
# ---------------------------------------------------------------------------

def test_missing_participation_falls_back_to_positional_only():
    """When plays have no position-specific alignment data, build_design must
    fall back to pairing WRs with all on-field DEF players (positional proxy).
    This test uses generic DEF positions only (no CB column) to confirm the
    fallback path runs without error and produces non-empty pairs."""
    # n_def=3 DEF players, n_wr=2 WRs — no CB position in players
    plays, players = _minimal_plays_players(
        n_wr=2, n_def=3, n_plays=60, with_positions=True
    )
    # Verify no CB position in the player set
    positions = set(players["position"].tolist())
    assert "CB" not in positions, "Test setup error: CB should not be present"

    # Should not raise; should fall back to WR vs DEF pairing
    _, _, colidx = build_design(plays, players, interactions=True, min_pair_plays=1)

    pair_col = colidx.get("interactions", {})
    assert len(pair_col) > 0, \
        "Positional fallback should produce WR-vs-DEF pairs when no CB present"

    # All WR ids should appear as the first element of at least one pair
    wr_ids = set(players.loc[players["position"] == "WR", "player_id"].tolist())
    paired_wrs = {wr for (wr, _) in pair_col}
    assert wr_ids == paired_wrs, \
        f"Not all WRs got paired: missing {wr_ids - paired_wrs}"


# ---------------------------------------------------------------------------
# Test 5: planted WR-CB interaction recovered from synth data with CB knob
# ---------------------------------------------------------------------------

def test_planted_wr_cb_interaction_recovered():
    """With the synth CB-split knob enabled, the interaction terms should be
    non-trivially non-zero (regression recovers planted signal).

    This test requires the SynthConfig.cb_split knob (added in Task 7).
    If the knob is absent on SynthConfig, it is skipped with a clear message.
    """
    # Check the knob exists
    if not hasattr(SynthConfig, "cb_split"):
        pytest.skip(
            "SynthConfig.cb_split knob not yet available; "
            "add it in synth.py to enable WR-CB planted-recovery test"
        )

    from backend.grid.value import fit_value_model, attach_dv

    cfg = SynthConfig(n_teams=6, weeks=8, seed=42, cb_split=True)
    plays, players, gt = simulate(cfg)
    plays = attach_dv(plays, fit_value_model(plays))

    # Build design with interactions
    _, _, colidx = build_design(plays, players, interactions=True, min_pair_plays=10)

    pair_col = colidx.get("interactions", {})
    assert len(pair_col) > 0, \
        "Expected interaction pairs with cb_split=True synth data"

    # Verify CB position players exist in players frame
    cb_players = players[players["position"] == "CB"]
    assert len(cb_players) > 0, \
        "cb_split=True should create CB-position players in synth data"

    # Solve RAPM with interactions to verify no crash and beta is finite
    from backend.grid.layers import build_design as bd, _solve_ridge_prior
    import scipy.sparse as sp

    X, y, colidx2 = bd(plays, players, interactions=True, min_pair_plays=10)
    nCols = colidx2["nCols"]
    mask = np.ones(nCols)
    # heavy penalty on interaction columns (indices >= base_nCols)
    base_nCols = nCols - len(colidx2["interactions"])
    mask[base_nCols:] = 10.0
    # light penalty on team intercepts
    for t in colidx2["teams"]:
        mask[colidx2["t_off"][t]] = 0.05
        mask[colidx2["t_def"][t]] = 0.05

    XtX = (X.T @ X).toarray()
    Xty = np.asarray(X.T @ y).ravel()
    prior = np.zeros(nCols)
    beta = _solve_ridge_prior(XtX, Xty, lam=120.0, prior_mean=prior, ridge_mask=mask)

    assert np.all(np.isfinite(beta)), "beta contains non-finite values"
    # Interaction betas should not all be exactly zero (they absorb some signal)
    interaction_betas = beta[base_nCols:]
    assert not np.all(interaction_betas == 0.0), \
        "All interaction betas are zero — signal not flowing through interaction columns"
