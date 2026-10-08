# P0-01 — Consolidate into an engine-only GRID Engine repository

## Outcome

GRID-Engine is now dedicated to building the GRID engine and nothing else. The specs and the work that
had actually been done in **GRID-One/GRID-Engine** and **Seismic-Fate/cautious-nevermore** are now in
this one repository:

- **`engine-spec.md`** is the new top authority (ADR-011). It consolidates `alpha-spec.md` and
  `final-build-spec.md` to engine-only scope, and adds:
  - the GRID signal stack;
  - the reference-oracle parity regime;
  - the synthetic-world validation contract;
  - an engine-only work sequence (P1-01..P1-12, P2-00..P2-09).

  Appendix H maps every superseded section to its new home. The originals are archived verbatim in
  `docs/00-meta/specs/superseded/`.
- **`reference/python/`** holds the working cautious-nevermore Python engine as the executable reference
  oracle (ADR-012). It has 105 files: 102 byte-identical, 2 documented patches, and correction-ledger
  entry L0. It is hash-manifested, isolation-guarded and platform-pinned, and its 446 tests pass. Rust
  is the target; the oracle supplies parity targets and is never shipped. You chose this direction on
  2026-10-01.
- **The app is removed.** `app/` (Flutter), `crates/ffi/`, the Flutter toolchain pin, the
  `flutter_rust_bridge` dependency and the `test-ffi`/`serve-ui` recipes are gone. This is de-scoping,
  not weakening: `test-ffi` ran 0 tests.
- **Knowledge is preserved and indexed:**
  - eight model specs and three contracts;
  - nflverse and CFBD provider docs;
  - a decision register of 64 owner decisions;
  - a known-issues backlog, re-verified against code;
  - lessons learned;
  - the cautious-nevermore engine history (non-authoritative) in `docs/07-archive/`;
  - the P1-00 review rounds, this consolidation's inventory, and its own review record in
    `docs/06-sessions/`.
- **PR #1 is included.** This branch fast-forwards over PR #1's 49 commits. Its round-4 findings R4-1
  (major) and R4-2 (minor) are fixed here, with mutation-verified guard cases.

## Decisions

| Decision | Status |
|---|---|
| DR-A1, A2, A4–A9, A11, A12 | Adopted by this PR; in force once your merge comment ratifies them (or overrides any) |
| DR-A3 | The default is kept (Windows stays merge-authoritative); the follow-up ADR is due before P1-11 |
| DR-A10 | **Your licence ruling for `reference/python/` is needed on this PR before merge** |
| DR-D31 | **Ratified 2026-10-08**, in writing: "I approve DR-D31 option 1: regenerate the Layer C golden under Python 3.11 with the Haswell pin, recorded as ledger entry LO" (L0). Implemented here as correction-ledger entry L0 |
| DR-B1..B6, DR-C1..C15, DR-D1..D30 | Proposed or open. Not settled. A work package is not Ready while a decision it depends on is unratified |

## Things you should know before reading the specs

- **The synthetic oracle has a structural bug (KI-NEW-Y0).** `synth.py:193` draws every play's defenders
  from the offense's own team, so no team-strength, DEF or matchup recovery claim was ever really
  validated. All synthetic recovery and golden numbers are legacy-generator values, not parity targets.
- **GRID has not cleared the Phase-1 gate on real data.** The cautious-nevermore real-data verdict is
  historical and non-citable:
  - ROS ties last-season at −0.015 and loses to season-to-date mean at −0.194;
  - the weekly +0.848 used in-season participation data that nflverse only publishes after the season;
  - the box-score labels were biased.
- **The oracle's golden is pinned to one numerical platform (KI-NEW-Z78, DR-D31).** The imported golden
  reproduced only on CPython 3.11 with Intel AVX-512 math kernels, so this PR's first CI runs failed its
  two Layer C tests on GitHub's AMD runners. Ledger entry L0 regenerated Layer C once under CPython 3.11
  with `OPENBLAS_CORETYPE=Haswell`, which every x86-64 AVX2 CPU runs:
  - only Layer C moved (max |Δ| 1.28e-2, kernel noise through the gradient-boosted credit);
  - a pytest plugin enforces the pin;
  - the tolerance is unchanged and no test is skipped.

## Verification (all run on the final content commit)

- `just verify` exits 0. It runs **8 recipes**, with nextest 3/3. `tests/guards/run.sh`
  carries 78 committed behaviour cases, up from 54.
- `pwsh scripts/verify.ps1 -Scope Full` exits 0 under PowerShell 7.4.6 on Linux. The
  `windows-authoritative` CI job on this PR is the Windows evidence.
- `justfile`, `verify.sh` and `verify.ps1` agree at **15 steps**. `verify.ps1` guards every native
  command with `Assert-Ok` (15/15).
- `Cargo.lock` resolves 171 crates (only `grid-ffi` removed; no version changed). A fresh clone
  compiles offline.
- Traceability: all 267 non-trivial paths traced against `origin/main`.
- Reference oracle:
  - `verify_manifest.py` OK (102 verbatim, 3 patched), also with `--upstream` against
    cautious-nevermore;
  - 446 passed under the pin, with 0 isolation-guard violations;
  - the new Linux-only `reference-oracle` CI job runs it outside the frozen verify chain, on the
    runner's tool-cache CPython 3.11 with the Haswell pin, and is expected green on this head.
- Fresh-context review: one round, three independent lenses, 26 findings (0 blockers). All are
  dispositioned as fixed in `docs/06-sessions/review-P0-01-adversarial-round1.md`.

## Completion record (Appendix C)

```text
Work package:                    P0-01 (docs/01-work-packages/p0-01-engine-consolidation.md)
Final commit:                    ee4a7e411050  (the commit the manifest attests to; the evidence
                                 commit carrying it follows and is not self-covered)
Model/harness identifier:        unrecorded (see Known limitations); harness Claude Code on the web
Environment:                     Linux 6.18.44-fc-v80; rustc 1.98.1; CPython 3.11.15 + OPENBLAS_CORETYPE=Haswell for the oracle
Authority documents read:        alpha-spec.md, final-build-spec.md (now superseded), ADR-001..010,
                                 PR #1 reviews rounds 1-4, cautious-nevermore@59bce1d code and docs
Contracts changed:               new docs/03-contracts/{plays-contract,engine-output-contract,
                                 parity-fixture-contract}.md (Draft)
Migrations changed:              none
Dependencies changed:            removed flutter_rust_bridge (workspace) and grid-ffi; none added
Targeted tests:                  tests/guards/run.sh 78/78; reference oracle 446/446 under the pin
Canonical verification command:  just verify (Linux smoke); verify.ps1 -Scope Full is merge-authoritative
Verification exit status:        0 (just verify); 0 (verify.ps1 under pwsh on Linux)
Golden files changed and approval: reference/python/tests/grid/golden/snapshot.npz, Layer C only,
                                 correction-ledger entry L0; approved by the owner in writing on
                                 2026-10-08 (DR-D31 option 1)
Performance evidence:            n/a (no engine computation yet)
Security/licensing review:       check-secrets OK; reference/python licence ruling pending (DR-A10)
Fresh-context reviewer:          three independent read-only agents, round 1 at f3fb390
Reviewer findings resolved:      26/26 (docs/06-sessions/review-P0-01-adversarial-round1.md)
Human approvals:                 2026-10-01 direction (Rust target + Python reference); 2026-10-08
                                 DR-D31 option 1 / ledger entry L0; no diff sign-off yet
Known limitations:               see below
Evidence manifest hash:          sha256:69aaeaa502b6ba02a3ecf7afeaecc2edc13c610b1aee90eb42c6a5eb740b83cd
Oracle parity result:            no Rust component yet; the oracle's own goldens pass under the pin
Reference-oracle job result:     red on the first two heads (KI-NEW-Z78); expected green on this head
Owner decisions depended on:     DR-A1..A12 (see Decisions); DR-D31 ratified 2026-10-08
```

## Non-goals

No Rust engine computation (the crates stay stubs until P1-01 and later). No oracle corrections other
than L0; the B-1 ledger (KI-NEW-Y0 first) is proposed, not applied. No change to cautious-nevermore. No
app, UI, FFI or installer.

## Migration and rollback

There are no migrations. To roll back, revert the PR's merge commit. The superseded specs, PR #1's
history and cautious-nevermore are unchanged and stay available. The legacy golden is in git history and
in cautious-nevermore, and `verify_manifest --upstream` proves the L0 patch.

## Known limitations

- **`ai-toolchain.lock` provenance is "unrecorded" for P0-01.** The implementing agent's harness policy
  forbids writing model identifiers into repository artifacts. This deviates from engine-spec §8.20; you
  may record the real values.
- **The branch name was set by the harness** (`claude/grid-engine-consolidation-e7kmh9`), not `wp/P0-01-…`.
- **Windows is proven only by this PR's CI.**
- **Drafts and unratified decisions.** The contracts and model specs are drafts. B, C and D decisions
  are proposed or open, except DR-D31.

## Before merging

1. **Licence ruling (DR-A10).** Record a licence ruling on this PR for `reference/python/`; the
   cautious-nevermore repository has no LICENSE file.
2. **Accept and ratify.** Accept ADR-011 and ADR-012, and ratify (or override) the A-decisions in the
   merge comment.
3. **Merge with a merge commit, never squash.** That keeps PR #1's attested commit `8d43203` reachable
   (DR-A6).
4. **Close the earlier PRs.** Close PR #1 as included, and close PRs #2 and #3 unmerged; their review
   records are already in `docs/06-sessions/`.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01PG15VStSfELgz5wh3pBoMo
