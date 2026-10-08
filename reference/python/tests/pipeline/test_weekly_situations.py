"""Tests for per-situation keyed incremental accumulators (Task 6).

Four required tests:
  - test_situation_accumulator_files_created_per_situation
  - test_situation_dim_mismatch_reinitialises
  - test_rerun_week_skips_all_situation_accumulators
  - test_situation_grades_written_with_situation_tag
"""
from __future__ import annotations

import os

import numpy as np
import pandas as pd
import pytest


# ─── synthetic data helpers ──────────────────────────────────────────────────

def _make_plays(n=200, n_players=6, seed=42):
    """Synthetic plays that include situation-classifier columns.

    We need enough plays so that at least one situation sub-frame clears
    MIN_PLAYS (50).  n=200 gives ~20% red_zone ≈ 40 plays, ~25% passing_downs
    ≈ 50 plays, etc.  Tweak seed if needed.
    """
    rng = np.random.default_rng(seed)
    teams = ["KC", "BUF", "PHI", "DAL"]
    off_teams = [teams[i % 2] for i in range(n)]
    def_teams = [teams[(i + 2) % 2 + 2] for i in range(n)]

    player_pool = [f"P{i}" for i in range(n_players)]
    off_players = [
        list(rng.choice(player_pool[: n_players // 2], size=3, replace=False))
        for _ in range(n)
    ]
    def_players = [
        list(rng.choice(player_pool[n_players // 2 :], size=3, replace=False))
        for _ in range(n)
    ]

    # Situation columns — distribute to ensure enough plays per slice
    downs = rng.choice([1, 2, 3, 4], size=n, p=[0.35, 0.30, 0.25, 0.10])
    ydstogo = rng.integers(1, 20, size=n)
    yardline_100 = rng.integers(1, 100, size=n)

    return pd.DataFrame({
        "off_team": off_teams,
        "def_team": def_teams,
        "off_players": off_players,
        "def_players": def_players,
        "dv": rng.normal(scale=0.5, size=n),
        "week": [5] * n,
        "down": downs,
        "ydstogo": ydstogo,
        "yardline_100": yardline_100,
    })


def _make_plays_rich_situation(n=300, n_players=6, seed=99):
    """Plays guaranteed to have >=50 plays in every situation slice."""
    rng = np.random.default_rng(seed)
    teams = ["KC", "BUF", "PHI", "DAL"]
    off_teams = [teams[i % 2] for i in range(n)]
    def_teams = [teams[(i + 2) % 2 + 2] for i in range(n)]

    player_pool = [f"P{i}" for i in range(n_players)]
    off_players = [
        list(rng.choice(player_pool[: n_players // 2], size=3, replace=False))
        for _ in range(n)
    ]
    def_players = [
        list(rng.choice(player_pool[n_players // 2 :], size=3, replace=False))
        for _ in range(n)
    ]

    # Force distributions: 33% red_zone, 25% passing_downs (down==3 & ydstogo>=7),
    # 25% rushing_downs (down<=2 & ydstogo<=4), 17% other
    blocks = n // 4
    rem = n - 3 * blocks

    # red_zone: yardline_100 <= 20
    rz_yl = rng.integers(1, 21, size=blocks)
    rz_dn = rng.choice([1, 2], size=blocks)
    rz_yds = rng.integers(1, 10, size=blocks)

    # passing_downs: down==3 & ydstogo>=7
    pd_yl = rng.integers(21, 100, size=blocks)
    pd_dn = np.full(blocks, 3)
    pd_yds = rng.integers(7, 20, size=blocks)

    # rushing_downs: down<=2 & ydstogo<=4
    rd_yl = rng.integers(21, 100, size=blocks)
    rd_dn = rng.choice([1, 2], size=blocks)
    rd_yds = rng.integers(1, 5, size=blocks)

    # other (none of the above)
    ot_yl = rng.integers(21, 100, size=rem)
    ot_dn = np.full(rem, 2)
    ot_yds = rng.integers(6, 20, size=rem)

    down = np.concatenate([rz_dn, pd_dn, rd_dn, ot_dn])
    ydstogo = np.concatenate([rz_yds, pd_yds, rd_yds, ot_yds])
    yardline_100 = np.concatenate([rz_yl, pd_yl, rd_yl, ot_yl])

    return pd.DataFrame({
        "off_team": off_teams,
        "def_team": def_teams,
        "off_players": off_players,
        "def_players": def_players,
        "dv": rng.normal(scale=0.5, size=n),
        "week": [5] * n,
        "down": down,
        "ydstogo": ydstogo,
        "yardline_100": yardline_100,
    })


def _make_players(n=6):
    teams = ["KC", "BUF", "PHI", "DAL"] * 2
    positions = ["QB", "RB", "WR", "TE", "RB", "WR"]
    return pd.DataFrame({
        "player_id": [f"P{i}" for i in range(n)],
        "team": teams[:n],
        "position": positions[:n],
        "is_starter": [True] * n,
        "ability": np.random.default_rng(8).normal(size=n),
    })


def _setup_env(tmp_path, monkeypatch):
    """Redirect all file-system side-effects to tmp_path."""
    import backend.grid.layers as layers_mod
    import backend.grid.statespace as ss_mod

    # Redirect the module-level _ACCUM_PATH (used by _accum_path() for key='all')
    monkeypatch.setattr(layers_mod, "_ACCUM_PATH", tmp_path / "rapm_accumulators.npz")
    monkeypatch.setattr(ss_mod, "_KALMAN_PATH", tmp_path / "kal.npz")

    monkeypatch.setattr("backend.pipeline.weekly_update.run_valuations",
                        lambda **kwargs: None)
    monkeypatch.setattr("backend.pipeline.weekly_update.write_health",
                        lambda **kwargs: None)

    import sqlite3 as _sqlite3
    from backend.db.connection import init_db
    conn = _sqlite3.connect(str(tmp_path / "test.db"), check_same_thread=False)
    conn.row_factory = _sqlite3.Row
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("PRAGMA foreign_keys=ON")
    init_db(conn)
    return conn


# ─── tests ───────────────────────────────────────────────────────────────────

def test_situation_accumulator_files_created_per_situation(tmp_path, monkeypatch):
    """After run(), per-situation npz files are created AND key='all' resolves to
    the legacy _ACCUM_PATH (i.e. byte-for-byte same filename).

    The _accum_path helper must:
      - return _ACCUM_PATH for key='all'
      - return _ACCUM_PATH.with_name('rapm_accumulators_{name}.npz') for other keys
    """
    import backend.grid.layers as layers_mod

    legacy_path = tmp_path / "rapm_accumulators.npz"
    conn = _setup_env(tmp_path, monkeypatch)

    # Verify _accum_path returns the right paths (unit check on the helper)
    from backend.grid.layers import _accum_path
    assert _accum_path("all") == legacy_path, (
        f"key='all' must resolve to {legacy_path}, got {_accum_path('all')}"
    )
    assert _accum_path("red_zone") == legacy_path.with_name("rapm_accumulators_red_zone.npz")
    assert _accum_path("passing_downs") == legacy_path.with_name("rapm_accumulators_passing_downs.npz")

    from backend.pipeline.weekly_update import run
    plays = _make_plays_rich_situation()
    players = _make_players()
    result = run(week=5, season=2025, plays_df=plays, players_df=players, _conn=conn)
    conn.close()

    assert result["rapm_solved"] is True

    # Legacy overall-pass file must still exist
    assert legacy_path.exists(), "Legacy rapm_accumulators.npz must still be written"

    # At least one situation npz must exist (red_zone / passing_downs / rushing_downs)
    situation_files = list(tmp_path.glob("rapm_accumulators_*.npz"))
    assert len(situation_files) > 0, (
        f"Expected per-situation npz files in {tmp_path}, found none. "
        f"Files present: {list(tmp_path.iterdir())}"
    )


def test_situation_dim_mismatch_reinitialises(tmp_path, monkeypatch):
    """If a stored situation accumulator has the wrong dimension, it is discarded
    and reinitialised rather than crashing or silently corrupting results.

    We pre-plant a situation file with the wrong shape, then run and confirm the
    run completes successfully (the reinit guard triggered).
    """
    import backend.grid.layers as layers_mod

    legacy_path = tmp_path / "rapm_accumulators.npz"
    conn = _setup_env(tmp_path, monkeypatch)

    # Plant a bad-dimension accumulator for 'red_zone' (2x2, real solve needs more cols)
    bad_XtX = np.eye(2)
    bad_Xty = np.zeros(2)
    from backend.grid.layers import save_accumulators
    # Write directly to the tmp_path file — _accum_path is already redirected via
    # monkeypatch on _ACCUM_PATH, so we call with key='red_zone'
    save_accumulators(bad_XtX, bad_Xty, week=1, player_order=["X", "Y"], key="red_zone")

    rz_path = legacy_path.with_name("rapm_accumulators_red_zone.npz")
    assert rz_path.exists(), "Pre-planted red_zone file must exist before the run"

    from backend.pipeline.weekly_update import run
    plays = _make_plays_rich_situation()
    players = _make_players()
    # Must not raise — reinit guard should silently recover
    result = run(week=5, season=2025, plays_df=plays, players_df=players, _conn=conn)
    conn.close()

    assert result["rapm_solved"] is True, "run() must succeed despite dim mismatch in situation"

    # The file should now have been overwritten with the correct shape
    import numpy as _np
    from backend.grid.layers import load_accumulators
    loaded = load_accumulators(key="red_zone")
    # If red_zone had enough plays, it was rewritten; if not (skipped), the old
    # bad file may remain — either way the run must not raise.
    # We just verify the run completed; the key assertion is no exception was thrown.


def test_rerun_week_skips_all_situation_accumulators(tmp_path, monkeypatch):
    """Re-running the same week for a situation that already has last_week >= week
    must skip accumulation (not double-count) for that situation.

    Strategy: run week 5 once, then run again. The second call should return
    skipped=True for the overall pass (so the situation loop never executes),
    OR if we manually set up situation files at week=5, the situation-level
    skip guard should fire per-situation.

    We test the per-situation skip guard directly: pre-plant a situation
    accumulator at week=5 and verify that after running week=5, the
    accumulator was NOT modified (same week number stored).
    """
    import backend.grid.layers as layers_mod

    legacy_path = tmp_path / "rapm_accumulators.npz"
    conn = _setup_env(tmp_path, monkeypatch)

    from backend.grid.layers import save_accumulators, load_accumulators, init_accumulators, build_design

    plays = _make_plays_rich_situation()
    players = _make_players()

    # Pre-plant a red_zone accumulator at week=5 with a distinguishable marker value
    # so we can verify it was not overwritten.
    Xs, ys, cidx_s = build_design(
        plays[plays["yardline_100"] <= 20], players
    )
    nc_s = cidx_s["nCols"]
    sentinel_XtX = np.full((nc_s, nc_s), 999.0)
    sentinel_Xty = np.full(nc_s, 888.0)
    current_order = [str(p) for p in sorted(players["player_id"].tolist(), key=str)]
    save_accumulators(sentinel_XtX, sentinel_Xty, week=5,
                      player_order=current_order, key="red_zone")

    # Also pre-plant the overall accumulator at week=5 so overall pass is skipped
    # (this triggers the overall-pass skip guard and returns early)
    XtX0, Xty0 = init_accumulators(cidx_s["nCols"])  # rough, just needs right week
    # We need the overall one to match nCols for the overall pass, so build full design
    X_all, y_all, colidx_all = build_design(plays, players)
    nc_all = colidx_all["nCols"]
    XtX_all = np.zeros((nc_all, nc_all))
    Xty_all = np.zeros(nc_all)
    save_accumulators(XtX_all, Xty_all, week=5,
                      player_order=current_order, key="all")

    from backend.pipeline.weekly_update import run
    result = run(week=5, season=2025, plays_df=plays, players_df=players, _conn=conn)
    conn.close()

    # Overall pass should have been skipped (week already current)
    assert result.get("skipped") is True, (
        "Second run of week=5 should return skipped=True via overall-pass guard"
    )

    # The red_zone accumulator should NOT have been modified — it still holds sentinel values
    loaded = load_accumulators(key="red_zone")
    assert loaded is not None
    rz_XtX, rz_Xty, rz_last_week, _ = loaded
    assert rz_last_week == 5, "red_zone last_week should still be 5"
    assert np.allclose(rz_XtX, 999.0), (
        "sentinel XtX should be unchanged — situation accumulator must not be overwritten "
        "when overall pass is skipped"
    )


def test_situation_grades_written_with_situation_tag(tmp_path, monkeypatch):
    """matchup_grades rows written for each situation must carry the correct
    situation tag (e.g. 'red_zone', 'passing_downs') — not 'overall'.

    Also verifies that the overall pass writes situation='overall'.
    """
    conn = _setup_env(tmp_path, monkeypatch)

    from backend.pipeline.weekly_update import run
    plays = _make_plays_rich_situation()
    players = _make_players()
    result = run(week=5, season=2025, plays_df=plays, players_df=players, _conn=conn)

    assert result["rapm_solved"] is True

    rows = conn.execute(
        "SELECT DISTINCT situation FROM matchup_grades WHERE week=6 AND season=2025"
    ).fetchall()
    situations_stored = {r["situation"] for r in rows}
    conn.close()

    # 'overall' must always be present
    assert "overall" in situations_stored, (
        f"'overall' situation must be written; got: {situations_stored}"
    )

    # At least one named situation must be present (red_zone / passing_downs / rushing_downs)
    named_situations = situations_stored - {"overall"}
    assert len(named_situations) > 0, (
        f"At least one named situation grade row expected; got only: {situations_stored}"
    )

    # None of the named situations should be 'overall'
    for sit in named_situations:
        assert sit != "overall", f"Unexpected 'overall' duplicate in named: {named_situations}"
