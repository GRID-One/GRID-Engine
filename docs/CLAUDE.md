# CLAUDE.md — GRID Engine: architecture, boundaries and protocol

Repository-wide module rules (engine-spec §8.15). Root rules are in `/CLAUDE.md`; the full map and
path aliases are in `docs/00-meta/authority-index.md`.

## Authority Order

Per `engine-spec.md` §1.5 (DR-A2). When requirements conflict, the following order controls:

1. the specification, `engine-spec.md`, and its byte-identical mirror at
   `docs/00-meta/specs/engine-spec.md`
2. accepted Architecture Decision Records in `docs/02-adr/`
3. versioned contracts and schemas, model specifications, and provider manifests in
   `docs/03-contracts/`, `docs/05-model-specs/`, and `docs/04-providers/`
4. the approved work-package file in `docs/01-work-packages/`
5. tests and fixtures that implement the approved contracts, including committed reference-oracle
   fixtures and golden files
6. existing source code, comments, and local conventions, including the `reference/python/` source

Existing code is not authoritative merely because it already exists. Tests are not authoritative
if they contradict a higher-level approved requirement. A conflict, or an absent decision that
materially changes behavior, stops the work package at a clean boundary with a decision request.

The reference oracle never overrides the specification. The superseded specifications,
`docs/07-archive/` and `docs/06-sessions/` are non-authoritative history.

## Architecture

- **Engine only** (ADR-011): Rust library API (commands, queries, events), a headless CLI that is a
  thin adapter over it, and versioned file and database outputs. No UI, FFI layer or installer.
- **Rust owns all authoritative state**: data, model parameters and versions, predictions and
  intervals, feature definitions, training and ingestion status, persisted settings.
- **SQLite is the durable source of truth.** Bulk artifacts (Parquet frames, serialized
  accumulators) are content-hashed and registered in a SQLite manifest (proposed — DR-C15).
- **Concurrency.** Tokio for async I/O and job coordination; Rayon or `spawn_blocking` for CPU-heavy
  work. Never block the async runtime.
- **Statistical interfaces are first-class**: ridge, RAPM-style sparse regularized effects, Kalman,
  RTS and fixed-lag smoothing, empirical Bayes, affine maps, gradient boosting (trait-abstracted),
  plus the GRID primitives of engine-spec §6.2.
- **Data and identity.** nflverse is primary; CFBD is the NCAA supplement. `gsis_id` is the
  canonical NFL player key. The three-season NFL window is strictly enforced. NCAA-to-NFL linking
  uses conservative tiers and never auto-promotes an ambiguous match.

## Crate Boundaries

Current workspace: 11 crates, each package named `grid-<crate>` (engine-spec §8.1). The boundary
text is quoted by engine-spec §8.1; the engine role is what each crate owns for GRID.

| Crate | Boundary | Engine role |
|---|---|---|
| `domain` | IDs, types, errors, DTOs. No external deps. | Canonical IDs, every version ID, the `AsOf` type, typed error codes, output-contract DTOs, plays/players/market/college contract types |
| `persistence` | SQLx, migrations, SQLite. No business logic. | Schema groups, artifact manifest, atomic artifact writes, production-pointer transaction |
| `ingestion` | nflverse, CFBD adapters. Raw data retention. | Provider adapters, hashed raw retention, normalization and quarantine, once-per-day fetch cap, offline replay, plays-contract builder |
| `identity` | Player registry, cross-source linking, review queue. | Canonical registry, NCAA linking tiers, identity review queue |
| `features` | Deterministic feature generation, schema versioning. | Point-in-time feature store, situation masks, GRID-derived features as of the projection timestamp |
| `models` | Ridge, RAPM, Kalman, EB, Affine, Boosting trait. | `ridge`, `rapm`, `value`, `credit`, `statespace` (incl. RTS and fixed-lag), `priors`, `empirical_bayes`, `affine`, `booster`, Layers A–E, `ensemble` |
| `simulation` | Correlated Monte Carlo, stat-to-points. | Layer F seeded draws with hard constraints; scoring applied through `scoring` |
| `scoring` | Fantasy profile transforms, versioned affine mapping. | Versioned Standard, Half-PPR, PPR and custom profiles over stat vectors and draw matrices |
| `evaluation` | Rolling-origin backtests, PB-MAE, benchmark comparison. | Walk-forward backtest, player pool, metrics, week-clustered bootstrap, leakage harness, scorecards, lineup simulation, synthetic recovery harness |
| `governance` | Model promotion, rollback, snapshot, job queue. | Model states and the promotion gate, threshold registry, production pointers and snapshot records, durable job queue. The oracle verdict is rendered, never acted on |
| `application` | Commands, queries, events, app services. Orchestrates crates. | The engine facade: library API (§8.2–§8.4) and orchestration of the daily pipeline DAG (§8.6). "App services" means application services behind the library API, not a user interface |

**Target crates (DR-A8, pending ratification).** P0-01 only drops `ffi`. Later packages change the
rest:

- `application` → `pipeline` (P1-01): library facade, DAG orchestration, durable-job execution.
- new `grid-cli` crate, binary `grid` (skeleton P1-01; subcommands, reports and export P1-10): argument
  parsing, output formatting, exit codes. It depends only on the orchestration crate and `domain`.
- new `synth` crate (P1-01 loader for the committed oracle synthetic fixtures; P1-12 Rust-native
  generator of the corrected synthetic world, validated by recovery gates, not by RNG-stream parity).
- `persistence` and SQLite stay.

**Rules.**

1. No crate depends on Python: no binding or embedding crate, no build script or Rust test that
   invokes an interpreter, no Python process at run time. Oracle outputs enter Rust only as committed
   fixtures under a sha256 manifest (`docs/03-contracts/parity-fixture-contract.md`).
2. Business logic stays out of the CLI, presentation and generated code.
3. Numerical crates (`models`, `simulation`, `scoring`, `synth`) SHOULD be synchronous, with no Tokio
   or SQLx dependency. Tokio and SQLx belong in `persistence`, `ingestion` and the orchestration and
   CLI crates.
4. If public DTOs need serialization derives, P1-01 decides by ADR where they live; `domain` keeps
   "no external deps" until then.
5. P1-12 owns `models::{value, rapm, credit, statespace, priors}` and the `synth` crate; P1-07 does
   not edit them. An overlap is a stop condition.
6. Not ported: `grid/cache.py`, `scoring/format_registry.py`,
   `pipeline/{data_pipeline,compute_valuations,sync_*,health_check}.py`. VOR exists only inside the
   evaluation lineup simulation (proposed — DR-C11).

## Reference Oracle (`reference/python/`)

Governed by ADR-012 and engine-spec §1.7. Operating rules and provenance: `reference/python/README.md`;
status, correction ledger, tolerances and divergences: `reference/python/PARITY.md`.

- **What it is.** The cautious-nevermore Python engine at `59bce1d`, package `backend.*`: 105 files,
  103 byte-identical and two patched (P1, P2 in `patches/`), each recorded in `MANIFEST.tsv`. Status
  `legacy-59bce1d`. Source sits at authority level 6; committed fixtures and goldens at level 5.
- **Never edited** under `backend/` or `tests/` except by an approved correction-ledger entry: a
  separate reviewed commit, failing test first, goldens regenerated with a model-spec note,
  `MANIFEST.tsv`, `patches/` and `PARITY.md` §(b) updated. A `requirements.lock` bump is an oracle
  change.
- **Defects are anti-targets.** No legacy output is frozen as a parity target for a quantity a ledger
  entry changes. Never port a `KI-…` defect; implement the spec.
- **Fixtures.** Exported single-threaded and committed under `fixtures/parity/` with a sha256
  manifest. Rust tests read files and never invoke Python. Fixture regeneration is never part of the
  change that ports the component it gates. Fixtures are synthetic-only (DR-A11).
- **Environment.** Linux only; `OMP_NUM_THREADS`, `OPENBLAS_NUM_THREADS`, `MKL_NUM_THREADS` = 1. The
  isolation guard blocks app imports and network access. `python3 tools/verify_manifest.py` must pass.
- **CI.** The `reference-oracle` job is outside the frozen verify chain, never wired into
  `verify.ps1`, `just verify` or `windows-authoritative`, and not merge-authoritative (DR-A3, DR-D30).
  If the runner's Python moves a golden, escalate; never loosen a tolerance.
- **Data.** Real third-party data is never committed. `backend/db/data/coaching_changes_2025.json` is
  unverified (KI-NEW-D1) and never becomes a fixture or provider input.
- **Spelling.** `_typos.toml` allowlists oracle identifiers entry by entry; never exclude the tree.

## Documentation Vault

- **Numbered paths only.** New documents use `docs/00-meta/` … `docs/07-archive/` and
  `docs/99-templates/`. Unnumbered spec names (`docs/adr/`, `docs/model-specs/`, …) resolve through
  the alias table in `docs/00-meta/authority-index.md`.
- **`docs/05-sessions/` is invalid.** 05 is model specs; sessions are `docs/06-sessions/`.
- **Verbatim trees are never edited:**
  - `docs/06-sessions/` records (imported records are byte-identical; the README indexes are
    maintained);
  - `docs/07-archive/cautious-nevermore/` copies (only `MANIFEST.md`, `HISTORY.md` and
    `real-data-results.md` there were written for the archive);
  - `docs/00-meta/specs/superseded/`.

  Corrections go in a current document, never in the copy. `.gitattributes` marks these trees, and
  `reference/python/`, `-text` so a Windows `core.autocrlf` checkout keeps their exact bytes.
- **`engine-spec.md` and `docs/00-meta/specs/engine-spec.md`** stay byte-identical
  (`scripts/check-authority-sync.sh`).
- **ADRs are immutable once Accepted.** Only front-matter `superseded-by` may change
  (`docs/02-adr/README.md`). An agent writes ADRs as Proposed.
- **Registers** (`decision-register.md`, `known-issues.md`, `lessons-learned.md`) track state. They
  bind only through the spec, an accepted ADR or a model spec.
- New ADRs, model specs, provider contracts, session logs and work packages start from
  `docs/99-templates/`.

## Work Package Protocol

Per engine-spec §8.17:

1. Load authority: `CLAUDE.md`, the authority index, the relevant `engine-spec.md` sections, the
   work package, and only the relevant contracts and specs.
2. Explore read-only.
3. Plan: files, contract impact, test plan, migration and rollback notes, assumptions.
4. Human gate for architecture, statistical semantics, the parity regime or correction ledger, data
   rights, security, or destructive persistence changes.
5. Implement on an isolated branch `wp/P<phase>-NN-description`. One writer per file.
6. Verify incrementally (`cargo test -p grid-<crate>`).
7. Run the canonical suite (`just verify`; `verify.ps1 -Scope Full` is merge-authoritative).
8. Fresh-context adversarial review.
9. Remediate and rerun.
10. Generate evidence (`just evidence <WP-ID>`) and prepare the PR (Appendix C, §12.8).
11. Human merge. The agent never merges.

**Stop and raise a decision request** on: a higher-authority conflict; undocumented provider
behaviour; destructive migration ambiguity; a missing numerical specification; licence uncertainty;
a requirement satisfiable only by weakening a test or safety control; an unexplained divergence from
the oracle; a dependency on an unratified owner decision; a requirement that would reproduce a known
oracle defect in Rust.

## Module-Level CLAUDE.md Policy

Per engine-spec §8.15, instruction files are layered and each stays small:

- Root `CLAUDE.md` — durable, non-obvious rules that apply to most work.
- This file (`docs/CLAUDE.md`) — repository-wide architecture, crate boundaries and protocol.
- `crates/<crate>/CLAUDE.md` — optional, for local commands, patterns and pitfalls only.
- `reference/python/README.md` — the oracle's operating rules, provenance and warnings.

Long domain tutorials, provider schemas and model equations belong in `docs/03-contracts/`,
`docs/04-providers/` and `docs/05-model-specs/` — never inlined here. A module file may narrow a
rule for its scope; it may never relax a rule set by a higher-authority document.

## Prohibited Shortcuts

The full list is engine-spec Appendix D (summarized in the root `CLAUDE.md`). Module-level
reminders:

- Inventing provider fields, crates, packages or APIs from memory.
- Adding unapproved production dependencies inside feature PRs.
- Regenerating golden files or oracle fixtures without semantic review and a ledger entry.
- Converting typed errors into silent defaults in critical paths; the oracle's silent-failure paths
  become typed failures in the engine (proposed — DR-B6).
- Merging, signing, publishing, promoting or deploying autonomously.
- Changing `rust-toolchain.toml` inside a feature PR.
- Rewriting migrations to hide incompatibility.
- Using `sudo` or `rm -rf /` in scripts.
