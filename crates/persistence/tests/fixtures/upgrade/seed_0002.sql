-- Upgrade seed for migration 0002_backbone (ADR-015 D9; engine-spec §8.19 "upgrade fixture").
--
-- Synthetic, invented values only: at least one row in every table 0002 creates, written in an
-- order the CHECKs and triggers accept (each job is inserted PENDING, gains its edges while
-- PENDING, is claimed and then finished; each model version goes TRAINING -> CANDIDATE ->
-- VALIDATING -> PRODUCTION). Every row must survive every later migration
-- (crates/persistence/tests/migrations.rs). The config hashes are the sha256 of each job's
-- config_json, as P1-02 D6 defines them.

-- engine_settings: a second version of a seeded key, with the same value.
INSERT INTO engine_settings (key, version, value, created_at, note)
VALUES ('jobs.max_attempts', 2, '3', 1700000000, 'upgrade seed_0002');

-- Job 1: DATA_FETCH, with its ingestion run, retained raw response and a quarantined row.
INSERT INTO jobs (job_id, job_type, status, idempotency_key, config_hash, config_json,
                  input_data_version, ingestion_run_id, output_model_version, restartable,
                  attempt_count, max_attempts, not_before, created_at)
VALUES (1, 'DATA_FETCH', 'PENDING',
        'DATA_FETCH:-:fbe47f6d82b435a89f6e6fca65ad16138195de06c5fc3ffb66826a76e64e479f',
        'fbe47f6d82b435a89f6e6fca65ad16138195de06c5fc3ffb66826a76e64e479f', '{"seed":1}',
        NULL, NULL, NULL, 1, 0, 3, 1700000000, 1700000000);
UPDATE jobs SET status = 'RUNNING', attempt_count = 1, started_at = 1700000010,
                heartbeat_at = 1700000010, lease_expires_at = 1700000310, owner_pid = 4242
WHERE job_id = 1;

INSERT INTO ingestion_runs (ingestion_run_id, job_id, source_name, offline, network_fetch,
                            started_at, finished_at, terminal_state, error_code, error_message)
VALUES (1, 1, 'seed/source', 0, 1, 1700000010, 1700000020, 'PARTIAL_SUCCESS', NULL, NULL);

INSERT INTO artifact_manifest (artifact_id, sha256, path, byte_size, format, schema_version,
                               producing_job_id, created_at)
VALUES (1, 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
        'artifacts/sha256/e3/e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
        0, 'raw', 'seed-1', 1, 1700000020);

INSERT INTO raw_data (raw_data_id, ingestion_run_id, source_name, source_version,
                      source_timestamp, retrieved_at, artifact_id, sha256, row_count,
                      schema_version, validation_status)
VALUES (1, 1, 'seed/source', 'seed-v1', 1699999000, 1700000015, 1,
        'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855', NULL, NULL, NULL);

INSERT INTO quarantined_rows (quarantined_row_id, ingestion_run_id, source_name, reason_code,
                              raw_row_ref, created_at)
VALUES (1, 1, 'seed/source', 'seed_reason', 'seed-partition#row-7', 1700000020);

UPDATE jobs SET status = 'SUCCEEDED', completed_at = 1700000020, ingestion_run_id = 1
WHERE job_id = 1;

-- Job 2: NORMALIZE_DATA, which validates the raw response and commits data version 1.
INSERT INTO jobs (job_id, job_type, status, idempotency_key, config_hash, config_json,
                  input_data_version, ingestion_run_id, output_model_version, restartable,
                  attempt_count, max_attempts, not_before, created_at)
VALUES (2, 'NORMALIZE_DATA', 'PENDING',
        'NORMALIZE_DATA:-:a61e75dcd678010325906ec0497a92ff374228ab114bdf4eec89209e2576ebb2',
        'a61e75dcd678010325906ec0497a92ff374228ab114bdf4eec89209e2576ebb2', '{"seed":2}',
        NULL, NULL, NULL, 1, 0, 3, 1700000000, 1700000000);
INSERT INTO job_dependencies (job_id, upstream_key, allow_skipped)
VALUES (2, 'DATA_FETCH:-:fbe47f6d82b435a89f6e6fca65ad16138195de06c5fc3ffb66826a76e64e479f', 0);
UPDATE jobs SET status = 'RUNNING', attempt_count = 1, started_at = 1700000030,
                heartbeat_at = 1700000030, lease_expires_at = 1700000330, owner_pid = 4242
WHERE job_id = 2;

UPDATE raw_data SET row_count = 12, schema_version = 'seed-schema-1', validation_status = 'seed-valid'
WHERE raw_data_id = 1;

INSERT INTO data_versions (data_version, parent_data_version, job_id, created_at)
VALUES (1, NULL, 2, 1700000040);
INSERT INTO data_version_inputs (data_version, raw_data_id) VALUES (1, 1);

UPDATE jobs SET status = 'SUCCEEDED', completed_at = 1700000040 WHERE job_id = 2;

-- Job 3: BOOSTING_UPDATE on data version 1, which trains model version 1 to VALIDATING.
INSERT INTO jobs (job_id, job_type, status, idempotency_key, config_hash, config_json,
                  input_data_version, ingestion_run_id, output_model_version, restartable,
                  attempt_count, max_attempts, not_before, created_at)
VALUES (3, 'BOOSTING_UPDATE', 'PENDING',
        'BOOSTING_UPDATE:1:4d687b6437462c4d42e6b3ca5812976a72f39b30a2d78d5f5d1be4880f9aa5a5',
        '4d687b6437462c4d42e6b3ca5812976a72f39b30a2d78d5f5d1be4880f9aa5a5', '{"seed":3}',
        1, NULL, NULL, 0, 0, 3, 1700000000, 1700000000);
INSERT INTO job_dependencies (job_id, upstream_key, allow_skipped)
VALUES (3, 'NORMALIZE_DATA:-:a61e75dcd678010325906ec0497a92ff374228ab114bdf4eec89209e2576ebb2', 0);
UPDATE jobs SET status = 'RUNNING', attempt_count = 1, started_at = 1700000050,
                heartbeat_at = 1700000050, lease_expires_at = 1700000350, owner_pid = 4242
WHERE job_id = 3;

INSERT INTO training_runs (training_run_id, job_id, model_family, data_version, config_hash,
                           started_at, completed_at)
VALUES (1, 3, 'seed_family', 1,
        '4d687b6437462c4d42e6b3ca5812976a72f39b30a2d78d5f5d1be4880f9aa5a5', 1700000050, NULL);

INSERT INTO model_versions (model_version_id, model_family, model_type, state, state_changed_at,
                            training_run_id, data_version, feature_schema_version,
                            hyperparameters_json, config_hash, random_seed, engine_crate_version,
                            engine_git_sha, contract_version, created_at)
VALUES (1, 'seed_family', 'seed_type', 'TRAINING', 1700000050, 1, 1, 1, '{"lambda":0.5}',
        '4d687b6437462c4d42e6b3ca5812976a72f39b30a2d78d5f5d1be4880f9aa5a5', '42',
        '0.1.0-alpha.1', NULL, '1.1', 1700000050);
UPDATE model_versions SET state = 'CANDIDATE', state_changed_at = 1700000060
WHERE model_version_id = 1;
UPDATE model_versions SET state = 'VALIDATING', state_changed_at = 1700000061
WHERE model_version_id = 1;

UPDATE training_runs SET completed_at = 1700000060 WHERE training_run_id = 1;

UPDATE jobs SET status = 'SUCCEEDED', completed_at = 1700000060, output_model_version = 1
WHERE job_id = 3;

-- Job 4: MODEL_PROMOTE, which snapshots the (empty) pointers, promotes model version 1 and
-- snapshots again, and records its event.
INSERT INTO jobs (job_id, job_type, status, idempotency_key, config_hash, config_json,
                  input_data_version, ingestion_run_id, output_model_version, restartable,
                  attempt_count, max_attempts, not_before, created_at)
VALUES (4, 'MODEL_PROMOTE', 'PENDING',
        'MODEL_PROMOTE:1:ef731a0a50cf73d25aa0bd97853117727063d6d5c77e5768de5ad4c8496b38c2',
        'ef731a0a50cf73d25aa0bd97853117727063d6d5c77e5768de5ad4c8496b38c2', '{"seed":4}',
        1, NULL, NULL, 0, 0, 3, 1700000000, 1700000000);
INSERT INTO job_dependencies (job_id, upstream_key, allow_skipped)
VALUES (4, 'BOOSTING_UPDATE:1:4d687b6437462c4d42e6b3ca5812976a72f39b30a2d78d5f5d1be4880f9aa5a5', 0);
UPDATE jobs SET status = 'RUNNING', attempt_count = 1, started_at = 1700000070,
                heartbeat_at = 1700000070, lease_expires_at = 1700000370, owner_pid = 4242
WHERE job_id = 4;

INSERT INTO snapshots (snapshot_id, job_id, config_hash, pointer_count, contract_version,
                       created_at)
VALUES (1, 4, 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855', 0, '1.1',
        1700000070);
UPDATE model_versions SET state = 'PRODUCTION', state_changed_at = 1700000071
WHERE model_version_id = 1;
INSERT INTO production_pointers (model_family, model_version_id, pre_move_snapshot_id,
                                 moved_by_job_id, moved_at)
VALUES ('seed_family', 1, 1, 4, 1700000071);
INSERT INTO snapshots (snapshot_id, job_id, config_hash, pointer_count, contract_version,
                       created_at)
VALUES (2, 4, 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855', 1, '1.1',
        1700000072);
INSERT INTO snapshot_pointers (snapshot_id, model_family, model_version_id)
VALUES (2, 'seed_family', 1);

INSERT INTO diagnostic_events (event_id, emitted_at, event_name, job_id, payload)
VALUES (1, 1700000072, 'ModelPromoted', 4, '{"seed":"ModelPromoted"}');

UPDATE jobs SET status = 'SUCCEEDED', completed_at = 1700000072 WHERE job_id = 4;

-- Job 5: PREDICT, which records one prediction run (a u64::MAX seed, as canonical text).
INSERT INTO jobs (job_id, job_type, status, idempotency_key, config_hash, config_json,
                  input_data_version, ingestion_run_id, output_model_version, restartable,
                  attempt_count, max_attempts, not_before, created_at)
VALUES (5, 'PREDICT', 'PENDING',
        'PREDICT:1:5c3c68cee554fe6fbe8ff3e033c4a2017cc9959999ec7a1eba8b1ad15de537a6',
        '5c3c68cee554fe6fbe8ff3e033c4a2017cc9959999ec7a1eba8b1ad15de537a6', '{"seed":5}',
        1, NULL, NULL, 1, 0, 3, 1700000000, 1700000000);
INSERT INTO job_dependencies (job_id, upstream_key, allow_skipped)
VALUES (5, 'MODEL_PROMOTE:1:ef731a0a50cf73d25aa0bd97853117727063d6d5c77e5768de5ad4c8496b38c2', 0);
UPDATE jobs SET status = 'RUNNING', attempt_count = 1, started_at = 1700000080,
                heartbeat_at = 1700000080, lease_expires_at = 1700000380, owner_pid = 4242
WHERE job_id = 5;

INSERT INTO prediction_runs (prediction_version_id, job_id, season, week, lock_kind, lock_at,
                             data_version, feature_schema_version, model_version_id,
                             engine_crate_version, engine_git_sha, scoring_profile_id,
                             scoring_profile_version, seed, n_draws, research_only, relaxed_rule,
                             contract_version, computed_at)
VALUES (1, 5, 2024, 1, 'Sunday', 1700100000, 1, 1, 1, '0.1.0-alpha.1', 'abcdef1', 'PPR', 1,
        '18446744073709551615', 1000, 0, NULL, '1.1', 1700000090);

UPDATE jobs SET status = 'SUCCEEDED', completed_at = 1700000090 WHERE job_id = 5;

-- Job 6: EVALUATE, still PENDING behind job 5, on an edge that allows a skipped upstream.
INSERT INTO jobs (job_id, job_type, status, idempotency_key, config_hash, config_json,
                  input_data_version, ingestion_run_id, output_model_version, restartable,
                  attempt_count, max_attempts, not_before, created_at)
VALUES (6, 'EVALUATE', 'PENDING',
        'EVALUATE:1:1dc5900f4db06df1078b057b83f2bc188de8cfe6e1b8c1d323105dcab9eb9047',
        '1dc5900f4db06df1078b057b83f2bc188de8cfe6e1b8c1d323105dcab9eb9047', '{"seed":6}',
        1, NULL, NULL, 1, 0, 3, 1700000000, 1700000000);
INSERT INTO job_dependencies (job_id, upstream_key, allow_skipped)
VALUES (6, 'PREDICT:1:5c3c68cee554fe6fbe8ff3e033c4a2017cc9959999ec7a1eba8b1ad15de537a6', 1);
