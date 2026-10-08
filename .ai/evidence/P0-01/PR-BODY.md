# P0-01 — Consolidate into an engine-only GRID Engine repository

## Outcome

GRID-Engine is now dedicated to building the GRID engine and nothing else. The specs and the work that
had actually been done in **GRID-One/GRID-Engine** and **Seismic-Fate/cautious-nevermore** are now in
this one repository:

- **`engine-spec.md`** is the new top authority (ADR-011). It consolidates `alpha-spec.md` and
  `final-build-spec.md` to engine-only scope and adds the GRID signal stack, the reference-oracle parity
  regime, the synthetic-world validation contract and an engine-only work sequence (P1-01..P1-12,
  P2-00..P2-09). Appendix H maps every superseded section to its new home; the originals are archived
  verbatim in `docs/00-meta/specs/superseded/`.
- **`reference/python/`** holds the working cautious-nevermore Python engine as the executable
  reference oracle (ADR-012). It has 105 files: 103 byte-identical and 2 documented patches. It is
  hash-manifested and isolation-guarded, and its 446 tests pass. Rust is the target; the oracle supplies
  parity targets and is never shipped. You chose this direction on 2026-10-01: Rust target plus a Python
  reference.
- **The app is removed.** `app/` (Flutter), `crates/ffi/`, the Flutter toolchain pin, the
  `flutter_rust_bridge` dependency and the `test-ffi`/`serve-ui` recipes are gone. This is
  de-scoping, not weakening: `test-ffi` ran 0 tests.
- **Knowledge is preserved and indexed:**
  - eight model specs, each derived from the oracle code with its defects and port plan;
  - three contracts;
  - nflverse and CFBD provider docs;
  - a decision register of 63 owner decisions (DR-A1..A12 adopted here pending your ratification;
    B, C and D proposed or open);
  - a known-issues backlog, re-verified against code;
  - lessons learned;
  - the cautious-nevermore engine history (non-authoritative) in `docs/07-archive/`;
  - the P1-00 review rounds and this consolidation's inventory in `docs/06-sessions/`.
- **PR #1 is included.** This branch fast-forwards over PR #1's 49 commits. Its round-4 findings R4-1
  (major) and R4-2 (minor) are fixed here, with mutation-verified guard cases.

## Things you should know before reading the specs

- **The synthetic oracle has a structural bug (KI-NEW-Y0).** `synth.py:193` draws every play's defenders
  from the offense's own team. So no team-strength, DEF or matchup recovery claim was ever really
  validated. All synthetic recovery and golden numbers are legacy-generator values, not parity targets.
- **GRID has not cleared the Phase-1 gate on real data.** The cautious-nevermore real-data verdict is
  historical and non-citable: ROS ties last-season at −0.015 and loses to season-to-date mean at −0.194.
  The weekly +0.848 used in-season participation data that nflverse only publishes after the season, and
  the box-score labels were biased.
- **None of the statistical choices are settled.** The spec adopts recommended defaults, but every one is
  tagged *(proposed — DR-xx)* or *(open — DR-xx)*. A work package is not Ready until the decisions it
  depends on are ratified.

## Verification (all run on the final content commit)

- `just verify` exits 0. It runs **8 recipes**, with nextest 3/3. `tests/guards/run.sh`
  carries 78 committed behaviour cases, up from 54.
- `pwsh scripts/verify.ps1 -Scope Full` exits 0 under PowerShell 7.4.6 on Linux. The
  `windows-authoritative` CI job on this PR is the Windows evidence.
- `justfile`, `verify.sh` and `verify.ps1` agree at **15 steps**. `verify.ps1` guards every
  native command with `Assert-Ok` (15/15).
- `Cargo.lock` resolves 171 crates (only `grid-ffi` removed; no version changed). A fresh clone
  compiles offline.
- Traceability: all 264 non-trivial paths are traced against `origin/main`.
- Reference oracle: `verify_manifest.py` OK (105 rows), and 446 tests passed single-threaded on Linux
  with 0 isolation-guard violations. The new Linux-only `reference-oracle` CI job runs it outside the
  frozen verify chain.

## Completion record (Appendix C)

```text
Work package:                    P0-01 (docs/01-work-packages/p0-01-engine-consolidation.md)
Final commit:                    a4f62f7c7cef  (the commit the manifest attests to; the evidence
                                 commit carrying it follows and is not self-covered)
Model/harness identifier:        unrecorded (see Known limitations); harness Claude Code on the web
Environment:                     Linux 6.18.44-fc-v80; rustc 1.98.1; Python 3.11.15 for the oracle
Authority documents read:        alpha-spec.md, final-build-spec.md (now superseded), ADR-001..010,
                                 PR #1 reviews rounds 1-4, cautious-nevermore@59bce1d code and docs
Contracts changed:               new docs/03-contracts/{plays-contract,engine-output-contract,
                                 parity-fixture-contract}.md (Draft)
Migrations changed:              none
Dependencies changed:            removed flutter_rust_bridge (workspace) and grid-ffi; none added
Targeted tests:                  tests/guards/run.sh 78/78; reference oracle 446/446
Canonical verification command:  just verify (Linux smoke); verify.ps1 -Scope Full is merge-authoritative
Verification exit status:        0 (just verify); 0 (verify.ps1 under pwsh on Linux)
Golden files changed and approval: none (the oracle golden is imported verbatim)
Performance evidence:            n/a (no engine computation yet)
Security/licensing review:       check-secrets OK; reference/python licence ruling pending (DR-A10)
Fresh-context reviewer:          not yet performed (required before merge)
Reviewer findings resolved:      n/a
Human approvals:                 owner's 2026-10-01 direction choice; no diff sign-off yet
Known limitations:               see below
Evidence manifest hash:          sha256:7b2aafeaa532a89c09be6095fcc827946bcf8cdf44a99e41b3f94a097aa29f9c
Owner decisions depended on:     DR-A1..A12 (adopted here, pending ratification at merge)
```

## Known limitations

- **`ai-toolchain.lock` provenance is "unrecorded" for P0-01.** The implementing agent's harness policy
  forbids writing model identifiers into repository artifacts. This deviates from engine-spec §8.20; you
  may record the real values.
- **The branch name was set by the harness** (`claude/grid-engine-consolidation-e7kmh9`), not `wp/P0-01-…`.
- **Windows and the runner's Python are proven only by this PR's first CI runs.**
- **Drafts and unratified decisions.** The contracts and model specs are drafts. B, C and D decisions
  are proposed or open.

## Before merging

1. **Licence ruling (DR-A10).** Record a licence ruling on this PR for `reference/python/`; the
   cautious-nevermore repository has no LICENSE file.
2. **Accept and ratify.** Accept ADR-011 and ADR-012, and ratify (or override) the A-decisions in the
   merge comment.
3. **Merge with a merge commit, never squash.** That keeps PR #1's attested commit `8d43203` reachable
   (DR-A6).
4. **Close the earlier PRs.** Close PR #1 as included, and close PRs #2 and #3 unmerged; their review
   records are already in `docs/06-sessions/`.
5. **Fresh-context review.** Run the review (engine-spec §7.11, §12.9).

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01PG15VStSfELgz5wh3pBoMo
