//! `grid-persistence` — SQLx, migrations, SQLite. No business logic.
//!
//! Crate boundary is defined in `docs/CLAUDE.md`; see `docs/02-adr/001-repo-bootstrap-decisions.md`.
//!
//! P1-00 establishes only the SQLx toolchain: one infrastructure table, one compile-time-checked
//! query, and the committed offline cache (ADR-002). The durable domain schema is P1-02.

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
        // Derived from the migrator, so this cannot silently pass on a stale literal.
        let version = applied_schema_version(&pool)
            .await
            .expect("reading the applied migration version should succeed");
        assert_eq!(version, Some(1), "migration 0001 should be applied");
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
