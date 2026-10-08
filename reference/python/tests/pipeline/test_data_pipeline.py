"""Tests for backend.pipeline.data_pipeline."""
from __future__ import annotations

import sqlite3
from pathlib import Path
from unittest.mock import patch

import pandas as pd
import pytest

from backend.db.connection import init_db
from backend.pipeline.data_pipeline import (
    _aggregate_stats_from_pbp,
    _upsert_players,
    _upsert_stats,
    run as pipeline_run,
)


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
def sample_roster() -> pd.DataFrame:
    return pd.DataFrame(
        [
            {"player_id": "p1", "name": "Alice QB", "team": "KC", "position": "QB"},
            {"player_id": "p2", "name": "Bob RB", "team": "DAL", "position": "RB"},
            {"player_id": "p3", "name": "Carol WR", "team": "NYG", "position": "WR"},
        ]
    )


@pytest.fixture()
def sample_pbp() -> pd.DataFrame:
    """Minimal normalized PBP dataframe (post _normalize_pbp shape)."""
    return pd.DataFrame(
        [
            # Alice QB: 2 pass attempts, 200 yards, 1 TD
            {
                "game_id": "g1",
                "play_id": 1,
                "week": 1,
                "off_team": "KC",
                "def_team": "DEN",
                "down": 1,
                "ydstogo": 10,
                "yardline_100": 50,
                "yards_gained": 15,
                "pass_attempt": 1,
                "rush_attempt": 0,
                "passer_id": "p1",
                "receiver_id": "p3",
                "rusher_id": None,
                "air_yards": 10,
                "yards_after_catch": 5,
                "td_type": None,
                "fumble": 0,
                "interception": 0,
                "td_player_id": None,
                "complete_pass": 1,
            },
            {
                "game_id": "g1",
                "play_id": 2,
                "week": 1,
                "off_team": "KC",
                "def_team": "DEN",
                "down": 2,
                "ydstogo": 5,
                "yardline_100": 35,
                "yards_gained": 35,
                "pass_attempt": 1,
                "rush_attempt": 0,
                "passer_id": "p1",
                "receiver_id": "p3",
                "rusher_id": None,
                "air_yards": 20,
                "yards_after_catch": 15,
                "td_type": "pass",
                "fumble": 0,
                "interception": 0,
                "td_player_id": "p3",
                "complete_pass": 1,
            },
            # Bob RB: 1 rush, 10 yards
            {
                "game_id": "g1",
                "play_id": 3,
                "week": 1,
                "off_team": "DAL",
                "def_team": "WAS",
                "down": 1,
                "ydstogo": 10,
                "yardline_100": 40,
                "yards_gained": 10,
                "pass_attempt": 0,
                "rush_attempt": 1,
                "passer_id": None,
                "receiver_id": None,
                "rusher_id": "p2",
                "air_yards": None,
                "yards_after_catch": None,
                "td_type": None,
                "fumble": 0,
                "interception": 0,
                "td_player_id": None,
                "complete_pass": 0,
            },
        ]
    )


# ---------------------------------------------------------------------------
# Unit tests: _aggregate_stats_from_pbp
# ---------------------------------------------------------------------------

class TestAggregateStatsFromPbp:
    def test_returns_dataframe(self, sample_pbp):
        result = _aggregate_stats_from_pbp(sample_pbp, season=2024)
        assert isinstance(result, pd.DataFrame)

    def test_has_required_columns(self, sample_pbp):
        result = _aggregate_stats_from_pbp(sample_pbp, season=2024)
        required = {"player_id", "season", "week", "passing_yards", "rushing_yards"}
        assert required.issubset(result.columns)

    def test_season_column_set(self, sample_pbp):
        result = _aggregate_stats_from_pbp(sample_pbp, season=2024)
        assert (result["season"] == 2024).all()

    def test_empty_pbp_returns_empty(self):
        empty = pd.DataFrame(
            columns=[
                "game_id", "play_id", "week", "off_team", "def_team", "down",
                "ydstogo", "yardline_100", "yards_gained", "pass_attempt",
                "rush_attempt", "passer_id", "receiver_id", "rusher_id",
                "air_yards", "yards_after_catch", "td_type", "fumble",
                "interception", "td_player_id", "complete_pass",
            ]
        )
        result = _aggregate_stats_from_pbp(empty, season=2024)
        assert result.empty or len(result) == 0


# ---------------------------------------------------------------------------
# Unit tests: _upsert_players
# ---------------------------------------------------------------------------

class TestUpsertPlayers:
    def test_inserts_players(self, mem_db, sample_roster):
        count = _upsert_players(mem_db, sample_roster)
        assert count == 3

    def test_idempotent(self, mem_db, sample_roster):
        _upsert_players(mem_db, sample_roster)
        count2 = _upsert_players(mem_db, sample_roster)
        assert count2 == 3
        rows = mem_db.execute("SELECT COUNT(*) FROM players").fetchone()[0]
        assert rows == 3

    def test_skips_missing_player_id(self, mem_db):
        import numpy as np
        df = pd.DataFrame([{"player_id": np.nan, "name": "Ghost", "position": "QB"}])
        count = _upsert_players(mem_db, df)
        assert count == 0

    def test_skips_missing_name(self, mem_db):
        df = pd.DataFrame([{"player_id": "x1", "name": "", "position": "QB"}])
        count = _upsert_players(mem_db, df)
        assert count == 0

    def test_updates_existing_player(self, mem_db, sample_roster):
        _upsert_players(mem_db, sample_roster)
        updated = pd.DataFrame(
            [{"player_id": "p1", "name": "Alice QB Updated", "team": "SF", "position": "QB"}]
        )
        _upsert_players(mem_db, updated)
        row = mem_db.execute("SELECT name, team FROM players WHERE player_id = 'p1'").fetchone()
        assert row["name"] == "Alice QB Updated"
        assert row["team"] == "SF"


# ---------------------------------------------------------------------------
# Unit tests: _upsert_stats
# ---------------------------------------------------------------------------

class TestUpsertStats:
    def test_inserts_stats(self, mem_db, sample_roster):
        _upsert_players(mem_db, sample_roster)
        stats = pd.DataFrame(
            [
                {
                    "player_id": "p1",
                    "season": 2024,
                    "week": 1,
                    "passing_yards": 200,
                    "passing_tds": 2,
                    "interceptions": 0,
                    "rushing_yards": 0,
                    "rushing_tds": 0,
                    "receptions": 0,
                    "receiving_yards": 0,
                    "receiving_tds": 0,
                    "fumbles_lost": 0,
                    "pass_attempts": 5,
                }
            ]
        )
        count = _upsert_stats(mem_db, stats)
        assert count == 1

    def test_skips_unknown_player(self, mem_db):
        stats = pd.DataFrame(
            [{"player_id": "unknown99", "season": 2024, "week": 1, "passing_yards": 300}]
        )
        count = _upsert_stats(mem_db, stats)
        assert count == 0

    def test_idempotent(self, mem_db, sample_roster):
        _upsert_players(mem_db, sample_roster)
        stats = pd.DataFrame(
            [{"player_id": "p2", "season": 2024, "week": 1, "rushing_yards": 100}]
        )
        _upsert_stats(mem_db, stats)
        count2 = _upsert_stats(mem_db, stats)
        assert count2 == 1
        rows = mem_db.execute("SELECT COUNT(*) FROM player_stats").fetchone()[0]
        assert rows == 1


# ---------------------------------------------------------------------------
# Integration test: run() with mocked loaders
# ---------------------------------------------------------------------------

class TestPipelineRun:
    def _make_db(self, tmp_path) -> sqlite3.Connection:
        """Create and return a schema-initialized in-memory SQLite connection."""
        conn = sqlite3.connect(":memory:")
        conn.row_factory = sqlite3.Row
        conn.execute("PRAGMA foreign_keys=ON")
        schema_path = Path(__file__).parent.parent.parent / "backend" / "db" / "schema.sql"
        conn.executescript(schema_path.read_text())
        conn.commit()
        return conn

    def _empty_pbp(self) -> pd.DataFrame:
        return pd.DataFrame(
            columns=[
                "game_id", "play_id", "week", "off_team", "def_team", "down",
                "ydstogo", "yardline_100", "yards_gained", "pass_attempt",
                "rush_attempt", "passer_id", "receiver_id", "rusher_id",
                "air_yards", "yards_after_catch", "td_type", "fumble",
                "interception", "td_player_id", "complete_pass",
            ]
        )

    def test_run_inserts_players(self, tmp_path):
        """run() inserts players into the DB."""
        conn = self._make_db(tmp_path)
        roster_df = pd.DataFrame(
            [{"player_id": "p1", "name": "Test Player", "team": "KC", "position": "QB"}]
        )
        with (
            patch("backend.pipeline.data_pipeline.load_rosters", return_value=roster_df),
            patch("backend.pipeline.data_pipeline.load_pbp", return_value=self._empty_pbp()),
        ):
            pipeline_run(years=[2024], cache_dir=str(tmp_path / "cache"), _conn=conn)

        count = conn.execute("SELECT COUNT(*) FROM players").fetchone()[0]
        assert count >= 1

    def test_run_idempotent(self, tmp_path):
        """run() can be called twice without duplicating data."""
        conn = self._make_db(tmp_path)
        roster_df = pd.DataFrame(
            [{"player_id": "p1", "name": "Test Player", "team": "KC", "position": "QB"}]
        )
        with (
            patch("backend.pipeline.data_pipeline.load_rosters", return_value=roster_df),
            patch("backend.pipeline.data_pipeline.load_pbp", return_value=self._empty_pbp()),
        ):
            pipeline_run(years=[2024], cache_dir=str(tmp_path / "cache"), _conn=conn)
            pipeline_run(years=[2024], cache_dir=str(tmp_path / "cache"), _conn=conn)

        count = conn.execute("SELECT COUNT(*) FROM players").fetchone()[0]
        assert count == 1  # no duplicates

    def test_run_handles_loader_failure_gracefully(self, tmp_path):
        """run() logs and continues when a loader raises."""
        conn = self._make_db(tmp_path)
        with patch(
            "backend.pipeline.data_pipeline.load_rosters",
            side_effect=Exception("Network failure"),
        ):
            pipeline_run(years=[2024], cache_dir=str(tmp_path / "cache"), _conn=conn)
            # no crash expected
