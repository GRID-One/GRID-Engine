---
work-package-id: P0-01
status: Review
risk-class: High
owner: Product/Architecture owner
implementer: Claude Code (the implementing agent; model identity unrecorded, see Evidence required)
reviewer: Pending — a fresh-context reviewer who is not the implementer
phase: 0
---

# P0-01 — Engine-only consolidation

## Status
**Review.** The content is written on branch `claude/grid-engine-consolidation-e7kmh9`. P0-01 is
**not Done** until three things have happened:

- every acceptance criterion below has passed on the final commit, and the evidence has been generated
  from that commit;
- a fresh-context review is complete;
- the owner has merged with the ratifications listed under "Required human approvals".

No criterion is claimed as passing here. Each is to be run against the final commit (`CLAUDE.md`,
"Before claiming done").

## Ownership and risk
- **Owner.** Product/Architecture owner. The repository owner holds every role today.
- **Implementer.** Claude Code (the implementing agent), run as a multi-agent consolidation workflow.
- **Reviewer.** Pending. It must be a fresh-context reviewer who is not the implementer (`CLAUDE.md`).
- **Risk class: High.** The package:
  - replaces authority 1;
  - makes the first D5 amendment that removes a step (ADR-011 D6);
  - imports 105 files whose licence ruling is still pending (DR-A10);
  - lands as one pull request of about 260 non-trivial paths.
- **Required human approvals:**
  1. **Product/Architecture owner.** In one merge review or comment that names each DR ID, ratify or
     override DR-A1, DR-A2, DR-A4, DR-A5, DR-A6, DR-A8, DR-A9 and DR-A12, and accept or override ADR-011
     and ADR-012 (`docs/00-meta/decision-register.md`, "How to ratify or override", step 3).
  2. **Statistical owner.** DR-A2, the oracle placement, jointly with item 1.
  3. **Security/Release owner.** DR-A7 and the guard changes (ADR-011 D7). Also the design of the
     `reference-oracle` job: the runner's preinstalled `python3`, and no `actions/setup-python`
     (ADR-012 §4).
  4. **Data/Licensing owner.** The DR-A10 licence ruling, **recorded on the pull request before merge**
     in the form of GRID-Engine PR #1 comment 5357318508. Also DR-A11.
  5. **Merge reviewer.** A merge commit, never a squash (DR-A6, ADR-011 D10).
- **Agent budget (engine-spec §8.18.1).** Not declared in advance. §8.18.1 is introduced by this package.
  This is recorded under Known limitations.

## Authority
- **Engine spec (`engine-spec.md`):** §0; §1.1, §1.3–§1.7; §2; §3.3; §8.1; §8.14–§8.20; §9.2.1; §9.5;
  §10.5; Appendices B, D, F and H.
- **Superseded authority in force while this package was executed:**
  - `alpha-spec.md` §1.5 (a conflict produces a decision request), §8.8 (work-package contract), §8.11
    (verification) and §8.12 (evidence);
  - `final-build-spec.md` §1, the scope being superseded.

  Both are now at `docs/00-meta/specs/superseded/`.
- **Owner instruction.** The owner's request of 2026-10-01 and the owner's choice of option (ADR-011,
  Context).
- **ADRs.** ADR-011 and ADR-012 (both proposed); ADR-001 D5 and ADR-005 (the amendment conditions);
  ADR-009; ADR-010.
- **Contracts, model specs and provider manifests:** the drafts this package creates (Scope).

## Owner decisions depended on

**Adopted by this package, pending owner ratification at merge.** This package is gated on its own
A-decisions being ratified at its merge (engine-spec §9.5), not on Ready.

| DR | Decision | Owner role | Status | Recorded in |
|---|---|---|---|---|
| DR-A1 | One `engine-spec.md` plus a mirror; originals archived verbatim | Product/Architecture | Adopted — pending ratification at merge | ADR-011 D3 |
| DR-A2 | Six-level authority order; oracle fixtures at level 5, oracle source at level 6 | Product/Architecture, Statistical | Adopted — pending ratification at merge | ADR-011 D4, ADR-012 §2 |
| DR-A3 | Authoritative CI platform | Product/Architecture, Security/Release | **Default kept (Windows authoritative); follow-up ADR required** before P1-11 | ADR-011 D8 |
| DR-A4 | Keep the AI-governance apparatus | Product/Architecture | Adopted — pending ratification at merge | ADR-011 D5 |
| DR-A5 | `P<phase>-NN` IDs; P1-10 and P1-11 re-scoped; new P1-12 and P2-09; traceability regex | Product/Architecture | Adopted — pending ratification at merge | ADR-011 D7, D9 |
| DR-A6 | PR #1 carried as a fast-forward and merged with a merge commit; PRs #2 and #3 closed | Product/Architecture | Adopted (adapted) — pending ratification at merge | ADR-011 D10 |
| DR-A7 | Fix R4-1 and R4-2 in this PR | Security/Release | Adopted — pending ratification at merge | ADR-011 D7 |
| DR-A8 | Drop only `ffi` now; P1-01 restructures | Product/Architecture | Adopted — pending ratification at merge | ADR-011 D11 |
| DR-A9 | `docs/07-archive/cautious-nevermore/` as non-authoritative history | Product/Architecture | Adopted — pending ratification at merge | ADR-011 D12 |
| DR-A10 | Licence for `reference/python/` | Data/Licensing | **Needs owner action: record licence ruling for reference/python/ on the PR.** Merge-blocking | ADR-012 §8 |
| DR-A11 | Parity fixtures synthetic-only; no real data committed | Data/Licensing | Adopted — pending ratification at merge | ADR-012 §9 |
| DR-A12 | Frozen CI oracle until parity and Phase 2 live evidence; review at P2-09 | Product/Architecture | Adopted — pending ratification at merge | ADR-012 §10 |

### Decisions raised but not decided

P0-01 decides none of these. Each B and C item stays **proposed**, and each D item stays proposed or
open, as its register entry says. Every work package they block is not Ready until the named owner rules
(engine-spec §8.16.1). The blocked packages are listed in `docs/01-work-packages/README.md`.
`docs/00-meta/decision-register.md` is authoritative for status, owner and Blocks.

| DR | Question | Owner role |
|---|---|---|
| DR-B1 | Oracle correction policy (the ledger) | Statistical |
| DR-B2 | Live oracle in CI, committed fixtures, or both | Statistical, Product/Architecture (Security/Release for CI actions) |
| DR-B3 | Parity tolerance classes | Statistical |
| DR-B4 | Fixing and extending the synthetic world | Statistical |
| DR-B5 | Team-strength estimand, Layer-3 market rows, matchup-grade sign | Statistical |
| DR-B6 | Typed failure vs. faithful port of oracle failure paths | Statistical, Product/Architecture |
| DR-C1 | RAPM and participation on the live path | Product/Architecture, Statistical |
| DR-C2 | Layers A–F vs. the GRID pipeline as the governing architecture | Product/Architecture, Statistical |
| DR-C3 | Where GRID talent enters Layer D | Statistical |
| DR-C4 | Projection horizons | Product/Architecture |
| DR-C5 | Primary metric and promotion gates | Statistical |
| DR-C6 | Three-season window for stateful components | Statistical |
| DR-C7 | Gradient boosting on GRID's critical path; the V(s) estimator | Product/Architecture, Statistical |
| DR-C8 | Booster backend | Product/Architecture |
| DR-C9 | NCAA and feeder prior form | Statistical |
| DR-C10 | Kalman semantics bundle | Statistical |
| DR-C11 | VOR and lineup-simulation scope | Product/Architecture |
| DR-C12 | Training-label stat definitions and season type | Statistical, Data/Licensing |
| DR-C13 | `drive_points` vocabulary and situation set | Statistical |
| DR-C14 | Engine operating model | Product/Architecture |
| DR-C15 | Storage | Product/Architecture |

The D items were raised by this consolidation's documents. Owner roles are as in the register.

| DR | Question | Owner role |
|---|---|---|
| DR-D1 | Source of record for coaching and coordinator changes | Data/Licensing, Statistical |
| DR-D2 | Timestamped pre-lock historical market-line source | Data/Licensing, Statistical |
| DR-D3 | ShareAlike obligations on participation-derived outputs | Data/Licensing |
| DR-D4 | Prior-season postseason plays in play-level estimators | Statistical |
| DR-D5 | The "corrected as retrieved" approximation for the historical proof | Statistical, Data/Licensing |
| DR-D6 | Non-affine scoring rules | Product/Architecture, Statistical |
| DR-D7 | Whether the superseded §5.1 stat-vector asymmetry is intended | Product/Architecture, Statistical |
| DR-D8 | The output contract's open definitions (percentiles, thresholds, change attribution) | Product/Architecture, Statistical |
| DR-D9 | Low-evidence opportunity thresholds and weights | Statistical |
| DR-D10 | Market-line scale | Statistical |
| DR-D11 | Fixed-point re-seed set, scale mapping and stopping rule | Statistical |
| DR-D12 | RAPM real-data penalty and identifiability | Statistical |
| DR-D13 | Layer-1 estimand (unit plus-minus vs. individual credit) | Statistical |
| DR-D14 | Layer-1 cross-fit design | Statistical |
| DR-D15 | Operational definition of Layer-1′ | Statistical |
| DR-D16 | Predictive exposure basis | Statistical |
| DR-D17 | Kalman season boundary | Statistical |
| DR-D18 | Changepoint semantics | Statistical |
| DR-D19 | Prior-scale hand-off to the Kalman state | Statistical |
| DR-D20 | Ensemble stacking level | Statistical |
| DR-D21 | Rate-model family | Statistical |
| DR-D22 | Provider EP/EPA columns as features | Statistical |
| DR-D23 | Missing-provider-projection policy | Statistical |
| DR-D24 | Last-season baseline in the gate set | Statistical |
| DR-D25 | Recency-baseline weights | Statistical |
| DR-D26 | Seed-ensemble statistics for recovery gates | Statistical |
| DR-D27 | V(s) parity envelope | Statistical |
| DR-D28 | Synthetic draw tape | Statistical |
| DR-D29 | On-disk parity-fixture format | Statistical, Product/Architecture |
| DR-D30 | `reference-oracle` as a required merge check | Security/Release, Product/Architecture |

## Objective
At the P0-01 merge commit, the repository is engine-only and consistent with itself:

- one authority document (`engine-spec.md`) with a byte-identical mirror;
- the Python oracle imported and passing its own suite;
- no application artifacts;
- every verification gate passing;
- every owner decision either adopted for ratification at merge or registered as open.

## Observable outcome
None for engine consumers, because no engine code changes. A maintainer can observe:

- `engine-spec.md`;
- an 8-recipe, 15-command `just verify`;
- the `reference-oracle` CI job;
- the work-package index (`docs/01-work-packages/README.md`);
- the registers in `docs/00-meta/`.

## Preconditions
- PR #1 (P1-00) approved at adversarial review round 4 with 0 blockers. Its 49 commits are the base of
  this branch (`3823478`; DR-A6).
- The owner's request and choice of option, 2026-10-01.
- The tools pinned in `toolchains/dev-tools.lock`, and CPython 3.11 for the oracle (verified on
  3.11.15).
- A merge gate rather than a precondition: the DR-A10 ruling.

## Scope
- **Modules and files expected to change:** every path in "Paths affected" below, and no other.
- **Contracts consumed:** none. This is the first engine-only package.
- **Contracts changed:** created as drafts.
  - `docs/03-contracts/engine-output-contract.md`, `parity-fixture-contract.md` and
    `plays-contract.md`.
  - The provider document `docs/04-providers/nflverse/access-and-license.md`.
  - Eight model specs in `docs/05-model-specs/`, all `Draft`: their equations are not approved by the
    Statistical owner.

### Paths affected

This is the same list as ADR-011 "Paths affected". `scripts/check-traceability.sh` requires each
non-trivial changed path to be named, exactly or by a directory prefix in backticks, in a document the
commit messages cite.

**Added**

- `engine-spec.md`
- `docs/00-meta/specs/engine-spec.md`
- `docs/00-meta/specs/superseded/`:
  - `docs/00-meta/specs/superseded/alpha-spec.md`
  - `docs/00-meta/specs/superseded/final-build-spec.md`
  - `docs/00-meta/specs/superseded/README.md`
- `README.md`
- `docs/00-meta/decision-register.md`
- `docs/00-meta/known-issues.md`
- `docs/00-meta/lessons-learned.md`
- `docs/01-work-packages/README.md`
- `docs/01-work-packages/p0-01-engine-consolidation.md`
- `docs/02-adr/011-engine-only-pivot.md`
- `docs/02-adr/012-python-reference-oracle.md`
- `docs/03-contracts/`:
  - `docs/03-contracts/README.md`
  - `docs/03-contracts/engine-output-contract.md`
  - `docs/03-contracts/parity-fixture-contract.md`
  - `docs/03-contracts/plays-contract.md`
- `docs/04-providers/nflverse/access-and-license.md`
- `docs/05-model-specs/`:
  - `docs/05-model-specs/README.md`
  - `docs/05-model-specs/cross-league-priors.md`
  - `docs/05-model-specs/evaluation-and-leakage.md`
  - `docs/05-model-specs/layer1-credit.md`
  - `docs/05-model-specs/projection-stack.md`
  - `docs/05-model-specs/rapm-attribution.md`
  - `docs/05-model-specs/state-space-kalman.md`
  - `docs/05-model-specs/synthetic-world.md`
  - `docs/05-model-specs/value-model.md`
- `docs/06-sessions/2026-10-01-consolidation-inventory/`
- `docs/06-sessions/review-P1-00-adversarial-round1.md`
- `docs/06-sessions/review-P1-00-adversarial-round2.md`
- `docs/06-sessions/review-P1-00-adversarial-round3.md`
- `docs/06-sessions/review-P1-00-adversarial-round4.md`
- `docs/07-archive/`
- `reference/python/`
- `.ai/evidence/P0-01/`

**Changed**

- `CLAUDE.md`
- `docs/CLAUDE.md`
- `docs/00-meta/authority-index.md`
- `docs/00-meta/dashboard.md`
- `docs/00-meta/daily-log.md`
- `docs/02-adr/README.md`
- `docs/02-adr/001-repo-bootstrap-decisions.md` (front matter `superseded-by` only)
- `docs/02-adr/006-deferred-deliverables.md` (front matter `superseded-by` only)
- `docs/02-adr/007-verify-covers-guards-and-doctests.md` (front matter `superseded-by` only)
- `docs/02-adr/009-verify-ps1-exit-codes.md` (front matter `superseded-by` only)
- `docs/04-providers/cfbd/README.md`
- `docs/04-providers/nflverse/README.md`
- `docs/06-sessions/README.md`
- `docs/99-templates/template-model-spec.md`
- `docs/99-templates/template-work-package.md`
- `Cargo.toml`
- `Cargo.lock`
- `ai-toolchain.lock`
- `crates/application/src/lib.rs`
- `crates/governance/src/lib.rs`
- `justfile`
- `scripts/verify.sh`
- `scripts/verify.ps1`
- `scripts/bootstrap-repo.sh`
- `scripts/check-authority-sync.sh`
- `scripts/check-traceability.sh`
- `scripts/check-evidence-claims.sh`
- `scripts/check-secrets.sh`
- `scripts/check-verify-parity.sh`
- `scripts/generate-evidence-manifest.sh`
- `tests/guards/run.sh`
- `_typos.toml`
- `.gitignore`
- `.gitattributes`
- `.github/workflows/alpha-ci.yml`

**Deleted or moved**

- `app/` (`app/pubspec.yaml`)
- `crates/ffi/` (`crates/ffi/Cargo.toml`, `crates/ffi/src/lib.rs`, `crates/ffi/src/generated/.gitkeep`)
- `toolchains/flutter.version`
- `alpha-spec.md`, moved to `docs/00-meta/specs/superseded/alpha-spec.md`
- `final-build-spec.md`, moved to `docs/00-meta/specs/superseded/final-build-spec.md`
- `docs/00-meta/specs/alpha-spec.md`, the old mirror, deleted (byte-identical to the archived copy)

**Checked and unchanged:** `deny.toml`, `migrations/`, `.sqlx/`, `.env`, `rust-toolchain.toml`,
`toolchains/dev-tools.lock`, `.ai/evidence/P1-00/` and `.claude/`.

## Non-goals
- **No Rust engine code.** No new crate and no crate rename; P1-01 does those (DR-A8).
- **No Rust parity tests** and no committed parity fixtures. Those start at P1-01.
- **No oracle correction.** The ledger in `reference/python/PARITY.md` is proposed, not applied
  (DR-B1). Oracle source changes are limited to patches P1 and P2.
- **No change to Windows merge authority** (DR-A3).
- **No dependency changes.** No new production dependency, no `cargo update`, and no change to
  `rust-toolchain.toml`.
- **No edits to protected records:**
  - `.ai/evidence/P1-00/`;
  - the body of any accepted ADR (front-matter annotations only);
  - verbatim records.
- **No change to `.claude/`.** Permission changes are Security/Release owner actions.
- **No owner actions.** Ratifying decisions, merging, closing pull requests and tagging are the owner's.
- **No real third-party data** is committed.

## Inputs and fixtures
- **GRID-Engine** `3823478` (the PR #1 head), on base `origin/main` `48ee320`.
- **cautious-nevermore** (`Seismic-Fate/cautious-nevermore`):
  - `59bce1d` for the oracle and two archived documents;
  - `165ccde`, `c33712e` and `546de72` for the other archived documents
    (`docs/07-archive/cautious-nevermore/MANIFEST.md`).
- **The P1-00 adversarial review records**, rounds 1–4, from PRs #2 and #3. They reviewed `7787921`,
  `de3b36c`, `8e07b04` and `3823478`.
- **External facts read on 2026-10-01:**
  - the nflreadr documentation, for licensing (`docs/04-providers/nflverse/access-and-license.md`);
  - GitHub, for `6b0eeee`, `87e227f` and CN PR #53 (`reference/python/README.md`).
- **Fixtures:** none created. The oracle's own legacy golden, `tests/grid/golden/snapshot.npz`, arrives
  verbatim as part of the import.

## Oracle parity targets
**None for Rust.** At import, the oracle's own suite passes on Linux:

- **446 passed**, isolation guard 0 violations;
- CPython 3.11.15, threads pinned to 1, 2026-10-01 (`reference/python/PARITY.md` section (a)).

**The oracle is legacy.** Its synthetic numbers are legacy-generator values (KI-NEW-Y0), and none is a
parity target until ledger entry 1 has run (DR-B1, proposed).

## Implementation constraints
- **Frozen recipes.** The verify recipes are frozen (ADR-001 D5). The only amendment is the `test-ffi`
  de-scoping, made in lockstep in all three implementations (ADR-011 D6).
- **Verbatim imports.** Imported oracle files are byte-identical except patches P1 and P2, and
  `MANIFEST.tsv` proves it.
- **Verbatim records are never edited.** That covers the superseded specs, the archive and the imported
  session records. Their known errors are recorded in their manifests and READMEs.
- **Guard changes are mutation-checked.** Each new positive case in `tests/guards/run.sh` must fail
  against the `3823478` version of its script.
- **CRLF bytes are preserved** in `Cargo.toml`, `justfile`, `scripts/verify.ps1` and `ai-toolchain.lock`.
- **No model identifiers.** The implementing agent's harness policy forbids writing an AI model
  identifier into repository artifacts. This has a consequence for `ai-toolchain.lock`; see Evidence
  required.
- **The agent never commits on its own authority and never merges.**

## Security and licensing considerations
- **Secrets.** None touched. `scripts/check-secrets.sh` scans every new file, including
  `reference/python/`.
- **Permissions.** `.claude/settings.json` is unchanged. It has no `Bash(python3 -m pytest:*)` rule, and
  adding one is a Security/Release owner decision.
- **New hosts.** The `reference-oracle` job installs from PyPI through pip, constrained to
  `reference/python/requirements.lock`. That is a new host for CI.
  `reference/python/tools/investigations/fetch_realdata.py` downloads pinned nflverse release assets by
  manual invocation only, never in CI.
- **CI security boundary.** No third-party action is added. `actions/setup-python` would need
  Security/Release approval.
- **Licence.** `reference/python/` is pending DR-A10, and that is merge-blocking. As ADR-001 D4 noted,
  publishing code permissively is not cleanly reversible once third parties can rely on it.
- **Provider terms.**
  - nflverse participation is CC-BY-SA 4.0, and no real data is committed (DR-A11).
  - `docs/04-providers/nflverse/access-and-license.md` is a draft. Its per-dataset terms for
    play-by-play and rosters still need Data/Licensing confirmation.
  - CFBD terms are unverified (`docs/04-providers/cfbd/README.md`).
- **Seed data.** `backend/db/data/coaching_changes_2025.json` is unverified (KI-NEW-D1). It is never a
  fixture or a provider input.
- **Dependency advisories.** `cargo audit` warns that `chacha20 0.10.1` is yanked. The warning predates
  this package (it was already present at `3823478`), and the lock entry is unchanged. `deny.toml` sets
  `yanked = "deny"`, yet `cargo deny check` reports `advisories ok`. That mismatch is for the
  Security/Release owner.
- **Environment leak.** `backend/db/connection.py` loads the repository-root `.env`. This is documented
  in `reference/python/README.md` and is harmless today.

## Acceptance criteria
Each criterion is verified by the command shown, run against the final content commit X.

**Guards and verification**

- [ ] `bash tests/guards/run.sh` ends with `0 failed`, with at least 78 cases.
- [ ] `./scripts/check-migrations.sh` prints OK.
- [ ] `./scripts/check-secrets.sh` prints OK.
- [ ] `./scripts/check-traceability.sh origin/<PR base>` prints `OK all N non-trivial path(s) traced`.
- [ ] `./scripts/check-authority-sync.sh` prints OK: `engine-spec.md` is byte-identical to
      `docs/00-meta/specs/engine-spec.md`.
- [ ] `./scripts/check-verify-parity.sh` prints OK `on 15 verification step(s)`.
- [ ] `./scripts/check-env-contract.sh` prints OK.
- [ ] `./scripts/check-evidence-claims.sh origin/<PR base> P0-01` prints `OK 10 claim(s)`.
- [ ] `typos` exits 0, with no exclusion added for `reference/python/` or `docs/`.
- [ ] `just verify` exits 0.
- [ ] `pwsh -NoProfile -File scripts/verify.ps1 -Scope Full` exits 0 where pwsh is available. The
      `windows-authoritative` job is green on the pull request; it is the merge gate (DR-A3).
- [ ] `cargo build --workspace --locked` and `cargo test --workspace --locked` exit 0.
- [ ] `env -u DATABASE_URL SQLX_OFFLINE=true cargo check --workspace --locked` exits 0.

**Application removal**

- [ ] `test ! -e app && test ! -e crates/ffi && test ! -e toolchains/flutter.version` exits 0.
- [ ] `! grep -q flutter_rust_bridge Cargo.toml Cargo.lock` exits 0.
- [ ] `grep -c '^\[\[package\]\]' Cargo.lock` prints 171.
- [ ] `grep -c 'Assert-Ok "' scripts/verify.ps1` prints 15.

**Specification and records**

- [ ] `git show 3823478:alpha-spec.md | cmp - docs/00-meta/specs/superseded/alpha-spec.md` and
      `git show 3823478:final-build-spec.md | cmp - docs/00-meta/specs/superseded/final-build-spec.md`
      both exit 0.
- [ ] `git diff --quiet 3823478 HEAD -- .ai/evidence/P1-00` exits 0, so the P1-00 record is untouched.
- [ ] `git merge-base --is-ancestor 8d43203 HEAD` exits 0.
- [ ] `grep -L '^status: Proposed' docs/02-adr/011-engine-only-pivot.md docs/02-adr/012-python-reference-oracle.md`
      prints nothing: both ADRs are still Proposed.

**Reference oracle**

- [ ] `cd reference/python && python3 tools/verify_manifest.py` prints
      `verify_manifest: OK (105 rows: 103 verbatim, 2 patched; ...)`.
- [ ] Set up a venv with `pip install -r requirements.txt -c requirements.lock` and
      `OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 MKL_NUM_THREADS=1`. Then `python3 -m pytest` in
      `reference/python/` reports `446 passed` and `isolation guard: 0 violations`, and the
      `reference-oracle` CI job is green.
- [ ] `grep -rn -i 'python\|pyo3' crates/ Cargo.toml` prints nothing.

**Evidence**

- [ ] `.ai/evidence/P0-01/manifest.json` is generated by `just evidence P0-01` with `commit` = X, and
      `jq empty .ai/evidence/P0-01/manifest.json` exits 0.

## Verification
```text
# targeted
bash tests/guards/run.sh
./scripts/check-migrations.sh
./scripts/check-secrets.sh
./scripts/check-traceability.sh origin/<PR base>
./scripts/check-authority-sync.sh
./scripts/check-verify-parity.sh
./scripts/check-env-contract.sh
typos
cargo build --workspace --locked && cargo test --workspace --locked
env -u DATABASE_URL SQLX_OFFLINE=true cargo check --workspace --locked
test ! -e app && test ! -e crates/ffi && test ! -e toolchains/flutter.version
git show 3823478:alpha-spec.md       | cmp - docs/00-meta/specs/superseded/alpha-spec.md
git show 3823478:final-build-spec.md | cmp - docs/00-meta/specs/superseded/final-build-spec.md
git diff --quiet 3823478 HEAD -- .ai/evidence/P1-00
(cd reference/python && python3 tools/verify_manifest.py)
(cd reference/python && OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -m pytest)

# canonical
mkdir -p target && sqlx database create && sqlx migrate run
just verify
pwsh -NoProfile -File scripts/verify.ps1 -Scope Full      # merge-authoritative on windows-latest (DR-A3)

# evidence (after X is committed)
just evidence P0-01
./scripts/check-evidence-claims.sh origin/<PR base> P0-01
```
Verification recipes are a frozen contract. Make the repository satisfy them; never edit a
recipe to make a failure disappear.

## Numerical/performance tolerances
- **Rust.** Not applicable: no numerical code changes.
- **Oracle.** The oracle suite's own tolerances, unchanged and verbatim. That includes the golden-master
  tolerance recorded in `reference/python/PARITY.md`.
- **Timing.** The suite takes about 150 s on 4 vCPU with threads pinned to 1. The `reference-oracle`
  job has a 30-minute timeout. The wall-clock assertion in `tests/grid/test_performance.py` is
  non-gating (`PARITY.md` section (f)).

## Migration and rollback
- **Database.** Not applicable. No migration is added or changed, and `scripts/check-migrations.sh`
  checks that.
- **Rollback.** Reverting the merge commit restores `app/`, `crates/ffi/`, the two specs at the root,
  and the 9-recipe chain. The specs were moved with git renames, so their history follows them.
- **Licence.** If DR-A10 is not ruled as recommended, `reference/python/` and the `reference-oracle` job
  cannot merge, and ADR-012 must be revised before acceptance. Once the tree is merged under
  `MIT OR Apache-2.0`, the relicensing is not cleanly reversible (ADR-001 D4).

## Evidence required
The verification-command outputs, the CI results (`linux-smoke`, `windows-authoritative`, `guards`,
`reference-oracle`), the generated manifest, `PR-BODY.md` and the fresh-context review report. The
steps follow; do them last, on the final commit.

1. **Provenance fields.** In `ai-toolchain.lock`, set `project_alias`, `model_public_id`, `harness` and
   `harness_version` to `unrecorded`, preserving the CRLF bytes.
   - **Why.** The implementing agent's harness policy forbids writing model identifiers into repository
     artifacts.
   - **This deviates from engine-spec §8.20,** which requires the model and harness actually used.
   - **The owner may record the real values** before step 5. Once the P0-01 record merges it is
     immutable (`scripts/check-evidence-claims.sh` refuses a rewrite), so a later correction cannot go
     into this record.
   - These fields still hold the P1-00 values until they are set.
2. **Commit.** Commit all P0-01 content. At least one commit message cites `P0-01` and `ADR-011`. Call
   the final content commit X. The worktree must be clean apart from the evidence files.
3. **Verify.** Run the Verification commands on X. Record the traceability count N, which depends on the
   base the pull request targets (`origin/main` is `48ee320`, or the post-PR-#1 merge commit if that
   lands first).
4. **Write the input.** Write `.ai/evidence/P0-01/verification-input.json` by hand from the real results,
   in the same shape as P1-00's. Set `run_against_commit` to X, set `"canonical": true` on `just verify`,
   and include the literal phrase `All N non-trivial paths traced`.
5. **Generate.** Run `just evidence P0-01`, which writes `.ai/evidence/P0-01/manifest.json` with `commit`
   set to X.
6. **Write the PR body.** Write `.ai/evidence/P0-01/PR-BODY.md` with these hooks, each re-derived on X:
   - `Final commit: <12 hex of X>`;
   - `agree at **15 steps**`;
   - `carries 78 committed behaviour cases` (more, if cases are added);
   - `runs **8 recipes**`;
   - `resolves 171 crates`;
   - `` `Assert-Ok` (15/15) ``;
   - `sha256:<sha256 of manifest.json>`.
7. **Check.** `./scripts/check-evidence-claims.sh origin/<PR base> P0-01` must print `OK 10 claim(s)`.
8. **Commit the evidence** as commit Y, citing P0-01, and repeat step 7 on Y. The evidence files are
   trivial for traceability, so N is unchanged.
9. **If the base moves before merge,** redo steps 3–8.

A review record committed to this pull request (`docs/06-sessions/review-P0-01-…`) is a new path. It
must then be added to "Paths affected" here and in ADR-011.

## Stop/decision conditions
- **The owner overrides an A-decision.** Change the pull request to match, before merge.
- **DR-A10 is not recorded, or is ruled otherwise.** Do not merge `reference/python/`.
- **The runner's Python moves the golden master.** Escalate to the owner. Never loosen a tolerance or
  edit the verbatim tree (critic X-7).
- **A verify recipe fails** for any reason other than the `test-ffi` de-scoping. Stop and report. This
  package makes no further D5 amendment.
- **The assembled `engine-spec.md` renumbers a section** that a contract, model spec, register or
  ADR-011/012 cites. Fix the citations before merge.
- **`typos` hits a word inside a verbatim record.** Never edit the record, and never exclude a
  directory. A domain term or identifier gets a justified allowlist entry; a verbatim copy whose hits
  are not domain terms is excluded by its exact path with a justification (P0-01 lists the nine
  inventory reports and the thirteen archived cautious-nevermore copies this way).
- **Traceability reports an unlisted path.** Add it to "Paths affected" here and in ADR-011 only if it
  belongs to P0-01. Otherwise remove it from the change set, as with a stray download or build artifact.

## Follow-up

| # | Item | Owner | Risk if deferred |
|---|---|---|---|
| 1 | Oracle correction-ledger work package, once DR-B1 is ratified, starting with ledger entry 1 (KI-NEW-Y0) | Statistical | Team, DEF and matchup parity stay meaningless, and P1-12 stays blocked |
| 2 | ADR on the authoritative platform (DR-A3), before P1-11 | Product/Architecture, Security/Release | The release platform and the reference machine stay undefined |
| 3 | A `Bash(python3 -m pytest:*)` rule in `.claude/settings.json` | Security/Release | Agent sessions keep prompting before running the oracle suite |
| 4 | `cargo audit` reports `chacha20 0.10.1` yanked, while `deny.toml` sets `yanked = "deny"` and `cargo deny check` passes | Security/Release | A dependency policy that is not enforced as written |
| 5 | `actions/setup-python` or a pinned runner image for `reference-oracle` | Security/Release | A change of the runner's `python3` moves the golden master unannounced |
| 6 | Crate restructure: `application` → `pipeline`, plus `grid-cli` and `synth` (DR-A8) | P1-01 | None until P1-01; the crate-count claims change then |
| 7 | `.gitattributes` rules for the verbatim trees (`reference/python/`, `docs/07-archive/`, `docs/00-meta/specs/superseded/`, the imported session records), if they do not land in P0-01 | Product/Architecture | A `core.autocrlf` checkout breaks the `MANIFEST.tsv` hashes and the verbatim copies |
| 8 | After merge, close PR #1 as included. Close PRs #2 and #3 unmerged | Owner | Merging #2 or #3 recreates the invalid `docs/05-sessions/` |
| 9 | Tag `oracle-legacy-59bce1d` on the import commit (`reference/python/PARITY.md` (a)) | Owner | The legacy reference has no stable name |
| 10 | Keep `engine-spec.md` Appendix F and §9.5 in step with the register's DR-D entries. The spec drafts list most DR-D items as "not yet in the register", and they describe DR-D7 and DR-D8 as one question, which the register separates | Register owner, spec assembler | Readiness is read from a stale table |
| 11 | Known-issues entries for defects that the model-spec authors raised without KI IDs, if not complete at merge | Register owner | Defects are cited by prose instead of ID |
| 12 | Commit the probe scripts behind numbers recorded as "not reproducible from the repo" in `layer1-credit.md`, `projection-stack.md` and `evaluation-and-leakage.md`, under `reference/python/tools/investigations/` | Statistical | Those numbers cannot be re-derived |
| 13 | Fix the review-workflow brief that names `docs/05-sessions/` (it is outside the repository) | Owner | Future reviews are written to an invalid path |
| 14 | `docs/99-templates/template-adr.md`: its Status guidance still ranks ADRs below the superseded specs and cites `alpha-spec.md` 1.4 | Template owner | New ADRs copy outdated rank text |
| 15 | Structural guards: no crate depends on Python (engine-spec §1.1 item 8), and the CLI depends only on the orchestration crate (§8.1 rule 4) | P1-01 | The rules rest on review alone |
| 16 | Extend the "Template conformance" tables of the eight draft model specs to the new template headings | Statistical, before each spec is approved | A spec could be approved with a §6.8 field missing |

### Known limitations
- **`ai-toolchain.lock` provenance is `unrecorded` for P0-01.** This deviates from engine-spec §8.20
  (Evidence required, step 1).
- **The branch name was assigned by the harness:** `claude/grid-engine-consolidation-e7kmh9`, not
  `wp/P0-01-…` (ADR-001 D1 convention). The owner may rename it when opening the pull request.
- **No agent budget was declared** (engine-spec §8.18.1).
- **Windows is unproven.** The full chain and the new `run.sh` evidence fixtures have run only under pwsh
  on Linux, in a scratch copy. The first `windows-authoritative` run is the evidence for Windows.
- **The runner's Python is unproven.** `ubuntu-latest`'s `python3` has not been checked against the lock,
  which was verified on 3.11.15. The first `reference-oracle` run is the evidence.
- **Contracts and model specs are drafts,** and their equations are unapproved. Their citations into
  `engine-spec.md` depend on the assembled numbering.
- **Guard residuals** (ADR-011 D7):
  - the R4-1 behaviour change for non-regular paths;
  - the R4-2 gaps (pipes, construct keywords);
  - the traceability rename gap.
