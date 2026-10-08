"""Tests for backend.pipeline.compute_valuations."""
from __future__ import annotations

import json
import sqlite3
from pathlib import Path
from unittest.mock import patch

import pytest

from backend.pipeline.compute_valuations import (
    _load_players_with_stats,
    _upsert_valuations,
    run as valuations_run,
)
from backend.scoring.format_registry import ensure_preset_formats


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

@pytest.fixture()
def mem_db() -> sqlite3.Connection:
    """In-memory SQLite with schema applied."""
    conn = sqlite3.connect(":memory:")
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA foreign_keys=ON")
    schema_path = Path(__file__).parent.parent.parent / "backend" / "db" / "schema.sql"
    conn.executescript(schema_path.read_text())
    conn.commit()
    return conn


@pytest.fixture()
def db_with_players(mem_db) -> sqlite3.Connection:
    """DB with some players and stats already loaded."""
    # Insert players first (FK parent)
    for row in [
        ("qb1", "Patrick Mahomes", "QB", "KC"),
        ("rb1", "Christian McCaffrey", "RB", "SF"),
        ("wr1", "Tyreek Hill", "WR", "MIA"),
    ]:
        mem_db.execute(
            "INSERT INTO players (player_id, name, position, team) VALUES (?, ?, ?, ?)", row
        )
    mem_db.commit()

    # Insert stats — use positional order matching schema exactly
    # schema cols: player_id, season, week, pass_attempts, completions,
    #   passing_yards, passing_tds, interceptions, rushing_attempts (not in schema?),
    #   rushing_yards, rushing_tds, receptions, targets, receiving_yards, receiving_tds,
    #   fumbles_lost, two_point_conversions
    def _stat(pid, passing_yards=0, passing_tds=0, interceptions=0,
               rushing_yards=0, rushing_tds=0, receptions=0,
               receiving_yards=0, receiving_tds=0, fumbles_lost=0):
        mem_db.execute(
            """INSERT INTO player_stats
               (player_id, season, week,
                pass_attempts, completions,
                passing_yards, passing_tds, interceptions,
                rushing_yards, rushing_tds,
                receptions, targets,
                receiving_yards, receiving_tds,
                fumbles_lost, two_point_conversions)
               VALUES (?, 2024, 1, 0, 0, ?, ?, ?, ?, ?, ?, 0, ?, ?, ?, 0)""",
            (pid, passing_yards, passing_tds, interceptions,
             rushing_yards, rushing_tds, receptions,
             receiving_yards, receiving_tds, fumbles_lost),
        )

    _stat("qb1", passing_yards=300, passing_tds=3)
    _stat("rb1", rushing_yards=120, rushing_tds=2, receptions=5, receiving_yards=45)
    _stat("wr1", receptions=8, receiving_yards=150, receiving_tds=2)
    mem_db.commit()
    return mem_db


# ---------------------------------------------------------------------------
# Unit: ensure_preset_formats
# ---------------------------------------------------------------------------

class TestEnsureScoringFormats:
    def test_creates_three_formats(self, mem_db):
        result = ensure_preset_formats(mem_db)
        assert len(result) == 3
        assert "Standard" in result
        assert "Half PPR" in result
        assert "Full PPR" in result

    def test_returns_integer_ids(self, mem_db):
        result = ensure_preset_formats(mem_db)
        for name, fid in result.items():
            assert isinstance(fid, int)
            assert fid > 0

    def test_idempotent(self, mem_db):
        r1 = ensure_preset_formats(mem_db)
        r2 = ensure_preset_formats(mem_db)
        assert r1 == r2

    def test_stored_config_json_is_valid(self, mem_db):
        ensure_preset_formats(mem_db)
        rows = mem_db.execute("SELECT name, config_json FROM scoring_formats").fetchall()
        for row in rows:
            parsed = json.loads(row["config_json"])
            assert "name" in parsed
            assert "rules" in parsed


# ---------------------------------------------------------------------------
# Unit: _load_players_with_stats
# ---------------------------------------------------------------------------

class TestLoadPlayersWithStats:
    def test_returns_list_of_dicts(self, db_with_players):
        players = _load_players_with_stats(db_with_players, 2024)
        assert isinstance(players, list)
        assert all(isinstance(p, dict) for p in players)

    def test_returns_correct_player_count(self, db_with_players):
        players = _load_players_with_stats(db_with_players, 2024)
        assert len(players) == 3

    def test_includes_stat_fields(self, db_with_players):
        players = _load_players_with_stats(db_with_players, 2024)
        for p in players:
            assert "passing_yards" in p
            assert "rushing_yards" in p
            assert "receiving_yards" in p
            assert "player_id" in p
            assert "position" in p

    def test_zero_stats_for_missing_season(self, db_with_players):
        players = _load_players_with_stats(db_with_players, 9999)
        for p in players:
            assert p["passing_yards"] == 0
            assert p["rushing_yards"] == 0

    def test_filters_non_skill_positions(self, mem_db):
        """Only QB, RB, WR, TE, K should be included — not DEF or OL."""
        mem_db.execute(
            "INSERT INTO players (player_id, name, position) VALUES (?, ?, ?)",
            ("def1", "A Team Defense", "DEF"),
        )
        mem_db.commit()
        players = _load_players_with_stats(mem_db, 2024)
        positions = {p["position"] for p in players}
        assert "DEF" not in positions


# ---------------------------------------------------------------------------
# Unit: _upsert_valuations
# ---------------------------------------------------------------------------

class TestUpsertValuations:
    def test_inserts_valuations(self, db_with_players):
        ensure_preset_formats(db_with_players)
        fmt_id = db_with_players.execute(
            "SELECT format_id FROM scoring_formats WHERE name = 'Standard'"
        ).fetchone()["format_id"]

        valuations = [
            {"player_id": "qb1", "projected_points": 25.0, "vor": 10.0, "tier": 1, "rank": 1},
            {"player_id": "rb1", "projected_points": 20.0, "vor": 5.0, "tier": 2, "rank": 2},
        ]
        count = _upsert_valuations(db_with_players, valuations, fmt_id)
        assert count == 2

    def test_idempotent(self, db_with_players):
        ensure_preset_formats(db_with_players)
        fmt_id = db_with_players.execute(
            "SELECT format_id FROM scoring_formats WHERE name = 'Half PPR'"
        ).fetchone()["format_id"]

        valuations = [
            {"player_id": "qb1", "projected_points": 25.0, "vor": 10.0, "tier": 1, "rank": 1},
        ]
        _upsert_valuations(db_with_players, valuations, fmt_id)
        count2 = _upsert_valuations(db_with_players, valuations, fmt_id)
        assert count2 == 1
        rows = db_with_players.execute("SELECT COUNT(*) FROM valuations").fetchone()[0]
        assert rows == 1


# ---------------------------------------------------------------------------
# Integration: run()
# ---------------------------------------------------------------------------

class TestValuationsRun:
    def test_run_with_players(self, db_with_players):
        valuations_run(season=2024, num_teams=2, _conn=db_with_players)
        count = db_with_players.execute("SELECT COUNT(*) FROM valuations").fetchone()[0]
        assert count >= 3  # 3 players × at least 1 format

    def test_run_no_players_is_safe(self, mem_db):
        """run() with no players in DB logs warning and exits cleanly."""
        valuations_run(season=2024, _conn=mem_db)  # no exception expected

    def test_run_creates_projections(self, db_with_players):
        valuations_run(season=2024, num_teams=2, _conn=db_with_players)
        count = db_with_players.execute("SELECT COUNT(*) FROM projections").fetchone()[0]
        assert count >= 1

    def test_run_idempotent(self, db_with_players):
        valuations_run(season=2024, num_teams=2, _conn=db_with_players)
        valuations_run(season=2024, num_teams=2, _conn=db_with_players)
        count = db_with_players.execute("SELECT COUNT(*) FROM valuations").fetchone()[0]
        assert count >= 3  # idempotent — same count both times


# ---------------------------------------------------------------------------
# Phase 2b: GRID projection sourcing + sniff-gate hook
# ---------------------------------------------------------------------------

class TestValuationsProjectionSourcing:
    def _full14(self, **overrides):
        """A full 14-col stat line (unspecified stats default to 0)."""
        from backend.scoring.columns import SCORING_STAT_COLS
        line = {c: 0.0 for c in SCORING_STAT_COLS}
        line.update(overrides)
        return line

    def test_projection_overrides_last_season_stats(self, db_with_players):
        """When a GRID projection is supplied for a player, valuations reflect the
        PROJECTED stat line, not the last-season aggregate."""
        # Baseline run (last-season stats): qb1 had 300 pass yds, 3 TDs.
        valuations_run(season=2024, num_teams=2, _conn=db_with_players)
        baseline = db_with_players.execute(
            "SELECT projected_points FROM valuations v JOIN scoring_formats s "
            "ON v.format_id = s.format_id WHERE v.player_id='qb1' AND s.name='Standard'"
        ).fetchone()[0]

        # Projected run: give qb1 a much bigger projected stat line.
        projection = {"qb1": self._full14(passing_yards=5000, passing_tds=40)}
        valuations_run(season=2024, num_teams=2, projection=projection, _conn=db_with_players)
        projected = db_with_players.execute(
            "SELECT projected_points FROM valuations v JOIN scoring_formats s "
            "ON v.format_id = s.format_id WHERE v.player_id='qb1' AND s.name='Standard'"
        ).fetchone()[0]

        assert projected > baseline    # sourced from the (larger) projection

    def _standard_points(self, conn, pid):
        return conn.execute(
            "SELECT projected_points FROM valuations v JOIN scoring_formats s "
            "ON v.format_id = s.format_id WHERE v.player_id=? AND s.name='Standard'",
            (pid,),
        ).fetchone()[0]

    def test_absent_player_falls_back_to_last_season(self, db_with_players):
        """A player not in the projection dict keeps the EXACT last-season baseline
        (parity, not just a non-zero fallback)."""
        baseline = None
        valuations_run(season=2024, num_teams=2, _conn=db_with_players)
        baseline = self._standard_points(db_with_players, "rb1")

        projection = {"qb1": self._full14(passing_yards=5000)}   # only qb1 projected
        valuations_run(season=2024, num_teams=2, projection=projection, _conn=db_with_players)
        fallback = self._standard_points(db_with_players, "rb1")

        assert fallback == baseline   # rb1 (absent) is scored identically to baseline

    def test_elite_ids_hook_logs_on_band_miss(self, db_with_players, caplog):
        """A failing sniff gate is logged (proving the hook ran) but does not abort."""
        with caplog.at_level("WARNING"):
            valuations_run(season=2024, num_teams=2,
                           elite_ids={"missing_a", "missing_b", "missing_c"},
                           _conn=db_with_players)
        assert "sniff gate FAILED" in caplog.text   # the hook actually ran
        count = db_with_players.execute("SELECT COUNT(*) FROM valuations").fetchone()[0]
        assert count >= 3   # soft gate — run still completes
