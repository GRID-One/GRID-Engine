//! Shared helpers of the `grid-persistence` integration tests (work package P1-02).
//!
//! Every database and artifact directory lives in a unique directory under
//! `CARGO_TARGET_TMPDIR`, never in the crate directory (P1-02 "Untracked files"). Test set-up uses
//! runtime `sqlx::query`; `query!` stays in library code (P1-02 "`query!`").

// Each integration-test crate compiles this module and uses a subset of the helpers.
#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use grid_persistence::error::PersistenceError;
use grid_persistence::{schema, Database, DbMode, Sha256Hex};
use sqlx::AssertSqlSafe;

/// A process-wide counter that keeps directory names unique.
static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A unique scratch directory under `CARGO_TARGET_TMPDIR`, removed on drop.
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// Creates `<CARGO_TARGET_TMPDIR>/grid-persistence/<label>-<pid>-<n>`.
    pub fn new(label: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("grid-persistence")
            .join(format!("{label}-{}-{n}", std::process::id()));
        if path.exists() {
            std::fs::remove_dir_all(&path).expect("a stale scratch directory can be removed");
        }
        std::fs::create_dir_all(&path).expect("the scratch directory can be created");
        Self { path }
    }

    /// The directory.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// `<dir>/grid.db`.
    pub fn db_path(&self) -> PathBuf {
        self.path.join("grid.db")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        // Best effort: the directory is under target/, so a leftover is harmless.
        let _ignored = std::fs::remove_dir_all(&self.path);
    }
}

/// Opens a new database in `dir` and migrates it to head.
pub async fn migrated(dir: &TempDir) -> Database {
    let db = Database::open(&dir.db_path(), DbMode::Create)
        .await
        .expect("a new database opens");
    schema::apply(&db).await.expect("migrations apply");
    db
}

/// Opens a new database in `dir`, migrates it to head and applies every upgrade seed.
pub async fn seeded(dir: &TempDir) -> Database {
    let db = migrated(dir).await;
    for version in schema::embedded_versions() {
        exec(&db, &read_seed(version)).await;
    }
    db
}

/// Runs a SQL script (several statements) and panics on failure.
pub async fn exec(db: &Database, sql: &str) {
    sqlx::raw_sql(AssertSqlSafe(sql.to_owned()))
        .execute(db.pool())
        .await
        .unwrap_or_else(|error| panic!("the script runs: {error}"));
}

/// Runs one statement and returns its classified error, or `None` when it succeeds.
pub async fn try_exec(db: &Database, sql: &str) -> Option<PersistenceError> {
    sqlx::raw_sql(AssertSqlSafe(sql.to_owned()))
        .execute(db.pool())
        .await
        .err()
        .map(|error| PersistenceError::from_sqlx("a test statement", error))
}

/// Asserts that `sql` is refused by a store constraint whose message contains `needle`.
pub async fn assert_refused(db: &Database, sql: &str, needle: &str) {
    match try_exec(db, sql).await {
        Some(PersistenceError::Constraint { detail, .. }) => assert!(
            detail.contains(needle),
            "`{sql}` was refused with `{detail}`, expected a message containing `{needle}`"
        ),
        Some(other) => panic!("`{sql}` failed with {other:?}, expected a constraint failure"),
        None => panic!("`{sql}` succeeded, expected a constraint failure containing `{needle}`"),
    }
}

/// Asserts that `sql` succeeds.
pub async fn assert_accepted(db: &Database, sql: &str) {
    if let Some(error) = try_exec(db, sql).await {
        panic!("`{sql}` was refused: {error:?}");
    }
}

/// `<crate>/../../migrations`.
pub fn migrations_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../migrations")
}

/// `<crate>/tests/fixtures/upgrade`.
pub fn seeds_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/upgrade")
}

/// The upgrade seed of a migration version: `seed_<NNNN>.sql`.
pub fn read_seed(version: i64) -> String {
    let path = seeds_dir().join(format!("seed_{version:04}.sql"));
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("the upgrade seed {} is readable: {error}", path.display()))
}

/// The sha256 of `text`, as 64 lowercase hexadecimal characters.
pub fn sha_hex(text: &str) -> String {
    Sha256Hex::digest(text.as_bytes()).to_hex()
}

/// The values of a job row that satisfies every CHECK, as an `INSERT` statement.
pub fn job_insert(
    stage: &str,
    status: &str,
    data_version: Option<i64>,
    config_json: &str,
    attempt_count: i64,
    max_attempts: i64,
) -> String {
    let hash = sha_hex(config_json);
    let version = data_version.map_or("-".to_owned(), |v| v.to_string());
    let input = data_version.map_or("NULL".to_owned(), |v| v.to_string());
    format!(
        "INSERT INTO jobs (job_type, status, idempotency_key, config_hash, config_json, \
         input_data_version, restartable, attempt_count, max_attempts, not_before, created_at) \
         VALUES ('{stage}', '{status}', '{stage}:{version}:{hash}', '{hash}', '{config_json}', \
         {input}, 1, {attempt_count}, {max_attempts}, 1700000000, 1700000000)"
    )
}

/// Inserts a PENDING job and returns its id.
pub async fn insert_pending_job(db: &Database, stage: &str, config_json: &str) -> i64 {
    sqlx::query(AssertSqlSafe(job_insert(
        stage,
        "PENDING",
        None,
        config_json,
        0,
        3,
    )))
    .execute(db.pool())
    .await
    .unwrap_or_else(|error| panic!("the job inserts: {error}"))
    .last_insert_rowid()
}

/// The names of every table, sorted.
pub async fn table_names(db: &Database) -> BTreeSet<String> {
    sqlx::query_scalar("SELECT name FROM sqlite_master WHERE type = 'table'")
        .fetch_all(db.pool())
        .await
        .expect("sqlite_master is readable")
        .into_iter()
        .collect()
}

/// The number of rows of `table`.
pub async fn row_count(db: &Database, table: &str) -> i64 {
    sqlx::query_scalar(AssertSqlSafe(format!("SELECT COUNT(*) FROM \"{table}\"")))
        .fetch_one(db.pool())
        .await
        .expect("the table is countable")
}

/// Every row of `table` as one `quote()`d text per row, sorted: a value-exact dump that does not
/// depend on column types.
pub async fn dump_table(db: &Database, table: &str) -> Vec<String> {
    let columns: Vec<String> = sqlx::query_scalar("SELECT name FROM pragma_table_info(?)")
        .bind(table)
        .fetch_all(db.pool())
        .await
        .expect("the table's columns are readable");
    assert!(!columns.is_empty(), "table {table} has columns");
    let expression = columns
        .iter()
        .map(|column| format!("quote(\"{column}\")"))
        .collect::<Vec<_>>()
        .join(" || '|' || ");
    let mut rows: Vec<String> = sqlx::query_scalar(AssertSqlSafe(format!(
        "SELECT {expression} FROM \"{table}\""
    )))
    .fetch_all(db.pool())
    .await
    .expect("the table is readable");
    rows.sort();
    rows
}

/// [`dump_table`] for every table except `sqlite_sequence`, whose counters move with any insert.
pub async fn dump_all(db: &Database) -> BTreeMap<String, Vec<String>> {
    let mut dump = BTreeMap::new();
    for table in table_names(db).await {
        if table != "sqlite_sequence" {
            let rows = dump_table(db, &table).await;
            dump.insert(table, rows);
        }
    }
    dump
}
