//! The SQLite connection and the write transaction (engine-spec §8.13; work package P1-02 D17).
//!
//! Every connection runs in WAL mode with `synchronous = FULL`, enforced foreign keys and a
//! five-second busy timeout. Every write goes through [`Database::begin_write`], which starts the
//! transaction with `BEGIN IMMEDIATE`: the write lock is taken at the start, so two writers
//! serialize at `BEGIN` instead of failing at their first write, and readers are never blocked
//! (WAL). No write transaction spans a computation (D17).

use std::io;
use std::path::Path;
use std::time::Duration;

use sqlx::sqlite::{
    SqliteConnectOptions, SqliteConnection, SqliteJournalMode, SqlitePool, SqlitePoolOptions,
    SqliteSynchronous,
};
use sqlx::{Sqlite, Transaction};

use crate::error::{PersistenceError, SchemaProblem};

/// The busy timeout of every connection (work package P1-02 D17).
pub const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// The pool size of a read-write store.
const READ_WRITE_POOL: u32 = 4;
/// The pool size of a read-only store.
const READ_ONLY_POOL: u32 = 2;

/// How [`Database::open`] opens the file. **Closed** enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DbMode {
    /// Read only. The file must exist; nothing is written, not even the journal mode.
    ReadOnly,
    /// Read and write. The file must exist; WAL is set.
    ReadWrite,
    /// Read and write, creating the file when it is absent; WAL is set. The directory must exist.
    /// Only `grid migrate` creates a store (ADR-015 D3).
    Create,
}

/// An open SQLite store: a connection pool with the D17 settings.
///
/// Reads may use [`Database::pool`] directly; every write uses [`Database::begin_write`].
#[derive(Debug, Clone)]
pub struct Database {
    pool: SqlitePool,
    mode: DbMode,
    file: String,
}

impl Database {
    /// Opens the store at `path` (work package P1-02 D17).
    ///
    /// The pragmas are `journal_mode = WAL` (except [`DbMode::ReadOnly`], which leaves the file
    /// untouched), `synchronous = FULL`, `foreign_keys = ON` and `busy_timeout` = 5 s. After
    /// connecting, `PRAGMA foreign_keys` is read back and must be 1. The file path is given to
    /// SQLite as a file name, never parsed as a URL.
    ///
    /// # Errors
    ///
    /// - [`SchemaProblem::Missing`] when the mode is not [`DbMode::Create`] and the file does not
    ///   exist; nothing is created.
    /// - [`PersistenceError::Io`] when the file's metadata cannot be read.
    /// - [`PersistenceError::Database`], [`PersistenceError::Busy`] or
    ///   [`PersistenceError::StateCorrupt`] when SQLite cannot open the file, or foreign keys are
    ///   not enforced.
    pub async fn open(path: &Path, mode: DbMode) -> Result<Self, PersistenceError> {
        let file = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        if mode != DbMode::Create {
            match std::fs::metadata(path) {
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    return Err(PersistenceError::Schema(SchemaProblem::Missing { file }));
                }
                Err(error) => {
                    return Err(PersistenceError::io(
                        "reading the database file's metadata",
                        &error,
                        Some(file),
                    ));
                }
            }
        }

        let mut options = SqliteConnectOptions::new()
            .filename(path)
            .foreign_keys(true)
            .synchronous(SqliteSynchronous::Full)
            .busy_timeout(BUSY_TIMEOUT);
        let pool_size = match mode {
            DbMode::ReadOnly => {
                options = options.read_only(true);
                READ_ONLY_POOL
            }
            DbMode::ReadWrite => {
                options = options.journal_mode(SqliteJournalMode::Wal);
                READ_WRITE_POOL
            }
            DbMode::Create => {
                options = options
                    .journal_mode(SqliteJournalMode::Wal)
                    .create_if_missing(true);
                READ_WRITE_POOL
            }
        };

        let pool = SqlitePoolOptions::new()
            .max_connections(pool_size)
            .connect_with(options)
            .await
            .map_err(|error| PersistenceError::from_sqlx("opening the database", error))?;

        // Runtime query: a PRAGMA has no schema for the query! macros to check.
        let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
            .fetch_one(&pool)
            .await
            .map_err(|error| PersistenceError::from_sqlx("reading PRAGMA foreign_keys", error))?;
        if foreign_keys != 1 {
            pool.close().await;
            return Err(PersistenceError::Database {
                context: "enabling foreign-key enforcement (PRAGMA foreign_keys is not 1)",
                sqlite_code: None,
            });
        }

        Ok(Self { pool, mode, file })
    }

    /// The mode the store was opened in.
    pub const fn mode(&self) -> DbMode {
        self.mode
    }

    /// The database file name, without its directory, for messages.
    pub fn file_name(&self) -> &str {
        &self.file
    }

    /// The connection pool, for reads. A write through it bypasses `BEGIN IMMEDIATE`; every write
    /// uses [`Database::begin_write`] instead (work package P1-02 D17).
    pub const fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Starts a write transaction with `BEGIN IMMEDIATE` (engine-spec §8.13; work package P1-02
    /// D17). Dropping the [`WriteTx`] without committing rolls it back.
    ///
    /// # Errors
    ///
    /// - [`PersistenceError::InvalidInput`] on a [`DbMode::ReadOnly`] store, which never writes.
    /// - [`PersistenceError::Busy`] when another writer holds the write lock past the busy
    ///   timeout.
    /// - [`PersistenceError::Database`] for any other SQLite failure.
    pub async fn begin_write(&self) -> Result<WriteTx, PersistenceError> {
        if self.mode == DbMode::ReadOnly {
            return Err(PersistenceError::invalid_input(
                "write transaction",
                "the store was opened read-only",
            ));
        }
        let tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|error| PersistenceError::from_sqlx("beginning a write transaction", error))?;
        Ok(WriteTx { tx })
    }

    /// Closes every connection of the pool and waits for them to finish (engine-spec §8.11
    /// shutdown step 4).
    pub async fn close(self) {
        self.pool.close().await;
    }
}

/// A write transaction started with `BEGIN IMMEDIATE` (work package P1-02 D17).
///
/// Commit it with [`WriteTx::commit`]; dropping it, or [`WriteTx::rollback`], rolls it back, so a
/// failure path that returns early leaves nothing behind.
#[derive(Debug)]
pub struct WriteTx {
    tx: Transaction<'static, Sqlite>,
}

impl WriteTx {
    /// The transaction's connection, on which the store modules execute their statements.
    pub fn connection(&mut self) -> &mut SqliteConnection {
        &mut self.tx
    }

    /// Commits the transaction.
    ///
    /// # Errors
    ///
    /// [`PersistenceError::Busy`], [`PersistenceError::Database`] or
    /// [`PersistenceError::Constraint`] when SQLite refuses the commit; the transaction is then
    /// rolled back.
    pub async fn commit(self) -> Result<(), PersistenceError> {
        self.tx
            .commit()
            .await
            .map_err(|error| PersistenceError::from_sqlx("committing a write transaction", error))
    }

    /// Rolls the transaction back explicitly.
    ///
    /// # Errors
    ///
    /// [`PersistenceError::Database`] when SQLite fails to roll back.
    pub async fn rollback(self) -> Result<(), PersistenceError> {
        self.tx
            .rollback()
            .await
            .map_err(|error| PersistenceError::from_sqlx("rolling back a write transaction", error))
    }
}
