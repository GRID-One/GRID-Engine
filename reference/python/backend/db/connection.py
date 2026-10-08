import sqlite3
import os
from pathlib import Path
from dotenv import load_dotenv

load_dotenv()

DB_PATH = os.getenv("DB_PATH", "data/db/fantasy.sqlite")


def get_db() -> sqlite3.Connection:
    path = Path(DB_PATH)
    path.parent.mkdir(parents=True, exist_ok=True)
    conn = sqlite3.connect(str(path), check_same_thread=False)
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("PRAGMA foreign_keys=ON")
    return conn


def init_db(conn: sqlite3.Connection | None = None) -> sqlite3.Connection:
    if conn is None:
        conn = get_db()
    schema_path = Path(__file__).parent / "schema.sql"
    conn.executescript(schema_path.read_text())
    conn.commit()
    # ---------------------------------------------------------------------------
    # Idempotent upgrade guards for matchup_grades (Task 3 migration)
    # ---------------------------------------------------------------------------
    # Guard 1: Add 'situation' column to old-shape DBs that lack it.
    try:
        conn.execute(
            "ALTER TABLE matchup_grades ADD COLUMN situation TEXT NOT NULL DEFAULT 'overall'"
        )
        conn.commit()
    except sqlite3.OperationalError:
        pass  # column already exists — no-op

    # Guard 2: Rebuild legacy tables that carry an inline 4-column UNIQUE
    # (UNIQUE(week, season, def_team, position)).  SQLite cannot DROP a table
    # constraint, so the only safe path is the standard rename→create→copy→drop
    # reconstruction idiom.  Detection: if the stored CREATE TABLE text contains
    # 'UNIQUE(' the ghost constraint is still present and we must rebuild.
    # Fresh DBs and already-rebuilt DBs will NOT contain 'UNIQUE(' in their DDL
    # (schema.sql no longer has it) so this block is fully idempotent.
    row = conn.execute(
        "SELECT sql FROM sqlite_master WHERE type='table' AND name='matchup_grades'"
    ).fetchone()
    if row and "UNIQUE(" in (row[0] or ""):
        # Canonical column list for the current schema (matches schema.sql exactly,
        # minus the inline UNIQUE).
        canonical_ddl = (
            "CREATE TABLE matchup_grades (\n"
            "    grade_id    INTEGER PRIMARY KEY AUTOINCREMENT,\n"
            "    week        INTEGER NOT NULL,\n"
            "    season      INTEGER NOT NULL,\n"
            "    def_team    TEXT NOT NULL,\n"
            "    position    TEXT NOT NULL CHECK (position IN ('QB','RB','WR','TE')),\n"
            "    situation   TEXT NOT NULL DEFAULT 'overall',\n"
            "    rapm_grade  REAL NOT NULL,\n"
            "    computed_at TEXT NOT NULL DEFAULT (datetime('now'))\n"
            ")"
        )
        # Explicit column list for the INSERT…SELECT — situation already exists on
        # the renamed table because Guard 1 ran first.
        cols = "grade_id, week, season, def_team, position, situation, rapm_grade, computed_at"
        with conn:  # implicit BEGIN / COMMIT (or ROLLBACK on exception)
            conn.execute("ALTER TABLE matchup_grades RENAME TO _matchup_grades_old")
            conn.execute(canonical_ddl)
            conn.execute(
                f"INSERT INTO matchup_grades ({cols}) "
                f"SELECT {cols} FROM _matchup_grades_old"
            )
            conn.execute("DROP TABLE _matchup_grades_old")

    # Guard 3: Ensure the 5-column unique index exists (safe on both new and
    # just-rebuilt tables; not placed in schema.sql to avoid ordering conflicts
    # when executescript runs before the ALTER on old-shape DBs).
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS ux_matchup_grades "
        "ON matchup_grades(week, season, def_team, position, situation)"
    )
    conn.commit()
    return conn
