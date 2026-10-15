# P0-02 — Record the P0-01 merge rulings

## Outcome

PR #4 merged on 2026-10-15 (`b819ae0`), but the repository still described your merge review, the
DR-A10 licence ruling and the three ADRs as pending. This package records them. It decides nothing
beyond your PR #4 comment 6185953773 (as you edited it) and your three answers in
`docs/06-sessions/2026-10-15-P0-02.md`. Work package: `docs/01-work-packages/p0-02-record-p0-01-merge-rulings.md`.

- **ADR-011, ADR-012 and ADR-013 are Accepted.** You accepted them in comment 6185953773, and answer 2
  approved me transcribing that into their status, which `docs/02-adr/README.md` otherwise reserves
  to you. ADR-001, 006, 007 and 009 drop the "proposed" qualifier from their `superseded-by` line;
  nothing else in them changes. The three accepted ADRs are immutable from here, so their bodies are
  final in this PR.
- **DR-A10 is ruled: option 1, `MIT OR Apache-2.0`.** The comment first said "Apache-2.0 terms" by
  mistake (answer 1); you edited it at 12:57Z to read "`MIT or Apache-2.0` terms". ADR-012 §8,
  `reference/python/README.md`, the register and `engine-spec.md` Appendix F.1 cite it.
  `reference/python/` code is now covered by `LICENSE-MIT` and `LICENSE-APACHE`; real third-party data
  stays excluded (DR-A11).
- **The merge review.** DR-A1, A2, A4–A9, A11 and A12 are named again, so DR-A2's order of authority is
  in force. Both decision records are confirmed. DR-A3 is restated unchanged.
- **ADR-013 binds** wherever DR-D19 waited on it. Every open part of DR-D19 and the other decisions
  stays open, with its "(proposed — DR-xx)" or "(open — DR-xx)" tag.
- **P0-01 and P1-00 are Done.** The dashboard, daily log, work-package index and the archive's own
  index files say so.

**Why a new package (answer 3).** `scripts/check-evidence-claims.sh` fails any change set that edits a
merged package's evidence, so P0-01's record stays as it attested `6cc6bbb`. P0-02 has its own record
in `.ai/evidence/P0-02/`.

## Impact

- **Architecture, contracts, schemas, models, engine API:** none in substance. Only status wording
  changes in the contracts (DR-A2, DR-A11, DR-D30), the nflverse provider document (DR-A11) and the
  model specs (DR-A2; ADR-013 accepted). One doc-comment sentence in `crates/application/src/lib.rs`
  says DR-A8 is ratified.
- **Oracle:** nothing under `reference/python/backend/`, `tests/` or `MANIFEST.tsv` changes; no golden
  or fixture changes; no parity claim. The oracle status stays `legacy-59bce1d + L0`. `PARITY.md`
  changes only its status wording (DR-A10 ruled, ADR-013 accepted).
- **Tests:** none added or changed.
- **Known issues:** KI-NEW-Z24's text now says ADR-013 is accepted; none is addressed or introduced.

## Verification (all run on the final content commit)

- `just verify` exits 0. It runs **8 recipes**, with nextest 3/3.
- `tests/guards/run.sh` carries 78 committed behaviour cases.
- `pwsh scripts/verify.ps1 -Scope Full` exits 0 under PowerShell 7.4.6 on Linux. That is not the
  Windows merge-authoritative run: the `windows-authoritative` CI job on this PR is.
- `justfile`, `verify.sh` and `verify.ps1` agree at **15 steps**. `verify.ps1` guards every native
  command with `Assert-Ok` (15/15).
- `Cargo.lock` resolves 171 crates; nothing in it changes. A fresh clone compiles offline.
- Traceability: all 44 non-trivial paths traced against `origin/main` (`b819ae0`).
- `engine-spec.md` and its mirror are byte-identical, and all 65 register summary rows match
  Appendix F.1.
- Reference oracle: `verify_manifest.py` OK (102 verbatim, 3 patched), also with `--upstream`; 446
  passed under the pin, 0 isolation-guard violations. The `reference-oracle` CI job is the Linux
  evidence on this PR.
- No drafting placeholder remains, and P0-01's and P1-00's evidence records and the oracle's imported
  files are byte-for-byte unchanged since `b819ae0`.
- Review: a drafting workflow (a drafter and an adversarial verifier per file group, then a
  completeness sweep), then three fresh-context review rounds over the applied diff, each finding
  re-checked by an adversarial verifier: 49 findings (48 applied, 1 refuted), 20 (19 applied; 1 was
  stale before the merge and is out of scope) and 8 (all applied, after your edit of the comment).

## Completion record (Appendix C)

```text
Work package:                    P0-02 (docs/01-work-packages/p0-02-record-p0-01-merge-rulings.md)
Final commit:                    47501bd21061  (the commit the manifest attests to; the evidence
                                 commit carrying it follows and is not self-covered)
Model/harness identifier:        unrecorded (see Known limitations); harness Claude Code on the web
Environment:                     Linux 6.18.44-fc-v114; rustc 1.98.1; CPython 3.11.15 + OPENBLAS_CORETYPE=Haswell for the oracle
Authority documents read:        engine-spec.md §1.5, §1.7, §8.16, §8.17, §9.5, Appendix F; ADR-011, ADR-012,
                                 ADR-013 and docs/02-adr/README.md; the decision register; PR #4's thread
Contracts changed:               status wording only (docs/03-contracts/README.md,
                                 parity-fixture-contract.md)
Migrations changed:              none
Dependencies changed:            none
Targeted tests:                  tests/guards/run.sh 78/78; reference oracle 446/446 under the pin
Canonical verification command:  just verify (Linux smoke); verify.ps1 -Scope Full is merge-authoritative
Verification exit status:        0 (just verify); 0 (verify.ps1 under pwsh on Linux)
Golden files changed and approval: none
Performance evidence:            n/a (no engine computation)
Security/licensing review:       check-secrets OK; DR-A10 ruled (MIT OR Apache-2.0, PR #4 comment
                                 6185953773 as edited by the owner)
Fresh-context reviewer:          independent read-only agents, three rounds over the uncommitted diff
                                 on b819ae0
Reviewer findings resolved:      round 1 49 (48 applied, 1 refuted); round 2 20 (19 applied, 1 out of
                                 scope); round 3 8/8 applied
Human approvals:                 2026-10-15 PR #4 comment 6185953773 (merge review and DR-A10
                                 ruling), edited 12:57Z; answers 1-3 of the 2026-10-15 record; no
                                 diff sign-off yet
Known limitations:               see below
Evidence manifest hash:          sha256:99ab4f91d64a2c072a9937777ec1cfbdac4c4dd6d8f4a2ef79779c832e680e6b
Oracle parity result:            no Rust component; the oracle's own goldens pass under the pin
Reference-oracle job result:     the CI job on this PR
Owner decisions depended on:     DR-A1..A12 (named again or ruled 2026-10-15); ADR-011, ADR-012,
                                 ADR-013 (accepted 2026-10-15); DR-A5 (this package's ID, answer 3)
```

## Non-goals

No decision beyond your comment and answers. No change to P0-01's or P1-00's evidence, to the oracle's
imported files, or to any earlier session record. No engine code, recipe, CI or guard change.

## Migration and rollback

No migrations. To roll back, revert this PR's merge commit. Your rulings stand in PR #4 comment
6185953773 either way.

## Known limitations

- **`ai-toolchain.lock` provenance is "unrecorded"**, as in P0-01: the harness policy forbids writing
  model identifiers into repository artifacts. This deviates from engine-spec §8.20.
- **The branch is P0-01's** (`claude/grid-engine-consolidation-e7kmh9`, restarted from `main`), not
  `wp/P0-02-…`; the harness assigned it.
- **No agent budget was declared** (engine-spec §8.18.1).
- **Windows is proven only by this PR's CI.**
- **Agent-only review.** Your merge confirms the transcription.
- **Your message after the edit** read "License comment is not corrected at …#issuecomment-6185953773".
  I re-read the comment, found it edited to "`MIT or Apache-2.0` terms", and recorded that edit as the
  correction. Please say so in your review if that is not what you meant.

## Before merging

1. Review the diff: above all the three ADRs (immutable once this merges), the DR-A10 register entry
   and ADR-012 §8.
2. Merge with a merge commit.
3. Still yours after that, unchanged by this PR: make `reference-oracle (Linux only)` a required status
   check on `main` (DR-D30); the DR-A3 follow-up ADR before P1-11; the KI-NEW-Z79 decision request;
   the P1-12 pre-registrations.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01PG15VStSfELgz5wh3pBoMo
