//! The embedded migrations, the read-only schema classification and the applier (ADR-015 D1–D3,
//! D6; owner ruling R3).
//!
//! - [`MIGRATOR`] embeds `migrations/` at compile time; `build.rs` re-embeds it when the directory
//!   changes (D1).
//! - [`status`] compares the database's `_sqlx_migrations` rows with the embedded list, version by
//!   version and checksum by checksum, and writes nothing (D2). Only [`SchemaStatus::Current`] lets
//!   an engine invocation proceed.
//! - [`apply`] is what `grid migrate` runs: it refuses a newer, mismatched, dirty or unrecorded
//!   database without writing, applies each pending migration in its own transaction, then checks
//!   `PRAGMA foreign_key_check` and `PRAGMA quick_check` (D3). The store lock is the caller's.
//! - [`backup_into`] writes the pre-migration copy with `VACUUM INTO` (D3 step 4).
//!
//! The version mechanism is `_sqlx_migrations` with its checksums; `schema_meta` is kept (D6).

use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::path::{Component, Path};

use sqlx::migrate::{MigrateError, Migrator};

use crate::db::{Database, DbMode};
use crate::error::{PersistenceError, SchemaProblem};

/// The migrations of `migrations/`, embedded at compile time (ADR-015 D1).
pub static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

/// The classification of a store's schema that does not refuse outright (ADR-015 D2).
/// **Closed** enum.
///
/// The refusing cases (newer, checksum mismatch, dirty, unrecorded, out of order) are errors of
/// [`status`], not values of this enum.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SchemaStatus {
    /// An empty database: no schema object and no migration record.
    NotInitialized,
    /// Every embedded migration is applied with its checksum, and nothing else is.
    Current {
        /// The highest embedded (and applied) version.
        head: i64,
    },
    /// A prefix of the embedded migrations is applied; the rest are pending.
    Pending {
        /// The highest applied version; `None` when the record exists but is empty.
        applied: Option<i64>,
        /// The embedded versions not applied yet, ascending. Never empty.
        pending: Vec<i64>,
    },
}

impl SchemaStatus {
    /// The head version when the schema is current; otherwise the refusal an engine invocation
    /// reports (ADR-015 D2: "run `grid migrate`").
    ///
    /// # Errors
    ///
    /// [`SchemaProblem::NotInitialized`] or [`SchemaProblem::Pending`].
    pub fn require_current(self) -> Result<i64, PersistenceError> {
        match self {
            SchemaStatus::Current { head } => Ok(head),
            SchemaStatus::NotInitialized => {
                Err(PersistenceError::Schema(SchemaProblem::NotInitialized))
            }
            SchemaStatus::Pending { applied, pending } => {
                Err(PersistenceError::Schema(SchemaProblem::Pending {
                    applied,
                    pending,
                }))
            }
        }
    }
}

/// The versions of the embedded up migrations, ascending (ADR-015 D1).
pub fn embedded_versions() -> Vec<i64> {
    MIGRATOR
        .iter()
        .filter(|migration| !migration.migration_type.is_down_migration())
        .map(|migration| migration.version)
        .collect()
}

/// The embedded up migrations by version, with their checksums.
fn embedded_checksums() -> BTreeMap<i64, &'static [u8]> {
    MIGRATOR
        .iter()
        .filter(|migration| !migration.migration_type.is_down_migration())
        .map(|migration| (migration.version, migration_checksum(migration)))
        .collect()
}

/// The checksum of an embedded migration. `MIGRATOR` is a static whose migrations borrow
/// `'static` data (`sqlx::migrate!` emits them as constants), so the borrow lives as long.
fn migration_checksum(migration: &'static sqlx::migrate::Migration) -> &'static [u8] {
    &migration.checksum
}

/// Classifies the store's schema without writing anything (ADR-015 D2; owner ruling R3).
///
/// 1. No `_sqlx_migrations` table: [`SchemaStatus::NotInitialized`] when the database holds no
///    schema object at all, otherwise [`SchemaProblem::Unrecorded`].
/// 2. A row recorded as failed: [`SchemaProblem::Dirty`].
/// 3. An applied version that is not embedded: [`SchemaProblem::Newer`].
/// 4. An applied version whose checksum differs: [`SchemaProblem::ChecksumMismatch`].
/// 5. An embedded version missing below the highest applied one: [`SchemaProblem::OutOfOrder`].
/// 6. Otherwise [`SchemaStatus::Current`] or [`SchemaStatus::Pending`].
///
/// # Errors
///
/// The refusing [`SchemaProblem`]s above, and [`PersistenceError::Database`] or
/// [`PersistenceError::StateCorrupt`] when the record cannot be read.
pub async fn status(db: &Database) -> Result<SchemaStatus, PersistenceError> {
    let pool = db.pool();
    let record = sqlx::query!(
        r#"SELECT COUNT(*) AS "count!: i64" FROM sqlite_master
           WHERE type = 'table' AND name = '_sqlx_migrations'"#
    )
    .fetch_one(pool)
    .await
    .map_err(|error| PersistenceError::from_sqlx("looking for the migration record", error))?;

    if record.count == 0 {
        let objects = sqlx::query!(
            r#"SELECT COUNT(*) AS "count!: i64" FROM sqlite_master
               WHERE name NOT LIKE 'sqlite\_%' ESCAPE '\'"#
        )
        .fetch_one(pool)
        .await
        .map_err(|error| PersistenceError::from_sqlx("counting schema objects", error))?;
        return if objects.count == 0 {
            Ok(SchemaStatus::NotInitialized)
        } else {
            Err(PersistenceError::Schema(SchemaProblem::Unrecorded))
        };
    }

    let rows = sqlx::query!(
        r#"SELECT version AS "version!: i64", checksum AS "checksum!: Vec<u8>",
                  success AS "success!: bool"
           FROM _sqlx_migrations ORDER BY version"#
    )
    .fetch_all(pool)
    .await
    .map_err(|error| PersistenceError::from_sqlx("reading the migration record", error))?;

    if let Some(row) = rows.iter().find(|row| !row.success) {
        return Err(PersistenceError::Schema(SchemaProblem::Dirty {
            version: row.version,
        }));
    }

    let embedded = embedded_checksums();
    for row in &rows {
        match embedded.get(&row.version) {
            None => {
                return Err(PersistenceError::Schema(SchemaProblem::Newer {
                    version: row.version,
                }));
            }
            Some(checksum) if *checksum != row.checksum.as_slice() => {
                return Err(PersistenceError::Schema(SchemaProblem::ChecksumMismatch {
                    version: row.version,
                }));
            }
            Some(_) => {}
        }
    }

    let applied = rows.iter().map(|row| row.version).max();
    let pending: Vec<i64> = embedded
        .keys()
        .copied()
        .filter(|version| !rows.iter().any(|row| row.version == *version))
        .collect();
    if let (Some(highest), Some(lowest_pending)) = (applied, pending.first()) {
        if *lowest_pending < highest {
            return Err(PersistenceError::Schema(SchemaProblem::OutOfOrder {
                version: *lowest_pending,
            }));
        }
    }

    match (pending.is_empty(), embedded.keys().next_back()) {
        (true, Some(head)) => Ok(SchemaStatus::Current { head: *head }),
        (true, None) => Err(PersistenceError::state_corrupt(
            "this build embeds no migration, so no schema can be current",
        )),
        (false, _) => Ok(SchemaStatus::Pending { applied, pending }),
    }
}

/// Applies every pending migration and checks the result (ADR-015 D3 steps 3, 5 and 6). The
/// caller holds the exclusive store lock (owner ruling R5) and has taken the pre-migration copy
/// ([`backup_into`]).
///
/// It refuses, writing nothing, a newer, mismatched, dirty, unrecorded or out-of-order database
/// ([`status`]). Each migration runs in its own transaction (sqlx's applier). Afterwards
/// `PRAGMA foreign_key_check` must report nothing, `PRAGMA quick_check` must report `ok`, and the
/// schema must be current. Returns the versions it applied, ascending; empty on a current store,
/// which makes it idempotent.
///
/// # Errors
///
/// - the refusing [`SchemaProblem`]s of [`status`];
/// - [`PersistenceError::InvalidInput`] on a [`DbMode::ReadOnly`] store;
/// - [`PersistenceError::Database`], [`PersistenceError::Busy`] or
///   [`PersistenceError::Constraint`] when a migration fails (its transaction rolls back);
/// - [`PersistenceError::StateCorrupt`] when a check fails after applying.
pub async fn apply(db: &Database) -> Result<Vec<i64>, PersistenceError> {
    let pending = match status(db).await? {
        SchemaStatus::Current { .. } => return Ok(Vec::new()),
        SchemaStatus::NotInitialized => embedded_versions(),
        SchemaStatus::Pending { pending, .. } => pending,
    };
    if db.mode() == DbMode::ReadOnly {
        return Err(PersistenceError::invalid_input(
            "migration",
            "the store was opened read-only",
        ));
    }

    MIGRATOR.run(db.pool()).await.map_err(migrate_error)?;

    check_integrity(db).await?;
    match status(db).await? {
        SchemaStatus::Current { .. } => Ok(pending),
        other => Err(PersistenceError::state_corrupt(format!(
            "the schema is not current after applying migrations: {other:?}"
        ))),
    }
}

/// Checks that `PRAGMA foreign_key_check` reports no row and `PRAGMA quick_check` reports `ok`
/// (ADR-015 D3 step 6).
///
/// # Errors
///
/// [`PersistenceError::StateCorrupt`] naming the first violation, or
/// [`PersistenceError::Database`] when a check cannot run.
pub async fn check_integrity(db: &Database) -> Result<(), PersistenceError> {
    // Runtime queries: a PRAGMA has no schema for the query! macros to check.
    let violations: Vec<(String,)> =
        sqlx::query_as("SELECT \"table\" FROM pragma_foreign_key_check")
            .fetch_all(db.pool())
            .await
            .map_err(|error| {
                PersistenceError::from_sqlx("running PRAGMA foreign_key_check", error)
            })?;
    if let Some((table,)) = violations.first() {
        return Err(PersistenceError::state_corrupt(format!(
            "PRAGMA foreign_key_check reports {} violation(s), the first in table `{table}`",
            violations.len()
        )));
    }

    let quick: Vec<(String,)> = sqlx::query_as("PRAGMA quick_check")
        .fetch_all(db.pool())
        .await
        .map_err(|error| PersistenceError::from_sqlx("running PRAGMA quick_check", error))?;
    let healthy = quick.len() == 1 && quick.first().is_some_and(|(result,)| result == "ok");
    if !healthy {
        let first = quick
            .first()
            .map(|(result,)| result.chars().take(200).collect::<String>())
            .unwrap_or_else(|| "no result".to_owned());
        return Err(PersistenceError::state_corrupt(format!(
            "PRAGMA quick_check reports {} problem(s): {first}",
            quick.len()
        )));
    }
    Ok(())
}

/// Classifies a failure of sqlx's migrator (ADR-015 D2).
fn migrate_error(error: MigrateError) -> PersistenceError {
    match error {
        MigrateError::VersionMissing(version) => {
            PersistenceError::Schema(SchemaProblem::Newer { version })
        }
        MigrateError::VersionMismatch(version) => {
            PersistenceError::Schema(SchemaProblem::ChecksumMismatch { version })
        }
        MigrateError::Dirty(version) => PersistenceError::Schema(SchemaProblem::Dirty { version }),
        MigrateError::Execute(error) | MigrateError::ExecuteMigration(error, _) => {
            PersistenceError::from_sqlx("applying a migration", error)
        }
        _ => PersistenceError::Database {
            context: "applying migrations",
            sqlite_code: None,
        },
    }
}

/// Writes a consistent copy of the database to `root/dest_relative` with `VACUUM INTO`, then
/// syncs it to disk: the pre-migration copy of ADR-015 D3 step 4 (`data/backups/grid-v<applied>-
/// <unix seconds>.db`, named by the caller).
///
/// `dest_relative` is relative to `root` (the data directory) and is the only path any error
/// names. Its parent directories are created. The destination must not exist: a copy is never
/// overwritten.
///
/// # Errors
///
/// - [`PersistenceError::InvalidInput`] when `dest_relative` is absolute, empty or contains `..`;
/// - [`PersistenceError::Io`] when the destination exists or cannot be created or synced;
/// - [`PersistenceError::Database`] or [`PersistenceError::Busy`] when `VACUUM INTO` fails.
pub async fn backup_into(
    db: &Database,
    root: &Path,
    dest_relative: &Path,
) -> Result<(), PersistenceError> {
    let plain = dest_relative
        .components()
        .all(|component| matches!(component, Component::Normal(_)));
    if !plain || dest_relative.as_os_str().is_empty() {
        // The refused path is not echoed: it may be absolute (engine-output-contract §8 rule 5).
        return Err(PersistenceError::invalid_input(
            "backup path",
            "it must be a non-empty path relative to the data directory, without `..`",
        ));
    }
    let shown = dest_relative.to_string_lossy().replace('\\', "/");
    let dest = root.join(dest_relative);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            PersistenceError::io("creating the backup directory", &error, Some(shown.clone()))
        })?;
    }
    match std::fs::symlink_metadata(&dest) {
        Ok(_) => {
            return Err(PersistenceError::Io {
                context: "writing the pre-migration copy (it already exists)",
                kind: std::io::ErrorKind::AlreadyExists,
                path: Some(shown),
            });
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(PersistenceError::io(
                "checking the backup path",
                &error,
                Some(shown),
            ));
        }
    }

    let dest_text = dest.to_str().ok_or_else(|| {
        PersistenceError::invalid_input("backup path", format!("`{shown}` is not valid UTF-8"))
    })?;
    // Runtime query: VACUUM takes its destination as a bound value and has no schema to check.
    sqlx::query("VACUUM INTO ?")
        .bind(dest_text)
        .execute(db.pool())
        .await
        .map_err(|error| PersistenceError::from_sqlx("writing the pre-migration copy", error))?;

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&dest)
        .map_err(|error| {
            PersistenceError::io(
                "opening the pre-migration copy",
                &error,
                Some(shown.clone()),
            )
        })?;
    file.sync_all().map_err(|error| {
        PersistenceError::io("syncing the pre-migration copy", &error, Some(shown))
    })?;
    Ok(())
}
