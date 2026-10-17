---
work-package-id: P1-01
status: In Progress
risk-class: High
owner: Product/Architecture owner
implementer: Claude Code (the implementing agent; model identity unrecorded, see Evidence required)
reviewer: Pending — a fresh-context reviewer who is not the implementer, then the owner's merge
phase: 1
---

# P1-01 — Domain types, engine API skeleton, oracle-fixture harness and crate restructure

## Status
**In Progress.** Started on 2026-10-17 on branch `claude/pensive-fermat-fgaweh`, from `main` at the
P0-02 merge (`73a8e3b`). That is the harness-assigned branch name, not `wp/P1-01-…` (CLAUDE.md,
"Branch, commit, PR"), the same deviation P0-01 and P0-02 record under "Known limitations". The
package was Ready when it started: every decision it depends on is settled (see "Owner decisions
depended on"), P0-01 and P0-02 are merged, and the contracts it implements exist as Drafts. It is Done
once every criterion below has passed on the final content commit, a fresh-context review is complete
and the owner has merged it with the approvals listed under "Ownership and risk" (engine-spec §8.16.2).
No criterion below is ticked; the runs on the final content commit are recorded in
`.ai/evidence/P1-01/`.

## Ownership and risk
- **Owner.** Product/Architecture owner. The repository owner holds every role today.
- **Implementer.** Claude Code (the implementing agent): one orchestrating session, with implementer
  agents on disjoint file sets and read-only reviewer agents (engine-spec §8.18).
- **Reviewer.** Pending: fresh-context adversarial reviewer agents who are not the implementer
  (contract fidelity, repository rules and guards, Rust quality, security and dependencies,
  reference parity), then the owner.
- **Risk class: High.** Every later Phase 1 package builds on what this one fixes: the domain IDs and
  the `AsOf` type that the leakage harness relies on, the typed error vocabulary, the output-contract
  type names and exit codes, the dependency boundary of `grid-domain` (ADR-014) and the crate set of
  DR-A8. An error here propagates to all of them. It contains no statistical computation and no
  persistence change.
- **Required human approvals.**
  1. **Product/Architecture owner.** Accept or reject ADR-014 (`grid-domain` takes `serde` as its only
     external dependency). Approve this package, which fixes the Rust type names of the plays and
     output contracts (plays-contract §11, engine-output-contract §10) and the exit-code table
     (engine-output-contract §13). The owner's merge is the approval.
  2. **Security/Release owner.** Review the agent definitions under `.claude/agents/` and the skill
     under `.claude/skills/`. They add no permission: `.claude/settings.json` is unchanged, and every
     reviewer agent is read-only by instruction and tool list.
  3. **Statistical owner.** None decided here. The `AsOf` type implements the semantics of
     `docs/05-model-specs/evaluation-and-leakage.md` §4.1 as written; the guards (G1 to G13) and any
     behavioural parity case are P1-05's.
  4. **Merge reviewer.** A merge commit, never a squash (DR-A6 precedent, ADR-011 D10).
- **Agent budget (engine-spec §8.18.1).** At most 400 agent turns in total across the implementing,
  reviewing and remediating agents; a wall-clock timeout of 10 hours for the implementing session;
  at most 4 concurrent writers, each on a disjoint file set, one writer per file; no cost ceiling
  (`ai-toolchain.lock` records `cost_ceiling_usd: null`). Exhaustion produces a partial evidence record
  and a Blocked package, never a skipped check.

## Authority
- Engine spec (`engine-spec.md`): §1.1, §1.5, §1.7, §2.2, §4.4.1, §4.5, §5.1–§5.5, §6.7, §6.8,
  §7.2, §7.12 (in particular §7.12.3–§7.12.5), §8.1–§8.4, §8.6.4, §8.9, §8.10, §8.14, §8.16–§8.18,
  §8.19, §8.20, §9.2.1, §9.5, §12.6, §14.1, §14.2, Appendices B, C and D.
- ADRs: ADR-001 (D5 frozen recipes), ADR-006 (item 1: agent definitions deferred to P1-01), ADR-010,
  ADR-011 (D1, D2, D11), ADR-012 (§2, §4), and ADR-014 (Proposed by this package).
- Contracts: `docs/03-contracts/plays-contract.md` (v1, §7, §9, §11–§13),
  `docs/03-contracts/engine-output-contract.md` (§1–§10), `docs/03-contracts/parity-fixture-contract.md`
  (§2–§6, §11, §12).
- Model specs: `docs/05-model-specs/evaluation-and-leakage.md` §4.1 (the `AsOf` information model) and
  §10.1; `docs/05-model-specs/synthetic-world.md` §5.3, §10.1 and §10.3 (S-1).
- Registers (tracking only): `docs/00-meta/decision-register.md`, `docs/00-meta/known-issues.md`
  (KI-A4, KI-A8, KI-P1, KI-V8, KI-NEW-C1, KI-NEW-Z38, KI-NEW-Z49, KI-NEW-Z61, KI-NEW-Z71).

## Owner decisions depended on
All settled; none is unratified. Statuses are those of `docs/00-meta/decision-register.md` on
2026-10-17.

| DR | Decision | Status | How this package uses it |
|---|---|---|---|
| DR-A2 | Authority order; oracle placement | Ratified 2026-10-08; named again 2026-10-15 (PR #4 comment 6185953773) | The oracle's `data_adapters.py` contract and `asof.py` are evidence at level 6, never authority over the contracts this package implements |
| DR-A8 | Crate set: `application` → `pipeline`, plus `grid-cli` and `synth`, in P1-01 | Ratified 2026-10-08; named again 2026-10-15 | The crate restructure below |
| DR-A11 | Parity fixtures synthetic-only | Ratified 2026-10-08; named again 2026-10-15 | The fixture reader refuses `provenance.data ≠ "synthetic"`; no real-data material enters any test input |
| DR-B1 | Oracle correction policy | Ratified 2026-10-08 (the policy; each ledger entry is approved as it lands) | The fixture manifest's `oracle.level` and `corrections_applied` are read and exposed; this package applies no ledger entry and targets no corrected output |
| DR-B2 | Committed fixtures and the `reference-oracle` job | Ratified 2026-10-08 | The Rust harness reads committed fixtures only and never invokes Python |
| DR-B3 | Parity tolerance table | Overridden 2026-10-09 (the merged table with the Class B, C-V, C-L1 and seed-ensemble amendments); still open: the Class D floor values (P1-12) | The tolerance helpers implement the exact, A, A′ and B criteria of parity-fixture-contract §6. Class C-V, C-L1 and D are stage-specific and are not implemented here |
| DR-C4 | Horizons: weekly primary; ROS and preseason derived; typed horizon field | Ratified 2026-10-08 | The `Horizon` enum of the output contract; no `week = 0` sentinel (KI-A4) |
| DR-D7 | Stat-vector asymmetry intended | Decided 2026-10-08 | The per-position stat-component sets of the output record |
| DR-D8 | Output open definitions as versioned settings recorded per output version | Decided in part 2026-10-08: the interim rule is the basis of this skeleton; the values are P1-08's and P2-07's | The fantasy distribution record carries the percentile-definition, threshold-set and starter-threshold version identifiers as fields, with no values fixed here |
| DR-D29 | Parity-fixture on-disk format | Ratified 2026-10-08 | The reader implements the manifest-plus-raw-little-endian format exactly |

Decisions this package **makes**, under the authority the spec gives it:
- the `grid-domain` dependency boundary for serializable DTOs, by ADR-014 (engine-spec §8.1 rule 5);
  Proposed here, accepted or rejected by the Product/Architecture owner at merge, following the
  ADR-011/012/013 precedent of implementing under a Proposed ADR that the owner rules on in the merge
  review;
- the Rust type names of plays-contract §11 and engine-output-contract §10 ("P1-01 fixes the names");
- the skeleton of the exit-code table (engine-spec §8.2 and §8.6.4: "skeleton in P1-01, completed in
  P1-10").

**The package is not Ready while any of them is unratified** (engine-spec §8.16.1). A proposed
default is not a ratification.

## Objective
The workspace has the DR-A8 crate set, and `grid-domain`, `grid-pipeline`, `grid-synth` and `grid-cli`
compile, lint clean and pass their tests under the frozen verify chain, giving every later Phase 1
package: the canonical IDs and version IDs, the `AsOf` type with its publication-lag axis, the typed
error and status vocabulary, the plays-contract v1 types with their core validators, the
output-contract record types with serialization round trips, the §8.2–§8.4 command, query and event
surface as types and traits, a reader and tolerance helpers for the DR-D29 fixture format, the
`SyntheticTruth` type and the `from_legacy_v0` loader, a structural test of the crate boundaries, and
the §8.18 agent definitions.

## Observable outcome
- `cargo run -p grid-cli -- --version` prints the engine version; `grid --help` lists the §8.2 and
  §8.3 subcommand surface and names the package that delivers each (P1-10 for all of them);
  `grid exit-codes` prints the terminal-state exit-code table of the output contract; an unknown or
  not-yet-available subcommand exits with the usage code 2 and a message on stderr. The CLI holds no
  business logic.
- Library consumers see the public types of `grid_domain`, the traits and request types of
  `grid_pipeline`, and the `grid_synth::fixture` reader. Nothing computes a projection: no §8.2 command
  or §8.3 query has an implementation, by design of this skeleton package, and no type claims otherwise.
- A maintainer sees 13 workspace crates, `.claude/agents/` and `.claude/skills/`, and the contracts'
  Rust-type sections fixed.

## Preconditions
- P0-01 merged (`b819ae0`, 2026-10-15) and P0-02 merged (`73a8e3b`, 2026-10-15). Both are.
- The contracts and model specs this package implements exist as Drafts under `docs/03-contracts/` and
  `docs/05-model-specs/`. They do.
- Every DR above settled. They are.
- Environment: the pinned dev tools of `toolchains/dev-tools.lock`, `DATABASE_URL` and `SQLX_OFFLINE`
  from the committed `.env`, a local SQLite development database from `just bootstrap`. No Python is
  needed: this package neither runs the oracle nor changes it.

## Scope

### Rust workspace
- `Cargo.toml`: workspace members become `domain`, `persistence`, `ingestion`, `identity`, `features`,
  `models`, `simulation`, `scoring`, `evaluation`, `governance`, `pipeline`, `grid-cli`, `synth`
  (13 crates). `[workspace.dependencies]` gains versioned path entries for the intra-workspace crates
  `grid-domain` and `grid-pipeline`, because `cargo deny check` (`wildcards = "deny"` in `deny.toml`)
  treats a version-less path dependency as a wildcard; the policy is not relaxed. No external crate
  enters the dependency set.
- `Cargo.lock`: regenerated by cargo for the renamed and added members only; no other package version
  moves (`cargo generate-lockfile` is never run).
- `crates/application/` is renamed to `crates/pipeline/` (package `grid-pipeline`; DR-A8). The
  orchestration crate holds the §8.2–§8.4 library-API skeleton: typed command requests and an
  `EngineCommands` trait (13 commands), typed queries, windowed response shapes and an `EngineQueries`
  trait (14 queries), the `EngineEvent` enumeration (21 events) carrying identifiers only, the
  `EventSubscriber` trait and an in-process fan-out, JSON record and sidecar serialization helpers
  that stamp the output-contract version, and two integration tests: output-record round trips and the
  structural crate-boundary test (below). First use of the already-declared workspace dependencies
  `serde` and `serde_json`.
- `crates/grid-cli/` (new; package `grid-cli`, binary `grid`): a thin adapter with no business logic.
  Hand-rolled argument handling on `std::env::args`; no argument-parser crate (one needs approval in
  the package that first requires it, engine-spec §14.2, which is P1-10). Depends only on
  `grid-pipeline` and `grid-domain` (engine-spec §8.1 rule 4).
- `crates/synth/` (new; package `grid-synth`): synchronous, no Tokio or SQLx. Modules:
  `fixture` (the DR-D29 reader: manifest via `serde_json`, raw little-endian arrays via `std`, typed
  and ragged views, byte-length, dtype and shape checks against the manifest, and the per-class
  tolerance helpers exact, A, A′ and B of parity-fixture-contract §6), `truth` (`SyntheticTruth`),
  `legacy_v0` (`from_legacy_v0`: a v0 columnar frame set to the v1 `PlaysFrame`, `PlayersFrame`,
  `CollegeFrame` and `SyntheticTruth`, and the v0 columnar view back, NaN and −1 sentinels included).
  First use of the already-declared workspace dependency `serde_json`.
- `crates/domain/` (package `grid-domain`): the types listed under "Implementation constraints",
  with `serde` derives (ADR-014). Hand-written `Display` and `std::error::Error` implementations; no
  other dependency.
- `scripts/bootstrap-repo.sh`: the crate list it scaffolds.

### Harness and agents
- `.claude/agents/`: the seven §8.18 role definitions (repository explorer, Rust implementer,
  data-contract reviewer, numerical reviewer, reference-parity reviewer, security/dependency reviewer,
  adversarial PR reviewer).
- `.claude/skills/`: the work-package execution skill (engine-spec §8.17).
- `.claude/settings.json` is **not** changed.

### Documents
- `docs/01-work-packages/p1-01-engine-skeleton.md` (this file) and `docs/01-work-packages/README.md`
  (index rows and status).
- `docs/02-adr/014-domain-serialization-boundary.md` (new, Proposed) and `docs/02-adr/README.md`
  (index row).
- `docs/03-contracts/engine-output-contract.md` (§10 type names fixed; §13 exit-code table added;
  version line), `docs/03-contracts/plays-contract.md` (§11 type names fixed; the serialization
  sentence follows ADR-014), `docs/03-contracts/parity-fixture-contract.md` (§11 reader location and
  dev-dependency rule), `docs/03-contracts/README.md` (index statuses).
- `docs/CLAUDE.md` (crate table and rules 3 to 5 after the restructure), `README.md` (status),
  `docs/00-meta/authority-index.md` (active work), `docs/00-meta/dashboard.md`,
  `docs/00-meta/daily-log.md`.
- `engine-spec.md` and its byte-identical mirror `docs/00-meta/specs/engine-spec.md`: factual status
  only, where this package makes a sentence false (§8.1 "Current workspace", §8.1 rule 5, the §8.14
  "Not yet present" rows, §9.2.1 item 4). No requirement changes.
- `docs/06-sessions/2026-10-17-P1-01.md` (new session log) and `docs/06-sessions/README.md` (one index
  row).
- `.ai/evidence/P1-01/`.
- `_typos.toml`, only if a new identifier needs an allowlist entry with a justification; never an
  exclusion.

### Contracts consumed
`grid.plays` v1 (Draft), `grid.output` 1 (Draft), `grid.parity-fixture` 1 (Draft).

### Contracts changed
- `grid.plays` v1: §11 names fixed; no field or rule changes. Still Draft; it becomes Approved when this
  package is approved (its §12).
- `grid.output` 1: §10 names fixed; §13 exit-code table added (an additive section). Version `1.0`.
- `grid.parity-fixture` 1: §11 reader location. No encoding or manifest change.

## Non-goals
- Any §8.2 command or §8.3 query implementation, any persistence (P1-02), ingestion (P1-03), identity
  (P1-04), feature store or leakage-guard behaviour (P1-05), numerical primitive (P1-06) or GRID
  component (P1-12).
- The oracle-side exporter under `reference/python/tools/parity/`, any committed parity fixture under
  `fixtures/parity/`, `INDEX.tsv`, the `-text` `.gitattributes` rule for that tree and the fixture-hash
  guard (`scripts/check-parity-fixtures.sh`). They arrive with the first fixture set (engine-spec
  §7.12.3, §8.14): the canonical synthetic worlds in P1-05, the first stage fixtures in P1-06 or P1-12.
- Any change under `reference/python/`, any correction-ledger entry, any golden or fixture regeneration.
- The CLI subcommands, reports and exports (P1-10); an argument-parser dependency; CSV export and
  formula-injection escaping (P1-10).
- The machine-readable schema artifact generated from the output types (engine-spec §5.5 rule 2):
  it accompanies the first file output, in P1-10.
- Any change to the frozen verify chain, `.claude/settings.json`, `.github/workflows/alpha-ci.yml`,
  `rust-toolchain.toml`, `migrations/` or `.sqlx/`.
- Any new workspace dependency. `serde` and `serde_json` are already declared in
  `[workspace.dependencies]` and already resolved in `Cargo.lock`.
- Deciding any open owner decision. Where a DR is open, the type carries the open item as a versioned
  identifier or a documented placeholder for the value, never a value.

## Inputs and fixtures
- Inputs: the contracts and model specs listed under Authority.
- Fixtures: none committed. The `grid-synth` tests build DR-D29 fixtures in temporary directories from
  hand-built synthetic toy frames, in the style of the oracle's own unit tests, and read them back.
  They derive from no third-party data (DR-A11). No oracle-exported fixture exists yet (engine-spec
  §7.12.3), and this package creates none.

## Oracle parity targets
No parity is asserted in this package. One target is prepared:

- **S-1, `synthetic-world`, `from_legacy_v0`** (synthetic-world.md §10.3; engine-spec §9.5 P1-01 row:
  "committed canonical synthetic fixtures load hash-identically, `data_adapters.py` contract
  invariants"). Oracle counterpart: `reference/python/backend/grid/data_adapters.py` (`load_synthetic`,
  the `plays`, `players`, `gt`, `college` contract) and `synth.py:162-317`. Class: exact. Fixture IDs
  and manifest hash: none yet. The committed `synthetic-world/canonical-legacy` case is created by P1-05
  (engine-spec §7.12.3, §8.1, §9.2.1 item 7). This package delivers the loader and a format-level
  conformance test (every column, id order, NaN position and v0 sentinel round-trips on a hand-built
  frame); P1-05 adds the one-line assertion over the committed case. Known oracle defects on this
  target: KI-NEW-Z38 (duplicate participation ids, kept as a multiset by the loader), KI-NEW-Z39
  (`yardline_100` up to 104; the legacy profile is exempt from the real-data rules), KI-NEW-Y0
  (defenders from the offense; the legacy world is loaded as-is and labelled `synthetic-legacy`),
  KI-NEW-Z49 (dtype drift; the v1 canonical types resolve it). Correction-ledger status: none applied;
  entry 1 (KI-NEW-Y0) is proposed and not applied.

The §9.5 parity column and §7.12.3 read differently on when the committed canonical fixture exists.
This package follows §7.12.3, §8.1 and §9.2.1 item 7 (loader now, fixture in P1-05) and records the
reading here and in the PR for the owner (see "Stop/decision conditions").

## Implementation constraints
- **No Python anywhere** in a crate, build script, test or run path (engine-spec §1.1 item 8, §8.1
  rule 1). Removing `reference/python/` must leave `cargo build` and `cargo test` unaffected.
- **`grid-domain`** (ADR-014): exactly one external dependency, `serde` with the `derive` feature, from
  `[workspace.dependencies]`. No `serde_json`, `thiserror`, `chrono` or `uuid`. Required content:
  - IDs (engine-spec §4.4.1, §8.9): `Season`, `Week` (never 0; KI-A4), `Slot(season, week)` with
    lexicographic order, `PlayerKey { Gsis(String), Synthetic(u32) }` ordered by its string form
    (`layers.py:252`), `TeamKey { Nfl(String), Synthetic(u16) }` in native order (`layers.py:255`),
    `GameId`, `PlayKey`, `DriveKey`, and the version identifiers of §8.9: data version, feature-schema
    version, model version, prediction version, scoring-profile id and version, ingestion run, job,
    training run, snapshot, `EngineVersion` (crate version plus git SHA), `ContractVersion`.
  - Time (§4.5): `UtcInstant` as whole seconds since the Unix epoch, UTC, ordered.
  - `AsOf` (§4.5; evaluation-and-leakage.md §4.1): `AsOf { season, week, lock: Lock { kind:
    Thursday | Sunday | Operational, at: UtcInstant }, mode: Backtest | Production | Research }` with
    `outcome_usable(slot, published_at)` (`slot < (S, W)` lexicographically and `published_at <
    lock.at`), `pregame_usable` (`slot ≤ (S, W)` and `published_at < lock.at`), a watermark check
    returning a typed `LeakageError`, and `research_only` carried on anything a Research-mode run
    produces. The week-only fallback of the oracle is not implemented: a record without a season is a
    `ContractError`.
  - Typed errors (§6.8 rule 4, §8.10, plays-contract §11): `EngineErrorCode` (closed), `EngineError
    { code, message }`, `ContractError` with the sixteen variants of plays-contract §11, `LeakageError`,
    `OutputInvariantError` for the §3.9 invariants of the output contract. Every error implements
    `Display` and `std::error::Error`.
  - Status (§8.4, §8.6.4, §8.10): `TerminalState` (14 values) with the exit-code mapping of
    engine-output-contract §13 for the twelve terminal values and none for `NotRun` and `Running`;
    `TrainingStatus` exactly as §8.4 writes it; `StageId` with the twenty §8.10 job types; `RunOutcome`;
    `JobStatus` (6 values).
  - Plays contract v1 (plays-contract §9, §11): `ContractVersion`, `SeasonType`, `PlayState`,
    `DrivePoints` (0, 3, 7; DR-C13), `NextState`, `GameClock`, `Participation { Listed, Unmatched,
    NotAvailable }` (order kept, multiset allowed under the legacy profile), `PlayRecord`,
    `PlaysFrame`, `PlaysFrameMeta` (producer, sources, participation availability, coverage),
    `PlayersFrame`, `MarketQuote` (with `quoted_at` and `source`; no 0.0 default), `CollegeFrame`, and
    validation of the §7 core rules 1 to 7 plus real-data rules 1 and 2 under a `RealData` profile
    (rules 3 and 4 need the as-of roster and the coverage policy: P1-03 and P1-05). Validation fails
    with a typed `ContractError`; nothing is coerced.
  - Output contract (engine-output-contract §2–§8): `Horizon`, `Position` (QB, RB, WR, TE), the
    lineage group, `PlayerWeekProjection`, `AvailabilityBlock`, the per-position stat component set
    (§3.2, DR-D7), `StatDistribution` (`mean`, `sd`, `q10`, `q25`, `q50`, `q75`, `q90`),
    `FantasyDistribution` with the DR-D8 items as versioned setting identifiers, `FreshnessFlags`,
    `ExplanationRecord`, `GridSignalBlock` (filtered and one-step predictive only; no smoothed field),
    `TeamGameEnvironment`, `ProjectionChange` with exactly the five categories, the report record
    skeletons of §7, and the §3.9 invariant checks. Every field's doc comment states units,
    nullability, enum semantics and whether the enum is open or closed. Missing is `Option::None` with
    a declared reason, never 0 (KI-P1).
  - Doctests: at least one on `AsOf` and one on `PlayerKey` ordering, so `test-doc` runs something.
- **`grid-synth`** reads the fixture format of parity-fixture-contract §2–§4 exactly: manifest keys as
  listed, `sha256` fields carried but not verified in Rust (DR-D29 puts hashing in a shell guard),
  byte length `= product(shape) × itemsize`, dtypes `f64le`, `i64le`, `i32le`, `u8`, `.json` strings,
  ragged `offsets + values` with non-decreasing offsets starting at 0, NaN preserved bit-for-bit,
  `provenance.data == "synthetic"` required. The tolerance helpers report the first failing index and
  both values; integer, boolean, id and categorical comparisons are exact; NaN positions must coincide.
- **Numerical crates stay synchronous** (`models`, `simulation`, `scoring`, `synth`): no Tokio or SQLx.
  `grid-models` never depends on `grid-synth` as a normal or build dependency; a numerical crate may use
  `grid-synth` only as a dev-dependency for its parity tests (parity-fixture-contract §11).
- **Structural test** (`crates/pipeline/tests/`): via `cargo metadata --offline --locked`, it asserts
  the 13 expected members named `grid-*`; `grid-domain`'s only external dependency is `serde`;
  `grid-cli` depends only on `grid-pipeline` and `grid-domain`; no numerical crate has a normal
  dependency on `tokio` or `sqlx`; `grid-models` has no normal or build dependency on `grid-synth`; no
  package in the resolved graph is a Python binding or embedding crate.
- **No business logic in the CLI** (§8.1 rule 2). The binary parses arguments, prints contract data and
  maps outcomes to exit codes.
- **Determinism and no placeholders.** No `TODO`, `FIXME`, `unimplemented!()` or `todo!()` in scope. A
  command or query that has no implementation is a trait method with no implementor, not a function
  that returns a fake result.
- **Frozen recipes.** No recipe, lint level, guard or test is weakened. `cargo clippy --all-targets
  --all-features -- -D warnings` must pass with no `allow` attribute added without a justification
  comment.
- **Line endings.** New files are LF. No verbatim tree is touched.
- **Model identity** is not written into any repository artifact (`ai-toolchain.lock` stays
  "unrecorded"; see Evidence required).

## Security and licensing considerations
- No secret, host, provider or data change. No network access at build or test time beyond cargo's
  registry during `cargo install` and dependency resolution, as today.
- No new production or development dependency: `serde` (MIT OR Apache-2.0) and `serde_json` (MIT OR
  Apache-2.0) are already in `[workspace.dependencies]` and in `Cargo.lock`; this package only adds
  them to member manifests. `cargo deny check` and `cargo audit` keep gating them.
- `.claude/agents/` and `.claude/skills/` grant no permission: the committed `.claude/settings.json`
  stays the permission boundary (§14.1). Reviewer agents are instructed never to edit and are given no
  editing tool. Security/Release owner review is requested (approval 2).
- No real-data-derived or provider-derived byte enters any test input (DR-A11).

## Acceptance criteria
Each criterion is verified on the final content commit.

- [ ] `just verify` exits 0 (all 8 recipes, 15 steps). `cargo nextest run --workspace` reports the
      new tests of `grid-domain`, `grid-synth`, `grid-pipeline` and `grid-cli` passing alongside the 3
      existing `grid-persistence` tests, and `cargo test --workspace --doc` runs at least two doctests.
- [ ] `env -u DATABASE_URL SQLX_OFFLINE=true cargo check --workspace` exits 0 (a fresh clone compiles
      offline).
- [ ] `Cargo.toml` lists exactly 13 members, each package named `grid-<crate>`; `crates/application/`
      no longer exists; `cargo metadata` shows no package version changed in `Cargo.lock` other than the
      renamed `grid-application` → `grid-pipeline` and the added `grid-cli` and `grid-synth`.
- [ ] `grid-domain`'s manifest declares `serde` and nothing else under `[dependencies]`, and the
      structural test in `crates/pipeline/tests/` passes every boundary rule listed under
      "Implementation constraints".
- [ ] The `grid-domain` tests cover: `Slot` lexicographic order across a season seam; `PlayerKey`
      string-form order (`"0" < "1" < "10" < "100"`); `TeamKey` native order; `Week::new(0)` rejected;
      `AsOf` usability on a record published after the lock (rejected), a same-week pre-game record
      (pregame yes, outcome no), a previous-season high-week record (both yes) and a watermark at
      `(S, W)` (typed `LeakageError`); every `TerminalState` exit code distinct and equal to the
      contract table; `TrainingStatus` shape; each plays-contract core rule with one passing and one
      failing frame; the real-data rules 1 and 2 enforced under `RealData` and exempt under
      `SyntheticLegacy`; every §3.9 output invariant rejected with the typed error; one round trip per
      output record type through `serde_json` (in `grid-pipeline`'s tests, where `serde_json` lives).
- [ ] The `grid-synth` tests cover: reading each dtype and the ragged encoding; a byte-length, dtype or
      shape mismatch and a missing required key rejected with a typed error; `provenance.data` other than
      `synthetic` rejected; NaN preserved bit-for-bit; the exact, A, A′ and B helpers passing and
      failing on constructed inputs with the failing index reported; `from_legacy_v0` round-tripping a
      hand-built v0 frame with NaN `terminal_value`, −1 `n_*` sentinels, a duplicated participant and
      `yardline_100 = 104`, then rebuilding the identical v0 columns.
- [ ] `cargo run -p grid-cli -- --version` exits 0; `--help` exits 0 and lists every §8.2 command and
      §8.3 query spelling; `exit-codes` prints the contract table; an unknown subcommand exits 2; the
      CLI integration test asserts all four.
- [ ] `.claude/agents/` holds the seven role definitions and `.claude/skills/` the work-package skill;
      `git diff --quiet main -- .claude/settings.json` exits 0.
- [ ] `./scripts/check-authority-sync.sh` passes; `./scripts/check-traceability.sh origin/main` passes
      with every non-trivial path traced to this file or ADR-014; `tests/guards/run.sh` passes with no
      case changed; `typos` is clean; `./scripts/check-evidence-claims.sh origin/main P1-01` passes.
- [ ] `git diff --quiet main -- reference/python .ai/evidence/P0-01 .ai/evidence/P0-02
      .ai/evidence/P1-00 migrations .sqlx justfile scripts/verify.sh scripts/verify.ps1
      .claude/settings.json .github` exits 0.
- [ ] `git grep -n -E 'TODO|FIXME|unimplemented!|todo!' -- crates` prints nothing.
- [ ] A fresh-context adversarial review is complete and every finding is dispositioned in
      `.ai/evidence/P1-01/`.

## Verification
```text
# targeted
cargo test -p grid-domain
cargo test -p grid-synth
cargo test -p grid-pipeline
cargo test -p grid-cli
cargo run -p grid-cli -- --version
cargo run -p grid-cli -- --help
cargo run -p grid-cli -- exit-codes
env -u DATABASE_URL SQLX_OFFLINE=true cargo check --workspace
./scripts/check-authority-sync.sh
./scripts/check-traceability.sh origin/main
bash tests/guards/run.sh
typos
git grep -n -E 'TODO|FIXME|unimplemented!|todo!' -- crates

# canonical
just verify
powershell -ExecutionPolicy Bypass -File scripts/verify.ps1 -Scope Full   # merge-authoritative (DR-A3), run by CI

# evidence (after the final content commit)
just evidence P1-01
./scripts/check-evidence-claims.sh origin/main P1-01
```
Verification recipes are a frozen contract. Make the repository satisfy them; never edit a recipe to
make a failure disappear.

## Numerical/performance tolerances
No numerical computation is performed. The tolerance helpers encode the pre-registered classes of
parity-fixture-contract §6 (exact; A ≤ 1e-12 absolute; A′ ≤ 1e-9 relative in the ∞-norm; B as the
three-part criterion with `tol_cg` read from the case) and are themselves tested on constructed
inputs. No threshold is set or changed here.

## Migration and rollback
No migrations; `migrations/` and `.sqlx/` are untouched. Rollback is a revert of this package's merge
commit: it removes three crates' content and restores `crates/application/`, and no persisted state
depends on anything added here.

## Evidence required
`.ai/evidence/P1-01/verification-input.json` (the recorded runs), `manifest.json` generated by
`just evidence P1-01` (never hand-written) and `PR-BODY.md` (Appendix C record). Test output of the
targeted and canonical commands; the structural test's output listing the boundary rules it checked;
the adversarial review findings and their dispositions. `ai-toolchain.lock` provenance stays
"unrecorded", as in P0-01 and P0-02: the implementing agent's harness policy forbids writing model
identifiers into repository artifacts, which deviates from engine-spec §8.20.

## Stop/decision conditions
Stop and raise a decision request rather than decide if:
- a contract or model spec needs a field, rule or value that no authority states (for example a
  stat-component unit, an exit code's semantics beyond §8.6.4, or an `AsOf` rule not in
  evaluation-and-leakage.md §4.1);
- `grid-domain` would need a second external dependency, or ADR-014 is rejected (then serialization
  moves to `grid-pipeline` as mirror types, which contradicts engine-output-contract §1 rule 1 and
  needs the owner's ruling);
- a frozen recipe cannot pass without a recipe, lint or guard change;
- any requirement would reproduce a known oracle defect (KI-A4 sentinel, KI-P1 zero-for-missing,
  KI-NEW-Z61 week-only slicing, KI-NEW-Z71 static market) in Rust;
- the fixture-timing reading above is not the owner's: this package treats the committed canonical
  synthetic fixture as P1-05's (§7.12.3, §8.1, §9.2.1 item 7) and the §9.5 parity column as the gate
  P1-05 closes. If the owner wants the canonical legacy world exported and committed in P1-01, that
  requires the oracle exporter (`reference/python/tools/parity/`), which §7.12.3 and §8.14 assign to the
  first port package, and is a scope change for a new package or a recorded owner decision, not an
  amendment written by the implementer (ADR-001, "On amending a work package mid-implementation").

## Follow-up
- **P1-05** commits `synthetic-world/canonical-legacy` and adds the S-1 assertion over it, the
  `-text` rule for `fixtures/parity/` and, with the Security/Release owner, the fixture-hash guard in
  the `guards` job (parity-fixture-contract §10). Risk until then: the loader is proven only on
  hand-built frames.
- **P1-06 and P1-12** create the oracle exporter and the first stage fixtures; their parity tests use
  `grid-synth::fixture` as a dev-dependency.
- **P1-10** completes the CLI (subcommands, table/JSON/CSV output, exports) and may request an
  argument-parser dependency; it also generates the machine-readable output schema artifact.
- **P1-02** fixes the full job-type enumeration; `StageId` here carries the twenty §8.10 names and
  P1-02 may extend it by a MINOR contract change.
- **Owner actions carried, unchanged by this package:** DR-D30 (make `reference-oracle` a required
  check), DR-A3's follow-up ADR before P1-11, the KI-NEW-Z79 decision request, the P1-12
  pre-registrations.
- **Spec wording:** the §9.5 P1-01 parity column and §7.12.3 differ on when the canonical fixture is
  committed; the owner may want one sentence aligned (a spec erratum, not a requirement change).

## Amendments (2026-10-17, for the owner's ruling at the merge)

The sections above are the package as committed in `e3b2b75`, with one factual correction kept in
place (Preconditions: the P0-02 merge date is 2026-10-15, per `git show 73a8e3b` and PR #5, not
2026-10-17). During implementation the fresh-context reviews found the items below. Each was first
written into the sections above; the second review moved them here, because ADR-001 ("On amending a
work package mid-implementation") requires an amendment to be a separate dated section rather than an
edit to the original text, and says that from P1-01 onward a scope change produces a decision request
the owner records, or a new package, not an amendment the implementer writes. Every row is therefore
proposed under the authority order (engine-spec §1.5), is not approved until the owner rules at the
merge, and is a decision request in the PR body. Where a row names a document, the document's
current text already carries the amended wording (the contracts stay Draft; the owner's approval of
this package is what makes them Approved).

| # | Where | Original (`e3b2b75`) | Amended (working tree) | Authority invoked | Status |
|---|---|---|---|---|---|
| 1 | Scope "Documents" and "Contracts changed": `grid.output` §4.2 | `regime_flags` not mentioned; "§10 names fixed; §13 exit-code table added (an additive section). Version `1.0`." | §4.2 `regime_flags` gains `observation_noise_inflation`, the fourth flag of engine-spec §5.5, and the row says so; `RegimeFlags` carries the four | engine-spec §5.5 names four flags and outranks the draft (§1.5); a MINOR addition on a Draft (§9) | Proposed; owner approval at the merge |
| 2 | Same: `grid.output` §3.9 | not mentioned | §3.9 gains the sd ≥ 0 bullet that `OutputInvariant::NegativeSd` checks | engine-spec §5 "Invariants" | Proposed; owner approval at the merge |
| 3 | Same: `grid.output` §2 `lock` | the §2 cell wrote the lock timestamp as `lock_at`; §10 recorded that the Rust field is `Lock::at` | the §2 cell writes `at`, the serialized key of `Lock { kind, at }`; §10 no longer records a mismatch | "P1-01 fixes the names" (§10), whose first paragraph makes the serialized field names the contract's field names; a one-word rename on a Draft with no producer (§9 counts a rename as MAJOR once a producer exists) | Proposed; owner approval at the merge |
| 4 | Scope "Documents" and "Contracts changed": `grid.parity-fixture` | "§11 reader location and dev-dependency rule" / "§11 reader location. No encoding or manifest change." | §11 reader location; new §5.1 fixing the `synthetic-world` array names the P1-05 exporter writes and the `legacy_v0::load_world` reader expects; no encoding or manifest-key change; the dev-dependency sentence withdrawn (row 5) | parity-fixture-contract §4 and §5: array names are contract data, not code, so the loader's names are fixed where a reader can find them | Proposed; owner approval at the merge |
| 5 | Implementation constraints "Numerical crates stay synchronous"; Follow-up "P1-06 and P1-12" | "`grid-models` never depends on `grid-synth` as a normal or build dependency; a numerical crate may use `grid-synth` only as a dev-dependency for its parity tests (parity-fixture-contract §11)"; "their parity tests use `grid-synth::fixture` as a dev-dependency" | the rule stated as the spec writes it: `models` MUST NOT depend on `synth` (engine-spec §8.1 rule 3; `synthetic-world.md` §10.1), unqualified; the structural test checks normal and build dependencies. Whether a numerical crate's parity tests may take `grid-synth` as a dev-dependency is stated by no document above this package | engine-spec §8.1 rule 3 outranks this file; the qualifier first appeared here and was repeated by `docs/CLAUDE.md` rule 3 and parity-fixture-contract §11 as if those documents stated it (both now state the rule as the spec does) | Withdrawn to an **open owner item**: a spec erratum or an ADR before the first parity test needs it (P1-06 or P1-12); the structural test's wording then follows the ruling |
| 6 | Scope "Documents", the `engine-spec.md` bullet | "§8.1 'Current workspace', §8.1 rule 5, the §8.14 'Not yet present' rows, §9.2.1 item 4" | the same places, enumerated after the review: the §8.1 "Current workspace" paragraph and crate table (`pipeline`, `grid-cli` and `synth` rows; the `domain` boundary cell, which quotes `docs/CLAUDE.md` and is marked "ADR-014, Proposed in P1-01"), §8.1 rule 3's "when it exists", §8.1 rule 5, the §8.14 tree comment lines and the two "Not yet present" rows moved below that table as present, §9.2.1 item 4 | the bullet's own allowance: factual status only, where this package makes a sentence false; no requirement changes | Recorded for the owner; each edit is a status sentence, approved with the package |
| 7 | Scope "Rust workspace", the `crates/pipeline/` bullet | "First use of the already-declared workspace dependencies `serde` and `serde_json`" | the same, with `serde_json`'s `float_roundtrip` feature enabled in `crates/pipeline/Cargo.toml`, so the JSON reader returns the exact bits of a parsed `f64` (engine-spec §12.6 "precision-preserving serialization round trips"; engine-output-contract §11); a feature of an already-resolved package, so `Cargo.lock` does not change and no dependency is added | engine-spec §12.6 outranks the bullet; the manifest comment and `tests/output_round_trip.rs` pin it | Proposed; owner approval at the merge |
| 8 | Follow-up, the "P1-02" bullet | "`StageId` here carries the twenty §8.10 names and P1-02 may extend it by a MINOR contract change" | `StageId` carries the twenty §8.10 names; P1-02 "fixes the full enumeration" (engine-spec §8.10) while `grid.output` is Draft; once the contract is Approved, a new variant of the closed `StageId` (carried by `TrainingStatus` in the §7 status report) is a MAJOR change (engine-output-contract §9) | engine-spec §8.10; engine-output-contract §9 ("adding a value to a closed enum" is MAJOR) | Recorded; a wording correction |
| 9 | Scope "Contracts changed", `grid.parity-fixture` ("No encoding or manifest change") and the corresponding "Documents" bullet | "§11 reader location. No encoding or manifest change." | §11 reader location; §5.1 (the `synthetic-world` array names, row 4) also defines one manifest key for that component, `provenance.synth_config_sha256`, the sha256 of the canonical JSON serialization of `params.synth_config` (the `SynthConfig` the exporter used), which `legacy_v0::load_world` carries into the producer's `config_hash`; every other component's manifest is unchanged, and no encoding changes | plays-contract §9 requires `FrameProducer::Synthetic::config_hash`; parity-fixture-contract §4 records provenance per case; P1-01 owns the fixture format (engine-spec §9.2.1 item 7). Without the key, no exported world could be upgraded (the first implementation refused with `MissingField` and raised a decision request); the key resolves it inside the format this package fixes | Proposed; owner approval at the merge; the P1-05 exporter writes the key |
| 10 | Observable outcome; acceptance criterion on `--help` ("names the package that delivers each (P1-10 for all of them)") | as written | as written: the usage names P1-10 for every §8.2 and §8.3 spelling (engine-spec §8.1 target table: `grid-cli` "P1-10 (subcommands, reports, export)"; §8.2 "completed in P1-10") and carries one footer line that the content behind Phase 2 outputs arrives with its owning package (§10.2.6). The implementer's interim per-entry Phase 2 labels (P2-01, P2-02, P2-06, P2-07 on five entries) were reverted by the orchestrator | engine-spec §8.1 and §8.2 assign the CLI surface; §10.2.6 assigns content, not the adapter | Resolved in this package; no owner action |
| 11 | Ownership and risk, approval 2; Security and licensing considerations, the `.claude/` bullet | "every reviewer agent is read-only by instruction and tool list"; "Reviewer agents are instructed never to edit and are given no editing tool" | reviewer agents are read-only **by instruction**: their tool lists carry no Edit or Write tool, but they keep Bash, and `.claude/settings.json` auto-allows some writing commands (`git add`, `git commit`, `cargo fmt`, `cargo sqlx prepare`, `sqlx migrate run`, `just bootstrap`) for every agent, so read-only is an instruction, not containment. The agent files now say so, and `rust-implementer` declares its tool list (no web or GitHub tool) and a no-network rule | engine-spec §8.18 ("Reviewer agents are read-only"), §14.1 (`.claude/settings.json` is the permission profile; changing it needs Security/Release owner approval). Found by the security review | **Decision request to the Security/Release owner:** accept the residual risk, or remove Bash from the reviewer definitions, or add deny/ask rules for writing commands in `.claude/settings.json` (a §14.1 change this package does not make) |
| 12 | Security and licensing considerations, the DR-A11 bullet | "No real-data-derived or provider-derived byte enters any test input (DR-A11)" | as written, now true: the test inputs used real nflverse identifiers (two GSIS ids and two 2023 game ids); they are replaced by fabricated, format-valid ones (`00-99000NN`, season `2099` game ids). Doc-comment examples of the id format keep the contract's own example (plays-contract §4) | DR-A11; parity-fixture-contract §8. Found by the security review | Resolved in this package; no owner action |
