---
reviewer: fresh-context-adversarial
date: 2026-08-20
work_package: P1-00
branch: wp/P1-00-repo-bootstrap
base: origin/main
review_branch: claude/adversarial-review-p1-00-l32emp
head_reviewed: de3b36cf98a36ad59052f292ea2b0791ce90408b
---

# Adversarial Review: P1-00

> **Target resolution.** The review brief names branch `wp/P1-00-description` on base
> `origin/alpha/phase-1`. Neither exists. The actual work-package branch is
> `wp/P1-00-repo-bootstrap` (PR #1), based on `origin/main`; `alpha/phase-1` has never
> existed in this repository. Reviewed `git diff origin/main..origin/wp/P1-00-repo-bootstrap`
> at head `de3b36c` — 79 files, +5676 / −1044, 23 commits.
>
> **Prior review.** PR #2 (`claude/grid-alpha-adversarial-review-ikkjuk`) contains an earlier
> fresh-context review at `7787921` — CONDITIONAL, 6 blockers / 10 major / 9 minor. I read the
> diff cold and did not read that review before forming findings; I reference it below only
> where the branch's own artifacts cite it. Several findings here are recurrences of failure
> classes that review already named, which is itself a signal.

## Summary

- **Verdict: CONDITIONAL** — 3 blockers, 9 major, 11 minor.
  - **C1** `scripts/verify.sh` silently passes every check on a mistyped scope argument.
  - **C2** Both `CLAUDE.md` files still instruct the exact `SQLX_OFFLINE` setting that ADR-002
    identifies as the cause of a fixed blocker.
  - **C3** GPL-3.0 → MIT/Apache relicensing, out of stated scope, without the sign-off the
    branch's own evidence says it requires.
- **Confidence: High.** Every finding below was reproduced against a checkout, the committed
  artifacts, or the actual GitHub Actions logs. Where I could not verify something I say so.
- **Time spent:** ~55 minutes.

**This is good work with a specific, repeating failure mode.** The engineering is strong: the
guards fail closed, the SQLx gate is real and mutation-tested, the security remediation is
genuine, and the ADRs record their own uncomfortable parts rather than arguing them away. The
recurring defect is not in the code — it is that *the prose describing the code drifts behind
the code*, and this PR's central deliverable is prose. Three separate statements in the
completion record are checkably false at HEAD (M2). That is the same class as the prior
review's C4, which the implementer named explicitly ("I asserted what I intended the artifact
to say without re-reading it") — and it recurred twice after that acknowledgement.

## Authority Compliance

- [x] **All authority documents read and understood?** `docs/CLAUDE.md`,
      `docs/00-meta/authority-index.md`, all 7 ADRs in `docs/02-adr/`,
      `docs/01-work-packages/p1-00-work-package.md`, and the alpha-spec / final-build-spec
      sections its Authority field names (§1.1–1.6, §4.6, §8.1, §8.7–8.12, §9.2, §9.4, §9.5,
      §12.7–12.8, §14, §17.1, §17.3, Appendices B/C/D/E). Note: `docs/adr/` does not exist —
      the path is `docs/02-adr/`, ratified in ADR-001 D3 with an alias table.
- [ ] **Work package objective satisfied without scope expansion?** Objective satisfied;
      **scope expanded** — see M6 (non-goals amended by the implementer mid-PR), M7 (sqlx major
      upgrade), C3 (relicensing).
- [x] **Crate boundaries preserved?** Verified directly. `grid-domain` has an empty
      `[dependencies]`. `grid-persistence` depends only on `sqlx.workspace = true` and contains
      no business logic — two queries and a migration test. `grid-ffi` has no dependencies and
      an empty `src/generated/`. No Dart source exists anywhere in the tree, so no Flutter
      business logic is structurally possible. Each `lib.rs` restates its boundary in a doc
      comment.
- [x] **Generated files are source-derived, not hand-edited?** The two `.sqlx/query-*.json`
      files match the two `sqlx::query!` macros in `crates/persistence/src/lib.rs` byte-for-byte
      in their `query` field, carry generator-shaped `hash`/`describe` structure, and are gated
      by `cargo sqlx prepare --check --workspace -- --lib`. `.gitattributes` marks `.sqlx/**`
      and `Cargo.lock` `linguist-generated=true`. No FFI bindings exist yet.

## Architecture & Boundaries

- [x] **No business logic leaked into Flutter?** No Dart source exists. `app/pubspec.yaml` is
      the only Flutter artifact.
- [x] **No persistence logic leaked into domain/models?** `grid-domain` has zero dependencies
      and zero code beyond a doc comment.
- [x] **FFI contract changes versioned and synchronized?** No FFI surface is introduced.
      `flutter_rust_bridge` is now pinned `"=2.5.0"` on the Rust side to match the exact Dart
      pin in `app/pubspec.yaml` — a genuine improvement over the prior caret requirement, which
      could have floated the two sides apart in P1-01. `app/pubspec.lock` is absent and
      correctly reported as *blocked* (Flutter not installed) rather than fabricated.

## Data Integrity & Leakage

**Not applicable, and correctly so.** No feature, model, simulation, scoring, or evaluation
code exists in this diff; the only executable logic is two SQLite queries against a two-column
key/value table. There is no timestamp surface, no training or evaluation path, no provider
schema, and no identity matching to review. I checked for the failure modes rather than
assuming: `grep` across the diff finds no `as_of`, no lock-time handling, no feature
generation, no seeding, and no numeric code of any kind.

- [n/a] Timestamps (source, retrieval, feature, projection, lock)
- [n/a] Post-lock / future data in training or evaluation paths
- [n/a] Provider schema versioning — `docs/04-providers/{nflverse,cfbd}/README.md` are
  placeholders naming P1-03/P1-04 as owners; no schema is asserted
- [n/a] Identity matching

## Numerical & Statistical Correctness

**Not applicable.** No equations, no units crossing the FFI boundary, no NaN/inf paths, no
seeds, no golden files. `just mutants` and `just bench` are placeholders pointing at P1-06/P1-07.
`_typos.toml` whitelists exactly one term (`NMAE`, from alpha-spec §7.4) with a cited reason —
appropriately narrow for a config that could otherwise be used to silence real defects.

One typed-error observation that *is* in scope and is correct: `schema_meta_value` returns
`Ok(None)` for an absent key rather than an error, with a test asserting it
(`absent_key_is_none_not_an_error`), and `applied_schema_version` is derived from
`_sqlx_migrations` rather than a hand-written literal. An earlier revision seeded
`schema_version = '0001'`; removing it was the right call and the reasoning is recorded in the
migration file itself.

## Testing

- [x] **Unit tests for new logic?** Three `#[sqlx::test]` tests in `crates/persistence/src/lib.rs`.
- [x] **Failure tests for edge cases?** `absent_key_is_none_not_an_error`; the guard suite covers
      modify/delete/fail-closed cases.
- [~] **Integration tests for full pipeline segments?** No pipeline exists. The guard suite
      (`tests/guards/run.sh`, 12 cases against throwaway git repos, no network) is the nearest
      equivalent and is well built — it caught a real defect in the M3 fix on its first run.
- [n/a] Leakage tests / FFI round-trip tests — no such surface. `just test-ffi` runs
      `cargo test -p grid-ffi --features flutter-bridge-tests`; I verified the feature exists in
      `crates/ffi/Cargo.toml` so the recipe is not vacuously erroring, it is genuinely zero-test.
- [x] **No tests ignored/skipped without risk acceptance?** No `#[ignore]`, no `--skip`, no
      disabled assertions anywhere in the diff.
- [ ] **Gap:** two of six guards have no behaviour tests, including the one that is structurally
      load-bearing — see **M8**.

## Security & Dependencies

- [x] **No new production dependencies?** Correct — `sqlx` is a version bump of an existing
      workspace declaration; no crate was added. `Cargo.lock` holds 172 packages, all reachable
      from sqlx/sqlite/tokio; `nalgebra`, `statrs`, `sprs`, `rayon`, `flutter_rust_bridge` are
      declared in `[workspace.dependencies]` but unused, so they correctly do not enter the lock.
- [x] **License check passed?** `deny.toml` allows 8 permissive licenses with
      `unused-allowed-license = "allow"` and reasoning for the pre-allowed entries. CI log
      (job 96308538154): `advisories ok, bans ok, licenses ok, sources ok` over 172 crates.
- [x] **RUSTSEC-2023-0071 remediation independently verified.** `rsa` is absent from
      `Cargo.lock` on this branch; `sqlx` resolves to `0.9.0`. This is real, not asserted.
      (The upgrade's *process* is a separate problem — M7.)
- [x] **No secrets, credentials, or private data?** Verified by inspection of every added file.
      `.env` contains only `DATABASE_URL=sqlite:target/grid-dev.db` and `SQLX_OFFLINE=true`.
      The guard test fixtures assemble their fake credentials at runtime specifically so the
      scanner is not excluded from `tests/` — the right trade-off.
- [x] **No unsafe or destructive shell?** No `sudo`, no `rm -rf` outside `mktemp -d` cleanup
      under a `trap`. `.claude/settings.json` denies `sudo`, `su`, force-push, rebase, amend,
      `cargo publish`, `signtool`, and `rm -rf` of system paths, and — commendably — *removed*
      pipeline-shaped deny rules with a comment explaining that they read as protection while
      providing none. That is exactly the right instinct.
- [ ] **Supply chain:** build tooling is unpinned — see **M4**.

## Migrations & Rollback

- [x] **Append-only?** One added migration; `scripts/check-migrations.sh` enforces it, correctly
      scoped to paths present in the base so an unmerged draft can still be corrected.
- [x] **Blank database tested?** `migrations_apply_to_a_blank_database` asserts the applied
      version is derived from the migrator. `#[sqlx::test]` provisions a fresh database per test.
- [x] **Rollback documented?** Yes — revert the branch; the database lives in gitignored
      `target/` and is recreated by `just bootstrap`. Trivially true because no data exists.
- [ ] **But:** the migration exists only because the frozen `check-sqlx`/`test-rust` recipes
      cannot pass without it, and its future is explicitly undecided ("whether `schema_meta`
      remains the version mechanism is P1-02's call"). See **min-9**.

## Evidence & Verification

- [x] **Verification commands actually run?** I verified the CI claims against the GitHub API
      rather than taking them on trust. Run `32329932430` is real: `run_number` 10, `head_sha`
      `6478c24`, conclusion `success`, three jobs — `repository guards` 6s, `linux-smoke`
      16m48s, `windows-authoritative` 32m12s. Every duration in the manifest matches to the
      second. **Additionally**, run `32332121205` (run_number 11) is green on `de3b36c`, the
      actual PR head — so CI is green on the true final commit, which the evidence artifacts do
      not yet claim. The implementer under-claims here rather than over-claims.
- [x] **Evidence manifest valid?** `.ai/evidence/P1-00/manifest.json` parses; its sha256 is
      `fd46a0bb…3476b`, which matches the hash in the completion record exactly. The generator
      derives `verification_status` from recorded exit statuses instead of asserting it, and
      merges operator input from a committed file rather than accepting hand-edits — good design.
- [ ] **"All tests pass" claims cited and current?** **No.** Five separate claims in the
      manifest and PR body are checkably false against HEAD — see **M2**. And the mandated
      §8.12 field `human_approvals.required` reads `[]` while a sibling field lists four
      required sign-offs — see **M3**.

---

## Findings

### Critical (must fix before merge)

**C1. `scripts/verify.sh` fails open on any scope argument it does not recognise.**

- **Location:** `scripts/verify.sh:4-8`, `:37-39`
- **Authority violated:** alpha-spec §8.11 ("Bootstrap and verification commands … return
  non-zero on failure"; "may not … treat an unrun check as passing"); Appendix D; ADR-001 D5.
- **Reproduced:**
  ```
  $ bash scripts/verify.sh Full
  [verify] Starting verification scope: Full
  [verify] All checks passed.          # exit 0 — nothing ran
  $ bash scripts/verify.sh FULL   → exit 0, nothing ran
  $ bash scripts/verify.sh smoke  → exit 0, nothing ran
  ```
  `SCOPE="${1:-full}"` is compared against the lowercase literals `full`/`changed`. Anything
  else skips the entire `if` body and falls through to the unconditional
  `echo "[verify] All checks passed."`.
- **Why this is a blocker and not a nit:** the capitalised form is what every authority document
  in this repository shows. `CLAUDE.md`, `docs/CLAUDE.md`, `docs/00-meta/authority-index.md` and
  the work package all print the canonical command as `-Scope Full`. The natural transliteration
  to bash — `./scripts/verify.sh Full` — is precisely the silent no-op. A human or agent
  following the repository's own documentation gets a green "All checks passed" having verified
  nothing. `verify.ps1` is safe (`[ValidateSet]` rejects bad input); only the bash side is
  exposed, and the bash side is the one a Linux agent will reach for.
- **Fix required:** normalise case and reject unknown scopes:
  ```bash
  SCOPE="$(tr '[:upper:]' '[:lower:]' <<< "${1:-full}")"
  case "$SCOPE" in full|changed) ;; *) echo "[verify] unknown scope: $1" >&2; exit 2 ;; esac
  ```
  Add a guard case to `tests/guards/run.sh`. Note this touches a frozen file (D5) — but per
  ADR-005's own three conditions it *strengthens* the gate, which is the qualifying direction.

---

**C2. Both `CLAUDE.md` files still instruct the `SQLX_OFFLINE` setting that caused blocker C2.**

- **Location:** `CLAUDE.md:30-31` (root); `docs/CLAUDE.md:52-53`
- **Authority violated:** alpha-spec §8.7.1 (the root `CLAUDE.md` carries canonical bootstrap
  and verification commands); §1.5 (an ADR at level 3 and the instruction files must not
  contradict each other on a resolved decision).
- **Issue:** both files read:
  > Requires `DATABASE_URL=sqlite:target/grid-dev.db` and **`SQLX_OFFLINE` unset** for local
  > work; see `docs/02-adr/002-sqlx-offline-cache.md`. `scripts/check-env-contract.sh` verifies this.

  ADR-002 was corrected during this same PR (blocker C2, commit `3ad054b`) to say the exact
  opposite, with measurements: with `DATABASE_URL` set and `SQLX_OFFLINE` unset,
  `cargo check -p grid-persistence` exits **101** on a fresh clone because the `query!` macro
  attempts a live connection and never consults the committed cache. `.env` now sets
  `SQLX_OFFLINE=true` and the file's own comment calls it "load-bearing, not convenience."

  The fix landed in `.env` and ADR-002. It was never propagated to either instruction file.
  The two documents a fresh session reads *first* — which is this work package's entire stated
  purpose — now tell that session to reproduce the blocker.
- **Second defect in the same sentence:** "`scripts/check-env-contract.sh` verifies this" is
  false. Reading `scripts/check-env-contract.sh:70-78`, the script fails only when
  `SQLX_OFFLINE=true` *and* `.sqlx/` is absent. It has no rule requiring `SQLX_OFFLINE` to be
  unset, and passes happily with it set. The cited enforcement does not exist.
- **Fix required:** correct both files to `SQLX_OFFLINE=true` (supplied by the committed `.env`),
  and either drop the enforcement claim or make `check-env-contract.sh` actually assert it.

---

**C3. Project relicensed from GPL-3.0 to MIT OR Apache-2.0 without the sign-off the branch's own evidence says is required.**

- **Location:** commit `63e9e7b`; `LICENSE` deleted (−674), `LICENSE-MIT` + `LICENSE-APACHE` added
- **Authority violated:** alpha-spec §1.4 (licensing is the Data/Licensing owner's authority);
  §8.9 step 4 and §1.5 ("Claude must stop and create a decision request when it encounters a
  higher-authority conflict, undocumented provider behavior, destructive migration ambiguity,
  missing numerical specification, **license uncertainty**, or a requirement that can only be
  satisfied by weakening a test or safety control" — alpha-spec.md:1287); §14.2.
- **Facts:**
  - `LICENSE` (GPL-3.0) was added by the repository owner in his own commit `f71a241`
    ("Add GNU GPL v3 license", Ethan Nelson, 2026-08-19) — a deliberate act, not a template
    artifact. The `license = "MIT OR Apache-2.0"` line in `Cargo.toml` arrived in the skeleton
    commit and is the value a `cargo new`-style scaffold produces. The conflict was resolved in
    favour of the scaffold-shaped value and against the human's explicit act.
  - P1-00's Scope, acceptance criteria and non-goals mention neither licensing nor `LICENSE`.
    ADR-001 D4 concedes this in terms: *"This was out of the work package's stated scope … the
    change deletes a file the repository owner added in their own commit."*
  - The branch's own evidence says the required approval was **not** obtained.
    `manifest.json` → `human_approvals.role_scoped_signoffs_obtained: []`, with
    `role_scoped_signoffs_required` listing *"Data/Licensing owner: the D4 licence change, which
    was outside the work package's stated scope."* The PR body repeats it.
  - The only record of the authorizing ruling is the implementer's own prose in ADR-001 D4 and
    the commit message ("Owner ruled `Cargo.toml` correct"). There is no owner-authored artifact.
- **Why blocking:** relicensing is the one change in this diff that is not cleanly reversible in
  the real world. Once published under a permissive license, third parties may rely on it;
  re-imposing copyleft afterwards requires the consent of every copyright holder. A change with
  that property, explicitly outside scope, resting on an unrecorded verbal ruling, is not
  something a merge reviewer should absorb inside a bootstrap PR. The §1.5 stop condition names
  "license uncertainty" by name.
- **Fix required:** revert D4 from this branch (it touches only `LICENSE*`; ADR-001 states
  correctly that nothing else depends on it) and land it as its own change with the
  Data/Licensing owner's approval recorded in an artifact they authored. If the owner prefers to
  keep it here, that decision needs to be recorded by the owner on PR #1, not by the implementer
  in an ADR.

---

### Major (should fix; a human can accept the risk)

**M1. The merge-authoritative CI job's green includes two governance gates that did not run.**

- **Location:** `.github/workflows/alpha-ci.yml:78-83` (`linux-smoke`), `:98-103`
  (`windows-authoritative`) — neither sets `fetch-depth`
- **Evidence — actual CI log, run 32329932430 job 96308538154, step `just verify`:**
  ```
  ./scripts/check-migrations.sh
  check-migrations: SKIP (NOT VERIFIED) base ref 'origin/main' unresolvable, so append-only could not be checked.
  ./scripts/check-secrets.sh
  check-secrets: OK no secret patterns found (FULL tracked tree (base 'origin/main' unresolvable))
  ./scripts/check-traceability.sh
  check-traceability: SKIP (NOT VERIFIED) base ref 'origin/main' unresolvable, so no change set could be derived.
  ```
  `actions/checkout@v4` defaults to `fetch-depth: 1`, so `origin/main` does not exist in either
  build job. Both scripts print a NOT-VERIFIED notice to stderr and `exit 0`.
- **Risk:** ADR-007's stated benefit — *"secret scanning, traceability and doctests run wherever
  `just verify` runs"* — holds for secret scanning (which degrades to a full-tree scan, a sound
  fallback) but not for the other two. `windows-authoritative` is the §8.11 merge gate; its green
  does not include append-only migration verification or traceability. The dedicated `guards`
  job does set `fetch-depth: 0` and does run them properly, so coverage exists — but the
  authoritative job's green means less than it appears to, and this is exactly the "a gate that
  passes vacuously is a gate nobody has tested" argument ADR-002 makes against itself.
- **Fix:** add `with: fetch-depth: 0` to both build jobs. Consider making the SKIP path exit
  non-zero when `${CI:-}` is set, so it can never be green in CI again.

---

**M2. Five checkably-false statements in the completion record and evidence manifest at HEAD.**

- **Location:** `.ai/evidence/P1-00/PR-BODY.md`, `.ai/evidence/P1-00/manifest.json`,
  `.ai/evidence/P1-00/verification-input.json`
- **Authority violated:** `CLAUDE.md` Prohibited — *"Claiming a command passed when it was not
  run against the final commit"*; alpha-spec §8.11 ("verification command, exit status, **test
  summary**"); §1.6 "evidence over assertion"; Appendix D #1.

  | Claim | Location | Actual state at `de3b36c` |
  |---|---|---|
  | "Public Rust API: two items … `SCHEMA_VERSION_KEY` and `schema_meta_value()`" | PR-BODY, Architecture impact | `SCHEMA_VERSION_KEY` was **deleted** in `a37e49f`. The public API is `schema_meta_value` and `applied_schema_version`. |
  | "Across all three implementations, **exactly one command changed**" | PR-BODY, frozen-recipe audit | False since `6478c24`. ADR-007 added `test-doc` and `check-guards` to all three. Three recipes changed, not one. |
  | "`cargo test -p grid-persistence --lib` → 2 passed" / "cargo nextest: 2 tests run" | PR-BODY, manifest, verification-input | There are **three** tests. `schema_meta_round_trips` was added in `a37e49f`. |
  | "All **7** frozen recipes passed" | manifest `verification.commands[0].summary` | Nine recipes at HEAD. |
  | "justfile and scripts/verify.sh agree on **8** steps" | manifest, verification-input | CI log prints `agree on 16 verification step(s)`. |
  | "the remaining major and minor findings have **NOT yet been triaged**" | manifest `known_limitations[8]` | Contradicted by `a37e49f` ("triage the remaining adversarial-review findings") and by the PR body's own "TRIAGE COMPLETE." |

- **Root cause, and why it matters more than the individual errors:** the canonical local
  `just verify` recorded in the manifest ran against `18c8fb7` — three commits before HEAD, and
  before both the library change (`a37e49f`) and the verify-recipe change (`6478c24`). The
  manifest's `commit` field says `6478c24`, a commit the recorded local run never saw. CI does
  cover the real head (verified: run 11 green on `de3b36c`), so the *facts* are fine; the
  *record* is not. This is the third recurrence of the failure the prior review logged as C4 and
  the implementer named explicitly in commit `4de62ec`: asserting what the artifact was intended
  to say without re-reading it.
- **Fix:** regenerate the manifest and rewrite the completion record against `de3b36c`, citing
  run `32332121205` (green on the actual head) rather than run 10. Then diff each factual claim
  against the file it describes before committing.

---

**M3. The §8.12-mandated `human_approvals.required` field says no approvals are required.**

- **Location:** `.ai/evidence/P1-00/manifest.json` → `human_approvals`
- **Issue:** §8.12 requires the manifest record "human approvals required and obtained". The
  manifest emits `"required": []` and `"obtained": []` — while the sibling
  `role_scoped_signoffs_required` immediately below lists four outstanding approvals
  (Architecture, Security/Release, Data/Licensing, Merge reviewer). The prose note explains the
  distinction well, but an automated consumer, or a reviewer skimming the mandated field name,
  reads "no approvals required" for a PR whose own prose says four are outstanding. The prior
  review's C5 was about exactly this conflation; the fix separated the *prose* but left the
  mandated field empty and misleading.
- **Fix:** populate `required` with the four role-scoped sign-offs (and the D1–D6 rulings under
  `obtained`), keeping the explanatory note. `generate-evidence-manifest.sh` already merges these
  from `verification-input.json`, so this is a data change, not a code change.

---

**M4. Build tooling is unpinned, in a repository whose thesis is pinned reproducibility.**

- **Location:** `justfile:10-16` (`bootstrap`); `.github/workflows/alpha-ci.yml:71-77`, `:91-97`
- **Authority violated:** alpha-spec §8.11 — *"Rust, Flutter/Dart, FRB, SQLx, XGBoost/native
  artifacts, and package dependency versions are pinned through committed toolchain files and
  lockfiles."*
- **Issue:** every tool is installed as `cargo install <tool> --locked` with no `--version`.
  `--locked` pins the tool's *own* dependencies, not the tool. `cargo-nextest`, `sqlx-cli`,
  `cargo-deny`, `cargo-audit` and `typos-cli` all float to whatever is latest on the day CI runs.
  `ai-toolchain.lock` pins the *agent harness*, not the build toolchain, and there is no
  `toolchains/native-dependencies.lock`.
- **Concrete risk, in this diff:** `deny.toml:8-12` states its `unmaintained`/`unsound` keys are
  explicit *"so that a version bump of the tool cannot silently relax the policy."* The tool
  version is unpinned, so a cargo-deny release that renames, deprecates or re-defaults those keys
  does exactly what the comment says it prevents. (`unsound` in particular has been reshaped
  across cargo-deny config generations; I could not run `cargo deny` in this environment to
  confirm the key is still honoured rather than ignored — worth checking.) The same class of
  problem already bit this branch once: ADR-003 records an unpinned `sqlx-cli` resolving to 0.9
  against a 0.8 library.
- **Fix:** pin each tool with `--version` in the justfile and CI, and record the versions in
  `ai-toolchain.lock` or a new `toolchains/dev-tools.lock`. This also removes the ~13 min /
  ~24 min install steps' nondeterminism.

---

**M5. The offline-compile guarantee — the reason `.env` is committed — is never exercised by CI.**

- **Location:** `.github/workflows/alpha-ci.yml:33-35` (`env: DATABASE_URL`), `:85-86`
  (`just bootstrap` before `just verify`)
- **Evidence — CI log, job 96308538154:**
  `check-env-contract: OK SQLX_OFFLINE=<unset>, .sqlx/ present`
- **Issue:** blocker C2 was *"fresh clone does not compile … the committed cache is never
  consulted (exit 101)"*. The fix was `SQLX_OFFLINE=true` in `.env`, and ADR-002 calls
  compiling with no database the property the whole design exists to provide. But every CI job
  sets `DATABASE_URL` at workflow scope and runs `just bootstrap` (which creates a live SQLite
  file) *before* `just verify`, with `SQLX_OFFLINE` unset. So `query!` always resolves against a
  live database in CI. **No job ever compiles the way a fresh clone does.** The blocker can
  regress and CI will stay green.
- **Fix:** add one cheap step before bootstrap:
  ```yaml
  - name: Fresh clone compiles offline (C2 regression gate)
    run: env SQLX_OFFLINE=true -u DATABASE_URL cargo check --workspace
  ```

---

**M6. The work package's non-goals were rewritten by the implementer, inside the PR they constrain.**

- **Location:** `docs/01-work-packages/p1-00-work-package.md` — "Amendment note — 2026-08-20";
  commit `f5d0d8b`
- **Authority violated:** alpha-spec §1.5 (the approved work package is level-5 authority);
  §8.8/§8.9 step 4 (human gate).
- **Issue:** the original non-goal read *"No SQLx migrations beyond the empty `migrations/`
  directory (schema creation is P1-02)."* The branch adds `migrations/0001_schema_meta.sql`, a
  functioning `grid-persistence` with two queries, a committed `.sqlx` cache and three tests —
  then amends the non-goal, in the same PR, to permit it. Also moved ADR-002 from "resolved in
  P1-02" to "resolved in P1-00".
- **In fairness:** ADR-001 confronts this squarely and concedes it — *"A work package should not
  be the artifact that authorizes exceeding itself"* — and prescribes a different mechanism for
  P1-01 onward. The amendment is a dated separate section rather than an edit to the original
  text. The technical justification is real and I verified it: `cargo nextest run --workspace`
  does exit 4 on an empty workspace, and `cargo sqlx prepare --check` cannot succeed against a
  virtual root with no crates.
- **Residual risk:** none of that changes the fact that the level-5 authority document was
  rewritten by the constrained party, and the only record of the authorizing ruling is
  implementer-authored prose. A later reader sees amended scope presented as scope.
- **Fix:** have the owner record the amendment themselves on PR #1, or split the SQLx bootstrap
  into P1-00b. The self-diagnosis in ADR-001 is the right analysis; it should be applied to this
  package, not only inherited by the next one.

---

**M7. A semver-breaking dependency upgrade is folded into a bootstrap PR.**

- **Location:** `Cargo.toml` `sqlx = "0.8"` → `"0.9"`; ADR-003
- **Authority violated:** alpha-spec §14.2 — *"Major version upgrades are separate work packages
  and cannot be hidden inside feature changes"*; §8.11 — *"Claude may not perform an implicit
  toolchain or dependency upgrade while implementing an unrelated package"*; root `CLAUDE.md` —
  *"Major upgrades are their own work package, never folded into feature work."*
- **In fairness:** this is the best-argued deviation in the branch. The advisory is real, the
  remediation is verified (`rsa` absent from `Cargo.lock`; `cargo deny` and `cargo audit` green
  in CI), feature narrowing was *tested* rather than assumed, and ADR-003 explicitly **withdraws**
  its earlier reasoning that disclosure satisfies §14.2 — reframing it as an owner deviation.
  That withdrawal is exactly the right move and worth saying so.
- **Residual risk:** §14.2's dependency checklist (§14.2/alpha-spec:1927) requires a maintenance
  assessment and a **Windows compatibility check** for dependency changes. ADR-003 records
  neither; Windows compatibility is now demonstrated empirically by the green
  `windows-authoritative` job, but that is not the same as the assessment the spec asks for. And
  as with M6, the authorizing ruling exists only as implementer prose.
- **Fix:** either extract to a one-line P1-00b, or have the owner record the §14.2 deviation
  directly and add the missing checklist items to ADR-003.

---

**M8. Two of six guards have no behaviour tests — including the structurally load-bearing one.**

- **Location:** `tests/guards/run.sh`
- **Issue:** the suite covers `check-secrets` (4 cases), `check-migrations` (4),
  `check-traceability` (2) and `check-authority-sync` (2). It does **not** cover
  `check-verify-parity.sh` or `check-env-contract.sh`.
  - `check-verify-parity.sh` is the *only* thing preventing the justfile and `verify.sh` from
    drifting. ADR-001 D5 forbids unifying them, so this guard is load-bearing by design. Its own
    logic is the most fragile in the set — two `awk` recipe parsers, a `grep`-based command
    extractor with a hand-maintained keyword exclusion list, and `tr -d '\r'` to cope with the
    CRLF justfile. A parser that silently extracts zero commands from one side would compare two
    equal-but-empty lists. (It does guard the empty-`targets` case; it does not guard empty
    `just_cmds`/`sh_cmds`.)
  - The SKIP-exit-0 path of `check-migrations`/`check-traceability` (M1) is untested and is the
    same fail-open class as M1-the-original-finding, which is the stated reason this suite exists.
- **Fix:** add cases for both guards, plus a case asserting that a repo with no resolvable base
  behaves as intended, plus C1's scope-argument case.

---

**M9. The permission profile has already drifted from the justfile it enumerates.**

- **Location:** `.claude/settings.json` `permissions.allow`
- **Issue:** the allow list names verify recipes individually — `just check-fmt`,
  `just check-lint`, `just check-sqlx`, `just test-rust`, `just test-ffi`, `just audit`,
  `just check-typos` — but is missing `just test-doc` and `just check-guards`, both added by
  ADR-007 in the same PR. `just verify` is allowed as a whole, so this is not a functional break
  today; it is a security-boundary file that was not updated when the thing it enumerates changed,
  three commits before HEAD. The file's own header calls changes to it "a SECURITY BOUNDARY", and
  the Security/Release owner sign-off it requires is not obtained.
- **Fix:** add the two entries; consider `Bash(just check-*)`/`Bash(just test-*)` prefixes so the
  list cannot silently fall behind again.

---

### Minor (suggestions, non-blocking)

1. **`docs/05-sessions/` vs `docs/06-sessions/` collision — this file is in the contested
   directory.** `docs/00-meta/authority-index.md` declares *"`docs/05-sessions/` is **not** a
   valid path — 05 is model-specs"*, and `docs/06-sessions/README.md` says the prior review
   "should move here when that branch merges." The review brief I was given mandates
   `docs/05-sessions/review-P1-00-adversarial.md`, so that is where this file is written, as
   instructed. **Flagging the conflict rather than silently resolving it:** PR #2 already places
   a file at that exact path. If both merge, `docs/` will contain a directory the authority index
   calls invalid, holding two files. *Suggestion:* reconcile before merging either — move both to
   `docs/06-sessions/`, and update the review brief template so the next reviewer is not sent to
   an invalid path.

2. **A committed `.env` normalises a habit worth not normalising.** The file is credential-free
   today and the reasoning is written into it honestly, including the trade-off. But the
   mechanism does not require a tracked `.env`: `.cargo/config.toml` with
   `[env] SQLX_OFFLINE = "true"` is the idiomatic committed way to set a build-time variable, is
   scoped to cargo, and does not teach the next contributor that env files belong in git.

3. **Secret patterns are narrower than the acceptance criterion's literal text.** The criterion
   says "patterns matching `ghp_`, `sk-ant-`, …"; `scripts/secret-patterns.txt` requires
   `gh[pousr]_` + ≥36 chars and `sk-ant-` + ≥16. Sensible against false positives, but a
   truncated, redacted-but-recoverable, or short test token passes. Consider a lower-confidence
   warn tier alongside the deny tier.

4. **The secret scanner has a permanent two-file blind spot.** `check-secrets.sh` and
   `secret-patterns.txt` are excluded from every scan path so the detector cannot match its own
   definitions. Reasonable, but it means those two files are the one place in the repository
   where a secret is never looked for. A per-line marker (`# nosecret`) would let the files be
   scanned while still suppressing the pattern definitions.

5. **`cargo audit` fetches the advisory DB from the network on every CI run** (visible in the
   log: `Fetching advisory database from https://github.com/RustSec/advisory-db.git`). That is
   correct behaviour for an advisory scanner, but it makes `just verify` network-dependent, which
   sits awkwardly beside §8.11's "clean checkout, offline" framing and the deliberate avoidance
   of third-party actions. Worth stating explicitly as an accepted network dependency.

6. **`just bootstrap` re-installs tools CI already installed**, plus `hyperfine` and
   `cargo-mutants` which no CI step uses. This is most of the 16m48s Linux and 32m12s Windows job
   times. Combined with `concurrency: cancel-in-progress`, it is the mechanical cause of the merge
   gate being unobservable for 9 of 11 runs. A `bootstrap-ci` recipe, or `--force`-free
   idempotent installs, would cut it sharply.

7. **`.gitignore:7`** — `/target/grid-dev.db*` is redundant under `/target/` on line 2.

8. **`scripts/bootstrap-repo.sh` creates `data/`**, which `.gitignore` excludes, so a fresh clone
   and a bootstrapped tree differ. Harmless; worth a comment.

9. **`migrations/0001_schema_meta.sql` creates a table nothing reads**, whose future is
   explicitly undecided ("whether `schema_meta` remains the version mechanism is P1-02's call").
   It exists to make two frozen recipes pass. That is defensible as scaffolding, but append-only
   enforcement means P1-02 inherits it permanently. Consider recording in ADR-002 that removing
   it via a forward migration in P1-02 is pre-approved, so nobody has to litigate it later.

10. **`template-work-package.md` adds `## Security and licensing considerations`**, which
    Appendix B does not contain, against an acceptance criterion reading "follows the exact
    Appendix B structure." The addition is justified by §8.8's field list (which does name
    "security/licensing considerations"), so this is a spec-internal inconsistency rather than an
    implementer error — but the acceptance criterion says "exact" and the template is not exact.
    Worth one line in ADR-001 rather than leaving a reviewer to rediscover it.

11. **CRLF retained on `Cargo.toml`, `justfile`, `ai-toolchain.lock`, `rust-toolchain.toml`.**
    ADR-004's narrow-scope reasoning is sound (a blanket `text=auto` would destroy the review
    trail). But `check-verify-parity.sh` already has to `tr -d '\r'` to function, and the next
    tool that parses the justfile will need the same workaround. The P1-11 deferral is fine;
    the growing workaround count is worth tracking.

---

## What I checked and could not fault

Recording these so the next reviewer does not re-litigate them, and because a review that only
lists defects misrepresents the diff:

- **Authority order.** The §1.5 inversion in the pre-existing `docs/CLAUDE.md` and authority
  index was real and is correctly fixed, stated identically in three places, with the correction
  flagged in-place rather than silently rewritten.
- **Crate boundaries.** Verified per-crate from the manifests, not from the doc comments.
- **`.sqlx` cache integrity.** Both cached queries correspond to the two `query!` macros; the
  drift gate is real (mutation-tested per ADR-005, and it runs in CI).
- **RUSTSEC-2023-0071.** `rsa` absent from `Cargo.lock`; 172 crates; `cargo deny` reports
  `advisories ok, bans ok, licenses ok, sources ok` in the CI log.
- **Guard fail-closed rewrite.** `check-secrets.sh` genuinely fails closed now; the CI log shows
  all 12 guard cases passing, including the M1 regression case (committed secret, no base ref).
- **Migration hygiene.** Append-only, blank-database tested, no hand-written schema version.
- **CI claims.** Run 32329932430 verified against the GitHub API — job names, conclusions and all
  three durations match the manifest exactly. Run 32332121205 is additionally green on `de3b36c`.
- **`.claude/settings.json` pipeline-rule removal.** Deleting deny rules that "read as protection
  while providing none" is the correct instinct and correctly explained.

---

## Questions for Implementer

1. **C3 / D4:** what is the artifact recording the Data/Licensing owner's ruling on the license
   change? If it exists only as an in-session exchange, are you willing to revert D4 and land it
   separately, as ADR-001 D4 itself says a reviewer may ask for?
2. **C2:** was the `SQLX_OFFLINE` correction deliberately left out of both `CLAUDE.md` files, or
   was it missed? If missed — what would have caught it? (`check-authority-sync.sh` compares the
   two `alpha-spec.md` copies but nothing cross-checks the instruction files against the ADRs
   they cite.)
3. **M1:** was the `fetch-depth` interaction known when ADR-007 moved the guards into
   `just verify`? The ADR anticipates that "a shallow clone degrades them" but asserts they
   "fail closed or report an explicit unverified state rather than passing silently" — in CI, an
   exit 0 with a stderr line *is* passing silently. Is a non-zero exit under `${CI}` acceptable?
4. **M5:** is there a reason not to add an offline `cargo check` step? It is the only regression
   gate for the blocker that cost the most debugging time on this branch.
5. **M4:** `deny.toml` claims explicit keys prevent a tool version bump from relaxing policy,
   while the tool is installed unpinned. Which half is the intent — pin the tool, or drop the
   claim?
6. **M2:** the canonical local `just verify` in the manifest ran against `18c8fb7`, three
   commits before HEAD. Given CI is now green on `de3b36c`, would you regenerate the evidence
   against the real head and cite run 32332121205 instead of run 10?
7. **`deny.toml`:** did `cargo deny` emit any warning about the `unsound` key on the version CI
   installed? I could not run it here.
8. **`schema_meta`:** is it intended to survive P1-02, or is it scaffolding that P1-02 should
   drop via a forward migration?

## Follow-up Work Identified

- **P1-01 (blocking):** write `.claude/agents/` definitions before more agent-driven packages
  run. ADR-006 defers them with a good argument, but also concedes "the specialized roles §8.10
  describes are how §9.2 intends quality to be maintained, and until they exist that depends on
  whoever is driving." Three of this review's findings are process findings; that is what the
  gap looks like in practice.
- **P1-01:** add a guard that cross-checks instruction files (`CLAUDE.md`, `docs/CLAUDE.md`,
  authority index) against the ADRs they cite. C2 is a doc-vs-ADR contradiction that no existing
  guard could catch, and this repository's entire premise is that those files are trustworthy.
- **P1-01:** extend `tests/guards/run.sh` to all six guards plus `verify.sh` scope handling.
- **P1-02:** decide `schema_meta`'s fate; record whether removing it by forward migration is
  pre-approved.
- **P1-11:** implement `-Scope Changed` (currently an alias for Full in bash, and a validated-
  but-unimplemented value in PowerShell); repo-wide line-ending normalization; CI dependency
  caching (see min-6 — this is a merge-gate observability problem, not a cosmetic one);
  `deny.toml` policy finalization; `toolchains/native-dependencies.lock`.
- **Governance:** reconcile `docs/05-sessions/` vs `docs/06-sessions/` before PR #1 and PR #2
  both merge, and fix the review brief that sends reviewers to the invalid path.
- **Human, pre-merge:** the four role-scoped sign-offs the branch itself lists as outstanding —
  Architecture (authority order, crate boundaries, P1-02→P1-00 rework), Security/Release
  (`.claude/settings.json`), Data/Licensing (C3), Merge reviewer (final diff). Plus branch
  protection on `main`, an explicit P1-00 non-goal.
