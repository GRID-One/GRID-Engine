---
work-package-id: P1-02
status: In Progress
risk-class: High
owner: Product/Architecture owner
implementer: Claude Code (the implementing agent; model identity unrecorded, see Evidence required)
reviewer: Pending — fresh-context reviewer agents who are not the implementer, then the owner's merge
phase: 1
---

# P1-02 — SQLite backbone, `grid migrate`, durable job ledger, artifact store and production pointers

## Status
**In Progress.**
- **Start.** Started on 2026-10-21 on branch `claude/zen-hopper-ax8tge`, from `main` at the P0-03
  merge (`4b8df86`, PR #7).
- **Branch name.** It is harness-assigned, not `wp/P1-02-…` (root `CLAUDE.md`, "Branch, commit, PR").
  P0-01, P0-02, P0-03 and P1-01 record the same deviation.
- **Readiness.** The package was Ready when it started:
  - both decisions it depends on are ratified for its scope (see "Owner decisions depended on");
  - P1-01 is merged and its contracts are Approved;
  - the behaviour-changing choices the specification leaves to P1-02 were put to the owner, who ruled
    on them (R1 to R6 below) before any code was written.
- **Done when** every criterion below has passed on the final content commit, a fresh-context review
  is complete and the owner has merged the package with the approvals listed under "Ownership and
  risk" (engine-spec §8.16.2).
- **Not ticked yet.** No criterion below is ticked. The runs on the final content commit are recorded
  in `.ai/evidence/P1-02/`.

## Ownership and risk
- **Owner.** Product/Architecture owner. The repository owner holds every role today.
- **Implementer.** Claude Code (the implementing agent): one orchestrating session, implementer agents
  on disjoint file sets, and read-only reviewer agents (engine-spec §8.18).
- **Reviewer.** Pending: fresh-context reviewer agents who are not the implementer (security and
  dependencies, data contract, numerical, adversarial PR), then the owner.
- **Risk class: High.** It creates the durable store that every later package writes to:
  - the backbone tables;
  - the migration policy;
  - the job ledger and its crash semantics;
  - the artifact protocol;
  - the production-pointer transaction.

  Table names are permanent once the migration lands, and an error in the crash semantics corrupts or
  hides state.
- **Required human approvals.**
  1. **Product/Architecture owner.**
     - Accept or reject ADR-015 (explicit `grid migrate`).
     - Approve migration `0002_backbone.sql` and the names and semantics it fixes (the decisions
       below).
     - Approve the specification edits (§8.5, §8.6.2, §8.10, §8.11, §8.13, §9.5) and grid.output 1.1.
     - Approve the deviation from the P0-03 follow-up note (R4).

     The owner's merge is the approval.
  2. **Statistical and Product/Architecture owners.** Accept or reject ADR-016, the DR-B6 adopting
     record.
  3. **Security/Release owner.** The `sha2` dependency (approved in R2; the §14.2 record is below), the
     first use of the declared `tokio` and `tracing` workspace dependencies, the `Cargo.lock` delta and
     the `.gitattributes` rule. `.claude/settings.json` is unchanged.
  4. **Merge reviewer.** A merge commit, never a squash.
- **Agent budget (engine-spec §8.18.1).**
  - at most 600 agent turns across the implementing, reviewing and remediating agents;
  - a wall-clock timeout of 24 hours of active work for the implementing session;
  - one concurrent writer on the source tree, as `ai-toolchain.lock` sets
    (`max_concurrent_writers: 1`), with read-only agents in parallel;
  - no cost ceiling (`cost_ceiling_usd: null`).

  Exhaustion produces a partial evidence record and a Blocked package, never a skipped check.

## Authority
- **Engine spec (`engine-spec.md`):** §1.1, §1.3–§1.5, §1.7, §4.1.1, §4.5, §5.5, §6.8, §7.12.6,
  §8.1, §8.2–§8.4, §8.5, §8.6.2, §8.6.4, §8.7.4, §8.7.5, §8.8, §8.9, §8.10, §8.11, §8.12, §8.13, §8.14,
  §8.16–§8.20, §9.2.2, §9.4, §9.5, §12.5, §12.6, §12.8, §12.9, §14.2, §14.3, Appendices C and D.
- **ADRs:**
  - ADR-001 (D5 frozen recipes);
  - ADR-002 (the offline cache; `schema_meta` left to P1-02);
  - ADR-003, ADR-004 (line endings), ADR-005 (`check-sqlx --workspace`);
  - ADR-012 (§(e) of `PARITY.md`), ADR-014;
  - ADR-015 and ADR-016, Proposed by this package.
- **Contracts:** `docs/03-contracts/engine-output-contract.md` §2 (lineage), §9, §10, §12, §13 (moved
  to version 1.1 here); `docs/03-contracts/README.md` (versioning rules, derived artifacts).
- **Model specs:** none implemented. `latent_states`, the RAPM blocks and the other model-specific
  tables are out of scope (R1).
- **Registers (tracking only):** `docs/00-meta/decision-register.md` (DR-B6, DR-C15),
  `docs/00-meta/known-issues.md` (KI-A8, KI-A9, KI-G6, KI-G9, KI-V10, KI-NEW-W5, KI-NEW-Z14,
  KI-NEW-Z16).

## Owner decisions depended on
Statuses are those of `docs/00-meta/decision-register.md` on 2026-10-21.

| DR | Decision | Status | How this package uses it |
|---|---|---|---|
| DR-B6 | Typed failure, not a faithful port of oracle failure paths | Ratified in part 2026-10-08. The open part (the ill-conditioning threshold) blocks P1-06 only | `StateCorrupt` on missing or altered persisted state; typed job failure; the atomic stage commit; ADR-016 records the policy |
| DR-C15 | SQLite source of truth; immutable, content-addressed, registered bulk artifacts; the §8.11 write protocol; a missing or mismatched listed file is `StateCorrupt` | Ratified 2026-10-08 | `artifact_manifest` and the artifact store |

### Owner rulings in this package
The owner answered six questions in this session (`docs/06-sessions/2026-11-06-P1-02.md`). The chosen
options are quoted verbatim.

- **R1 (2026-10-21). Schema scope: "Backbone only".**
  > P1-02 creates the tables whose columns the spec already fixes: jobs and their edges, data
  > versions, raw_data, ingestion_runs, artifact_manifest, production_pointers, snapshots,
  > model_versions, training_runs, prediction_runs, engine_settings, diagnostic_events and
  > quarantined_rows. Every NFL-data, identity, feature, context, benchmark and model-specific
  > GRID-state table lands in the migration of the package that owns its contract or model spec,
  > under the §8.5 names. I would record this as your ruling, with a matching edit to §8.5 and §9.5.
- **R2 (2026-10-21). Hashing: "Approve sha2".**
  > Use RustCrypto `sha2` 0.10 (MIT OR Apache-2.0). It is already in Cargo.lock at 0.10.9 as a
  > transitive dependency of sqlx-core, so nothing new is downloaded or audited. I would add it to
  > [workspace.dependencies], have grid-persistence use it alone, and record the §14.2 justification
  > (licence, advisories, maintenance, platforms) in the work-package file.
- **R3 (2026-10-21). Migration policy: "Explicit `grid migrate`".**
  > Every start-up compares the database with the binary's migrations. If any are pending, the engine
  > refuses with a typed error and a distinct exit code until an operator runs `grid migrate`, which
  > applies them under the process lock. A database that is newer than the binary, or whose checksums
  > mismatch, is a typed failure. A scheduled `grid update` never changes the schema on its own.

  ADR-015 records it. The "distinct exit code" is the store-refusal code 30 of R6.
- **R4 (2026-10-21). Façade: "Engine struct, traits later".**
  > P1-02 builds `pipeline::Engine`, which owns the Tokio runtime, the SQLite pool, the process lock,
  > the artifact store, the job runner and event delivery to diagnostic_events and tracing. It exposes
  > inherent methods only for what P1-02 delivers: open/migrate and the job ledger. Each trait impl
  > lands once every one of its methods has a real stage, so no method is ever a placeholder. I would
  > record this as a deviation from the P0-03 follow-up note.
- **R5. Lock model: "Lease-based, narrow locks".**
  > Every invocation takes a shared lock, and `grid migrate` alone takes it exclusively, so the schema
  > never changes under a running process. `grid update` also holds a separate exclusive update lock,
  > so two updates never overlap. Rebuilds, exports, evaluations and queries run alongside it. A
  > RUNNING job counts as stale only when its heartbeat lease has expired, as §8.10 says, with no PID
  > probing. After a crash its job resumes once the lease lapses (default 5 min). Promotion and
  > rollback stay serialized by BEGIN IMMEDIATE plus a one-running-publication constraint.

  The ruling resolves the conflict between engine-spec §8.11 step 3 and §8.13 on one side, and §9.4
  and Appendix H on the other. The ruling was recorded together with a matching spec edit.
- **R6. Store refusals: "Reuse 30, clarify meaning".**
  > Every one of these cases exits 30. DATABASE_FAILURE is re-documented as 'the durable store
  > (SQLite database, registered artifacts or store locks) failed or is not usable'. The precise cause
  > travels in the EngineErrorCode and message: UnsupportedVersion when the schema is not current,
  > StateCorrupt for integrity failures, Internal (re-documented as exiting with its attached terminal
  > state when it has one) for I/O and a busy store. The contract goes to 1.1, recorded as your ruling
  > that this is a clarification and not MAJOR. No enum grows.

R5 and R6 were answered after 2026-10-22, while the session was resuming across worker restarts. The
session log records them.

### Decisions this package makes
These are made under the authority the specification and R1–R6 give P1-02, and each is flagged for
the owner's merge review.

| # | Decision | Authority |
|---|---|---|
| D1 | **Crate graph.** `governance → domain` (pure policy); `persistence → domain, sqlx, sha2` (SQL and files, no business logic); `pipeline → domain, governance, persistence, tokio, tracing` (the transaction that reads, plans with governance and executes; job execution). `ModelState` moves from `pipeline::queries` to `grid_domain::status` and is re-exported, with no serialization change | §8.1 rows `persistence`, `governance`, `pipeline`; `docs/CLAUDE.md` rule 3 |
| D2 | **Job types.** `StageId` stays at its 20 values, which already equal the §8.10 list ("P1-02 fixes the full enumeration"). The P0-03 note's "extending `StageId`" is not needed, so no MAJOR change. An informative §8.6.2 node → job mapping is in "Follow-up" | §8.10; P0-03 Follow-up |
| D3 | **Child tables.** Three child tables beyond R1's list: `job_dependencies` (R1's "edges"), `data_version_inputs` (raw inputs of a data version, for the §14.3 retention rule) and `snapshot_pointers` (the value of every pointer at a snapshot) | §8.5 as edited; §14.3 item 2 |
| D4 | **Columns beyond the spec's lists.** `jobs`: `config_json` (durable stage inputs), `max_attempts` (the setting at enqueue), `not_before` (persisted backoff), `restartable` (declared at enqueue, §8.10 "if its stage can restart from durable inputs"), `heartbeat_at`, `lease_expires_at`. `ingestion_runs`: `offline`, `network_fetch`. `artifact_manifest`: a surrogate `artifact_id`. `data_versions`: `parent_data_version`, `job_id`. `snapshots`: `pointer_count`, `config_hash`, `contract_version`. `model_versions`: `state_changed_at`, `contract_version`. `production_pointers`: `pre_move_snapshot_id`, `moved_by_job_id`. `prediction_runs`: `job_id`, `contract_version`. `engine_settings`: `version`, `note` | §8.5, §8.9, §8.10; contracts README "derived artifacts" |
| D5 | **Stored spellings.** Enum text uses the spec spellings (`StageId`, `JobStatus`, `TerminalState`, `ModelState`: SCREAMING_SNAKE) under `CHECK … IN` lists; `LockKind` uses its variant names. This is internal database spelling and does not pre-empt engine-output-contract §12's serialized-spelling item. Ids are `INTEGER` (u64 ↔ i64, checked); instants are `UtcInstant` seconds; tables are `STRICT` | §8.10; contract §12 |
| D6 | **Idempotency.** `idempotency_key = "<STAGE>:<data version or ->:<config_hash>"`. `config_hash` = sha256 of the canonical JSON of the job's configuration (keys sorted recursively, compact, root an object). The configuration carries the job's scope (season, week, source, lock kind, local day), which keeps §8.10's triple exact while separating per-week, per-source and per-lock jobs. A key with a PENDING, RUNNING, SUCCEEDED or SKIPPED row is reused, never re-executed. A FAILED or CANCELLED key re-enqueues as a new row | §8.10 Idempotency, Stale `RUNNING` |
| D7 | **Edges are by key**, so a re-enqueued upstream key reconnects its dependents. Enqueue refuses a cycle. A job starts only when every upstream key's latest row is SUCCEEDED, or SKIPPED_NO_NEW_DATA where its edge allows it. Otherwise it stays PENDING and is reported as *blocked*. That is a derived view, never a stored status: no `JobStatus` meaning changes | §8.10 Dependencies; `status.rs` docs |
| D8 | **Retries.** Every failure retries with persisted backoff until `max_attempts`; exhaustion sets FAILED and writes `JobFailed` ("Failed: attempts exhausted"). CANCELLED is used only at shutdown. SKIPPED_NO_NEW_DATA only when the stage reports it. Settings defaults: `jobs.max_attempts = 3`, `jobs.backoff_initial_secs = 5`, `jobs.backoff_max_secs = 60` (delay before attempt n ≥ 2 is min(5 s × 2ⁿ⁻², 60 s)), `jobs.lease_secs = 300`, `jobs.heartbeat_secs = 30` (2 × heartbeat < lease), `engine.shutdown_grace_secs = 30` | §8.10 Retries ("fixed in P1-02") |
| D9 | **Stale `RUNNING`** (R5). A RUNNING row whose lease has expired is stale. If its job is `restartable`, it goes back to PENDING (the crashed attempt counts). Otherwise it is marked FAILED, with `JobFailed`, and re-enqueued under the same key. If its attempts are exhausted it is FAILED. Recovery runs at every writer start-up; compare-and-set makes concurrent recoverers safe | §8.10 as edited; §8.11 step 6 |
| D10 | **Ingestion runs** are one row per source fetch, written in the stage's final success or failure commit, so no orphan RUNNING row exists. A running fetch is visible as its RUNNING `DATA_FETCH` job | `SourceFetchResult` (`queries.rs`); §8.6.4 |
| D11 | **`raw_data` has no uniqueness on (source, hash).** Whether a re-retrieval identical to an *earlier* (not the latest) retrieval is NO_NEW_DATA, the A → B → A revert case of the KI-A9 class, is P1-03's decision (Follow-up F-3). Bytes are de-duplicated in the artifact store | §4.1.1; KI-A9 |
| D12 | **Data versions** form one global linear chain per store; backtests and tests isolate by data directory (§4.5 "an injected namespace") | §8.6.2 "commit SQLite data version"; contract §2 |
| D13 | **Model lifecycle.** The §8.8 state lives on `model_versions.state`, with the permitted transitions enforced by governance and by a trigger. The §8.5 name `model_states` stays reserved for the persisted model state of P1-06 and P1-12. Validation metrics live in P1-09's §8.5 evaluation tables | §8.5, §8.8 |
| D14 | **Pointers.** Promotion writes, in one `BEGIN IMMEDIATE` transaction: a snapshot of every pointer, then the old version PRODUCTION → SUPERSEDED, the candidate VALIDATING → PRODUCTION, and the pointer upsert. Rollback moves only differing families, refuses a target that is not SUPERSEDED, leaves families absent from the snapshot untouched and reports them, and never copies or deletes. P1-02 delivers the primitives, not the §8.2 commands or the promotion gate (P2-07) | §8.7.4, §8.8, §8.13 |
| D15 | **Event delivery.** The `diagnostic_events` row is written in the payload's own transaction. After commit, the engine emits the `tracing` record and then the in-process subscribers, on the calling thread. Delivery to the durable table is exactly once; delivery to the live channels is at most once across a crash. This is how P1-02 reads §8.4's "MUST be delivered to all three" for a process that dies between commit and delivery | §8.4; §12.6 |
| D16 | **The artifact store.** Content-addressed paths `artifacts/sha256/<h[0..2]>/<h>` under the data directory (default `data/`); temporary files in `artifacts/tmp/`. The §8.11 order: write the temp file, `sync_all`, rename, fsync the directory (Unix; Windows cannot fsync a directory from std, a gap recorded for P1-11), register in the caller's transaction. A missing, short or altered file is `StateCorrupt`. Unregistered files are collectable by a tested primitive; scheduling is P1-11's | §8.11; DR-C15; §14.3 |
| D17 | **SQLite connection.** WAL, `synchronous = FULL`, `foreign_keys`, `busy_timeout` 5 s. Every write uses `BEGIN IMMEDIATE`, and no write transaction spans a computation, so readers are never blocked | §8.13 |
| D18 | **No closed enum grows.** `StageId` 20, `JobStatus` 6, `TerminalState` 14, `EngineErrorCode` 17, `EngineEvent` 21, `ModelState` 6 | contract §9 |

## Objective
`migrations/0002_backbone.sql` creates the R1 backbone, and the workspace delivers:
- `grid migrate` (ADR-015);
- a `pipeline::Engine` that opens the store under the R5 locks, refuses an uncurrent schema (R6) and
  recovers stale jobs;
- a durable job ledger with idempotency, key-based dependencies, persisted backoff, leases and
  cancellation;
- the §8.11 artifact write protocol with `StateCorrupt` on any integrity failure;
- the production-pointer and snapshot transaction;
- event delivery to `diagnostic_events`, `tracing` and subscribers.

All of it passes the frozen verify chain on Linux and on the merge-authoritative Windows job.

## Observable outcome
- **Operator.**
  - `grid migrate` creates `data/grid.db` on a fresh checkout and exits 0; run again, it applies
    nothing and exits 0.
  - Any other engine invocation against a missing, pending, newer or corrupt store exits 30 with a
    typed message (`grid exit-codes` is unchanged).
  - `grid migrate` with an extra argument exits 2.
- **Library consumers** see `grid_pipeline::Engine` (open, migrate, enqueue a job plan, run its jobs,
  read job records, request shutdown, close), the stage traits a later package implements, and
  `grid_persistence`'s typed store API.
- **The façade traits** `EngineCommands` and `EngineQueries` still have no implementor, by R4.
- **Nothing computes a projection.**

## Preconditions
- P1-01 merged (`8755552`) and P0-03 merged (`4b8df86`, PR #7). Both are.
- DR-B6 and DR-C15 ratified for this scope. They are.
- R1 to R6 given. They are.
- **Environment:**
  - the pinned dev tools of `toolchains/dev-tools.lock`;
  - `DATABASE_URL` and `SQLX_OFFLINE` from the committed `.env`;
  - a development database from `just bootstrap`, re-created when `0002_backbone.sql` changes on the
    branch: delete `target/grid-dev.db`, then `sqlx database create && sqlx migrate run`.

  No Python.

## Scope

### Rust workspace
- **`Cargo.toml`:** `[workspace.dependencies]` gains `grid-governance` and `grid-persistence` (path
  plus `version`, for cargo-deny `wildcards`) and `sha2 = "0.10"` (R2). `Cargo.lock` is regenerated
  by cargo's resolution only, never `cargo update`.
- **`.gitattributes`:** `migrations/*.sql text eol=lf` (ADR-015 D7).
- **`migrations/0002_backbone.sql`:** the backbone tables, indexes and triggers. `.sqlx/` is
  regenerated by `cargo sqlx prepare --workspace -- --lib`.
- **`crates/domain/`:**
  - `ModelState` moves into `src/status.rs`;
  - the R6 clarifications of `TerminalState::DatabaseFailure` (`src/status.rs`) and
    `EngineErrorCode::Internal` (`src/error.rs`);
  - `GRID_OUTPUT_CONTRACT_VERSION` = 1.1 (`src/output/mod.rs`).
- **`crates/governance/`:** job policy (transitions, readiness, retry, backoff, stale decision,
  idempotency key), model-state transitions, pointer and rollback plans, settings parsing, typed
  errors, unit tests.
- **`crates/persistence/`:**
  - `build.rs` (migration rebuild trigger);
  - the store (`db`, `schema`, `codec`, `hash`, `artifacts`, `jobs`, `ledger`, `models`, `pointers`,
    `events`, `settings`, `error`);
  - integration tests;
  - upgrade seeds under `crates/persistence/tests/fixtures/upgrade/`.
- **`crates/pipeline/`:**
  - `src/engine/` (the `Engine`: configuration, failure, store locks, lifecycle, recovery, delivery,
    shutdown);
  - `src/jobs/` (job plan, runner, stage traits);
  - `src/canonical.rs`;
  - the `ModelState` re-export in `src/queries.rs`;
  - doc updates in `src/lib.rs` and `src/events.rs`;
  - integration tests;
  - additive rules in `tests/crate_boundaries.rs` and an additive pin in
    `tests/serialized_spelling.rs`.
- **`crates/grid-cli/`:** the `grid migrate` subcommand and its tests.

### Documents
- this file and `docs/01-work-packages/README.md`;
- `docs/02-adr/015-explicit-grid-migrate.md`,
  `docs/02-adr/016-typed-failure-for-oracle-silent-failure-paths.md` and `docs/02-adr/README.md`;
- `engine-spec.md` and `docs/00-meta/specs/engine-spec.md`, byte-identical: §8.5, §8.6.2, §8.10,
  §8.11, §8.13 and §9.5, per R1, R3 and R5;
- `docs/03-contracts/engine-output-contract.md` (version 1.1; §13 store refusals and `migrate`) and
  `docs/03-contracts/README.md`;
- `reference/python/PARITY.md` §(e): Rust-status notes for rows 2 (generic `StateCorrupt` only), 3,
  5 and 6, naming the tests;
- `docs/00-meta/decision-register.md`: "where implemented" notes for DR-B6 and DR-C15;
- `docs/CLAUDE.md` (crate table, rule 3 for `sha2`);
- `docs/00-meta/authority-index.md`, `docs/00-meta/dashboard.md`, `docs/00-meta/daily-log.md`,
  `README.md`;
- `docs/06-sessions/2026-11-06-P1-02.md` and `docs/06-sessions/README.md`;
- `docs/01-work-packages/p0-03-record-p1-01-merge-rulings.md`: status-only, Done (PR #7);
- `.ai/evidence/P1-02/`.

### Contracts consumed
grid.output §2 (lineage group of `prediction_runs`), §10 (types), §13.

### Contracts changed
grid.output 1.0 → 1.1 (R6): §13 store refusals and `migrate`. No field, enum value, state or code is
added or moved.

## Non-goals
Each item has an owner.
- **Every §8.5 table not in R1's list.** It goes to its contract's or model spec's package:
  - P1-03: NFL data and labels;
  - P1-04: identity and NCAA;
  - P1-05: features;
  - P1-06 and P1-12: `model_states`, `latent_states`, the RAPM blocks and column maps,
    `value_model_versions`, `layer1_credit`, `feeder_player_seasons`, `player_ratings`, `teams`;
  - P1-08 and P1-10: predictions detail;
  - P1-09 and P2-05: benchmarks and evaluation;
  - P2-02: availability.
- **`EngineCommands` / `EngineQueries` implementations** (R4).
- **Built-in SNAPSHOT, MODEL_PROMOTE and ROLLBACK stages, the promotion gate and auto-rollback**
  (P2-07; DR-C5).
- **Wiring the §8.6.2 DAG into jobs** (F-1).
- **The JSON `tracing` formatter and lifecycle step 1** (P1-10, F-2). P1-02 emits records and spans
  only.
- **Fetch, raw retention policy and normalization** (P1-03). P1-02 delivers the tables and primitives.
- **GC scheduling and retention of `jobs` and `diagnostic_events`**; a kill during migration;
  end-to-end crash recovery (P1-11).
- **A settings query or command** (F-5).

## Inputs and fixtures
- Synthetic upgrade seeds under `crates/persistence/tests/fixtures/upgrade/`: hand-written SQL inserts
  of invented rows, no third-party data.
- No parity fixture.

## Oracle parity targets
None.
- The oracle's `.npz`, joblib and Parquet caches, its in-code `connection.py` migrations and its
  `weekly_update` failure paths are not ported (engine-spec §9.5).
- Anti-targets, each asserted by a typed-failure test (§7.12.6; ADR-016):
  - **KI-A8 (generic part):** a missing, truncated or altered registered artifact is `StateCorrupt`,
    never empty state;
  - **KI-G6 / KI-NEW-W5:** a failed job is never skipped, and validation is NULL until set, never a
    default "ok";
  - **KI-NEW-Z14:** stage outputs commit in one transaction with the job's success;
  - **KI-G9 / KI-V10:** atomic artifact writes, and an interrupted snapshot leaves earlier snapshots
    intact;
  - **KI-A9:** a correction is a new data version;
  - **KI-NEW-Z16 (primitive only):** immutable versioned records.

## Implementation constraints
- **Typed failures.** No `unwrap` or `expect` on persisted input; every failure is typed. Every
  value under a `CHECK` constraint is validated or normalized in Rust first: error messages are truncated to 2000
  characters on a character boundary, and an empty one becomes the code name. A CHECK then fires only
  on a programming error, and a failure path can never make the store unopenable.
- **Messages.** Store errors never carry an absolute path, a host or `sqlx::Error`'s raw text
  (`EngineError` rule).
- **`query!`** only in library code. Tests use runtime `sqlx::query`. `.sqlx/` is regenerated by the
  orchestrating session.
- **The async runtime.** The `Engine` builds a `current_thread` runtime and refuses to open inside
  another runtime. Computation runs on `spawn_blocking`. No transaction or pool connection outlives the
  `block_on` that created it. `Drop` closes the pool inside the runtime and shuts down with a bounded
  grace.
- **Numerical crates** never reach `tokio`, `sqlx` or `sha2` (`crates/pipeline/tests/crate_boundaries.rs`).
- **Platforms.** Locks use std `File::try_lock` and `try_lock_shared` (no dependency). Rename and
  remove get a bounded retry on Windows `PermissionDenied`.
- **Untracked files.** No test writes outside `target/` or the OS temp directory.

## Security and licensing considerations
- **The `sha2` §14.2 record (R2).**
  - **Justification:** §8.11 step 4 "record the sha256", §14.3 "the manifest hash verified on read",
    and DR-C15.
  - **Licence:** MIT OR Apache-2.0, within `deny.toml`.
  - **Advisories:** none at selection; `cargo deny` and `cargo audit` run in the verify chain.
  - **Maintenance:** RustCrypto/hashes, widely used.
  - **Platforms:** pure Rust on both targets.
  - **Lockfile:** already locked at 0.10.9 through `sqlx-core`, so no new package is downloaded.
  - **Use:** `grid-persistence` only, enforced by a boundary rule.
  - **Not covered:** the in-Rust check of DR-D29 parity-fixture hashes, which stays with the shell
    guard.
- **First use of declared workspace dependencies:**
  - `tokio` (`full` features, in `grid-pipeline`; it adds `tokio-macros` and `signal-hook-registry`
    to the lock);
  - `tracing` (already locked).

  No other dependency is added.
- **Permissions.** No secret, credential or host. `.claude/settings.json` and `.github/` are
  unchanged.
- **Data directory.** `data/` and the pre-migration copies in `data/backups/` are gitignored (`/data/`)
  and never committed.

## Acceptance criteria
Each criterion is verified on the final content commit; the test that evidences it is named.

**Migrations and policy**
- [ ] A blank database migrates to head: `crates/persistence/tests/migrations.rs::blank_database_reaches_head`
  (§8.19; R1's tables exist, `schema_meta` kept).
- [ ] An upgrade from every prior version preserves its seeded rows:
  `migrations.rs::upgrade_from_every_prior_version_preserves_seeded_rows`, `every_migration_has_an_upgrade_seed`
  (§8.19 "upgrade fixture").
- [ ] Migrations are LF-only and contain no unapproved destructive statement:
  `migrations.rs::migration_files_are_lf_only`, `no_unapproved_destructive_statement` (ADR-015 D5, D7).
- [ ] Open refuses a missing, pending, newer, mismatched or dirty store, untouched, with exit 30 and the
  R6 codes; open never migrates: `crates/pipeline/tests/engine_lifecycle.rs` (R3, R6).
- [ ] `grid migrate` creates, is idempotent, writes the pre-migration copy, refuses under contention and
  rejects extra arguments: `engine_lifecycle.rs` (`migrate_*`), `crates/grid-cli/tests/cli.rs` (ADR-015 D3).

**Schema rules**
- [ ] Enum `CHECK` lists admit exactly the domain values; the job and model-state triggers equal governance's
  tables over every pair; append-only tables reject UPDATE and DELETE:
  `crates/persistence/tests/schema_rules.rs`.

**Artifacts (§8.11; DR-C15)**
- [ ] SHA-256 known answers; a put lands at its content address with no temp file left; identical
  content is stored once; a producer error leaves no final file:
  `crates/persistence/tests/artifacts.rs`.
- [ ] A missing, truncated or altered registered file is `StateCorrupt`, never empty:
  `artifacts.rs::{missing,truncated,altered}_registered_file_is_state_corrupt` (KI-A8 generic).
- [ ] Garbage collection removes only old, unregistered, well-named files: `artifacts.rs::gc_*`.

**Ledger**
- [ ] A correction creates a new data version with its parent; validation fields are set once and
  never defaulted; quarantined rows round-trip as `QuarantinedRowRef`:
  `crates/persistence/tests/ledger.rs` (KI-A9, KI-NEW-W5).

**Pointers (§8.7.4, §8.8, §8.13)**
- [ ] Promotion moves the pointer and both states in one transaction; an illegal promotion changes
  nothing; rollback restores without copying or deleting and refuses a REJECTED target; an interrupted
  snapshot leaves earlier ones intact; readers answer during a write transaction:
  `crates/persistence/tests/pointers.rs` (KI-V10).

**Jobs (§8.10)**
- [ ] A SUCCEEDED key is not re-executed; a FAILED key re-enqueues as a new row; dependents wait for
  the latest row of each upstream key and stay blocked behind a failure; retries use the persisted
  backoff; exhaustion writes FAILED and one `JobFailed`; an oversized error message still reaches
  FAILED: `crates/pipeline/tests/job_runner.rs` (KI-G6).
- [ ] Stage outputs commit atomically with the job's success:
  `job_runner.rs::stage_outputs_commit_atomically_with_success` (KI-NEW-Z14).
- [ ] At most one publication job runs: `job_runner.rs::publication_jobs_never_run_concurrently`.
- [ ] Shutdown cancels at a durable boundary: `job_runner.rs::shutdown_*`.
- [ ] Lease-expired jobs recover per D9: `engine_lifecycle.rs::stale_*`.

**Locks (R5)**
- [ ] Every open takes the shared lock; `migrate` is exclusive; the update lock excludes a second
  updater and nothing else: `engine_lifecycle.rs::*lock*`.

**Events (§8.4; §12.6)**
- [ ] Every event reaches the table, `tracing` and the subscribers; none is published when its
  transaction rolls back; a subscriber observes committed state: `crates/pipeline/tests/event_delivery.rs`.

**Crash and restart (§8.11, §12.5)**
- [ ] A process killed at each P1-02 durable boundary recovers on rerun to the state of an
  uninterrupted run: `crates/pipeline/tests/crash_restart.rs`.

**Boundaries**
- [ ] `grid-governance` depends only on `grid-domain`; `grid-persistence` exactly on
  {`grid-domain`, `sqlx`, `sha2`}; only `grid-persistence` declares `sha2`; numerical crates reach
  none of them: `crates/pipeline/tests/crate_boundaries.rs`.

**Verification and records**
- [ ] `just verify` exits 0 on the final content commit (Linux smoke); the `windows-authoritative` CI
  job (`verify.ps1 -Scope Full`) is green on the PR.
- [ ] `.ai/evidence/P1-02/manifest.json` is generated by `just evidence P1-02`, and
  `./scripts/check-evidence-claims.sh origin/main P1-02` passes.

## Verification
```text
# targeted
cargo nextest run -p grid-domain -p grid-governance -p grid-persistence -p grid-pipeline -p grid-cli
cargo test -p grid-persistence --doc ; cargo test -p grid-pipeline --doc
sqlx migrate run && cargo sqlx prepare --check --workspace -- --lib
bash scripts/check-migrations.sh
# canonical
just verify
powershell -ExecutionPolicy Bypass -File scripts/verify.ps1 -Scope Full   # merge-authoritative, CI
# evidence (after the final content commit)
just evidence P1-02
./scripts/check-evidence-claims.sh origin/main P1-02
```
Verification recipes are a frozen contract. Make the repository satisfy them; never edit a recipe to
make a failure disappear.

## Numerical/performance tolerances
Not applicable: no numerical computation. Two exactness requirements apply:
- `config_hash` is deterministic (a golden vector);
- f64 values round-trip bit-exactly through the artifact store (−0.0, subnormals and NaN payloads).

## Migration and rollback
- **Forward path.** `0002_backbone.sql` is additive; `0001_schema_meta` is untouched.
  - An existing development database migrates with `sqlx migrate run`.
  - An engine data directory migrates with `grid migrate` (ADR-015).
  - The upgrade seeds (`seed_0001.sql`, `seed_0002.sql`) prove the forward path from every earlier
    version.
- **Rollback.** There are no down migrations (ADR-015 D8). Rollback is the previous binary plus the
  pre-migration copy that `grid migrate` writes, or a backup of `data/`.
- **On this branch.** `0002_backbone.sql` may still be amended while it is branch-new
  (`check-migrations.sh` allows it). The development database is then rebuilt as described under
  "Preconditions".

## Evidence required
- **Commands and outcomes:**
  - the targeted and canonical commands above, with exit status, against the final content commit;
  - the `windows-authoritative` CI result;
  - the reviewer reports;
  - the remediation record.
- **The manifest.** `.ai/evidence/P1-02/manifest.json` is generated by `just evidence P1-02`, never
  hand-written, with `verification-input.json` and `PR-BODY.md` beside it.
- **Model identity.** It is "unrecorded", as in P0-01, P0-02, P0-03 and P1-01: the implementing
  agent's harness policy forbids writing model identifiers into repository artifacts. This deviates
  from engine-spec §8.20.

## Stop/decision conditions
Stop and raise a decision request if:
- a criterion can be met only by growing a closed enum, by a MAJOR contract change, or by a
  destructive migration;
- a verified sqlx, tokio or std API behaves differently from what this file assumes, and the
  documented fallback would break R3 or R5. The cases are: `Migration` checksum visibility,
  `Pool::begin_with`, `query!` inference on `STRICT` tables with triggers, `File::try_lock_shared`;
- a guard or recipe fails in a way that only a recipe change would fix;
- implementing the job or pointer semantics requires deciding a behaviour that R1–R6 and the
  decisions above do not settle;
- a new dependency beyond `sha2` appears necessary.

## Follow-up
- **F-1. DAG wiring.** The owner assigns the package that turns the §8.6.2 DAG into job plans: job
  creation, per-edge `allow_skipped`, NO_NEW_DATA days, clean-up of blocked PENDING rows. Informative
  node → job mapping:

  | DAG step | Job |
  |---|---|
  | fetch + retain raw | `DATA_FETCH` |
  | normalize, validate, quarantine, commit data version | `NORMALIZE_DATA` |
  | resolve identities | `IDENTITY_RESOLVE` |
  | affected feature partitions | `BUILD_FEATURES` |
  | snapshot of pointers | `SNAPSHOT` |
  | V(s) → dV | `VALUE_MODEL_FIT` (boundary), `COMPUTE_DV` |
  | offseason path | `RAPM_UPDATE` |
  | weekly path | `LAYER1_CREDIT` |
  | Kalman → fixed-lag | `KALMAN_UPDATE`, `FIXED_LAG_SMOOTH` |
  | EB priors | `EMPIRICAL_BAYES_UPDATE`, `PRIOR_TRANSLATE` |
  | GRID-derived features | `BUILD_FEATURES` |
  | boosting | `BOOSTING_UPDATE` |
  | Layers A–F, scoring, candidate projections | `PREDICT` |
  | validate | `MODEL_VALIDATE` |
  | promote or reject | `MODEL_PROMOTE` |
  | not in the daily DAG | `RTS_SMOOTH`, `EVALUATE`, `ROLLBACK` |

- **F-2.** The JSON `tracing` formatter and §8.11 lifecycle step 1 go to P1-10 (it needs the
  `tracing-subscriber` `json` feature).
- **F-3.** P1-03 decides the raw-data revert semantics (A → B → A; D11) and the `validation_status`
  vocabulary.
- **F-4.** P1-11 owns orphan-GC scheduling, retention of `jobs`, `diagnostic_events` and the
  pre-migration copies, the kill-during-migration test, and the Windows directory-fsync gap (D16).
- **F-5.** A settings query and a settings command have no owning package (§8 "queryable (§8.3)").
- **F-6.** The `EngineCommands` / `EngineQueries` implementations land once every method has a stage
  (R4).
- **F-7.** P1-10 adds the `grid migrate` data-directory grammar (ADR-015 "Inherited").
