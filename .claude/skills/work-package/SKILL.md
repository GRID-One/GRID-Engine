---
name: work-package
description: Use when starting or continuing a GRID Engine work package (an ID of the form P<phase>-NN). It walks the engine-spec §8.17 eleven-step workflow and its stop conditions, the Definition of Ready and Done, the frozen verification commands, the branch, commit and PR rules, and the evidence rule, and names which agent definition to use at each step.
---

# Work package execution

Authority for everything below: `engine-spec.md` §8.16 to §8.18, §8.20, §12.8, §12.9, Appendices
C and D; the root `CLAUDE.md`; `docs/CLAUDE.md`. Where this skill and those documents differ, they
win and this skill is corrected.

## Before anything: is the package Ready? (§8.16.1)

A package is ready for implementation only when:

- the objective and acceptance criteria are unambiguous;
- the relevant contracts, fixtures and authority references exist;
- any architecture, statistical, security or licensing decision is already resolved, or is
  explicitly listed as a required gate;
- **every owner decision the package depends on is ratified** in
  `docs/00-meta/decision-register.md`. A "proposed" default, an "open" entry or the still-open part
  of a partly decided entry is not a ratification, wherever the DR is listed in
  `docs/01-work-packages/README.md`;
- a package that implements a production model component has its model spec approved by the
  statistical owner (§6.8);
- a package that declares oracle parity targets has the committed fixtures and their sha256
  manifest (or is the package that creates them), and every known oracle defect on those targets
  listed with its `KI-…` ID and correction-ledger status (DR-B1);
- the allowed scope and non-goals are stated;
- at least one objective verification path can be run;
- the earlier packages it depends on are merged and passing.

Not Ready means stop: write or amend the work-package file from
`docs/99-templates/template-work-package.md`, or raise the decision request. Never treat a package
as Ready while a decision it depends on is unratified (Appendix D item 15).

## The eleven steps (§8.17)

1. **Load authority.** Read `CLAUDE.md`, `docs/00-meta/authority-index.md`, the `engine-spec.md`
   sections the package lists, the work-package file, and only the contracts and model specs it
   names. Note the `KI-…` anti-targets and the DR statuses it depends on.
2. **Explore read-only.** Use the `repository-explorer` agent. Identify existing patterns, affected
   modules, generated files (`.sqlx/`, `Cargo.lock`, `fixtures/parity/`), tests, migrations and
   risks. No edits yet.
3. **Plan.** A file-level plan, the contract impact, a test plan, migration and rollback notes, and
   explicit assumptions. Assign each file to exactly one writer.
4. **Gate the plan.** Human approval is required when the package changes architecture,
   statistical semantics, the parity regime or the oracle correction ledger, data rights, security,
   or destructive persistence behaviour. Mechanical packages may proceed under pre-approved policy.
5. **Implement in isolation.** Use the `rust-implementer` agent, one per disjoint file set, on the
   package's branch. One writer owns a file at a time. Each implementer edits only the files its
   task names and runs its own gates (`cargo fmt --all`; `cargo clippy -p grid-<crate>
   --all-targets --all-features -- -D warnings`; `cargo test -p grid-<crate>`; `cargo test -p
   grid-<crate> --doc`; `typos <paths>`).
6. **Verify incrementally.** After each coherent change, `cargo test -p grid-<crate>`, not a single
   run at the end.
7. **Run the canonical suite.** `just verify` (Linux smoke: `check-fmt`, `check-lint`, `check-sqlx`,
   `test-rust`, `test-doc`, `audit`, `check-guards`, `check-typos`). The merge-authoritative run is
   `powershell -ExecutionPolicy Bypass -File scripts/verify.ps1 -Scope Full` (DR-A3), run by CI.
   Capture the machine-readable results.
8. **Adversarial review.** Fresh-context, read-only agents who are not the implementer: the
   specialist reviewers that the diff's content calls for (`data-contract-reviewer`,
   `numerical-reviewer`, `security-dependency-reviewer` on every PR, `reference-parity-reviewer`
   for any change that ports a component, touches `reference/python/` or a committed fixture, or
   claims parity), then the `adversarial-pr-reviewer` over the work package and the full diff.
9. **Remediate and rerun.** Fix every finding or disposition it in writing; rerun the affected
   checks and, for anything above minor, the canonical suite.
10. **Prepare evidence and PR.** An atomic commit series; `just evidence <WP-ID>` to generate
    `.ai/evidence/<WP-ID>/manifest.json` (never hand-written); `./scripts/check-evidence-claims.sh
    origin/main <WP-ID>`; a PR body per Appendix C that covers every §12.8 item.
11. **Human merge.** Protected-branch policy and human approval control the merge. **The agent
    never merges.** A merge commit, never a squash.

## Stop conditions (§8.17)

Stop at a clean boundary and create a decision request, never pick silently, on:

- a higher-authority conflict;
- undocumented provider behaviour;
- destructive migration ambiguity;
- a missing numerical specification;
- license uncertainty;
- a requirement that can only be satisfied by weakening a test or safety control;
- an unexplained divergence from the reference oracle;
- a dependency on an owner decision that is not ratified;
- a requirement that would reproduce a known oracle defect (KI) in Rust.

Repeated failure on the same gate triggers root-cause analysis or a decision request, never
suppression of the gate (§8.18.1). Budget exhaustion produces a partial evidence record and a
Blocked package, never a skipped check.

## Verification commands

```bash
export PATH="$HOME/.cargo/bin:$PATH"; set -a; . ./.env; set +a     # from the repo root
cargo test -p grid-<crate>                                           # incremental (step 6)
just verify                                                          # canonical, Linux smoke (step 7)
powershell -ExecutionPolicy Bypass -File scripts/verify.ps1 -Scope Full   # merge-authoritative (DR-A3)
just evidence <WP-ID>                                                # evidence manifest (step 10)
./scripts/check-evidence-claims.sh origin/main <WP-ID>               # the record is true of the tree
```

Verification recipes are a frozen contract (ADR-001 D5). Make the repository satisfy them. Never edit
a recipe, weaken a lint, or skip a check to make a failure disappear; if a recipe cannot pass, stop
and report. `scripts/check-verify-parity.sh` keeps `justfile`, `scripts/verify.sh` and
`scripts/verify.ps1` in lockstep. An oracle step is never wired into the chain without its own ADR.

## Branch, commit, PR (root `CLAUDE.md`)

- One work package per branch, `wp/P<phase>-NN-description`. When the harness assigns another name,
  record the deviation in the work-package file under "Known limitations", as P0-01 and P0-02 did.
- Atomic commits. The human reviews the diff before each commit. Every commit message cites its work
  package or ADR (`P1-06`, `ADR-012`, …) so `scripts/check-traceability.sh` can trace it.
- Generated files are never hand-edited: `.sqlx/` (`cargo sqlx prepare`), `Cargo.lock` (cargo), oracle
  fixtures and goldens (the exporter, under a reviewed ledger entry). Migrations are append-only.
- No new dependency without written justification and owner approval; never `cargo add`,
  `cargo update` or `cargo generate-lockfile` inside feature work; `rust-toolchain.toml` untouched.
- The PR body follows Appendix C (work package, final commit, environment, authority documents read,
  contracts, migrations and dependencies changed, targeted tests, canonical command and exit status,
  golden changes and approval, performance, security/licensing review, fresh-context reviewer,
  findings resolved, human approvals, known limitations, evidence manifest hash; plus the oracle
  parity result, reference-oracle job result and owner decisions depended on where they apply) and
  covers every §12.8 item.
- A PR may not say "all tests pass" unless the listed command was run against the final commit. CI
  is authoritative if local and CI results differ.
- **The agent never merges.** A fresh-context reviewer who is not the implementer is required.

## Definition of Done (§8.16.2)

A work package is done only when:

- implementation, tests, migration changes, docs and generated artifacts are synchronized;
- targeted tests and the canonical verification suite pass (§8.19);
- all acceptance criteria are mapped to evidence;
- declared parity targets pass at their tolerance class, or each divergence is recorded in
  `reference/python/PARITY.md` with an accepted ADR (§7.12);
- no warnings, skipped critical tests, placeholder implementations or unexplained `TODO`/`FIXME`
  items remain in the package scope;
- a fresh-context review is complete;
- required human approvals are recorded;
- the evidence manifest is generated by `just evidence <WP-ID>` (§8.20);
- the pull request is mergeable without unpublished local state.

## Evidence, not assertion

A completion claim needs the verification command, its exit status, a test summary and
`.ai/evidence/<WP-ID>/manifest.json` generated by `just evidence <WP-ID>`. Never claim a command
passed that was not run against the final commit (Appendix D item 1). A reviewer's finding needs the
exact code and the exact authority text. No unexplained `TODO`, skipped test or placeholder remains
in scope. Model identity is not written into any repository artifact; `ai-toolchain.lock` records
the harness policy as it stands.
