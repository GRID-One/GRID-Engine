---
work-package-id: P0-03
status: Review
risk-class: Medium
owner: Product/Architecture owner
implementer: Claude Code (the implementing agent; model identity unrecorded, see Evidence required)
reviewer: Pending — a fresh-context reviewer who is not the implementer, then the owner's merge
phase: 0
---

# P0-03 — Record the owner's rulings on the decisions P1-01 raised

## Status
**Review.** Written on branch `claude/pensive-fermat-fgaweh`, restarted from `main` at the P1-01
merge commit `8755552`. That is P1-01's harness-assigned branch name, not `wp/P0-03-…` (CLAUDE.md,
"Branch, commit, PR"), the same deviation P0-01, P0-02 and P1-01 record under "Known limitations". The
owner chose this package on 2026-10-19 (answer 7 of `docs/06-sessions/2026-10-19-P0-03.md`). It is
Done once every criterion below has passed on the final content commit, a fresh-context review is
complete and the owner has merged it (engine-spec §8.16.2). No criterion below is ticked; the runs on
the final content commit are recorded in `.ai/evidence/P0-03/`.

## Ownership and risk
- **Owner.** Product/Architecture owner. The repository owner holds every role today
  (Product/Architecture, Statistical, Security/Release, Data/Licensing).
- **Implementer.** Claude Code (the implementing agent): one orchestrating session, with one Rust
  implementer agent and one documentation agent on disjoint file sets.
- **Reviewer.** Pending: a fresh-context reviewer who is not the implementer, then the owner.
- **Risk class: Medium.** It records rulings the owner has already given and decides nothing, but two
  rulings change files beyond documents: the reviewer agents' tool lists (`.claude/agents/`, a
  permission-relevant change that only removes a tool) and the `grid --help` usage text with its
  tests (`crates/grid-cli/`).
- **Required human approvals.** The owner's merge of this package's pull request, which confirms the
  transcription. The rulings themselves are given: answers 1 to 7 of
  `docs/06-sessions/2026-10-19-P0-03.md`.
- **Agent budget (engine-spec §8.18.1).** One implementing session; one writer per file; fewer than
  ten concurrent agents. This package declares no maximum number of agent turns, wall-clock timeout
  or limit on concurrent writers, which deviates from §8.18.1, as in P0-01 and P0-02.

## Authority
- Engine spec (`engine-spec.md`): §1.5, §7.12.5, §8.1, §8.2, §8.13, §8.16, §8.17, §8.18, §10.2.6,
  §14.1.
- ADRs: ADR-014 and `docs/02-adr/README.md` ("Status").
- Work package: `docs/01-work-packages/p1-01-engine-skeleton.md` ("Amendments", rows 1 to 24).
- Precedent: `docs/01-work-packages/p0-02-record-p0-01-merge-rulings.md` (an owner-directed
  transcription of merge rulings).

## Owner decisions depended on
All given on 2026-10-19 in `docs/06-sessions/2026-10-19-P0-03.md`; none is unratified.

- **ADR-014.** Accepted (answer 1). The answer and the owner's request to "open a PR to record them"
  direct the agent to transcribe the acceptance into the ADR's status.
- **P1-01 Amendments row 11.** Remove Bash from the five reviewer agent definitions (answer 2).
- **Row 20.** Keep the façade synchronous (answer 3).
- **Row 21.** The ∞-norm everywhere; amend the RAPM model spec's P-3 (answer 4).
- **Row 22.** Each Phase 2 surface, subcommand included, belongs to its content package; an erratum
  to engine-spec §8.1 (answer 5).
- **The other rows and the three contracts.** Approved; the contracts become Approved (answer 6).
- **DR-A5.** This package's ID (answer 7).

## Objective
No maintained document or code comment in the repository still describes ADR-014, the four P1-01
decision requests, the other P1-01 Amendments rows or the three P1-01 contracts as pending, and the
files the rulings govern implement them. Session records, evidence records and earlier daily-log
entries are history and stay as written.

## Observable outcome
- `grid --help` names P2-01, P2-02, P2-06 or P2-07 as the delivering package of the five Phase 2
  spellings, P1-10 for every other spelling, and no line carries "decision request".
- The five reviewer agent definitions list the tools Read, Grep and Glob only.
- ADR-014 reads `Accepted`; the three contracts read `Approved`; P1-01 reads Done.

## Preconditions
- P1-01 merged (`8755552`, 2026-10-19, PR #6).
- The owner's seven answers (`docs/06-sessions/2026-10-19-P0-03.md`).

## Scope
- **Contracts consumed.** None.
- **Contracts changed.** Status only: `grid.plays` v1, `grid.output` 1.0 and `grid.parity-fixture` 1
  move from Draft to Approved, and their "ADR-014, Proposed" mentions read "accepted". No field, rule
  or version changes.
- **Model specs changed.** `rapm-attribution.md` §10.3: P-3's norm becomes the ∞-norm, and Class B
  part 3 names the same norm (version 0.1.1).
- **Specification.** Errata carrying rulings, in `engine-spec.md` and its mirror: §8.1 (the `domain`
  cell, rule 5 and the `grid-cli` row), §8.2 (the synchronous façade) and §10.2.6 (the subcommand
  belongs to the content package).

### Paths affected
- `.claude/agents/adversarial-pr-reviewer.md`
- `.claude/agents/data-contract-reviewer.md`
- `.claude/agents/numerical-reviewer.md`
- `.claude/agents/reference-parity-reviewer.md`
- `.claude/agents/security-dependency-reviewer.md`
- `.claude/skills/work-package/SKILL.md`
- `README.md`
- `crates/domain/src/lib.rs` (one doc-comment reference: ADR-014 accepted)
- `crates/grid-cli/src/args.rs` (usage text and its unit tests)
- `crates/grid-cli/src/main.rs` (module documentation)
- `crates/grid-cli/tests/cli.rs`
- `crates/pipeline/src/lib.rs` (crate documentation)
- `docs/CLAUDE.md`
- `docs/00-meta/authority-index.md`
- `docs/00-meta/daily-log.md`
- `docs/00-meta/dashboard.md`
- `docs/00-meta/specs/engine-spec.md`
- `docs/01-work-packages/README.md`
- `docs/01-work-packages/p0-03-record-p1-01-merge-rulings.md`
- `docs/01-work-packages/p1-01-engine-skeleton.md`
- `docs/02-adr/014-domain-serialization-boundary.md`
- `docs/02-adr/README.md`
- `docs/03-contracts/README.md`
- `docs/03-contracts/engine-output-contract.md`
- `docs/03-contracts/parity-fixture-contract.md`
- `docs/03-contracts/plays-contract.md`
- `docs/05-model-specs/rapm-attribution.md`
- `docs/06-sessions/2026-10-19-P0-03.md` (new record)
- `docs/06-sessions/README.md` (one index row)
- `engine-spec.md`
- `.ai/evidence/P0-03/`

## Non-goals
- Any decision beyond the seven answers of `docs/06-sessions/2026-10-19-P0-03.md`. In particular, the
  items the session lists under "Not ruled in this session" stay open: P1-01 Amendments row 5, the
  open items of engine-output-contract §12, and the §9.5 and §7.12.3 wording on fixture timing.
- Any change to `.claude/settings.json`. Answer 2 chose to remove Bash from the reviewers, not to
  change the permission profile.
- Any change to P1-01's evidence record or to earlier ones: each attests to its own final commit and
  is never rewritten (`scripts/check-evidence-claims.sh`).
- Any oracle change, any fixture, any verification recipe, CI or guard, any dependency.
- Any session record other than the new one.
- The owner's open actions carried from P0-02: the DR-D30 branch-protection setting, DR-A3's
  follow-up ADR, the KI-NEW-Z79 decision request and the P1-12 pre-registrations.

## Inputs and fixtures
Not applicable: no fixture or data input.

## Oracle parity targets
None. The tolerance helpers in `grid-synth` already use the ∞-norm; no helper changes.

## Implementation constraints
- ADR-014 becomes immutable once Accepted, so its body is final in this package; only its status
  changes.
- `engine-spec.md` and `docs/00-meta/specs/engine-spec.md` stay byte-identical.
- The P1-01 work package keeps its 2026-10-17 "Amendments" text; the rulings go in a new dated section
  (ADR-001, "On amending a work package mid-implementation").
- The CLI change touches only the usage text and its tests: no subcommand gains behaviour, and every
  spelling still exits 2 until delivered (engine-output-contract §13).
- No model identifier is written into any file.

## Security and licensing considerations
Removing Bash from the reviewer agents narrows what those agents can do; it grants nothing.
`.claude/settings.json` is unchanged, so the permission profile for the orchestrating session and the
implementer agent is as before. No secret, host or dependency changes.

## Acceptance criteria
Each criterion is verified on the final content commit.

- [ ] `just verify` exits 0, and `pwsh scripts/verify.ps1 -Scope Full` exits 0 where pwsh is
      available. The `windows-authoritative` job is green on the pull request; it is the merge gate
      (DR-A3). The `reference-oracle` job is green.
- [ ] `./scripts/check-authority-sync.sh` passes: the spec mirror is byte-identical.
- [ ] `grep -L '^status: Accepted' docs/02-adr/014-*.md` prints nothing, and
      `grep -L '^status: Approved' docs/03-contracts/plays-contract.md docs/03-contracts/engine-output-contract.md docs/03-contracts/parity-fixture-contract.md`
      prints nothing.
- [ ] `git grep -n 'ADR-014, Proposed'` prints nothing outside `docs/06-sessions/`, `.ai/evidence/`
      and the P1-01 work package's 2026-10-17 text.
- [ ] `grep -n '^tools:' .claude/agents/*-reviewer.md` shows `Read, Grep, Glob` for each of the five
      reviewers, and `git diff --quiet 8755552 HEAD -- .claude/settings.json` exits 0.
- [ ] `git grep -n 'decision request' -- crates/grid-cli` prints nothing, and the `grid-cli` tests
      assert the delivering package of every §8.2 and §8.3 spelling.
- [ ] `git diff --quiet 8755552 HEAD -- .ai/evidence reference/python justfile scripts .github migrations .sqlx rust-toolchain.toml Cargo.toml Cargo.lock`
      exits 0 before the evidence commit.
- [ ] `./scripts/check-traceability.sh` and `./scripts/check-evidence-claims.sh` pass against
      `origin/main`.
- [ ] A fresh-context review of the diff is complete, and every finding is dispositioned.

## Verification
```text
# targeted
cargo test -p grid-cli -p grid-domain -p grid-pipeline
bash tests/guards/run.sh
./scripts/check-authority-sync.sh
./scripts/check-traceability.sh origin/main
typos

# canonical
just verify
pwsh scripts/verify.ps1 -Scope Full

# evidence (after the final content commit)
just evidence P0-03
./scripts/check-evidence-claims.sh origin/main P0-03
```
Verification recipes are a frozen contract. Make the repository satisfy them; never edit a recipe to
make a failure disappear.

## Numerical/performance tolerances
Not applicable: no computation. The Class A′ ruling changes no bound and no helper.

## Migration and rollback
No migrations. To roll back, revert this package's merge commit. The owner's rulings stand in
`docs/06-sessions/2026-10-19-P0-03.md` either way.

## Evidence required
`.ai/evidence/P0-03/verification-input.json` (the recorded runs), `manifest.json` generated by
`just evidence P0-03` (never hand-written) and `PR-BODY.md`. `ai-toolchain.lock` provenance stays
"unrecorded", as in P0-01, P0-02 and P1-01: the implementing agent's harness policy forbids writing
model identifiers into repository artifacts. This deviates from engine-spec §8.20.

## Stop/decision conditions
Stop and ask the owner if:
- a statement can be made true only by deciding something the seven answers do not settle;
- removing Bash leaves a reviewer check that cannot be performed from supplied outputs or by reading
  files;
- a guard fails in a way only a recipe change would fix.

## Follow-up
- **The reviewer workflow.** From this package on, the orchestrating session saves the diff and the
  verification and guard outputs to files and gives the reviewer agents their paths
  (`.claude/skills/work-package/SKILL.md`).
- **P1-02** implements the synchronous façade traits; **P1-10** builds the CLI adapter and the Phase 1
  subcommands; **P2-01, P2-02, P2-06 and P2-07** each add their own subcommand; **P1-12** writes its
  first A′ test with the ∞-norm.
- **Still open, unchanged:** P1-01 Amendments row 5; the engine-output-contract §12 open items;
  DR-D30; DR-A3's follow-up ADR; the KI-NEW-Z79 decision request; the P1-12 pre-registrations.
