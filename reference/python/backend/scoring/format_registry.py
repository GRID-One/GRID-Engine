from __future__ import annotations
import sqlite3
from backend.scoring.engine import ScoringConfig
from backend.scoring.formats import STANDARD, HALF_PPR, FULL_PPR

DEFAULT_ROSTER_SLOTS: dict[str, int] = {
    "QB": 1,
    "RB": 2,
    "WR": 2,
    "TE": 1,
    "K": 1,
    "DEF": 1,
}


def ensure_preset_formats(conn: sqlite3.Connection) -> dict[str, int]:
    """Upsert STANDARD/HALF_PPR/FULL_PPR presets. Returns {name: format_id}."""
    result: dict[str, int] = {}
    for fmt in (STANDARD, HALF_PPR, FULL_PPR):
        conn.execute(
            """
            INSERT INTO scoring_formats (name, config_json)
            VALUES (?, ?)
            ON CONFLICT(name) DO UPDATE SET config_json = excluded.config_json
            """,
            (fmt.name, fmt.to_json()),
        )
        conn.commit()
        row = conn.execute(
            "SELECT format_id FROM scoring_formats WHERE name = ?", (fmt.name,)
        ).fetchone()
        result[fmt.name] = row["format_id"]
    return result


def resolve_or_create_format(conn: sqlite3.Connection, name: str, config: ScoringConfig) -> int:
    """Match config by content; insert if new. Returns format_id.

    The ``name`` is only a display label — formats are identified by their
    *content* (``config_json``), so two different configs are always two
    different rows. Because ``scoring_formats.name`` is UNIQUE, a new config
    whose desired name is already taken by a DIFFERENT config is stored under a
    disambiguated label (``"Custom (2)"``) and its own format_id is returned.

    The previous ``INSERT OR IGNORE`` + ``SELECT ... WHERE name = ?`` silently
    no-op'd on such a name collision and then returned the EXISTING row's id —
    handing back the wrong config. (audit finding A3)

    Transaction contract: this helper owns its transaction — it ``commit``s on
    success and, on a rare cross-connection write-race, ``rollback``s before
    retrying. Call it with no uncommitted DML pending on ``conn`` (the existing
    callers do; ``ensure_preset_formats``/``_upsert_league`` self-commit too).
    """
    config_json = config.to_json()
    for _attempt in range(5):
        rows = conn.execute(
            "SELECT format_id, name, config_json FROM scoring_formats"
        ).fetchall()
        for row in rows:
            if row["config_json"] == config_json:
                return row["format_id"]    # content match — dedup regardless of name
        existing_names = {row["name"] for row in rows}
        unique_name = name
        n = 2
        while unique_name in existing_names:
            unique_name = f"{name} ({n})"
            n += 1
        try:
            cur = conn.execute(
                "INSERT INTO scoring_formats (name, config_json) VALUES (?, ?)",
                (unique_name, config_json),
            )
            conn.commit()
            return int(cur.lastrowid)
        except sqlite3.IntegrityError:
            # A concurrent writer claimed ``unique_name`` (or inserted this very
            # config) between our SELECT and INSERT. Roll back and retry: the
            # next iteration's content-match loop returns their row if it was our
            # config, otherwise we pick a fresh disambiguated name.
            conn.rollback()
    raise RuntimeError(
        f"could not resolve or create scoring format {name!r} after 5 attempts"
    )
