-- P1-02 / ADR-015 (Proposed): the backbone of the durable schema, engine-spec §8.5 as edited by
-- owner ruling R1 (2026-10-21, "Backbone only").
--
-- This migration creates the thirteen tables R1 names (jobs, data_versions, raw_data,
-- ingestion_runs, artifact_manifest, production_pointers, snapshots, model_versions,
-- training_runs, prediction_runs, engine_settings, diagnostic_events, quarantined_rows) and the
-- three child tables of work package P1-02 decision D3 (job_dependencies, data_version_inputs,
-- snapshot_pointers). Every other §8.5 table lands in the migration of the package that owns its
-- contract or model specification. schema_meta (0001) is kept (ADR-015 D6).
--
-- Migrations are append-only (scripts/check-migrations.sh) and contain no destructive statement
-- without the owner's approval (ADR-015 D5); this file is LF-only (ADR-015 D7).
--
-- Conventions (work package P1-02 D4, D5):
--   * Every table is STRICT, so SQLite never coerces a value to a column's affinity.
--   * Ids are INTEGER (u64 <-> i64, checked in grid-persistence's codec). Surrogate ids are
--     INTEGER PRIMARY KEY AUTOINCREMENT, never reused.
--   * Instants are INTEGER whole seconds since the Unix epoch, UTC (grid_domain::UtcInstant).
--   * Booleans are INTEGER 0 or 1. Hashes are 64 lowercase hexadecimal characters.
--   * u64 values that may exceed i64 (seeds) are canonical decimal TEXT.
--   * Enum text uses the domain spellings: StageId, JobStatus, TerminalState and ModelState by
--     their spec_name() (SCREAMING_SNAKE), EngineErrorCode by name(), the engine events by
--     EngineEvent::NAMES, LockKind by its variant name. This is internal database spelling and
--     does not pre-empt engine-output-contract §12.
--   * Every rule is enforced in Rust first; the CHECKs and triggers are the store's backstop, and
--     every trigger aborts with a message that starts with "grid: <table>:".

-- Engine settings: append-only and versioned per key; the latest version of a key is effective
-- (engine-spec §8.5 "engine_settings", §8.10 Retries; P1-02 D4, D8, D21).
CREATE TABLE engine_settings (
    key        TEXT NOT NULL CHECK (length(key) BETWEEN 1 AND 128 AND key NOT GLOB '*[^a-z0-9_.]*'),
    version    INTEGER NOT NULL CHECK (version >= 1),
    value      TEXT NOT NULL CHECK (length(value) BETWEEN 1 AND 4096),
    created_at INTEGER NOT NULL,
    note       TEXT NOT NULL CHECK (length(note) BETWEEN 1 AND 512),
    PRIMARY KEY (key, version)
) STRICT, WITHOUT ROWID;

-- The P1-02 D8 defaults. The delay before attempt n >= 2 is min(5 s x 2^(n-2), 60 s), and
-- 2 x heartbeat < lease.
INSERT INTO engine_settings (key, version, value, created_at, note) VALUES
    ('jobs.max_attempts', 1, '3', CAST(strftime('%s', 'now') AS INTEGER),
     'P1-02 D8 default (engine-spec 8.10 Retries)'),
    ('jobs.backoff_initial_secs', 1, '5', CAST(strftime('%s', 'now') AS INTEGER),
     'P1-02 D8 default (engine-spec 8.10 Retries)'),
    ('jobs.backoff_max_secs', 1, '60', CAST(strftime('%s', 'now') AS INTEGER),
     'P1-02 D8 default (engine-spec 8.10 Retries)'),
    ('jobs.lease_secs', 1, '300', CAST(strftime('%s', 'now') AS INTEGER),
     'P1-02 D8 default (engine-spec 8.10 Stale RUNNING; owner ruling R5)'),
    ('jobs.heartbeat_secs', 1, '30', CAST(strftime('%s', 'now') AS INTEGER),
     'P1-02 D8 default; 2 x heartbeat < lease'),
    ('engine.shutdown_grace_secs', 1, '30', CAST(strftime('%s', 'now') AS INTEGER),
     'P1-02 D8 default (engine-spec 8.11 shutdown)');

CREATE TRIGGER engine_settings_no_update BEFORE UPDATE ON engine_settings
BEGIN
    SELECT RAISE(ABORT, 'grid: engine_settings: append-only; a change is a new version of the key (P1-02 D4)');
END;

CREATE TRIGGER engine_settings_no_delete BEFORE DELETE ON engine_settings
BEGIN
    SELECT RAISE(ABORT, 'grid: engine_settings: append-only; a change is a new version of the key (P1-02 D4)');
END;

-- The durable run ledger (engine-spec §8.10 Fields and Semantics; P1-02 D4, D6-D9, D22).
CREATE TABLE jobs (
    job_id               INTEGER PRIMARY KEY AUTOINCREMENT,
    job_type             TEXT NOT NULL CHECK (job_type IN (
                             'DATA_FETCH', 'NORMALIZE_DATA', 'BUILD_FEATURES', 'RAPM_UPDATE',
                             'KALMAN_UPDATE', 'RTS_SMOOTH', 'EMPIRICAL_BAYES_UPDATE',
                             'BOOSTING_UPDATE', 'MODEL_VALIDATE', 'MODEL_PROMOTE',
                             'IDENTITY_RESOLVE', 'VALUE_MODEL_FIT', 'COMPUTE_DV', 'LAYER1_CREDIT',
                             'FIXED_LAG_SMOOTH', 'PRIOR_TRANSLATE', 'PREDICT', 'EVALUATE',
                             'SNAPSHOT', 'ROLLBACK')),
    status               TEXT NOT NULL CHECK (status IN (
                             'PENDING', 'RUNNING', 'SUCCEEDED', 'FAILED', 'CANCELLED',
                             'SKIPPED_NO_NEW_DATA')),
    idempotency_key      TEXT NOT NULL,
    config_hash          TEXT NOT NULL CHECK (length(config_hash) = 64
                             AND config_hash NOT GLOB '*[^0-9a-f]*'),
    -- The job's durable stage inputs, canonical JSON with an object root (P1-02 D4, D6).
    config_json          TEXT NOT NULL CHECK (length(config_json) BETWEEN 2 AND 65536
                             AND json_valid(config_json) AND json_type(config_json) = 'object'),
    input_data_version   INTEGER REFERENCES data_versions (data_version),
    ingestion_run_id     INTEGER REFERENCES ingestion_runs (ingestion_run_id),
    output_model_version INTEGER REFERENCES model_versions (model_version_id),
    -- Declared at enqueue: whether the stage can restart from durable inputs (§8.10; D4, D9).
    restartable          INTEGER NOT NULL CHECK (restartable IN (0, 1)),
    attempt_count        INTEGER NOT NULL CHECK (attempt_count >= 0),
    -- jobs.max_attempts at enqueue (D4).
    max_attempts         INTEGER NOT NULL CHECK (max_attempts BETWEEN 1 AND 100),
    -- Persisted backoff: the job is not claimed before this instant (D4, D8).
    not_before           INTEGER NOT NULL,
    created_at           INTEGER NOT NULL,
    started_at           INTEGER,
    completed_at         INTEGER,
    heartbeat_at         INTEGER,
    lease_expires_at     INTEGER,
    -- Diagnostic only; never probed (owner ruling R5).
    owner_pid            INTEGER CHECK (owner_pid BETWEEN 0 AND 4294967295),
    error_code           TEXT CHECK (error_code IN (
                             'ContractViolation', 'Leakage', 'StateCorrupt', 'IllConditioned',
                             'NotConverged', 'InvalidProbability', 'ShareOverflow',
                             'ImpossibleStat', 'NonFinite', 'MissingData',
                             'ParticipationUnavailable', 'SourceUnavailable', 'UnsupportedVersion',
                             'NonAffineScoringRule', 'UncoveredDefender', 'InsufficientPool',
                             'Internal')),
    error_message        TEXT CHECK (length(error_message) BETWEEN 1 AND 2000),
    -- idempotency_key = "<STAGE>:<input data version or ->:<config hash>" (§8.10; D6).
    CHECK (idempotency_key = job_type || ':' || COALESCE(CAST(input_data_version AS TEXT), '-')
           || ':' || config_hash),
    CHECK (attempt_count <= max_attempts),
    CHECK ((error_code IS NULL) = (error_message IS NULL)),
    CHECK (status <> 'RUNNING' OR (started_at IS NOT NULL AND heartbeat_at IS NOT NULL
           AND lease_expires_at IS NOT NULL AND owner_pid IS NOT NULL AND attempt_count >= 1)),
    CHECK ((status IN ('SUCCEEDED', 'FAILED', 'CANCELLED', 'SKIPPED_NO_NEW_DATA'))
           = (completed_at IS NOT NULL)),
    CHECK (status <> 'FAILED' OR error_code IS NOT NULL),
    CHECK (status <> 'SUCCEEDED' OR error_code IS NULL),
    CHECK (output_model_version IS NULL OR status = 'SUCCEEDED')
) STRICT;

-- A key with a PENDING, RUNNING, SUCCEEDED or SKIPPED_NO_NEW_DATA row is reused, never
-- re-executed; a FAILED or CANCELLED key re-enqueues as a new row (§8.10 Idempotency; D6).
CREATE UNIQUE INDEX jobs_live_or_done_key ON jobs (idempotency_key)
    WHERE status IN ('PENDING', 'RUNNING', 'SUCCEEDED', 'SKIPPED_NO_NEW_DATA');
-- At most one RUNNING publication job (§8.10 Serialization, §8.13; owner ruling R5).
CREATE UNIQUE INDEX jobs_one_running_publication ON jobs (status)
    WHERE status = 'RUNNING' AND job_type IN ('MODEL_PROMOTE', 'ROLLBACK');
-- The latest row of a key is its highest job_id (D7).
CREATE INDEX jobs_by_key ON jobs (idempotency_key, job_id);
CREATE INDEX jobs_by_status ON jobs (status, not_before, job_id);

CREATE TRIGGER jobs_insert_as_pending BEFORE INSERT ON jobs
WHEN NEW.status IS NOT 'PENDING'
  OR NEW.attempt_count NOT BETWEEN 0 AND NEW.max_attempts
  OR NEW.started_at IS NOT NULL
  OR NEW.completed_at IS NOT NULL
  OR NEW.heartbeat_at IS NOT NULL
  OR NEW.lease_expires_at IS NOT NULL
  OR NEW.owner_pid IS NOT NULL
  OR NEW.output_model_version IS NOT NULL
BEGIN
    SELECT RAISE(ABORT, 'grid: jobs: a job is inserted only as an unstarted PENDING row with 0 <= attempt_count <= max_attempts (engine-spec 8.10; P1-02 D22)');
END;

-- Exactly the six pairs of grid_governance::jobs::JOB_TRANSITIONS.
CREATE TRIGGER jobs_status_transition BEFORE UPDATE OF status ON jobs
WHEN NEW.status IS NOT OLD.status
 AND NOT ((OLD.status = 'PENDING' AND NEW.status = 'RUNNING')
       OR (OLD.status = 'RUNNING' AND NEW.status IN
               ('SUCCEEDED', 'SKIPPED_NO_NEW_DATA', 'FAILED', 'PENDING', 'CANCELLED')))
BEGIN
    SELECT RAISE(ABORT, 'grid: jobs: illegal status transition (engine-spec 8.10; P1-02 D6-D9)');
END;

CREATE TRIGGER jobs_terminal_rows_frozen BEFORE UPDATE ON jobs
WHEN OLD.status IN ('SUCCEEDED', 'FAILED', 'CANCELLED', 'SKIPPED_NO_NEW_DATA')
BEGIN
    SELECT RAISE(ABORT, 'grid: jobs: a terminal job row is never changed (engine-spec 8.10; P1-02 D6)');
END;

-- An attempt is counted at claim and only there: PENDING -> RUNNING adds exactly one.
CREATE TRIGGER jobs_attempt_counted_at_claim BEFORE UPDATE ON jobs
WHEN (OLD.status = 'PENDING' AND NEW.status = 'RUNNING'
      AND NEW.attempt_count IS NOT OLD.attempt_count + 1)
  OR (NOT (OLD.status = 'PENDING' AND NEW.status = 'RUNNING')
      AND NEW.attempt_count IS NOT OLD.attempt_count)
BEGIN
    SELECT RAISE(ABORT, 'grid: jobs: attempt_count rises by exactly one, only on PENDING to RUNNING (engine-spec 8.10 Retries; P1-02 D8)');
END;

CREATE TRIGGER jobs_immutable_columns BEFORE UPDATE ON jobs
WHEN NEW.job_id IS NOT OLD.job_id
  OR NEW.job_type IS NOT OLD.job_type
  OR NEW.idempotency_key IS NOT OLD.idempotency_key
  OR NEW.config_hash IS NOT OLD.config_hash
  OR NEW.config_json IS NOT OLD.config_json
  OR NEW.input_data_version IS NOT OLD.input_data_version
  OR NEW.restartable IS NOT OLD.restartable
  OR NEW.max_attempts IS NOT OLD.max_attempts
  OR NEW.created_at IS NOT OLD.created_at
  OR (OLD.ingestion_run_id IS NOT NULL AND NEW.ingestion_run_id IS NOT OLD.ingestion_run_id)
  OR (OLD.output_model_version IS NOT NULL
      AND NEW.output_model_version IS NOT OLD.output_model_version)
BEGIN
    SELECT RAISE(ABORT, 'grid: jobs: the identity, configuration and enqueue columns of a job never change (engine-spec 8.10; P1-02 D4, D6)');
END;

CREATE TRIGGER jobs_no_delete BEFORE DELETE ON jobs
BEGIN
    SELECT RAISE(ABORT, 'grid: jobs: job rows are never deleted (engine-spec 8.10 diagnostic history)');
END;

-- Dependency edges, by upstream idempotency key (§8.10 "dependency edges"; P1-02 D3, D7).
CREATE TABLE job_dependencies (
    job_id        INTEGER NOT NULL REFERENCES jobs (job_id),
    upstream_key  TEXT NOT NULL CHECK (length(upstream_key) BETWEEN 74 AND 108
                      AND substr(upstream_key, -65, 1) = ':'
                      AND substr(upstream_key, -64) NOT GLOB '*[^0-9a-f]*'),
    allow_skipped INTEGER NOT NULL CHECK (allow_skipped IN (0, 1)),
    PRIMARY KEY (job_id, upstream_key)
) STRICT, WITHOUT ROWID;

CREATE INDEX job_dependencies_by_upstream ON job_dependencies (upstream_key);

CREATE TRIGGER job_dependencies_insert_rules BEFORE INSERT ON job_dependencies
WHEN NOT EXISTS (SELECT 1 FROM jobs WHERE idempotency_key = NEW.upstream_key)
  OR (SELECT status FROM jobs WHERE job_id = NEW.job_id) IS NOT 'PENDING'
  OR NEW.upstream_key IS (SELECT idempotency_key FROM jobs WHERE job_id = NEW.job_id)
BEGIN
    SELECT RAISE(ABORT, 'grid: job_dependencies: an edge names an existing upstream key other than its own and is added only while the job is PENDING (engine-spec 8.10; P1-02 D7)');
END;

CREATE TRIGGER job_dependencies_no_update BEFORE UPDATE ON job_dependencies
BEGIN
    SELECT RAISE(ABORT, 'grid: job_dependencies: append-only (P1-02 D7)');
END;

CREATE TRIGGER job_dependencies_no_delete BEFORE DELETE ON job_dependencies
BEGIN
    SELECT RAISE(ABORT, 'grid: job_dependencies: append-only (P1-02 D7)');
END;

-- One row per source fetch, written in the stage's final success or failure commit, so it is
-- never RUNNING (engine-spec §4.1.1, §8.6.1; P1-02 D4, D10).
CREATE TABLE ingestion_runs (
    ingestion_run_id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id           INTEGER NOT NULL REFERENCES jobs (job_id),
    source_name      TEXT NOT NULL CHECK (length(source_name) BETWEEN 1 AND 128),
    offline          INTEGER NOT NULL CHECK (offline IN (0, 1)),
    -- Whether the run performed a network fetch; the fetch cap counts successful ones (§8.6.1).
    network_fetch    INTEGER NOT NULL CHECK (network_fetch IN (0, 1)),
    started_at       INTEGER NOT NULL,
    finished_at      INTEGER NOT NULL,
    terminal_state   TEXT NOT NULL CHECK (terminal_state IN (
                         'SUCCESS', 'NO_NEW_DATA', 'PARTIAL_SUCCESS', 'NETWORK_FAILURE',
                         'TIMEOUT', 'RATE_LIMITED', 'AUTHENTICATION_FAILURE', 'INVALID_RESPONSE',
                         'SCHEMA_FAILURE', 'DATA_VALIDATION_FAILURE', 'DATABASE_FAILURE',
                         'LEARNING_FAILURE')),
    error_code       TEXT CHECK (error_code IN (
                         'ContractViolation', 'Leakage', 'StateCorrupt', 'IllConditioned',
                         'NotConverged', 'InvalidProbability', 'ShareOverflow', 'ImpossibleStat',
                         'NonFinite', 'MissingData', 'ParticipationUnavailable',
                         'SourceUnavailable', 'UnsupportedVersion', 'NonAffineScoringRule',
                         'UncoveredDefender', 'InsufficientPool', 'Internal')),
    error_message    TEXT CHECK (length(error_message) BETWEEN 1 AND 2000),
    CHECK (finished_at >= started_at),
    CHECK ((error_code IS NULL) = (error_message IS NULL)),
    CHECK ((terminal_state IN ('SUCCESS', 'NO_NEW_DATA', 'PARTIAL_SUCCESS'))
           = (error_code IS NULL)),
    CHECK (offline = 0 OR network_fetch = 0)
) STRICT;

CREATE INDEX ingestion_runs_by_source ON ingestion_runs (source_name, terminal_state, finished_at);
CREATE INDEX ingestion_runs_by_job ON ingestion_runs (job_id);

CREATE TRIGGER ingestion_runs_no_update BEFORE UPDATE ON ingestion_runs
BEGIN
    SELECT RAISE(ABORT, 'grid: ingestion_runs: append-only, written once at the final commit (P1-02 D10)');
END;

CREATE TRIGGER ingestion_runs_no_delete BEFORE DELETE ON ingestion_runs
BEGIN
    SELECT RAISE(ABORT, 'grid: ingestion_runs: append-only, written once at the final commit (P1-02 D10)');
END;

-- Every registered bulk artifact, immutable once written (engine-spec §8.5 GRID state group,
-- §8.11 artifact write protocol; DR-C15; P1-02 D4, D16). Identical bytes registered twice are two
-- rows (two lineages) and one file.
CREATE TABLE artifact_manifest (
    artifact_id      INTEGER PRIMARY KEY AUTOINCREMENT,
    sha256           TEXT NOT NULL CHECK (length(sha256) = 64 AND sha256 NOT GLOB '*[^0-9a-f]*'),
    -- Relative to the data directory and derived from the hash, never chosen freely.
    path             TEXT NOT NULL CHECK (path = 'artifacts/sha256/' || substr(sha256, 1, 2)
                         || '/' || sha256),
    byte_size        INTEGER NOT NULL CHECK (byte_size >= 0),
    format           TEXT NOT NULL CHECK (length(format) BETWEEN 1 AND 64
                         AND format NOT GLOB '*[^a-z0-9_.-]*'),
    schema_version   TEXT NOT NULL CHECK (length(schema_version) BETWEEN 1 AND 64
                         AND schema_version NOT GLOB '*[^A-Za-z0-9_.+-]*'),
    producing_job_id INTEGER NOT NULL REFERENCES jobs (job_id),
    created_at       INTEGER NOT NULL
) STRICT;

CREATE INDEX artifact_manifest_by_sha256 ON artifact_manifest (sha256);
CREATE INDEX artifact_manifest_by_job ON artifact_manifest (producing_job_id);

CREATE TRIGGER artifact_manifest_one_size_per_hash BEFORE INSERT ON artifact_manifest
WHEN EXISTS (SELECT 1 FROM artifact_manifest
             WHERE sha256 = NEW.sha256 AND byte_size IS NOT NEW.byte_size)
BEGIN
    SELECT RAISE(ABORT, 'grid: artifact_manifest: one content hash has one byte size (engine-spec 8.11)');
END;

CREATE TRIGGER artifact_manifest_no_update BEFORE UPDATE ON artifact_manifest
BEGIN
    SELECT RAISE(ABORT, 'grid: artifact_manifest: immutable once written (engine-spec 8.5)');
END;

CREATE TRIGGER artifact_manifest_no_delete BEFORE DELETE ON artifact_manifest
BEGIN
    SELECT RAISE(ABORT, 'grid: artifact_manifest: immutable once written (engine-spec 8.5)');
END;

-- Every retained raw response with its §4.1.1 metadata (engine-spec §4.1.1, §8.5 raw data
-- retention; P1-02 D11). No uniqueness on (source, hash): the revert case is P1-03's (F-3). The
-- validation fields are NULL until validation sets them, once; NULL means "not validated", never
-- "ok" (KI-NEW-W5).
CREATE TABLE raw_data (
    raw_data_id       INTEGER PRIMARY KEY AUTOINCREMENT,
    ingestion_run_id  INTEGER NOT NULL REFERENCES ingestion_runs (ingestion_run_id),
    source_name       TEXT NOT NULL CHECK (length(source_name) BETWEEN 1 AND 128),
    source_version    TEXT NOT NULL CHECK (length(source_version) BETWEEN 1 AND 128),
    source_timestamp  INTEGER NOT NULL,
    retrieved_at      INTEGER NOT NULL,
    artifact_id       INTEGER NOT NULL REFERENCES artifact_manifest (artifact_id),
    -- The content hash over the retained raw bytes; equal to the artifact's hash.
    sha256            TEXT NOT NULL CHECK (length(sha256) = 64 AND sha256 NOT GLOB '*[^0-9a-f]*'),
    row_count         INTEGER CHECK (row_count >= 0),
    schema_version    TEXT CHECK (length(schema_version) BETWEEN 1 AND 64),
    validation_status TEXT CHECK (length(validation_status) BETWEEN 1 AND 64),
    CHECK ((row_count IS NULL) = (validation_status IS NULL)),
    CHECK ((schema_version IS NULL) = (validation_status IS NULL))
) STRICT;

CREATE INDEX raw_data_by_source ON raw_data (source_name, raw_data_id);
CREATE INDEX raw_data_by_run ON raw_data (ingestion_run_id);

CREATE TRIGGER raw_data_hash_matches_artifact BEFORE INSERT ON raw_data
WHEN NEW.sha256 IS NOT (SELECT sha256 FROM artifact_manifest WHERE artifact_id = NEW.artifact_id)
BEGIN
    SELECT RAISE(ABORT, 'grid: raw_data: sha256 equals the hash of the registered artifact (engine-spec 4.1.1)');
END;

CREATE TRIGGER raw_data_validation_set_once BEFORE UPDATE ON raw_data
WHEN OLD.validation_status IS NOT NULL
  OR NEW.validation_status IS NULL
  OR NEW.raw_data_id IS NOT OLD.raw_data_id
  OR NEW.ingestion_run_id IS NOT OLD.ingestion_run_id
  OR NEW.source_name IS NOT OLD.source_name
  OR NEW.source_version IS NOT OLD.source_version
  OR NEW.source_timestamp IS NOT OLD.source_timestamp
  OR NEW.retrieved_at IS NOT OLD.retrieved_at
  OR NEW.artifact_id IS NOT OLD.artifact_id
  OR NEW.sha256 IS NOT OLD.sha256
BEGIN
    SELECT RAISE(ABORT, 'grid: raw_data: only the validation fields change, once, from NULL (engine-spec 4.1.1; KI-NEW-W5)');
END;

CREATE TRIGGER raw_data_no_delete BEFORE DELETE ON raw_data
BEGIN
    SELECT RAISE(ABORT, 'grid: raw_data: retained raw data is never deleted here (engine-spec 8.5; retention is P1-11)');
END;

-- Committed data versions: one global linear chain per store (engine-spec §8.6.2 "commit SQLite
-- data version"; P1-02 D4, D12). A correction is a new version (KI-A9).
CREATE TABLE data_versions (
    data_version        INTEGER PRIMARY KEY AUTOINCREMENT,
    parent_data_version INTEGER REFERENCES data_versions (data_version),
    job_id              INTEGER NOT NULL REFERENCES jobs (job_id),
    created_at          INTEGER NOT NULL
) STRICT;

CREATE UNIQUE INDEX data_versions_one_child_per_parent ON data_versions (parent_data_version);

CREATE TRIGGER data_versions_extend_the_head BEFORE INSERT ON data_versions
WHEN NEW.parent_data_version IS NOT (SELECT MAX(data_version) FROM data_versions)
BEGIN
    SELECT RAISE(ABORT, 'grid: data_versions: a new version names the current head as its parent (P1-02 D12)');
END;

CREATE TRIGGER data_versions_no_update BEFORE UPDATE ON data_versions
BEGIN
    SELECT RAISE(ABORT, 'grid: data_versions: immutable versioned records (P1-02 D12; KI-A9)');
END;

CREATE TRIGGER data_versions_no_delete BEFORE DELETE ON data_versions
BEGIN
    SELECT RAISE(ABORT, 'grid: data_versions: immutable versioned records (P1-02 D12; KI-A9)');
END;

-- The raw inputs of a data version, for the retention rule (engine-spec §14.3 item 2; P1-02 D3).
CREATE TABLE data_version_inputs (
    data_version INTEGER NOT NULL REFERENCES data_versions (data_version),
    raw_data_id  INTEGER NOT NULL REFERENCES raw_data (raw_data_id),
    PRIMARY KEY (data_version, raw_data_id)
) STRICT, WITHOUT ROWID;

CREATE INDEX data_version_inputs_by_raw ON data_version_inputs (raw_data_id);

CREATE TRIGGER data_version_inputs_no_update BEFORE UPDATE ON data_version_inputs
BEGIN
    SELECT RAISE(ABORT, 'grid: data_version_inputs: append-only (P1-02 D3)');
END;

CREATE TRIGGER data_version_inputs_no_delete BEFORE DELETE ON data_version_inputs
BEGIN
    SELECT RAISE(ABORT, 'grid: data_version_inputs: append-only (P1-02 D3)');
END;

-- Rejected rows for §8.6.3 partial-data acceptance (engine-spec §8.5; QuarantinedRowRef).
CREATE TABLE quarantined_rows (
    quarantined_row_id INTEGER PRIMARY KEY AUTOINCREMENT,
    ingestion_run_id   INTEGER NOT NULL REFERENCES ingestion_runs (ingestion_run_id),
    source_name        TEXT NOT NULL CHECK (length(source_name) BETWEEN 1 AND 128),
    reason_code        TEXT NOT NULL CHECK (length(reason_code) BETWEEN 1 AND 128),
    -- A reference into the retained raw partition, never the row's content.
    raw_row_ref        TEXT NOT NULL CHECK (length(raw_row_ref) BETWEEN 1 AND 512),
    created_at         INTEGER NOT NULL
) STRICT;

CREATE INDEX quarantined_rows_by_run ON quarantined_rows (ingestion_run_id);

CREATE TRIGGER quarantined_rows_no_update BEFORE UPDATE ON quarantined_rows
BEGIN
    SELECT RAISE(ABORT, 'grid: quarantined_rows: append-only (engine-spec 8.5)');
END;

CREATE TRIGGER quarantined_rows_no_delete BEFORE DELETE ON quarantined_rows
BEGIN
    SELECT RAISE(ABORT, 'grid: quarantined_rows: append-only (engine-spec 8.5)');
END;

-- Training runs: §8.9 training_run_id, training_start (started_at) and training_end
-- (completed_at).
CREATE TABLE training_runs (
    training_run_id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id          INTEGER NOT NULL REFERENCES jobs (job_id),
    model_family    TEXT NOT NULL CHECK (length(model_family) BETWEEN 1 AND 128),
    data_version    INTEGER NOT NULL REFERENCES data_versions (data_version),
    config_hash     TEXT NOT NULL CHECK (length(config_hash) = 64
                        AND config_hash NOT GLOB '*[^0-9a-f]*'),
    started_at      INTEGER NOT NULL,
    completed_at    INTEGER,
    CHECK (completed_at IS NULL OR completed_at >= started_at)
) STRICT;

CREATE INDEX training_runs_by_job ON training_runs (job_id);

CREATE TRIGGER training_runs_complete_once BEFORE UPDATE ON training_runs
WHEN OLD.completed_at IS NOT NULL
  OR NEW.training_run_id IS NOT OLD.training_run_id
  OR NEW.job_id IS NOT OLD.job_id
  OR NEW.model_family IS NOT OLD.model_family
  OR NEW.data_version IS NOT OLD.data_version
  OR NEW.config_hash IS NOT OLD.config_hash
  OR NEW.started_at IS NOT OLD.started_at
BEGIN
    SELECT RAISE(ABORT, 'grid: training_runs: only completed_at changes, once, from NULL (engine-spec 8.9)');
END;

CREATE TRIGGER training_runs_no_delete BEFORE DELETE ON training_runs
BEGIN
    SELECT RAISE(ABORT, 'grid: training_runs: never deleted (engine-spec 8.9)');
END;

-- Model versions: versioned scientific artifacts with the §8.8 state on the row (engine-spec
-- §8.8, §8.9; P1-02 D4, D13). Validation metrics live in the §8.5 evaluation tables (P1-09).
CREATE TABLE model_versions (
    model_version_id       INTEGER PRIMARY KEY AUTOINCREMENT,
    -- The production-pointer key (§8.5 production_pointers "per model family").
    model_family           TEXT NOT NULL CHECK (length(model_family) BETWEEN 1 AND 128),
    model_type             TEXT NOT NULL CHECK (length(model_type) BETWEEN 1 AND 128),
    state                  TEXT NOT NULL CHECK (state IN (
                               'TRAINING', 'VALIDATING', 'CANDIDATE', 'PRODUCTION', 'REJECTED',
                               'SUPERSEDED')),
    state_changed_at       INTEGER NOT NULL,
    training_run_id        INTEGER REFERENCES training_runs (training_run_id),
    -- §8.9 data_snapshot_version.
    data_version           INTEGER NOT NULL REFERENCES data_versions (data_version),
    feature_schema_version INTEGER NOT NULL CHECK (feature_schema_version BETWEEN 0 AND 4294967295),
    hyperparameters_json   TEXT NOT NULL CHECK (length(hyperparameters_json) BETWEEN 2 AND 65536
                               AND json_valid(hyperparameters_json)
                               AND json_type(hyperparameters_json) = 'object'),
    config_hash            TEXT NOT NULL CHECK (length(config_hash) = 64
                               AND config_hash NOT GLOB '*[^0-9a-f]*'),
    -- §8.9 "random_seed, when applicable": a u64 in canonical decimal.
    random_seed            TEXT CHECK (length(random_seed) BETWEEN 1 AND 20
                               AND random_seed NOT GLOB '*[^0-9]*'
                               AND (random_seed = '0' OR random_seed NOT GLOB '0*')
                               AND (length(random_seed) < 20
                                    OR random_seed <= '18446744073709551615')),
    -- §8.9 engine addition 1: crate version plus git SHA (NULL outside a git checkout).
    engine_crate_version   TEXT NOT NULL CHECK (length(engine_crate_version) BETWEEN 1 AND 64),
    engine_git_sha         TEXT CHECK (length(engine_git_sha) BETWEEN 7 AND 64
                               AND engine_git_sha NOT GLOB '*[^0-9a-f]*'),
    -- MAJOR.MINOR of the contract the record was built from (contracts README, derived artifacts).
    contract_version       TEXT NOT NULL CHECK (length(contract_version) BETWEEN 3 AND 11
                               AND contract_version GLOB '[0-9]*.[0-9]*'
                               AND contract_version NOT GLOB '*[^0-9.]*'
                               AND contract_version NOT GLOB '*.*.*'),
    created_at             INTEGER NOT NULL,
    UNIQUE (model_family, model_version_id)
) STRICT;

-- A version is PRODUCTION exactly when a pointer names it, so a family has at most one (§8.8).
CREATE UNIQUE INDEX model_versions_one_production_per_family ON model_versions (model_family)
    WHERE state = 'PRODUCTION';

CREATE TRIGGER model_versions_insert_as_training BEFORE INSERT ON model_versions
WHEN NEW.state IS NOT 'TRAINING'
BEGIN
    SELECT RAISE(ABORT, 'grid: model_versions: a version is inserted as TRAINING (engine-spec 8.8)');
END;

-- Exactly the seven pairs of grid_governance::model_state::MODEL_TRANSITIONS; the cause is
-- checked by the pipeline before persistence is called (P1-02 D19).
CREATE TRIGGER model_versions_state_transition BEFORE UPDATE OF state ON model_versions
WHEN NEW.state IS NOT OLD.state
 AND NOT ((OLD.state = 'TRAINING' AND NEW.state = 'CANDIDATE')
       OR (OLD.state = 'CANDIDATE' AND NEW.state = 'VALIDATING')
       OR (OLD.state = 'VALIDATING' AND NEW.state = 'PRODUCTION')
       OR (OLD.state = 'VALIDATING' AND NEW.state = 'REJECTED')
       OR (OLD.state = 'TRAINING' AND NEW.state = 'REJECTED')
       OR (OLD.state = 'PRODUCTION' AND NEW.state = 'SUPERSEDED')
       OR (OLD.state = 'SUPERSEDED' AND NEW.state = 'PRODUCTION'))
BEGIN
    SELECT RAISE(ABORT, 'grid: model_versions: illegal state transition (engine-spec 8.8)');
END;

CREATE TRIGGER model_versions_immutable_columns BEFORE UPDATE ON model_versions
WHEN NEW.model_version_id IS NOT OLD.model_version_id
  OR NEW.model_family IS NOT OLD.model_family
  OR NEW.model_type IS NOT OLD.model_type
  OR NEW.training_run_id IS NOT OLD.training_run_id
  OR NEW.data_version IS NOT OLD.data_version
  OR NEW.feature_schema_version IS NOT OLD.feature_schema_version
  OR NEW.hyperparameters_json IS NOT OLD.hyperparameters_json
  OR NEW.config_hash IS NOT OLD.config_hash
  OR NEW.random_seed IS NOT OLD.random_seed
  OR NEW.engine_crate_version IS NOT OLD.engine_crate_version
  OR NEW.engine_git_sha IS NOT OLD.engine_git_sha
  OR NEW.contract_version IS NOT OLD.contract_version
  OR NEW.created_at IS NOT OLD.created_at
BEGIN
    SELECT RAISE(ABORT, 'grid: model_versions: only state and state_changed_at change (engine-spec 8.9: a version is never rewritten)');
END;

CREATE TRIGGER model_versions_no_delete BEFORE DELETE ON model_versions
BEGIN
    SELECT RAISE(ABORT, 'grid: model_versions: a version is never deleted (engine-spec 8.7.4, 8.8)');
END;

-- Prediction runs: the engine-output-contract §2 keys and lineage group of one emitted
-- projection set, plus the producing job and the contract version (P1-02 D4).
CREATE TABLE prediction_runs (
    prediction_version_id   INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id                  INTEGER NOT NULL REFERENCES jobs (job_id),
    season                  INTEGER NOT NULL CHECK (season BETWEEN 0 AND 65535),
    -- Never 0: no week-0 sentinel (KI-A4).
    week                    INTEGER NOT NULL CHECK (week BETWEEN 1 AND 255),
    lock_kind               TEXT NOT NULL CHECK (lock_kind IN ('Thursday', 'Sunday', 'Operational')),
    lock_at                 INTEGER NOT NULL,
    -- §2 data_snapshot_id.
    data_version            INTEGER NOT NULL REFERENCES data_versions (data_version),
    feature_schema_version  INTEGER NOT NULL CHECK (feature_schema_version BETWEEN 0 AND 4294967295),
    model_version_id        INTEGER NOT NULL REFERENCES model_versions (model_version_id),
    engine_crate_version    TEXT NOT NULL CHECK (length(engine_crate_version) BETWEEN 1 AND 64),
    engine_git_sha          TEXT CHECK (length(engine_git_sha) BETWEEN 7 AND 64
                                AND engine_git_sha NOT GLOB '*[^0-9a-f]*'),
    scoring_profile_id      TEXT NOT NULL CHECK (length(scoring_profile_id) BETWEEN 1 AND 128),
    scoring_profile_version INTEGER NOT NULL CHECK (scoring_profile_version BETWEEN 0 AND 4294967295),
    -- The Layer-F seed, a u64 in canonical decimal.
    seed                    TEXT NOT NULL CHECK (length(seed) BETWEEN 1 AND 20
                                AND seed NOT GLOB '*[^0-9]*'
                                AND (seed = '0' OR seed NOT GLOB '0*')
                                AND (length(seed) < 20 OR seed <= '18446744073709551615')),
    n_draws                 INTEGER NOT NULL CHECK (n_draws BETWEEN 1 AND 4294967295),
    research_only           INTEGER NOT NULL CHECK (research_only IN (0, 1)),
    relaxed_rule            TEXT CHECK (length(relaxed_rule) BETWEEN 1 AND 256),
    contract_version        TEXT NOT NULL CHECK (length(contract_version) BETWEEN 3 AND 11
                                AND contract_version GLOB '[0-9]*.[0-9]*'
                                AND contract_version NOT GLOB '*[^0-9.]*'
                                AND contract_version NOT GLOB '*.*.*'),
    computed_at             INTEGER NOT NULL,
    CHECK ((research_only = 1) = (relaxed_rule IS NOT NULL))
) STRICT;

CREATE INDEX prediction_runs_by_slot ON prediction_runs (season, week, lock_kind);
CREATE INDEX prediction_runs_by_model ON prediction_runs (model_version_id);

CREATE TRIGGER prediction_runs_no_update BEFORE UPDATE ON prediction_runs
BEGIN
    SELECT RAISE(ABORT, 'grid: prediction_runs: a prediction version is never overwritten (engine-output-contract 2)');
END;

CREATE TRIGGER prediction_runs_no_delete BEFORE DELETE ON prediction_runs
BEGIN
    SELECT RAISE(ABORT, 'grid: prediction_runs: a prediction version is never overwritten (engine-output-contract 2)');
END;

-- Snapshots: an audit record of every production pointer (engine-spec §8.7.4, §8.11 rule 6;
-- P1-02 D4, D14). pointer_count makes an incomplete snapshot detectable on read.
CREATE TABLE snapshots (
    snapshot_id      INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id           INTEGER NOT NULL REFERENCES jobs (job_id),
    -- sha256 of the canonical JSON of the effective engine settings.
    config_hash      TEXT NOT NULL CHECK (length(config_hash) = 64
                         AND config_hash NOT GLOB '*[^0-9a-f]*'),
    pointer_count    INTEGER NOT NULL CHECK (pointer_count >= 0),
    contract_version TEXT NOT NULL CHECK (length(contract_version) BETWEEN 3 AND 11
                         AND contract_version GLOB '[0-9]*.[0-9]*'
                         AND contract_version NOT GLOB '*[^0-9.]*'
                         AND contract_version NOT GLOB '*.*.*'),
    created_at       INTEGER NOT NULL
) STRICT;

CREATE TRIGGER snapshots_no_update BEFORE UPDATE ON snapshots
BEGIN
    SELECT RAISE(ABORT, 'grid: snapshots: immutable (engine-spec 8.7.4; KI-V10)');
END;

CREATE TRIGGER snapshots_no_delete BEFORE DELETE ON snapshots
BEGIN
    SELECT RAISE(ABORT, 'grid: snapshots: immutable (engine-spec 8.7.4; KI-V10)');
END;

-- The value of every pointer at a snapshot (P1-02 D3).
CREATE TABLE snapshot_pointers (
    snapshot_id      INTEGER NOT NULL REFERENCES snapshots (snapshot_id),
    model_family     TEXT NOT NULL CHECK (length(model_family) BETWEEN 1 AND 128),
    model_version_id INTEGER NOT NULL,
    PRIMARY KEY (snapshot_id, model_family),
    FOREIGN KEY (model_family, model_version_id)
        REFERENCES model_versions (model_family, model_version_id)
) STRICT, WITHOUT ROWID;

CREATE TRIGGER snapshot_pointers_within_count BEFORE INSERT ON snapshot_pointers
WHEN (SELECT COUNT(*) FROM snapshot_pointers WHERE snapshot_id = NEW.snapshot_id)
     >= (SELECT pointer_count FROM snapshots WHERE snapshot_id = NEW.snapshot_id)
BEGIN
    SELECT RAISE(ABORT, 'grid: snapshot_pointers: a snapshot holds exactly pointer_count pointers (P1-02 D14)');
END;

CREATE TRIGGER snapshot_pointers_no_update BEFORE UPDATE ON snapshot_pointers
BEGIN
    SELECT RAISE(ABORT, 'grid: snapshot_pointers: immutable (engine-spec 8.7.4; KI-V10)');
END;

CREATE TRIGGER snapshot_pointers_no_delete BEFORE DELETE ON snapshot_pointers
BEGIN
    SELECT RAISE(ABORT, 'grid: snapshot_pointers: immutable (engine-spec 8.7.4; KI-V10)');
END;

-- The current production version per model family (engine-spec §8.5, §8.7.4, §8.8, §8.13;
-- P1-02 D4, D14). The composite foreign key keeps a pointer inside its family.
CREATE TABLE production_pointers (
    model_family         TEXT NOT NULL PRIMARY KEY CHECK (length(model_family) BETWEEN 1 AND 128),
    model_version_id     INTEGER NOT NULL,
    -- The snapshot recorded before this move: every move leaves a rollback target.
    pre_move_snapshot_id INTEGER NOT NULL REFERENCES snapshots (snapshot_id),
    moved_by_job_id      INTEGER NOT NULL REFERENCES jobs (job_id),
    moved_at             INTEGER NOT NULL,
    FOREIGN KEY (model_family, model_version_id)
        REFERENCES model_versions (model_family, model_version_id)
) STRICT, WITHOUT ROWID;

CREATE TRIGGER production_pointers_name_production_on_insert BEFORE INSERT ON production_pointers
WHEN (SELECT state FROM model_versions
      WHERE model_version_id = NEW.model_version_id AND model_family = NEW.model_family)
     IS NOT 'PRODUCTION'
BEGIN
    SELECT RAISE(ABORT, 'grid: production_pointers: a pointer names a PRODUCTION version of its own family (engine-spec 8.8)');
END;

CREATE TRIGGER production_pointers_name_production_on_update BEFORE UPDATE ON production_pointers
WHEN (SELECT state FROM model_versions
      WHERE model_version_id = NEW.model_version_id AND model_family = NEW.model_family)
     IS NOT 'PRODUCTION'
BEGIN
    SELECT RAISE(ABORT, 'grid: production_pointers: a pointer names a PRODUCTION version of its own family (engine-spec 8.8)');
END;

CREATE TRIGGER production_pointers_no_delete BEFORE DELETE ON production_pointers
BEGIN
    SELECT RAISE(ABORT, 'grid: production_pointers: a pointer moves and is never deleted (engine-spec 8.7.4)');
END;

-- Delivery target 1 of every engine event (engine-spec §8.4; P1-02 D15). payload is the
-- internal JSON encoding of the event envelope.
CREATE TABLE diagnostic_events (
    event_id   INTEGER PRIMARY KEY AUTOINCREMENT,
    emitted_at INTEGER NOT NULL,
    event_name TEXT NOT NULL CHECK (event_name IN (
                   'DataFetchStarted', 'DataFetchCompleted', 'DataPartialSuccess', 'DataChanged',
                   'IdentityReviewRequired', 'FeaturesBuilt', 'ValueModelFitted',
                   'Layer1CreditComputed', 'RAPMUpdated', 'LatentStatesUpdated',
                   'FixedLagSmoothed', 'BoostingUpdated', 'ProjectionRunCompleted',
                   'BenchmarkSnapshotImported', 'EvaluationCompleted', 'CandidateValidated',
                   'CandidateRejected', 'ModelPromoted', 'RollbackCompleted', 'JobFailed',
                   'TrainingStatusChanged')),
    job_id     INTEGER REFERENCES jobs (job_id),
    payload    TEXT NOT NULL CHECK (length(payload) BETWEEN 2 AND 65536 AND json_valid(payload)
                   AND json_type(payload) = 'object')
) STRICT;

CREATE INDEX diagnostic_events_by_job ON diagnostic_events (job_id);
CREATE INDEX diagnostic_events_by_time ON diagnostic_events (emitted_at, event_id);

CREATE TRIGGER diagnostic_events_no_update BEFORE UPDATE ON diagnostic_events
BEGIN
    SELECT RAISE(ABORT, 'grid: diagnostic_events: append-only (engine-spec 8.4)');
END;

CREATE TRIGGER diagnostic_events_no_delete BEFORE DELETE ON diagnostic_events
BEGIN
    SELECT RAISE(ABORT, 'grid: diagnostic_events: append-only (engine-spec 8.4; retention is P1-11)');
END;
