//! `grid-persistence` — SQLx, migrations, SQLite. No business logic.
//!
//! Crate boundary is defined in `docs/CLAUDE.md`; see `docs/02-adr/001-repo-bootstrap-decisions.md`.
//!
//! P1-00 established the SQLx toolchain: one infrastructure table, one compile-time-checked
//! query, and the committed offline cache (ADR-002). P1-02 builds the durable store on it
//! (engine-spec §8.5, §8.11, §8.13; owner rulings R1–R6; work package P1-02). This crate holds SQL
//! and files and no business logic: `grid-governance` decides, this crate executes
//! compare-and-set writes whose store triggers are a backstop, and `grid-pipeline` reads, plans
//! with governance and has this crate execute the plan inside one write transaction (D1). It
//! depends on `grid-domain`, `sqlx` and `sha2` only.
//!
//! # What this crate contains
//!
//! | Module | Content | Governing text |
//! |---|---|---|
//! | [`error`] | [`PersistenceError`], [`SchemaProblem`], the R6 mapping to `EngineError` and terminal states, message normalization | owner ruling R6; P1-02 "Typed failures" |
//! | [`codec`] | checked id, boolean, instant and enum-text conversions | P1-02 D5 |
//! | [`db`] | [`Database`] with the D17 pragmas and the `BEGIN IMMEDIATE` [`WriteTx`] | engine-spec §8.13; P1-02 D17 |
//! | [`schema`] | the embedded [`MIGRATOR`](schema::MIGRATOR), the read-only schema status, the applier and the pre-migration copy | ADR-015 D1–D3, D6 |
//! | [`hash`] | [`Sha256Hex`] and the streaming hasher | engine-spec §8.11; owner ruling R2 |
//! | [`artifacts`] | the content-addressed [`ArtifactStore`] and the artifact manifest | engine-spec §8.11; DR-C15; P1-02 D16 |
//!
//! The schema itself is `migrations/0002_backbone.sql` (owner ruling R1).

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod artifacts;
pub mod codec;
pub mod db;
pub mod error;
pub mod hash;
pub mod schema;

pub use artifacts::{ArtifactId, ArtifactMeta, ArtifactStore, ManifestEntry, StagedArtifact};
pub use db::{Database, DbMode, WriteTx};
pub use error::{PersistenceError, SchemaProblem};
pub use hash::Sha256Hex;
pub use schema::SchemaStatus;

use sqlx::SqlitePool;

/// Reads a value from the `schema_meta` key-value table, or `None` when the key is absent.
///
/// Errors surface as typed [`sqlx::Error`]; a missing key is a legitimate `Ok(None)` and is
/// never conflated with a failure (alpha-spec.md 12.7).
pub async fn schema_meta_value(
    pool: &SqlitePool,
    key: &str,
) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query!("SELECT value FROM schema_meta WHERE key = ?", key)
        .fetch_optional(pool)
        .await?;
    Ok(row.map(|record| record.value))
}

/// Highest migration version applied to this database, read from the migrator's own bookkeeping.
///
/// Derived, never hand-written: a literal duplicating `_sqlx_migrations` would go stale the
/// moment a later migration landed, and the test asserting it would keep passing on the wrong
/// value. Returns `None` for a database with no migrations applied.
pub async fn applied_schema_version(pool: &SqlitePool) -> Result<Option<i64>, sqlx::Error> {
    let row = sqlx::query!("SELECT MAX(version) AS version FROM _sqlx_migrations")
        .fetch_one(pool)
        .await?;
    Ok(row.version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrations = "../../migrations")]
    async fn migrations_apply_to_a_blank_database(pool: SqlitePool) {
        // Derived from the migrator, so this cannot silently pass on a stale literal. Every
        // embedded version must be applied, not only the highest; the reviewed literal list
        // lives in tests/migrations.rs::embedded_migrations_are_exactly_0001_and_0002.
        let applied: Vec<i64> =
            sqlx::query_scalar("SELECT version FROM _sqlx_migrations ORDER BY version")
                .fetch_all(&pool)
                .await
                .expect("reading the applied migration versions should succeed");
        let embedded = schema::embedded_versions();
        assert!(!embedded.is_empty(), "the build embeds migrations");
        assert_eq!(
            applied, embedded,
            "every embedded migration should be applied"
        );
        let version = applied_schema_version(&pool)
            .await
            .expect("reading the applied migration version should succeed");
        assert_eq!(
            version,
            embedded.last().copied(),
            "the highest applied version is the embedded head"
        );
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn schema_meta_round_trips(pool: SqlitePool) {
        // Runtime-checked `query` rather than the compile-time `query!`: the frozen
        // check-sqlx recipe prepares `--lib` only, so a macro used solely in #[cfg(test)]
        // would never enter the committed cache and could not build offline. Test SETUP
        // does not need compile-time checking; the function under test does.
        sqlx::query("INSERT INTO schema_meta (key, value) VALUES (?, ?)")
            .bind("k")
            .bind("v")
            .execute(&pool)
            .await
            .expect("insert should succeed");
        let got = schema_meta_value(&pool, "k")
            .await
            .expect("read should succeed");
        assert_eq!(got.as_deref(), Some("v"));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn absent_key_is_none_not_an_error(pool: SqlitePool) {
        let value = schema_meta_value(&pool, "no-such-key")
            .await
            .expect("querying an absent key is not a failure");
        assert!(value.is_none());
    }
}
