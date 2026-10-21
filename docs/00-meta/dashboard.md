# Work-package dashboard

Requires the Obsidian Dataview plugin. Fields come from the frontmatter in
`docs/99-templates/template-work-package.md`; keys are hyphenated to match this query.

```dataview
TABLE status, risk-class, owner, implementer, reviewer
FROM "01-work-packages"
WHERE file.name != "README"
SORT status ASC, file.name ASC
```

## Status (2026-10-17)

| Item | State |
|---|---|
| Scope | Engine only: Rust target, Python reference oracle in `reference/python/` (ADR-011, ADR-012, both Accepted 2026-10-15) |
| Latest package | P0-03, the recording change for the owner's rulings on the decisions P1-01 raised: **Review** on branch `claude/pensive-fermat-fgaweh` from `8755552` (`docs/01-work-packages/p0-03-record-p1-01-merge-rulings.md`; session log `docs/06-sessions/2026-10-19-P0-03.md`). P1-01, domain types, engine API skeleton, oracle-fixture harness and crate restructure: **Done**, merged on 2026-10-19 at `8755552` (PR #6); ADR-014 accepted the same day. P0-02, the recording change for the P0-01 merge review, the DR-A10 ruling and the ADR acceptances: **Done**, merged on 2026-10-15 at `73a8e3b` (PR #5). P0-01: Done, merged 2026-10-15 at `b819ae0`. `ai-toolchain.lock` provenance stays unrecorded for every package |
| Rust engine | Skeleton: 13 crates, the DR-A8 set. P1-01 fills `domain` (contract types with serde derives, `AsOf`, typed errors, status enums and exit codes), `pipeline` (the §8.2–§8.4 façade as traits with no implementor; JSON envelope and sidecar), `synth` (DR-D29 fixture reader, tolerance helpers, `SyntheticTruth`, `from_legacy_v0`) and `grid-cli` (`--version`, `--help`, `exit-codes`); `persistence` holds the SQLx scaffold and one migration. No engine computation yet |
| Reference oracle | `legacy-59bce1d`, imported (105 files); its own suite passed on Linux at import (CPython 3.11.15). Its golden is pinned to CPython 3.11 with `OPENBLAS_CORETYPE=Haswell` (KI-NEW-Z78; DR-D31, ratified 2026-10-08): correction-ledger entry L0 regenerated Layer C once under that platform, and 446 tests pass. No other correction applied |
| Evidence level | "Experimental projections" (`engine-spec.md` §3.2, §3.3) |
| Owner actions around the P0-01 merge | Done on 2026-10-15 in PR #4 comment 6185953773, which holds the DR-A10 licence ruling and the merge review: DR-A1, DR-A2, DR-A4 to DR-A9, DR-A11 and DR-A12 named again (ratified 2026-10-08 in the decision interview); the interview record and the 2026-10-14 decision record (`docs/06-sessions/2026-10-14-P0-01.md`) confirmed as transcribed into the register; ADR-011, ADR-012 and ADR-013 accepted; the DR-A10 licence ruling (option 1) posted, its "Apache-2.0 terms" wording corrected by the owner's edit to "`MIT or Apache-2.0` terms". PRs #2 and #3 closed unmerged; GitHub records PR #1 as merged. Now that P0-01 has merged, the owner applies the branch-protection setting that makes `reference-oracle` a required status check (DR-D30) |
| Owner decisions | The decision interview (2026-10-08/09) settled most B, C and D decisions in full or in part. On 2026-10-14 the owner settled further parts of DR-D14 and DR-D17, decided the new DR-D32, and chose a reversal in part of DR-D19 (a season-level target for the feeder equivalency), which binds now that the owner has accepted ADR-013 (2026-10-15). On 2026-10-15 the owner also ruled on DR-A10 (option 1, `MIT OR Apache-2.0`; PR #4 comment 6185953773 as edited by the owner on 2026-10-15). On 2026-10-19 the owner ruled on the decisions P1-01 raised (`docs/06-sessions/2026-10-19-P0-03.md`): ADR-014 accepted, reviewer agents without Bash, a synchronous library façade, the ∞-norm for Class A′, each Phase 2 CLI surface delivered by its content package, and the three contracts Approved. Still to come: DR-A3's follow-up ADR before P1-11, the KI-NEW-Z79 decision request and the P1-12 pre-registrations (DR-D14, DR-D17, DR-D19 with DR-C5 and DR-C9). The rest stay proposed or open (`decision-register.md`). An unsettled decision, or an unsettled part of one, gates readiness of the packages it blocks |

## Sequence (engine-spec.md §9.5)

| WP | Scope | Status |
|----|-------|--------|
| P1-00 | Agent-ready repository, authority index, CI, verify scripts | Done: PR #1 approved at review round 4; landed with P0-01 on 2026-10-15 (DR-A6), and GitHub records PR #1 as merged |
| P0-01 | Engine-only consolidation: ADR-011, ADR-012, ADR-013, `engine-spec.md`, registers, contracts, model specs, oracle import, app removal, guard fixes | Done (merged 2026-10-15, `b819ae0`) |
| P0-02 | Record the P0-01 merge review, the DR-A10 ruling and the ADR acceptances | Done (merged 2026-10-15, `73a8e3b`, PR #5) |
| P1-01 | Domain IDs, as-of types, errors, library API and output-contract skeleton, oracle-fixture format, crate restructure | Done (merged 2026-10-19, `8755552`, PR #6; ADR-014 accepted 2026-10-19) |
| P0-03 | Record the owner's rulings on the decisions P1-01 raised: ADR-014, the four decision requests, the Amendments rows, the three contracts Approved | Review (branch `claude/pensive-fermat-fgaweh`) |
| P1-02 | SQLite schema, migrations, durable jobs, version and artifact-manifest primitives | Planned; after P1-01 |
| P1-03 | nflverse provider contracts and ingestion | Planned; after P1-02 |
| P1-04 | Player registry, NFL identity, NCAA adapter and linking | Planned; after P1-03 |
| P1-05 | Point-in-time feature store and leakage harness | Planned; after P1-04 |
| P1-06 | Numerical primitives: scoring, ridge, Kalman, RTS, EB, affine, booster trait | Planned; parallel with P1-02 to P1-05 after P1-01 |
| P1-07 | Layers A–D, NCAA/rookie priors, naive baselines | Planned; after P1-05 and P1-06 |
| P1-12 | GRID component port: V(s), RAPM, Layer 3, credit, state space, priors, corrected synthetic world | Planned; parallel with P1-07 after P1-05 and P1-06 |
| P1-08 | Layer E context, Layer F correlated simulation, distributions, ensemble, explanations | Planned; after P1-07 and P1-12 |
| P1-09 | Rolling-origin backtest, player pool, PB-MAE, calibration, scorecards | Planned; after P1-08 |
| P1-10 | Engine CLI, reports and exports (re-scoped; was the Flutter UI) | Planned; may start once the P1-01 output contract is frozen |
| P1-11 | Recovery, release artifact, clean-checkout reproduction, acceptance evidence (re-scoped; was the installer) | Planned; after P1-10 |
| P2-00 to P2-09 | Live weekly operation, benchmarking, governance, release hardening; P2-09 optional oracle retirement review | Planned; Phase 2 (`docs/01-work-packages/README.md`) |
