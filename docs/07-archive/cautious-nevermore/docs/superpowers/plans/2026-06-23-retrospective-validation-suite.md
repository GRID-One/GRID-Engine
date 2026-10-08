# Retrospective Validation Suite — Design Plan

**Date:** 2026-06-23
**Author:** Claude (brainstorm → design)
**Status:** LOCKED — approved, implementation not yet started (pending external validity review)
**Branch:** `claude/retrospective-validation-testing-ibbub8`
**Relevant code:** `backend/grid/` (engine), `backend/scoring/` (fantasy points + VOR),
`backend/pipeline/compute_valuations.py` (today's projection), `backend/pipeline/weekly_update.py`
(incremental forward path), `run_demo.py` (existing synthetic recovery),
`backend/db/data/coaching_changes_2025.json` (changepoint seed).

---

## 1. Purpose

Validate that the GRID engine **predicts real fantasy outcomes out of sample**, not just
that its estimators recover planted synthetic truth. Today the only validation is
ground-truth recovery (`run_demo.py`): it proves the *math* works on data generated from
known abilities. It does **not** answer the product question — *does GRID beat the
baselines a fantasy manager already has?* This suite answers that, and guards against
silent regressions (e.g. the mislabeled `matchup_grades` noted in the Phase-4 plan).

"Retrospective" here means **walk-forward backtesting**: at each historical point in time,
produce predictions using only information available *then*, and score them against what
actually happened.

## 2. Decisions locked (from review)

- **Two horizons, both first-class.** Weekly (last-minute injury replacement) **and**
  rest-of-season / aggregate (bench-depth shopping). They get *different* machinery and
  *different* calibration standards (§7).
- **Scaffold the attribution/matchup backtest now** (Tier 2). **Real data is available** —
  see §15: nflverse `pbp_participation` (FTN Data, CC-BY-SA) supplies per-play
  `offense_players`/`defense_players` free for 2016–2025. Tier 2 runs on synth in CI and is
  unblocked on real *historical* data once `load_participation` is implemented (scheduled in
  roadmap Phase 1). It is NOT blocked on data/cost — only on a small loader.
- **Historical scope approved: ingest seasons 2019–2024** (5 train + holdout). Pipeline
  plumbing exists (`nflverse_loader` fetches + caches real Parquet; network verified
  reachable). A thin ingest script is in scope (§10).
- **Two engine prerequisites approved as in-scope edits** (both small + additive, touch
  `backend/grid/`): (a) surface the Kalman one-step predictive `(mean, S)`; (b) make the
  cache paths (`_ACCUM_PATH`/`_KALMAN_PATH`) injectable. See §7 and §8.
- **Calibrate-then-gate.** Only Tier 0 / 0.5 (deterministic) are hard CI gates from day one.
  Real-data KPIs run report-only for one backtest pass to measure the noise floor, then the
  directional KPIs are promoted to gates against the *observed* baseline distribution.
- **Two headline verdicts**, one per lever, both vs the last-season-actuals baseline (what
  `compute_valuations` ships today):
  - **H1 (bench-depth lever) — kill criterion:** GRID beats last-season-actuals on
    **rest-of-season skill**. If it can't, the engine's complexity isn't paying rent.
  - **H2 (injury-replacement lever) — weekly lineup point margin:** a GRID-set weekly
    lineup produces a higher *realized* weekly point total than a lineup set from
    last-season data, on an identical roster with an identical slot-filling rule. Success =
    cumulative weekly point margin > 0 (CI excl. 0), reported overall **and** restricted to
    weeks with a starter OUT (the true injury-replacement case).
    - **Roster source:** synthetic (VOR-drafted) rosters for now — reproducible, no data
      dependency. **Revisit:** populate H2 with *real* league rosters as part of a planned
      **draft-day → week-1 update** (sync real post-draft rosters via `sync_leagues`, then
      re-run H2 against actual lineups people fielded). Tracked in §13 / §14.

## 3. The two kinds of validation (both stay)

| | Ground-truth recovery (exists) | Real out-of-sample backtest (new) |
|---|---|---|
| Source | `synth.py` planted abilities | historical `player_stats` |
| Question | does the estimator recover truth? | does GRID beat real baselines? |
| Determinism | seeded, exact | noisy, data-dependent |
| Role | hard CI gate (math regressions) | report → promoted gates (product claim) |

## 4. Tiered structure

- **Tier 0 — Recovery.** Formalize `run_demo.py` correlations into asserted, seeded tests.
- **Tier 0.5 — Golden master + determinism.** Freeze engine output on a fixed synth seed;
  catch *any* drift and *mislabeling*. (Design in §6.)
- **Tier 1 — Real walk-forward backtest.** The product deliverable; report + promoted gates.
  Primary targets: weekly fantasy points (Kalman filtered) and ROS VOR/rankings — these need
  no participation. RAPM/matchup targets (Tier 2) use real participation once `load_participation`
  lands (§15). (Calibration design in §7; leakage design in §8.)
- **Tier 2 — Attribution / matchup backtest.** Scaffolded against the `data_adapters`
  contract; runs on synth in CI today, and on real historical data once `load_participation`
  is implemented (Phase 1) — participation is free via nflverse FTN through 2025 (§15).

---

## 5. Target choice (what we score against)

Realized fantasy points = volume × efficiency + TD variance. Volume is sticky/predictable;
efficiency and TDs are largely weekly noise. Therefore:
- Score **both** realized points **and** a decomposed opportunity target
  (carries/targets/snaps), reported separately. Nailing opportunity but not TD variance is a
  *good, honest* result and reveals the ceiling.
- **Horizon split = the two levers.** Weekly is TD-variance-dominated; ROS/aggregate averages
  it out and maps to draft/trade decisions. Both reported per position.

**Evaluation population (no silent cheating):**
- **No survivorship** — freeze the eval set to everyone rosterable *as of W*; score
  injuries/benchings as the low outcomes they were.
- **DNP/zero weeks** scored two ways (0 vs excluded), reported; the Kalman's
  predict-without-update absence path validated as its own question.
- **Per-position always**, ranking metrics **top-N** (what people draft/start), not full-list.

---

## 6. Tier 0.5 — Golden master design (deep)

The motivating bug (`matchup_grades` storing *mislabeled* ratings) is correct-looking numbers
under the wrong semantic. A float-freeze `allclose` may miss it and gets "fixed" by
regenerating. So the golden master is **three layers**, most→least robust:

- **Layer A — schema & semantic invariants (exact, no tolerance).** Column names, dtypes,
  shapes, label→meaning mapping, **plus sign/relationship checks anchored to planted truth**
  ("`matchup_grade` correlates *negatively* with opposing-defense planted strength";
  "`talent` is the most persistent Kalman component"; "strong planted defenses rank top").
  This is the layer that actually catches mislabeling, and it **cannot be gamed by
  regenerating** because it's anchored to truth, not last run.
- **Layer B — ordering & membership invariants (exact ints/sets).** Player rank order
  (argsort), tier assignments, top-N membership, changepoint count + week-location, sign/shape
  of the focus-QB injury dip. Robust to float jitter, sensitive to logic changes.
- **Layer C — numeric values (`assert_allclose`, rtol 1e-5 / atol 1e-6).** Ratings, team
  ratings, qb_weekly credit, Kalman filtered/predictive states for canonical players, VOR.
  Failure reporter prints the **largest-moving rows** (player, old, new, Δ) so failures are
  actionable, not silenced.

**Prerequisite — determinism audit.** Run synth→fit→score twice in-process; assert match
< 1e-9. Hazards: pin `random_state` on `HistGradientBoostingRegressor` and the Layer-1
`KFold`; canonical `player_id` column order (already a concern in `build_design`); pin
`OMP_NUM_THREADS=1` to kill BLAS reduction-order jitter; **`requirements.txt` uses `>=`** —
record/pin library versions for the validation env.

**Cross-platform reality.** Dev builds on Windows; CI/container is Linux (floats diverge
~1e-6). Golden files generated on a **platform of record** (Linux/CI); Layers A/B
(ints/strings/signs) carry regression-detection weight because they're platform-invariant;
Layer C tolerances absorb cross-platform jitter.

**Update workflow.** One command regenerates golden files (small, committed). The **golden
diff in a PR is the feature** — shows reviewers which players moved. Anti-gaming backstop:
Tier 0 recovery thresholds + Layer A semantic invariants must *also* pass; those are
truth-anchored and can't be made green by regenerating.

---

## 7. Calibration design (deep) — GRID's moat

**Engine gap to fix first (also fix the DB write path):** `kalman_two_component` returns
`total_filt` (mean) and `var_total_filt = H·P_filtered·Hᵀ`. **And `_write_kalman_trajectory`
(weekly_update.py) persists a 2-component (`talent+form`, dropping `scheme_fit`), R-omitted
variance — the value the viz/UX actually surfaces.** The calibration fix must reach *both* the
return value and this write path, or the shipped trajectory uncertainty stays wrong. For calibration that variance is wrong twice
over — it's the *filtered* (post-update, mildly circular) variance, and it **omits
observation noise R**. The honest one-step predictive variance `S = H·P_predict·Hᵀ + R` is
**already computed inside the update loop but never returned.** Surface the per-week
predictive `(mean = H·xp, var = S)`. Without this, every coverage number is overconfident by
construction. (Smoothed covariances `var_tau_smooth`/`sigma_smooth` are off-limits for
forecast calibration — retrospective, uses future weeks.)

**Three distributions, named:** filtered/nowcast (weak evidence, circular), one-step-ahead
(the honest weekly target, needs `S`), multi-step/ROS (variance compounds through
predict-with-no-update). One-step ↔ injury-replacement lever; multi-step ↔ bench-depth lever.

**R depends on snaps you don't know yet** (`R = r_scale/snaps`). Report **conditional**
calibration (given realized snaps — isolates the value model) and **unconditional**
(forecasts snaps too — honest). The gap quantifies volume-uncertainty vs value-uncertainty.

**Metrics (chosen to diagnose, not just flag):**
- **NIS** `z_w = innov/√S`, `mean(z²)≈1`. Audits the discount `d` and `r_scale` knobs.
  Runs on synth with no fantasy-space mapping → a **deterministic gate**.
- **PIT histogram** `u_w = Φ(z_w)`; read the shape (∪ overconfident, ∩ underconfident, slope
  bias, edge-spikes zero-inflation).
- **Coverage + sharpness together** — PICP @ 80/50 **paired with** PINAW. Never coverage
  alone (predicting the marginal is calibrated but useless).
- **CRPS** (closed form for Gaussian, no sampling) — headline, reported as **skill vs a
  baseline that emits a spread** (season-to-date mean ± sd).
- **Pinball at floor (20th) & ceiling (80th), separately** — ties to levers: injury
  replacement cares about ceiling/safe floor, bench depth about floor. A model can be
  median-calibrated yet wrong in the decision-driving tail.

**Non-Gaussianity & the aggregation escape.** Weekly points are skewed, TD-lumpy,
zero-inflated → Gaussian PIT *will* show tail defects (reality, not bug). **ROS totals are
sums of ~17 weeks → CLT → most Gaussian.** Consequence baked into the KPIs:
**ROS → Gaussian coverage/CRPS gates; weekly → quantile/pinball + randomized PIT for the zero
mass.**

**Regime-conditional calibration (proves the fancy parts):** first-game-back coverage with
vs without the rust inflation; during-absence variance growth + comeback coverage; rookie
prior-variance coverage in early weeks.

**Real-data scope (updated — weekly skill-position signal now in alpha scope).** Today the
weekly Layer-1 signal is **QB-only** and the Kalman runs in **SV/dV currency**, not
fantasy points. The roadmap moves **per-position weekly Layer-1 credit + an SV→fantasy-points
map** into Phase 2b (required for H2 on RB/WR/TE — without them H2 is unwinnable for skill
positions, and weekly per-player projections for RB/WR/TE simply do not exist). Calibration
scope = (a) NIS on the value signal (now); (b) weekly fantasy calibration for **all skill
positions** once the weekly credit + SV→points map land; (c) **ROS-aggregate** (most Gaussian)
— still the most robust, start here.

---

## 8. Leakage guards (deep) — three axes, not one

`run_rapm` builds from **accumulated normal equations** persisted to fixed global paths
(`_ACCUM_PATH`, `_KALMAN_PATH`); `weekly_update.py` appends weeks. Leakage therefore has
three axes:
1. **Temporal** — future rows reach a stage.
2. **Scope** — a global fit spanning time leaks into a local prediction even with correct rows
   (V(s), VOR replacement, RAPM market anchor, cross-league priors).
3. **State** — persistent caches are a global singleton: cross-fold contamination,
   *production* contamination (a dev's `data/cache/` leaks in — and the backtest could
   **overwrite the user's real Kalman state**), non-reentrancy.

**Headline guard — metamorphic future-poisoning (mechanism-agnostic).** To forecast week W,
run twice with two different random futures for rows `week ≥ W`; assert the week-W output is
**bit-identical**. Any dependence on future *content* — row read, global fit, or contaminated
cache — surfaces as a difference. Covers all three axes. Runs on small synth as a hard gate +
sampled on real data.

**Choke point — as-of accessor + injectable cache namespace.** Every stage gets inputs via
`AsOf(season, week, mode)` → `slice_plays/market/pool/interventions`. **Concrete refactor
(recommended regardless): make `_ACCUM_PATH`/`_KALMAN_PATH` injectable parameters, not module
constants** — required for fold isolation and to remove the production-overwrite hazard.
Extend the accumulator's `last_week` into a **temporal watermark**; assert `≤ W-1` after each
fold. Test-mode frames wrapped in a **tripwire proxy** for stack-trace localization. Division
of labor: *filter for correctness, poison to verify completeness, tripwire to localize.*

**Scope-leaks, each with a poison check:** V(s) frozen to pre-period or refit ≤W-1 (poison
future drive outcomes → dV invariant); VOR replacement from as-of pool (poison future pool →
VOR invariant); RAPM `market_strength` must be the line **as published before W**, not final;
cross-league `estimate_equivalency` frozen to prior seasons (poison rookie's future NFL weeks
→ prior invariant).

**Three GRID-specific leaks generic lore misses:**
- **Intervention foreknowledge** — the Kalman spikes `d` at known regime weeks; handing a
  week-5 forecast the full-season intervention set tells it about a week-9 injury. Filter
  interventions to those **known as of W**; check coaching-seed dates. Leaks *confidence*, a
  calibration leak.
- **Information-time ≠ week index** — the cutoff is "available **before kickoff** of W."
  Outcomes are post-game; injury reports / depth charts / Vegas lines are pre-game and *are*
  usable for W. Design the accessor with an **information-timestamp** per source now (free);
  retrofitting later isn't.
- **Player-universe leakage** — initializing design columns / Kalman roster from the
  full-season rosters reveals later call-ups. Build the universe as-of W.

**Two-path equivalence (not free — depends on §10 adapter).** `weekly_update.py` walks forward
and is leakage-safe by construction (modulo cache isolation) — but it **cannot run on real data
today** (it `KeyError`s on the missing `plays` contract and a broad try/except silently no-ops).
Once the §10 nflverse→GRID adapter exists, assert the offline backtest's week-W forecast equals
what `weekly_update` produces after weeks 1..W-1 — validating the harness against the production
path and doubling as a `weekly_update` regression. Until then this guard is blocked, not free.

**Anti-rot:** every guard ships with a **deliberate-leak fixture it must catch** ("test the
test"); all run on the fast synth fixture so they're hard gates, not opt-in.

---

## 9. KPI targets per tier

Principles: gate on **relative skill** for real data; every real-data target carries a
**confidence qualifier** ("skill > 0" = 95% CI excludes 0 given the player-week sample).
Per the locked decision, real-data rows are **report-only first**, then promoted to gates
against the observed noise floor.

### North star — two headlines (one per lever)
- **H1 (bench-depth / ROS):** GRID beats both season-to-date-mean and last-season-actuals on
  **ROS skill** for RB/WR/TE, with 80% ROS PICP within ±5pp, and injury-return + rookie-prior
  mechanisms each reducing error where intended. *(Kill criterion: the beat-last-season half.)*
  **Note: beating last-season-actuals is a *floor*, not proof of value** — it's a weak fantasy
  baseline, so clearing it means "not broken." The real value signal is beating
  **season-to-date-mean and market**; treat those, not last-season, as the bar for "GRID is good."
- **H2 (injury-replacement / weekly):** GRID's weekly lineup beats a last-season-set lineup on
  **realized weekly point margin** (CI excl. 0), and the effect holds — ideally strengthens —
  on the starter-OUT subset.

### Tier 0 — Recovery (deterministic, hard gates)
| KPI | Metric | Target | Gate |
|---|---|---|---|
| Attribution (pooled) | corr(rating, ability) | ≥ 0.85 | hard |
| Attribution (per pos) | corr per QB/RB/WR/TE/DEF | ≥ 0.70 each | hard* |
| Team strength | corr(team_rating, planted) | ≥ 0.90 | hard |
| Kalman trajectory | corr(total_filt, true signal) | ≥ 0.80 | hard |
| Injury detection | dip at planted weeks + recovery | boolean | hard |
| Prior equivalency | OOS R² of feeder→NFL | ≥ 0.50 | hard* |
| Filter consistency | NIS mean(z²) on synth | ∈ [0.8, 1.25] | hard |

\* calibrate from first run; set provisionally, tighten once the synth baseline is seen.

### Tier 0.5 — Golden master + determinism (deterministic, hard gates)
| KPI | Metric | Target | Gate |
|---|---|---|---|
| Determinism (in-process) | max abs diff, two runs (OMP=1) | < 1e-9 | hard *(in-process only; NOT a cross-platform gate — see below)* |
| Semantic invariants (A) | sign/label/ordering | exact | hard |
| Rank/tier stability (B) | argsort + tier match | exact | hard |
| Numeric drift (C) | allclose rtol/atol | 1e-5 / 1e-6 | hard |
| PIT uniformity (synth) | shape vs uniform | within tol | hard |

### Tier 1 — Real walk-forward (report → promoted gates), per position × {weekly, ROS}
**Accuracy (report/context):** weekly RMSE PPR ≈ QB 6–7, RB/WR 7–9, TE 5–6; ROS MAE within
~15% of realized. Context only, not gated.

**Skill vs baselines — the gates:**
| KPI | Metric | Target | Gate |
|---|---|---|---|
| Beat persistence | skill vs last-week | > 0.10, CI excl. 0 | hard (low bar) |
| Beat season-mean | skill vs season-to-date mean | > 0, CI excl. 0 | hard (real bar) |
| **Beat last-season-actuals** | skill vs `compute_valuations` | **> 0, CI excl. 0** | **hard — kill criterion** |
| Beat market | skill vs ADP/ESPN | > 0 | stretch/report |

**Ranking:** Spearman ≥ 0.55 (RB/WR) / 0.65 (QB/TE); top-N hit ≥ 60%; tier accuracy ≥ 70%
(directional).

**Decision value (Headline H2 — weekly lineup point margin):** identical roster + slot-filling
rule for both methods; only the projection source differs (GRID as-of weekly forecast vs
last-season weekly average). Realized margin = Σ(GRID lineup actual) − Σ(baseline lineup
actual), bootstrapped over rosters × weeks (margin is TD-variance-heavy → needs many samples).
| KPI | Metric | Target | Gate |
|---|---|---|---|
| **Weekly lineup margin (overall)** | mean realized per-week point margin | **> 0, CI excl. 0** | **hard — headline H2** |
| Weekly lineup margin (starter-OUT subset) | mean margin in injury-replacement weeks | > 0; ≥ overall | directional |
| Lineup win rate | fraction of weeks GRID lineup ≥ baseline lineup | ≥ 55% | directional |

**Calibration:**
| KPI | Metric | Target | Gate |
|---|---|---|---|
| ROS coverage | PICP @ 80% | within ±5pp | hard |
| ROS coverage | PICP @ 50% | within ±7pp | directional |
| Weekly tails | pinball @ 20th/80th | within ±7pp | directional |
| Sharpness | PINAW @ equal coverage | ≤ baseline | directional |
| Prob. skill | CRPS skill vs season-mean±sd | > 0, CI excl. 0 | hard |

**Regime mechanisms:**
| KPI | Metric | Target | Gate |
|---|---|---|---|
| Injury-return | RMSE first game back, rust on vs off | ≥ 10% reduction | directional |
| Injury-return calib | PICP in return weeks | within ±10pp | hard-ish |
| Changepoint detect | precision / recall vs seed | ≥ 0.6 / ≥ 0.5 | directional |
| Changepoint noise | false-positive rate, stable players | ≤ 10% | hard |
| Rookie priors | wk 1–4 error vs naive; converge by ~wk 8 | beat naive | directional |

### Tier 2 — Attribution/matchup (synth gates now; real historical via FTN participation)
| KPI | Metric | Target | Gate |
|---|---|---|---|
| Situation RAPM recovery | corr to planted situational ability (synth) | ≥ 0.65 | hard (synth) |
| Matchup-grade semantics | sign vs planted defense strength | exact | hard (synth) |
| Matchup predictive (real) | corr(grade, realized pts allowed) | > 0, CI excl. 0 | real, once `load_participation` lands (Phase 1) |

---

## 10. Real-data ingest + the nflverse→GRID adapter (unblocks Tier 1)

**Correction (post-verification):** only the **baseline** path is a thin wrapper. The
**GRID-engine** path needs a substantial new adapter — this was previously mis-scoped.

- **Baseline path (works today):** `nflverse_loader.load_pbp/load_rosters` → `data_pipeline`
  (→ `player_stats`) → `compute_valuations` (→ `valuations`). A thin wrapper:
  `python -m backend.pipeline.data_pipeline --years 2019..2025` → `compute_valuations`.
- **GRID `plays`-contract adapter (NEW, L–XL — the real work):** `nflverse_loader._normalize_pbp`
  emits a stats-aggregation contract with **none** of `drive_points, terminal, terminal_value,
  n_down/n_ydstogo/n_yardline_100, off_players, def_players, drive_id`. So `fit_value_model`,
  `compute_dv`, and `build_design` all `KeyError` on real data today. The adapter must do **drive
  segmentation, next-state derivation, next-score (`drive_points`) labeling, and the
  participation join** (`pbp_participation`, §15) — logic that exists only in `synth.py`. This is
  the gating dependency for *all* of Tier 1/2 on real data.
- (c) freeze the pulled Parquet as the Tier-1/Tier-2 data snapshot for reproducibility; keep the
  raw nflverse parquet cached so cache rebuilds are offline.

## 11. Proposed file layout
```
backend/validation/
  asof.py          # AsOf accessor + injectable cache namespace (leakage choke point)
  backtest.py      # walk-forward driver (rolling origin)
  metrics.py       # MAE/RMSE/bias, skill, CRPS/PIT/coverage/pinball/NIS, NDCG/top-N
  baselines.py     # persistence, season-mean, last-season, market
  lineup_sim.py    # roster sampling + slot-filling rule + realized-margin scoring (H2)
  report.py        # per-position tables + calibration plots → artifact
tests/validation/
  test_leakage_guards.py        # poisoning + tripwire + watermark + two-path (each w/ canary)
  test_recovery.py              # Tier 0 (hard)
  test_golden_master.py         # Tier 0.5 (hard)
  test_calibration_synth.py     # NIS/PIT on synth (hard)
  test_backtest_skill.py        # Tier 1 accuracy/skill/calibration (report → promoted gates)
  test_lineup_margin.py         # Tier 1 H2 decision value (report → promoted gate)
  test_attribution_backtest.py  # Tier 2 (synth gate; real once load_participation lands)
```

## 11.5 Compute budget (see roadmap §7.5)

Two budgets, kept separate: **CI gates** (Tier 0/0.5 + synth calibration + leakage canaries) run
on small synth, deterministic, target **< ~60s** — these are the per-PR "runs many times" surface.
The **real-data weekly walk-forward** (Tier 1/2, H1/H2, calibration) is a **dev/nightly job, not
per-PR**, target **< ~10 min**, achieved by **incremental RAPM accumulators + a frozen pre-period
V(s)** (avoids per-origin GBM refits) + sampled H2 bootstrap with logged caps. Naive
refit-per-origin would be hours — so this is a **design requirement on the `backtest.py` driver**,
not just a target. All numbers provisional-until-measured.

## 12. Build order (once approved)
1. **nflverse→GRID `plays`-contract adapter (L–XL, §10)** + baseline ingest + frozen snapshot —
   the gating dependency for all real-data Tier 1/2; the real long pole, not a thin wrapper.
2. Engine prerequisites: surface predictive `(mean, S)` from the Kalman **+ fix
   `_write_kalman_trajectory`**; make cache paths injectable (incl. `KalmanState`); pin
   determinism (seeds, OMP, versions).
3. Tier 0 + 0.5 (recovery + golden master + synth calibration) — hard gates, no data dep.
4. `asof.py` + leakage guards (poisoning headline) — the foundation everything trusts.
5. `baselines.py` + `metrics.py`.
6. Tier 1 weekly + ROS backtest → report; run once; promote directional KPIs to gates (H1).
7. `lineup_sim.py` + H2 weekly lineup-margin backtest → report; promote to gate (H2).
8. Tier 2: synth scaffold **plus** `load_participation` (Phase 1) → real historical RAPM/
   matchup validation.

## 13. Sign-off status
- **Resolved & approved:** seasons 2019–2025; engine prerequisites (predictive `S` + the
  `_write_kalman_trajectory` variance fix; injectable cache paths incl. `KalmanState`); H2 =
  weekly lineup point margin (decision value); H2 roster source = **VOR-drafted synthetic
  rosters** for now; **per-position weekly Layer-1 credit + SV→points map are in alpha scope**
  (Phase 2b) so H2 covers RB/WR/TE.
- **Deferred to first run:** confirm provisional Tier 0 thresholds (0.70 per-pos, 0.50
  equivalency) once the synth baseline is observed.

## 14. Future work (post-lock, tracked)
- **Real rosters for H2 (draft-day → week-1 update).** Replace synthetic VOR rosters with
  *actual* post-draft league rosters synced via `sync_leagues`, and re-run the weekly
  lineup-margin headline against the lineups managers really fielded. Scheduled as a discrete
  update in the window between draft day and Week 1, when post-draft rosters first exist.
- ~~SV→fantasy-points map to extend weekly calibration beyond QB~~ — **moved into alpha scope**
  (Phase 2b) along with per-position weekly Layer-1 credit, since H2 needs weekly RB/WR/TE
  projections. No longer future work.
- **Live in-season RAPM** would require *weekly* participation, which the free FTN/nflverse
  feed does not provide (it drops once, after the postseason). For alpha, in-season stays on
  the Kalman-on-points path. **Research item (tracked):** investigate other ways to obtain
  per-play participation on a **weekly** in-season cadence — candidates to evaluate on
  cost/latency/licensing/coverage: ESPN/NFL GameCenter or play-by-play feeds, NGS endpoints,
  paid feeds (SportsRadar, Genius Sports, SIS/TruMedia, PFF), community/scraped sources, or
  deriving approximate lineups from snap-count + depth-chart data. Goal: a weekly participation
  source that would let RAPM update in-season, not just yearly.

## 15. Data-source finding: participation is free (corrects prior "blocked/paid" claim)

The Phase-4 plan and `data_adapters.py:53` treat per-play participation as paid/unavailable.
**Verified false for historical data (2026-06-23):** nflverse publishes
`pbp_participation_{year}.parquet` with `offense_players`/`defense_players` (per-play GSIS IDs,
11/side, 100% populated) **free for 2016–2025**. Not discontinued — it **transitioned to FTN
Data** (2023 onward, CC-BY-SA 4.0, credit "FTN Data via nflverse"). **Caveat:** the FTN feed
publishes once per year *after the postseason* and does **not** update in-season — so RAPM is
real for draft-prep/prior-season priors and validation, but live in-season RAPM during the
current season is not available without a paid feed. Implementing `load_participation` (split
on `;`, join on game+`play_id`) unblocks RAPM/matchup/situation + Tier-2 on real history.
