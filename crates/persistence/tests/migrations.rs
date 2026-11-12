//! Migration policy tests (ADR-015 D1–D3, D5–D7, D9; engine-spec §8.19; owner rulings R1, R3).

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use common::{
    dump_all, exec, migrated, migrations_dir, read_seed, row_count, seeded, seeds_dir, table_names,
    TempDir,
};
use grid_domain::{EngineErrorCode, TerminalState};
use grid_persistence::error::{PersistenceError, SchemaProblem};
use grid_persistence::schema::{self, SchemaStatus, MIGRATOR};
use grid_persistence::{applied_schema_version, schema_meta_value, Database, DbMode};

/// The sixteen tables of `0002_backbone.sql`: the thirteen of owner ruling R1 and the three child
/// tables of work package P1-02 D3.
const BACKBONE_TABLES: [&str; 16] = [
    "artifact_manifest",
    "data_version_inputs",
    "data_versions",
    "diagnostic_events",
    "engine_settings",
    "ingestion_runs",
    "job_dependencies",
    "jobs",
    "model_versions",
    "prediction_runs",
    "production_pointers",
    "quarantined_rows",
    "raw_data",
    "snapshot_pointers",
    "snapshots",
    "training_runs",
];

/// Migrations the owner approved to contain a destructive statement (ADR-015 D5). Empty: only an
/// owner-approved change may add a file name here.
const APPROVED_DESTRUCTIVE_MIGRATIONS: [&str; 0] = [];

#[test]
fn embedded_migrations_are_exactly_0001_and_0002() {
    // The reviewed literal: a new migration must update this list deliberately.
    assert_eq!(schema::embedded_versions(), vec![1, 2]);
    let descriptions: Vec<String> = MIGRATOR
        .iter()
        .map(|migration| migration.description.to_string())
        .collect();
    assert_eq!(descriptions, vec!["schema meta", "backbone"]);
}

#[sqlx::test]
async fn blank_database_reaches_head() {
    let dir = TempDir::new("blank");
    let db = Database::open(&dir.db_path(), DbMode::Create)
        .await
        .expect("a new database opens");
    assert_eq!(
        schema::status(&db).await.expect("status reads"),
        SchemaStatus::NotInitialized
    );

    let applied = schema::apply(&db).await.expect("migrations apply");
    let embedded = schema::embedded_versions();
    assert_eq!(applied, embedded, "every embedded migration is applied");
    let head = *embedded.last().expect("migrations are embedded");
    assert_eq!(
        schema::status(&db).await.expect("status reads"),
        SchemaStatus::Current { head }
    );
    assert_eq!(
        applied_schema_version(db.pool()).await.expect("readable"),
        Some(head)
    );

    // Exactly the backbone, schema_meta (kept, ADR-015 D6) and SQLite's and sqlx's own tables.
    let mut expected: BTreeSet<String> = BACKBONE_TABLES.iter().map(|t| t.to_string()).collect();
    expected.extend(["_sqlx_migrations", "schema_meta", "sqlite_sequence"].map(String::from));
    assert_eq!(table_names(&db).await, expected);

    let violations: Vec<(String,)> =
        sqlx::query_as("SELECT \"table\" FROM pragma_foreign_key_check")
            .fetch_all(db.pool())
            .await
            .expect("foreign_key_check runs");
    assert!(violations.is_empty(), "{violations:?}");
    let integrity: Vec<String> = sqlx::query_scalar("PRAGMA integrity_check")
        .fetch_all(db.pool())
        .await
        .expect("integrity_check runs");
    assert_eq!(integrity, vec!["ok".to_owned()]);

    // schema_meta is kept and usable.
    sqlx::query("INSERT INTO schema_meta (key, value) VALUES ('kept', 'yes')")
        .execute(db.pool())
        .await
        .expect("schema_meta accepts a row");
    assert_eq!(
        schema_meta_value(db.pool(), "kept")
            .await
            .expect("readable")
            .as_deref(),
        Some("yes")
    );

    // The seeded engine settings are the P1-02 D8 defaults.
    let settings: Vec<(String, i64, String)> =
        sqlx::query_as("SELECT key, version, value FROM engine_settings ORDER BY key")
            .fetch_all(db.pool())
            .await
            .expect("engine_settings is readable");
    let expected_settings = [
        ("engine.shutdown_grace_secs", "30"),
        ("jobs.backoff_initial_secs", "5"),
        ("jobs.backoff_max_secs", "60"),
        ("jobs.heartbeat_secs", "30"),
        ("jobs.lease_secs", "300"),
        ("jobs.max_attempts", "3"),
    ]
    .map(|(key, value)| (key.to_owned(), 1, value.to_owned()));
    assert_eq!(settings, expected_settings.to_vec());

    // Idempotent: a current store applies nothing.
    assert_eq!(
        schema::apply(&db).await.expect("re-apply"),
        Vec::<i64>::new()
    );
    db.close().await;
}

#[sqlx::test]
async fn upgrade_from_every_prior_version_preserves_seeded_rows() {
    let embedded = schema::embedded_versions();
    let head = *embedded.last().expect("migrations are embedded");
    let prior: Vec<(usize, i64)> = embedded
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, version)| *version != head)
        .collect();
    assert!(!prior.is_empty(), "there is at least one prior version");

    for (index, version) in prior {
        let dir = TempDir::new(&format!("upgrade-from-{version}"));
        let db = Database::open(&dir.db_path(), DbMode::Create)
            .await
            .expect("a new database opens");
        MIGRATOR
            .run_to(version, db.pool())
            .await
            .expect("migrating to the prior version succeeds");
        assert_eq!(
            applied_schema_version(db.pool()).await.expect("readable"),
            Some(version),
            "run_to includes its target"
        );
        for seed_version in &embedded[..=index] {
            exec(&db, &read_seed(*seed_version)).await;
        }
        let before = dump_all(&db).await;
        let seeded_rows: usize = before.values().map(Vec::len).sum();
        assert!(seeded_rows > 0, "the seeds wrote rows at version {version}");

        let pending = embedded[index + 1..].to_vec();
        assert_eq!(
            schema::status(&db).await.expect("status reads"),
            SchemaStatus::Pending {
                applied: Some(version),
                pending: pending.clone()
            }
        );
        assert_eq!(schema::apply(&db).await.expect("upgrade applies"), pending);
        assert_eq!(
            schema::status(&db).await.expect("status reads"),
            SchemaStatus::Current { head }
        );

        let after = dump_all(&db).await;
        for (table, rows) in &before {
            let kept = after
                .get(table)
                .unwrap_or_else(|| panic!("table {table} survives the upgrade from {version}"));
            for row in rows {
                let before_count = rows.iter().filter(|r| *r == row).count();
                let after_count = kept.iter().filter(|r| *r == row).count();
                assert!(
                    after_count >= before_count,
                    "row {row} of {table} survives the upgrade from {version}"
                );
            }
        }

        // The later seeds still apply on top of the upgraded store.
        for seed_version in &embedded[index + 1..] {
            exec(&db, &read_seed(*seed_version)).await;
        }
        schema::check_integrity(&db)
            .await
            .expect("the upgraded, seeded store is consistent");
        db.close().await;
    }
}

#[sqlx::test]
async fn every_backbone_table_has_a_seeded_row() {
    let dir = TempDir::new("head-seed");
    let db = seeded(&dir).await;
    for table in BACKBONE_TABLES {
        assert!(
            row_count(&db, table).await >= 1,
            "seed_0002.sql writes at least one row into {table}"
        );
    }
    schema::check_integrity(&db)
        .await
        .expect("the seeded store passes foreign_key_check and quick_check");
    db.close().await;
}

/// The version prefix of a `NNNN_name.sql` or `seed_NNNN.sql` file name.
fn version_of(name: &str, prefix: &str) -> Option<i64> {
    let rest = name.strip_prefix(prefix)?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.len() == 4 {
        digits.parse().ok()
    } else {
        None
    }
}

/// The `.sql` file names of a directory, sorted.
fn sql_files(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("the directory is readable")
        .map(|entry| {
            entry
                .expect("the entry is readable")
                .file_name()
                .into_string()
                .expect("the file name is UTF-8")
        })
        .filter(|name| name.ends_with(".sql"))
        .collect();
    names.sort();
    names
}

#[test]
fn every_migration_has_an_upgrade_seed() {
    let migrations: Vec<i64> = sql_files(&migrations_dir())
        .iter()
        .map(|name| version_of(name, "").unwrap_or_else(|| panic!("{name} starts with NNNN_")))
        .collect();
    let seeds: Vec<i64> = sql_files(&seeds_dir())
        .iter()
        .map(|name| {
            version_of(name, "seed_").unwrap_or_else(|| panic!("{name} is named seed_NNNN.sql"))
        })
        .collect();
    assert_eq!(migrations, schema::embedded_versions());
    assert_eq!(
        seeds, migrations,
        "every migration ships seed_<NNNN>.sql and no seed lacks a migration (ADR-015 D9)"
    );
}

#[test]
fn migration_files_are_lf_only() {
    let names = sql_files(&migrations_dir());
    assert!(!names.is_empty());
    for name in names {
        let bytes = std::fs::read(migrations_dir().join(&name)).expect("readable");
        assert!(
            !bytes.contains(&b'\r'),
            "{name} contains a carriage return; migrations are LF-only (ADR-015 D7)"
        );
    }
}

/// One token of a migration, with the line it starts on.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Token {
    text: String,
    line: usize,
}

/// Splits SQL into upper-cased words and single punctuation characters, dropping `--` and
/// `/* */` comments, string literals and quoted identifiers, so text inside them can never count
/// as a statement.
fn tokenize(sql: &str) -> Vec<Token> {
    let chars: Vec<char> = sql.chars().collect();
    let mut tokens = Vec::new();
    let mut line = 1;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '\n' {
            line += 1;
            i += 1;
        } else if c.is_whitespace() {
            i += 1;
        } else if c == '-' && next == Some('-') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            i += 2;
            while i < chars.len() && !(chars[i] == '*' && chars.get(i + 1) == Some(&'/')) {
                if chars[i] == '\n' {
                    line += 1;
                }
                i += 1;
            }
            i += 2;
        } else if matches!(c, '\'' | '"' | '`' | '[') {
            let close = if c == '[' { ']' } else { c };
            i += 1;
            while i < chars.len() {
                if chars[i] == '\n' {
                    line += 1;
                }
                if chars[i] == close {
                    // A doubled quote is an escaped quote inside the literal.
                    if close != ']' && chars.get(i + 1) == Some(&close) {
                        i += 2;
                        continue;
                    }
                    break;
                }
                i += 1;
            }
            i += 1;
        } else if c.is_alphanumeric() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || matches!(chars[i], '_' | '$')) {
                i += 1;
            }
            tokens.push(Token {
                text: chars[start..i].iter().collect::<String>().to_uppercase(),
                line,
            });
        } else {
            tokens.push(Token {
                text: c.to_string(),
                line,
            });
            i += 1;
        }
    }
    tokens
}

/// The destructive statements of ADR-015 D5 found in `sql`, as `(line, description)`.
///
/// It flags `DROP` of any object, `DELETE FROM`, `ALTER TABLE … DROP` or `… RENAME`, and every
/// `UPDATE` that is a statement (`UPDATE` followed by anything but the `OF` or `ON` of a trigger
/// head, so `UPDATE … SET` and `DO UPDATE SET` are both caught). It also flags `REPLACE INTO` and
/// `OR REPLACE`, which delete a conflicting row: a strengthening of D5, never a relaxation.
fn destructive_statements(sql: &str) -> Vec<(usize, String)> {
    let tokens = tokenize(sql);
    let mut found = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        let next = tokens.get(index + 1).map(|t| t.text.as_str());
        let previous = index
            .checked_sub(1)
            .and_then(|i| tokens.get(i))
            .map(|t| t.text.as_str());
        let finding = match token.text.as_str() {
            "DROP" => Some(format!("DROP {}", next.unwrap_or(""))),
            "RENAME" => Some("ALTER TABLE … RENAME".to_owned()),
            "DELETE" if next == Some("FROM") => Some("DELETE FROM".to_owned()),
            "UPDATE" if !matches!(next, Some("OF") | Some("ON")) => {
                Some(format!("UPDATE {}", next.unwrap_or("")))
            }
            "REPLACE" if next == Some("INTO") || previous == Some("OR") => {
                Some("REPLACE".to_owned())
            }
            _ => None,
        };
        if let Some(description) = finding {
            found.push((token.line, description));
        }
    }
    found
}

#[test]
fn destructive_statement_scanner_catches_each_form() {
    // The scanner must not be vacuous: each D5 form is caught, and the trigger heads, comments and
    // string literals of a legitimate migration are not.
    let flagged = [
        "DROP TABLE jobs;",
        "drop index jobs_by_key;",
        "DROP VIEW v;",
        "DROP TRIGGER t;",
        "DELETE FROM jobs WHERE job_id = 1;",
        "ALTER TABLE jobs DROP COLUMN owner_pid;",
        "ALTER TABLE jobs RENAME TO old_jobs;",
        "ALTER TABLE jobs RENAME COLUMN owner_pid TO pid;",
        "UPDATE jobs SET status = 'FAILED';",
        "UPDATE OR ROLLBACK jobs SET status = 'FAILED';",
        "INSERT INTO t (a) VALUES (1) ON CONFLICT (a) DO UPDATE SET a = 2;",
        "INSERT OR REPLACE INTO t (a) VALUES (1);",
        "REPLACE INTO t (a) VALUES (1);",
        "CREATE TRIGGER t AFTER INSERT ON a BEGIN DELETE FROM b; END;",
        "CREATE TRIGGER t AFTER INSERT ON a BEGIN UPDATE b SET c = 1; END;",
    ];
    for sql in flagged {
        assert!(
            !destructive_statements(sql).is_empty(),
            "the scanner misses `{sql}`"
        );
    }
    let clean = [
        "CREATE TRIGGER t BEFORE UPDATE OF status ON jobs BEGIN SELECT RAISE(ABORT, 'x'); END;",
        "CREATE TRIGGER t BEFORE UPDATE ON jobs BEGIN SELECT RAISE(ABORT, 'x'); END;",
        "CREATE TRIGGER t BEFORE DELETE ON jobs BEGIN SELECT RAISE(ABORT, 'x'); END;",
        "-- DROP TABLE jobs; DELETE FROM jobs; UPDATE jobs SET a = 1\nCREATE TABLE a (b TEXT);",
        "/* DROP TABLE jobs */ CREATE TABLE a (b TEXT);",
        "SELECT RAISE(ABORT, 'do not DROP or DELETE FROM or UPDATE x SET y or RENAME');",
        "CREATE TABLE a (b TEXT CHECK (replace(b, 'x', 'y') = b));",
        "CREATE TABLE \"drop\" (\"update\" TEXT);",
    ];
    for sql in clean {
        assert_eq!(
            destructive_statements(sql),
            Vec::<(usize, String)>::new(),
            "the scanner flags `{sql}`"
        );
    }
}

#[test]
fn no_unapproved_destructive_statement() {
    let names = sql_files(&migrations_dir());
    assert!(!names.is_empty());
    let mut violations: BTreeMap<String, Vec<(usize, String)>> = BTreeMap::new();
    for name in names {
        if APPROVED_DESTRUCTIVE_MIGRATIONS.contains(&name.as_str()) {
            continue;
        }
        let sql = std::fs::read_to_string(migrations_dir().join(&name)).expect("readable");
        let found = destructive_statements(&sql);
        if !found.is_empty() {
            violations.insert(name, found);
        }
    }
    assert!(
        violations.is_empty(),
        "destructive statements need the owner's approval (ADR-015 D5): {violations:?}"
    );
}

#[sqlx::test]
async fn status_pending() {
    let dir = TempDir::new("status-pending");
    let db = Database::open(&dir.db_path(), DbMode::Create)
        .await
        .expect("a new database opens");
    MIGRATOR.run_to(1, db.pool()).await.expect("0001 applies");
    let status = schema::status(&db).await.expect("status reads");
    assert_eq!(
        status,
        SchemaStatus::Pending {
            applied: Some(1),
            pending: vec![2]
        }
    );
    let refusal = status
        .require_current()
        .expect_err("a pending store is not current");
    assert_eq!(
        refusal,
        PersistenceError::Schema(SchemaProblem::Pending {
            applied: Some(1),
            pending: vec![2]
        })
    );
    assert_eq!(
        refusal.engine_error().code,
        EngineErrorCode::UnsupportedVersion
    );
    assert!(refusal.engine_error().message.contains("grid migrate"));
    assert_eq!(
        refusal.terminal_state(),
        Some(TerminalState::DatabaseFailure)
    );
    db.close().await;

    // A blank file is not initialized, and an absent file is missing; neither is created.
    let blank = TempDir::new("status-blank");
    let db = Database::open(&blank.db_path(), DbMode::Create)
        .await
        .expect("a new database opens");
    let status = schema::status(&db).await.expect("status reads");
    assert_eq!(status, SchemaStatus::NotInitialized);
    let refusal = status.require_current().expect_err("not current");
    assert_eq!(
        refusal.engine_error().code,
        EngineErrorCode::UnsupportedVersion
    );
    db.close().await;

    let absent = TempDir::new("status-absent");
    let missing = Database::open(&absent.db_path(), DbMode::ReadWrite)
        .await
        .expect_err("an absent database does not open");
    assert_eq!(
        missing,
        PersistenceError::Schema(SchemaProblem::Missing {
            file: "grid.db".to_owned()
        })
    );
    assert_eq!(
        missing.engine_error().code,
        EngineErrorCode::UnsupportedVersion
    );
    assert_eq!(
        missing.terminal_state(),
        Some(TerminalState::DatabaseFailure)
    );
    assert!(!absent.db_path().exists(), "nothing was created");
}

#[sqlx::test]
async fn status_newer() {
    let dir = TempDir::new("status-newer");
    let db = migrated(&dir).await;
    sqlx::query(
        "INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time) \
         VALUES (9999, 'from a newer build', 1, X'00', 0)",
    )
    .execute(db.pool())
    .await
    .expect("the record accepts a row");
    let before = dump_all(&db).await;

    let refusal = schema::status(&db)
        .await
        .expect_err("a newer store refuses");
    assert_eq!(
        refusal,
        PersistenceError::Schema(SchemaProblem::Newer { version: 9999 })
    );
    assert_eq!(
        refusal.engine_error().code,
        EngineErrorCode::UnsupportedVersion
    );
    assert_eq!(
        refusal.terminal_state(),
        Some(TerminalState::DatabaseFailure)
    );

    let applied = schema::apply(&db).await.expect_err("apply refuses too");
    assert_eq!(applied, refusal);
    assert_eq!(dump_all(&db).await, before, "the refusal wrote nothing");
    db.close().await;
}

#[sqlx::test]
async fn status_checksum_mismatch() {
    let dir = TempDir::new("status-checksum");
    let db = migrated(&dir).await;
    sqlx::query("UPDATE _sqlx_migrations SET checksum = X'00' WHERE version = 2")
        .execute(db.pool())
        .await
        .expect("the record is writable in a test");
    let before = dump_all(&db).await;

    let refusal = schema::status(&db).await.expect_err("a mismatch refuses");
    assert_eq!(
        refusal,
        PersistenceError::Schema(SchemaProblem::ChecksumMismatch { version: 2 })
    );
    assert_eq!(refusal.engine_error().code, EngineErrorCode::StateCorrupt);
    assert_eq!(
        refusal.terminal_state(),
        Some(TerminalState::DatabaseFailure)
    );
    assert_eq!(
        schema::apply(&db).await.expect_err("apply refuses"),
        refusal
    );
    assert_eq!(dump_all(&db).await, before, "the refusal wrote nothing");
    db.close().await;
}

#[sqlx::test]
async fn status_dirty() {
    let dir = TempDir::new("status-dirty");
    let db = migrated(&dir).await;
    sqlx::query("UPDATE _sqlx_migrations SET success = 0 WHERE version = 2")
        .execute(db.pool())
        .await
        .expect("the record is writable in a test");
    let before = dump_all(&db).await;

    let refusal = schema::status(&db)
        .await
        .expect_err("a dirty store refuses");
    assert_eq!(
        refusal,
        PersistenceError::Schema(SchemaProblem::Dirty { version: 2 })
    );
    assert_eq!(refusal.engine_error().code, EngineErrorCode::StateCorrupt);
    assert_eq!(
        refusal.terminal_state(),
        Some(TerminalState::DatabaseFailure)
    );
    assert_eq!(
        schema::apply(&db).await.expect_err("apply refuses"),
        refusal
    );
    assert_eq!(dump_all(&db).await, before, "the refusal wrote nothing");
    db.close().await;
}

#[sqlx::test]
async fn status_unrecorded_schema_refuses() {
    let dir = TempDir::new("status-unrecorded");
    let db = Database::open(&dir.db_path(), DbMode::Create)
        .await
        .expect("a new database opens");
    exec(&db, "CREATE TABLE foreign_table (a INTEGER)").await;
    let refusal = schema::status(&db)
        .await
        .expect_err("an unknown schema refuses");
    assert_eq!(refusal, PersistenceError::Schema(SchemaProblem::Unrecorded));
    assert_eq!(refusal.engine_error().code, EngineErrorCode::StateCorrupt);
    assert_eq!(
        schema::apply(&db).await.expect_err("apply refuses"),
        refusal
    );
    assert!(
        !table_names(&db).await.contains("_sqlx_migrations"),
        "the refusal created no migration record"
    );
    db.close().await;
}

/// The bytes of every file in `dir`, by name.
fn files_in(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    std::fs::read_dir(dir)
        .expect("readable")
        .map(|entry| {
            let entry = entry.expect("readable entry");
            let name = entry.file_name().into_string().expect("UTF-8 name");
            let bytes = std::fs::read(entry.path()).expect("readable file");
            (name, bytes)
        })
        .collect()
}

#[sqlx::test]
async fn status_does_not_write() {
    let dir = TempDir::new("status-readonly");
    let db = Database::open(&dir.db_path(), DbMode::Create)
        .await
        .expect("a new database opens");
    MIGRATOR.run_to(1, db.pool()).await.expect("0001 applies");
    db.close().await;
    let before = files_in(dir.path());
    assert!(before.contains_key("grid.db"));

    for mode in [DbMode::ReadOnly, DbMode::ReadWrite] {
        let db = Database::open(&dir.db_path(), mode)
            .await
            .expect("the store opens");
        assert_eq!(
            schema::status(&db).await.expect("status reads"),
            SchemaStatus::Pending {
                applied: Some(1),
                pending: vec![2]
            }
        );
        db.close().await;
        let after = files_in(dir.path());
        assert_eq!(
            after.get("grid.db"),
            before.get("grid.db"),
            "status in {mode:?} mode left the database file byte-identical"
        );
        let wal_bytes = after.get("grid.db-wal").map_or(0, Vec::len);
        assert_eq!(wal_bytes, 0, "status in {mode:?} mode wrote no WAL frame");
    }
}

#[sqlx::test]
async fn sqlite_supports_strict_and_json() {
    let dir = TempDir::new("sqlite-features");
    let db = Database::open(&dir.db_path(), DbMode::Create)
        .await
        .expect("a new database opens");
    let version: String = sqlx::query_scalar("SELECT sqlite_version()")
        .fetch_one(db.pool())
        .await
        .expect("sqlite_version() runs");
    let parts: Vec<u32> = version
        .split('.')
        .map(|part| part.parse().expect("numeric version part"))
        .collect();
    assert!(
        (parts[0], parts[1]) >= (3, 38),
        "SQLite {version} predates STRICT tables (3.37) and built-in JSON (3.38)"
    );
    exec(&db, "CREATE TABLE strict_probe (a INTEGER NOT NULL) STRICT").await;
    let coerced = sqlx::query("INSERT INTO strict_probe (a) VALUES ('not a number')")
        .execute(db.pool())
        .await;
    assert!(coerced.is_err(), "a STRICT table refuses a mistyped value");
    let valid: i64 = sqlx::query_scalar("SELECT json_valid('{\"a\":1}')")
        .fetch_one(db.pool())
        .await
        .expect("json_valid runs");
    assert_eq!(valid, 1);
    db.close().await;
}

#[sqlx::test]
async fn backup_into_writes_a_consistent_copy() {
    let dir = TempDir::new("backup");
    let db = seeded(&dir).await;
    let relative = Path::new("backups").join("grid-v2-1700000000.db");
    schema::backup_into(&db, dir.path(), &relative)
        .await
        .expect("the copy is written");

    let copy = Database::open(&dir.path().join(&relative), DbMode::ReadOnly)
        .await
        .expect("the copy opens");
    assert_eq!(
        schema::status(&copy).await.expect("status reads"),
        SchemaStatus::Current { head: 2 }
    );
    assert_eq!(
        dump_all(&copy).await,
        dump_all(&db).await,
        "the copy is exact"
    );
    copy.close().await;

    let again = schema::backup_into(&db, dir.path(), &relative)
        .await
        .expect_err("a copy is never overwritten");
    assert!(
        matches!(
            &again,
            PersistenceError::Io { kind: std::io::ErrorKind::AlreadyExists, path: Some(path), .. }
                if path == "backups/grid-v2-1700000000.db"
        ),
        "{again:?}"
    );
    let escaping = schema::backup_into(&db, dir.path(), Path::new("../outside.db"))
        .await
        .expect_err("a path outside the data directory is refused");
    assert!(matches!(escaping, PersistenceError::InvalidInput { .. }));
    let absolute = schema::backup_into(&db, dir.path(), &dir.path().join("abs.db"))
        .await
        .expect_err("an absolute path is refused");
    assert!(matches!(absolute, PersistenceError::InvalidInput { .. }));
    assert!(
        !absolute
            .to_string()
            .contains(&*dir.path().to_string_lossy()),
        "the message names no absolute path: {absolute}"
    );
    db.close().await;
}
