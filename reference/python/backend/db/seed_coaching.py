"""Seed helper for the coaching_changes table.

Usage (explicit call — NOT wired into init_db so tests run without it):
    from backend.db.seed_coaching import seed_coaching_changes
    seed_coaching_changes()                      # loads default 2025 JSON
    seed_coaching_changes(conn=my_conn)          # reuse existing connection
    seed_coaching_changes(conn=my_conn, rows=[…]) # supply rows directly (for tests)
"""
from __future__ import annotations
import json
import sqlite3
from pathlib import Path
from typing import Any

_DEFAULT_JSON = Path(__file__).parent / "data" / "coaching_changes_2025.json"


def seed_coaching_changes(
    conn: sqlite3.Connection | None = None,
    rows: list[dict[str, Any]] | None = None,
    json_path: str | Path | None = None,
) -> int:
    """Idempotently upsert rows into coaching_changes.

    Parameters
    ----------
    conn :      existing SQLite connection to reuse; if None, opens its own.
    rows :      explicit list of row dicts (bypasses JSON file — for tests).
    json_path : path to JSON seed file; defaults to coaching_changes_2025.json.

    Returns
    -------
    Number of rows inserted (rows skipped by ON CONFLICT are not counted).
    """
    _owns_conn = conn is None
    if _owns_conn:
        from backend.db.connection import init_db
        conn = init_db()

    if rows is None:
        path = Path(json_path) if json_path is not None else _DEFAULT_JSON
        with open(path, encoding="utf-8") as f:
            rows = json.load(f)

    inserted = 0
    for row in rows:
        cursor = conn.execute(
            """
            INSERT OR IGNORE INTO coaching_changes
                (season, week_effective, team, role, old_name, new_name)
            VALUES
                (:season, :week_effective, :team, :role, :old_name, :new_name)
            """,
            row,
        )
        inserted += cursor.rowcount

    conn.commit()

    if _owns_conn:
        conn.close()

    return inserted
