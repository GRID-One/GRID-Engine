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
  - a decision register of 64 owner decisions, most of them settled by your decision interview of
    2026-10-08/09 (`docs/06-sessions/2026-10-09-P0-01-decision-interview.md`);
  - a known-issues backlog, re-verified against code;
  - lessons learned;
  - the cautious-nevermore engine history (non-authoritative) in `docs/07-archive/`;
  - the P1-00 review rounds, this consolidation's inventory, and its own review record in
    `docs/06-sessions/`.
- **PR #1 is included.** This branch fast-forwards over PR #1's 49 commits. Its round-4 findings R4-1
  (major) and R4-2 (minor) are fixed here, with mutation-verified guard cases.

## Decisions

You answered every open register entry except DR-D9 and DR-D10 in a decision interview (62 questions,
2026-10-08/09). The record quotes every answer verbatim, and `docs/00-meta/decision-register.md` carries
the outcomes. The spec, model specs, contracts and governance documents now cite a settled decision
plainly, and keep "(proposed — DR-xx)" or "(open — DR-xx)" only where something is still open.

| Decision | Status |
|---|---|
| DR-A1, A2, A4–A9, A11, A12 | **Ratified 2026-10-08** in the interview; to be named again in your merge comment |
| DR-A3 | Direction ratified (Linux authoritative, Windows kept as an extra job); Windows stays merge-authoritative until the follow-up ADR, due before P1-11 |
| DR-A10 | **Option 1 chosen** (MIT OR Apache-2.0, you as sole author); **your ruling comment on this PR is still needed before merge** |
| ADR-011, ADR-012 | Still Proposed; you chose to accept them in the merge comment |
| DR-D31 | Ratified 2026-10-08 (ledger entry L0) |
| B, C and D: settled | 27: 18 ratified as recommended; 3 overridden (DR-B3, DR-D26, DR-D30); 6 decided where the register had no default (DR-D5, D7, D11, D17, D22, D24). Some still name follow-on work: each DR-B1 ledger entry needs its own approval, and DR-B4's recalibrated values come later |
| B, C and D: settled in part | 19, each with its open part named in the register: 10 ratified in part, 3 overridden in part (DR-C6, DR-C12, DR-C13), 6 decided in part |
| Still open | DR-D2, DR-D3, DR-D13 (interim rules and directions confirmed); DR-D9, DR-D10 (not asked: they need real-data measurements) |
| Found in review, for you before P1-12 | DR-D17: the starting prior for a player with NFL history; DR-D14: plays whose defender has no fold-wise rating |

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
- Traceability: all 268 non-trivial paths traced against `origin/main`.
- Reference oracle:
  - `verify_manifest.py` OK (102 verbatim, 3 patched), also with `--upstream` against
    cautious-nevermore;
  - 446 passed under the pin, with 0 isolation-guard violations;
  - the new Linux-only `reference-oracle` CI job runs it outside the frozen verify chain, on the
    runner's tool-cache CPython 3.11 with the Haswell pin. It passed on head `05aba0a`, with all four
    CI jobs green.
- Fresh-context review:
  - consolidation: one round, three independent lenses, 26 findings (0 blockers), all fixed
    (`docs/06-sessions/review-P0-01-adversarial-round1.md`);
  - decision recording: a drafter and an adversarial verifier per document chunk, then seven reviewers
    over the applied diff, 79 findings (0 blockers). 74 were applied, 2 were judged correct as written,
    and 3 needed your input: one (DR-D14) is now recorded as open, and two were answered from the
    session transcript.

## Completion record (Appendix C)

```text
Work package:                    P0-01 (docs/01-work-packages/p0-01-engine-consolidation.md)
Final commit:                    3d8448215afe  (the commit the manifest attests to; the evidence
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
Fresh-context reviewer:          independent read-only agents: round 1 at f3fb390; the
                                 decision-recording review at a60e970 and b8f600c
Reviewer findings resolved:      round 1 26/26; decision recording 79/79 dispositioned (74 applied,
                                 2 kept as written, 3 needed owner input)
Human approvals:                 2026-10-01 direction (Rust target + Python reference); 2026-10-08
                                 DR-D31 option 1 / ledger entry L0; 2026-10-08/09 the decision
                                 interview (62 answers); no diff sign-off yet
Known limitations:               see below
Evidence manifest hash:          sha256:0e2b5ed1ce62388d0746b2ec6bc2a29a0bdc5216422306476123b1c905463b7e
Oracle parity result:            no Rust component yet; the oracle's own goldens pass under the pin
Reference-oracle job result:     red on the first two heads (KI-NEW-Z78); green on 05aba0a
Owner decisions depended on:     DR-A1..A12 and the interview outcomes (see Decisions); DR-A10 ruling pending
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
- **Drafts and open decisions.** The contracts and model specs are drafts with unapproved equations.
  Most B, C and D decisions are settled in full or in part; what stays open is named in each register
  entry.
- **Agent-only review of the decision recording.** The edits that carry your answers were checked by
  agents against the interview record. Your merge comment confirms the record.

## Before merging

1. **Licence ruling (DR-A10).** Post your licence ruling for `reference/python/` as a comment on this PR
   (you chose option 1 in the interview). The cautious-nevermore repository has no LICENSE file.
2. **Merge comment.** Accept ADR-011 and ADR-012, name DR-A1, A2, A4–A9, A11 and A12 as ratified, and
   confirm `docs/06-sessions/2026-10-09-P0-01-decision-interview.md`.
3. **Merge with a merge commit, never squash.** That keeps PR #1's attested commit `8d43203` reachable
   (DR-A6).
4. **Close the earlier PRs.** Close PR #1 as included, and close PRs #2 and #3 unmerged; their review
   records are already in `docs/06-sessions/`.
5. **After merge (DR-D30).** Make `reference-oracle (Linux only)` a required status check on `main`.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01PG15VStSfELgz5wh3pBoMo
