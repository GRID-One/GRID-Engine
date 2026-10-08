"""Compute VOR and tier valuations for all scoring formats and store in SQLite.

Run:
    python -m backend.pipeline.compute_valuations [--season 2024]

Idempotent: uses INSERT OR REPLACE on the valuations table.
Logs to data/logs/pipeline.log.
"""
from __future__ import annotations

import argparse
import sqlite3
import sys
from typing import Any

from backend.db.connection import get_db, init_db
from backend.scoring.engine import ScoringConfig, calculate_points
from backend.scoring.formats import STANDARD, HALF_PPR, FULL_PPR
from backend.scoring.vor import assign_tiers, calculate_vor
from backend.scoring.format_registry import ensure_preset_formats, DEFAULT_ROSTER_SLOTS
from backend.pipeline._logging import get_logger
from backend.pipeline.health_check import write_health
from backend.validation.sniff import check_rank_bands

logger = get_logger("compute_valuations")

DEFAULT_NUM_TEAMS = 12

ALL_FORMATS: list[ScoringConfig] = [STANDARD, HALF_PPR, FULL_PPR]


def _load_players_with_stats(
    conn: sqlite3.Connection,
    season: int,
) -> list[dict[str, Any]]:
    """Load players joined with their season-aggregate stats from SQLite.

    Returns a list of dicts with keys:
      player_id, name, position, team, and all stat fields.
    """
    rows = conn.execute(
        """
        SELECT
            p.player_id,
            p.name,
            p.position,
            p.team,
            COALESCE(SUM(ps.pass_attempts), 0)       AS pass_attempts,
            COALESCE(SUM(ps.completions), 0)         AS completions,
            COALESCE(SUM(ps.passing_yards), 0)       AS passing_yards,
            COALESCE(SUM(ps.passing_tds), 0)         AS passing_tds,
            COALESCE(SUM(ps.interceptions), 0)       AS interceptions,
            COALESCE(SUM(ps.rush_attempts), 0)       AS rush_attempts,
            COALESCE(SUM(ps.rushing_yards), 0)       AS rushing_yards,
            COALESCE(SUM(ps.rushing_tds), 0)         AS rushing_tds,
            COALESCE(SUM(ps.targets), 0)             AS targets,
            COALESCE(SUM(ps.receptions), 0)          AS receptions,
            COALESCE(SUM(ps.receiving_yards), 0)     AS receiving_yards,
            COALESCE(SUM(ps.receiving_tds), 0)       AS receiving_tds,
            COALESCE(SUM(ps.fumbles_lost), 0)        AS fumbles_lost,
            COALESCE(SUM(ps.two_point_conversions), 0) AS two_point_conversions
        FROM players p
        LEFT JOIN player_stats ps
            ON p.player_id = ps.player_id AND ps.season = ?
        WHERE p.position IN ('QB', 'RB', 'WR', 'TE', 'K')
        GROUP BY p.player_id
        """,
        (season,),
    ).fetchall()
    return [dict(r) for r in rows]


def _upsert_valuations(
    conn: sqlite3.Connection,
    valuations: list[dict],
    format_id: int,
) -> int:
    """Upsert valuation rows. Returns count written."""
    count = 0
    for v in valuations:
        conn.execute(
            """
            INSERT INTO valuations
                (player_id, format_id, projected_points, vor, tier, rank)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(player_id, format_id) DO UPDATE SET
                projected_points = excluded.projected_points,
                vor = excluded.vor,
                tier = excluded.tier,
                rank = excluded.rank,
                computed_at = datetime('now')
            """,
            (
                v["player_id"],
                format_id,
                v["projected_points"],
                v["vor"],
                v["tier"],
                v["rank"],
            ),
        )
        count += 1
    conn.commit()
    return count


# ---------------------------------------------------------------------------
# Main pipeline
# ---------------------------------------------------------------------------

def run(
    season: int,
    roster_slots: dict[str, int] | None = None,
    num_teams: int = DEFAULT_NUM_TEAMS,
    projection: dict | None = None,
    elite_ids=None,
    _conn: sqlite3.Connection | None = None,
) -> None:
    """Compute VOR/tier valuations for all scoring formats.

    Args:
        season: NFL season to compute valuations for.
        roster_slots: Position slot counts for replacement-level calc.
        num_teams: Number of teams (default 12).
        projection: Optional ``{player_id: stat_line}`` from the GRID projection
            (``projection.project_preseason``).  When supplied, a player present
            in it is scored from the *projected* stat line instead of the
            last-season aggregate; players absent from it fall back to
            last-season (so the pipeline still runs before the engine has
            produced projections for everyone).  ``None`` (default) preserves the
            pre-GRID baseline behavior exactly.
        elite_ids: Optional consensus-elite ``player_id`` set.  When supplied, the
            known-player rank-band **sniff gate** (`validation.sniff`) runs on
            each format's ranking and logs a warning if the elites don't occupy a
            sane band — a soft gate on gross projection breakage, not a hard fail.
        _conn: Optional pre-opened connection (for testing). When provided the
               connection is NOT closed by this function.
    """
    logger.info("=== compute_valuations START  season=%d ===", season)
    _owns_conn = _conn is None
    conn = _conn if _conn is not None else init_db()

    if roster_slots is None:
        roster_slots = DEFAULT_ROSTER_SLOTS

    # 1. Ensure scoring format rows exist, get their IDs
    format_ids = ensure_preset_formats(conn)
    logger.info("Scoring formats: %s", list(format_ids.keys()))

    # 2. Load player stats from DB
    players = _load_players_with_stats(conn, season)
    logger.info("Loaded %d players from DB (season=%d)", len(players), season)

    if not players:
        logger.warning(
            "No players found for season=%d — run data_pipeline first.", season
        )
        if _owns_conn:
            conn.close()
        return

    # 3. Compute valuations for each scoring format
    for fmt in ALL_FORMATS:
        format_id = format_ids[fmt.name]
        logger.info("Computing VOR for format '%s' (format_id=%d) …", fmt.name, format_id)

        # Add projected_points field — required by calculate_vor.  Source the
        # stat line from the GRID projection when available for this player,
        # else fall back to the last-season aggregate (the pre-GRID baseline).
        enriched: list[dict] = []
        for p in players:
            if projection is not None and p["player_id"] in projection:
                stats = projection[p["player_id"]]
            else:
                stats = {
                    k: v
                    for k, v in p.items()
                    if k
                    not in ("player_id", "name", "position", "team", "projected_points")
                }
            projected = calculate_points(stats, fmt)
            enriched.append({**p, "projected_points": projected})

        # calculate_vor uses projected_points directly — no config param
        vor_players = calculate_vor(enriched, roster_slots, num_teams)

        # assign_tiers expects list[dict] with 'vor' field; returns sorted list with 'tier'
        tiered = assign_tiers(vor_players)

        # Optional known-player sniff gate: warn (soft gate) if consensus elites
        # don't land in a sane rank band — catches gross projection breakage.
        if elite_ids:
            res = check_rank_bands(tiered, elite_ids)
            if not res.passed:
                logger.warning("sniff gate FAILED for format '%s': %s",
                               fmt.name, res.failures)

        n = _upsert_valuations(conn, tiered, format_id)
        logger.info("  format='%s'  rows upserted: %d", fmt.name, n)

        # Also upsert into projections table for API access
        source = "computed"
        for v in tiered:
            conn.execute(
                """
                INSERT INTO projections
                    (player_id, format_id, source, season, week, projected_points)
                VALUES (?, ?, ?, ?, 0, ?)
                ON CONFLICT(player_id, format_id, source, season, week) DO UPDATE SET
                    projected_points = excluded.projected_points,
                    projected_at = datetime('now')
                """,
                (v["player_id"], format_id, source, season, v["projected_points"]),
            )
        conn.commit()
        logger.info("  format='%s'  projections rows upserted: %d", fmt.name, n)

    if _owns_conn:
        conn.close()
    logger.info("=== compute_valuations DONE ===")
    write_health(status="ok", message="compute_valuations complete")


def _parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Compute VOR/tier valuations for all scoring formats"
    )
    parser.add_argument(
        "--season",
        type=int,
        default=2024,
        help="NFL season to compute valuations for (default: 2024)",
    )
    parser.add_argument(
        "--num-teams",
        type=int,
        default=DEFAULT_NUM_TEAMS,
        help="Number of teams for replacement-level calculation (default: 12)",
    )
    return parser.parse_args(argv)


if __name__ == "__main__":
    args = _parse_args()
    run(season=args.season, num_teams=args.num_teams)
