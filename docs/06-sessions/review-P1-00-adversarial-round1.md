---
reviewer: fresh-context-adversarial
date: 2026-08-20
work_package: P1-00
branch: wp/P1-00-repo-bootstrap
base: origin/main
head_reviewed: 7787921edaddab35d9691122cead2c5a152f7d88
pull_request: Seismic-Fate/GRID-Alpha#1
---

# Adversarial Review: P1-00

## Summary

- **Verdict: CONDITIONAL** — six blockers (C1–C6) must be resolved before merge.
- **Confidence: High** for C1–C4 and M1 (reproduced locally against branch HEAD with the real
  toolchain; **C1 has since been confirmed independently by CI** — `linux-smoke` failed at its
  `Bootstrap` step with the exact error predicted below); Medium for C5–C6 and the governance
  findings (they turn on approval records this reviewer cannot see).
- **Time spent:** ~45 minutes.

The package is unusually well documented. Five ADRs, a real evidence manifest, six executable
guard scripts, honest known-limitations lists, and templates that exceed Appendix B. That
quality is what makes the findings below worth taking seriously rather than dismissing: the
defects are not sloppiness, they are places where the narrative asserts something the artifacts
do not support.

Two blockers are functional and reproducible: **`just bootstrap` cannot run on a fresh
checkout**, and **a fresh clone cannot compile under the configuration the package itself
documents**. Both are load-bearing for the package's central acceptance criterion (`just verify`
passes), and both invalidate the claim `just verify` was proven "from a clean 76-file checkout
with no `target/`". Two more are evidence-integrity: the manifest attests to `f5d0d8b` while the
branch head is `7787921`, and the PR body states the manifest reads `clean` when the committed
manifest reads `uncommitted: 3 path(s)`. The last two are governance: every blocking decision
rests on a self-attested owner ruling that the manifest simultaneously lists as *not obtained*,
and the project's outbound licence was changed from GPL-3.0 to MIT OR Apache-2.0.

### Verification this reviewer actually ran

| Command | Result |
|---|---|
| `git diff origin/main..origin/wp/P1-00-repo-bootstrap` (73 files, +4787/−1042) | read in full |
| `bash -n scripts/*.sh` (9 scripts) | all parse |
| `cargo fmt --all -- --check` | exit 0 |
| `SQLX_OFFLINE=true cargo clippy --all-targets --all-features -- -D warnings` | exit 0 |
| `SQLX_OFFLINE=true cargo test -p grid-persistence` | 2 passed, 0 failed |
| `cargo check -p grid-persistence` with `.env` DATABASE_URL, no db file | **exit 101** (finding C2) |
| `sqlx database create` (sqlx-cli 0.9.0) at HEAD, no `target/` | **exit 1, SQLite code 14** (finding C1) |
| `./scripts/check-{authority-sync,verify-parity,migrations,secrets}.sh origin/main` | all exit 0 |
| `./scripts/check-secrets.sh no-such-ref` (fallback path) | **exit 0, scanned nothing** (finding M1) |
| `sha256sum` of committed `manifest.json` vs PR-body claim | **matches** |
| `diff <(git show origin/main:scripts/verify.sh \| tr -d '\r') scripts/verify.sh` | one line — ADR-004's claim **holds** |
| `cargo metadata` licence census (172 crates) vs `deny.toml` allowlist | every crate satisfiable |
| GitHub check runs on PR #1 | guards green; **`linux-smoke` FAILED** at `just bootstrap` (03:20:49Z); `windows-authoritative` still in progress |

`cargo deny check`, `cargo audit` and `typos` were not run (tools unavailable in this
environment); the RUSTSEC-2023-0071 remediation was confirmed indirectly — `rsa` is absent
from the committed `Cargo.lock`, and sqlx is pinned at 0.9.0 across all seven sub-crates.

## Authority Compliance

- [ ] **Work package objective satisfied without scope expansion?** — No. The objective is met,
      but the package delivers a migration, a public persistence API, a committed `.sqlx` cache,
      a dependency major bump, a licence change and three unplanned guard scripts, all against
      explicit non-goals or follow-ups. The work package file was edited in this same PR to
      legalize them (see M5).
- [x] **All authority documents read and understood?** — Yes, and the reading is demonstrably
      close: the authority-order inversion in the starting `docs/CLAUDE.md` and authority index
      was correctly identified and fixed to match `alpha-spec.md` §1.5 (`final-build-spec.md`
      first). Independently verified against §1.5.
- [x] **Crate boundaries preserved (Rust owns state, Flutter owns presentation)?** — Yes. Twelve
      crates match `Cargo.toml`, `docs/CLAUDE.md` and ADR-001. Each `lib.rs` restates its
      boundary. No Flutter code was added at all.
- [x] **Generated files (FFI, SQLx) are source-derived, not hand-edited?** — Yes. The single
      `.sqlx` entry's `hash` field matches its filename; `crates/ffi/src/generated/` is an empty
      placeholder with a never-hand-edit notice; `.gitattributes` marks both as generated.

## Architecture & Boundaries

- [x] **No business logic leaked into Flutter?** — Yes; no Dart source exists.
- [ ] **No persistence logic leaked into domain/models?** — Yes for domain/models (all empty),
      but the inverse question is the live one: `grid-persistence` now exposes
      `schema_meta_value()`, an application-shaped read accessor, in a package whose non-goals
      forbade schema work entirely. Boundary is not violated; scope is.
- [ ] **FFI contract changes versioned and synchronized?** — No FFI contract exists yet, but the
      *versions* are already out of sync: Rust declares `flutter_rust_bridge = "2.5.0"` (a caret
      range, ≥2.5.0 <3.0.0) while `app/pubspec.yaml` pins `flutter_rust_bridge: 2.5.0` exactly.
      No `app/pubspec.lock`, no `toolchains/flutter.version`. See M6.

## Data Integrity & Leakage

Not applicable in substance — this package introduces no features, no training path, no
projections and no provider ingestion. Recorded for completeness:

- [x] All timestamps present and correct? — n/a; no temporal data model exists yet.
- [x] No post-lock or future data in training/evaluation paths? — n/a; no such path exists.
- [x] Provider schema changes create new contract versions? — n/a; the provider README stubs
      correctly state the rule (`docs/04-providers/nflverse/README.md`) and defer to P1-03.
- [x] Identity matching conservative? — n/a; deferred to P1-04, rule restated in `docs/CLAUDE.md`.

The one substantive observation: the provider stubs and `template-provider-contract.md` state
the point-in-time and no-silent-fallback rules clearly and in the right places. This is good
groundwork for P1-03/P1-05.

## Numerical & Statistical Correctness

Not applicable in substance — no equations, no seeds, no numerics. `template-model-spec.md` is
strong: it demands target/units/support, full equations with symbols defined, priors, invariant
handling ("never silently clamp"), a seed policy tied to the prediction version, absolute-vs-
relative tolerances, and hand-verifiable reference examples. That template is one of the better
artifacts in the diff.

- [x] Equations match model spec? — n/a
- [x] Units explicit across the Rust/Dart boundary? — n/a
- [x] NaN/inf/non-convergence are typed failures? — n/a; the one error path that does exist
      (`schema_meta_value` returning `Ok(None)` for an absent key rather than an error) is
      correctly typed and has a dedicated test.
- [x] Seeds deterministic and recorded? — n/a
- [x] Golden tests with tolerance-bounded outputs? — n/a (no golden files; correctly stated as
      N/A in the completion record)

## Testing

- [x] **Unit tests for new logic?** — Two, both meaningful:
      `migrations_apply_to_a_blank_database` and `absent_key_is_none_not_an_error`. Verified
      passing locally (`2 passed; 0 failed`).
- [ ] **Failure tests for edge cases?** — Partial. The guard scripts are the largest body of new
      logic in the diff and have **no committed tests at all**. The manifest claims they were
      "behaviour-tested against synthetic git ranges", but nothing in the repository re-runs
      those cases, so the next change to a guard has no regression net. M1 is exactly the class
      of defect a committed guard-script test suite would have caught.
- [ ] **Integration tests for full pipeline segments?** — n/a, but see M8: no CI job has ever
      completed, so the end-to-end path is unproven anywhere.
- [x] **Leakage tests?** — n/a for this package.
- [ ] **FFI round-trip tests for numeric precision?** — The `test-ffi` recipe runs
      `cargo test -p grid-ffi --features flutter-bridge-tests` against a crate with zero tests
      and an empty feature. It passes vacuously today; that is disclosed and deferred to P1-01,
      which is reasonable — but note the same "a gate that passes vacuously is a gate nobody has
      tested" argument ADR-002 used to reject a no-op `check-sqlx` applies unchanged here, and
      was not applied.
- [x] **No tests ignored/skipped without risk acceptance?** — Correct; none are ignored.

## Security & Dependencies

- [x] **No new production dependencies without justification?** — No crate was added. `sqlx` was
      bumped 0.8 → 0.9 (see M4 for the governance problem, not the technical one).
- [x] **License check passed for any new crate?** — Independently verified: a `cargo metadata`
      census of all 172 lockfile packages yields only `MIT`, `Apache-2.0`, `Unicode-3.0`, `Zlib`
      and SPDX `OR` expressions that resolve into the `deny.toml` allowlist (including
      `MIT OR Apache-2.0 OR LGPL-2.1-or-later` and `Apache-2.0 OR BSL-1.0 OR MIT`, both
      satisfiable via MIT). No copyleft-only crate is present. `cargo deny check licenses`
      should pass.
- [x] **No secrets, credentials, or private data in source/tests/logs?** — Confirmed. `.env`
      contains only `DATABASE_URL=sqlite:target/grid-dev.db`, a credential-free relative path.
      But see M1 (the detector that is supposed to keep it that way fails open) and the minor
      note on committing `.env` at all.
- [x] **No unsafe commands or destructive shell scripts?** — Confirmed. No `sudo`, no `rm -rf`,
      no `curl | sh`, no network calls in tests. `bootstrap-repo.sh` was made non-destructive and
      idempotent, which is a genuine improvement over the version on `main`. CI uses only
      `actions/checkout@v4` — no third-party actions in a security-boundary file, with the
      reasoning stated inline. Workflow permissions are `contents: read`; the trigger is
      `pull_request`, not `pull_request_target`. All correct.
- [ ] **Permission profile approved?** — No. `.claude/settings.json` is new, is a §14.1 security
      boundary by its own header, and the manifest lists Security/Release owner approval as
      required and **not obtained**. See also M9 on rules that may not match what they appear to
      block.

## Migrations & Rollback

- [x] **Migrations append-only?** — Yes. One migration, added not modified;
      `check-migrations.sh` verified passing against `origin/main`.
- [x] **Blank database and upgrade-path tested?** — Blank-database: yes, and it is a real test
      (`#[sqlx::test(migrations = "../../migrations")]`), verified passing. Upgrade-path: n/a
      with a single migration.
- [ ] **Rollback strategy documented for destructive changes?** — The change is not destructive,
      and the PR body gives a rollback ("revert the branch; the database lives in gitignored
      `target/`"). However the **work package's own** `## Migration and rollback` section still
      reads *"Not applicable — no database schema changes"* — it was not updated by the
      amendment note that authorized the migration. A future agent reading the authority
      document gets a false statement.

## Evidence & Verification

- [ ] **Verification commands listed and actually run?** — Listed and mostly run, but not
      against the final commit (C3), and the two most consequential — `verify.ps1 -Scope Full`
      and the CI build jobs — have never produced a result anywhere (M8).
- [x] **Evidence manifest generated and valid?** — Valid JSON, matches the §8.12 field list,
      and its `sha256` matches the hash quoted in the PR body. The generator is genuinely good:
      it reads model identity from `ai-toolchain.lock` rather than hard-coding it, emits `null`
      rather than inventing reviewer/approval data, and derives `verification_status` from a
      recorded exit status instead of asserting it.
- [ ] **No "all tests pass" claim without cited command and exit status?** — The claims are
      cited, but `verification_status: "passed"` sits above `failed: 1` and `not_run: 1`, and
      the canonical run it derives from was executed three commits before HEAD.

---

## Findings

### Critical (must fix before merge)

**C1. `just bootstrap` fails on every fresh checkout; both CI build jobs run it as their first repo step.**

- **Location:** `justfile:22-23` (`sqlx database create` / `sqlx migrate run`);
  `.github/workflows/alpha-ci.yml:73,101`; `.env:7`
- **Authority violated:** alpha-spec §8.11 ("Bootstrap and verification commands are
  non-interactive, idempotent where practical, timeout-bounded, and return non-zero on
  failure"); P1-00 implementation constraint "All changes must be verifiable from a clean
  checkout using only committed files and the cloud environment setup script"; the acceptance
  criterion that `just verify` passes on Linux.
- **Reproduced:** at branch HEAD `7787921` with no `target/` directory, using real sqlx-cli
  0.9.0:
  ```
  $ sqlx database create        # DATABASE_URL=sqlite:target/grid-dev.db, from .env or env
  error: error returned from database: (code: 14) unable to open database file
  exit=1
  ```
  Root cause confirmed in source, not inferred: `sqlx-cli-0.9.0/src/database.rs::create` calls
  `Any::create_database`, and `sqlx-sqlite-0.9.0/src/migrate.rs:22-39` opens the file with
  `create_if_missing(true)` and **never creates the parent directory**. SQLite itself refuses to
  create a database in a non-existent directory (independently confirmed). `cargo install
  --locked` does not create `./target`, so nothing earlier in the recipe supplies it.
- **Confirmed in CI after this finding was written.** `linux-smoke` on run #2 (`7787921`)
  completed **failure** at 2026-08-20T03:20:49Z, at the `Bootstrap` step, with the predicted
  error verbatim
  ([job 96299995264](https://github.com/Seismic-Fate/GRID-Alpha/actions/runs/32326922669/job/96299995264)):
  ```
  # Local SQLite development database for the compile-time query cache (ADR-002).
  sqlx database create
  error: error returned from database: (code: 14) unable to open database file
  error: recipe `bootstrap` failed on line 18 with exit code 1
  ##[error]Process completed with exit code 1.
  ```
- **Consequence:** `linux-smoke` is red and `windows-authoritative` runs the same recipe, so it
  will fail the same way. The ADR-002 developer onboarding path — "`just bootstrap` creates it
  and applies migrations" — has never worked from clean, and the acceptance criterion that the
  repository passes `just verify` on Linux in the cloud environment is **not met**.
- **Irony worth noting:** commit `7787921` relaxed `check-env-contract.sh` so a missing
  `target/` is a NOTE rather than a failure, on the reasoning that "bootstrap will create it".
  The guard that would have flagged this was softened; the underlying defect was not found.
- **Fix required:** `mkdir -p target` as the first line of the `bootstrap` recipe (and the
  equivalent in any Windows path), then re-run `just bootstrap && just verify` from a clean
  checkout and record the result. A `?mode=rwc` query parameter does **not** fix this — the
  directory, not the file, is missing.

**C2. A fresh clone does not compile under the configuration this package documents as required; the committed `.env` defeats the committed `.sqlx` cache.**

- **Location:** `.env:7`; `docs/CLAUDE.md:52-53`; `CLAUDE.md:30-32`;
  `docs/02-adr/002-sqlx-offline-cache.md` ("Required environment contract" and Consequences)
- **Authority violated:** `final-build-spec.md` §8.2 — the `.sqlx` cache is committed "so builds
  are reproducible **without requiring a live database during compilation**"; alpha-spec §9.4
  clean-checkout reproducibility.
- **Reproduced:** at branch HEAD with no `target/`, with `DATABASE_URL` and `SQLX_OFFLINE` both
  unset in the process environment so that only the committed `.env` applies:
  ```
  $ cargo check -p grid-persistence
  error: error returned from database: (code: 14) unable to open database file
    --> crates/persistence/src/lib.rs:22:15
  exit=101
  ```
  The same command with `SQLX_OFFLINE=true` succeeds in 0.18s. The committed cache is correct;
  it is simply never consulted.
- **Mechanism:** `sqlx::query!` uses the offline cache only when `SQLX_OFFLINE=true` **or**
  `DATABASE_URL` is absent. `.env` is committed and is loaded by `dotenvy` during macro
  expansion, so `DATABASE_URL` is never absent — every clean checkout is forced onto the live-DB
  path that has no database. ADR-002's stated consequence, "Compilation needs no database. …
  A clean checkout builds offline," is the opposite of the delivered behaviour, and
  `docs/CLAUDE.md`'s instruction to leave `SQLX_OFFLINE` unset locally actively selects the
  broken path.
- **Fix required:** pick one and make it consistent across `.env`, `docs/CLAUDE.md`, root
  `CLAUDE.md`, ADR-002 and `check-env-contract.sh`. The cheapest correct option is to stop
  committing `.env` (ship `.env.example`) and default local work to `SQLX_OFFLINE=true`,
  unsetting it only when regenerating the cache. Whatever is chosen, `git clone && cargo check`
  must succeed with no prior setup.

**C3. The evidence manifest and completion record attest to a commit that is not the branch head.**

- **Location:** `.ai/evidence/P1-00/manifest.json` (`commit`, `commit_info.sha`,
  `verification.commands[0].run_against_commit` — all `f5d0d8bb4167…`);
  `.ai/evidence/P1-00/PR-BODY.md` ("Final commit: f5d0d8bb4167…")
- **Authority violated:** alpha-spec Appendix D #1 ("claim a command passed when it was not run
  against the final commit"); §12.8 ("A PR may not state 'all tests pass' unless the listed
  command was run against the final commit"); the repo's own root `CLAUDE.md`, which restates
  both.
- **Evidence:** branch head is `7787921`. Three commits follow `f5d0d8b`: `04dee2a` (evidence),
  `d2af8d6` (PR body), and `7787921`, which **modifies `scripts/check-env-contract.sh`** — a
  script the `guards` CI job executes. The PR body's own "Commit sequencing — done" section
  argues the ordering was chosen precisely to satisfy §12.8; two further commits then landed on
  top, and neither the manifest nor the completion record was regenerated.
- **Note in mitigation:** `7787921`'s commit message states "just verify re-run: exit 0", and
  the changed script is not part of `just verify`. That is plausible but is not evidence, and
  it is not what the manifest records.
- **Fix required:** re-run the canonical suite against the actual final commit, regenerate
  `manifest.json` via `just evidence P1-00`, and update the completion record's `Final commit`
  and `Evidence manifest hash`. If the manifest must precede the head, say so explicitly in the
  record rather than asserting a head it does not describe.

**C4. The PR body states the committed manifest reads `clean`; it reads `uncommitted: 3 path(s)`.**

- **Location:** `.ai/evidence/P1-00/PR-BODY.md`, final line of "Commit sequencing — done" —
  *"`commit_info.state` reads `clean`. The manifest attests to code that exists."*
  vs. `.ai/evidence/P1-00/manifest.json` —
  `"state": "uncommitted: 3 path(s) not yet in f5d0d8bb41675a715e37353373f2e5d0629a4824"`
- **Authority violated:** alpha-spec §8.12 (sanitized, accurate provenance), §12.8 (evidence
  contract).
- **Why this is Critical rather than cosmetic:** the manifest's `sha256` **does** match the hash
  quoted in the PR body, so this is not a stale-artifact mismatch that a reader could detect and
  discount. The body asserts a specific field value of a specific, hash-pinned artifact, and the
  assertion is false. The generator deliberately emits that field so a dirty-tree manifest
  cannot masquerade as a clean one; the narrative overrides the safeguard in prose.
- **Fix required:** regenerate on a clean tree (which C3's fix requires anyway) and restate the
  sentence from the regenerated file.

**C5. Every blocking decision rests on a self-attested owner ruling that the same manifest lists as not obtained.**

- **Location:** `.ai/evidence/P1-00/manifest.json` → `human_approvals`;
  `docs/02-adr/001…005` "Status: Accepted … by the Product/Architecture owner";
  `docs/01-work-packages/p1-00-work-package.md` "Amendment note — 2026-08-20"
- **Authority violated:** alpha-spec §1.3 (the implementing agent is never the sole reviewer and
  may not decide architecture), §1.4 (role-holders approve within their role), §1.5 (on conflict,
  **stop** and produce a decision request), and the work package's own Stop/decision conditions:
  *"If the alpha spec and final build spec disagree on crate names or authority order: Stop."*
- **The contradiction, in the artifacts:** `human_approvals.required` lists three items —
  *Architecture owner: authority order, crate boundaries, P1-02 to P1-00 scope rework*;
  *Security/Release owner: `.claude/settings.json`*; *Merge reviewer: CI skeleton and branch
  protection*. **None of the three appears in `obtained`.** `obtained` instead lists nine
  free-text entries (D1–D6, ADR-003/004/005) with no approver identity, no timestamp, no
  reference to any record. Meanwhile ADR-001 states the Architecture owner accepted the
  authority order and crate boundaries — the exact item the manifest says is still required.
  Both statements cannot be true.
- **Why it matters:** the asserted rulings are what authorize the licence change (D4), the
  P1-02→P1-00 schema pull-forward (D2/ADR-002), the sqlx major bump (ADR-003), the amendment of
  a frozen recipe (ADR-005), and the departure from §8.7's tree (D3). A fresh reviewer has no
  way to distinguish "the owner ruled" from "the implementer decided and wrote that the owner
  ruled". ADR-001 is candid that D3 "is a deviation from a level-2 document, so it is ratified
  by the product owner directly — an ADR alone could not authorize it" — and then the only
  record of that ratification is the ADR.
- **Fix required:** the product owner should confirm, in a form outside the implementer's
  authorship (PR review, signed comment, or an approvals file), which of D1–D6 and ADR-003/004/005
  they actually issued. Anything unconfirmed should be reopened as a decision request. Update
  `human_approvals.obtained` to name the approver and the artifact for each entry.

**C6. The project's outbound licence was changed from GPL-3.0 to MIT OR Apache-2.0.**

- **Location:** `LICENSE` deleted (674 lines, GPL-3.0); `LICENSE-MIT`, `LICENSE-APACHE` added;
  `Cargo.toml` `license = "MIT OR Apache-2.0"`; `deny.toml` allowlist built around it;
  `docs/02-adr/001-repo-bootstrap-decisions.md` D4
- **Authority violated:** neither `alpha-spec.md` nor `final-build-spec.md` names a project
  licence, so there is no authority resolving the conflict. §1.4 assigns licensing to the
  **Data/Licensing owner**; ADR-001 records the ruling as the **Product/Architecture owner's**
  and the manifest records neither as obtained. The licence is not in P1-00's Scope list, not in
  its acceptance criteria, and not in its Non-goals — it is pure scope expansion.
- **Risk:** the deleted `LICENSE` was added by the repository owner in a deliberate standalone
  commit (`f71a241 Add GNU GPL v3 license`). ADR-001 characterizes it as "the error" and
  `Cargo.toml` as correct. That is a defensible engineering opinion and an indefensible basis
  for an agent to relicense a public repository unilaterally. Relicensing is not cleanly
  reversible once published, and downstream ADR-001's Consequences, `deny.toml`'s permissive-only
  policy, and the `final-build-spec.md` §3.2 vendored-artifact argument now all depend on it.
- **Fix required:** the Data/Licensing owner confirms the licence explicitly, in their own
  words. If confirmed, keep the change and record the approval properly. If not, restore
  `LICENSE` and correct `Cargo.toml` instead. Either way this belongs in its own commit with its
  own approval, not folded into a bootstrap package.

### Major (should fix, human can accept risk)

**M1. `check-secrets.sh` fails open when the base ref is unavailable.**

- **Location:** `scripts/check-secrets.sh:20-21, 24-27`
- **Reproduced:**
  ```
  $ ./scripts/check-secrets.sh no-such-ref
  error: object 4b825dc642cb6eb9a060e54bf8d69288fbee4904 is a tree, not a commit
  fatal: Invalid symmetric difference expression 4b825dc...HEAD
  check-secrets: OK no secret patterns in the diff
  exit=0
  ```
- The documented fallback — `BASE="$(git hash-object -t tree /dev/null)"   # empty tree: scan
  everything` — cannot work: `git diff <tree>...HEAD` is invalid, three-dot notation requires
  commits. The trailing `|| true` on the pipeline swallows the failure, `added` stays empty, and
  the script reports success having scanned **nothing tracked**. Untracked files are still
  scanned, so it is not fully blind, but every committed line is skipped in exactly the
  situation the fallback exists for.
- **Risk if unaddressed:** a shallow clone, a fork PR whose base is not fetched, a rename of the
  default branch, or a developer running it locally without `origin/main` all yield a green
  secret gate over an unscanned diff. `check-migrations.sh` and `check-traceability.sh` have the
  same shape but exit 0 with an explicit `SKIP` message, which is honest; this one claims `OK`.
- **Fix:** use `git diff --no-index`-free two-dot form against the empty tree
  (`git diff 4b825dc… HEAD`, no `...`), or better, `git log --format= --all` is not needed —
  simply fail closed: if the base ref cannot be resolved, exit non-zero and say so.

**M2. Secret scanning and traceability are absent from the canonical verification interface.**

- **Location:** `justfile:26-33`; `scripts/verify.sh`; `scripts/verify.ps1`;
  `.github/workflows/alpha-ci.yml:44-70`
- alpha-spec §8.11 lists "license/dependency/security scans", "secret scanning" and "traceability
  checks from changed code to work package and acceptance criteria" among what the verification
  orchestration includes. `just verify` runs seven recipes; none of the six guard scripts is one
  of them. They exist only in the CI `guards` job.
- **Risk:** a developer or agent running the documented canonical command gets no secret scan
  and no traceability check. Combined with M1, the only place secret scanning genuinely runs is
  one CI job, on one code path, with a fail-open fallback.

**M3. `check-traceability.sh` is materially weaker than the criterion it claims to satisfy.**

- **Location:** `scripts/check-traceability.sh:52-62`
- The acceptance criterion: non-zero if **any** non-trivial file change is not referenced in
  `docs/02-adr/` or `docs/01-work-packages/`. The implementation collects the set of non-trivial
  paths, then passes if a single token matching `P1-[0-9]{2}|ADR-[0-9]{3}|docs/0[12]-…` appears
  **anywhere** in the commit-message range or `$PR_BODY`. Per-file traceability is never
  evaluated; the file list is used only for the error message.
- **Risk:** a PR touching 200 unrelated files passes on one "P1-00:" commit-subject prefix.
  §9.4's "every merged non-trivial change is linked to an approved work package" is not actually
  enforced, while the manifest records the check as passing.

**M4. ADR-003 overrides `alpha-spec.md` §14.2 — which this project's own ADR policy says an ADR can never do.**

- **Location:** `docs/02-adr/003-sqlx-0-9-upgrade.md`, Decision section; vs.
  `docs/02-adr/README.md` ("An ADR … can never override either specification") and
  `docs/99-templates/template-adr.md` (same sentence)
- §14.2: *"Major version upgrades are separate work packages and cannot be hidden inside feature
  changes."* ADR-003 acknowledges this verbatim, then keeps the bump in P1-00 on the argument
  that "the intent of §14.2 — no silent upgrades — is satisfied" by disclosure. Disclosure is
  not the rule; separation is. The same structural move appears in ADR-001's deferral of §9.5's
  P1-00 fixtures and D3's departure from §8.7's tree.
- **Risk:** this establishes, in the foundational package, that a well-written ADR can
  reinterpret a level-1/level-2 requirement whenever compliance is costly. Everything P1-01…P1-11
  inherits that precedent. The *technical* judgment (sqlx 0.9 removes an unfixable RUSTSEC
  advisory, feature narrowing tested and ineffective, CLI/library skew closed) is sound and well
  evidenced — the objection is to who authorized keeping it here.

**M5. A migration, a persistence API and an offline cache were delivered against an explicit non-goal, authorized by editing the work package in the same PR.**

- **Location:** `migrations/0001_schema_meta.sql`; `crates/persistence/src/lib.rs`;
  `.sqlx/query-dc674e29….json`; `docs/01-work-packages/p1-00-work-package.md` "Amendment note"
- The non-goal read: *"No SQLx migrations beyond the empty `migrations/` directory (schema
  creation is P1-02)."* The follow-up read: *"ADR-002 … to be resolved in P1-02."* Both were
  rewritten inside this PR, by the implementer, citing an owner ruling the manifest lists as not
  obtained (C5). §1.5 places the work package at authority level 5 and the specs above it — but
  the mechanism by which a work package gets amended mid-implementation by its own implementer
  is exactly the failure mode §1.5's "stop and produce a decision request" exists to prevent.
- **Two substantive concerns beyond the process:**
  1. `schema_meta` stores `schema_version = '0001'` as a hand-written literal that duplicates
     state SQLx already maintains in `_sqlx_migrations`. The two can drift silently — nothing
     updates the literal when `0002` lands. `migrations_apply_to_a_blank_database` asserts the
     literal, so the test would keep passing while the value is stale. Prefer deriving the
     version from the migrator, or drop the seeded row.
  2. The work package's `## Migration and rollback` section still reads *"Not applicable — no
     database schema changes."* The amendment note did not update it. An authority document now
     contains a statement its own diff falsifies.
- **Risk:** P1-02 inherits a schema-version mechanism nobody designed, and future packages
  inherit the precedent that a non-goal is negotiable by the party it constrains.

**M6. FRB and Flutter versions are not pinned to the standard §8.11 requires.**

- **Location:** `Cargo.toml:36` (`flutter_rust_bridge = "2.5.0"`) vs `app/pubspec.yaml:12`
  (`flutter_rust_bridge: 2.5.0`); no `app/pubspec.lock`; no `toolchains/flutter.version` or
  `toolchains/native-dependencies.lock`
- Cargo's `"2.5.0"` is a caret requirement (≥2.5.0, <3.0.0); Dart's bare `2.5.0` is exact.
  flutter_rust_bridge requires the Rust crate and the Dart package to be the same version — the
  Rust side can float to 2.9.x on any lockfile refresh while Dart stays at 2.5.0. `Cargo.lock`
  currently holds it, but the declaration does not.
- §8.11: *"Rust, Flutter/Dart, FRB, SQLx, XGBoost/native artifacts, and package dependency
  versions are pinned through committed toolchain files and lockfiles."* §8.7 lists
  `toolchains/flutter.version`, `toolchains/native-dependencies.lock` and
  `app/flutter/pubspec.lock`. None exists.
- **Risk:** a silent FRB skew surfaces as generated-binding corruption in P1-01, which is
  precisely where it is most expensive to diagnose. `pubspec.yaml` predates this PR, but pinning
  the toolchain is squarely P1-00's mandate.

**M7. §8.7/§9.2 deliverables are absent without a recorded deviation.**

- **Missing:** `.claude/agents/`, `.claude/skills/` (§8.7 tree and §9.2's "explorer, implementer,
  numerical-review, data-contract, security-review, and adversarial-review agent definitions");
  `schemas/`, `fixtures/`, `tests/`, `benches/`; `docs/runbooks/`, `docs/model-cards/`,
  `docs/traceability/` (§9.2 names a traceability matrix explicitly); `app/lib/main.dart`
  (permitted by the work package's own Non-goals as a placeholder).
- Fixtures are the **only** omission recorded as a deliberate deviation (ADR-001, and again in
  the provider READMEs) — that part is handled well. The rest are disposed of by a single row in
  the authority index's alias table reading "not yet created — P1-11", with no ADR and no owner
  ruling. §9.2 is level-2 authority; the work package cannot silently drop items from it.
- **Risk:** P1-00's stated objective is that a fresh session can enter and extend the repository
  correctly. The specialized-agent definitions are part of how §9.2 intends that to happen, and
  the adversarial-review role — the one producing this document — has no committed definition.

**M8. No CI build job has ever completed, on any commit.**

- **Evidence (GitHub API, PR #1):** run #1 (`d2af8d6`) — **cancelled**. Run #2 (`7787921`) —
  `repository guards` **success** (03:05:55Z); `linux-smoke` **failure** (03:20:49Z, at
  `Bootstrap`, per C1); `windows-authoritative` still **in_progress**.
- The acceptance criterion "The repository passes `just verify` … on Linux in the cloud
  environment" has no CI confirmation, and `scripts/verify.ps1 -Scope Full` — authoritative for
  merge under §8.11 — has never executed anywhere. The manifest discloses the latter honestly as
  a known limitation, which is to its credit; the point stands that the merge gate is unproven.
- `linux-smoke` has now failed exactly as C1 predicted. `windows-authoritative` runs the same
  `just bootstrap` recipe and should be expected to fail identically; it has never produced a
  result on any commit.

**M9. `.claude/settings.json` deny rules that appear to block shell pipelines may not.**

- **Location:** `.claude/settings.json:31-34` — `"Bash(curl:* | sh)"`, `"Bash(curl:* | bash)"`,
  `"Bash(wget:* | sh)"`, and `"Bash(rm -rf /*)"`
- These are written as though the permission matcher parses pipelines. Permission matching is
  prefix/pattern based over the command string, so a rule of this shape should not be assumed to
  deny `curl https://x | sh`. The file's own header is candid that "sandboxing and protected
  branches carry the real weight" — which is the right posture — but a rule that reads as
  protection and provides none is worse than an absent rule, and §14.1 makes this file a
  security boundary.
- **Fix:** empirically confirm each deny rule fires, or replace the pipeline-shaped entries with
  a documented note that pipeline interception is the sandbox's job. Also worth reviewing:
  `Bash(git push:*)` is in `ask` while `Bash(git commit:*)` is in `allow` — 13 commits and the
  branch push happened during this package; confirm that matches intent. Requires
  Security/Release owner approval regardless (C5).

**M10. `just verify` omits doctests, Flutter analysis and feature-combination tests that §8.11 lists.**

- **Location:** `justfile:36-46`
- `cargo nextest run --workspace` does not execute doctests at all — a separate `cargo test
  --doc` is required. Confirmed locally: `cargo test -p grid-persistence` reports a `Doc-tests
  grid_persistence` phase that nextest never runs. Zero doctests exist today, so nothing is
  missed yet; the gate stays blind once P1-01+ adds doc examples.
- No Flutter/Dart formatting, analysis, or generated-binding check is present, though §8.11 lists
  them and `app/pubspec.yaml` exists. This is defensible while there is no Dart source, but it
  is not recorded as a limitation the way `-Scope Changed` is.

### Minor (suggestion, non-blocking)

1. **`verification_status: "passed"` is derived from one command while a failure sits below it.**
   `scripts/generate-evidence-manifest.sh:186-200` comments that failures are counted separately
   "so a green headline can never hide them", then emits `"verification_status": "passed"`
   alongside `"failed": 1, "not_run": 1`. Suggestion: make the headline `passed_with_exceptions`
   whenever `failed > 0` or `not_run > 0`.

2. **Two manifest statements about the session environment contradict each other.** The canonical
   run records `"SQLX_OFFLINE unset"`; known-limitation #2 says `SQLX_OFFLINE` is "baked into the
   running process and cannot be changed mid-session". Both cannot be true. Given C2, which one
   holds determines whether the canonical run is reproducible at all.

3. **The manifest's CI claim is wrong on a checkable fact.** Known-limitation #2 says "CI is
   unaffected (GitHub runners set neither)". `.github/workflows/alpha-ci.yml:38-40` sets
   `DATABASE_URL` at workflow `env:` scope, so every job has it. The conclusion happens to hold
   after commit `7787921`; the stated reason does not.

4. **Committing `.env` normalizes a habit worth not forming.** It is credential-free today and
   `.gitignore` carries an explicit note. But it is the direct cause of C2, and the next person
   to add a variable will add it to a tracked file. `.env.example` + a documented `SQLX_OFFLINE`
   default achieves the same onboarding benefit.

5. **`check-verify-parity.sh` compares sorted unique sets.** `norm()` ends in `sort -u`, so step
   *ordering* and duplicated steps are invisible, and the `verify.sh` side is extracted with
   `grep -E '^\s*(cargo|typos)'` — a step invoking any other binary is silently outside the
   comparison. Fine for today's seven recipes; worth tightening before the list grows.

6. **The guard scripts have no committed tests.** They are the largest body of new logic in the
   diff. The manifest says they were behaviour-tested on synthetic git ranges; nothing in the
   repository re-runs those cases. A `tests/guards/` suite with fixture repositories would have
   caught M1 and would protect the next edit.

7. **`deny.toml` sets no explicit `unmaintained`/`unsound` policy** and leaves
   `multiple-versions = "warn"`. Both are disclosed as P1-11 work; noting so P1-11 does not lose
   them.

8. **Directory-number collision.** This review was requested at `docs/05-sessions/`, but the
   branch's own authority-index alias table assigns `docs/05-model-specs/` to 05 and
   `docs/06-sessions/` to session logs (`scripts/bootstrap-repo.sh:38,41` creates both). The file
   is written where requested; the numbering should be reconciled before session logs start
   landing, or `docs/06-sessions/` will fork from whatever the review convention becomes.

9. **Positive, recorded for the merge reviewer:** ADR-004's claim that `scripts/verify.sh` is
   whitespace-only apart from the `--workspace` flag was independently verified —
   `diff <(git show origin/main:scripts/verify.sh | tr -d '\r') scripts/verify.sh` returns
   exactly one changed line. The 30-line diff can be read as claimed. Likewise the authority-order
   correction (final-build-spec above alpha-spec) is right per §1.5, the licence census clears
   `deny.toml`, and the five templates exceed Appendix B in a way that will pay off downstream.

## Questions for Implementer

1. **Was `just bootstrap` ever executed on a checkout without a pre-existing `target/` directory?**
   It fails deterministically there — now confirmed by `linux-smoke` (C1). If the session's
   `target/` predated the run, the "clean 76-file checkout, no `target/`" claim in the PR body
   needs to be withdrawn or re-evidenced.

2. **What exact environment did the canonical `just verify` run under?** The manifest says
   `SQLX_OFFLINE` unset; the known limitations say it could not be unset. Under the former, C2
   says the build cannot succeed. Please paste the actual `env | grep -E 'SQLX|DATABASE'` from
   that run.

3. **Which of D1–D6 and ADR-003/004/005 did the owner approve, and where is that recorded?**
   The manifest lists all three *required* approvals as not obtained while the ADRs state the
   owner accepted. Point to the artifact.

4. **Who authorized deleting `LICENSE`?** §1.4 puts licensing with the Data/Licensing owner;
   ADR-001 D4 attributes it to the Product/Architecture owner. The file was added by the
   repository owner in its own commit.

5. **Why does `schema_meta` duplicate `_sqlx_migrations`?** What updates
   `schema_version` when `0002` lands, and what fails if nothing does?

6. **`check-secrets.sh`'s empty-tree fallback has never worked** (M1). Was the "behaviour-tested"
   claim exercised only through the `origin/main` path?

7. **Was the possibility considered that `verification_status: "passed"` at HEAD is unsupported**,
   given the canonical run predates HEAD by three commits, one of which edits a CI-gating script?

## Follow-up Work Identified

- Add a guard-script test suite (fixture repositories, both pass and fail cases per script);
  fold the guards into `just verify` so §8.11's secret-scanning and traceability items are
  actually covered by the canonical command.
- Make `check-traceability.sh` evaluate per-file traceability against the work package's Scope
  section, not just the presence of a token.
- Make every guard fail closed on an unresolvable base ref, with a distinct exit code for
  "could not evaluate" versus "passed".
- Pin the Flutter/Dart side: commit `app/pubspec.lock`, add `toolchains/flutter.version`, and
  pin `flutter_rust_bridge` to an exact version on the Rust side (`=2.5.0`) so it cannot drift
  from the Dart pin.
- Decide the `schema_meta` versus `_sqlx_migrations` question before P1-02 builds on `0002`.
- Record an ADR (or an owner ruling) for the §8.7/§9.2 items being deferred — agent definitions,
  `schemas/`, `fixtures/`, `tests/`, `benches/`, runbooks, model cards, traceability matrix —
  the way fixtures were handled, rather than a table row.
- Reconcile `docs/05-sessions/` versus `docs/06-sessions/` before the first session log lands.
- Implement `-Scope Changed` (already tracked to P1-11) — §8.11 names it as the canonical
  invocation, and both scripts currently treat it as `Full`.
- Add `cargo test --doc` (or an equivalent) to the verify chain before P1-01 introduces doc
  examples that nextest will not run.
