//! Schema-rule tests of `0002_backbone.sql`: the enum CHECK lists, the append-only and lifecycle
//! triggers, the key formats, the production invariants, the connection pragmas and the codec
//! (work package P1-02 D5–D7, D12–D14, D17, D22; engine-spec §8.8, §8.10).
//!
//! The exhaustive "trigger equals governance table" tests over every pair live in
//! `crates/pipeline/tests/store_policy.rs` (D23); these are the store-side basics.

mod common;

use std::collections::BTreeSet;

use common::{
    assert_accepted, assert_refused, exec, job_insert, migrated, seeded, sha_hex, TempDir,
};
use grid_domain::{
    EngineErrorCode, JobStatus, LockKind, ModelState, StageId, TerminalState, UtcInstant,
};
use grid_persistence::codec::{
    bool_from_sql, bool_to_sql, id_from_sql, id_to_sql, instant_from_sql, instant_to_sql,
    u32_from_sql, StoredEnum,
};
use grid_persistence::error::PersistenceError;
use grid_persistence::{Database, DbMode};

/// The DDL of `table` as SQLite stored it.
async fn ddl(db: &Database, table: &str) -> String {
    sqlx::query_scalar("SELECT sql FROM sqlite_master WHERE type = 'table' AND name = ?")
        .bind(table)
        .fetch_one(db.pool())
        .await
        .unwrap_or_else(|error| panic!("table {table} exists: {error}"))
}

/// The quoted values of the column CHECK `CHECK (<column> IN ('a', 'b', …))` in `ddl`. Exactly
/// one such CHECK must exist.
fn check_list(ddl: &str, column: &str) -> BTreeSet<String> {
    let marker = format!("CHECK ({column} IN (");
    let mut occurrences = ddl.match_indices(&marker);
    let (start, _) = occurrences
        .next()
        .unwrap_or_else(|| panic!("a `{marker}` list exists"));
    assert!(occurrences.next().is_none(), "one `{marker}` list");
    let rest = &ddl[start + marker.len()..];
    let end = rest.find(')').expect("the list closes");
    let list = &rest[..end];
    let mut values = BTreeSet::new();
    let mut parts = list.split('\'');
    // Odd pieces are inside quotes.
    parts.next();
    while let Some(value) = parts.next() {
        assert!(values.insert(value.to_owned()), "{value} is listed once");
        parts.next();
    }
    values
}

/// The stored names of every value of a domain enum.
fn names<T: StoredEnum>() -> BTreeSet<String> {
    T::ALL
        .iter()
        .map(|value| value.stored_name().to_owned())
        .collect()
}

#[sqlx::test]
async fn enum_check_lists_admit_exactly_the_domain_values() {
    let dir = TempDir::new("enum-lists");
    let db = migrated(&dir).await;

    let terminal: BTreeSet<String> = TerminalState::ALL
        .iter()
        .filter(|state| state.is_terminal())
        .map(|state| state.spec_name().to_owned())
        .collect();
    assert_eq!(terminal.len(), 12);

    let cases: [(&str, &str, BTreeSet<String>); 7] = [
        ("jobs", "job_type", names::<StageId>()),
        ("jobs", "status", names::<JobStatus>()),
        ("jobs", "error_code", names::<EngineErrorCode>()),
        ("ingestion_runs", "terminal_state", terminal.clone()),
        ("ingestion_runs", "error_code", names::<EngineErrorCode>()),
        ("model_versions", "state", names::<ModelState>()),
        ("prediction_runs", "lock_kind", names::<LockKind>()),
    ];
    for (table, column, expected) in cases {
        assert_eq!(
            check_list(&ddl(&db, table).await, column),
            expected,
            "{table}.{column} admits exactly the domain values"
        );
    }
    assert_eq!(names::<StageId>().len(), 20);
    assert_eq!(names::<JobStatus>().len(), 6);
    assert_eq!(names::<EngineErrorCode>().len(), 17);
    assert_eq!(names::<ModelState>().len(), 6);

    // Behaviour, not only text: every stage inserts, and near misses are refused.
    for stage in StageId::ALL {
        assert_accepted(
            &db,
            &job_insert(stage.spec_name(), "PENDING", None, "{\"stage\":1}", 0, 3),
        )
        .await;
    }
    for bogus in ["Predict", "predict", "PREDICTION", ""] {
        assert_refused(
            &db,
            &job_insert(bogus, "PENDING", None, "{\"bogus\":1}", 0, 3),
            "CHECK constraint failed",
        )
        .await;
    }

    // Every terminal state and error code is storable on an ingestion run; NOT_RUN and RUNNING
    // are not terminal and are refused.
    let job = common::insert_pending_job(&db, "DATA_FETCH", "{\"ingest\":1}").await;
    let ingestion = |state: &str, code: Option<&str>| {
        let (code, message) = match code {
            Some(code) => (format!("'{code}'"), "'failed'".to_owned()),
            None => ("NULL".to_owned(), "NULL".to_owned()),
        };
        format!(
            "INSERT INTO ingestion_runs (job_id, source_name, offline, network_fetch, started_at, \
             finished_at, terminal_state, error_code, error_message) \
             VALUES ({job}, 'src', 0, 1, 10, 20, '{state}', {code}, {message})"
        )
    };
    for state in &terminal {
        let success = ["SUCCESS", "NO_NEW_DATA", "PARTIAL_SUCCESS"].contains(&state.as_str());
        let code = if success {
            None
        } else {
            Some("SourceUnavailable")
        };
        assert_accepted(&db, &ingestion(state, code)).await;
    }
    for code in EngineErrorCode::ALL {
        assert_accepted(&db, &ingestion("NETWORK_FAILURE", Some(code.name()))).await;
    }
    for state in ["NOT_RUN", "RUNNING", "Success"] {
        assert_refused(&db, &ingestion(state, None), "CHECK constraint failed").await;
    }
    assert_refused(
        &db,
        &ingestion("NETWORK_FAILURE", Some("NetworkFailure")),
        "CHECK constraint failed",
    )
    .await;
    db.close().await;
}

#[sqlx::test]
async fn append_only_tables_reject_update_and_delete() {
    let dir = TempDir::new("append-only");
    let db = seeded(&dir).await;
    let append_only: [(&str, &str); 11] = [
        ("engine_settings", "value"),
        ("job_dependencies", "allow_skipped"),
        ("ingestion_runs", "source_name"),
        ("artifact_manifest", "format"),
        ("data_versions", "created_at"),
        ("data_version_inputs", "raw_data_id"),
        ("quarantined_rows", "reason_code"),
        ("prediction_runs", "n_draws"),
        ("snapshots", "pointer_count"),
        ("snapshot_pointers", "model_version_id"),
        ("diagnostic_events", "payload"),
    ];
    for (table, column) in append_only {
        let needle = format!("grid: {table}:");
        assert_refused(
            &db,
            &format!("UPDATE {table} SET {column} = {column}"),
            &needle,
        )
        .await;
        assert_refused(&db, &format!("DELETE FROM {table}"), &needle).await;
    }
    for table in [
        "jobs",
        "raw_data",
        "training_runs",
        "model_versions",
        "production_pointers",
    ] {
        assert_refused(
            &db,
            &format!("DELETE FROM {table}"),
            &format!("grid: {table}:"),
        )
        .await;
    }
    db.close().await;
}

/// Claims job `id`: `PENDING → RUNNING` with the attempt counted and the lease set.
fn claim(id: i64, attempt: i64) -> String {
    format!(
        "UPDATE jobs SET status = 'RUNNING', attempt_count = {attempt}, started_at = 100, \
         heartbeat_at = 100, lease_expires_at = 400, owner_pid = 7 WHERE job_id = {id}"
    )
}

#[sqlx::test]
async fn jobs_triggers_enforce_the_ledger_rules() {
    let dir = TempDir::new("jobs-triggers");
    let db = migrated(&dir).await;

    // INSERT only as an unstarted PENDING row with 0 <= attempt_count <= max_attempts (D22).
    assert_refused(
        &db,
        &job_insert("PREDICT", "RUNNING", None, "{\"a\":1}", 0, 3),
        "grid: jobs:",
    )
    .await;
    assert_refused(
        &db,
        &job_insert("PREDICT", "SUCCEEDED", None, "{\"a\":2}", 0, 3),
        "grid: jobs:",
    )
    .await;
    assert_refused(
        &db,
        &job_insert("PREDICT", "PENDING", None, "{\"a\":3}", 4, 3),
        "grid: jobs:",
    )
    .await;
    assert_accepted(
        &db,
        &job_insert("PREDICT", "PENDING", None, "{\"carried\":1}", 3, 3),
    )
    .await;

    let id = common::insert_pending_job(&db, "PREDICT", "{\"life\":1}").await;
    // No direct PENDING → terminal transition.
    assert_refused(
        &db,
        &format!("UPDATE jobs SET status = 'SUCCEEDED', completed_at = 1 WHERE job_id = {id}"),
        "grid: jobs: illegal status transition",
    )
    .await;
    // The claim counts exactly one attempt.
    assert_refused(&db, &claim(id, 0), "grid: jobs: attempt_count").await;
    assert_refused(&db, &claim(id, 2), "grid: jobs: attempt_count").await;
    assert_accepted(&db, &claim(id, 1)).await;
    // A heartbeat changes no attempt; an attempt never changes outside a claim.
    assert_accepted(
        &db,
        &format!("UPDATE jobs SET heartbeat_at = 130, lease_expires_at = 430 WHERE job_id = {id}"),
    )
    .await;
    assert_refused(
        &db,
        &format!("UPDATE jobs SET attempt_count = 2 WHERE job_id = {id}"),
        "grid: jobs: attempt_count",
    )
    .await;
    // RUNNING → PENDING (a retry with persisted backoff), then a second claim.
    assert_accepted(
        &db,
        &format!(
            "UPDATE jobs SET status = 'PENDING', not_before = 200, error_code = 'Internal', \
             error_message = 'retry' WHERE job_id = {id}"
        ),
    )
    .await;
    assert_accepted(&db, &claim(id, 2)).await;
    // Identity and configuration never change.
    assert_refused(
        &db,
        &format!("UPDATE jobs SET config_json = '{{\"life\":2}}' WHERE job_id = {id}"),
        "grid: jobs:",
    )
    .await;
    assert_refused(
        &db,
        &format!("UPDATE jobs SET max_attempts = 5 WHERE job_id = {id}"),
        "grid: jobs:",
    )
    .await;
    // RUNNING → SUCCEEDED, after which the row is frozen.
    assert_accepted(
        &db,
        &format!(
            "UPDATE jobs SET status = 'SUCCEEDED', completed_at = 300, error_code = NULL, \
             error_message = NULL WHERE job_id = {id}"
        ),
    )
    .await;
    assert_refused(
        &db,
        &format!("UPDATE jobs SET heartbeat_at = 999 WHERE job_id = {id}"),
        "grid: jobs: a terminal job row is never changed",
    )
    .await;
    // A SUCCEEDED key is unique among live and done rows; a FAILED one re-enqueues.
    assert_refused(
        &db,
        &job_insert("PREDICT", "PENDING", None, "{\"life\":1}", 0, 3),
        "UNIQUE constraint failed",
    )
    .await;

    let failed = common::insert_pending_job(&db, "EVALUATE", "{\"fail\":1}").await;
    assert_accepted(&db, &claim(failed, 1)).await;
    assert_refused(
        &db,
        &format!("UPDATE jobs SET status = 'FAILED', completed_at = 300 WHERE job_id = {failed}"),
        "CHECK constraint failed",
    )
    .await;
    assert_accepted(
        &db,
        &format!(
            "UPDATE jobs SET status = 'FAILED', completed_at = 300, error_code = 'Internal', \
             error_message = 'exhausted' WHERE job_id = {failed}"
        ),
    )
    .await;
    assert_accepted(
        &db,
        &job_insert("EVALUATE", "PENDING", None, "{\"fail\":1}", 0, 3),
    )
    .await;

    // At most one RUNNING publication job.
    let promote = common::insert_pending_job(&db, "MODEL_PROMOTE", "{\"p\":1}").await;
    let rollback = common::insert_pending_job(&db, "ROLLBACK", "{\"r\":1}").await;
    assert_accepted(&db, &claim(promote, 1)).await;
    assert_refused(&db, &claim(rollback, 1), "UNIQUE constraint failed").await;
    db.close().await;
}

#[sqlx::test]
async fn job_dependency_edges_name_existing_upstream_keys() {
    let dir = TempDir::new("edges");
    let db = migrated(&dir).await;
    let upstream = common::insert_pending_job(&db, "DATA_FETCH", "{\"u\":1}").await;
    let downstream = common::insert_pending_job(&db, "NORMALIZE_DATA", "{\"d\":1}").await;
    let upstream_key = format!("DATA_FETCH:-:{}", sha_hex("{\"u\":1}"));
    let own_key = format!("NORMALIZE_DATA:-:{}", sha_hex("{\"d\":1}"));
    let edge = |job: i64, key: &str| {
        format!(
            "INSERT INTO job_dependencies (job_id, upstream_key, allow_skipped) \
             VALUES ({job}, '{key}', 0)"
        )
    };
    assert_refused(
        &db,
        &edge(downstream, &format!("DATA_FETCH:-:{}", sha_hex("absent"))),
        "grid: job_dependencies:",
    )
    .await;
    assert_refused(&db, &edge(downstream, &own_key), "grid: job_dependencies:").await;
    assert_refused(
        &db,
        &edge(downstream, "not-a-key"),
        "CHECK constraint failed",
    )
    .await;
    assert_accepted(&db, &edge(downstream, &upstream_key)).await;
    // Edges are added only while the job is PENDING.
    assert_accepted(&db, &claim(upstream, 1)).await;
    let another = format!("DATA_FETCH:-:{}", sha_hex("{\"u\":2}"));
    common::insert_pending_job(&db, "DATA_FETCH", "{\"u\":2}").await;
    assert_refused(&db, &edge(upstream, &another), "grid: job_dependencies:").await;
    db.close().await;
}

#[sqlx::test]
async fn idempotency_key_check_matches_the_documented_format() {
    let dir = TempDir::new("keys");
    let db = migrated(&dir).await;
    let hash = sha_hex("{\"k\":1}");
    let insert = |stage: &str, key: &str, config_hash: &str, input: &str| {
        format!(
            "INSERT INTO jobs (job_type, status, idempotency_key, config_hash, config_json, \
             input_data_version, restartable, attempt_count, max_attempts, not_before, created_at) \
             VALUES ('{stage}', 'PENDING', '{key}', '{config_hash}', '{{\"k\":1}}', {input}, 1, \
             0, 3, 0, 0)"
        )
    };
    // "<STAGE>:<input data version or ->:<config hash>" and nothing else (D6).
    assert_refused(
        &db,
        &insert("DATA_FETCH", &format!("PREDICT:-:{hash}"), &hash, "NULL"),
        "CHECK constraint failed",
    )
    .await;
    assert_refused(
        &db,
        &insert("DATA_FETCH", &format!("DATA_FETCH:1:{hash}"), &hash, "NULL"),
        "CHECK constraint failed",
    )
    .await;
    let upper = hash.to_uppercase();
    assert_refused(
        &db,
        &insert(
            "DATA_FETCH",
            &format!("DATA_FETCH:-:{upper}"),
            &upper,
            "NULL",
        ),
        "CHECK constraint failed",
    )
    .await;
    assert_refused(
        &db,
        &insert(
            "DATA_FETCH",
            &format!("DATA_FETCH:-:{}", &hash[..63]),
            &hash[..63],
            "NULL",
        ),
        "CHECK constraint failed",
    )
    .await;
    assert_accepted(
        &db,
        &insert("DATA_FETCH", &format!("DATA_FETCH:-:{hash}"), &hash, "NULL"),
    )
    .await;

    // With an input data version the key carries it in decimal.
    exec(
        &db,
        "INSERT INTO data_versions (parent_data_version, job_id, created_at) VALUES (NULL, 1, 0)",
    )
    .await;
    assert_accepted(
        &db,
        &insert("PREDICT", &format!("PREDICT:1:{hash}"), &hash, "1"),
    )
    .await;
    // The configuration is canonical JSON with an object root.
    assert_refused(
        &db,
        &job_insert("PREDICT", "PENDING", None, "[1,2]", 0, 3),
        "CHECK constraint failed",
    )
    .await;
    db.close().await;
}

#[sqlx::test]
async fn data_versions_form_one_linear_chain() {
    let dir = TempDir::new("chain");
    let db = migrated(&dir).await;
    let job = common::insert_pending_job(&db, "NORMALIZE_DATA", "{\"n\":1}").await;
    let version = |parent: &str| {
        format!(
            "INSERT INTO data_versions (parent_data_version, job_id, created_at) \
             VALUES ({parent}, {job}, 0)"
        )
    };
    assert_refused(&db, &version("1"), "grid: data_versions:").await;
    assert_accepted(&db, &version("NULL")).await;
    assert_refused(&db, &version("NULL"), "grid: data_versions:").await;
    assert_accepted(&db, &version("1")).await;
    assert_refused(&db, &version("1"), "grid: data_versions:").await;
    assert_accepted(&db, &version("2")).await;
    db.close().await;
}

#[sqlx::test]
async fn raw_data_validation_is_set_once_and_hash_matches_the_artifact() {
    let dir = TempDir::new("raw-data");
    let db = seeded(&dir).await;
    let empty = sha_hex("");
    let other = sha_hex("other");
    let raw = |sha: &str| {
        format!(
            "INSERT INTO raw_data (ingestion_run_id, source_name, source_version, \
             source_timestamp, retrieved_at, artifact_id, sha256) \
             VALUES (1, 'src', 'v1', 0, 1, 1, '{sha}')"
        )
    };
    assert_refused(&db, &raw(&other), "grid: raw_data: sha256").await;
    assert_accepted(&db, &raw(&empty)).await;
    let id: i64 = sqlx::query_scalar("SELECT MAX(raw_data_id) FROM raw_data")
        .fetch_one(db.pool())
        .await
        .expect("readable");
    // NULL until validation sets every field together, once.
    assert_refused(
        &db,
        &format!("UPDATE raw_data SET row_count = 3 WHERE raw_data_id = {id}"),
        "grid: raw_data:",
    )
    .await;
    assert_refused(
        &db,
        &format!(
            "UPDATE raw_data SET row_count = 3, validation_status = 'valid' WHERE raw_data_id = {id}"
        ),
        "CHECK constraint failed",
    )
    .await;
    let validate = format!(
        "UPDATE raw_data SET row_count = 3, schema_version = 's1', validation_status = 'valid' \
         WHERE raw_data_id = {id}"
    );
    assert_accepted(&db, &validate).await;
    assert_refused(&db, &validate, "grid: raw_data:").await;
    assert_refused(
        &db,
        &format!("UPDATE raw_data SET source_name = 'x' WHERE raw_data_id = 1"),
        "grid: raw_data:",
    )
    .await;
    db.close().await;
}

#[sqlx::test]
async fn artifact_manifest_paths_derive_from_the_hash_and_sizes_agree() {
    let dir = TempDir::new("manifest-rules");
    let db = seeded(&dir).await;
    let sha = sha_hex("abc");
    let row = |path: &str, size: i64| {
        format!(
            "INSERT INTO artifact_manifest (sha256, path, byte_size, format, schema_version, \
             producing_job_id, created_at) VALUES ('{sha}', '{path}', {size}, 'raw', '1', 1, 0)"
        )
    };
    let derived = format!("artifacts/sha256/{}/{sha}", &sha[..2]);
    assert_refused(&db, &row("../../etc/passwd", 3), "CHECK constraint failed").await;
    assert_refused(
        &db,
        &row(&format!("artifacts/sha256/zz/{sha}"), 3),
        "CHECK constraint failed",
    )
    .await;
    assert_accepted(&db, &row(&derived, 3)).await;
    assert_accepted(&db, &row(&derived, 3)).await;
    assert_refused(
        &db,
        &row(&derived, 4),
        "grid: artifact_manifest: one content hash",
    )
    .await;
    db.close().await;
}

/// Inserts a TRAINING model version of `family` and returns its id.
async fn insert_model_version(db: &Database, family: &str) -> i64 {
    sqlx::query(
        "INSERT INTO model_versions (model_family, model_type, state, state_changed_at, \
         training_run_id, data_version, feature_schema_version, hyperparameters_json, \
         config_hash, random_seed, engine_crate_version, engine_git_sha, contract_version, \
         created_at) VALUES (?, 'type', 'TRAINING', 0, NULL, 1, 1, '{}', ?, NULL, '0.1.0', \
         NULL, '1.1', 0)",
    )
    .bind(family)
    .bind(sha_hex(family))
    .execute(db.pool())
    .await
    .expect("a TRAINING version inserts")
    .last_insert_rowid()
}

/// Moves model version `id` through `states` in order.
async fn move_through(db: &Database, id: i64, states: &[&str]) {
    for state in states {
        assert_accepted(
            db,
            &format!("UPDATE model_versions SET state = '{state}' WHERE model_version_id = {id}"),
        )
        .await;
    }
}

#[sqlx::test]
async fn model_versions_triggers_enforce_the_lifecycle() {
    let dir = TempDir::new("model-triggers");
    let db = seeded(&dir).await;
    assert_refused(
        &db,
        "INSERT INTO model_versions (model_family, model_type, state, state_changed_at, \
         data_version, feature_schema_version, hyperparameters_json, config_hash, \
         engine_crate_version, contract_version, created_at) VALUES ('f', 't', 'CANDIDATE', 0, \
         1, 1, '{}', 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855', \
         '0.1.0', '1.1', 0)",
        "grid: model_versions: a version is inserted as TRAINING",
    )
    .await;

    let id = insert_model_version(&db, "lifecycle").await;
    let set = |state: &str| {
        format!("UPDATE model_versions SET state = '{state}' WHERE model_version_id = {id}")
    };
    for illegal in ["PRODUCTION", "VALIDATING", "SUPERSEDED"] {
        assert_refused(
            &db,
            &set(illegal),
            "grid: model_versions: illegal state transition",
        )
        .await;
    }
    move_through(&db, id, &["CANDIDATE"]).await;
    assert_refused(&db, &set("REJECTED"), "illegal state transition").await;
    assert_refused(
        &db,
        &format!("UPDATE model_versions SET model_family = 'other' WHERE model_version_id = {id}"),
        "grid: model_versions: only state and state_changed_at change",
    )
    .await;
    assert_accepted(
        &db,
        &format!("UPDATE model_versions SET state_changed_at = 5 WHERE model_version_id = {id}"),
    )
    .await;

    // A rejected version never returns to PRODUCTION.
    let rejected = insert_model_version(&db, "rejected").await;
    move_through(&db, rejected, &["REJECTED"]).await;
    assert_refused(
        &db,
        &format!(
            "UPDATE model_versions SET state = 'PRODUCTION' WHERE model_version_id = {rejected}"
        ),
        "illegal state transition",
    )
    .await;
    db.close().await;
}

#[sqlx::test]
async fn one_production_version_per_family() {
    let dir = TempDir::new("one-production");
    let db = seeded(&dir).await;
    let first = insert_model_version(&db, "family_a").await;
    let second = insert_model_version(&db, "family_a").await;
    let other = insert_model_version(&db, "family_b").await;
    for id in [first, second, other] {
        move_through(&db, id, &["CANDIDATE", "VALIDATING"]).await;
    }
    move_through(&db, first, &["PRODUCTION"]).await;
    assert_refused(
        &db,
        &format!(
            "UPDATE model_versions SET state = 'PRODUCTION' WHERE model_version_id = {second}"
        ),
        "UNIQUE constraint failed",
    )
    .await;
    move_through(&db, other, &["PRODUCTION"]).await;
    // Superseding the first frees the family for the second.
    move_through(&db, first, &["SUPERSEDED"]).await;
    move_through(&db, second, &["PRODUCTION"]).await;
    db.close().await;
}

#[sqlx::test]
async fn pointer_must_name_a_production_version_of_its_family() {
    let dir = TempDir::new("pointers");
    let db = seeded(&dir).await;
    let validating = insert_model_version(&db, "family_c").await;
    move_through(&db, validating, &["CANDIDATE", "VALIDATING"]).await;
    let pointer = |family: &str, id: i64| {
        format!(
            "INSERT INTO production_pointers (model_family, model_version_id, \
             pre_move_snapshot_id, moved_by_job_id, moved_at) VALUES ('{family}', {id}, 1, 4, 0)"
        )
    };
    assert_refused(
        &db,
        &pointer("family_c", validating),
        "grid: production_pointers:",
    )
    .await;
    move_through(&db, validating, &["PRODUCTION"]).await;
    // The seed's PRODUCTION version 1 belongs to seed_family, not family_c.
    assert_refused(&db, &pointer("family_c", 1), "grid: production_pointers:").await;
    assert_accepted(&db, &pointer("family_c", validating)).await;
    // Moving the seed's pointer to a non-PRODUCTION version is refused.
    let candidate = insert_model_version(&db, "seed_family").await;
    move_through(&db, candidate, &["CANDIDATE", "VALIDATING"]).await;
    assert_refused(
        &db,
        &format!(
            "UPDATE production_pointers SET model_version_id = {candidate} \
             WHERE model_family = 'seed_family'"
        ),
        "grid: production_pointers:",
    )
    .await;
    db.close().await;
}

#[sqlx::test]
async fn pragmas_are_wal_full_and_foreign_keys() {
    let dir = TempDir::new("pragmas");
    let db = migrated(&dir).await;
    for mode in [DbMode::ReadWrite, DbMode::ReadOnly] {
        let reopened = Database::open(&dir.db_path(), mode)
            .await
            .expect("the store reopens");
        let journal: String = sqlx::query_scalar("PRAGMA journal_mode")
            .fetch_one(reopened.pool())
            .await
            .expect("readable");
        let synchronous: i64 = sqlx::query_scalar("PRAGMA synchronous")
            .fetch_one(reopened.pool())
            .await
            .expect("readable");
        let foreign_keys: i64 = sqlx::query_scalar("PRAGMA foreign_keys")
            .fetch_one(reopened.pool())
            .await
            .expect("readable");
        let busy: i64 = sqlx::query_scalar("PRAGMA busy_timeout")
            .fetch_one(reopened.pool())
            .await
            .expect("readable");
        assert_eq!(journal, "wal", "{mode:?}");
        assert_eq!(synchronous, 2, "synchronous = FULL in {mode:?}");
        assert_eq!(foreign_keys, 1, "{mode:?}");
        assert_eq!(busy, 5000, "{mode:?}");
        reopened.close().await;
    }

    // A read-only store refuses to start a write transaction.
    let reader = Database::open(&dir.db_path(), DbMode::ReadOnly)
        .await
        .expect("the store reopens");
    let refused = reader
        .begin_write()
        .await
        .expect_err("a read-only store never writes");
    assert!(matches!(refused, PersistenceError::InvalidInput { .. }));
    reader.close().await;

    // A dropped write transaction rolls back.
    let mut tx = db.begin_write().await.expect("BEGIN IMMEDIATE");
    sqlx::query("INSERT INTO schema_meta (key, value) VALUES ('dropped', 'x')")
        .execute(tx.connection())
        .await
        .expect("insert");
    drop(tx);
    let tx = db.begin_write().await.expect("BEGIN IMMEDIATE");
    tx.commit().await.expect("an empty commit");
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM schema_meta WHERE key = 'dropped'")
        .fetch_one(db.pool())
        .await
        .expect("readable");
    assert_eq!(count, 0, "nothing of the dropped transaction remains");
    db.close().await;
}

#[test]
fn codec_round_trips_and_refuses_corrupt_values() {
    assert!(matches!(
        id_to_sql("job_id", u64::MAX),
        Err(PersistenceError::IdOutOfRange {
            what: "job_id",
            value: u64::MAX
        })
    ));
    let max = u64::try_from(i64::MAX).expect("i64::MAX is a u64");
    assert_eq!(id_to_sql("job_id", max).ok(), Some(i64::MAX));
    assert_eq!(id_from_sql("job_id", i64::MAX).ok(), Some(max));
    let negative = id_from_sql("job_id", -1).expect_err("a negative id is corrupt");
    assert!(matches!(negative, PersistenceError::StateCorrupt { .. }));
    assert_eq!(negative.engine_error().code, EngineErrorCode::StateCorrupt);
    assert_eq!(
        negative.terminal_state(),
        Some(TerminalState::DatabaseFailure)
    );
    assert_eq!(
        id_to_sql("job_id", u64::MAX)
            .expect_err("out of range")
            .terminal_state(),
        None,
        "an id out of range is a programming error (exit 70)"
    );

    assert_eq!(u32_from_sql("n_draws", 4_294_967_295).ok(), Some(u32::MAX));
    assert!(u32_from_sql("n_draws", 4_294_967_296).is_err());
    assert!(u32_from_sql("n_draws", -1).is_err());
    assert_eq!(bool_from_sql("flag", bool_to_sql(true)).ok(), Some(true));
    assert_eq!(bool_from_sql("flag", bool_to_sql(false)).ok(), Some(false));
    assert!(matches!(
        bool_from_sql("flag", 2),
        Err(PersistenceError::StateCorrupt { .. })
    ));
    let instant = UtcInstant::from_unix_seconds(-5);
    assert_eq!(instant_from_sql(instant_to_sql(instant)), instant);

    fn round_trip<T: StoredEnum + std::fmt::Debug>() {
        for value in T::ALL {
            assert_eq!(T::from_stored(value.stored_name()).ok(), Some(*value));
        }
        for unknown in ["", "UNKNOWN", "pending"] {
            assert!(
                matches!(
                    T::from_stored(unknown),
                    Err(PersistenceError::StateCorrupt { .. })
                ),
                "{} {unknown:?} is corrupt",
                T::WHAT
            );
        }
    }
    round_trip::<StageId>();
    round_trip::<JobStatus>();
    round_trip::<TerminalState>();
    round_trip::<ModelState>();
    round_trip::<EngineErrorCode>();
    round_trip::<LockKind>();
}
