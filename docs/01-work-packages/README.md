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
  - The DR-A decisions adopted by P0-01 were ratified on 2026-10-08 in the owner's decision interview,
    and the owner's merge review named them again on 2026-10-15 (PR #4 comment 6185953773). That
    covers DR-A1 to DR-A12, except DR-A3 (its direction is ratified; the follow-up ADR is still owed)
    and DR-A10 (ruled on 2026-10-15: PR #4 comment 6185953773, as edited by the owner on
    2026-10-15).
  - `docs/00-meta/decision-register.md` is authoritative for each decision's status.
- **Status.** The status column here is informational. The dashboard (`docs/00-meta/dashboard.md`) reads
  each file's front matter, and this README is excluded from its query.

## Index

The "Must be ratified" column reproduces engine-spec §9.5 and §10.5 for the packages listed there.
The DR-D items in "Also open" are taken from the register's "Blocks" fields as of 2026-10-15, after
the owner's decision interview of 2026-10-08/09, the owner's decisions of 2026-10-14 and the owner's
acceptance of ADR-013 on 2026-10-15; a partly settled DR names the part still open. Under the
register's readiness rule ("How to ratify or override", step 5), any unsettled DR, or unsettled part
of one, that a package depends on gates Ready, wherever it is listed. "Spec refs" are the
`engine-spec.md` sections that define each package's scope.

### Foundation

| WP | Scope (one line) | Status | Spec refs | Must be ratified before Ready | Also open, or other preconditions | File |
|---|---|---|---|---|---|---|
| P1-00 | Agent-ready repository, authority index, CI, verify scripts | Done. Implemented in PR #1, approved at review round 4 with 0 blockers; it landed with P0-01 (DR-A6), which merged on 2026-10-15 (`b819ae0`) | §9.2.1, §9.5 | — | — | [p1-00-work-package.md](p1-00-work-package.md) |
| P0-01 | Engine-only consolidation: ADR-011, ADR-012, ADR-013 (Accepted 2026-10-15), `engine-spec.md`, registers, contracts, model specs, the `reference/python/` oracle, app removal, guard fixes | Done (merged 2026-10-15, `b819ae0`) | §1.1, §1.5, §1.7, §8.1, §8.19, §9.5 | DR-A1, DR-A2, DR-A4 to DR-A9, DR-A11, DR-A12, all adopted; ratified on 2026-10-08 in the decision interview and named again in the owner's merge review on 2026-10-15 (PR #4 comment 6185953773) | DR-A10: option 1 chosen on 2026-10-08 and ruled on 2026-10-15 (`MIT OR Apache-2.0`; PR #4 comment 6185953773, as edited by the owner on 2026-10-15). DR-A3: default kept, direction ratified on 2026-10-08; follow-up ADR before P1-11. DR-D30: P0-01 has merged, and the owner now makes `reference-oracle` a required check in branch protection. DR-D31 (raised here; ratified by the owner on 2026-10-08 and implemented here as correction-ledger entry L0, KI-NEW-Z78) | [p0-01-engine-consolidation.md](p0-01-engine-consolidation.md) |
| P0-02 | Record the P0-01 merge review, the DR-A10 ruling and the acceptance of ADR-011, ADR-012 and ADR-013 | Review | §1.5, §1.7, §8.16, Appendix F | — (the rulings are given: PR #4 comment 6185953773, as edited by the owner) | — | [p0-02-record-p0-01-merge-rulings.md](p0-02-record-p0-01-merge-rulings.md) |

### Phase 1: engine core and historical proof (planned; no file yet)

| WP | Scope (one line) | Spec refs | Must be ratified before Ready | Also open, or other preconditions |
|---|---|---|---|---|
| P1-01 | Domain IDs, the `AsOf` type with a publication-lag axis, typed errors; library API and output-contract skeleton; oracle-fixture format, synthetic-fixture loader and parity-harness skeleton; `application` → `pipeline`, plus `grid-cli` and `synth` skeletons; agent definitions | §4.5, §5.5, §7.12, §8.1, §8.2–§8.4, §8.18, §9.2.1 | DR-A2, DR-A8, DR-A11 (ratified 2026-10-08); DR-B1, DR-B2, DR-B3, DR-C4 | — (settled 2026-10-08: DR-D7, DR-D29, and the DR-D8 interim rule as the skeleton's basis) |
| P1-02 | Schema groups including GRID state; migrations; durable jobs; raw-data, version and artifact-manifest primitives; atomic writes; production pointers | §8.5, §8.7, §8.10, §8.11, §9.2.2 | DR-B6, DR-C15 | — |
| P1-03 | nflverse provider contracts and raw/normalized ingestion; plays-contract builder; official labels; fetch cap and offline replay; per-dataset publication rules | §4.1, §4.5, §4.7, §8.6.1, §9.2.2 | DR-A11, DR-C12, DR-C13, DR-C14 | DR-D15's involvement roles were decided on 2026-10-09. `docs/04-providers/nflverse/access-and-license.md`: a draft exists, and the per-dataset terms await Data/Licensing verification. A declared publication-lag rule for every dataset |
| P1-04 | Canonical registry, NFL identity, CFBD adapter, NCAA linking tiers, identity review queue | §4.2, §4.4, §9.2.2 | DR-C9 | A `docs/04-providers/cfbd/` licence and terms record before the adapter is Ready |
| P1-05 | Point-in-time feature store; leakage harness with the publication-lag axis and four guards; committed corrected synthetic fixtures | §4.5, §11, §12.3, §9.2.3 | DR-A11, DR-B4, DR-C1, DR-C6, DR-C12 | — (DR-D4 ratified the REG-only reading on 2026-10-09) |
| P1-06 | Numerical primitives: affine scoring, generalized ridge (dense Cholesky and preconditioned CG), Kalman (Joseph form), full RTS and fixed-lag, EB, booster trait | §6.4, §8.7.3, §9.2.4 | DR-B1, DR-B3, DR-B6, DR-C5, DR-C7, DR-C8, DR-C10 | Model specs approved. DR-D6 was ratified on 2026-10-09 (affine profiles only) |
| P1-07 | Layers A–D; NCAA/rookie prior assembly; naive baselines | §6.1, §6.5, §9.2.4 | DR-B4, DR-C2, DR-C3, DR-C5 (EB estimator), DR-C9 | DR-D2 (Layer B market inputs on real data); DR-D9; DR-D10 (Layer B); DR-D13; DR-D21 (the yardage family and the receiving rate base); DR-D25 (the calibrated values). ADR-013 (accepted 2026-10-15) reverses DR-D19 in part: the feeder translation that P1-07 assembles into the §6.5 prior is fitted to the season level instead of end-of-season smoothed talent, and the "residual variance of the equivalency" in DR-C9's proposed engine form of `σ²_prior` is `σ̂²_u`, the between-player variance net of the known target noise, with the §6.5 widening applied to it. Whether `σ²_prior` also carries the line's parameter uncertainty is not yet decided, nor is the variance of the `q` blend (open — DR-C9). DR-B5 is settled for this package (2026-10-09), and so are DR-D19's Kalman credit scale and its rule of no separate scale map (2026-10-09), which ADR-013 keeps. `projection-stack.md` approved |
| P1-08 | Layer E context; Layer F correlated simulation; conditional and unconditional outputs; distributions; ensemble stacking; explanations | §5.2, §5.3, §6.1, §6.6, §6.7 | DR-B4, DR-B5, DR-C2, DR-C4 | DR-D8 (the distribution values); DR-D16 (the exact definition, once Layer C exists); DR-D25 (the calibrated values). DR-C3, DR-D20 and DR-D7 are settled (2026-10-08/09) |
| P1-09 | Rolling-origin backtest; player pool; PB-MAE and secondary metrics; calibration; scorecards; week-clustered bootstrap; lineup simulation; threshold registry; H1/H2 diagnostics | §7.1–§7.7, §7.14, §9.2.6, §9.4 | DR-B1, DR-C1, DR-C4, DR-C5, DR-C6, DR-C11, DR-C12 | DR-D2; DR-D9 (the low-evidence slice). DR-D5 and DR-D24 were decided on 2026-10-09 |
| P1-10 | Engine CLI, reports and exports; completion of the output contract; identity review queue, read-only. Re-scoped: this was the Flutter UI | §5.6, §8.2–§8.3, §9.2.7 | DR-C14 | DR-D3 (before any public distribution of participation-derived outputs). DR-D6 (custom profiles) was ratified on 2026-10-09: affine profiles only |
| P1-11 | Crash recovery; the release artifact (the `grid` CLI binary and library crates); clean-checkout reproduction; end-to-end acceptance evidence; model card; historical validation report; runbooks. Re-scoped: this was the installer and clean machine | §8.11, §9.4, §17.1 | DR-A3 (the follow-up ADR should land first); DR-C5 (the auto-rollback sanity thresholds used by recovery) | DR-D3 (public distribution). `toolchains/native-dependencies.lock` only if a native booster is adopted (DR-C8). Deferrals carried from ADR-004, ADR-007 and ADR-008 (ADR-011 D9) |
| P1-12 | GRID component port. New. Recommended split, in order: (1) RAPM with injected `dV`; (2) the state space; (3) cross-league priors; (4) V(s) and the Layer-1 context model; (5) the single-pass fit (no fixed-point loop; DR-D11); (6) the Rust-native corrected generator and the Class D gates | §6.2, §6.9, §7.12, §7.13, §9.2.5 | DR-A5, DR-B1, DR-B3, DR-B4, DR-B5, DR-B6, DR-C1, DR-C2, DR-C5 (RAPM domain controls), DR-C6, DR-C7, DR-C9, DR-C10, DR-C13; DR-D1 (real-data scheme-reset path only) | DR-D2, DR-D10 and DR-D12 (real data; DR-D12's values and remedy come from its calibration study); DR-D13; DR-D15 (penalty plays; the snap-count source); DR-D14 (gates within P1-12, not its start: the R-7 check's rates, offsets, statistics, margins, pass rule and gate kind, and the training-row comparison's statistics and margin, pre-registered before P1-12 tunes or gates anything; real-data promotion of a Layer-1 run that used a substitute, until the R-7 check passes); DR-D17 (likewise: the tuning criterion of κ_c and λ_c, the base scale and the recency weight, before P1-12 tunes anything; the window-hyperparameter estimator, DR-C5, and the minimum class size and the degenerate-pool thresholds, before the builder is implemented; real-data promotion of the windowed prior); DR-D18 (`z_thresh` and the precision/recall gate); DR-D19 (`P0_form`, `P0_scheme` and `P0` for window-restarted tracks; before P1-12 implements the feeder equivalency, the equivalency's estimator, including its weighting (DR-C5), its class and the minimum shared players per class and build (DR-D19, DR-C9; KI-G8); under ADR-013, accepted on 2026-10-15, each DR-D32 build fits the equivalency to its own season-level targets with their known variance); DR-D26 (the seed set, quantile and margins); DR-D27 (parts 3 and 4); the training features of `g′` (KI-NEW-Z79: a decision request is still to be raised). Settled for this package on 2026-10-08/09: DR-D4, DR-D11, DR-D16, DR-D28 and DR-D31 (parity fixtures exported from the oracle; KI-NEW-Z78); on 2026-10-14: DR-D32 (two priors, strict window; real-data promotion of the season-boundary priors waits on P1-12's own measurements; for feeder-linked tracks both canaries can hold under ADR-013, accepted on 2026-10-15). DR-D9 no longer gates this package: since DR-D17 (2026-10-14) it governs no Kalman prior, and no P1-12 component uses it |

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
| P2-03 | Fixed-lag smoothing in the live path; bounded boosting continuation and replay; GRID live tier and governed offseason refresh | §6.3, §8.7.2, §8.7.3, §10.2.3 | DR-C5, DR-C10, DR-D1 | DR-D15 (Layer-1′ in live operation: penalty plays, the snap-count source); DR-D18 (`z_thresh` and the precision/recall gate). DR-D17 was decided on 2026-10-09 |
| P2-04 | Scenario-aware opportunity transfer; calibrated tails; benchmarked production draw count | §10.2.3, §13 | None beyond the Phase 1 dependencies | The stat-vector synthetic world before Layer C/F recovery gates are claimed (DR-B4) |
| P2-05 | Benchmark provider registry, importers, legal-use metadata, lock-time hashing, immutable snapshots | §7.6, §10.2.4 | — | Per-provider acquisition and licence approval |
| P2-06 | Pairwise provider evaluation, week-clustered confidence intervals, scorecards, benchmark report, claim-evidence package | §7.7–§7.9, §10.2.4 | — | DR-D3 (public reports). DR-D23 was ratified on 2026-10-09 |
| P2-07 | Candidate governance, promotion serialization, automatic reject and rollback, audit report, projection change log | §8.8, §10.2.5 | DR-C5 (including the auto-rollback thresholds) | DR-D8 (the change-attribution method, before the change log is built) |
| P2-08 | Release packaging of the CLI and library; recovery, performance, security and independent-audit hardening | §10.4.3, §13, §14 | DR-A3 (the release platform) | DR-C8 (only if a native booster is adopted) |
| P2-09 | Optional oracle retirement review: freeze, archive, or keep `reference/python/` as a CI oracle. New | §1.7, §10.5 | DR-A12 | Every ported component has parity evidence **and** Phase 2 live evidence exists. Retirement never deletes the legacy tag, the committed fixtures or the ledger |

The traceability guard recognizes `P2-NN` since P0-01 (DR-A5). That is a prerequisite of every P2
package.

## Decisions with no work package

Two raised decisions block no package in the register. The owner decided both in the decision interview
(2026-10-09):

- **DR-D22** (provider `ep`/`epa` as features): EPA-type features come from GRID `dV` only, permanently;
  nflverse `ep`/`epa` never enter features unless an ADR reopens the question
  (`docs/05-model-specs/value-model.md`).
- **DR-D30** (`reference-oracle` as a required merge check) is a branch-protection setting
  (engine-spec §8.19). The job becomes a required status check for every pull request to `main`; the
  owner applies the setting now P0-01 has merged, with no workflow edit. DR-D31 (the oracle's numerical
  platform, KI-NEW-Z78) was ratified on 2026-10-08 and implemented in P0-01 as correction-ledger entry
  L0; it no longer blocks P1-12.
