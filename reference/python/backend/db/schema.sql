CREATE TABLE IF NOT EXISTS players (
    player_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    team TEXT,
    position TEXT NOT NULL,
    espn_id TEXT,
    sleeper_id TEXT,
    nflverse_id TEXT,
    bye_week INTEGER,
    birth_date TEXT,
    draft_round INTEGER,
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS player_stats (
    player_id TEXT NOT NULL REFERENCES players(player_id),
    season INTEGER NOT NULL,
    week INTEGER NOT NULL,
    pass_attempts INTEGER DEFAULT 0,
    completions INTEGER DEFAULT 0,
    passing_yards INTEGER DEFAULT 0,
    passing_tds INTEGER DEFAULT 0,
    interceptions INTEGER DEFAULT 0,
    rush_attempts INTEGER DEFAULT 0,
    rushing_yards INTEGER DEFAULT 0,
    rushing_tds INTEGER DEFAULT 0,
    receptions INTEGER DEFAULT 0,
    targets INTEGER DEFAULT 0,
    receiving_yards INTEGER DEFAULT 0,
    receiving_tds INTEGER DEFAULT 0,
    fumbles_lost INTEGER DEFAULT 0,
    two_point_conversions INTEGER DEFAULT 0,
    PRIMARY KEY (player_id, season, week)
);

CREATE TABLE IF NOT EXISTS scoring_formats (
    format_id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    config_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS projections (
    player_id TEXT NOT NULL REFERENCES players(player_id),
    format_id INTEGER NOT NULL REFERENCES scoring_formats(format_id),
    source TEXT NOT NULL,
    season INTEGER NOT NULL,
    week INTEGER NOT NULL DEFAULT 0,
    projected_points REAL NOT NULL,
    projected_at TEXT DEFAULT (datetime('now')),
    PRIMARY KEY (player_id, format_id, source, season, week)
);

CREATE TABLE IF NOT EXISTS leagues (
    league_id INTEGER PRIMARY KEY AUTOINCREMENT,
    platform TEXT NOT NULL CHECK (platform IN ('espn', 'sleeper')),
    external_id TEXT NOT NULL,
    name TEXT NOT NULL,
    scoring_format_id INTEGER REFERENCES scoring_formats(format_id),
    roster_slots_json TEXT NOT NULL,
    num_teams INTEGER NOT NULL,
    draft_type TEXT DEFAULT 'snake',
    season INTEGER NOT NULL,
    UNIQUE (platform, external_id, season)
);

CREATE TABLE IF NOT EXISTS rosters (
    league_id INTEGER NOT NULL REFERENCES leagues(league_id),
    player_id TEXT NOT NULL REFERENCES players(player_id),
    team_slot TEXT NOT NULL,
    roster_position TEXT,
    updated_at TEXT DEFAULT (datetime('now')),
    PRIMARY KEY (league_id, player_id)
);

CREATE TABLE IF NOT EXISTS valuations (
    player_id TEXT NOT NULL REFERENCES players(player_id),
    format_id INTEGER NOT NULL REFERENCES scoring_formats(format_id),
    projected_points REAL NOT NULL,
    vor REAL NOT NULL,
    tier INTEGER NOT NULL,
    rank INTEGER NOT NULL,
    computed_at TEXT DEFAULT (datetime('now')),
    PRIMARY KEY (player_id, format_id)
);

CREATE TABLE IF NOT EXISTS draft_history (
    pick_id INTEGER PRIMARY KEY AUTOINCREMENT,
    league_id INTEGER NOT NULL REFERENCES leagues(league_id),
    round INTEGER NOT NULL,
    pick_number INTEGER NOT NULL,
    player_id TEXT REFERENCES players(player_id),
    team_slot TEXT NOT NULL,
    picked_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS saved_views (
    view_id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    config_json TEXT NOT NULL,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS draft_sessions (
    session_id     TEXT NOT NULL PRIMARY KEY,
    league_id      INTEGER REFERENCES leagues(league_id),
    session_type   TEXT NOT NULL CHECK (session_type IN ('mock', 'live')),
    settings_json  TEXT NOT NULL DEFAULT '{}',
    user_slot      INTEGER NOT NULL DEFAULT 1,
    status         TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'complete')),
    created_at     TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS draft_queue (
    session_id  TEXT NOT NULL REFERENCES draft_sessions(session_id) ON DELETE CASCADE,
    player_id   TEXT NOT NULL REFERENCES players(player_id),
    priority    INTEGER NOT NULL DEFAULT 0,
    added_at    TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (session_id, player_id)
);

CREATE TABLE IF NOT EXISTS matchup_grades (
    grade_id    INTEGER PRIMARY KEY AUTOINCREMENT,
    week        INTEGER NOT NULL,
    season      INTEGER NOT NULL,
    def_team    TEXT NOT NULL,
    position    TEXT NOT NULL CHECK (position IN ('QB','RB','WR','TE')),
    situation   TEXT NOT NULL DEFAULT 'overall',
    rapm_grade  REAL NOT NULL,
    computed_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS coaching_changes (
    change_id       INTEGER PRIMARY KEY AUTOINCREMENT,
    season          INTEGER NOT NULL,
    week_effective  INTEGER NOT NULL,
    team            TEXT NOT NULL,
    role            TEXT NOT NULL CHECK (role IN ('HC', 'OC', 'DC')),
    old_name        TEXT NOT NULL,
    new_name        TEXT NOT NULL,
    UNIQUE (season, week_effective, team, role)
);

CREATE TABLE IF NOT EXISTS situation_grades (
    player_id   TEXT NOT NULL REFERENCES players(player_id),
    season      INTEGER NOT NULL,
    week        INTEGER NOT NULL,
    situation   TEXT NOT NULL CHECK (situation IN ('red_zone','passing_down','rushing_down','two_minute')),
    rapm_grade  REAL NOT NULL,
    snaps       INTEGER NOT NULL DEFAULT 0,
    computed_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (player_id, season, week, situation)
);

CREATE TABLE IF NOT EXISTS kalman_trajectory (
    player_id   TEXT NOT NULL,
    season      INTEGER NOT NULL,
    week        INTEGER NOT NULL,
    talent      REAL NOT NULL,
    form        REAL NOT NULL,
    total       REAL NOT NULL,
    var_total   REAL NOT NULL,
    scheme_fit  REAL,
    computed_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (player_id, season, week)
);

CREATE TABLE IF NOT EXISTS trade_history (
    trade_id             INTEGER PRIMARY KEY AUTOINCREMENT,
    league_id            INTEGER NOT NULL REFERENCES leagues(league_id),
    week                 INTEGER NOT NULL,
    season               INTEGER NOT NULL,
    team_a_player_ids    TEXT NOT NULL,
    team_b_player_ids    TEXT NOT NULL,
    team_a_rsv_at_trade  REAL,
    team_b_rsv_at_trade  REAL,
    team_a_actual_points REAL,
    team_b_actual_points REAL,
    executed_at          TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ---------------------------------------------------------------------------
-- viz_query_cache — stores LLM-generated ChartSpec results keyed by query hash
-- (Task 14: also created at FastAPI startup via main.py hook for production DBs)
-- ---------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS viz_query_cache (
    query_hash  TEXT PRIMARY KEY,
    query_text  TEXT    NOT NULL,
    spec_json   TEXT    NOT NULL,
    hit_count   INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT    NOT NULL DEFAULT (datetime('now'))
);
