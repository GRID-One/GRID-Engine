"""Ingest nflverse play-by-play and roster data into SQLite.

Run:
    python -m backend.pipeline.data_pipeline [--years 2022 2023 2024]

Idempotent: uses INSERT OR REPLACE so repeated runs are safe.
Logs to data/logs/pipeline.log.
"""
from __future__ import annotations

import argparse
import sqlite3
import sys
from datetime import datetime
from pathlib import Path

import pandas as pd

from backend.db.connection import get_db, init_db
from backend.grid.nflverse_loader import load_pbp, load_rosters
from backend.pipeline._logging import get_logger
from backend.pipeline.health_check import write_health
from backend.scoring.columns import SCORING_STAT_COLS as stat_cols

logger = get_logger("data_pipeline")

DEFAULT_YEARS = [2024]


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _aggregate_stats_from_pbp(pbp: pd.DataFrame, season: int) -> pd.DataFrame:
    """Aggregate play-by-play rows into per-player per-week stat rows.

    PBP columns after _normalize_pbp:
      game_id, play_id, week, off_team, def_team, down, ydstogo,
      yardline_100, yards_gained, pass_attempt, rush_attempt,
      passer_id, receiver_id, rusher_id, air_yards, yards_after_catch,
      td_type, fumble, interception, td_player_id
    """
    rows: list[dict] = []

    # Helper: is this play a touchdown?
    pbp = pbp.copy()
    pbp["is_td"] = pbp["td_type"].notna()

    # --- Passing stats (passer) ---
    passers = pbp[pbp["pass_attempt"] == 1].copy()
    pass_agg = (
        passers.groupby(["passer_id", "week"])
        .agg(
            pass_attempts=("pass_attempt", "sum"),
            completions=("complete_pass", "sum"),
            passing_yards=("yards_gained", "sum"),
            passing_tds=("is_td", lambda x: x[passers.loc[x.index, "td_type"] == "pass"].sum()),
            interceptions=("interception", "sum"),
        )
        .reset_index()
        .rename(columns={"passer_id": "player_id"})
    )
    pass_agg["season"] = season
    rows.append(pass_agg)

    # --- Rushing stats ---
    rushers = pbp[pbp["rush_attempt"] == 1].copy()
    rush_agg = (
        rushers.groupby(["rusher_id", "week"])
        .agg(
            rush_attempts=("rush_attempt", "sum"),
            rushing_yards=("yards_gained", "sum"),
            rushing_tds=("is_td", lambda x: x[rushers.loc[x.index, "td_type"].isin(["rush"])].sum()),
        )
        .reset_index()
        .rename(columns={"rusher_id": "player_id"})
    )
    rush_agg["season"] = season
    rows.append(rush_agg)

    # --- Receiving stats ---
    receivers = pbp[pbp["pass_attempt"] == 1].copy()
    recv_agg = (
        receivers.groupby(["receiver_id", "week"])
        .agg(
            targets=("pass_attempt", "sum"),
            receptions=("complete_pass", "sum"),
            receiving_yards=("yards_gained", "sum"),
            receiving_tds=("is_td", lambda x: x[receivers.loc[x.index, "td_type"] == "pass"].sum()),
        )
        .reset_index()
        .rename(columns={"receiver_id": "player_id"})
    )
    recv_agg["season"] = season
    rows.append(recv_agg)

    # --- Fumbles ---
    # Attribute fumbles to the ball-carrier: rusher on rush plays, receiver on pass plays.
    # Grouping by td_player_id was wrong — it is null on non-TD plays (the vast majority),
    # which caused almost all fumbles to be silently dropped.
    fumble_plays = pbp[pbp["fumble"] == 1].copy()
    fumble_plays["fumbler_id"] = fumble_plays["rusher_id"].where(
        fumble_plays["rush_attempt"] == 1,
        other=fumble_plays["receiver_id"],
    )
    fumble_agg = (
        fumble_plays[fumble_plays["fumbler_id"].notna()]
        .groupby(["fumbler_id", "week"])
        .agg(fumbles_lost=("fumble", "sum"))
        .reset_index()
        .rename(columns={"fumbler_id": "player_id"})
    )
    fumble_agg["season"] = season
    rows.append(fumble_agg)

    # Merge all partial frames on player_id + week + season
    merged: pd.DataFrame | None = None
    for frame in rows:
        if frame.empty or "player_id" not in frame.columns:
            continue
        frame = frame[frame["player_id"].notna()].copy()
        if frame.empty:
            continue
        if merged is None:
            merged = frame
        else:
            merged = merged.merge(
                frame, on=["player_id", "week", "season"], how="outer"
            )

    if merged is None or merged.empty:
        return pd.DataFrame(columns=["player_id", "season", "week"] + stat_cols)

    for col in stat_cols:
        if col not in merged.columns:
            merged[col] = 0
    merged[stat_cols] = merged[stat_cols].fillna(0).astype(int)
    merged = merged[merged["player_id"].notna() & (merged["player_id"] != "")]
    return merged[["player_id", "season", "week"] + stat_cols].copy()


def _upsert_players(conn: sqlite3.Connection, rosters: pd.DataFrame) -> int:
    """Insert or replace player rows from roster data. Returns count upserted."""
    count = 0
    for _, row in rosters.iterrows():
        player_id = row.get("player_id")
        name = row.get("name", "")
        team = row.get("team", None)
        position = row.get("position", "UNK")
        # Guard: skip rows with null/empty player_id or name
        if player_id is None or (isinstance(player_id, float) and player_id != player_id):
            continue
        player_id = str(player_id).strip()
        if not player_id or not name:
            continue
        conn.execute(
            """
            INSERT INTO players (player_id, name, team, position, nflverse_id, updated_at)
            VALUES (?, ?, ?, ?, ?, datetime('now'))
            ON CONFLICT(player_id) DO UPDATE SET
                name = excluded.name,
                team = excluded.team,
                position = excluded.position,
                nflverse_id = excluded.nflverse_id,
                updated_at = excluded.updated_at
            """,
            (player_id, name, team, position, player_id),
        )
        count += 1
    conn.commit()
    return count


def _upsert_stats(conn: sqlite3.Connection, stats: pd.DataFrame) -> int:
    """Insert or replace player_stats rows. Returns count upserted."""
    count = 0
    for _, row in stats.iterrows():
        player_id = row.get("player_id")
        season = int(row.get("season", 0))
        week = int(row.get("week", 0))
        if not player_id or season == 0:
            continue

        # Only insert if the player exists in the players table
        existing = conn.execute(
            "SELECT 1 FROM players WHERE player_id = ?", (player_id,)
        ).fetchone()
        if not existing:
            continue

        vals = [int(row.get(c, 0)) for c in stat_cols]
        conn.execute(
            f"""
            INSERT INTO player_stats
                (player_id, season, week, {', '.join(stat_cols)})
            VALUES
                (?, ?, ?, {', '.join(['?'] * len(stat_cols))})
            ON CONFLICT(player_id, season, week) DO UPDATE SET
                {', '.join(f'{c} = excluded.{c}' for c in stat_cols)}
            """,
            [player_id, season, week, *vals],
        )
        count += 1
    conn.commit()
    return count


# ---------------------------------------------------------------------------
# Main pipeline
# ---------------------------------------------------------------------------

def run(
    years: list[int],
    cache_dir: str = "data/cache",
    _conn: sqlite3.Connection | None = None,
) -> None:
    """Ingest nflverse data for the given years into SQLite.

    Args:
        years: NFL seasons to ingest.
        cache_dir: Parquet cache directory.
        _conn: Optional pre-opened connection (for testing). When provided the
               connection is NOT closed by this function.
    """
    logger.info("=== data_pipeline START  years=%s ===", years)
    _owns_conn = _conn is None
    conn = _conn if _conn is not None else init_db()

    total_players = 0
    total_stats = 0

    for year in years:
        logger.info("Processing year %d …", year)

        # 1. Rosters → players table
        try:
            rosters = load_rosters([year], cache_dir=cache_dir)
            n = _upsert_players(conn, rosters)
            total_players += n
            logger.info("  year=%d  players upserted: %d", year, n)
        except Exception as exc:
            logger.error("  year=%d  roster load FAILED: %s", year, exc)
            continue

        # 2. PBP → player_stats table
        try:
            pbp = load_pbp([year], cache_dir=cache_dir)
            stats = _aggregate_stats_from_pbp(pbp, season=year)
            n = _upsert_stats(conn, stats)
            total_stats += n
            logger.info("  year=%d  stat rows upserted: %d", year, n)
        except Exception as exc:
            logger.error("  year=%d  pbp load FAILED: %s", year, exc)

    if _owns_conn:
        conn.close()
    logger.info(
        "=== data_pipeline DONE  players=%d  stat_rows=%d ===",
        total_players,
        total_stats,
    )
    write_health(status="ok", message=f"data_pipeline complete: {total_players} players, {total_stats} stat rows")


def _parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Ingest nflverse data into SQLite")
    parser.add_argument(
        "--years",
        nargs="+",
        type=int,
        default=DEFAULT_YEARS,
        help="NFL seasons to ingest (default: 2024)",
    )
    parser.add_argument(
        "--cache-dir",
        default="data/cache",
        help="Directory for parquet cache (default: data/cache)",
    )
    return parser.parse_args(argv)


if __name__ == "__main__":
    args = _parse_args()
    run(years=args.years, cache_dir=args.cache_dir)
