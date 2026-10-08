# Work packages

Each work package is one committed file here, written from `docs/99-templates/template-work-package.md`
(engine-spec §8.16, Appendix B). A package is authority level 4 (engine-spec §1.5).

## Conventions

- **IDs** have the form `P<phase>-<NN>` (DR-A5, ADR-011 D9). Suffixed IDs such as `P1-12a` are not used.
  A split takes the next free number and names its parent workstream (engine-spec §9.5).
- **File names** are `p<phase>-<nn>-<slug>.md`, and the lowercase ID must appear in the name.
  `scripts/check-traceability.sh` resolves a cited ID by the glob `docs/01-work-packages/*<id>*`, so a
  file named any other way is never found.
- **Readiness.** A package is not Ready while any decision in its "Must be ratified" column is
  unratified. A proposed default is not a ratification (engine-spec §8.16.1).
  - The DR-A decisions adopted by P0-01 count as ratified once the owner's merge comment names them.
    That covers DR-A1 to DR-A12, except DR-A3 and DR-A10.
  - `docs/00-meta/decision-register.md` is authoritative for each decision's status.
- **Status.** The status column here is informational. The dashboard (`docs/00-meta/dashboard.md`) reads
  each file's front matter, and this README is excluded from its query.

## Index

The "Must be ratified" column reproduces engine-spec §9.5 and §10.5. The DR-D items in "Also open" are
taken from the register's "Blocks" fields as of 2026-10-08. Under the register's readiness rule ("How to
ratify or override", step 5), any DR a package depends on gates Ready, wherever it is listed. "Spec refs"
are the `engine-spec.md` sections that define each package's scope.

### Foundation

| WP | Scope (one line) | Status | Spec refs | Must be ratified before Ready | Also open, or other preconditions | File |
|---|---|---|---|---|---|---|
| P1-00 | Agent-ready repository, authority index, CI, verify scripts | Implemented in PR #1, approved at review round 4 with 0 blockers. It lands with P0-01 (DR-A6) and is Done when P0-01 merges | §9.2.1, §9.5 | — | — | [p1-00-work-package.md](p1-00-work-package.md) |
| P0-01 | Engine-only consolidation: ADR-011, ADR-012, `engine-spec.md`, registers, contracts, model specs, the `reference/python/` oracle, app removal, guard fixes | Review | §1.1, §1.5, §1.7, §8.1, §8.19, §9.5 | DR-A1, DR-A2, DR-A4 to DR-A9, DR-A11, DR-A12, all adopted and ratified by the owner's merge comment | DR-A10: the licence ruling is recorded on the PR before merge. DR-A3: default kept; follow-up ADR before P1-11. DR-D31 (raised here; ratified by the owner on 2026-10-08 and implemented here as correction-ledger entry L0, KI-NEW-Z78) | [p0-01-engine-consolidation.md](p0-01-engine-consolidation.md) |

### Phase 1: engine core and historical proof (planned; no file yet)

| WP | Scope (one line) | Spec refs | Must be ratified before Ready | Also open, or other preconditions |
|---|---|---|---|---|
| P1-01 | Domain IDs, the `AsOf` type with a publication-lag axis, typed errors; library API and output-contract skeleton; oracle-fixture format, synthetic-fixture loader and parity-harness skeleton; `application` → `pipeline`, plus `grid-cli` and `synth` skeletons; agent definitions | §4.5, §5.5, §7.12, §8.1, §8.2–§8.4, §8.18, §9.2.1 | DR-A2, DR-A8, DR-A11 (at the P0-01 merge); DR-B1, DR-B2, DR-B3, DR-C4 | DR-D7; DR-D8; DR-D29 |
| P1-02 | Schema groups including GRID state; migrations; durable jobs; raw-data, version and artifact-manifest primitives; atomic writes; production pointers | §8.5, §8.7, §8.10, §8.11, §9.2.2 | DR-B6, DR-C15 | — |
| P1-03 | nflverse provider contracts and raw/normalized ingestion; plays-contract builder; official labels; fetch cap and offline replay; per-dataset publication rules | §4.1, §4.5, §4.7, §8.6.1, §9.2.2 | DR-A11, DR-C12, DR-C13, DR-C14 | DR-D15 (the involvement-roles field). `docs/04-providers/nflverse/access-and-license.md`: a draft exists, and the per-dataset terms await Data/Licensing verification. A declared publication-lag rule for every dataset |
| P1-04 | Canonical registry, NFL identity, CFBD adapter, NCAA linking tiers, identity review queue | §4.2, §4.4, §9.2.2 | DR-C9 | A `docs/04-providers/cfbd/` licence and terms record before the adapter is Ready |
| P1-05 | Point-in-time feature store; leakage harness with the publication-lag axis and four guards; committed corrected synthetic fixtures | §4.5, §11, §12.3, §9.2.3 | DR-A11, DR-B4, DR-C1, DR-C6, DR-C12 | DR-D4 (only to relax the REG-only reading) |
| P1-06 | Numerical primitives: affine scoring, generalized ridge (dense Cholesky and preconditioned CG), Kalman (Joseph form), full RTS and fixed-lag, EB, booster trait | §6.4, §8.7.3, §9.2.4 | DR-B1, DR-B3, DR-B6, DR-C5, DR-C7, DR-C8, DR-C10 | DR-D6 (only if a profile needs a non-affine rule). Model specs approved |
| P1-07 | Layers A–D; NCAA/rookie prior assembly; naive baselines | §6.1, §6.5, §9.2.4 | DR-B4, DR-C2, DR-C3, DR-C5 (EB estimator), DR-C9 | DR-B5 and DR-D2 (Layer B market inputs on real data); DR-D9; DR-D10 (Layer B); DR-D13; DR-D19; DR-D21; DR-D25. `projection-stack.md` approved |
| P1-08 | Layer E context; Layer F correlated simulation; conditional and unconditional outputs; distributions; ensemble stacking; explanations | §5.2, §5.3, §6.1, §6.6, §6.7 | DR-B4, DR-B5, DR-C2, DR-C4 | DR-C3 and DR-D20 (§6.6 readiness); DR-D7; DR-D8; DR-D16; DR-D25 |
| P1-09 | Rolling-origin backtest; player pool; PB-MAE and secondary metrics; calibration; scorecards; week-clustered bootstrap; lineup simulation; threshold registry; H1/H2 diagnostics | §7.1–§7.7, §7.14, §9.2.6, §9.4 | DR-B1, DR-C1, DR-C4, DR-C5, DR-C6, DR-C11, DR-C12 | DR-D2; DR-D5; DR-D24 |
| P1-10 | Engine CLI, reports and exports; completion of the output contract; identity review queue, read-only. Re-scoped: this was the Flutter UI | §5.6, §8.2–§8.3, §9.2.7 | DR-C14 | DR-D3 (before any public distribution of participation-derived outputs); DR-D6 (custom profiles) |
| P1-11 | Crash recovery; the release artifact (the `grid` CLI binary and library crates); clean-checkout reproduction; end-to-end acceptance evidence; model card; historical validation report; runbooks. Re-scoped: this was the installer and clean machine | §8.11, §9.4, §17.1 | DR-A3 (the follow-up ADR should land first); DR-C5 (the auto-rollback sanity thresholds used by recovery) | DR-D3 (public distribution). `toolchains/native-dependencies.lock` only if a native booster is adopted (DR-C8). Deferrals carried from ADR-004, ADR-007 and ADR-008 (ADR-011 D9) |
| P1-12 | GRID component port. New. Recommended split, in order: (1) RAPM with injected `dV`; (2) the state space; (3) cross-league priors; (4) V(s) and the Layer-1 context model; (5) the fixed point; (6) the Rust-native corrected generator and the Class D gates | §6.2, §6.9, §7.12, §7.13, §9.2.5 | DR-A5, DR-B1, DR-B3, DR-B4, DR-B5, DR-B6, DR-C1, DR-C2, DR-C5 (RAPM domain controls), DR-C6, DR-C7, DR-C9, DR-C10, DR-C13; DR-D1 (real-data scheme-reset path only) | DR-D2, DR-D10 and DR-D12 (real data); DR-D4; DR-D9 (priors); DR-D11; DR-D13; DR-D14; DR-D15; DR-D16; DR-D17; DR-D18; DR-D19; DR-D26; DR-D27; DR-D28; DR-D31 (parity fixtures exported from the oracle; KI-NEW-Z78) |

**Dependency order** (engine-spec §9.5):

- P1-01 comes first. P1-02 to P1-05 then run in sequence, and P1-06 runs in parallel with them.
- P1-07 and P1-12 run in parallel once P1-05 and P1-06 have merged.
- P1-08 follows both, then P1-09, P1-10 and P1-11. CLI and report work (P1-10) may start once the
  P1-01 output contract is frozen, but it computes nothing the engine services lack.
- P1-12 owns `models::{value, rapm, credit, statespace, priors}` and the `synth` crate. P1-07 does not
  edit them.

### Phase 2: live weekly intelligence and competitive proof (planned; no file yet)

| WP | Scope (one line) | Spec refs | Must be ratified before Ready | Also open, or other preconditions |
|---|---|---|---|---|
| P2-00 | Live-operation threat model, least-privilege permission profile, operational runbooks, shadow environment | §10.2.1, §14.1 | DR-A5 | — |
| P2-01 | Once-daily `grid update`, catch-up, Thursday/Sunday immutable locks, source-specific freshness thresholds, Phase 2 data-quality report | §8.6, §10.2.2 | DR-C14 | — |
| P2-02 | Availability snapshots, availability report and override import, provider-neutral availability and weather adapter contracts | §4.1.2, §10.2.2 | DR-D1 | Data/Licensing terms review before any new live provider is enabled |
| P2-03 | Fixed-lag smoothing in the live path; bounded boosting continuation and replay; GRID live tier and governed offseason refresh | §6.3, §8.7.2, §8.7.3, §10.2.3 | DR-C5, DR-C10, DR-D1 | DR-D15 (Layer-1′ in live operation); DR-D17; DR-D18 |
| P2-04 | Scenario-aware opportunity transfer; calibrated tails; benchmarked production draw count | §10.2.3, §13 | None beyond the Phase 1 dependencies | The stat-vector synthetic world before Layer C/F recovery gates are claimed (proposed — DR-B4) |
| P2-05 | Benchmark provider registry, importers, legal-use metadata, lock-time hashing, immutable snapshots | §7.6, §10.2.4 | — | Per-provider acquisition and licence approval |
| P2-06 | Pairwise provider evaluation, week-clustered confidence intervals, scorecards, benchmark report, claim-evidence package | §7.7–§7.9, §10.2.4 | — | DR-D3 (public reports); DR-D23 (no §7.8 or §7.9 result is admissible until it is defined) |
| P2-07 | Candidate governance, promotion serialization, automatic reject and rollback, audit report, projection change log | §8.8, §10.2.5 | DR-C5 (including the auto-rollback thresholds) | DR-D8 (the change-attribution method, before the change log is built) |
| P2-08 | Release packaging of the CLI and library; recovery, performance, security and independent-audit hardening | §10.4.3, §13, §14 | DR-A3 (the release platform) | DR-C8 (only if a native booster is adopted) |
| P2-09 | Optional oracle retirement review: freeze, archive, or keep `reference/python/` as a CI oracle. New | §1.7, §10.5 | DR-A12 | Every ported component has parity evidence **and** Phase 2 live evidence exists. Retirement never deletes the legacy tag, the committed fixtures or the ledger |

The traceability guard recognizes `P2-NN` since P0-01 (DR-A5). That is a prerequisite of every P2
package.

## Open decisions with no work package

Two raised decisions block no package in the register:

- **DR-D22** (provider `ep`/`epa` as features) blocks any feature that reads provider `ep` or `epa`.
  Until it is decided, EPA-type features come from GRID `dV` only (`docs/05-model-specs/value-model.md`).
- **DR-D30** (`reference-oracle` as a required merge check) is a branch-protection setting
  (engine-spec §8.19). DR-D31 (the oracle's numerical platform, KI-NEW-Z78) was ratified on 2026-10-08
  and implemented in P0-01 as correction-ledger entry L0; it no longer blocks P1-12.
