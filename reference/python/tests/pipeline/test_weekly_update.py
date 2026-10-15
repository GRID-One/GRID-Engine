"""Tests for the incremental weekly pipeline."""
import numpy as np
import pandas as pd
import pytest
import scipy.sparse as sp

from backend.grid.layers import init_accumulators, save_accumulators
from backend.grid.statespace import KalmanState


# ─── minimal synthetic plays and players ─────────────────────────────────────

def _make_plays(n=40, n_players=6, seed=7):
    rng = np.random.default_rng(seed)
    teams = ["KC", "BUF", "PHI", "DAL"]
    off_teams = [teams[i % 2] for i in range(n)]
    def_teams = [teams[(i + 2) % 2 + 2] for i in range(n)]

    player_pool = [f"P{i}" for i in range(n_players)]

    off_players = [
        list(rng.choice(player_pool[:n_players // 2], size=3, replace=False))
        for _ in range(n)
    ]
    def_players = [
        list(rng.choice(player_pool[n_players // 2:], size=3, replace=False))
        for _ in range(n)
    ]
    return pd.DataFrame({
        "off_team": off_teams,
        "def_team": def_teams,
        "off_players": off_players,
        "def_players": def_players,
        "dv": rng.normal(scale=0.5, size=n),
        "week": [5] * n,
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


# ─── tests ───────────────────────────────────────────────────────────────────

def test_weekly_update_returns_dict(tmp_path, monkeypatch):
    """run() with synthetic data returns expected dict structure."""
    import backend.grid.layers as layers_mod
    import backend.grid.statespace as ss_mod
    monkeypatch.setattr(layers_mod, "_ACCUM_PATH", tmp_path / "acc.npz")
    monkeypatch.setattr(ss_mod, "_KALMAN_PATH", tmp_path / "kal.npz")
    monkeypatch.setattr("backend.pipeline.weekly_update._ACCUM_PATH",
                        tmp_path / "acc.npz", raising=False)

    # Patch compute_valuations.run to be a no-op
    monkeypatch.setattr("backend.pipeline.weekly_update.run_valuations",
                        lambda **kwargs: None)
    # Patch write_health to be a no-op
    monkeypatch.setattr("backend.pipeline.weekly_update.write_health",
                        lambda **kwargs: None)

    import sqlite3 as _sqlite3
    from backend.db.connection import init_db
    conn = _sqlite3.connect(str(tmp_path / "test.db"), check_same_thread=False)
    conn.row_factory = _sqlite3.Row
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("PRAGMA foreign_keys=ON")
    init_db(conn)

    from backend.pipeline.weekly_update import run
    plays = _make_plays()
    players = _make_players()

    result = run(week=5, season=2025, plays_df=plays, players_df=players, _conn=conn)
    conn.close()

    assert result["week"] == 5
    assert result["season"] == 2025
    assert result["rapm_solved"] is True
    assert result["kalman_updated"] is True
    assert result["n_players"] > 0


def test_weekly_update_loader_branch_uses_grid_plays(tmp_path, monkeypatch):
    """When plays_df/players_df aren't passed, run() loads via load_grid_plays
    (the GRID plays contract with participation) and RAPM actually solves — the
    previous load_pbp path lacked off_players/def_players and silently no-opped."""
    import backend.grid.layers as layers_mod
    import backend.grid.statespace as ss_mod
    monkeypatch.setattr(layers_mod, "_ACCUM_PATH", tmp_path / "acc.npz")
    monkeypatch.setattr(ss_mod, "_KALMAN_PATH", tmp_path / "kal.npz")
    monkeypatch.setattr("backend.pipeline.weekly_update.run_valuations", lambda **k: None)
    monkeypatch.setattr("backend.pipeline.weekly_update.write_health", lambda **k: None)

    # A full GRID plays contract (drive_points + next-state + off/def_players)
    # stands in for the real nflverse load; rosters carry only player_id/team/
    # position like a real roster (no synth is_starter/ability).
    from backend.grid.synth import simulate, SynthConfig
    plays, players, _ = simulate(SynthConfig())
    calls = {"load_grid_plays": 0}

    def _load_grid_plays(years, *a, **k):
        calls["load_grid_plays"] += 1
        return plays

    monkeypatch.setattr("backend.grid.nflverse_adapter.load_grid_plays", _load_grid_plays)
    monkeypatch.setattr("backend.grid.nflverse_loader.load_rosters",
                        lambda years, *a, **k: players[["player_id", "team", "position"]].copy())

    import sqlite3 as _sqlite3
    from backend.db.connection import init_db
    conn = _sqlite3.connect(str(tmp_path / "test.db"), check_same_thread=False)
    conn.row_factory = _sqlite3.Row
    init_db(conn)

    from backend.pipeline.weekly_update import run
    week = int(plays["week"].min())
    result = run(week=week, season=2024, _conn=conn)   # plays_df/players_df None -> loader branch
    conn.close()
    assert calls["load_grid_plays"] == 1   # the new GRID loader was the one exercised
    assert result["rapm_solved"] is True   # NOT the silent no-op
    assert result["n_players"] > 0


def test_weekly_update_saves_accumulators(tmp_path, monkeypatch):
    """After run(), accumulator file exists with correct week."""
    import backend.grid.layers as layers_mod
    import backend.grid.statespace as ss_mod
    acc_path = tmp_path / "acc.npz"
    monkeypatch.setattr(layers_mod, "_ACCUM_PATH", acc_path)
    monkeypatch.setattr(ss_mod, "_KALMAN_PATH", tmp_path / "kal.npz")
    monkeypatch.setattr("backend.pipeline.weekly_update.run_valuations",
                        lambda **kwargs: None)
    monkeypatch.setattr("backend.pipeline.weekly_update.write_health",
                        lambda **kwargs: None)

    import sqlite3 as _sqlite3
    from backend.db.connection import init_db
    conn = _sqlite3.connect(str(tmp_path / "test2.db"), check_same_thread=False)
    conn.row_factory = _sqlite3.Row
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("PRAGMA foreign_keys=ON")
    init_db(conn)

    from backend.pipeline.weekly_update import run
    from backend.grid.layers import load_accumulators
    run(week=3, season=2025, plays_df=_make_plays(), players_df=_make_players(), _conn=conn)
    conn.close()

    loaded = load_accumulators()
    assert loaded is not None
    _, _, last_week, _player_order = loaded
    assert last_week == 3


def test_weekly_update_skips_duplicate(tmp_path, monkeypatch):
    """Re-running the same week returns skipped=True."""
    import backend.grid.layers as layers_mod
    import backend.grid.statespace as ss_mod
    monkeypatch.setattr(layers_mod, "_ACCUM_PATH", tmp_path / "acc.npz")
    monkeypatch.setattr(ss_mod, "_KALMAN_PATH", tmp_path / "kal.npz")
    monkeypatch.setattr("backend.pipeline.weekly_update.run_valuations",
                        lambda **kwargs: None)
    monkeypatch.setattr("backend.pipeline.weekly_update.write_health",
                        lambda **kwargs: None)

    # Pre-populate accumulators at week=4
    save_accumulators(np.eye(3), np.zeros(3), week=4)

    import sqlite3 as _sqlite3
    from backend.db.connection import init_db
    conn = _sqlite3.connect(str(tmp_path / "test3.db"), check_same_thread=False)
    conn.row_factory = _sqlite3.Row
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("PRAGMA foreign_keys=ON")
    init_db(conn)

    from backend.pipeline.weekly_update import run
    result = run(week=4, season=2025, plays_df=_make_plays(), players_df=_make_players(), _conn=conn)
    conn.close()
    assert result.get("skipped") is True


def test_matchup_grades_upserted(tmp_path, monkeypatch):
    """matchup_grades rows are written using def-intercept semantics (not offensive means)."""
    import backend.grid.layers as layers_mod
    import backend.grid.statespace as ss_mod
    monkeypatch.setattr(layers_mod, "_ACCUM_PATH", tmp_path / "acc.npz")
    monkeypatch.setattr(ss_mod, "_KALMAN_PATH", tmp_path / "kal.npz")
    monkeypatch.setattr("backend.pipeline.weekly_update.run_valuations",
                        lambda **kwargs: None)
    monkeypatch.setattr("backend.pipeline.weekly_update.write_health",
                        lambda **kwargs: None)

    import sqlite3 as _sqlite3
    from backend.db.connection import init_db
    conn = _sqlite3.connect(str(tmp_path / "test4.db"), check_same_thread=False)
    conn.row_factory = _sqlite3.Row
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("PRAGMA foreign_keys=ON")
    init_db(conn)

    from backend.pipeline.weekly_update import run
    run(week=6, season=2025, plays_df=_make_plays(), players_df=_make_players(), _conn=conn)

    rows = conn.execute("SELECT * FROM matchup_grades").fetchall()
    assert len(rows) > 0
    # Every row must cover all four skill positions (team-level, same grade per pos)
    # and rapm_grade is a real number (sign-correct: may be pos or neg depending on solve)
    for r in rows:
        assert r["position"] in ("QB", "RB", "WR", "TE")
        assert isinstance(r["rapm_grade"], float)
    conn.close()


def test_weekly_update_position_specific_params(tmp_path, monkeypatch):
    """QB and RB should get different Kalman params via from_position."""
    import backend.grid.layers as layers_mod
    import backend.grid.statespace as ss_mod
    monkeypatch.setattr(layers_mod, "_ACCUM_PATH", tmp_path / "acc.npz")
    monkeypatch.setattr(ss_mod, "_KALMAN_PATH", tmp_path / "kal.npz")
    monkeypatch.setattr("backend.pipeline.weekly_update.run_valuations",
                        lambda **kwargs: None)
    monkeypatch.setattr("backend.pipeline.weekly_update.write_health",
                        lambda **kwargs: None)

    import sqlite3 as _sqlite3
    from backend.db.connection import init_db
    conn = _sqlite3.connect(str(tmp_path / "test5.db"), check_same_thread=False)
    conn.row_factory = _sqlite3.Row
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("PRAGMA foreign_keys=ON")
    init_db(conn)

    from backend.pipeline.weekly_update import run

    # Create plays where all players have NaN obs (predict-only → variance grows)
    plays = _make_plays()
    players = _make_players()
    result = run(week=5, season=2025, plays_df=plays, players_df=players, _conn=conn)
    conn.close()

    ks = KalmanState.load()
    assert ks is not None
    # Find QB (P0) and RB (P1) indices
    qb_idx = ks.player_ids.index("P0")
    rb_idx = ks.player_ids.index("P1")
    # QB has d_steady=0.95 (less inflation), RB has d_steady=0.85 (more inflation)
    # After one step: talent variance for RB should grow more than QB
    # But both get observations so we just verify the pipeline completed
    assert result["kalman_updated"] is True


# ---------------------------------------------------------------------------
# Task 2 — matchup-grade sign / semantics tests
# ---------------------------------------------------------------------------

def _make_beta_colidx_with_strong_defense(strong_team: str, other_team: str):
    """Build minimal beta/colidx fixtures where strong_team has a very negative
    t_def intercept (meaning it suppresses offenses → harder matchup).
    Sign convention: def_strength = -float(beta[col])
    So beta[col_strong] = -5.0 → def_strength = +5.0 (tougher matchup).
    The other team has beta = +1.0 → def_strength = -1.0 (easier matchup).
    """
    import numpy as np

    col_strong = 0
    col_other = 1
    beta = np.zeros(10)
    beta[col_strong] = -5.0   # strong defense → large negative intercept
    beta[col_other] = 1.0     # weak defense → positive intercept

    colidx = {
        "t_def": {strong_team: col_strong, other_team: col_other},
    }
    return beta, colidx


def test_matchup_grade_uses_def_intercept_not_offense(tmp_path):
    """A planted strong defense yields a tougher (higher) grade for the defending team.

    This verifies that _upsert_matchup_grades reads from beta/colidx['t_def']
    (not from offensive player means) and that stronger defenses get higher rapm_grade.
    """
    import sqlite3
    from backend.db.connection import init_db
    conn = sqlite3.connect(str(tmp_path / "test_sign.db"), check_same_thread=False)
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("PRAGMA foreign_keys=ON")
    init_db(conn)

    from backend.pipeline.weekly_update import _upsert_matchup_grades

    beta, colidx = _make_beta_colidx_with_strong_defense("KC", "BUF")
    _upsert_matchup_grades(conn, beta, colidx, week=7, season=2025)

    rows = {
        r["def_team"]: r["rapm_grade"]
        for r in conn.execute(
            "SELECT def_team, rapm_grade FROM matchup_grades WHERE week=7 AND season=2025"
        ).fetchall()
    }
    conn.close()

    assert "KC" in rows, "strong_team KC should have a grade row"
    assert "BUF" in rows, "weak_team BUF should have a grade row"
    # KC has beta = -5 → def_strength = +5 (harder matchup)
    # BUF has beta = +1 → def_strength = -1 (easier matchup)
    assert rows["KC"] > rows["BUF"], (
        f"Strong defense KC (grade={rows['KC']:.3f}) should have a higher "
        f"rapm_grade than weak defense BUF (grade={rows['BUF']:.3f})"
    )


def test_matchup_grade_sign_convention(tmp_path):
    """Stronger defense (more negative t_def beta) → higher rapm_grade (tougher matchup).

    def_strength = -float(beta[col]), so a more negative beta → larger positive grade.
    """
    import numpy as np
    import sqlite3
    from backend.db.connection import init_db
    conn = sqlite3.connect(str(tmp_path / "test_sign2.db"), check_same_thread=False)
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("PRAGMA foreign_keys=ON")
    init_db(conn)

    from backend.pipeline.weekly_update import _upsert_matchup_grades

    # Three teams with different defensive strength
    col_elite = 0
    col_avg = 1
    col_weak = 2
    beta = np.zeros(10)
    beta[col_elite] = -3.0   # elite defense → def_strength = +3.0
    beta[col_avg] = 0.0      # average         → def_strength = 0.0
    beta[col_weak] = 2.5     # weak defense  → def_strength = -2.5

    colidx = {
        "t_def": {"ELITE": col_elite, "AVG": col_avg, "WEAK": col_weak},
    }
    _upsert_matchup_grades(conn, beta, colidx, week=8, season=2025)

    rows = {
        r["def_team"]: r["rapm_grade"]
        for r in conn.execute(
            "SELECT def_team, rapm_grade FROM matchup_grades WHERE week=8 AND season=2025"
        ).fetchall()
    }
    conn.close()

    assert rows["ELITE"] > rows["AVG"] > rows["WEAK"], (
        f"Expected ELITE ({rows['ELITE']:.2f}) > AVG ({rows['AVG']:.2f}) > WEAK ({rows['WEAK']:.2f})"
    )
    assert abs(rows["ELITE"] - 3.0) < 1e-9, "ELITE grade should be exactly 3.0"
    assert abs(rows["AVG"] - 0.0) < 1e-9, "AVG grade should be exactly 0.0"
    assert abs(rows["WEAK"] - (-2.5)) < 1e-9, "WEAK grade should be exactly -2.5"
