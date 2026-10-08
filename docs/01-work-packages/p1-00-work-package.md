---
work-package-id: P1-00
status: In Progress
risk-class: Medium
owner: Product/Architecture owner
implementer: Claude Opus 5
reviewer: Pending — fresh-context reviewer or human
alpha-phase: 1
---

# P1-00 — Agent-Ready Repository, Authority Index, CI, Verification Scripts, and Work-Package Template

## Status
In Progress

## Ownership and risk
- Owner: [Human Product Owner]
- Implementer: Claude Opus 5 (claude-opus-5-20251401)
- Reviewer: Fresh-context reviewer agent or human
- Risk class: Medium (foundational; errors here propagate to all downstream packages)
- Required human approvals: Architecture owner approval of authority order and crate boundaries; Merge reviewer approval of CI skeleton and branch protection rules.

## Authority
- `final-build-spec.md`: §2 (System Architecture), §3 (Deployment Model), §7 (Rust Core and Concurrency), §8 (Persistence), §19 (Testing), §21 (Dependency Strategy), §22 (Application Lifecycle)
- Alpha spec: §1.1 (Non-negotiable architecture constraints), §1.5 (Order of authority), §1.6 (AI-first engineering principles), §8.7 (Claude-ready repository structure), §8.8 (Work-package contract), §8.9 (Required Claude execution workflow), §8.10 (Specialized agent roles), §8.11 (Canonical verification interface), §8.12 (AI contribution evidence), §9.2 (Phase 1 functional scope — AI implementation foundation), §9.4 (Phase 1 exit criteria — AI implementation quality), §9.5 (Phase 1 workstream sequence — P1-00), §12.7 (AI-specific regression protections), §12.8 (Pull-request evidence contract), §17.1 (Phase 1 deliverables), §17.3 (AI implementation evidence deliverables), Appendix B (Work-Package Template), Appendix D (Prohibited AI Coding Shortcuts), Appendix E (Claude Code Workflow Reference Notes)

## Objective
Establish the repository as a self-describing, authority-driven environment that a fresh Claude session can enter, understand, and correctly extend without relying on prior chat context. Produce the canonical verification scripts, authority index, AI toolchain lock, CI skeleton, and the first work-package template so that every subsequent P1 workstream begins from a documented, gated, and reproducible baseline.

## User-visible outcome
None — pure infrastructure and developer-experience foundation.

## Preconditions
- GitHub repository `Seismic-Fate/GRID-Alpha` exists and is writable by the implementer.
- The initial skeleton commit (from pre-Claude bootstrap) contains: `Cargo.toml` workspace root, empty `crates/*/`, `app/pubspec.yaml`, `docs/CLAUDE.md`, `justfile`, `ai-toolchain.lock`, `.claude/settings.json`, `scripts/verify.sh`, `scripts/verify.ps1`, `.gitignore`, `rust-toolchain.toml`.
- The cloud environment `grid-alpha-opus5` is available and has completed its setup script successfully.
- The human product owner has approved the crate boundary list and the authority ordering.

## Scope
- Modules/files expected to change:
  - `docs/00-meta/authority-index.md` (create — canonical authority map)
  - `docs/00-meta/dashboard.md` (create — Dataview query stub for Obsidian)
  - `docs/01-work-packages/P1-00-repo-bootstrap.md` (create — this file, committed as its own evidence)
  - `docs/99-templates/template-work-package.md` (create — reusable Appendix B template)
  - `docs/99-templates/template-adr.md` (create — lightweight ADR template)
  - `docs/99-templates/template-session-log.md` (create — per-session trace)
  - `docs/99-templates/template-model-spec.md` (create — stub for P1-06)
  - `docs/99-templates/template-provider-contract.md` (create — stub for P1-03/P1-04)
  - `.github/workflows/alpha-ci.yml` (create — Linux smoke + Windows authoritative CI skeleton)
  - `scripts/check-migrations.sh` (create — validates append-only migration policy)
  - `scripts/check-secrets.sh` (create — scans for secret patterns in PR diff)
  - `scripts/check-traceability.sh` (create — validates that every non-trivial file change maps to a work-package or ADR)
  - `scripts/generate-evidence-manifest.sh` (refine — ensure it emits the exact JSON schema required by §8.12)
  - `.ai/evidence/P1-00/` (create — first evidence manifest)
  - `docs/CLAUDE.md` (refine — add canonical verification commands if missing, ensure module-level CLAUDE.md policy is explicit)
  - `CLAUDE.md` (repo root — refine if needed, but do not bloat)
  - `docs/02-adr/001-repo-bootstrap-decisions.md` (create — ADR recording crate split, authority order, and tool choices)
- Contracts consumed:
  - None (this is the foundational package; no upstream business contracts exist yet).
- Contracts changed:
  - None (no FFI or provider contracts are introduced yet).

## Non-goals
- No implementation of ingestion adapters, models, feature store, or simulation.
- No Flutter UI code beyond `app/pubspec.yaml` and `app/lib/main.dart` placeholder.
- No SQLx migrations beyond the empty `migrations/` directory (schema creation is P1-02).
- No provider fixtures, raw data, or sanitized sample files (P1-03 and P1-04).
- No GitHub branch protection rule enforcement by Claude (human must configure these post-merge via GitHub UI).
- No Windows code-signing certificates or Azure Key Vault configuration.
- No `cargo-deny` policy finalization (initial deny.toml skeleton only; policy refinement is P1-11).
- No Flutter Windows golden tests (no UI to test yet).
- No benchmark or performance tests (no code to measure yet).
- No changes to `rust-toolchain.toml` channel (stable is fixed).

## Inputs and fixtures
- Input: The pre-existing bootstrap skeleton from the initial commit.
- Input: `alpha-spec.md` and `final-build-spec.md` (read-only authority documents).
- Fixture: None (this is a documentation and script package).

## Implementation constraints
- All changes must be verifiable from a clean checkout using only committed files and the cloud environment setup script.
- No new production Rust or Flutter dependencies may be introduced.
- The CI skeleton must reference the existing `justfile` and `scripts/verify.*` without changing their CLI contracts.
- All scripts must be non-interactive, idempotent where practical, and return non-zero on failure.
- The authority index must be a human-readable markdown file, not a generated JSON blob.
- The work-package template must follow Appendix B exactly so that future packages can be copy-pasted mechanically.
- The evidence manifest generator must produce a file path and JSON structure consistent with §8.12.
- No AI-generated content may be committed without a corresponding verification command having been run and captured.

## Acceptance criteria
- [ ] `docs/00-meta/authority-index.md` exists and correctly orders authority from `final-build-spec.md` → `alpha-spec.md` → ADRs → contracts → model specs → work packages → tests → existing code.
- [ ] `docs/99-templates/template-work-package.md` exists and follows the exact Appendix B structure from the alpha spec.
- [ ] `docs/99-templates/template-adr.md` exists and contains: status, context, decision, consequences, compliance, and alternatives considered.
- [ ] `docs/99-templates/template-session-log.md` exists with frontmatter: date, branch, model, phase.
- [ ] `docs/99-templates/template-model-spec.md` exists with: target, inputs, equations, priors, constraints, seed policy, tolerances, reference examples, and explanation fields.
- [ ] `docs/99-templates/template-provider-contract.md` exists with: source name, access method, schema version, fixtures, normalization map, failure cases, and retention rules.
- [ ] `.github/workflows/alpha-ci.yml` exists and defines two jobs: `linux-smoke` (runs `just verify` on `ubuntu-latest`) and `windows-authoritative` (runs `scripts/verify.ps1 -Scope Full` on `windows-latest`).
- [ ] `scripts/check-migrations.sh` exists and returns 0 if no migration files have been deleted or modified (only additions allowed).
- [ ] `scripts/check-secrets.sh` exists and returns non-zero if the diff contains patterns matching `ghp_`, `sk-ant-`, `CFBD_API_KEY`, or hardcoded database connection strings.
- [ ] `scripts/check-traceability.sh` exists and returns non-zero if any non-trivial file change (excluding templates, lockfiles, and `.github/`) is not referenced in either `docs/02-adr/` or `docs/01-work-packages/` within the commit message or PR body.
- [ ] `scripts/generate-evidence-manifest.sh` has been refined to accept a `WORK_PACKAGE_ID` argument, emit valid JSON, and include the fields required by §8.12: work-package ID, commit SHA, model/harness identifier, environment, files changed, contracts/ADRs referenced, verification results, reviewer findings, human approvals, known limitations.
- [ ] `docs/02-adr/001-repo-bootstrap-decisions.md` exists and records the crate boundary list, authority order, and the decision to use `just` over Make.
- [ ] `docs/CLAUDE.md` (root) contains the canonical `just verify` and `scripts/verify.ps1 -Scope Full` commands as the single source of verification truth.
- [ ] The `justfile` contains a `bootstrap` target that installs dev-only cargo tools (nextest, sqlx-cli, deny, audit, typos, just, hyperfine, mutants).
- [ ] The repository passes `just verify` (format, lint, typos, audit) on Linux in the cloud environment. Since there is no Rust code beyond empty `lib.rs` files, `cargo test` and `cargo clippy` must pass trivially (no warnings).
- [ ] `.ai/evidence/P1-00/manifest.json` exists and validates as JSON.
- [ ] A pull request is prepared with an atomic commit series, a PR body linking to this work package, and a completion record per Appendix C.

## Verification
```bash
# Targeted checks
echo "[P1-00] Checking authority index..."
test -f docs/00-meta/authority-index.md
echo "[P1-00] Checking templates..."
test -f docs/99-templates/template-work-package.md
test -f docs/99-templates/template-adr.md
test -f docs/99-templates/template-session-log.md
test -f docs/99-templates/template-model-spec.md
test -f docs/99-templates/template-provider-contract.md
echo "[P1-00] Checking CI..."
test -f .github/workflows/alpha-ci.yml
echo "[P1-00] Checking scripts..."
test -x scripts/check-migrations.sh
test -x scripts/check-secrets.sh
test -x scripts/check-traceability.sh
test -x scripts/generate-evidence-manifest.sh
echo "[P1-00] Checking ADR..."
test -f docs/02-adr/001-repo-bootstrap-decisions.md
echo "[P1-00] Checking evidence..."
test -f .ai/evidence/P1-00/manifest.json
jq empty .ai/evidence/P1-00/manifest.json

# Canonical verification
just verify
```

## Numerical/performance tolerances
Not applicable.

## Migration and rollback
**Amended 2026-08-20.** This originally read "Not applicable — no database schema changes",
which the package's own diff falsifies. Adversarial review of PR #1 caught it (finding M5).

`migrations/0001_schema_meta.sql` adds one infrastructure-only key-value table. Forward path:
`sqlx migrate run` against a blank database, covered by a test. Rollback: revert the branch —
the database lives in gitignored `target/` and is recreated by `just bootstrap`. The directory
is append-only from here, enforced by `scripts/check-migrations.sh`.

No schema version is seeded. An earlier revision wrote `schema_version = '0001'` into the
table, duplicating state SQLx already keeps in `_sqlx_migrations`; the literal would have gone
stale at `0002` while the test asserting it kept passing. The applied version is now derived
from the migrator. Whether a richer mechanism is needed is P1-02's decision.

## Evidence required
- Terminal output of the verification commands above.
- `just verify` exit status (must be 0).
- `.ai/evidence/P1-00/manifest.json` file content.
- PR body with the standard completion record per Appendix C.
- Screenshot or text capture of `typos` output showing no spelling errors in new documentation.

## Stop/decision conditions
- **If the alpha spec and final build spec disagree on crate names or authority order:** Stop. Produce a decision request comparing the two passages and asking the human product owner to resolve before continuing.
- **If the cloud environment cannot install `cargo-nextest` or `cargo-deny` due to network failures:** Retry up to 3 times with exponential backoff. If persistent, stop and report the environment as blocked.
- **If `just verify` fails due to a pre-existing issue in the bootstrap skeleton:** Stop. Do not weaken the `justfile` or `.claude/settings.json` to make the failure disappear. Produce a remediation work package or ask for human intervention.
- **If GitHub Actions YAML syntax is invalid:** The implementer must fix it; no human gate required for YAML formatting.
- **If a proposed template deviates from Appendix B structure:** The implementer must correct it before claiming done.

## Follow-up
- P1-01: Domain IDs, time/as-of types, error enums, and FFI contract skeleton (blocked until P1-00 is merged).
- ADR-002: Decision on whether to use `sqlx` offline mode with `cargo sqlx prepare` checked into `.sqlx/`, or to require a live database for CI (to be resolved in P1-02).
- Dependency policy refinement: `cargo-deny` `deny.toml` configuration with explicit license whitelist and banned crate categories (deferred to P1-11).
- Windows CI runner dependency caching: evaluate `Swatinem/rust-cache` and `subosito/flutter-action` caching (deferred to P1-11).
- Obsidian vault sync automation: evaluate GitHub Action to validate that `docs/` frontmatter is well-formed (deferred to P1-11).

---

## Amendment note — 2026-08-20

Recorded under `alpha-spec.md` §1.5: the product owner resolved the conflicts below during
implementation. Rationale and alternatives are in `docs/02-adr/001-repo-bootstrap-decisions.md`.

### Scope amended

| Original | Amended to | Why |
|---|---|---|
| Non-goal: "No SQLx migrations beyond the empty `migrations/` directory (schema creation is P1-02)." | One infrastructure-only `schema_meta` migration. All **domain** schema remains P1-02. | The frozen `check-sqlx` recipe cannot pass against an empty workspace, and `cargo nextest run --workspace` exits 4 with no tests. See ADR-002. |
| Follow-up: "ADR-002 … to be resolved in P1-02." | ADR-002 resolved in P1-00. | Same cause: the decision is a precondition for `check-sqlx` passing at all. |
| Scope path: `docs/adr/001-repo-bootstrap-decisions.md` | `docs/02-adr/001-repo-bootstrap-decisions.md` | Numbered vault retained over the flat §8.7 tree (ADR-001 D3). |

### Added, not in the original scope

Each was found during implementation and escalated rather than worked around.

| Addition | Cause |
|---|---|
| ADR-003 — sqlx 0.8 → 0.9 | `cargo audit` failed on RUSTSEC-2023-0071 (`rsa`, no fix available) reached via `sqlx-mysql`. Feature narrowing is ineffective; 0.9 removes it. Also resolves a library/CLI version skew. |
| ADR-004 + `.gitattributes` + LF conversion of `scripts/*.sh` | Every committed file was CRLF, leaving `scripts/verify.sh` unparsable by bash. The §8.11 canonical Linux command could not run. |
| `scripts/check-authority-sync.sh` | `alpha-spec.md` exists twice (root and vault mirror) with nothing preventing a fork. |
| `scripts/check-verify-parity.sh` | The `justfile` and `scripts/verify.sh` duplicate all seven steps. D5 forbids unifying them, so drift is asserted against instead. |
| `scripts/check-env-contract.sh` | The environment presets a stale `DATABASE_URL`; a committed `.env` cannot override it. Failure surfaced only as SQLite error code 14. |
| `tests/guards/run.sh` | The guard scripts were the largest body of new logic with no committed tests. Both `check-secrets.sh` defects were found by ad-hoc testing that nothing re-ran. Added on review finding "minor 6"; it caught a real gap in the M3 fix on its first run. |
| `toolchains/flutter.version` | §8.11 requires the Flutter/FRB toolchain be pinned through committed toolchain files; §8.7 names this path. Review finding M6. |
| `docs/02-adr/006-deferred-deliverables.md` | §8.7/§9.2 items deferred without a recorded deviation. Review finding M7. |

### Unchanged

P1-02's substantive scope (domain schema, durable jobs, raw-data and version primitives) is
untouched — only the SQLx **toolchain** bootstrap moved. Fixtures remain deferred to
P1-03/P1-04, recorded in ADR-001 rather than accepted silently.
