import sqlite3
import pytest
import backend.db.connection as _conn_mod
from backend.db.connection import get_db, init_db


@pytest.fixture
def db(tmp_path, monkeypatch):
    db_path = str(tmp_path / "test.sqlite")
    monkeypatch.setattr(_conn_mod, "DB_PATH", db_path)
    conn = get_db()
    init_db(conn)
    yield conn
    conn.close()


EXPECTED_TABLES = [
    "players", "player_stats", "projections", "scoring_formats",
    "leagues", "rosters", "valuations", "draft_history", "saved_views",
    "draft_sessions", "draft_queue",
    "matchup_grades", "trade_history",
]


def test_all_tables_created(db):
    cursor = db.execute(
        "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"
    )
    tables = [row["name"] for row in cursor.fetchall()]
    for t in EXPECTED_TABLES:
        assert t in tables, f"Missing table: {t}"


def test_players_columns(db):
    cursor = db.execute("PRAGMA table_info(players)")
    cols = {row["name"] for row in cursor.fetchall()}
    assert cols >= {"player_id", "name", "team", "position", "espn_id", "sleeper_id", "nflverse_id", "bye_week", "birth_date", "draft_round"}


def test_scoring_formats_columns(db):
    cursor = db.execute("PRAGMA table_info(scoring_formats)")
    cols = {row["name"] for row in cursor.fetchall()}
    assert cols >= {"format_id", "name", "config_json"}


def test_leagues_columns(db):
    cursor = db.execute("PRAGMA table_info(leagues)")
    cols = {row["name"] for row in cursor.fetchall()}
    assert cols >= {"league_id", "platform", "external_id", "name", "scoring_format_id", "roster_slots_json", "num_teams"}


def test_valuations_columns(db):
    cursor = db.execute("PRAGMA table_info(valuations)")
    cols = {row["name"] for row in cursor.fetchall()}
    assert cols >= {"player_id", "format_id", "projected_points", "vor", "tier", "computed_at"}


def test_matchup_grades_columns(db):
    cursor = db.execute("PRAGMA table_info(matchup_grades)")
    cols = {row["name"] for row in cursor.fetchall()}
    assert cols >= {"grade_id", "week", "season", "def_team", "position", "rapm_grade"}


def test_trade_history_columns(db):
    cursor = db.execute("PRAGMA table_info(trade_history)")
    cols = {row["name"] for row in cursor.fetchall()}
    assert cols >= {"trade_id", "league_id", "week", "season",
                    "team_a_player_ids", "team_b_player_ids"}


def test_foreign_key_enforcement(db):
    with pytest.raises(sqlite3.IntegrityError):
        db.execute(
            "INSERT INTO rosters (league_id, player_id, team_slot) VALUES (999, 'fake', 'QB')"
        )


# ---------------------------------------------------------------------------
# Task 3 — situation dimension tests
# ---------------------------------------------------------------------------

def test_matchup_grades_has_situation_column(db):
    """matchup_grades must have a 'situation' column after init_db."""
    cursor = db.execute("PRAGMA table_info(matchup_grades)")
    cols = {row["name"] for row in cursor.fetchall()}
    assert "situation" in cols, "matchup_grades missing 'situation' column"


def test_init_db_idempotent_on_existing_db_adds_situation(tmp_path, monkeypatch):
    """init_db on an OLD-shape DB (no situation col) must upgrade it and allow
    an ON CONFLICT(...,situation) upsert to succeed."""
    import backend.db.connection as _conn_mod_local
    db_path = str(tmp_path / "old_shape.sqlite")
    monkeypatch.setattr(_conn_mod, "DB_PATH", db_path)
    conn = _conn_mod.get_db()

    # Build the OLD table shape manually — no situation column, old unique constraint
    conn.execute("""
        CREATE TABLE IF NOT EXISTS matchup_grades (
            grade_id    INTEGER PRIMARY KEY AUTOINCREMENT,
            week        INTEGER NOT NULL,
            season      INTEGER NOT NULL,
            def_team    TEXT NOT NULL,
            position    TEXT NOT NULL CHECK (position IN ('QB','RB','WR','TE')),
            rapm_grade  REAL NOT NULL,
            computed_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(week, season, def_team, position)
        )
    """)
    conn.commit()

    # Verify old shape: no situation column
    cols_before = {row["name"] for row in conn.execute("PRAGMA table_info(matchup_grades)").fetchall()}
    assert "situation" not in cols_before, "Pre-condition: old shape should lack 'situation'"

    # Now run init_db — must upgrade the existing DB
    init_db(conn)

    # Post-condition: situation column exists
    cols_after = {row["name"] for row in conn.execute("PRAGMA table_info(matchup_grades)").fetchall()}
    assert "situation" in cols_after, "init_db should have added 'situation' column"

    # Post-condition: an ON CONFLICT(...,situation) upsert must NOT raise
    conn.execute("""
        INSERT INTO matchup_grades (week, season, def_team, position, situation, rapm_grade)
        VALUES (1, 2025, 'NE', 'QB', 'overall', 3.5)
        ON CONFLICT(week, season, def_team, position, situation) DO UPDATE SET
            rapm_grade = excluded.rapm_grade,
            computed_at = datetime('now')
    """)
    conn.commit()
    conn.close()


def test_upsert_overall_and_situation_coexist(tmp_path, monkeypatch):
    """'overall' and 'red_zone' for the same week/team/pos must both persist as
    separate rows (no unique-conflict between them)."""
    db_path = str(tmp_path / "coexist.sqlite")
    monkeypatch.setattr(_conn_mod, "DB_PATH", db_path)
    conn = _conn_mod.get_db()
    init_db(conn)

    conn.execute("""
        INSERT INTO matchup_grades (week, season, def_team, position, situation, rapm_grade)
        VALUES (3, 2025, 'KC', 'WR', 'overall', 1.2)
        ON CONFLICT(week, season, def_team, position, situation) DO UPDATE SET
            rapm_grade = excluded.rapm_grade
    """)
    conn.execute("""
        INSERT INTO matchup_grades (week, season, def_team, position, situation, rapm_grade)
        VALUES (3, 2025, 'KC', 'WR', 'red_zone', 2.5)
        ON CONFLICT(week, season, def_team, position, situation) DO UPDATE SET
            rapm_grade = excluded.rapm_grade
    """)
    conn.commit()

    rows = conn.execute(
        "SELECT situation, rapm_grade FROM matchup_grades "
        "WHERE week=3 AND season=2025 AND def_team='KC' AND position='WR' "
        "ORDER BY situation"
    ).fetchall()
    assert len(rows) == 2, f"Expected 2 rows (overall + red_zone), got {len(rows)}"
    situations = {r["situation"] for r in rows}
    assert situations == {"overall", "red_zone"}
    conn.close()


# ---------------------------------------------------------------------------
# Task 3 Fix — ghost 4-column UNIQUE on upgraded-from-old DBs
# ---------------------------------------------------------------------------

def test_upgraded_old_db_allows_multi_situation_rows(tmp_path, monkeypatch):
    """An old-shape DB with inline UNIQUE(week,season,def_team,position) that is
    upgraded via init_db must allow two rows that share (week,season,def_team,position)
    but differ in situation (e.g. 'overall' vs 'red_zone').

    Without the rebuild fix, the ghost 4-column UNIQUE survives the ALTER TABLE
    upgrade and the second INSERT raises sqlite3.IntegrityError.
    """
    db_path = str(tmp_path / "legacy_unique.sqlite")
    monkeypatch.setattr(_conn_mod, "DB_PATH", db_path)
    conn = _conn_mod.get_db()

    # --- Step 1: Create the OLD-shape table with the inline 4-column UNIQUE
    conn.execute("""
        CREATE TABLE matchup_grades (
            grade_id    INTEGER PRIMARY KEY AUTOINCREMENT,
            week        INTEGER NOT NULL,
            season      INTEGER NOT NULL,
            def_team    TEXT NOT NULL,
            position    TEXT NOT NULL CHECK (position IN ('QB','RB','WR','TE')),
            rapm_grade  REAL NOT NULL,
            computed_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(week, season, def_team, position)
        )
    """)
    # Insert a legacy row (no situation column yet)
    conn.execute(
        "INSERT INTO matchup_grades (week, season, def_team, position, rapm_grade) "
        "VALUES (5, 2024, 'DAL', 'RB', 2.1)"
    )
    conn.commit()

    # --- Step 2: Run init_db — must upgrade AND drop the ghost UNIQUE
    init_db(conn)

    # --- Step 3: Legacy row already has situation='overall' from the ALTER DEFAULT.
    #             Now insert a SECOND row for the SAME (week,season,def_team,position)
    #             but a DIFFERENT situation ('red_zone').
    #             The ghost 4-col UNIQUE would block this even though the 5-col key differs.
    conn.execute(
        "INSERT INTO matchup_grades (week, season, def_team, position, situation, rapm_grade) "
        "VALUES (5, 2024, 'DAL', 'RB', 'red_zone', 1.5)"
    )
    conn.commit()

    rows = conn.execute(
        "SELECT situation, rapm_grade FROM matchup_grades "
        "WHERE week=5 AND season=2024 AND def_team='DAL' AND position='RB' "
        "ORDER BY situation"
    ).fetchall()
    assert len(rows) == 2, (
        f"Expected 2 rows (overall + red_zone) after upgrade, got {len(rows)}: "
        f"{[dict(r) for r in rows]}"
    )
    situations = {r["situation"] for r in rows}
    assert "overall" in situations and "red_zone" in situations, (
        f"Both 'overall' and 'red_zone' must coexist; got {situations}"
    )

    # --- Step 4: Pre-existing legacy row must have survived (situation='overall' from DEFAULT)
    overall_row = conn.execute(
        "SELECT rapm_grade FROM matchup_grades "
        "WHERE week=5 AND season=2024 AND def_team='DAL' AND position='RB' AND situation='overall'"
    ).fetchone()
    assert overall_row is not None, "Legacy row (situation='overall') did not survive the table rebuild"
    assert overall_row["rapm_grade"] == 2.1, (
        f"Legacy row rapm_grade changed; expected 2.1 got {overall_row['rapm_grade']}"
    )

    # --- Step 5: Idempotency — second init_db must not error and rows must stay intact
    init_db(conn)
    rows_after = conn.execute(
        "SELECT situation FROM matchup_grades "
        "WHERE week=5 AND season=2024 AND def_team='DAL' AND position='RB' "
        "ORDER BY situation"
    ).fetchall()
    situations_after = {r["situation"] for r in rows_after}
    assert "overall" in situations_after and "red_zone" in situations_after, (
        f"Rows lost after second init_db call; situations={situations_after}"
    )
    conn.close()
