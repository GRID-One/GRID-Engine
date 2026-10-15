# Work-package dashboard

Requires the Obsidian Dataview plugin. Fields come from the frontmatter in
`docs/99-templates/template-work-package.md`; keys are hyphenated to match this query.

```dataview
TABLE status, risk-class, owner, implementer, reviewer
FROM "01-work-packages"
WHERE file.name != "README"
SORT status ASC, file.name ASC
```

## Status (2026-10-14)

| Item | State |
|---|---|
| Scope | Engine only: Rust target, Python reference oracle in `reference/python/` (ADR-011, ADR-012, both Proposed) |
| Active package | P0-01, engine-only consolidation: **Review**. Done only when merged with the owner's ratifications |
| Rust engine | Pre-implementation: 11 crate stubs; `persistence` holds the SQLx scaffold and one migration |
| Reference oracle | `legacy-59bce1d`, imported (105 files); its own suite passed on Linux at import (CPython 3.11.15). Its golden is pinned to CPython 3.11 with `OPENBLAS_CORETYPE=Haswell` (KI-NEW-Z78; DR-D31, ratified 2026-10-08): correction-ledger entry L0 regenerated Layer C once under that platform, and 446 tests pass. No other correction applied |
| Evidence level | "Experimental projections" (`engine-spec.md` §3.2, §3.3) |
| Owner actions around the P0-01 merge | Before: in the merge comment, name again DR-A1, DR-A2, DR-A4 to DR-A9, DR-A11 and DR-A12 (ratified 2026-10-08 in the decision interview) and confirm the interview record and the 2026-10-14 decision record (`docs/06-sessions/2026-10-14-P0-01.md`); accept ADR-011 and ADR-012 (the owner's choice in the decision interview); accept or reject ADR-013 (Proposed; DR-D19, answer 10 of the 2026-10-14 record); post the DR-A10 licence ruling (option 1, chosen 2026-10-08) on the PR. After: make `reference-oracle` a required status check in branch protection (DR-D30) |
| Owner decisions | The decision interview (2026-10-08/09) settled most B, C and D decisions in full or in part. On 2026-10-14 the owner settled further parts of DR-D14 and DR-D17, decided the new DR-D32, and chose a reversal in part of DR-D19 (a season-level target for the feeder equivalency) that binds once the owner accepts ADR-013 (Proposed); until then DR-D19's 2026-10-09 text stands. The rest stay proposed or open (`decision-register.md`). An unsettled decision, or an unsettled part of one, gates readiness of the packages it blocks |

## Sequence (engine-spec.md §9.5)

| WP | Scope | Status |
|----|-------|--------|
| P1-00 | Agent-ready repository, authority index, CI, verify scripts | PR #1 approved at review round 4; lands with P0-01 (DR-A6) |
| P0-01 | Engine-only consolidation: ADR-011, ADR-012, ADR-013 (Proposed), `engine-spec.md`, registers, contracts, model specs, oracle import, app removal, guard fixes | Review |
| P1-01 | Domain IDs, as-of types, errors, library API and output-contract skeleton, oracle-fixture format, crate restructure | Planned; blocked on the P0-01 merge and its DRs |
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
