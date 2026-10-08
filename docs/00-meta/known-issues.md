# Known issues: GRID engine backlog

This register lists every known defect, latent hazard and open design gap in the Python GRID engine that
is imported as the reference oracle under `reference/python/` (cautious-nevermore, CN, at `59bce1d`). It
is the engine backlog that the Rust port works against.

- **Not an authority.** Like `docs/00-meta/decision-register.md`, this file tracks state. It does not
  rank in the order of authority (`engine-spec.md` §1.5). A fix binds through the document that implements
  it: a model spec, a contract, an ADR, or the oracle correction ledger in `reference/python/PARITY.md`.
- **Verified against code, not against CN's own logs.** Status comes from the code at `59bce1d`, checked
  in the consolidation inventory (`docs/06-sessions/2026-10-01-consolidation-inventory/cn-issues.md`). The
  critic's adjudications in `critic.md` §2 (X-1 to X-21) are applied on top. Where they change an issue's
  status, severity or meaning, the row says so. CN's `docs/06-issues-log.md` status column is stale and is
  not a source.
- **Line numbers.** `reference/python/` mirrors CN byte-for-byte, except for patches P1 (`lineup_sim.py`)
  and P2 (`run_demo.py`). Its line numbers therefore equal CN@`59bce1d`. Paths marked **CN-only** were not
  imported into the oracle.
- **Evidence scripts.** These live in `reference/python/tools/investigations/` under the filenames used
  below. Scripts that depend on the synthetic generator ran on the **legacy** generator (the defender bug,
  KI-NEW-Y0) unless the row says otherwise.
- **Full finding text.** For the reasoning and measurements in full, see `cn-issues.md` §3.x (each
  section heading below names its source section) and `critic.md` §1–§2.
- **KI-NEW-Z rows.** KI-NEW-Z1 to KI-NEW-Z77 were found during the 2026-10 consolidation, by the
  authors of `docs/05-model-specs/` and `docs/03-contracts/`. Z marks the consolidation as their origin.
  - Each row names the spec or contract that describes or measured it. That document has the full text,
    and carries the KI ID next to its own description of the defect.
  - Several measurements come from model-spec probe scripts that are not yet committed under
    `reference/python/tools/investigations/`. The specs label those numbers "recorded, not reproducible
    from the repo". The caveat applies to the rows that cite them until the scripts are committed.

Verified at: CN `59bce1d` (shallow clone; boundary `c1b6748`, 2026-06-22). The defender bug was
re-verified on 2026-10-01 against a scratch copy of the same commit.

The KI-NEW-Z rows were verified on 2026-10-07 against `reference/python/`, which is byte-identical to the
same commit apart from patches P1 and P2:

- every row by reading the code at its cited lines;
- rows marked "confirmed" also by re-running the cited reproduction against the imported oracle (Linux,
  CPython 3.11.15, the `requirements.lock` pins, threads = 1).

---

## Summary

There are 172 issues:

- 93 cn-issues IDs;
- KI-NEW-Y0 and KI-NEW-D1, from the critic;
- KI-NEW-Z1 to KI-NEW-Z77, from the 2026-10 consolidation.

A further alias row, KI-V1 → KI-G1, is not counted.

| Severity \ Status | OPEN | PARTIAL | FIXED | FALSE-POSITIVE | UNVERIFIABLE | Total |
|---|---|---|---|---|---|---|
| CRIT | 5 | 0 | 0 | 0 | 0 | 5 |
| HIGH | 14 | 0 | 0 | 0 | 0 | 14 |
| MOD | 63 | 0 | 0 | 0 | 0 | 63 |
| LOW | 70 | 3 | 0 | 1 | 0 | 74 |
| — (no severity) | 5 | 0 | 6 | 4 | 1 | 16 |
| **Total** | **157** | **3** | **6** | **5** | **1** | **172** |

- **The Z rows** are 1 HIGH, 34 MOD and 42 LOW, all OPEN. The HIGH row is KI-NEW-Z68: the oracle
  walk-forward uses same-season participation.
- **Re-grade.** KI-NEW-A5 moved from LOW to MOD on the 2026-10 real-data evidence.

- The five "—" OPEN entries are research items: KI-#42, #43, #44, #45 and #46. KI-#49 is also carried as
  research, but it has a MOD severity and is counted in that row.
- Every CRIT and HIGH entry is OPEN.
- The **Rust port must not reproduce** (MNR) flag is set on 53 entries: 26 from the earlier rows and 27
  from the Z rows.

**Highest priority**, in correction-ledger order (DR-B1):
1. KI-NEW-Y0: the synthetic defender bug.
2. KI-G1 / KI-NEW-A1 / KI-NEW-Y1: the team-strength convention (DR-B5).
3. KI-NEW-A2: the matchup-grade sign.
4. KI-#15: the Kalman cold-start look-ahead.
5. KI-NEW-I1–I4 and KI-NEW-V0a: real-data ingest bias.

Outside the ledger, the incremental path (KI-NEW-W1 to W5, KI-#24, KI-NEW-S1) means `weekly_update` is
**not** an oracle. The Rust incremental loop starts from the batch walk-forward semantics instead.

**App-only IDs deliberately excluded:** GitHub #16, #21, #22, #26, #28, #29, #30, #34–#40, #50, #51, #52,
#58, #59, #61, #62 (#39 is operations; its engine piece is KI-V10). Audit W1–W3, W5–W11, W13–W15, W17–W20,
the unlisted W4/W16, A2, A10–A14, A16–A18, F1–F8.

## Legend

**Severity** (re-graded against code, per cn-issues):
- **CRIT**: corrupts engine numbers on the main path.
- **HIGH**: corrupts numbers on a supported path, or blocks real-data use.
- **MOD**: wrong in a reachable edge or design case.
- **LOW**: latent, cosmetic, or hygiene.
- **—**: not applicable (fixed, false positive, research item, alias).

**Status:**
- **OPEN**: present at `59bce1d`. "(confirmed)" means reproduced by a script or measurement. "(latent)"
  means reachable but not hit by current callers. "(design)" means an open convention rather than a coding
  error.
- **PARTIAL**: partly fixed, or partly a false positive.
- **FIXED (commit, PR)**: fixed in CN before `59bce1d`.
- **FALSE-POSITIVE**: the filed defect does not exist. Any residual is noted.
- **UNVERIFIABLE**: the ID exists in a CN log but no text or source survives.

**Oracle impact** (does this contaminate something frozen as a parity target?). Test files are under
`reference/python/tests/grid/`:

| Code | Meaning |
|---|---|
| **GM-A / GM-B / GM-C** | `test_golden_master.py` Layer A (semantic invariants), Layer B (ordering), Layer C (numeric, rtol 1e-5), against `golden/snapshot.npz` |
| **T0** | `test_tier0_recovery.py` recovery gates |
| **CAL** | `test_calibration_synth.py` |
| **REAL** | real-data outputs: verdict, valuations, `player_stats`. Historical and non-parity (DR-B1, `docs/07-archive/cautious-nevermore/real-data-results.md`) |
| **INCR** | only the incremental `weekly_update` path, which is never a parity target |
| **NONE** | no oracle impact |
| **Ledger *n*** | position *n* in the `PARITY.md` correction ledger (DR-B1) |

**Rust port:**
- **MNR**: must not reproduce. The port must not copy this behaviour.
- **DESIGN INPUT**: a convention the port has to choose explicitly, in a model spec.
- **TYPED FAILURE**: becomes an explicit typed error (DR-B6).
- **DROP**: not ported.

**DR**: the decision in `docs/00-meta/decision-register.md` that governs the fix. "—" means no owner
decision is needed (an obvious default, or hygiene).

**Aliases** (CN combined IDs). Cite the primary ID.

| Alias | Primary |
|---|---|
| KI-V1 | KI-G1 |
| KI-#27 | KI-G6 |
| KI-P6 | KI-P2 |
| KI-#47 | KI-A5 |

---

## 1. Attribution: RAPM, Layer 3, Layer 1 (cn-issues §3.1)

| ID | Title | Sev | Status | Location | Evidence | Oracle impact | Rust port | DR |
|---|---|---|---|---|---|---|---|---|
| **KI-G1** (alias KI-V1) | The Layer-3 market pseudo-row anchors γ_off + γ_def, but `team_rating` reports γ_off − γ_def; the pattern is duplicated at three sites | HIGH | OPEN. **Meaning changed by critic X-2: the `[+1,+1]` row is right for net strength; the CN-queued `[+1,−1]` "fix" is rejected** | `reference/python/backend/grid/layers.py:400-408` (row), `:426` (team_rating); `reference/python/backend/validation/backtest.py:101-112`; `reference/python/backend/pipeline/weekly_update.py:269-276` | By reading. The `[+1,−1]` evidence (0.6644 → 0.7318 in `sign_test.py` and `sign_test2.py`; also 0.682 → 0.739 and 0.696 → 0.758 in other reports) was measured on the legacy synth and **must not be cited** (critic X-19) | GM-A `test_layerA_team_rating_sign_and_top`, GM-B `test_layerB_team_ranking_order`, GM-C `test_layerC_player_and_team_ratings`; T0 `test_team_strength_recovery` (observed 0.6643, legacy). Ledger 2 | **MNR**: do not port `[+1,−1]` or `team_rating = γ_off − γ_def`. DESIGN INPUT per DR-B5 | DR-B5, DR-B1 |
| **KI-NEW-A1** | Team-strength convention is inverted for real markets. In the play model a larger β_def is a better defence, so a spread prices net strength β_off + β_def; CN's off − def makes a great defence *lower* "team strength" | HIGH | OPEN (design) | `reference/python/backend/grid/layers.py:426`; `reference/python/backend/grid/synth.py:285-294` | `sign_test2.py` (legacy synth): net-strength option, 5-seed mean 0.536 vs 0.403 with no market. `w_market=40` against about 800–3,500 plays per intercept makes Layer 3 a weak nudge | as KI-G1 | DESIGN INPUT: net strength from gauge-invariant aggregates (DR-B5) | DR-B5 |
| **KI-NEW-A2** | The matchup-grade sign is inverted. The stored grade `−β_def` is an "easiness" score, and the docstring has the algebra backwards | HIGH | OPEN (confirmed). **Critic X-3: inverted, not "inconclusive"** | `reference/python/backend/pipeline/weekly_update.py:62-66` (docstring), `:84`; `reference/python/backend/validation/backtest.py:48-51`, `:185-186`; tautological tests `reference/python/tests/pipeline/test_weekly_update.py:286-370`, `reference/python/tests/validation/test_verdict.py:47-51` | `defsign_planted.py` (its own generator with correct defenders; 20k plays; planted −0.5/0/+0.5): the ELITE defence has β_def = +0.409 and gets grade −0.409; the WEAK defence has β_def = −0.382 and gets grade +0.382. `def_sign.py` on the defender-fixed synth: corr(β_def, planted D) = +0.53. CN's "−0.22, inconclusive" came from the legacy synth | REAL / INCR (`matchup_grades`, Tier-2 `def_grades`). The tests that pass encode the wrong sign. Ledger 3 | **MNR**: grade = +E_def, higher = tougher (DR-B5). Truth-anchored GM-A test | DR-B5 |
| **KI-NEW-A3** | On real data the starting QB is nearly collinear with the team-offense intercept, and the 20× lighter intercept ridge absorbs the starter's value | HIGH (real data) | OPEN (plausible, with real-data evidence) | `reference/python/backend/grid/layers.py:378-389` | `real_rapm.py` (2023 REG, full-roster universe): QB rating SD 0.042 ≈ WR 0.045 ≈ RB 0.044 (synth QB 0.259). corr(starter QB rating, own t_off) = 0.13 | REAL | DESIGN INPUT: multi-season pooling, a QB-specific prior or ridge, or Layer-1 credit. Validate on real data before freezing any real-RAPM golden | DR-C1, DR-C2, DR-B4 |
| **KI-G6** (alias KI-#27) | `layer1_all_qbs` silently drops QBs whose fit raises (`except Exception: pass`) | LOW | OPEN. Dead in production: only tests call it; `verdict.py:40,180` uses `layer1_all_players` | `reference/python/backend/grid/layers.py:541-557` (except at `:555-556`) | By reading | NONE | DROP. If ever needed: TYPED FAILURE plus log | — (decision-register "Defaults applied" item 4) |
| **KI-G12** | A missing `dv` column raises a bare KeyError | LOW | OPEN | `reference/python/backend/grid/layers.py:326` (the CN audit cited `:307` on an older tree; cn-issues cited `:327`) | By reading | NONE | TYPED FAILURE (contract validation) | DR-B6 |
| **KI-G14** | The `fit()` fixed point re-seeds the Layer-2 prior only for `focus_qb`. The production paths (`run_rapm`, `walk_forward`, `weekly_update`) have no Layer-1 → Layer-2 coupling at all | MOD | OPEN (by design: "light coupling") | `reference/python/backend/grid/layers.py:591-610` | By reading | GM-B/GM-C: the golden runs `fit()` (ratings, qb_credit) | DESIGN INPUT: generalise the re-seed, or drop the "fixed point" claim (`docs/05-model-specs/rapm-attribution.md`) | — |
| **KI-#32** | Position-specific λ (`lambda_by_pos`) is never passed by the production solvers. There is no WR/TE target-share attribution | MOD | OPEN | `reference/python/backend/grid/layers.py:381-386`; `reference/python/backend/pipeline/weekly_update.py:278`; `reference/python/backend/validation/backtest.py:114` | By reading | REAL | DESIGN INPUT | DR-C1, DR-C3 |
| **KI-#57** | Situation names are plural in code (`passing_downs`) but singular in the schema CHECK (`passing_down`), and nothing produces `situation_grades` | LOW | OPEN | `reference/python/backend/grid/situations.py:30-34`; `reference/python/backend/db/schema.sql:148` | By reading | NONE | DESIGN INPUT: one canonical situation enum | DR-C13 |
| **KI-NEW-A4** | `two_minute = quarter_seconds_remaining ≤ 120` ignores `qtr`, so it includes the ends of Q1 and Q3. The plays contract never emits the column, so on real data the mask is always absent | LOW | OPEN (latent) | `reference/python/backend/grid/situations.py:23`, `:57-58`; `reference/python/backend/grid/nflverse_adapter.py:127-146` | By reading | NONE | **MNR**: use `half_seconds_remaining`, which the adapter must emit (DR-C13) | DR-C13 |
| **KI-NEW-A5** | The Layer-1 "cross-fitted" residual uses `def_sum` from ratings fitted on all plays, held-out fold included, so the context feature is in-sample. Folds are also play-level `KFold(shuffle)`, while the calibration gate uses week-grouped folds | MOD (re-graded from LOW on 2026-10 real-data evidence) | OPEN (measured) | `reference/python/backend/grid/layers.py:496-519` (`def_sum` at `:506`, KFold at `:512`) | By reading; measured in `layer1-credit.md` §7.3, §7.5. Real 2023 (weeks ≤ 18): the context model's out-of-fold R² is 0.0144 with the shipped in-sample ratings and −0.0184 with fold-wise RAPM ratings, so the whole apparent opponent adjustment on real data is in-sample leakage. Fold-wise ratings move the focus-QB weekly credit by up to 0.024 (fixed synth) and 0.101 (legacy) | GM-C `qb_credit` (`test_layerC_qb_weekly_and_kalman`); REAL | DESIGN INPUT: fold-wise ratings or a frozen prior; week-grouped folds | DR-D14, DR-C7 |
| **KI-NEW-A6** | `np.linalg.cond(A)` (an SVD) runs before every solve. The `lstsq` fallback at cond > 1e10 is silent apart from a log line | LOW | OPEN | `reference/python/backend/grid/layers.py:355-358` | By reading. The fallback was added **deliberately** in CN PR #53, audit item C3 (critic G-3) | NONE | TYPED FAILURE: Cholesky with a typed singular failure. The reversal needs an ADR that cites PR #53 C3 (DR-B6) | DR-B6 |
| **KI-#42** | Matrix factorisation on RAPM residuals | — | OPEN (research) | — | — | NONE | Research backlog, `models` crate, post-MVP | — |
| **KI-NEW-Z1** | Layer-1 credit is an on-field-unit plus-minus, not individual credit. Every listed offensive player gets the same play residual, so RB/WR/TE credit mostly measures the team offense. The verdict still uses it as RB/WR/TE `smoothed_talent` | MOD | OPEN (design; measured) | `reference/python/backend/grid/layers.py:533-536`, `:578-587`; consumer `reference/python/backend/validation/verdict.py:127-191` | `layer1-credit.md` §7.4, §7.7. Fixed synth: RB/WR/TE starters' season credit correlates 0.961–0.967 with team starting-offense ability and 0.337–0.620 with their own; WR credit vs planted ability 0.497 (RAPM 0.860). Real 2023: main QB vs main WR weekly credit, median corr 0.909; QB season credit vs team mean residual 0.974 | REAL (verdict `smoothed_talent`). The QB-based golden and CAL values are unaffected, because the QB dominates the unit | DESIGN INPUT: an explicit estimand. Until it is decided, outputs carry `role = on_field` | DR-D13 |
| **KI-NEW-Z2** | Weekly Layer-1 aggregation groups by `week` only, so a multi-season frame silently merges seasons | LOW | OPEN (confirmed; latent: the verdict calls it once per season) | `reference/python/backend/grid/layers.py:535`, `:584`; workaround `reference/python/backend/validation/verdict.py:176-182` | The canonical synth split into two 7-week "seasons" with renumbered weeks returns 7 focus-QB rows instead of 12 (`layer1-credit.md` §7.4; re-run 2026-10-07) | NONE | **MNR**: key every record by (season, week) | — |
| **KI-NEW-Z3** | The Layer-1 context booster early-stops implicitly. scikit-learn's `early_stopping='auto'` switches on when the training rows exceed 10,000, so the algorithm changes at that size and `n_iter_` varies by fold. The split and its settings (`validation_fraction` 0.1, `n_iter_no_change` 10, `tol` 1e-7) are undocumented | MOD | OPEN (measured) | `reference/python/backend/grid/layers.py:514-516` (no `early_stopping` argument); scikit-learn 1.9.1 `ensemble/_hist_gradient_boosting/gradient_boosting.py:533-534` | `layer1-credit.md` §4.9, §5.3: `n_iter_` 136–200 (legacy synth), 47–72 (fixed) and 25–98 (real 2023) across folds | GM-C `qb_credit`; T0 and CAL Kalman gates, which observe Layer-1 credit | **MNR**: an explicit booster configuration, with `n_iter` recorded per fold. Sibling of KI-NEW-Z41 (V(s)) | DR-D14, DR-C7 |
| **KI-NEW-Z4** | Layer-1 inputs are not validated. A defender missing from `ratings_lookup` silently contributes 0.0 to the opponent feature, and NaN state features reach a booster that accepts missing values | LOW | OPEN (latent) | `reference/python/backend/grid/layers.py:493` (`.get(p, 0.0)`), `:506-508` | By reading (`layer1-credit.md` §8 rows 5 and 16) | NONE | TYPED FAILURE: `CreditError::UncoveredDefender`, unless the DR-D14 policy names a counted substitute | DR-D14, DR-B6 |
| **KI-NEW-Z5** | The Layer-1 opponent feature `D_i` sums on-field defender ratings but omits the defensive intercept `γ_def`, so it depends on how the solve splits an effect between intercept and players (gauge-dependent) | MOD | OPEN (design) | `reference/python/backend/grid/layers.py:493`, `:506` | By reading (`layer1-credit.md` §4.3; `rapm-attribution.md` §4.6) | GM-C `qb_credit` | DESIGN INPUT: `δ_i = γ_def[def_team(i)] + Σ β_p` | DR-D14, DR-B5 |
| **KI-NEW-Z6** | Layer-1 eligibility (`min_plays = 20`) is counted over the whole frame, which looks ahead within a season. An ineligible player is omitted rather than flagged | LOW | OPEN | `reference/python/backend/grid/layers.py:580-582` | By reading (`layer1-credit.md` §8 row 7) | NONE | **MNR**: as-of eligibility; `no_data` records | — |
| **KI-NEW-Z7** | Batch Layer-1 credit for week w depends on later weeks, because the context model and the defender ratings are fitted on the whole frame. The synthetic Kalman calibration and Tier-0 Kalman gates are therefore not strictly causal one-step tests | MOD | OPEN (design) | `reference/python/backend/grid/layers.py:496-519`, `:575-576`; consumers `reference/python/tests/grid/test_calibration_synth.py`, `test_tier0_recovery.py` | By reading (`layer1-credit.md` §8 row 8) | CAL; T0 Kalman gates | DESIGN INPUT: offseason or retrospective use only. Calibration gates that claim one-step semantics use causal credit | DR-C1, DR-C10 |
| **KI-NEW-Z8** | A NaN prior mean turns every RAPM coefficient into NaN, because the dense `L @ prior_mean` spreads it. `fit()` reaches this when the focus QB has no plays: the re-seed is then the mean of an empty series | MOD | OPEN (latent) | `reference/python/backend/grid/layers.py:352-354` (`_solve_ridge_prior`); `:605` (re-seed) | By reading (`rapm-attribution.md` §8 row 19; `layer1-credit.md` §8 row 12) | NONE (the synthetic focus QB always plays) | TYPED FAILURE: `ContractError::NonFinitePrior`, `CreditError::NoEvidence` | DR-B6, DR-D11 |
| **KI-NEW-Z9** | `fit()` defaults to `verbose=True`, which reads the synth-only `ability` column, so a default call fails on real data | LOW | OPEN | `reference/python/backend/grid/layers.py:591-592` (signature), `:606-608` | By reading (`rapm-attribution.md` §8 row 9; `layer1-credit.md` §8 row 19) | NONE | **MNR**: no estimator code reads synth-only columns | DR-D11 |
| **KI-NEW-Z10** | Unknown participant IDs are dropped from the design with only a log warning, while an unknown team or prior ID raises a bare `KeyError` | MOD | OPEN | `reference/python/backend/grid/layers.py:277-298` (warning at `:294`); `:308-313` (teams); `:376` (prior) | By reading (`rapm-attribution.md` §3.1, §8 row 6, citing CN PR #53 audit item C4). The first real verdict lost about 22k participants per origin this way (CN PR #89; lessons-learned LL-18) | REAL | TYPED FAILURE. Unknown participants are counted and reported, never dropped silently | DR-B6 |
| **KI-NEW-Z11** | The three market-row sites treat a team with no line differently: `run_rapm` raises `KeyError`, `backtest.solve_rapm` skips the team, and `weekly_update` anchors it to 0.0 | MOD | OPEN | `reference/python/backend/grid/layers.py:406`; `reference/python/backend/validation/backtest.py:105-106`; `reference/python/backend/pipeline/weekly_update.py:274` | By reading (`rapm-attribution.md` §4.4; reconcile-code-first §1.12) | INCR (the zero anchor). NONE on synth, where every team has a line | **MNR**: a team without a line at lock is not anchored; one market-row builder (LL-22) | DR-B5 |
| **KI-NEW-Z12** | The synthetic-harness market target is in ability units, while the estimand it anchors is in dV units: SD 0.0157 against `E_off + E_def` SD 0.268 (legacy) and 0.208 (fixed) EP per play | MOD | OPEN (measured) | `reference/python/tests/grid/test_tier0_recovery.py:34`, `golden_master.py:70`; `reference/python/backend/grid/synth.py:285-294` | `rapm-attribution.md` §4.4, from single solves with the oracle's `_solve_ridge_prior` | T0 `test_team_strength_recovery`; GM (the golden fit uses the market) | DESIGN INPUT: `s_t` in EP per play of net strength. Until then the synthetic market is labelled as ability units (`synthetic-world.md` G-5) | DR-D10, DR-B4 |
| **KI-NEW-Z13** | The WR×CB interaction block is unreachable from `run_rapm`, and the synth plants no WR×CB effect, so the interaction "recovery" test asserts only non-empty, finite output | LOW | OPEN | `reference/python/backend/grid/layers.py:369` (`build_design` called without `interactions`), `:335-342`; `reference/python/tests/grid/test_design_interactions.py` | By reading (`rapm-attribution.md` §4.1, §8 row 12; reconcile-code-first C25) | NONE | Research only: no interaction claim before an effect is planted | DR-B4 |

**Note on KI-G1, KI-NEW-A1 and KI-NEW-Y1 (critic X-2).**
- The defect is not the market row's sign. In the design matrix, defenders enter at −1, so a better
  defence has a larger β_def, and a spread prices *net* quality: off + def.
- The CN-queued `[+1,−1]` change only makes the code agree with the buggy generator (KI-NEW-Y0).
- What is wrong:
  - the reported team strength (`γ_off − γ_def`);
  - the generator's planted truth (`off − def`);
  - the generator's defender draw.
- The fix is decided in DR-B5. Net strength is computed from gauge-invariant aggregates, and the market
  anchors it using the line at lock.
- CN's `docs/04-lessons-learned.md` and `docs/08-phase-task-context.md` "Known Traps" both state the
  opposite. They are wrong and must not be ported (see lessons-learned LL-09).

---

## 2. Incremental weekly pipeline (cn-issues §3.2)

`weekly_update` is the in-season loop. Its findings compound, and the path cannot serve as an oracle. The
Rust incremental design must start from the batch walk-forward semantics (`backtest.py`, `verdict.py`).

| ID | Title | Sev | Status | Location | Evidence | Oracle impact | Rust port | DR |
|---|---|---|---|---|---|---|---|---|
| **KI-NEW-W1** | Season rollover: the accumulator skip guard `last_week >= week` has no season key, so week 1 of a new season is skipped forever. The Kalman state is not season-keyed either | CRIT (in-season) | OPEN (confirmed) | `reference/python/backend/pipeline/weekly_update.py:237-244`; `reference/python/backend/grid/layers.py:90-117` (accumulators store `week` only) | `rollover_test.py`: 2024 wk 18 is solved, then 2025 wk 1 returns `{'rapm_solved': False, 'skipped': True}` | INCR | **MNR**: key state by (season, week), with an explicit carry-over policy | DR-C10, DR-C6 |
| **KI-NEW-W2** | Any change in roster size or order reinitialises the RAPM accumulators and discards every prior week. nflverse rosters grow weekly | CRIT (in-season) | OPEN (confirmed) | `reference/python/backend/pipeline/weekly_update.py:245-252`; situation pass `:453-464` | `reinit_test.py`: one added player logs "dimension mismatch (16 vs 17) — reinitialising"; the P0 diagonal stays 60 instead of 120 | INCR | **MNR**: a stable player index, with XtX grown by zero-padding | DR-C6 |
| **KI-NEW-W3** | The Kalman observation is the **cumulative** season-to-date RAPM coefficient, fed every week to every player, including players who did not play | HIGH | OPEN. **Critic X-13: a defect, not a design choice** | `reference/python/backend/pipeline/weekly_update.py:280-309` | By reading. The validated batch path (`reference/python/backend/validation/verdict.py:126-200`, the golden, the calibration gates) observes weekly Layer-1 credit with `played` masks | INCR | **MNR**: weekly credit as y, exposures as precision, NaN for non-play | DR-C10, DR-C1 |
| **KI-#24** | `snaps = np.ones` makes R = r_scale / 1 (R = 0.40–0.55 against ratings of about 0.04 SD). Changepoint z cannot reach 3 on real data | HIGH (re-graded from LOW) | OPEN | `reference/python/backend/pipeline/weekly_update.py:307-309`; `detect_changepoints` call at `:381` | By calculation: with P0 = diag(.05, .02, .01), S ≈ 0.63, the week-1 gain on total ≈ 0.13 and z ≈ 0.05 | INCR | **MNR**: R from real exposures | DR-C10 |
| **KI-#15** | Cold-start look-ahead: the batch filter's `x0 = nanmean(y[:3])` lets the week-1 *filtered* estimate see weeks 2–3. `KalmanState.init` uses zeros, and priors are wired into neither path | HIGH | OPEN | `reference/python/backend/grid/statespace.py:183-188`; `:46-51` | By reading | GM-C `k_total_filt`, `k_total_pred` (`test_layerC_qb_weekly_and_kalman`; `reference/python/tests/grid/golden_master.py:88-111`). The RTS/smoothed use is retrospective and benign. Ledger 4 | **MNR** in the filtered path: x0/P0 from the prior, or diffuse | DR-C10, DR-B1 |
| **KI-A9** | Re-running an earlier week is a silent no-op, so stat corrections cannot be applied | MOD | OPEN (compounds with KI-NEW-W1) | `reference/python/backend/pipeline/weekly_update.py:240-244` | By reading | INCR | DESIGN INPUT: idempotent recompute from week-keyed deltas | DR-C12, DR-C14 |
| **KI-A8** | "1-D Kalman state not handled" | LOW | FALSE-POSITIVE as filed: `load()` only returns width 2 (migrated) or 3. Residual: a 1-D `mu` on disk raises IndexError at `statespace.py:103` instead of returning None | `reference/python/backend/pipeline/weekly_update.py:127-135`; `reference/python/backend/grid/statespace.py:103-118` | By reading | NONE | TYPED FAILURE on a state-file schema mismatch | DR-B6, DR-C15 |
| **KI-G5** | `var_total` omitted scheme_fit and the covariances | — | FIXED (CN #63, `9d30abe`, 2026-06-28). It now uses predictive S | `reference/python/backend/pipeline/weekly_update.py:105-141` | By reading | NONE | Keep: persist predictive S | — |
| **KI-W12** | Same as KI-G5, in the API fallback | — | FIXED (CN #63) | **CN-only**: `backend/api/routes/stats.py:258-268` | — | NONE | — | — |
| **KI-#25** | "Offensive ratings stored as def_team grades" | — | FIXED as filed, before the shallow boundary `c1b6748`; the GitHub issue is stale. **Superseded by KI-NEW-A2** | `reference/python/backend/pipeline/weekly_update.py:54-98` | By reading | — | See KI-NEW-A2 | DR-B5 |
| **KI-#31** | There is no schedule (player → opponent) table, so matchup grades are never keyed to opponents | MOD | OPEN | absent | — | NONE | DESIGN INPUT: nflverse schedules as an ingestion artifact | — |
| **KI-#33** | `trajectory_factor` stub | LOW | PARTIAL: the engine side exists as per-week `kalman_trajectory` snapshots; the trade side is app | `reference/python/backend/pipeline/weekly_update.py:105-170` | — | NONE | — | — |
| **KI-NEW-W4** | Called without frames, V(s) is fit on the whole season (`load_grid_plays([season])`), which leaks future weeks into dV on backfill | MOD | OPEN | `reference/python/backend/pipeline/weekly_update.py:210-214` | By reading | INCR | **MNR**: an as-of V(s), as `backtest.py:165-168` already does | DR-C7, DR-C6 |
| **KI-NEW-W5** | Health is reported "ok" unconditionally, even when every year failed | LOW | OPEN | `reference/python/backend/pipeline/weekly_update.py:488`; `reference/python/backend/pipeline/data_pipeline.py:262`; `reference/python/backend/pipeline/compute_valuations.py:221` | By reading | NONE | DESIGN INPUT: run records with a non-zero exit on failure | DR-C14 |
| **KI-NEW-Z14** | `weekly_update` saves the RAPM accumulators, non-atomically, before the solve and the Kalman step. A failure after the save leaves the week marked done, and a re-run is then skipped by the watermark | MOD | OPEN | `reference/python/backend/pipeline/weekly_update.py:259` (save), `:278` (solve), `:389-398` (Kalman), `:240-244` (skip); `reference/python/backend/grid/layers.py:117` (`np.savez`) | By reading. DR-B6 divergence item 4; the engine-spec §8.6.6 draft cites it without a KI ID | INCR | **MNR**: stage outputs commit atomically after the run succeeds, with a run record | DR-B6, DR-C15 |
| **KI-NEW-Z15** | On a player-universe change, `weekly_update` rebuilds the Kalman state for the current universe only, so the state of every player not in it is dropped | MOD | OPEN | `reference/python/backend/pipeline/weekly_update.py:296-305` | By reading (`state-space-kalman.md` §8.2 D-3) | INCR | **MNR**: carry state forward for absent players | DR-C10 |
| **KI-NEW-Z16** | The persisted Kalman state is incomplete. It lacks `SSParams`, `games_since_event`, the (season, week) and the data version. `state_version` is written but never read, the default path is relative to the working directory, and the write is not atomic | MOD | OPEN | `reference/python/backend/grid/statespace.py:28`, `:53-67` (save), `:105-111` (load) | By reading (`state-space-kalman.md` §8.2 D-4). Related: KI-NEW-W1 (no season key), KI-NEW-S1 (`games_since_event`), KI-A8 | INCR | **MNR**: a versioned state record, written atomically (`state-space-kalman.md` §6.4 item 4) | DR-C10, DR-C15, DR-B6 |
| **KI-NEW-Z17** | Auto-detected changepoints are applied in the week that detects them, so the persisted one-step predictive variance for that week is computed after looking at `y_w`. The band widens exactly when \|z\| > 3, which biases NIS and coverage | MOD | OPEN (latent: dormant on real data because of KI-#24) | `reference/python/backend/pipeline/weekly_update.py:381-397`; `reference/python/backend/grid/statespace.py:373-376`, `:396-399` | By reading (`state-space-kalman.md` §8.2 D-8) | INCR (trajectory `pred_var`) | **MNR**: the published predictive for week w is the pre-detection `(m_w, S_w)` | DR-D18 |

---

## 3. State-space (cn-issues §3.3)

| ID | Title | Sev | Status | Location | Evidence | Oracle impact | Rust port | DR |
|---|---|---|---|---|---|---|---|---|
| **KI-NEW-S1** | The batch filter (`kalman_two_component`) and the incremental filter (`kalman_step`) disagree after interventions. Post-event R inflation lasts 2 games in batch and only the intervention week in incremental; there is no persisted `games_since_event` | MOD | OPEN (confirmed) | `reference/python/backend/grid/statespace.py:192-233` vs `:397-411` | `kparity.py` (same x0/P0, intervention at week 6): identical through week 6, then \|Δ total_filt\| = 0.0093, 0.0039, 0.0037 … | INCR vs GM-C | **MNR**: one filter core with persisted event counters; parity test batch == incremental | DR-C10 |
| **KI-G4** | The RTS smoothed covariance `Ps = Pf + C(Ps₊ − Pp₊)Cᵀ` is not guaranteed PSD (`1e-10·I` jitter only) | LOW | OPEN in theory; not reproduced | `reference/python/backend/grid/statespace.py:237-245` | `psd.py`: 3,000 random stress runs gave min eigenvalue 0.0, max asymmetry 2e-17 and no negative `var_tau_smooth` | NONE | TYPED FAILURE plus symmetrise; prefer a stable (square-root or Joseph-style) smoother | DR-B6 |
| **KI-NEW-S2** | The focus-QB NIS gate `≤ 10` is too loose to be a parity target, and the QB calibration was tuned on the legacy synth | MOD | OPEN. **Changed by critic G-1** (see below) | `reference/python/tests/grid/test_tier0_recovery.py:119-128`; `reference/python/backend/grid/statespace.py:121-133` (QB `r_scale` 0.55, CN PR #66) | Legacy synth: focus-QB NIS 4.581, about 4.6× overconfident under `SSParams()`. Defender-fixed synth (`tier0_legacy_vs_fixed.py`): focus-QB NIS **8.371**, and CAL `test_pooled_nis_bounded` **fails** (pooled NIS 1.829 vs band [0.8, 1.4]) | T0 `test_filter_nis_is_bounded`; CAL `test_pooled_nis_bounded` | DESIGN INPUT: recalibrate on the fixed synth, and freeze the NIS value itself as the parity number | DR-B4, DR-B3, DR-C10 |
| **KI-#46** | Time-varying R and discount (DLM) | — | OPEN (research) | `SSParams` | — | NONE | Research, `models` crate; do KI-NEW-S1/S2 first | — |
| **KI-NEW-Z18** | ±inf observations are treated as missing by the batch filter but poison the incremental `kalman_step` | LOW | OPEN | `reference/python/backend/grid/statespace.py:222` (`isfinite`) vs `:403` (`isnan`) | By reading (`state-space-kalman.md` §8.2 D-1) | NONE | TYPED FAILURE: `NonFiniteObservation`. Only an explicit missing marker means missing | DR-B6 |
| **KI-NEW-Z19** | An unknown position label silently gets the default `SSParams`, and `weekly_update` maps a player with no position to `"UNKNOWN"` | LOW | OPEN | `reference/python/backend/grid/statespace.py:149-152`; `reference/python/backend/pipeline/weekly_update.py:359` | By reading (`state-space-kalman.md` §8.2 D-2) | NONE | TYPED FAILURE: `UnknownPositionClass` (QB/RB/WR/TE only) | DR-B6 |
| **KI-NEW-Z20** | The filter validates neither its inputs nor its parameters. An empty series (`W = 0`) raises `IndexError` | LOW | OPEN | `reference/python/backend/grid/statespace.py:168-239` (`IndexError` at `:239`) | By reading (`state-space-kalman.md` §4.9, §8.2 D-5) | NONE | TYPED FAILURE: `InvalidParams`, `EmptySeries` | DR-B6 |
| **KI-NEW-Z21** | A scheme reset overwrites, rather than adds to, the scheme inflation of an intervention in the same week. The test docstring calls the combined effect "additive" | LOW | OPEN (design) | `reference/python/backend/grid/statespace.py:376` vs `:387`; `reference/python/tests/grid/test_coaching_changes.py:170` | By reading (`state-space-kalman.md` §4.6, §8.2 D-6) | NONE | DESIGN INPUT: specify it explicitly. The default keeps the oracle semantics (`Σ[2,2] = scheme_reset_var`) | — |
| **KI-NEW-Z22** | The intervention discount is computed as `/d_steady · (d_steady/d_spike)` in `kalman_step` and as `/d_spike` in the batch filter, which is not bitwise equal | LOW | OPEN | `reference/python/backend/grid/statespace.py:368`, `:374` vs `:197-198` | By reading (`state-space-kalman.md` §8.2 D-7) | NONE. It breaks only a bitwise batch == incremental comparison | One filter core divides by `d_w` once | DR-C10 |
| **KI-NEW-Z23** | On did-not-play weeks the one-step predictive uses the snaps floor (`R = r_scale`), so a predictive is produced as if the player would play (golden values 0.404 and 0.406) | LOW | OPEN (design) | `reference/python/backend/grid/statespace.py:211-215`, `:396-399` | By reading (`state-space-kalman.md` §8.2 D-9) | GM-C `k_var_total_pred` on the injury weeks | DESIGN INPUT: mark it not applicable, or condition it on projected exposure | DR-D16 |

---

## 4. Cross-league priors (cn-issues §3.4)

| ID | Title | Sev | Status | Location | Evidence | Oracle impact | Rust port | DR |
|---|---|---|---|---|---|---|---|---|
| **KI-G8** | Small shared pools break the OOS R² computation: n = 2–3 gives −inf, n = 0 a KeyError, n = 1 a sklearn ValueError | MOD | OPEN (confirmed) | `reference/python/backend/grid/priors.py:103-111` | Reproduced in cn-issues | NONE (synth n is large) | TYPED FAILURE (minimum-n guard) | DR-B6, DR-C9 |
| **KI-#48** | One 70/30 split with seed 3 is unstable (±0.15) | LOW | OPEN | `reference/python/backend/grid/priors.py:103-111` | — | T0 `test_prior_equivalency_mapping` (`oos_r2 ≥ 0.05`; observed 0.1489 legacy, 0.2134 defender-fixed) | DESIGN INPUT: k-fold | DR-C9 |
| **KI-#23** | `league_factor` is computed and documented but never applied | MOD | OPEN | `reference/python/backend/grid/priors.py:93-94` vs `:143` | By reading | NONE | DESIGN INPUT: per-league slopes estimated from data, not a constant table | DR-C9 |
| **KI-#49** | Age and draft step functions: ±0.05 at age < 24 / > 30; round 1 +0.08, rounds 2–3 +0.03, undrafted −0.03 | MOD | OPEN. On the real rating scale these exceed 1–2 SD (KI-NEW-P4) | `reference/python/backend/grid/priors.py:41-68` | By reading. Origin: the lost CN Phase-3 plan (critic G-3) | NONE | DESIGN INPUT: back to the Statistical owner; not ported as-is | DR-C9 |
| **KI-NEW-P1** | `prior_var` is a hand-set `PRIOR_SD[pos]²`, not derived from the equivalency residual variance | MOD | OPEN | `reference/python/backend/grid/priors.py:148-149` | By reading | NONE | DESIGN INPUT | DR-C9 |
| **KI-NEW-P2** | Priors are wired to no real-data path: `prior_mean` is hard-coded 0.0 in the verdict. CN's "wired to real data" claim is false | MOD | OPEN | `reference/python/backend/validation/verdict.py:280`; `reference/python/backend/projection/features.py:20-23`, `:116-126` | By reading: nothing outside `backend/grid` calls `build_priors` or `estimate_equivalency` | REAL | DESIGN INPUT | DR-C9 |
| **KI-NEW-P3** | The `__main__` self-tests use `from synth import …`, which fails under `python -m` | LOW | OPEN (dead code) | `reference/python/backend/grid/priors.py:174-197`; `layers.py:613-631`; `statespace.py:419-448`; `value.py:113-123`; `synth.py:320-328` (all under `reference/python/backend/grid/`) | By reading | NONE | DROP | — |
| **KI-NEW-P4** | Scale constants are calibrated in synth units (QB ability SD 0.09; synth RAPM SD about 0.26). Real RAPM SD is about 0.04 at every position, so P0, `PRIOR_SD` and the age/draft adjustments are 4–5× too wide or large | HIGH (real data) | OPEN | `reference/python/backend/grid/priors.py:26`, `:41-68`; `reference/python/backend/grid/statespace.py:46-51`, `:121-147` | `real_rapm.py` | REAL | DESIGN INPUT: data-derived scale per run, recorded in the model spec; a realistic synth profile | DR-C9, DR-B4 |
| **KI-#44** | Hierarchical Bayesian equivalency | — | OPEN (research) | — | — | NONE | Research, `models` crate; would address KI-#23/#48 together | DR-C9 |
| **KI-NEW-Z24** | Scale mismatch at the prior hand-off: the feeder prior is on the RAPM-rating scale, while the Kalman state is on the Layer-1-credit scale (credit-on-rating slope 1.19–1.40, legacy synth). KI-#15's proposed `x0 = prior_mean` would mis-scale it | MOD | OPEN (design; latent while KI-NEW-P2 stands) | `reference/python/backend/grid/priors.py:71-126`, `:143`; `reference/python/backend/grid/statespace.py:183-188` | `cross-league-priors.md` §8.2 P-1 | NONE | DESIGN INPUT: one scale, or an explicit versioned scale map; typed `ScaleMismatch` | DR-D19 |
| **KI-NEW-Z25** | The Tier-0 rookie-prior gate cannot fail for any positive slope. Without age or draft columns `prior_mean` is affine in `feeder_sv`, so its correlation with ability equals corr(`feeder_sv`, ability) = 0.5829 on both generators | LOW | OPEN | `reference/python/tests/grid/test_tier0_recovery.py:141-143`; `reference/python/backend/grid/priors.py:143` | `cross-league-priors.md` §7.1 | T0 `test_rookie_prior_recovery` | DESIGN INPUT: an estimator-sensitive Class D gate | DR-B3, DR-C9 |
| **KI-NEW-Z26** | The washout diagnostic contradicts the filter it describes. `washout_table` is a one-component conjugate formula, fed with observation variances not derived from `SSParams` (0.40/90 in `priors.py`, 0.12² in `run_demo.py`). The implemented filter washes out the RB/WR/TE talent prior far more slowly, so "wide prior → fast washout" holds for QB only | MOD | OPEN (measured) | `reference/python/backend/grid/priors.py:154-171`, `:197`; `reference/python/run_demo.py:91` | `cross-league-priors.md` §4.4.2, from exact prior weights of `kalman_two_component` at 60 snaps a week: games to < 50% prior weight are RB 12, WR 11 and TE > 12 in the filter, against 2 each in the conjugate formula. `state-space-kalman.md` §8.2 D-10 | NONE (demo output only) | DROP the conjugate table as a diagnostic of record; derive the diagnostic from the actual filter | DR-D19 |
| **KI-NEW-Z27** | The equivalency regression is pooled across positions, while the prior variance is position-specific and the rating scale differs by position. Per-position slopes differ by up to 6× | MOD | OPEN (design) | `reference/python/backend/grid/priors.py:98-113` vs `:148` | `cross-league-priors.md` §7.2 (canonical legacy synth: QB slope 2.404, TE 0.416) | T0 `test_prior_equivalency_mapping` (pooled slope and OOS R²) | DESIGN INPUT: a per-position-class equivalency, or an explicit position term | DR-C9 |
| **KI-NEW-Z28** | Silent fallbacks in the prior translation: an unknown position gets `prior_sd` 0.10, an unknown league the factor 0.25, a NaN age or draft round no adjustment, and a named but absent column is skipped | LOW | OPEN | `reference/python/backend/grid/priors.py:148`; `:117`; `:41-68`; `:144-147` | By reading (`cross-league-priors.md` §8.2 P-5) | NONE | TYPED FAILURE: `UnknownPositionClass`, `UnknownFeederLeague`, `NonFiniteInput`, `MissingRequiredColumn` | DR-B6 |
| **KI-NEW-Z29** | The equivalency has no as-of handling: it is fitted on every shared player, whenever his NFL rating was observed | MOD | OPEN (design; latent while KI-NEW-P2 stands) | `reference/python/backend/grid/priors.py:71-126` | By reading (`cross-league-priors.md` §3.3, §8.2 P-6) | NONE | **MNR**: as-of equivalency fits (engine-spec §4.5) | DR-C9 |
| **KI-NEW-Z30** | Duplicate `player_id`s silently duplicate regression rows in the equivalency merge | LOW | OPEN (latent) | `reference/python/backend/grid/priors.py:98-99` | By reading (`cross-league-priors.md` §8.2 P-7) | NONE | TYPED FAILURE: `DuplicatePlayerId` | DR-B6 |
| **KI-NEW-Z31** | `feeder_snaps` is carried in the college contract but ignored: exposure affects neither the prior variance nor the regression weights | LOW | OPEN (design) | `reference/python/backend/grid/data_adapters.py:15`; `reference/python/backend/grid/priors.py:71-151` | By reading (`cross-league-priors.md` §8.2 P-8) | NONE | DESIGN INPUT: the statistical owner decides the weighting | DR-C9 |

---

## 5. Synthetic generator (cn-issues §3.5, critic G-1)

| ID | Title | Sev | Status | Location | Evidence | Oracle impact | Rust port | DR |
|---|---|---|---|---|---|---|---|---|
| **KI-NEW-Y0** | **Defenders are drawn from the offense team on every play** (critic G-1) | CRIT (P0) | OPEN (verified) | `reference/python/backend/grid/synth.py:193`: `off_pl, def_pl = _pick_onfield(tidx[off_team], rng)`. `_pick_onfield` (`:107-128`) returns both lists from one team index | Canonical `load_synthetic()` (16,825 plays): `def_players` ⊂ `off_team` on 100% of plays and ⊂ `def_team` on 0% (re-run 2026-10-01). `tier0_legacy_vs_fixed.py` (legacy vs fixed Tier-0) and `pytest_defender_fix.py` (the oracle's gates on the fixed generator); `sign_exp.py`, `sign_exp2.py` (SF C1) | **Every** team-strength, Layer-3, DEF-recovery and matchup number. On the fixed synth: GM 4 of 11 fail (`test_layerB_top_player_ranking`, `test_layerB_team_ranking_order`, `test_layerC_player_and_team_ratings`, `test_layerC_qb_weekly_and_kalman`, qb_credit Δ up to 0.317); CAL 1 of 6 fails (`test_pooled_nis_bounded`); T0 8 of 8 still pass. Ledger 1 | **MNR**: the Rust synth draws defenders from `def_team`, with its own planted-strength definition and its own golden | DR-B4, DR-B1, DR-B5 |
| **KI-G3** | With `cb_split=True`, CBs are left out of the team defensive mean | LOW (re-graded from CRIT) | OPEN | `reference/python/backend/grid/synth.py:292` | By reading: only `cb_split=True` configs (interaction tests) reach it; the default and the golden use `cb_split=False` | NONE | **MNR**: include CB in the defensive aggregate | DR-B4 |
| **KI-G7** | Float truthiness in `drive_points or 0.0` | — | FALSE-POSITIVE: a no-op, because `drive_points` is always 0.0 on a stalled drive (style only) | `reference/python/backend/grid/synth.py:260` | By reading | NONE | — (Rust types make it moot) | — |
| **KI-G11** | `__main__` divides by `terminal.mean()` | — | FALSE-POSITIVE: every drive has a terminal play; dead code | `reference/python/backend/grid/synth.py:324-326` | By reading | NONE | DROP | — |
| **KI-NEW-Y1** | Planted team strength is `mean(off starters) − mean(DEF starters)` | HIGH | OPEN (design). **Critic X-2: coherent only with KI-NEW-Y0; fixed together with it, and the planted truth becomes net quality (off + def)** | `reference/python/backend/grid/synth.py:285-294` | By reading | T0 `test_team_strength_recovery`; GM-A `test_layerA_team_rating_sign_and_top`; GM-B `test_layerB_team_ranking_order` | DESIGN INPUT: plant net strength (DR-B5) | DR-B5, DR-B4 |
| **KI-NEW-Y2** | The synth QB effect is about 6× the real one, and synth starters rotate out on 20% of snaps (real QBs do not). The synth is easiest exactly where KI-NEW-A3 bites | MOD | OPEN | `reference/python/backend/grid/synth.py:37-38`; `:107-128` | `real_rapm.py` vs synth | All synth gates (T0, GM, CAL) | DESIGN INPUT: a "realistic" synth profile (QB ~99% snaps, real RAPM scale) in the Rust parity suite | DR-B4 |
| **KI-NEW-Z32** | The 12-play drive cap is non-Markov: a drive that reaches it ends with 0 points whatever its state. Together with persistent lineup quality, this makes synthetic dV not state-centred, unlike real dV | MOD | OPEN (measured) | `reference/python/backend/grid/synth.py:49` (`max_plays_per_drive = 12`), `:192`, `:256-260` | `value-model.md` §7.4. 24.1% (legacy) and 29.1% (fixed) of drives end by play count. E[dV \| down 4] is +0.649 (legacy) and +0.556 (fixed), against +0.068 on real 2023. Raising the cap to 200 gives +0.344, and also removing player effects gives −0.027. An exact cell-mean V shows the same pattern, so the estimator is not the cause. Real 2023 state-only context R² is −0.0005 (`layer1-credit.md` §7.3) | GM, T0, CAL: the synth gates over-exercise the state part of the Layer-1 context model | DESIGN INPUT: the corrected generator ends drives by football events (`synthetic-world.md` G-8; realistic profile R-4) | DR-B4 |
| **KI-NEW-Z33** | Focus-QB truth mismatch: the Tier-0 and golden gates compare the focus QB's rating with his static draw (0.1001), which generated none of his plays. Every snap he played used the weekly τ (mean 0.1538 over played weeks) | MOD | OPEN (measured) | `reference/python/backend/grid/synth.py:196-201`; gate `reference/python/tests/grid/test_tier0_recovery.py:89-91` | `synthetic-world.md` §4.4, §8.3 N-2: QB recovery corr 0.869 against the static draw and 0.897 against the τ mean (legacy); 0.8845 and 0.9089 (fixed) | T0 `test_attribution_per_position` (QB); GM | DESIGN INPUT: gate on the effective truth (`synthetic-world.md` G-4) | DR-B4 |
| **KI-NEW-Z34** | The Tier-0 floors hold only for the canonical seed. With identical configuration, every one of the ten non-canonical generator seeds (1–11) fails at least one floor, on both generators | MOD | OPEN (measured) | `reference/python/tests/grid/test_tier0_recovery.py:84-143`; `reference/python/backend/grid/synth.py:45` (`seed = 7`) | `synthetic-world.md` §7.6. Fixed generator, seeds failing each gate: QB 4 (min 0.571), TE 5 (min 0.363), Kalman total_smooth 5 (min 0.622), equivalency slope 5 (max 2.475), team 6, pooled 1 (0.646), OOS R² 1 (−0.015) | T0 (all floors) | DESIGN INPUT: a seed-ensemble statistic. A Rust-native generator cannot be held to single-seed floors | DR-D26, DR-B3, DR-B4 |
| **KI-NEW-Z35** | Truth and estimand must move together. Pointing the unchanged team gate (`γ_off − γ_def`) at planted net strength reads 0.465 on the fixed world (0.566 legacy), below its 0.60 floor | LOW | OPEN (design: an ordering constraint on the ledger) | `reference/python/tests/grid/test_tier0_recovery.py:97-99` | `synthetic-world.md` §7.1, §8.3 N-4 | T0 `test_team_strength_recovery` | DESIGN INPUT: ledger entry 1 keeps the legacy key; entry 2 moves the gate, the estimand and the market together. E_off + E_def against net truth reads 0.959 (`rapm-attribution.md` §7.2) | DR-B1, DR-B5 |
| **KI-NEW-Z36** | No value-function truth is planted, so V(s) has never been validated against truth | MOD | OPEN (design gap) | `reference/python/backend/grid/synth.py:162-282` | `synthetic-world.md` §8.3 N-5; `value-model.md` §7.5 | NONE (no gate exists) | DESIGN INPUT: plant a value-function truth (`synthetic-world.md` G-8, SHOULD) | DR-B4, DR-C7 |
| **KI-NEW-Z37** | `_round_robin` silently idles one team every week when `n_teams` is odd. It is also not a round robin: each week is a fresh random pairing, so pairings can repeat | LOW | OPEN (confirmed; latent: the canonical world has 12 teams) | `reference/python/backend/grid/synth.py:151-159` | Re-run 2026-10-07: `n_teams = 5` schedules 4 teams in each of 3 weeks (`synthetic-world.md` §8.3 N-6) | NONE | TYPED FAILURE: `SynthConfigError::InvalidLeague`. The realistic profile uses a real schedule shape | DR-B6, DR-B4 |
| **KI-NEW-Z38** | The generator can list the same player twice in `off_players`: a backup drawn into two WR slots, or the injured focus QB's backup drawn into the QB slot and then prepended. The planted yards count his ability twice, and `build_design` sums the duplicates into a +2 design value | LOW | OPEN (confirmed) | `reference/python/backend/grid/synth.py:115-121`, `:204-210`; `reference/python/backend/grid/layers.py:326` (`csr_matrix` sums duplicates) | 245 of 16,825 canonical legacy plays (re-run 2026-10-07; plays-contract D-8). 30 legacy and 28 fixed of them come from the focus-QB backup (`synthetic-world.md` §4.4) | GM, T0 (legacy synth) | DESIGN INPUT: legacy parity fixtures treat participation as a multiset. The real-data profile forbids duplicates (`ContractError::DuplicateParticipant`) | DR-B4 |
| **KI-NEW-Z39** | The synthetic state space leaves football: `yardline_100` reaches 104, `ydstogo` can exceed `yardline_100`, and there are no safeties | LOW | OPEN | `reference/python/backend/grid/synth.py:188`, `:214-239` | plays-contract §6.2, D-9 | GM, T0: V(s) support differs between synthetic and real data | DESIGN INPUT: V(s) support declared per profile; the corrected and realistic profiles are football-valid | DR-B4 |
| **KI-NEW-Z40** | Synthetic truth columns (`ability`, `is_starter`) live in the `players` input frame, where estimator code can read them | LOW | OPEN | `reference/python/backend/grid/synth.py:57-83`; read by `reference/python/backend/grid/layers.py:417-422` and `:606-608` | plays-contract §4, D-13 | NONE | **MNR**: a separate `SyntheticTruth` type that estimator code cannot reach | DR-B4 |

**KI-NEW-Y0 consequences: legacy vs defender-fixed generator.**

Same `fit(n_iter=3)` and market seed 1, threads = 1. Source: `critic.md` G-1, `tier0_legacy_vs_fixed.py` (critic's `t0.py`), and the critic's fixed
copy, in which line 193 becomes two `_pick_onfield` calls (one on `tidx[off_team]` for offense, one on
`tidx[def_team]` for defense).

| Metric | Legacy (current gates observe) | Defender-fixed |
|---|---|---|
| pooled attribution corr | 0.8025 | 0.8276 |
| QB / RB / WR / TE / DEF | .869 / .7433 / .7973 / .7713 / .7653 | .8845 / .7711 / .8599 / .7784 / .7827 |
| team corr, vs the legacy off − def target (semantically wrong once fixed) | 0.6643 | 0.6596 |
| Kalman total_smooth / tau_smooth | 0.958 / 0.6745 | 0.976 / 0.6538 |
| focus-QB NIS (gate ≤ 10) | 4.581 | **8.371** |
| equivalency slope / OOS R² | 1.3159 / 0.1489 | 1.2119 / 0.2134 |
| rookie prior corr | 0.5829 | 0.5829 |

**What follows from the fix:**
1. Every legacy number is labelled "legacy synth (defender bug)". This covers the T0 observed values, the
   golden snapshot and the parity tables in the inventory reports. None is a Rust target (DR-B1).
2. The fix changes the RNG stream, because `_pick_onfield` is now called twice. The corrected generator
   therefore needs:
   - its own planted-strength definition (net quality, DR-B5);
   - a new golden;
   - re-calibrated QB NIS bands (DR-B4).

   A one-line patch is not enough.
3. After the fix, focus-QB NIS of 8.37 sits close to the ≤ 10 guard (KI-NEW-S2).

---

## 6. Value model and plays-contract adapter (cn-issues §3.6)

| ID | Title | Sev | Status | Location | Evidence | Oracle impact | Rust port | DR |
|---|---|---|---|---|---|---|---|---|
| **KI-NEW-V0a** | There is no `season_type` filter, so postseason plays (weeks 19–22; 1,638 rows in 2023) enter RAPM, V(s), the walk-forward origins and the verdict | MOD | OPEN (confirmed) | `reference/python/backend/grid/nflverse_adapter.py:91-92` | `real_contract.py` | REAL. Ledger 5 | **MNR**: an explicit `season_type` policy (REG only for training labels) | DR-C12 |
| **KI-NEW-V0b** | `drive_points` maps "Opp touchdown" (502 plays in 2023) and "Safety" (62) to 0, so V(s) ignores defensive scoring. The V(s) state also has no clock or score | MOD | OPEN; documented in the adapter as a scaffold (`:20-22`) | `reference/python/backend/grid/nflverse_adapter.py:36`, `:106` | `real_contract.py` | REAL | DESIGN INPUT: the EP target definition in `docs/05-model-specs/value-model.md` (7/3/0 kept for v1, DR-C13) | DR-C13, DR-C7 |
| **KI-NEW-V0c** | `REAL_LOADERS["pbp"]` raises NotImplementedError, while the real path (`nflverse_adapter.load_grid_plays`) bypasses the documented "swap point" | LOW | OPEN (stale contract) | `reference/python/backend/grid/data_adapters.py:39-46`, `:76-77` | By reading | NONE | DESIGN INPUT: a single loader trait (`docs/03-contracts/plays-contract.md`) | — |
| **KI-NEW-V0d** | Duplicate (game_id, play_id) participation keys make `part.loc[k]` return a DataFrame, and `_split_ids` would then raise | LOW | OPEN (latent; 2023 had no duplicates) | `reference/python/backend/grid/nflverse_adapter.py:205-213` | By reading | NONE | TYPED FAILURE | DR-B6 |
| **KI-NEW-Z41** | V(s) early stopping is hidden. With scikit-learn's `early_stopping='auto'`, any frame over 10,000 rows early-stops on a seeded random 10% validation split, so full-season fits train on 90% of the rows and never reach `max_iter = 300`. Frames of 10,000 rows or fewer, such as the backtest warm-up (4,784 / 8,017 rows), run all 300 iterations on all rows. The same call is two different estimators | MOD | OPEN (confirmed) | `reference/python/backend/grid/value.py:51-54`; scikit-learn 1.9.1 `gradient_boosting.py:533-534` | `value-model.md` §4.1: `n_iter_` 77 (legacy synth), 53 (fixed), 87 (real 2023); seeds 1–10 give 47–182 (legacy). Re-run 2026-10-07 on `load_synthetic()`: `n_iter_ = 77`, `do_early_stopping_ = True` | GM-C (every V(s)-dependent value); T0; REAL; the backtest warm-up V(s) | **MNR**: no size-dependent mode switch; any validation split is declared and recorded. Sibling of KI-NEW-Z3 | DR-C7, DR-D27 |
| **KI-NEW-Z42** | Oracle V(s) predictions leave the label range [0, 7]: from −0.240 to 7.046 on the canonical synth rows (up to 7.130 on the integer grid), and from −0.435 to 6.415 on real 2023 | LOW | OPEN (confirmed) | `reference/python/backend/grid/value.py:51-55`, `:92` | `value-model.md` §4.6, §8.2 N-3; synthetic range re-run 2026-10-07 | GM-C, REAL | TYPED FAILURE: `OutOfHull`. The engine estimator respects the hull by construction | DR-C7 |
| **KI-NEW-Z43** | Oracle V(s) is non-monotone. On first and 10 it rises with distance to goal at 18 (synth) and 17 (real) of 98 one-yard steps, and it rises with down in 191 of 1,368 synthetic cells | LOW | OPEN (design; measured) | `reference/python/backend/grid/value.py:51-54` (no monotonicity constraint: the library default `monotonic_cst=None`) | `value-model.md` §4.6, §8.2 N-6 | GM-C, REAL | DESIGN INPUT to the DR-C7 estimator choice (monotonicity constraints) | DR-C7 |
| **KI-NEW-Z44** | Out-of-support and sentinel states are evaluated silently: `V(−1, −1, −1) = 7.130` and `V(1, 10, −1) = V(1, 10, 1)` on the canonical synth, so a wrong terminal flag would give a silent dV of about +7 | LOW | OPEN (confirmed; latent) | `reference/python/backend/grid/value.py:89-104` | `value-model.md` §3.1, §8.2 N-4; re-run 2026-10-07 | NONE | TYPED FAILURE: `SentinelOnContinuingRow`, `OutOfSupport` | DR-B6 |
| **KI-NEW-Z45** | The V(s) artifact is unversioned: `cache_path` silently overwrites an earlier model, and the artifact is a joblib (pickle) file | LOW | OPEN | `reference/python/backend/grid/value.py:40-45` (docstring), `:57-59`, `:84-86` | By reading (`value-model.md` §8.1 row 8; reconcile-spec-first R55) | NONE | **MNR**: a versioned artifact, written atomically, with no pickle | DR-C15 |
| **KI-NEW-Z46** | An unknown `fixed_drive_result` value silently scores 0 in `drive_points` | LOW | OPEN | `reference/python/backend/grid/nflverse_adapter.py:36`, `:106` | plays-contract D-3; `value-model.md` §8.1 row 4 | REAL | TYPED FAILURE: a closed result table; an unknown value is `SchemaDrift` (P1-03) | DR-C13 |
| **KI-NEW-Z47** | A `fixed_drive` that spans a change of possession is treated as one drive: the adapter chains the next state across offenses and broadcasts one result to both | LOW | OPEN (confirmed: one drive in 2023) | `reference/python/backend/grid/nflverse_adapter.py:97-99`, `:110-121` | plays-contract §6.3, D-4 | REAL | **MNR**: never chain the next state across an `off_team` change; quarantine the drive (P1-03) | — |
| **KI-NEW-Z48** | "Participation unavailable" is indistinguishable from "empty". Absent participation, or an unmatched row, yields `()`, and RAPM then fits a team-intercept-only design without error | MOD | OPEN | `reference/python/backend/grid/nflverse_adapter.py:184-186`, `:211-213`; `reference/python/backend/grid/layers.py:469-473` | plays-contract D-5 | REAL | **MNR**: `Participation::{Listed, Unmatched, NotAvailable}` plus frame-level availability. Participation-dependent stages reject frames whose participation is not available as of the lock | DR-C1 |
| **KI-NEW-Z49** | Producer dtypes drift between the synthetic and nflverse producers: `play_id` and `yardline_100` are float in real data and int in synthetic data, ids are strings in real data and ints in synthetic data, and `drive_id` and the position vocabularies differ | LOW | OPEN | `reference/python/backend/grid/synth.py:241-248`; `reference/python/backend/grid/nflverse_adapter.py:127-146` | plays-contract §2.1, D-12. The Parquet round-trip part is KI-V8 | NONE | DESIGN INPUT: the v1 canonical types (plays-contract §11) | — |
| **KI-NEW-Z50** | The plays contract carries no involvement roles (passer, rusher, receiver, sack, scramble), which Layer-1′ needs. Snap counts have no provider contract, no PFR → GSIS crosswalk and no licence ruling | MOD | OPEN (design gap) | `docs/03-contracts/plays-contract.md` §11; `docs/04-providers/nflverse/README.md:76`; `docs/04-providers/nflverse/access-and-license.md:40` | `layer1-credit.md` §8 row 21 | NONE | DESIGN INPUT: an optional, versioned involvement-roles field and a snap-count provider contract before Layer-1′ (P1-03) | DR-D15, DR-C1 |

---

## 7. nflverse box-score ingest (cn-issues §3.7)

The figures below come from `cmp_stats.py`. It runs CN's `_normalize_pbp` and `_aggregate_stats_from_pbp`
on the 2023 nflverse PBP (weeks ≤ 18) and compares the result with the official nflverse `player_stats`
(REG), inner-joined on `player_id`. Real-data inputs are fetched, not committed, by
`reference/python/tools/investigations/fetch_realdata.py` (DR-A11).

| ID | Title | Sev | Status | Location | Evidence | Oracle impact | Rust port | DR |
|---|---|---|---|---|---|---|---|---|
| **KI-NEW-I1** | `pass_attempts` counts sacks, and `passing_yards = Σ yards_gained` nets sack yardage | CRIT | OPEN (confirmed) | `reference/python/backend/pipeline/data_pipeline.py:50-62`; `reference/python/backend/grid/nflverse_loader.py:66` | `cmp_stats.py`: 19,658 vs 18,315 (+7.3%); 119,092 vs 128,567 (−7.4%); 1,459 sack rows carry a passer. Completions, INTs, targets and receptions match exactly | REAL. Ledger 5 | **MNR**: labels from official stats (DR-C12) | DR-C12 |
| **KI-NEW-I2** | `td_type` comes from `touchdown` (any TD on the play), so pick-sixes and fumble-return TDs are credited as passing, receiving or rushing TDs | CRIT | OPEN (confirmed) | `reference/python/backend/grid/nflverse_loader.py:73-77`; consumed at `reference/python/backend/pipeline/data_pipeline.py:57,73,89` | `cmp_stats.py`: passing_tds 814 vs 754, receiving_tds 799 vs 754, rushing_tds 473 vs 470. 66 offensive plays have `touchdown=1` with neither `pass_touchdown` nor `rush_touchdown` | REAL. Ledger 5 | **MNR**: use `pass_touchdown` / `rush_touchdown` and `td_player_id` | DR-C12 |
| **KI-G10** | Row-wise `df.apply` builds `td_type` | LOW | OPEN (performance). The real defect at these lines is KI-NEW-I2 | `reference/python/backend/grid/nflverse_loader.py:73-77` | — | NONE | — | — |
| **KI-A1** | "fumbles_lost counts ALL fumbles" | — | FALSE-POSITIVE: the column comes from `fumble_lost` (also in the CN #94 triage). CN `docs/04-lessons-learned.md` still repeats the false claim | `reference/python/backend/grid/nflverse_loader.py:78`; `reference/python/backend/pipeline/data_pipeline.py:97-114` | By reading | — | — (the real defect is KI-NEW-I3) | — |
| **KI-NEW-I3** | Fumble *attribution* is wrong: `fumbler = rusher if rush else receiver`. Sack fumbles, which have no receiver, are dropped, and QB fumbles on pass plays are charged to the targeted receiver | HIGH | OPEN (confirmed) | `reference/python/backend/pipeline/data_pipeline.py:101-105` | `cmp_stats.py`: fumbles_lost 174 vs 256 official (−32%), 47 players mismatched | REAL. Ledger 5 | **MNR**: use `fumbled_1_player_id` plus `fumble_lost` | DR-C12 |
| **KI-NEW-I4** | There is no `season_type` filter in the stats path, so postseason weeks 19–22 are summed into season totals and per-game rates | HIGH | OPEN (confirmed) | `reference/python/backend/grid/nflverse_loader.py:53-54` → `reference/python/backend/pipeline/compute_valuations.py:44-66`, `reference/python/backend/validation/verdict.py:533-562` | `real_contract.py`: 1,638 postseason rows in the normalised 2023 PBP | REAL. Ledger 5 | **MNR** | DR-C12 |
| **KI-NEW-I5** | `two_point_conversions` is never populated (always 0), and rush attempts are −2.8% because kneels are excluded | MOD | OPEN (confirmed) | `reference/python/backend/pipeline/data_pipeline.py:34-139`; `reference/python/backend/scoring/columns.py` | `cmp_stats.py`: rush attempts 14,178 vs 14,588 | REAL | DESIGN INPUT: stat definitions | DR-C12 |
| **KI-G2** | `complete_pass` is a scalar 0 when the column is missing | — | FALSE-POSITIVE: the scalar broadcasts to the existing index, and real nflverse always ships the column | `reference/python/backend/grid/nflverse_loader.py:81` | By reading | — | — | — |
| **KI-A15** | `week = 0` is not rejected | LOW | OPEN (trivial: PBP never has week 0) | `reference/python/backend/pipeline/data_pipeline.py:178-182` | By reading | NONE | TYPED FAILURE | DR-B6 |
| **KI-A7** | There is no DEF aggregation, and K is selected but never aggregated, so K and DEF projections are empty | MOD | OPEN (missing feature) | `reference/python/backend/pipeline/compute_valuations.py:65` | By reading | NONE | DESIGN INPUT: K/DST are Phase 2 and scored separately (alpha-spec §2.1, superseded) | — |
| **KI-G9** | `ParquetCache`: expired files are never deleted and there is no stale-while-revalidate. `put` is non-atomic, so a crash leaves a "fresh" partial parquet. `_path` key sanitising collides (`"a/b"` and `"a_b"` map to one file) | LOW | OPEN. The TTL boundary test passes on Linux (it fails only on Windows mtime granularity; LL-04) | `reference/python/backend/grid/cache.py:16-28` | By reading | NONE | **MNR**: temp file plus rename, content-addressed keys. `ParquetCache` itself is not ported (DR-C15) | DR-C15 |
| **KI-NEW-I6** | Loader tests write to a global `/tmp/test_cache` instead of `tmp_path` | LOW | OPEN (test hygiene) | `reference/python/tests/grid/test_nflverse_loader.py:47`, `:62` | By reading | NONE | — | — |

---

## 8. Projection (cn-issues §3.8)

| ID | Title | Sev | Status | Location | Evidence | Oracle impact | Rust port | DR |
|---|---|---|---|---|---|---|---|---|
| **KI-P1** | A NaN `prior_mean` crashed `Ridge.predict` | — | FIXED (`ccc1ffe`, squashed into CN #94 / `91ae204`, 2026-07-17: `nan_to_num → 0.0`). Residual: 0.0 conflates "no prior" with "prior = 0", and the fit side's `fillna(0.0)` (`:138`) does the same | `reference/python/backend/projection/model.py:74-83`, `:138` | By reading | NONE | DESIGN INPUT: an explicit missing-indicator feature | DR-C3 |
| **KI-P2** (alias KI-P6) | NaN handling (`max(0.0, NaN) → 0.0`) in the orphaned `fantasy_scoring.py` | LOW | OPEN, but the module is orphaned and **not imported** into the oracle | **CN-only**: `backend/fantasy_scoring.py:32`, `:57` | By reading | NONE | DROP | — |
| **KI-P3** | A missing `VOLUME_COLS` column crashes `groupby.agg` | LOW | OPEN (latent: callers fill the columns) | `reference/python/backend/projection/volume.py:78-81` | By reading | NONE | TYPED FAILURE | DR-B6 |
| **KI-P4** | A missing stat or feature column gives a KeyError | LOW | OPEN (latent) | `reference/python/backend/projection/model.py:134-145` | By reading | NONE | TYPED FAILURE | DR-B6 |
| **KI-P5** | NaN credit or points: `np.ptp(NaN) == 0` is False, so `polyfit` runs on NaN | MOD | OPEN | `reference/python/backend/projection/sv_to_points.py:82-84` | By reading | NONE | TYPED FAILURE | DR-B6 |
| **KI-P7** | The exact `== 0` spread check lets a near-zero spread produce a degenerate slope | LOW | OPEN | `reference/python/backend/projection/sv_to_points.py:82` | By reading | NONE | A tolerance-based guard | — |
| **KI-NEW-R1** | The per-unit rate regression is **unweighted**: `y = stat/volume` from a 1-attempt player weighs the same as from a 600-attempt player | HIGH | OPEN | `reference/python/backend/projection/model.py:134-145` | By reading | REAL | **MNR**: WLS by volume, or a Poisson/binomial GLM | DR-C3 |
| **KI-NEW-R2** | "Games" counts only weeks with a stat row, which drops active-but-no-touch weeks and inflates backups' per-game rates. `k_shrink = 8.0` is a constant, though the docstring says "learned". Postseason rows are included | MOD | OPEN | `reference/python/backend/projection/volume.py:79`; docstring `:12-14`; `:55` | By reading | REAL | **MNR**: games from participation/snaps; fit k by EB | DR-C3 |
| **KI-NEW-R3** | The weekly H2 GRID forecast is `affine(RAPM rating)`, fit in-sample within the pre-period. The Kalman is not used in the weekly forecast; it only feeds ROS `smoothed_talent` | MOD | OPEN (methodology) | `reference/python/backend/validation/verdict.py:78-93`; `reference/python/backend/validation/tier1.py:67-95` | By reading | REAL | DESIGN INPUT | DR-C4, DR-C5 |
| **KI-A4** | There is no per-week projection producer; the season total is stored under a `week=0` sentinel | LOW | OPEN (deferred in CN #94) | `reference/python/backend/pipeline/compute_valuations.py:208`; **CN-only** reader `backend/trades/trade_model.py:57-67` | By reading | NONE | DESIGN INPUT: a typed horizon field, never a sentinel (`docs/03-contracts/engine-output-contract.md`) | DR-C4 |
| **KI-NEW-Z51** | The verdict's "smoothed" talent is the filtered endpoint: `assemble_smoothed_talent` returns `tau_smooth[-1]`, which equals `tau_filt[-1]` exactly by the RTS boundary condition. Inside its window it also inherits the KI-#15 look-ahead initialisation | LOW | OPEN (confirmed) | `reference/python/backend/projection/features.py:111`; `reference/python/backend/grid/statespace.py:239` | `projection-stack.md` §1.3 item 2: the difference is exactly 0 over 200 random series. Re-run 2026-10-07: exactly 0 over 50 series with gaps and an intervention | REAL (the ROS feature) | DESIGN INPUT: name and compute it as end-of-window filtered talent, from a prior-based `x0`/`P0` | DR-C10 |
| **KI-NEW-Z52** | The volume shrinkage target averages each player's own most recent prior season, even when that season is years old (a departed player), and the average is not weighted by games | MOD | OPEN | `reference/python/backend/projection/volume.py:91-97` | By reading (`projection-stack.md` §4.1, §8 row 6) | REAL | **MNR**: a window-estimated, exposure-weighted target | DR-C5, DR-C3 |
| **KI-NEW-Z53** | Current-season usage is never used: the volume base is each player's most recent prior season only | MOD | OPEN (design) | `reference/python/backend/projection/volume.py:86-91` | By reading (`projection-stack.md` §8 row 7; reconcile-spec-first R28) | REAL | DESIGN INPUT: the engine-spec §2.4 window with in-season data. The oracle rule survives as a parity mode | DR-C6 |
| **KI-NEW-Z54** | Receptions are projected as volume, so catch rate is not modelled, and `targets` is projected but drives nothing | MOD | OPEN (design) | `reference/python/backend/projection/volume.py:34`; `reference/python/backend/projection/model.py:54-58` | By reading (`projection-stack.md` §1.3 item 3) | REAL | DESIGN INPUT: `receptions = targets × catch_probability` (Layer D) | DR-D21, DR-C3 |
| **KI-NEW-Z55** | The projection fixes `fumbles_lost` and `two_point_conversions` at 0.0 | MOD | OPEN | `reference/python/backend/projection/model.py:22-24`, `:50-53`, `:99` | By reading (`projection-stack.md` §8 row 4; reconcile-code-first C6). The label side is KI-NEW-I5 | REAL | **MNR**: modelled with strongly shrunk rates | DR-C12, DR-D21 |
| **KI-NEW-Z56** | Stat-line constraints are not enforced. The linear ridge rate is unbounded, so completions can exceed pass attempts, and negative values are silently clamped to 0. `receptions ≤ targets` holds by linearity unless an override sets only one of the two | LOW | OPEN | `reference/python/backend/projection/model.py:101`, `:107`; `reference/python/backend/projection/volume.py:110-116` | By reading (`projection-stack.md` §8 row 8) | REAL | **MNR**: rates on their support; typed `ImpossibleStat` | DR-D21, DR-B6 |
| **KI-NEW-Z57** | The ROS rate models are fitted on pre-first-origin player-season labels paired with the first-origin RAPM rating, but the forecast uses each origin's own rating. Fit and serve share a column, not a quantity | MOD | OPEN | `reference/python/backend/validation/verdict.py:276-277` vs `reference/python/backend/validation/tier1.py:161` | By reading (`projection-stack.md` §8 row 9) | REAL | **MNR**: one feature definition shared by fit and serve, as of the label's own period | DR-C3 |
| **KI-NEW-Z58** | One verdict run uses two V(s) models: the `smoothed_talent` path refits V(s) on the whole pre-first-origin slice, while `walk_forward` fits it on the warm-up slots | LOW | OPEN | `reference/python/backend/validation/verdict.py:172` vs `reference/python/backend/validation/backtest.py:166-168` | By reading (`projection-stack.md` §8 row 13) | REAL | **MNR**: one versioned V(s) per data version and as-of | DR-C7, DR-C6 |
| **KI-NEW-Z59** | `sv_to_points` documents a weekly Layer-1 credit → points map, but the verdict fills its `credit` column with RAPM ratings. This naming and scale confusion sits on top of KI-NEW-R3 | LOW | OPEN | `reference/python/backend/projection/sv_to_points.py:54-73`; `reference/python/backend/validation/verdict.py:79-95` | By reading (`layer1-credit.md` §8 row 18; `projection-stack.md` §8 row 10) | REAL | "Credit" denotes Layer-1 and Layer-1′ credit only; the SV map is a diagnostic | DR-C4, DR-C5 |
| **KI-NEW-Z60** | `SVToPointsMap.predict` returns 0.0 for a position without a fitted map, although its docstring says such cases are "surfaced as no projection" | LOW | OPEN | `reference/python/backend/projection/sv_to_points.py:42-49` | By reading (`projection-stack.md` §8 row 18) | REAL | A typed absence (no projection), never 0.0 | DR-B6 |

---

## 9. Validation (cn-issues §3.9)

| ID | Title | Sev | Status | Location | Evidence | Oracle impact | Rust port | DR |
|---|---|---|---|---|---|---|---|---|
| **KI-NEW-V1** | H1/H2 CIs are **iid** percentile bootstraps over correlated cells, so they are too narrow | HIGH | OPEN | `reference/python/backend/validation/lineup_sim.py:187-210`; `reference/python/backend/validation/tier1.py:296-304`; `reference/python/backend/validation/metrics.py:134-180` | By reading | REAL (the headline H2 "PASS" CI) | **MNR**: week-clustered paired bootstrap | DR-C5 |
| **KI-NEW-V2** | Tier-2 has never run on engine-produced grades; an intercept-only grade carries no validated signal | MOD | OPEN. Measured on the legacy synth, so it must be re-measured on the fixed synth (critic X-1) | `reference/python/tests/validation/test_tier2.py:32` (planted grades); `reference/python/backend/validation/tier2.py` | `defgrade_test.py` (3 seeds): the KPI is about 0 for either sign, and the CIs include 0. `defgrade_test2.py`: adding on-field defender ratings did not help (both legacy synth) | REAL | DESIGN INPUT: grade = total defensive effect (E_def), gated on real points allowed | DR-B5 |
| **KI-V2** | The walk-forward watermark guard fires only within a season, and accumulation runs across seasons with no decay | MOD | OPEN | `reference/python/backend/validation/backtest.py:178-181`, `:191-198` | By reading | REAL | **MNR**: check across seasons. Multi-season weighting is a design input | DR-C6 |
| **KI-V3** | `normal_cdf` uses `np.vectorize(erf)` | LOW | PARTIAL: the "0-d array" part is a false positive; the slowness is real | `reference/python/backend/validation/metrics.py:34-37` | By reading | NONE | — | — |
| **KI-V4** | `crps_gaussian` with sd = 0 gives NaN; it should be \|actual − mean\| | MOD | OPEN (confirmed) | `reference/python/backend/validation/metrics.py:209-220` | Reproduced in cn-issues | NONE | TYPED FAILURE, or the closed-form limit | DR-B6 |
| **KI-V5** | `nis` with var ≤ 0 gives inf | MOD | OPEN (confirmed) | `reference/python/backend/validation/metrics.py:192-201` | Reproduced in cn-issues | NONE | TYPED FAILURE | DR-B6 |
| **KI-V6** | `pinaw` returns 0.0 when the actual range is 0 | LOW | OPEN | `reference/python/backend/validation/metrics.py:248-250` | By reading | NONE | TYPED FAILURE or NaN | DR-B6 |
| **KI-V12** | `spearman` returns 0.0, not NaN, when there is no variance, and the Tier-2 KPI averages these | LOW | OPEN (confirmed) | `reference/python/backend/validation/metrics.py:296-298` | Reproduced in cn-issues | NONE | NaN plus exclusion | DR-B6 |
| **KI-V7** | Non-numeric or None ranks raise TypeError | LOW | OPEN | `reference/python/backend/validation/sniff.py:33-44`, `:73` | By reading | NONE | TYPED FAILURE | DR-B6 |
| **KI-V8** | After a parquet round-trip, participation cells are `numpy.ndarray`, and `slice_pool` raises TypeError | MOD | OPEN (confirmed; latent, since only tests call `slice_pool`) | `reference/python/backend/validation/asof.py:92-114` | Reproduced in cn-issues | NONE | **MNR**: typed participation lists | — |
| **KI-V9** | `len()` is called on None/NaN participation | LOW | OPEN (latent: the adapter always emits tuples) | `reference/python/backend/pipeline/ingest_grid.py:29-39` | By reading | NONE | — | — |
| **KI-V10** | Snapshot writes are non-atomic, and the manifest is written last | MOD | OPEN | `reference/python/backend/pipeline/ingest_grid.py:63-81` | By reading | NONE | **MNR**: temp file plus rename, manifest hash | DR-C15 |
| **KI-V11** | An empty history returns `sd={}` rather than None | LOW | OPEN | `reference/python/backend/validation/baselines.py:73-102` | By reading | NONE | A typed Option | — |
| **KI-V1** | Alias of KI-G1 (the `backtest.py` market row) | — | see KI-G1 | `reference/python/backend/validation/backtest.py:101-112` | — | — | — | DR-B5 |
| **KI-#41** | Testing gaps | LOW | PARTIAL. The engine part is mostly fixed (`test_layers.py`, `test_kalman_numerical.py`, `test_tier0_recovery.py`, `test_golden_master.py` exist). There is no coverage tooling and no end-to-end real-data test | — | — | — | — | — |
| **KI-NEW-Z61** | The record-list branch of `AsOf.slice_interventions` compares the week only and ignores the season. The DataFrame branch is season-aware | LOW | OPEN (confirmed; latent: only tests call it) | `reference/python/backend/validation/asof.py:138-147` (DataFrame branch `:135-137`) | Re-run 2026-10-07: `AsOf(2023, 5)` keeps a (2024, week 2) record and drops a (2022, week 17) one (`evaluation-and-leakage.md` §1.3 item 2) | NONE | **MNR**: season-aware, with `announced_at < lock` (guard G6) | DR-D1 |
| **KI-NEW-Z62** | `run_verdict` passes one seed to both the bootstrap and the Layer-1 cross-fit behind `smoothed_talent`, so changing the evaluation seed changes the forecast being evaluated | MOD | OPEN (measured) | `reference/python/backend/validation/verdict.py:414-415`, `:438-445` | `projection-stack.md` §5.2 (legacy synth): seed 0 vs 1 moves `smoothed_talent` by up to 0.0487, against a cross-player SD of 0.198 | REAL | **MNR**: separate, recorded model and evaluation seeds | DR-C5 |
| **KI-NEW-Z63** | `bootstrap_ci` materialises an n × N int64 index matrix and a value matrix of the same shape: about 563 MB each at the real H1 size (10,000 × 7,035) | LOW | OPEN (performance) | `reference/python/backend/validation/metrics.py:169-171` | By calculation (`evaluation-and-leakage.md` §5.1) | NONE | Streaming resamples | — |
| **KI-NEW-Z64** | `top_n_hit_rate` and `ndcg_at_k` use NumPy's unstable default `argsort`, so tie handling is implementation-defined. `ndcg_at_k` also assumes non-negative relevance without checking, and fantasy points can be negative | LOW | OPEN | `reference/python/backend/validation/metrics.py:308-309`, `:318`, `:324` | By reading (`evaluation-and-leakage.md` §8 row 18) | NONE | A deterministic tie-break; a typed check on relevance | — |
| **KI-NEW-Z65** | `skill_score` silently returns 0.0 when the reference score is 0 | LOW | OPEN (confirmed) | `reference/python/backend/validation/metrics.py:111-112` | Re-run 2026-10-07: `skill_score(5.0, 0.0) = 0.0` | NONE | TYPED FAILURE, or NaN with exclusion | DR-B6 |
| **KI-NEW-Z66** | The Tier-1 calibration spread is the SD of past signed errors around their own mean, so it leaves out the forecaster's bias | LOW | OPEN | `reference/python/backend/validation/tier1.py:267` | By reading (`evaluation-and-leakage.md` §8 row 15) | REAL | Calibration from the Layer-F predictive distribution; diagnostics declare their spread | DR-C5 |
| **KI-NEW-Z67** | The walk-forward universe is the fixed `players` frame, not the as-of pool. A not-yet-seen player's normal equation decouples, so he gets a rating of exactly 0 and, where his position has a fitted map, a forecast. `slice_pool` is used only by tests | MOD | OPEN | `reference/python/backend/validation/backtest.py:193` | By reading (`evaluation-and-leakage.md` §1.3 item 5). Other RAPM coefficients are unaffected; the spurious ratings and forecasts look like evidence | REAL | **MNR**: the as-of universe; `no_data` records (LL-18) | — |
| **KI-NEW-Z68** | The walk-forward folds current-season participation into RAPM at every in-season origin, although nflverse publishes participation only after the postseason. This is train/serve skew, and the headline real H2 (+0.848) was measured this way | HIGH | OPEN (design) | `reference/python/backend/validation/backtest.py:189-198`; publication note `reference/python/backend/grid/nflverse_loader.py:175-178` | reconcile-code-first C1; reconcile-spec-first R14; critic G-5 | REAL: the H2 result is not live evidence | **MNR**: a publication-lag axis. Runs that relax it are labelled research-only | DR-C1 |
| **KI-NEW-Z69** | The verdict scores with STANDARD instead of the Half-PPR default comparison profile | LOW | OPEN | `reference/python/backend/validation/verdict.py:47`, `:427`, `:551-563` | By reading (reconcile-code-first C5; lessons-learned LL-27) | REAL | DESIGN INPUT: the scoring profile is part of the protocol version; Half-PPR is the default | DR-C5 |
| **KI-NEW-Z70** | Tier-1 survivorship: a cell exists only when a player has both a forecast and a realized row, so players inactive after the forecast are dropped, and the ROS target averages only the weeks the player has rows for | MOD | OPEN | `reference/python/backend/validation/tier1.py:258-260`, `:183-187` | By reading (`evaluation-and-leakage.md` §4.7; reconcile-spec-first R46) | REAL | **MNR**: the union pool with inactive = 0 (decision register, "Defaults applied" item 6) | DR-C5 |
| **KI-NEW-Z71** | `AsOf.slice_market` passes a static `{team: strength}` dict through unchanged, and slices a per-week table at week granularity rather than at the lock. The synthetic market is planted truth | MOD | OPEN (latent: the real market loader is a stub) | `reference/python/backend/validation/asof.py:116-126`; `reference/python/backend/validation/backtest.py:136-141` (static anchor) | By reading (`evaluation-and-leakage.md` §4.2 G7) | NONE on real data today; on synth the market is planted | **MNR**: only lines published before the lock | DR-B5, DR-D2 |
| **KI-NEW-Z72** | The oracle's gate is a function of the run it judges: `run_verdict` promotes each KPI's gate from a sign-flip null of the same run's samples, then checks that run against it. Freeze-once does not make it pre-registered | MOD | OPEN (methodology) | `reference/python/backend/validation/verdict.py:470-476` | By reading (`evaluation-and-leakage.md` §1.3 item 4; critic X-16) | REAL | **MNR**: gates pre-registered on a disjoint calibration period, and never for spec thresholds | DR-C5 |
| **KI-NEW-Z73** | The threshold registry's docstring says the frozen gates are committed, while its test asserts that the committed registry is empty | LOW | OPEN (documentation) | `reference/python/backend/validation/thresholds.py:1-19`; `reference/python/tests/validation/test_thresholds.py:76` | By reading (cn-docs §14 item 8) | NONE | Committed, versioned registry entries | DR-C5 |

---

## 10. Scoring (cn-issues §3.10)

| ID | Title | Sev | Status | Location | Evidence | Oracle impact | Rust port | DR |
|---|---|---|---|---|---|---|---|---|
| **KI-A3** | `resolve_or_create_format` returned the wrong id on a name collision | — | FIXED (`ffb3074` / `868cfa6`, squashed into CN #94 / `91ae204`). Residual: KI-NEW-C1 | `reference/python/backend/scoring/format_registry.py:36-83` | By reading | NONE | `format_registry` is outside the port scope (ADR-011; critic X-20) | — |
| **KI-NEW-C1** | Config identity is the raw JSON string, so it is key-order-sensitive and two equal rule sets can create duplicates | LOW | OPEN | `reference/python/backend/scoring/format_registry.py:54`, `:60` | By reading | NONE | Canonical (sorted) serialisation plus a content hash for scoring-profile versions | — |
| **KI-A5** (alias KI-#47) | VOR tiers fragment near zero and negative VOR (`denom = 1.0` when `prev_vor == 0`) | MOD | OPEN | `reference/python/backend/scoring/vor.py:92-119` (`:112`) | By reading | NONE | **MNR** if VOR is ported inside `evaluation` (absolute plus relative gap) | DR-C11 |
| **KI-A6** | FLEX replacement was skipped for positions not in `roster_slots` | — | FIXED (CN #85, `64216d6`, 2026-07-01) | `reference/python/backend/scoring/vor.py:47-71` | By reading | — | — | DR-C11 |
| **KI-NEW-C2** | Replacement is at 0-based index `slots*num_teams`, i.e. rank +1. The code and its docstring agree; CN `CLAUDE.md` is off by one | LOW | OPEN (doc nit) | `reference/python/backend/scoring/vor.py:38` | By reading | NONE | State it precisely in the spec | DR-C11 |

**Inventory error, not a defect.** `cn-docs.md` §2.6 gives `assign_tiers` a `max_gap_pct` of 0.15. The code
default is 0.10 (`reference/python/backend/scoring/vor.py:92`). Do not propagate the inventory figure.

---

## 11. Research items to carry, not port (cn-issues §3.11)

The research items live in their area tables: KI-#42 (§1), KI-#44 and KI-#49 (§4), KI-#46 (§3). The two
below were filed against app modules but are engine-relevant.

| ID | Title | Sev | Status | Location | Evidence | Oracle impact | Rust port | DR |
|---|---|---|---|---|---|---|---|---|
| **KI-#43** | Copula / correlated outcomes (filed against `trade_model`) | — | OPEN (research) | — | — | NONE | `simulation` crate: Layer F joint distributions | DR-C2 |
| **KI-#45** | Player × defence embedding for matchup edges (filed against `trade_model`) | — | OPEN (research) | — | — | NONE | `models` / `features`; depends on KI-#31 and on a validated matchup signal (KI-NEW-V2) | DR-B5 |

---

## 12. Seed and context data (critic G-4)

| ID | Title | Sev | Status | Location | Evidence | Oracle impact | Rust port | DR |
|---|---|---|---|---|---|---|---|---|
| **KI-NEW-D1** | `coaching_changes_2025.json` holds wrong or implausible real-world rows, and CN docs call it "hand-curated" default seed data | MOD (latent: no number depends on it today) | OPEN | `reference/python/backend/db/data/coaching_changes_2025.json` (14 rows, byte-identical to CN). Loaded only by an explicit `seed_coaching_changes()` call (`reference/python/backend/db/seed_coaching.py:15-29`) | Critic G-4, judged from general knowledge; the Data/Licensing owner still has to confirm against a sourced list. 2024-cycle changes labelled 2025: TEN HC Vrabel → Callahan, CAR HC Reich → Canales, WAS DC Del Rio → Whitt, LAR OC LaFleur → Robinson. Wrong roles: CHI OC Getsy → Ben Johnson (Johnson was hired as HC); DAL OC Schottenheimer → Kellen Moore (Schottenheimer became HC). Implausible mid-season rows: ATL OC → Kirk Cousins (a quarterback), HOU DC → Duce Staley. The other rows are unverified, not confirmed. Tests pass explicit rows (`reference/python/tests/grid/test_coaching_changes.py:304`) | NONE today: never loaded by tests or by any automatic pipeline path | **MNR**: never a fixture, golden or provider input. In Rust, coaching changes come from a provider contract with sourced, dated records and an information timestamp (AsOf intervention-foreknowledge rule) | DR-D1, DR-A11 |

The file stays verbatim in the oracle (the import is byte-identical by policy). `reference/python/README.md`
carries the warning: "illustrative, unverified; never a fixture or provider input".

---

## 13. Unverifiable audit IDs

| ID | Title | Sev | Status | Location | Evidence | Oracle impact | Rust port | DR |
|---|---|---|---|---|---|---|---|---|
| **KI-G13** | An ID missing from the CN 2026-07-13 engine audit (MINOR tier; the tables skip from G12 to G14) | — | UNVERIFIABLE: no text survives in CN `docs/06-issues-log.md`, and no audit source exists in git history | — | — | — | — | — |

Also unverifiable, and listed without IDs because none exist: the CN audit totals claim
"11 CRIT / 54 MOD / 24 MINOR", but the tables enumerate fewer. Missing are 9 of the 19 application-logic
MODERATE items and 1 of the 5 A-MINOR items. W4 and W16 are app-only. The earlier PR #53 audit (C1–C7, W1,
W7) is a different audit; its document was never committed (critic G-3).

---

## 14. Parity criteria, fixtures and documentation (found in the 2026-10 consolidation)

| ID | Title | Sev | Status | Location | Evidence | Oracle impact | Rust port | DR |
|---|---|---|---|---|---|---|---|---|
| **KI-NEW-Z74** | The DR-B3 Class C criterion (corr ≥ 0.999 and \|ΔV\| ≤ 0.10 EP on supported cells) is unsatisfiable for both booster stages, even by the oracle against itself under a seed change | MOD | OPEN (measured) | DR-B3; `docs/03-contracts/parity-fixture-contract.md` §6 and `reference/python/PARITY.md` (d) put both booster stages in Class C | V(s), seeds 1–10 (`value-model.md` §7.3): corr(dV) 0.9891–0.9942 (legacy), 0.9896–0.9931 (fixed), 0.9940–0.9960 (real 2023); max \|ΔV\| on states with support ≥ 120 up to 0.270 / 0.200 / 0.216 EP; only 14 synthetic states (about 11% of rows) have that support. Layer-1 residual, fold seeds 1–4 (`layer1-credit.md` §5.3): 0.9936–0.9941 (legacy), 0.9966–0.9974 (fixed) | Parity Class C (proposed) | DESIGN INPUT: stage-specific criteria C-V (DR-D27) and C-L1 (`layer1-credit.md` §10.3) | DR-B3, DR-D27 |
| **KI-NEW-Z75** | The golden master's comment attributes the ~1e-2 multi-threaded drift to V(s), but V(s) is thread-invariant; the drift appears downstream. `layer1-credit.md` §5.2 repeats the claim, and lessons-learned LL-09 did until 2026-10-07 | LOW | OPEN (documentation) | `reference/python/tests/grid/golden_master.py:55-59` | `value-model.md` §5.2: V(s) predictions at 1 and 4 OpenMP threads differ by 0.0. Under 4 threads the golden drifts in ratings (9.6e-4), `qb_credit` (0.021) and `k_total_filt` (0.0185) | NONE (the golden runs single-threaded) | Documentation only; the oracle is unchanged. The Rust determinism gate covers every stage | — |
| **KI-NEW-Z76** | `tests/grid/golden/snapshot.npz` stores the focus QB's `qb_week` and `qb_credit` but not their exposures (snaps), which the Kalman needs as precision. `state-space-kalman.md` §10.3 PF-SS-02 says the snapshot holds the series | LOW | OPEN | `reference/python/tests/grid/golden/snapshot.npz` (keys listed 2026-10-07) | `layer1-credit.md` §8 row 20. The exposures from a live run are 99, 97, 105, 97, 88, 82, 84, 116, 99, 87, 75, 81 | GM: the Kalman parity case PF-SS-02 cannot be built from the snapshot alone | Layer-1 and Kalman fixtures export exposures from the run | — |
| **KI-NEW-Z77** | Wrong `value.py` line citations in two contracts (`value.py` has 123 lines): plays-contract §1 cites `:87,111-112,151-166` and §2.1–§2.2 cite `:112`, `:157`, `:159`, `:161-164`, `:169-172`; parity-fixture-contract §5 cites `:153-164` and `:113-116` | LOW | OPEN (documentation) | `docs/03-contracts/plays-contract.md` §1, §2.1–§2.2; `docs/03-contracts/parity-fixture-contract.md` §5 | `value-model.md` §8.2 N-8. The correct sites are `:50` (label), `:51-54` (estimator; `min_samples_leaf` at `:53`), `:89-104` (`compute_dv`: terminal `:95`, `terminal_value` `:97`, next state `:99-102`) and `:107-110` (`attach_dv`) | NONE | — (the contract owners fix the citations) | — |

---

## Parity trust map (cn-issues §4, as adjudicated by critic G-1 / X-4)

**Valid regardless of the defender bug.** These are input-driven, closed-form outputs. They stay valid
parity material when fed committed fixtures:
- Kalman/RTS on given inputs;
- `metrics.py` on non-degenerate inputs;
- scoring arithmetic (`engine.calculate_points`);
- ridge/RAPM solves on given dV and design;
- priors arithmetic on a given permutation.

**Must not be frozen as Rust targets:**
- every synth recovery number and golden value from the legacy generator (KI-NEW-Y0), including the
  player-recovery floors that cn-issues §4 had judged safe;
- golden Layer B/C team ratings and `qb_credit` (KI-G1, KI-G14, KI-NEW-A5);
- golden `k_total_filt` / `k_total_pred` (KI-#15);
- anything from `weekly_update` (§2 above);
- every real-data number: the verdict, valuations and `player_stats` (§7; KI-NEW-V1, KI-NEW-Z68);
- any single-seed Tier-0 floor as a gate for a Rust-native generator (KI-NEW-Z34; DR-D26);
- booster outputs held to the DR-B3 Class C bound, which the oracle cannot meet against itself
  (KI-NEW-Z74; see C-V, DR-D27, and C-L1).

Rust targets the **corrected** oracle after the DR-B1 ledger. The legacy golden is kept for audit only.

## Related items owned elsewhere

- **Typed-failure divergences** (DR-B6). The list lives in `reference/python/PARITY.md`. It covers:
  - the `lstsq` fallback;
  - `KalmanState.load` returning None followed by a reinit;
  - `weekly_update` skip-on-failure;
  - non-atomic accumulator saves;
  - refitting V(s) on every run;
  - mutable `.npz` state;
  - unpersisted `SSParams`.

  These have KI rows:
  - KI-NEW-A6: the `lstsq` fallback;
  - KI-A8: `KalmanState.load`;
  - KI-NEW-W4: refitting V(s) on every run;
  - KI-G9 and KI-V10: non-atomic writes;
  - KI-NEW-Z14: accumulators saved before the solve;
  - KI-NEW-Z16: unpersisted `SSParams` and the incomplete `.npz` state.

  The `weekly_update` skip-on-failure has no row.
- **`reference/python/tests/grid/test_performance.py:187-198`** asserts a wall-clock speedup of at least
  1.5×. That is flake-prone under CI CPU contention (critic G-6). It runs with threads pinned to 1 and is
  listed as non-gating in `PARITY.md` if it flakes.
- **The golden Layer C and `test_cache.py::test_ttl_expired` failures** that CN reported are Windows-only.
  Both pass on Linux, the platform of record. This is a platform rule, not a defect: lessons-learned LL-04,
  DR-A3.
