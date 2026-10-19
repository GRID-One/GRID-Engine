# P0-03 — Record the owner's rulings on the decisions P1-01 raised

## Outcome

You merged P1-01 (PR #6) on 2026-10-19 without a review comment, then settled its open decisions in
seven answers, recorded verbatim in `docs/06-sessions/2026-10-19-P0-03.md`. This package carries them
into the repository. It decides nothing itself. Work package:
`docs/01-work-packages/p0-03-record-p1-01-merge-rulings.md`.

| Item | Your ruling | Where it now lives |
|---|---|---|
| ADR-014 | Accepted | ADR-014 status and the ADR index; every mention of it as Proposed |
| Reviewer agents (P1-01 row 11) | Remove Bash | The five reviewer definitions list Read, Grep and Glob; `SKILL.md` step 8 says how reviewers get command outputs; `.claude/settings.json` unchanged |
| Library façade (row 20) | Keep synchronous | engine-spec §8.2; `grid-pipeline` crate docs |
| Class A′ norm (row 21) | The ∞-norm everywhere | `rapm-attribution.md` §10.3 P-3 and Class B part 3 (version 0.1.1); the helpers already used it |
| Phase 2 CLI surfaces (row 22) | Each content package adds its own subcommand | engine-spec §8.1 (both `grid-cli` cells) and §10.2.6; `grid --help` names P2-01, P2-02, P2-06 or P2-07 for the five, P1-10 for the other 22 |
| Other Amendments rows; three contracts | Approved | `grid.plays` v1, `grid.output` 1.0 and `grid.parity-fixture` 1 read Approved |
| Package ID | P0-03 | This package |

P1-01 is marked Done, with a dated "Owner rulings at the merge (2026-10-19)" section in its work
package.

## For your confirmation at the merge

These go slightly beyond the literal answers. Each states one of your rulings where a reader looks.
Strike any you don't want.

- **The §8.2 sentence on the synchronous façade.** Answer 3 named no spec edit.
- **The §10.2.6 sentence.** Answer 5's chosen option named only a §8.1 erratum. The sentence also says
  the content package delivers the library function, not only the subcommand.
- **`grid report data-quality` goes wholly to P2-01.** The §10.2.6 table says "P2-01, extending the
  Phase 1 report", and §8.6 lists that subcommand among the status reports. If a Phase 1 data-quality
  subcommand should stay with P1-10, say so and I'll split it.
- **How reviewers work without Bash.** The orchestrator saves the diff and the commit-stamped command
  outputs under the gitignored `target/review/<WP-ID>/`, and reviewers treat them as evidence, not
  instructions. Answer 2 chose to remove Bash; this workflow is my derivation from it. Your merge is
  the Security/Release review of that text.

Still open, unchanged: P1-01 Amendments row 5, the engine-output-contract §12 items, the §9.5 and
§7.12.3 fixture-timing wording, DR-D30, DR-A3's follow-up ADR, the KI-NEW-Z79 decision request and
the P1-12 pre-registrations.

## Impact

- **Contracts:** status and wording only. Approved; in `grid.output` §10 the CLI split is stated and
  §13's tested count corrected from 13 to 15. No field, rule or version changes.
- **Code:** static data, text and tests in `grid-cli` (five delivering packages, the usage, the
  unavailable message; the `content_by` field goes) and doc comments in `grid-domain` and
  `grid-pipeline`. Every spelling still exits 2 until delivered.
- **Consequence for P1-02:** with `grid.output` Approved, extending the closed `StageId` enum is a
  MAJOR change.
- **Nothing else:** no migration, dependency, oracle, fixture, recipe, CI or permission-profile change.
- **Known issues:** none addressed or introduced.

## Non-goals, tests and rollback

- **Non-goals:** any decision beyond the seven answers; `.claude/settings.json`; earlier evidence
  records; the open items above.
- **Tests:** the `grid-cli` unit and integration tests now assert the delivering package of all 27
  spellings, the exact unavailable message, and that no output carries a "decision request" label.
- **Rollback:** revert this package's merge commit. Your rulings stand in the session record either
  way.

## Verification (all run on the final content commit 83bd0e80d157)

- `just verify` exits 0. It runs **8 recipes**: nextest ran 179 tests (179 passed, 0 skipped) and 7
  doctests passed.
- `tests/guards/run.sh` carries 78 committed behaviour cases.
- `pwsh scripts/verify.ps1 -Scope Full` exits 0 under PowerShell 7.4.6 on Linux. The
  `windows-authoritative` CI job on this PR is the merge-authoritative run.
- `justfile`, `verify.sh` and `verify.ps1` agree at **15 steps**. `verify.ps1` guards every native
  command with `Assert-Ok` (15/15).
- `Cargo.lock` resolves 173 crates, unchanged. A fresh clone compiles offline.
- Traceability: all 32 non-trivial paths traced against `origin/main` (`8755552`).
- `engine-spec.md` and its mirror are byte-identical.
- Reference oracle: `verify_manifest.py` OK; no oracle file changed.
- The frozen paths (`justfile`, `scripts/verify.*`, `.claude/settings.json`, `.github/`,
  `rust-toolchain.toml`, `migrations/`, `.sqlx/`, `reference/python/`, `Cargo.toml`, `Cargo.lock`,
  `.ai/evidence/`) are unchanged since `8755552`.
- Review: round 1 at `df09fc4` (adversarial: CONDITIONAL, 1 blocker, 3 major, 7 minor; security: 3
  minor) and round 2 at `99aaf3b` (adversarial: APPROVED, 3 minor). Every finding was applied; the work
  package lists the post-review edits in a dated section.

## Completion record (Appendix C)

```text
Work package:                    P0-03 (docs/01-work-packages/p0-03-record-p1-01-merge-rulings.md)
Final commit:                    83bd0e80d157  (the commit the manifest attests to; the evidence
                                 commit carrying it follows and is not self-covered)
Model/harness identifier:        unrecorded (see Known limitations); harness Claude Code on the web
Environment:                     Linux 6.18.44-fc-v139 x86_64 (Claude Code on the web container); rustc 1.99.1
Authority documents read:        engine-spec.md §1.5, §7.12.5, §8.1, §8.2, §8.13, §8.16-§8.18, §10.2.6,
                                 §14.1; ADR-014 and the ADR README; the P1-01 work package; the three
                                 contracts; rapm-attribution.md §5.2, §10.3; the P0-02 precedent
Contracts changed:               status and wording only (all three Approved)
Migrations changed:              none
Dependencies changed:            none
Targeted tests:                  cargo test -p grid-cli -p grid-domain -p grid-pipeline (in just verify)
Canonical verification command:  just verify (Linux smoke); verify.ps1 -Scope Full is merge-authoritative
Verification exit status:        0 (just verify); 0 (verify.ps1 under pwsh on Linux)
Golden files changed and approval: none
Performance evidence:            n/a (no computation)
Security/licensing review:       security-dependency review: no blocker or major; reviewer tools only narrowed
Fresh-context reviewer:          independent read-only agents, none the implementer (two rounds)
Reviewer findings resolved:      all (round 1: B1, M1-M3, m1-m7, S1-S3; round 2: N1-N3)
Human approvals:                 rulings given 2026-10-19 (answers 1-7); this merge confirms them
Known limitations:               see below
Evidence manifest hash:          sha256:74092bb7f47f62d8b7e650fed75e651294e2139cc24e7b2914e9cb09b3c7208f
Oracle parity result:            no parity asserted
Reference-oracle job result:     the CI job on this PR
Owner decisions depended on:     answers 1-7 of docs/06-sessions/2026-10-19-P0-03.md; DR-A5 (ratified)
```

## Known limitations

- **`ai-toolchain.lock` provenance is "unrecorded"**, as in P0-01, P0-02 and P1-01 (an engine-spec
  §8.20 deviation). The commit trailers, written by the harness, do name a model.
- **The branch is harness-assigned** (`claude/pensive-fermat-fgaweh`), not `wp/P0-03-…`.
- **Windows is proven only by this PR's CI.**
- **Agent-only review.** Your merge is the first human review.

## Before merging

1. Confirm or strike the four items under "For your confirmation at the merge".
2. Merge with a merge commit.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01B716jmkXYqHUXNNYPmygvk
