---
work-package-id: P0-02
status: Review
risk-class: Low
owner: Product/Architecture owner
implementer: Claude Code (the implementing agent; model identity unrecorded, see Evidence required)
reviewer: Pending — a fresh-context reviewer who is not the implementer, then the owner's merge
phase: 0
---

# P0-02 — Record the P0-01 merge rulings

## Status
**Review.** Written on branch `claude/grid-engine-consolidation-e7kmh9`, restarted from `main` at the
P0-01 merge commit `b819ae0`. That is P0-01's branch name, not `wp/P0-02-…` (CLAUDE.md, "Branch,
commit, PR"), the same deviation P0-01 records under "Known limitations". The owner chose this
package on 2026-10-15 (answer 3 of `docs/06-sessions/2026-10-15-P0-02.md`). It is Done once every
criterion below has passed on the final content commit, a fresh-context review is complete and the
owner has merged it (engine-spec §8.16.2). No criterion below is ticked; the runs on the final
content commit are recorded in `.ai/evidence/P0-02/`.

## Ownership and risk
- **Owner.** Product/Architecture owner. The repository owner holds every role today.
- **Implementer.** Claude Code (the implementing agent), with a drafting-and-verification workflow.
- **Reviewer.** Pending: a fresh-context reviewer who is not the implementer, then the owner.
- **Risk class: Low.** Documentation only, apart from one doc-comment sentence in
  `crates/application/src/lib.rs`. It records rulings the owner has already given and decides nothing.
- **Required human approvals.** The owner's merge of this package's pull request, which confirms the
  transcription. The rulings themselves are already given (PR #4 comment 6185953773, as
  edited by the owner; answers 1 to 3 of the 2026-10-15 record).
- **Agent budget (engine-spec §8.18.1).** One implementing session; one writer per file; fewer than
  ten concurrent read-only agents. This package declares no maximum number of agent turns,
  wall-clock timeout or limit on concurrent writers, which deviates from §8.18.1, as in P0-01.

## Authority
- Engine spec (`engine-spec.md`): §1.5, §1.7, §8.16, §8.17, §9.5, Appendix F.
- ADRs: ADR-011, ADR-012, ADR-013 and `docs/02-adr/README.md` ("Status", "Partial supersession").
- Register: `docs/00-meta/decision-register.md`, "How to ratify or override".

## Owner decisions depended on
All given; none is unratified.

- **DR-A1, DR-A2, DR-A4 to DR-A9, DR-A11, DR-A12.** Ratified 2026-10-08 and named again in the
  owner's P0-01 merge review, PR #4 comment 6185953773 (2026-10-15).
- **DR-A10.** Ruled 2026-10-15, option 1, `MIT OR Apache-2.0`: PR #4 comment 6185953773 as edited
  by the owner on 2026-10-15 (answer 1 of the 2026-10-15 record).
- **ADR-011, ADR-012 and ADR-013.** Accepted 2026-10-15 in PR #4 comment 6185953773. Answer 2 of
  the record directs the agent to transcribe the acceptance into their status.
- **DR-A5.** This package's ID (answer 3).

## Objective
No maintained document in the repository still describes the P0-01 merge review, the DR-A10 ruling,
the merge or the acceptance of ADR-011, ADR-012 and ADR-013 as pending. Session records, evidence
records and earlier daily-log entries are history and stay as written.

## Observable outcome
None for engine consumers. A maintainer sees ADR-011, ADR-012 and ADR-013 as `Accepted`, the
`reference/python/` licence section citing the ruling, P0-01 and P1-00 as Done, and DR-A10 ruled in
the register and in `engine-spec.md` Appendix F.1.

## Preconditions
- P0-01 merged (`b819ae0`, 2026-10-15).
- The owner's correction of the DR-A10 licence sentence: an edit of PR #4 comment 6185953773
  (2026-10-15, 12:57Z).

## Scope
- **Contracts consumed.** None.
- **Contracts changed.** None in substance. `docs/03-contracts/README.md` changes only the status
  wording of DR-A2, and `parity-fixture-contract.md` only that of DR-A11 and DR-D30. The provider
  manifest `docs/04-providers/nflverse/access-and-license.md` changes only that of DR-A11.

### Paths affected
- `CLAUDE.md`
- `README.md`
- `crates/application/src/lib.rs` (one doc-comment sentence: DR-A8 is ratified)
- `docs/CLAUDE.md`
- `docs/00-meta/authority-index.md`
- `docs/00-meta/daily-log.md`
- `docs/00-meta/dashboard.md`
- `docs/00-meta/decision-register.md`
- `docs/00-meta/known-issues.md`
- `docs/00-meta/specs/engine-spec.md`
- `docs/00-meta/specs/superseded/README.md` (the maintained index only)
- `docs/01-work-packages/README.md`
- `docs/01-work-packages/p0-01-engine-consolidation.md`
- `docs/01-work-packages/p0-02-record-p0-01-merge-rulings.md`
- `docs/01-work-packages/p1-00-work-package.md`
- `docs/02-adr/001-repo-bootstrap-decisions.md` (front-matter `superseded-by` only)
- `docs/02-adr/006-deferred-deliverables.md` (front-matter `superseded-by` only)
- `docs/02-adr/007-verify-covers-guards-and-doctests.md` (front-matter `superseded-by` only)
- `docs/02-adr/009-verify-ps1-exit-codes.md` (front-matter `superseded-by` only)
- `docs/02-adr/011-engine-only-pivot.md`
- `docs/02-adr/012-python-reference-oracle.md`
- `docs/02-adr/013-feeder-target-season-level.md`
- `docs/02-adr/README.md`
- `docs/03-contracts/README.md`
- `docs/03-contracts/parity-fixture-contract.md`
- `docs/04-providers/nflverse/access-and-license.md`
- `docs/05-model-specs/README.md`
- `docs/05-model-specs/cross-league-priors.md`
- `docs/05-model-specs/evaluation-and-leakage.md`
- `docs/05-model-specs/layer1-credit.md`
- `docs/05-model-specs/projection-stack.md`
- `docs/05-model-specs/rapm-attribution.md`
- `docs/05-model-specs/state-space-kalman.md`
- `docs/05-model-specs/synthetic-world.md`
- `docs/05-model-specs/value-model.md`
- `docs/06-sessions/2026-10-15-P0-02.md` (new record)
- `docs/06-sessions/README.md` (one index row and the maintained list of remaining actions)
- `docs/07-archive/README.md` (the archive's own index)
- `docs/07-archive/cautious-nevermore/HISTORY.md` (written for the archive, one sentence)
- `docs/07-archive/cautious-nevermore/MANIFEST.md` (written for the archive, one sentence)
- `engine-spec.md`
- `reference/python/PARITY.md`
- `reference/python/README.md`
- `reference/python/tools/investigations/README.md`
- `.ai/evidence/P0-02/`

## Non-goals
- Any decision beyond the owner's PR #4 comment 6185953773 (as edited) and the three answers of the
  2026-10-15 record.
- Any change to P0-01's or P1-00's evidence record: each attests to its own final commit and is never
  rewritten (`scripts/check-evidence-claims.sh`).
- Any oracle change: nothing under `reference/python/backend/`, `reference/python/tests/` or
  `reference/python/MANIFEST.tsv`.
- Any session record other than the new one; in `docs/06-sessions/README.md`, anything but its index
  row and its maintained list of remaining actions.
- Engine code, verification recipes, CI and guards.
- The owner's open actions: the DR-D30 branch-protection setting, DR-A3's follow-up ADR, the
  KI-NEW-Z79 decision request and the P1-12 pre-registrations.

## Inputs and fixtures
Not applicable: no fixture or data input.

## Oracle parity targets
None.

## Implementation constraints
- ADR-011, ADR-012 and ADR-013 become immutable once Accepted, so their bodies are final in this
  package. ADR-001 to ADR-010 change only in front-matter `superseded-by`.
- `engine-spec.md` and `docs/00-meta/specs/engine-spec.md` stay byte-identical, and the register's
  summary rows keep matching Appendix F.1.
- Every "(proposed — DR-xx)" and "(open — DR-xx)" tag on an open item stays.
- No model identifier is written into any file.
- In `docs/00-meta/specs/superseded/` only the maintained `README.md` index changes (`docs/CLAUDE.md`,
  "Documentation Vault"), in `docs/06-sessions/` only the maintained `README.md` and the new record,
  and in `docs/07-archive/` only the files written for the archive (`README.md`, and `MANIFEST.md` and
  `HISTORY.md` under `cautious-nevermore/`); the archived specs, the existing session records and the
  archived copies are untouched.

## Security and licensing considerations
The DR-A10 ruling brings the CN-derived code in `reference/python/`, and later corrections to it under
`PARITY.md`, under `MIT OR Apache-2.0`. It covers code only: real third-party data stays excluded
(DR-A11), and so does `reference/python/backend/db/data/coaching_changes_2025.json` as data
(KI-NEW-D1). No secret, permission, host or dependency changes.

## Acceptance criteria
Each criterion is verified on the final content commit.

- [ ] `just verify` exits 0, and `pwsh scripts/verify.ps1 -Scope Full` exits 0 where pwsh is
      available. The `windows-authoritative` job is green on the pull request; it is the merge gate
      (DR-A3). The `reference-oracle` job is green (DR-D30).
- [ ] `./scripts/check-authority-sync.sh` passes: the spec mirror is byte-identical.
- [ ] Every register summary row's Status and Blocks cells equal the matching Appendix F.1 row's.
- [ ] `grep -L '^status: Accepted' docs/02-adr/011-*.md docs/02-adr/012-*.md docs/02-adr/013-*.md`
      prints nothing.
- [ ] `git grep -n -E 'CORRECTION[_](ID|RECORD)|WORK[_]COMPLETED|VERIFICATION[_]RUN'` prints nothing.
- [ ] `git diff --quiet b819ae0 HEAD -- .ai/evidence/P0-01 .ai/evidence/P1-00 reference/python/backend reference/python/tests reference/python/MANIFEST.tsv`
      exits 0.
- [ ] `(cd reference/python && python3 tools/verify_manifest.py)` passes.
- [ ] `./scripts/check-traceability.sh` and `./scripts/check-evidence-claims.sh` pass against
      `origin/main`.
- [ ] A fresh-context review of the diff is complete, and every finding is dispositioned.

## Verification
```text
# targeted
bash tests/guards/run.sh
./scripts/check-authority-sync.sh
./scripts/check-traceability.sh origin/main
typos
git grep -n -E 'CORRECTION[_](ID|RECORD)|WORK[_]COMPLETED|VERIFICATION[_]RUN'
(cd reference/python && python3 tools/verify_manifest.py)

# canonical
just verify
pwsh scripts/verify.ps1 -Scope Full

# evidence (after the final content commit)
just evidence P0-02
./scripts/check-evidence-claims.sh origin/main P0-02
```
Verification recipes are a frozen contract. Make the repository satisfy them; never edit a recipe to
make a failure disappear.

## Numerical/performance tolerances
Not applicable: no computation.

## Migration and rollback
No migrations. To roll back, revert this package's merge commit. The owner's rulings stand in PR #4
comment 6185953773 either way.

## Evidence required
`.ai/evidence/P0-02/verification-input.json` (the recorded runs), `manifest.json` generated by
`just evidence P0-02` (never hand-written) and `PR-BODY.md`. `ai-toolchain.lock` provenance stays
"unrecorded", as in P0-01: the implementing agent's harness policy forbids writing model identifiers
into repository artifacts. This deviates from engine-spec §8.20, which requires the model and harness
actually used.

## Stop/decision conditions
Stop and ask the owner if:
- a statement can be made true only by deciding something the owner's comment and answers do not
  settle;
- the corrected licence sentence of PR #4 comment 6185953773 names terms other than answer 1's
  `MIT OR Apache-2.0` (its lowercase "or" names the same repository terms, ADR-001 D4);
- a guard fails in a way only a recipe change would fix.

## Follow-up
- **DR-D30.** The owner makes `reference-oracle (Linux only)` a required status check on `main`.
  Until then a failing oracle run does not block a merge.
- **DR-A3.** The follow-up ADR, due before P1-11. Until then Windows stays merge-authoritative.
- **KI-NEW-Z79.** Raise its decision request. Until it is decided, the training features of the live
  context model `g′_S` carry team data from before the window.
- **P1-12 pre-registrations** (DR-D14, DR-D17, DR-D19 with DR-C5 and DR-C9). Until they are made,
  P1-12 cannot tune, gate or implement the affected steps.
