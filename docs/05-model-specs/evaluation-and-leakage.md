---
model-spec-id: MS-EVALUATION-AND-LEAKAGE
status: Draft            # Draft | Approved | Superseded
statistical-owner: statistical owner (role defined in engine-spec §1); approval pending
version: 0.1.0 (2026-10-07, written under consolidation WP P0-01). This version number is also the
  evaluation-protocol version that engine-spec §7.7 requires results to record.
supersedes: none. First engine-only spec for this component. Replaces the cautious-nevermore
  retrospective-validation plan now archived (non-authoritative) under docs/07-archive/cautious-nevermore/.
---

# Model spec: evaluation and leakage (as-of information model, leakage guards, walk-forward backtest, baselines, metrics, player pool, bootstrap, diagnostics, gates)

This is a contract written before implementation. Under the authority order in engine-spec §1.5
(DR-A2, adopted subject to owner ratification) it is an authority-level-3 document. The statistical
owner approves the definitions. The implementing agent may not redefine them, and may not change a
metric, threshold or evaluation population after seeing results (alpha-spec §7.7 and §12.7,
superseded; carried into engine-spec §7.7 and §12.7).

**How to read this spec**

- **Normative words.** MUST, SHOULD and MAY are requirements on the Rust engine. Everything
  described as "the oracle" is the behaviour of the Python reference at `reference/python/`
  (ADR-012; engine-spec §1.7). Every `reference/python/...:LINE` citation uses cautious-nevermore @
  `59bce1d` line numbers, which are byte-identical in the import, **except**
  `backend/validation/lineup_sim.py`. That file carries import patch P1, which inlines
  `snake_order`; its `reference/python` line numbers are CN's plus 17 after CN line 31. This spec cites
  the `reference/python` numbers.
- **Proposed decisions.** Statements tagged "(proposed — DR-xx)" are defaults recorded in
  `docs/00-meta/decision-register.md`. The owner has not ratified them, so they are not settled.
- **Binding protocol vs estimator detail.** engine-spec §7 states the binding protocol (locks, pool,
  PB-MAE, statistical comparison, gates). This spec holds the estimator-level definitions engine-spec
  §7 delegates to it: the `scale_p` estimator, the bootstrap, the missing-provider policy, the
  as-of type, the leakage harness and the oracle's diagnostics.
- **Synthetic generators.** *Legacy synth* is the oracle's generator as imported; its defenders are
  drawn from the offense's own roster (KI-NEW-Y0; critic G-1). Every synthetic number below was
  measured on it and is labelled so. No legacy number is a Rust target.

---

## 1. Purpose and statistical intent (template: Target)

### 1.1 What the component decides

| Object | Definition | Status |
|---|---|---|
| Information state `AsOf` | What may be read to forecast `(season S, week W)` at a lock | Implemented (oracle), week-granular. Engine: lock- and publication-time-granular (§4.1) |
| Leakage harness | Guards with deliberate-leak canaries on the temporal, scope and state axes | Implemented (oracle), four guards. Engine adds publication lag, season seams, interventions, market lock (§4.2) |
| Walk-forward backtest | Rolling-origin replay producing frozen as-of forecasts | Implemented (oracle) for RAPM ratings only (§4.3) |
| Baselines | Reference forecasters GRID must beat | Implemented (oracle): persistence, season-to-date mean, last season, market (§4.4) |
| Metrics | Accuracy, probabilistic calibration, ranking | Implemented (oracle) except PB-MAE, Brier, median AE (§4.5) |
| Statistical comparison | Paired confidence intervals | Implemented (oracle) as an **iid** bootstrap. Engine: week-clustered (§4.6) |
| Evaluation population | The player-weeks scored | Oracle: paired cells with realized rows. Engine: alpha-spec §7.3 union pool (§4.7) |
| Diagnostics | Tier 1, Tier 2, H1/H2 verdict, lineup simulation, sniff, report | Implemented (oracle). Engine: diagnostics only (§4.8–§4.10) |
| Gate registry | Calibrate-then-gate thresholds, frozen once | Implemented (oracle). Engine: only for KPIs without a spec number, on a disjoint period (§4.11) |
| Promotion gates | alpha-spec §9.4, §7.8, §7.9 | **Absent** in the oracle. Kept verbatim and pre-registered (§4.12; proposed — DR-C5) |

### 1.2 Statistical intent (load-bearing comments, preserved)

- **Leakage-safe by construction, on three axes.** `reference/python/backend/validation/asof.py:4-17`:

  > "Every backtest stage gets its inputs through an :class:`AsOf` so the harness is
  > **leakage-safe by construction** instead of by spot-checking. The three leakage axes §8 names
  > ... **Temporal** (future rows reach a stage) ... **Scope** (a global fit spanning time leaks
  > into a local prediction) ... **State** (persistent caches are a global singleton)"

- **Information time, not week index.** `asof.py:19-23`: "the cutoff is "available **before
  kickoff** of W." *Outcomes* ... are known only through ``W-1``; *pre-game* signals (Vegas lines,
  injury/depth reports, a known regime change for W) are available *for* W." The engine keeps the
  split and makes the cutoff a timestamp (§4.1).
- **Filter, poison, tripwire.** The validation plan's doctrine (cn-docs §3.7): "Filter for
  correctness, poison to verify completeness, tripwire to localize." Each guard ships a canary,
  "test the test" (`tests/validation/test_leakage_guards.py:3-5`).
- **A season-aware tuple, never a week-only filter.** `asof.py:44-49`: "a week-only filter would
  admit future-season rows with low week numbers and drop prior-season rows with high week numbers
  that were already known."
- **Fast by design.** `reference/python/backend/validation/backtest.py:6-20`: V(s) is "fit **once**
  on a reserved warm-up pre-period"; the RAPM accumulators make "the incremental solve equal a
  from-scratch fit (the two-path equivalence the leakage suite asserts)."
- **Skill against references, not raw error.** `baselines.py:3-6`: "the walk-forward backtest reports
  GRID's **skill vs baselines**, not raw error — "does GRID beat the references a fantasy manager
  already has?""
- **Coverage is paired with sharpness.** `metrics.py:8-11`: "**PICP + PINAW** (coverage *paired* with
  sharpness — never coverage alone ...)".
- **One decision rule for both methods.** `lineup_sim.py:13-16`: "the single greedy start/sit rule
  used for *both* methods; only the projection fed to it may differ ... any difference in realized
  margin must come from the projections, never from the lineup mechanics."
- **Within-week correlation removes week effects.** `tier2.py:11-14`: "Correlating within each week
  ... keeps league-wide week effects — weather slates, scoring environment — from confounding a
  pooled correlation."
- **Freeze-once gates.** `thresholds.py:12-16`: "The entry is then **frozen**: later runs gate against
  it and a re-promotion attempt is a no-op — no quiet re-tuning of the bar".
- **A verdict is rendered, never acted on.** `verdict.py:12-14`: "flipping ``compute_valuations`` to
  GRID-powered by default is a deliberate follow-up gated on the measured H1 (the user's call), never
  an automatic side effect of a validation run." Engine: promotion is a separate, approved act
  (engine-spec §8.8).
- **Success is not "GRID wins".** CN roadmap (cn-docs §3.1; lessons-learned LL-28): "Defining
  success as 'GRID wins' would create pressure to defeat the leakage guards." Beating last-season
  actuals is a floor, not the bar.

### 1.3 Facts this spec relies on (verified on the oracle, 2026-10-07; §7.7)

1. **The oracle's watermark cannot be checked at a season seam.** `assert_watermark(last_week)` takes
   a week only (`asof.py:150-159`). At the seam the previous slot's week (for example 14) exceeds
   `W − 1 = 0`, so the check would always raise; the driver therefore skips it whenever the season
   changes (`backtest.py:178-181`; KI-V2). On a two-season synth, **1 of 24 origins** (the seam,
   2023 week 1) is unchecked. The defect is in the type, not just the call site.
2. **`slice_interventions` is season-blind for record lists.** The DataFrame branch is season-aware
   (`asof.py:135-137`), but the record-list branch compares the week only (`asof.py:138-147`).
   `AsOf(2023, 5)` keeps a `(2024, week 2)` record and drops a `(2022, week 17)` one. Only tests call
   it today (latent). New defect.
3. **The oracle's CIs are iid and too narrow even on the synth.** On the legacy-synth verdict, a
   week-clustered bootstrap widens the H1 ROS interval by ×1.21 and the H2 interval by ×1.09 (§7.4).
   The synth plants independent per-player-week noise. Real players share game-level shocks, so real
   intervals can widen more; no real-data figure is claimed here.
4. **The oracle's gate is a function of the run it judges.** `run_verdict` promotes each KPI's gate
   from a sign-flip null of the same run's samples and then checks that run against it
   (`verdict.py:470-476`). Freeze-once does not make it pre-registered (critic X-16).
5. **The walk-forward universe is the fixed player frame, not the as-of pool.** Columns come from
   `players`, not `AsOf.slice_pool` (`backtest.py:16-20`, `:193`). A not-yet-seen player's column has
   no rows, so its normal equation decouples and its rating is exactly 0 without changing any other
   coefficient. RAPM numbers are therefore unaffected, but every not-yet-seen player gets a rating and,
   where his position has a fitted map, a forecast, both of which look like evidence (§8 row 10).

---

## 2. Position in the engine

### 2.1 Where each piece goes

| Oracle module | Engine home (engine-spec) | Role in the engine |
|---|---|---|
| `validation/asof.py` | `domain` (as-of types) and the leakage harness in `features` / `evaluation` (§4.5, §12.3) | Binding semantics, made stricter (§4.1, §4.2) |
| `validation/backtest.py` | `evaluation` walk-forward (§7.1) | Mechanics parity source only; not the protocol |
| `validation/baselines.py` | `evaluation` baselines (§7, §9.4 naive set) | Persistence and season-to-date are parity cases; last season is a diagnostic baseline (DR-D24) |
| `validation/metrics.py` | `evaluation` metrics (§7.4, §7.5) | Class A parity on non-degenerate inputs; PB-MAE, Brier, median AE added |
| `validation/tier1.py`, `tier2.py`, `verdict.py`, `report.py`, `sniff.py` | `evaluation` diagnostics, `governance` reports (§7.14) | Diagnostics only; never gates |
| `validation/lineup_sim.py` (+ `scoring/vor.py` inside it) | `evaluation` lineup simulation (§7.14.2; proposed — DR-C11) | The start/sit decision metric |
| `validation/thresholds.py` + `provisional_thresholds.json` | `governance` registry (§7.14.4) | Only for KPIs without a spec number, on a disjoint calibration period |

### 2.2 Flow (engine)

```text
sources ──(published_at)──► AsOf(S, W, lock) ──► features / models fitted as of the lock
                                │                         │
                                │ guards: poisoning, watermark(season,week), tripwire,
                                │ publication lag, interventions, market-at-lock, universe, state
                                ▼                         ▼
                     frozen locked projection (per origin) ──► scored after official outcomes (§4.7 labels)
                                                              │
                         union pool (§4.7) ── paired errors ──┼─► PB-MAE, §7.5 metrics, calibration
                                                              ├─► week-clustered bootstrap CIs (§4.6)
                                                              ├─► §9.4 / §7.8 / §7.9 gates (pre-registered)
                                                              └─► diagnostics: H1, H2 lineup sim, Tier 2
```

### 2.3 Contracts and neighbouring specs

- engine-spec §4.5 (as-of, publication lag), §7 (protocol), §8.8 (promotion), §12.3 (leakage tests).
- `rapm-attribution.md` §2.3, §6 (walk-forward accumulators, research-only in-season RAPM, two-path).
- `state-space-kalman.md` (NIS, PIT, predictive `S`; filtered vs smoothed).
- `layer1-credit.md` (participation dependence of Layer 1; Layer-1′).
- `projection-stack.md` (the forecasts being evaluated; seed separation).
- `docs/03-contracts/engine-output-contract.md` §7 (model scorecard, benchmark results reports).

---

## 3. Inputs and outputs (template: Inputs)

### 3.1 Inputs

| Input | Oracle form | Null and failure semantics (oracle → required) | As-of rule |
|---|---|---|---|
| Plays (participation) | plays contract with `season`, `week` (`backtest.py:117-128`) | none checked → plays contract validation | Participation of season `S` only after its publication (§4.1; proposed — DR-C1) |
| Player frame | `[player_id, team, position]`, deduped at the verdict boundary (`verdict.py:383-388`) | duplicates deduped silently → typed error at ingestion | Universe as of the origin (§4.2 G8) |
| Realized points history | `[player_id, season, week, points]` (`tier1.py:206-208`) | absent rows = absent player | Official stats as labels, REG only (proposed — DR-C12) |
| Box-score history | raw `player_stats` frame (`verdict.py:364-368`) | degraded columns warned and zero-filled (`verdict.py:98-108`) → typed `LabelError` | as above |
| Points allowed | `[season, week, def_team, points_allowed]` (`tier2.py:44-46`) | duplicates rejected (`tier2.py:66-72`) | outcomes ≤ W−1 for grades, W for scoring |
| Market | static `{team: strength}` dict, or a per-week table (`asof.py:116-126`) | dict passed through unsliced | **Line published before the lock** (engine-spec §7.2; proposed — DR-B5). A static dict is not admissible |
| Interventions | records or frame with a week (`asof.py:128-147`) | undated → `LeakageError` (kept) | announced before the lock (DR-D1) |
| Availability (`out`) | `{(season, week): set(pid)}` (`verdict.py:372-373`) | none → no starter-OUT subset | observed before the lock |
| Benchmark projections | `baselines.market` wrapper (`baselines.py:123-139`), never wired | — | provider snapshot hashed at lock (engine-spec §7.6) |

### 3.2 Outputs

**Oracle.** `OriginResult{season, week, n_train_plays, ratings, def_grades}` (`backtest.py:44-58`);
`PointForecast{mean, sd?}` (`baselines.py:31-42`); `BootstrapCI{point, lower, upper, excludes_zero}`
(`metrics.py:119-131`); the Tier-1 / Tier-2 / H2 dictionaries; `run_verdict`'s results dict
(`verdict.py:377-382`); `phase2c_verdict.{json,md}` (`report.py:125-136`); the registry JSON.

**Required engine outputs** (records under `docs/03-contracts/engine-output-contract.md` §7):

| Record | Content |
|---|---|
| Locked projection set per origin | the projection records of the output contract, with lock, data snapshot, model and scoring-profile versions; immutable |
| Scorecard | PB-MAE (primary), per-position NMAE and MAE, RMSE, median AE, rank metrics, active/inactive Brier, interval coverage paired with width, bias slices; by week and position; rookie and low-evidence slice |
| Comparison | per baseline or provider: paired statistic, week-clustered CI, seed, resample count, cluster unit, imputed-cell counts |
| Gate result | each pre-registered gate item: value, threshold, pass/fail/not-evaluable, protocol version |
| Diagnostics | H1, H2 (overall and starter-OUT), Tier 2, calibration diagnostics; labelled diagnostic |
| Leakage report | guard outcomes for the run, `research_only` flag and its reason |

---

## 4. Model and equations (template: Equations, Priors, Constraints)

### 4.1 The information model (`AsOf`)

**Oracle** (`asof.py:41-165`):

```text
AsOf(season S, week W, mode ∈ {"backtest", "production"})     mode is carried, never enforced (asof.py:65-66)
outcome_cutoff = W − 1;  pregame_cutoff = W                                      (asof.py:73-81)
keep(frame, c) = (season < S) ∨ (season = S ∧ week ≤ c)    if a season column exists
               = (week ≤ c)                                otherwise              (asof.py:41-54)
slice_plays    = keep(plays, W − 1)                                              (asof.py:84-90)
slice_pool     = ∪ participants of slice_plays (list/tuple/set cells)            (asof.py:92-114)
slice_market   = dict unchanged;  DataFrame → keep(market, W)                     (asof.py:116-126)
slice_interventions = DataFrame → keep(·, W);  records → week ≤ W (no season); undated → LeakageError
                                                                                  (asof.py:128-147)
assert_watermark(last_week): raise if last_week > W − 1                          (asof.py:150-159)
```

**Required engine type (proposed — DR-C1; engine-spec §4.5).**

```text
AsOf { season S, week W, lock: Lock { kind ∈ {Thursday, Sunday, Operational}, at: UtcInstant },
       mode ∈ {Backtest, Production, Research} }

Every source record r carries an event slot (season_r, week_r) and a publication time published_at_r
(and, for interventions and coaching changes, announced_at_r).

outcome_usable(r)  ⇔ (season_r, week_r) ≤_lex (S, W − 1)  ∧  published_at_r < lock.at
pregame_usable(r)  ⇔ (season_r, week_r) ≤_lex (S, W)      ∧  published_at_r < lock.at
static metadata    ⇔ exempt from the slot rule;  published_at_r < lock.at still applies (alpha-spec §2.4)
```

- `≤_lex` is lexicographic order on `(season, week)`. A frame without a season key is a
  `ContractError`, never a week-only fallback.
- **Publication lag.** nflverse participation for season `s` is published once, after `s`'s
  postseason (`reference/python/backend/grid/nflverse_loader.py:175-178`; critic G-5). With this
  rule, no in-season origin of season `S` can read season-`S` participation, so production RAPM and
  Layer-1 credit are season-boundary products and the in-season path uses Layer-1′ (proposed —
  DR-C1; `rapm-attribution.md` §2.3; `layer1-credit.md`).
- **Historical publication times.** Backfilled history lacks recorded publication times. The
  declared approximation, for example "participation of season `s` published after `s`'s last
  postseason game", is open (DR-D5) and recorded with every
  historical run.
- **Research mode.** `mode = Research` MAY relax the publication-lag rule, for example to reproduce
  the oracle's in-season RAPM. Every output of such a run carries `research_only = true` with the
  relaxed rule named. It MUST NOT feed a gate, a promotion or a published claim (alpha-spec §12.3:
  "current-season participation unavailable at serve time must not appear in promoted live
  features").
- **Lock granularity.** The engine's cutoff is the lock timestamp. Where week granularity and lock
  granularity coincide, the oracle's `AsOf` cases are parity targets; where they differ, the engine
  rule governs and the divergence is listed in `reference/python/PARITY.md` (engine-spec §4.5).

### 4.2 Leakage guards (engine-spec §12.3)

The engine ports the oracle's four guards and adds the rest. Every guard ships a canary: a
deliberately leaky input or forecaster that the guard MUST catch.

| # | Guard | Oracle | Engine requirement | Canary |
|---|---|---|---|---|
| G1 | **Metamorphic future-poisoning.** Forecast origin W twice with two different futures (rows at weeks ≥ W with their state columns permuted). Outputs MUST be bit-identical | `test_leakage_guards.py:84-92`, `_poison_future` permutes `STATE_COLS` of weeks ≥ W (`:50-63`) | Applied to every per-origin output: ratings, credit, Kalman state, projections. Poison also future-season rows and post-lock records | A forecaster fitted on all weeks differs (`test_leakage_guards.py:95-104`) |
| G2 | **Watermark.** The accumulators and states behind an origin absorbed only slots `≤ (S, W−1)` | week-only, skipped at season seams (`backtest.py:178-181`; KI-V2) | `last_slot: (season, week)` checked **at every origin**, including the first origin and every season seam | Absorbed `(S, W)` raises (`test_leakage_guards.py:118-121`); **plus a seam canary**: absorbing `(S, 1)` before origin `(S, 1)` raises |
| G3 | **Tripwire.** A frame carrying a future row raises at the read site, including `len()` | `TripwireFrame` re-checks on every read (`asof.py:216-267`) | Same; a season key is mandatory | A row injected after wrapping trips on read (`test_leakage_guards.py:135-144`) |
| G4 | **Two-path equivalence.** The offline walk-forward at origin W equals the production incremental path after weeks 1..W−1, with last slot `W−1` and the same persisted column order | `rtol = atol = 1e-6` (`test_leakage_guards.py:150-197`); observed 0.0 (`rapm-attribution.md` §6.2) | Rust-internal: the `evaluation` walk-forward equals the weekly pipeline update, ≤ 1e-12 abs (`rapm-attribution.md` P-6) for RAPM blocks and Class A for Kalman states | — (equivalence test) |
| G5 | **Publication lag** (new) | absent | Injecting season-`S` participation with `published_at ≥ lock` leaves every non-research output bit-identical | The same injection with `mode = Research` changes outputs and sets `research_only` |
| G6 | **Interventions** | undated raise; record list season-blind (§1.3 item 2) | Dated by `announced_at`; season-aware; only those announced before the lock apply | A future-season low-week record is rejected |
| G7 | **Market at lock** | static dict passes through (`asof.py:124-126`); the synth anchor is planted truth plus noise | Only lines published before the lock; a closing line after the lock is post-lock data (engine-spec §7.2; proposed — DR-B5; DR-D2) | A post-lock line injected into a locked run is rejected |
| G8 | **Player universe** | fixed frame (§1.3 item 5); `slice_pool` exists but is used only by tests, and fails on `ndarray` cells (KI-V8) | Universe as of the origin; not-yet-seen players are `no_data`, never rated or forecast as evidence | A player first seen at `W + 1` appears in no origin-`W` output |
| G9 | **State namespace** | `CacheNamespace` refuses roots overlapping a cwd-relative `data/cache` (`asof.py:171-210`) | Backtests and tests write only under an isolated state root, checked against the **configured** production root, before any write | Pointing the namespace at production raises (`test_asof.py:116-119`) |
| G10 | **Scope** | V(s) frozen on the warm-up (`backtest.py:165-168`) | V(s), priors, equivalency and VOR replacement fit before W; calibration periods for gates disjoint from evaluation (§4.11) | A V(s) refit on the whole season (KI-NEW-W4) is detected by G1 |
| G11 | **Corrections and labels** | absent; postseason rows enter (KI-NEW-V0a, KI-NEW-I4) | Stat corrections create a new data version; postseason rows never enter REG labels (proposed — DR-C12) | A corrected stat replayed into an earlier snapshot changes its data version |
| G12 | **Cross-fit folds** | play-level `KFold(shuffle)` (KI-NEW-A5) | Folds hold out whole weeks; no input to a held-out fold is fitted on it (`layer1-credit.md`; DR-D14) | — |
| G13 | **Kalman initialisation** | `x0 = nanmean(y[:3])` (KI-#15) | No observation from the forecast week or later initialises any state (proposed — DR-C10) | Changing week-2 observations leaves the week-1 filtered value unchanged |

### 4.3 Walk-forward backtest (`backtest.walk_forward`, `backtest.py:117-201`)

**Oracle algorithm.**

```text
slots          = distinct (season, week) sorted                                   (backtest.py:77-81)
require warmup_weeks > 0, origin_stride > 0, |slots| > warmup_weeks             (backtest.py:147-156)
origins        = slots[warmup:], every origin_stride-th, the subset logged       (backtest.py:158-163)
V(s)           = fit_value_model(plays in slots[:warmup])   frozen                (backtest.py:165-168)
for (s, w) in slots:
    if (s, w) ∈ origins and accumulators exist:
        if last_slot.season = s: assert_watermark(last_slot.week)                 (backtest.py:179-181)
        β = solve_rapm(XtX, Xty, λ=120, w_mkt=40, market static)                  (backtest.py:182-183)
        record ratings {pid: β_p}, def_grades {t: −β[t_def]}                     (backtest.py:184-189)
    X, y = build_design(attach_dv(plays at (s, w), V), players)  accumulate        (backtest.py:191-199)
```

- `solve_rapm` skips teams absent from the market ("no false zero prior", `backtest.py:105-106`).
  The market is applied unchanged at every origin (`backtest.py:136-141`).
- Accumulation runs across seasons with no window (KI-V2; DR-C6).
- Incremental equals from scratch: observed max abs diff 4.8e-15 against an asserted 1e-8
  (`tests/validation/test_backtest.py:67-92`; `rapm-attribution.md` §6.2).

**Required protocol** (engine-spec §7.1, proposed — DR-C1, DR-C6):

1. For each target week, reconstruct the snapshot available before the lock (§4.1).
2. Fit or update only on the engine-spec §2.4 window: `S[1..W−1]` plus `S−1, S−2`; preseason and
   week 1: `S−1..S−3`. RAPM uses exact per-season block sums (`rapm-attribution.md` §6.4).
3. Generate and **freeze** the locked projection set (§3.2). An operational re-projection is a new
   version and never replaces a locked one.
4. Score after official outcomes and stat corrections are available, with labels from engine-spec
   §4.7.
5. Persist player-level errors, weekly metrics and model metadata.
6. Backtests of the live path hide current-season participation (G5). Oracle-style in-season RAPM is
   `research_only`.
7. `origin_stride > 1` is a development mode; its results are labelled `subset` and never gate.

### 4.4 Baselines (`baselines.py`)

**Oracle.** All forecasts are fantasy points per game, from a realized-points history:

```text
persistence(i)   = mean of i's points in his latest current-season week ≤ W−1   (baselines.py:51-70)
                   absent if i has no current-season game before W; empty at W = 1
season_to_date(i)= mean of i's current-season points, weeks ≤ W−1               (baselines.py:73-102)
   sd_i          = sample sd (ddof 1) if ≥ 2 games; otherwise, or if sd_i ≤ 0 or non-finite,
                   pooled = sd (ddof 1) of the per-player means across players, floored to 1e-6
last_season(i)   = mean of i's points in season S − 1                            (baselines.py:105-120)
market(i)        = external projection, passed through (report only)            (baselines.py:123-139)
```

- `default_baselines()` excludes market (`tier1.py:51-64`); it was never wired.
- The pooled fallback spread is the *between-player* dispersion of means, used as a per-player
  predictive spread (`baselines.py:96-101`). It is a convenience for CRPS, not a predictive model.
- An empty history returns `sd = {}` rather than `None` (KI-V11).

**Required baseline set.**

| Baseline | Definition (per player-week, in points under the evaluation profile) | Role |
|---|---|---|
| Prior-game fantasy points | = oracle `persistence` | §9.4 naive set; parity case |
| Rolling three-game average | mean of the player's last three active games before the lock, within the §2.4 window | §9.4 naive set (no oracle source) |
| Season-to-date average | = oracle `season_to_date` mean | §9.4 naive set; parity case |
| Position/depth-chart median | median of the player's position × depth-chart-slot group's points over the training window | §9.4 naive set (no oracle source) |
| Last-season per-game actuals | = oracle `last_season`, REG only | Diagnostic baseline on every scorecard; gate membership open (DR-D24) |
| Market / providers | provider snapshot at lock | Benchmark only (engine-spec §7.6); never a model input |

Every baseline MUST yield a value for every pooled player-week. A baseline without a value for a
player (no history) uses the position/depth-chart median as its declared fallback, and the scorecard
reports the fallback count per baseline (proposed — DR-C5).

### 4.5 Metrics (`metrics.py`)

**Oracle closed forms** (numpy only; `metrics.py:34-331`). With `e = pred − actual`:

```text
mae  = mean |e|;   rmse = sqrt(mean e²);   bias = mean e                        (metrics.py:86-101)
skill(m, r) = 1 − m / r;  0.0 if r = 0                                          (metrics.py:104-113)
Φ(z) = ½ (1 + erf(z/√2))   (math.erf, element-wise)                             (metrics.py:34-37)
Φ⁻¹(p) = Acklam rational approximation, central region 0.02425 ≤ p ≤ 0.97575,
         tails via q = sqrt(−2 ln p); raises for p ∉ (0, 1)                     (metrics.py:46-80)
z = (actual − mean)/sd;   nis = mean z² with sd = sqrt(var)                      (metrics.py:186-201)
pit = Φ(z)                                                                       (metrics.py:204-206)
crps = mean sd · [ z(2Φ(z) − 1) + 2φ(z) − 1/√π ]          (Gneiting & Raftery)   (metrics.py:209-220)
picp(level) = mean 1{ |actual − mean| ≤ Φ⁻¹((1+level)/2)·sd }   (inclusive)      (metrics.py:223-233)
pinaw(level)= mean(2 Φ⁻¹((1+level)/2) sd) / (max actual − min actual);  0.0 if range 0 (metrics.py:236-251)
pinball(q)  = mean max(q·d, (q−1)·d),   d = actual − quantile_pred              (metrics.py:254-262)
spearman    = Pearson correlation of average ranks (ties share the mean rank);
              NaN if n < 2; 0.0 if either rank vector has zero variance           (metrics.py:268-299)
top_n_hit   = |top_n(pred) ∩ top_n(actual)| / n                                  (metrics.py:302-310)
ndcg@k      = Σ_{r≤k} actual[order_pred(r)]/log2(r+1) ÷ same for the ideal order (metrics.py:313-331)
```

**Primary metric: PB-MAE** (alpha-spec §7.4, kept verbatim in engine-spec §7.4). The oracle has no
PB-MAE.

```text
MAE_p    = mean over the position-p pool cells of |projected_points − actual_points|
NMAE_p   = MAE_p / scale_p
PB-MAE   = mean(NMAE_QB, NMAE_RB, NMAE_WR, NMAE_TE)
improvement_j = (PB-MAE_j − PB-MAE_engine) / PB-MAE_j          lower PB-MAE is better
```

- `projected_points` is the **unconditional** expected points of the locked projection under the
  evaluation scoring profile, Half-PPR by default (engine-spec §2.3; output contract §3.4).
- **`scale_p` estimator (proposed — DR-C5).** engine-spec §7.4 requires a "fixed position scale
  estimated only from the training period" and delegates the statistic here. Proposed:

  ```text
  T        = the training period: every REG week of the seasons before the first evaluated season,
             within the data available to the protocol version; disjoint from every evaluation week
  A_{p,w}  = actual points of the top N_p players by actual points at position p in week w ∈ T
             (N_QB = 20, N_RB = 40, N_WR = 50, N_TE = 15; bye players excluded)
  scale_p  = mean over (w, i) of | a_{p,w,i} − mean(A_{p,w}) |      (mean absolute deviation within week)
  ```

  It depends on outcomes only, so it cannot be moved by any forecaster or provider, and it removes
  week-level scoring shifts. The alternative, the MAE of a declared naive baseline on `T`, makes
  `NMAE` a relative MAE but ties the scale to a baseline definition. `scale_p`, `T` and the protocol
  version are frozen before any PB-MAE result is recorded; a change is a new protocol version.
- **Strongest naive baseline** = the §4.4 naive baseline with the lowest overall PB-MAE on the
  evaluation period (proposed — DR-C5).

**Other secondary metrics with no oracle source** (engine-spec §7.5): median absolute error; start/sit
accuracy at positional starter cutoffs; the ranking Accuracy Gap; active/inactive Brier score
`mean (p_active − 1{active})²`; quantile calibration error; bias slices (team, favourite/underdog,
home/away, rookie status, injury state); weekly win rate per provider. Their definitions are
spec-defined goldens (engine-spec §12.2); thresholds that use them follow §4.12.

### 4.6 Statistical comparison: the bootstrap

**Oracle** (`metrics.py:134-180`):

```text
rng   = numpy default_rng(seed)
idx   = rng.integers(0, N, size = (n, N))        every cell resampled independently (iid)
boot_b = statistic(samples[idx_b])                mean by default
CI    = [quantile_{α/2}(boot), quantile_{1−α/2}(boot)]   numpy "linear" quantile (type 7)
excludes_zero = lower > 0 or upper < 0;   n = 10,000, α = 0.05, seed = 0 by default
```

Every oracle CI (Tier-1 margins, H2 margins, Tier-2 weekly values) uses it. Cells share realized weeks
and, for ROS, overlapping windows, so the intervals are too narrow (KI-NEW-V1).

**Required: paired, week-clustered bootstrap** (engine-spec §7.7; critic §3.4 item 7):

```text
cells   c ∈ C, each with a cluster key k(c) = (season, week) of its target week and paired values
        (for example |e_A(c)|, |e_B(c)| for the same player-week under two forecasters)
K       = the distinct clusters, in (season, week) order
for b = 1..n:
    draw K cluster indices with replacement from K (the injected or seeded index stream)
    C_b   = the cells of the drawn clusters, with multiplicity
    T_b   = T(C_b)                                   the full statistic recomputed on C_b
CI      = [Q_{α/2}(T_1..T_n), Q_{1−α/2}(T_1..T_n)],   Q = linear-interpolation quantile (type 7)
excludes_zero = lower > 0 ∨ upper < 0
```

- `T` is the full statistic, recomputed per resample: for a gate on PB-MAE, `T_b` is the PB-MAE
  difference or `improvement_j` on `C_b`, with `scale_p` held fixed. Per-position results resample
  the same clusters.
- **Optional second stage** (alpha-spec §7.7): resample games within each drawn week. If used, it
  is part of the protocol version.
- **ROS diagnostics.** Rest-of-season targets of adjacent origins overlap, so week clusters of
  origins are still dependent. The ROS clustering scheme (for example a moving-block bootstrap over
  origin weeks with block length equal to the horizon) is open (proposed — DR-C5). Until it is
  fixed, ROS intervals are reported as diagnostics only.
- Recorded with every interval: `n`, `α`, the seed, the cluster unit, `K`, and the resampling
  scheme. Evaluation seeds are separate from model seeds (`projection-stack.md` §5.2).

### 4.7 Evaluation population: the player pool

**Oracle (Tier 1).** A cell exists only when the player has a GRID forecast **and** a realized row
at that origin (`tier1.py:258-260`). Players inactive after the forecast are dropped (survivorship),
and the ROS target is the mean over the weeks the player actually has rows for (`tier1.py:183-187`),
so missed weeks are dropped too. Skill is computed on "paired cells" with each baseline
(`tier1.py:278-286`), so each baseline's sample is different.

**Required: the alpha-spec §7.3 union pool** (engine-spec §7.3; critic §3.4 item 6):

```text
for each position p and target week w:
  P_{p,w} = top N_p by the engine's locked projection
          ∪ top N_p by each benchmark provider j
          ∪ top N_p by actual fantasy points,            N_QB = 20, N_RB = 40, N_WR = 50, N_TE = 15
  minus players on a bye;  ties at rank N_p are all included (proposed — DR-C5)
  a pooled player inactive after the lock has actual = 0
  every forecaster (engine, baselines, providers) is scored on the same P_{p,w}
```

- **Missing provider projections (DR-D23; proposed default).** A
  pooled player-week without a projection from provider `j` receives provider `j`'s **lowest
  published projection at that position and week** (a provider-tail estimate). The rule is identical
  for every provider, and each comparison reports the imputed-cell count per provider. No §7.8 or §7.9
  result is admissible until the owner ratifies the policy.
- **The engine has no missing projections.** It projects every eligible player. A pooled player-week
  without an engine projection is a typed `EvaluationError::MissingEngineProjection`, never
  imputed.
- **Week 18** is scored separately from weeks 1–17 (engine-spec §2.2).
- The oracle Tier-1 pairing is a diagnostic only (§4.8).

### 4.8 Tier 1 (oracle diagnostic; `tier1.py`)

```text
forecast_weekly_points: rating → sv_map.predict(position, rating) per origin;
   unmapped or unknown positions omitted                                           (tier1.py:67-95)
forecast_ros_points: StatLineModel(volume_asof, {rapm_rating, smoothed_talent}) scored with the
   given profile, per game (games = 1); no model, no volume or unknown position → omitted;
   a present-but-empty model gives a genuine 0.0                                    (tier1.py:98-171)
realized: weekly = week-W points;  ros = mean of points over weeks ≥ W of the season (tier1.py:174-187)
for each origin, in order:
   for each forecast cell with a realized value:
      calibration first: sd_pos = sample sd (ddof 1) of that position's signed errors from strictly
         earlier origins, used only if ≥ min_sd_history = 20 errors                  (tier1.py:264-276)
      paired margins vs each baseline: |base − actual| − |forecast − actual|          (tier1.py:278-286)
   fold this origin's errors into the pools afterwards                             (tier1.py:289-290)
report per position and ALL: n_cells, MAE; per baseline n_pairs, MAEs, skill = 1 − MAE_grid/MAE_base,
   margin = bootstrap_ci(margins); calibration {picp, pinaw, crps, nis(var = sd²)} at level 0.80
                                                                                   (tier1.py:292-318)
```

- The spread is the SD of past *signed* errors around their mean, so a biased forecaster's Gaussian
  `N(forecast, sd)` omits its bias (`tier1.py:267`).
- The `ALL` bucket pools every position, including non-skill ones (DB, P rows appeared in the real
  run; `docs/07-archive/cautious-nevermore/real-data-results.md` §3.3).

### 4.9 Tier 2 (oracle diagnostic; `tier2.py:31-90`)

```text
join grades [season, week, def_team, rapm_grade] with allowed [season, week, def_team, points_allowed]
   (inner join; duplicates on the key rejected; min_teams ≥ 2 enforced)
weekly_k = −spearman(grade, points_allowed) for each week with ≥ min_teams (6) joined defences
predictive = bootstrap_ci(weekly values);   no qualifying week → ValueError
```

- With a correct grade (`+E_def`, higher = tougher; proposed — DR-B5) a real signal gives a positive
  KPI. The oracle's grades are `−β_def` (KI-NEW-A2), so on real data a real signal would come out
  negative.
- **The synth verdict test is tautological.** It builds `points_allowed = 20 − 30·grade` from the
  verdict's own grades (`tests/validation/test_verdict.py:46-51`; critic X-3), so `predictive > 0.9`
  is guaranteed. Tier 2 has never run on engine-produced grades against independent outcomes
  (KI-NEW-V2). It stays a diagnostic until a points-allowed feed exists and the grade follows DR-B5
  (engine-spec §7.14.4).

### 4.10 H1, H2, the lineup simulation and the verdict (oracle diagnostics)

**H1** (`verdict.py:448-459`): the ALL-row ROS paired margin against last season.
`SHIP_GRID` if `point > 0` and `excludes_zero`; `BASELINE_FALLBACK` otherwise; `INCONCLUSIVE` without
paired cells. In the engine H1 is a diagnostic, not a kill criterion (proposed — DR-C4, DR-C5;
engine-spec §7.14.1).

**H2 lineup simulation** (`lineup_sim.py`; proposed — DR-C11: kept as the start/sit decision metric,
VOR internal):

```text
roster construction (shared, method-neutral):
  pool  = calculate_vor(pre-first-origin per-game means, roster_slots, num_teams), ordered by VOR rank
  order = snake_order(num_teams, num_rounds)                                       (lineup_sim.py:37-52)
  each pick: the highest-ranked available player filling a dedicated need, else a FLEX need for
             RB/WR/TE; with no open need (bench) or nothing fitting: best available (lineup_sim.py:88-109)
set_lineup(roster, proj, slots, out): rank available players by (−proj (missing = 0.0), player_id);
  fill dedicated slots, then FLEX for RB/WR/TE                                     (lineup_sim.py:117-152)
for roster, for week in sorted(actual):
  margin = Σ actual(lineup_A) − Σ actual(lineup_B);  a player without an actual row scores 0
  win = 1 if margin > 0, ½ if margin = 0, 0 otherwise
  starter-OUT cell: some player B would start with everyone available is OUT that week
overall = bootstrap_ci(margins);  starter_out = bootstrap_ci(subset) or None    (lineup_sim.py:178-236)
```

VOR inside roster construction (`reference/python/backend/scoring/vor.py:4-119`):

```text
replacement_p  = projected points at 0-based index slots_p · num_teams of position p, sorted descending
                 (rank slots_p·num_teams + 1; KI-NEW-C2); the last player if fewer; 0.0 if none   (vor.py:31-45)
FLEX: demand = (Σ_{RB,WR,TE} slots + flex) · num_teams; flex_repl at that index of the pooled RB/WR/TE list;
      replacement_p ← max(replacement_p, flex_repl) for eligible positions with a slot;
      FLEX-only positions get flex_repl (CN #85, KI-A6 fixed)                                    (vor.py:47-71)
vor_i = projected_i − replacement_{pos(i)};  0.0 for positions with no replacement level          (vor.py:74-82)
rank by vor descending; tiers: new tier when (prev − cur)/(|prev| or 1) > max_gap_pct = 0.10      (vor.py:85-119)
```

The tier fragmentation near zero VOR (KI-A5) does not affect H2, which uses rank only. The oracle's
H2 configuration is Standard scoring, 8 teams × 8 rounds, slots QB 1 / RB 2 / WR 2 / TE 1 / FLEX 1
(`verdict.py:68`, `:348-349`), 4 warm-up weeks, `n = 10,000`, `α = 0.05`, seed 0. It is a parity
configuration only; the engine's configuration is part of the protocol version and its default
profile is Half-PPR (engine-spec §7.14.2).

**Engine H2.** Method A is the engine's locked weekly projection; method B a declared baseline. The
oracle's method A is the SV→points map of the as-of RAPM rating (KI-NEW-R3), which uses
current-season participation (proposed — DR-C1); neither is reproduced. Intervals are week-clustered.

**Sniff test** (`sniff.py:47-89`): at least `min_in_top_k = 3` of a curated elite set in the top
`top_k = 5`, and none ranked worse than `max_rank = 24`; missing elites count against the top-k bar;
it never raises on a band miss. Kept as a smoke check on ranking outputs, never a gate.

**Report** (`report.py:125-136`): strict JSON (`allow_nan = False`), so a NaN fails the write. The
engine's reports follow the output contract §7 and keep the "fail on non-finite" rule.

### 4.11 Calibrate-then-gate registry (`thresholds.py`)

```text
promote(name, baseline_samples, z = 1.959964):
   if name frozen and not force: return copy of the frozen entry                 (thresholds.py:81-83)
   n ≥ 2 else ValueError                                                           (thresholds.py:86-89)
   gate = mean + z · sd(ddof 1)/√n;  entry {gate, baseline_mean, baseline_se, n, z, run_label}
gate_check(name, value) = (value > gate, gate), or (None, None) if never promoted (thresholds.py:104-115)
verdict: baseline_samples = observed per-unit samples × seeded random signs (sign-flip null),
         promoted and checked in the same run                                    (verdict.py:331-335, :470-476)
```

**Required (proposed — DR-C5; critic X-16; engine-spec §7.14.4).**

- Allowed only for KPIs the spec does not fix numerically (the H2 margin, the Tier-2 KPI), never for
  any §7.8, §7.9 or §9.4 threshold.
- The procedure is pre-registered in the protocol version, and the **calibration period is disjoint
  from every evaluation period it gates**. A gate promoted from the run it judges (the oracle's
  verdict) is non-conforming.
- Registry semantics are kept: one entry per KPI with gate, mean, SE, `n`, `z`, explicit run label;
  freeze-once; `force` only as a reviewed diff; pass iff strictly greater than the gate; a
  never-promoted KPI has no verdict. Each entry also records its calibration period and the protocol
  version, and frozen gates are committed in the `governance` registry.
- The oracle's committed `provisional_thresholds.json` is empty and a test enforces it
  (`tests/validation/test_thresholds.py:76`), while its docstring says frozen gates are committed
  (`thresholds.py:17-19`; cn-docs §14 item 8). The engine resolves this in favour of committed,
  versioned entries.
- The sign-flip null resamples cells iid. Any null used for a gate is week-clustered (§4.6).

### 4.12 Promotion and claim gates (alpha-spec §9.4, §7.8, §7.9; proposed — DR-C5)

The thresholds are **kept verbatim and pre-registered** (engine-spec §7.8, §7.9, §9.4). The oracle
implements none of them. This section fixes how each is evaluated.

| Gate item | Evaluation |
|---|---|
| §9.4: the promoted ensemble beats every naive baseline on overall PB-MAE | PB-MAE (§4.5) on the union pool, rolling origin over the declared evaluation seasons; point estimate lower for each §4.4 naive baseline; week-clustered CI of each difference reported |
| §9.4: beats the strongest naive baseline by at least 3% overall | `improvement = (PB-MAE_b − PB-MAE_e)/PB-MAE_b ≥ 0.03` for the strongest naive baseline (§4.5) |
| §9.4: no core position worse than the strongest naive baseline by more than 1% | per position `(NMAE_{p,e} − NMAE_{p,b})/NMAE_{p,b} ≤ 0.01` |
| §9.4: NCAA priors improve low-evidence PB-MAE or calibration without degrading veterans | the same comparison restricted to the engine-spec §2.5 low-evidence slice and its complement, ensemble with vs without the prior member; "without degrading" needs a numeric margin (open, DR-C5) |
| §9.4: 80% intervals cover 72%–88% historically | PICP at 0.80 of the Layer-F distribution, paired with PINAW, on the union pool |
| §7.8 items 1–6, §7.9 items 1–9 | as stated in engine-spec; "materially worse" (§7.8 item 4), "documented recalibration" (§7.8 item 5) and "material degradation" (§7.9 item 7) need numeric definitions before the evaluation they govern (open, DR-C5) |

- **Current standing.** CN's only recorded comparison is a tie with last season and a loss to the
  season-to-date mean (`docs/07-archive/cautious-nevermore/real-data-results.md` §3.3). The engine
  acknowledges that GRID is currently below the §9.4 gate (engine-spec §3.3; proposed — DR-C5).
- A gate item whose definition is open is reported as `not evaluable`, never as passed.

### 4.13 Constants

| Constant | Value | Defined at | Provenance / engine status |
|---|---|---|---|
| Outcome / pre-game cutoffs | `W − 1` / `W` | `asof.py:73-81` | design (CN PR #73). Engine: lock timestamp (§4.1) |
| `warmup_weeks` | 4 | `backtest.py:64`, `:121`; `verdict.py:350` | hand-set (CN PR #75) |
| `origin_stride` | 1 | `backtest.py:125` | design; subset mode logged (`backtest.py:210-215`) |
| RAPM in the walk-forward | λ 120, `w_mkt` 40, team ridge 0.05 | `backtest.py:39-41` | `rapm-attribution.md` §4.9 |
| Bootstrap | `n = 10,000`, `α = 0.05`, `seed = 0`, iid, numpy linear quantile | `metrics.py:134-180` | hand-set (CN PR #84). Engine: week-clustered, seed separate from model seeds |
| `z` for gates | 1.959964 | `thresholds.py:30` | matches the 95% convention |
| PICP / PINAW level | 0.80 | `metrics.py:223`, `:236`; `tier1.py:196` | design (validation plan §7) |
| `min_sd_history` | 20 | `tier1.py:197` | hand-set (CN PR #86) |
| Tier-2 `min_teams` | 6 | `tier2.py:35` | hand-set (CN PR #87) |
| H2 roster slots | QB 1, RB 2, WR 2, TE 1, FLEX 1 | `verdict.py:68` | hand-set (CN PR #88) |
| H2 league | 8 teams, 8 rounds | `verdict.py:348-349` | hand-set (CN PR #88) |
| VOR tier gap | 0.10 | `vor.py:92` | hand-set (CN). cn-docs §2.6 states 0.15; the code is 0.10 |
| Sniff bands | top-5 ≥ 3, ceiling 24 | `sniff.py:51-53` | hand-set (CN PR #71) |
| Season-to-date spread floor | 1e-6 | `baselines.py:98` | hand-set (CN PR #74) |
| Evaluation scoring | STANDARD | `verdict.py:47`, `:563` | hand-set (CN PR #88). Engine: Half-PPR |
| Pool sizes | QB 20, RB 40, WR 50, TE 15 | alpha-spec §7.3 | spec (kept verbatim) |

---

## 5. Algorithm, numerics and determinism (template: Seed policy, Tolerances)

### 5.1 Cost

- The oracle's 137 validation tests run in about 35 s together with the 71 projection and scoring
  tests, threads pinned to 1 (§7.1).
- `run_verdict` on the canonical legacy synth (16,825 plays, 10 origins, `n = 10,000`) takes 4.0 s
  on 4 vCPU.
- **Bootstrap memory.** The oracle materializes an `n × N` int64 index matrix and the same-shape
  resampled values (`metrics.py:169-171`). At the real H1 size (`n = 10,000`, `N = 7,035` cells)
  each is about 563 MB. The engine MUST stream resamples, or accumulate per-cluster sufficient
  statistics, so memory is `O(N + K)`, not `O(n·N)`.

### 5.2 Numerical definitions the port must match

| Item | Oracle | Measured (2026-10-07) | Engine requirement |
|---|---|---|---|
| `Φ⁻¹` | Acklam approximation (`metrics.py:46-80`) | vs SciPy exact: max abs error 7.36e-9 on `[1e-12, 1 − 1e-12]`, max relative error 1.13e-9; the 80% `z` is 1.281551564140156 vs 1.281551565544600 | Implement the same coefficients and branch points. The approximation **is** the v1 metric definition; replacing it with an exact inverse is a protocol-version change |
| `Φ` | `math.erf` element-wise | vs SciPy: ≤ 2.2e-16 on `[−8, 8]` | Rust stable `std` has **no** `f64::erf`: on rustc 1.98.1 it is the unstable `float_erf` feature (rust-lang/rust#136321). An in-house implementation, or a dependency approved under the dependency policy, MUST match CPython `math.erf` to ≤ 1e-12 on `[−8, 8]` |
| Quantile | numpy default (`linear`, Hyndman–Fan type 7) | — | Type 7: `h = (n−1)p`, `Q = x_⌊h⌋ + (h − ⌊h⌋)(x_⌊h⌋+1 − x_⌊h⌋)` on sorted values |
| Means | numpy pairwise summation | — | Pairwise or compensated summation; Class A tolerance covers the difference |
| Ties in `top_n_hit_rate`, `ndcg_at_k` | `np.argsort` default (unstable) (`metrics.py:308-309`, `:324`) | — | Deterministic tie-break by stable id order, declared in the protocol. Parity only on tie-free inputs |
| Spearman ties | stable mergesort + average ranks (`metrics.py:268-280`) | — | Same; order-invariant |

### 5.3 Determinism and seed policy

- **Oracle randomness:** the bootstrap stream (`default_rng(seed)`), the sign-flip nulls
  (`default_rng(seed + i)`, `verdict.py:471-474`), and indirectly the Layer-1 cross-fit behind
  `smoothed_talent`, which receives the same `seed` as the bootstrap (`verdict.py:414-415`;
  `projection-stack.md` §5.2). That coupling MUST NOT be reproduced: model seeds are part of the
  model version, evaluation seeds part of the protocol record.
- Rust cannot reproduce numpy PCG64 streams. Parity of resampling statistics therefore uses
  **injected index matrices** (parity-fixture contract §5), sized small enough to commit (for example
  `n = 200`).
- Every evaluation output is reproducible from the data snapshot, the model versions, the protocol
  version and the recorded seeds (engine-spec §7.10).

### 5.4 Typed failures (alpha-spec §6.6 rule 4; proposed — DR-B6)

| Condition | Oracle behaviour | Required |
|---|---|---|
| `crps_gaussian` with `sd = 0` | NaN (KI-V4; measured) | the closed-form limit `|actual − mean|`, declared here as the definition |
| `nis` with variance ≤ 0 | inf (KI-V5) | `MetricError::NonPositiveVariance` |
| `pinaw` with zero range of actuals | 0.0 (KI-V6) | `MetricError::DegenerateRange` |
| `spearman` with a constant input | 0.0, averaged into Tier 2 (KI-V12) | NaN, excluded from aggregates, exclusion count reported |
| `skill_score` with reference 0 | 0.0 (`metrics.py:111-112`) | `MetricError::ZeroReference` |
| `ndcg_at_k` with negative relevance | silently computed; "assumes non-negative relevance" (`metrics.py:318`) | fantasy points can be negative: typed error, or a declared transform in the protocol |
| `normal_ppf` with `p ∉ (0, 1)` | `ValueError` | kept as a typed error |
| `bootstrap_ci`: empty sample, `α ∉ (0,1)`, `n < 1` | `ValueError` | kept as typed errors |
| Non-numeric or None ranks in `sniff` | `TypeError` (KI-V7) | typed error |
| `slice_pool` on `ndarray` cells | `TypeError` (KI-V8) | typed participation lists |
| Empty history in `season_to_date_mean` | `sd = {}` (KI-V11) | typed `Option` |
| Frame without a season column | week-only fallback (`asof.py:54`, `:242-243`) | `ContractError::MissingSeasonKey` |
| Watermark at a season seam | skipped (KI-V2) | checked with `(season, week)` (G2) |
| Undated intervention | `LeakageError` | kept |
| Namespace overlapping production | `LeakageError` before any write | kept, against the configured production root |
| Tier 2: duplicates, `min_teams < 2`, no qualifying week | `ValueError` | kept as typed errors |
| `promote` with fewer than 2 samples | `ValueError` | kept |
| Non-finite value in a report | strict-JSON failure | kept |

---

## 6. Incremental and online behaviour

- **Two-path equivalence is the incremental contract.** The walk-forward and the weekly pipeline
  MUST produce identical states and outputs for the same as-of inputs (G4). In the oracle this
  holds only with injected frames, because `weekly_update` refits V(s) on the whole season
  (KI-NEW-W4; reconcile-spec-first R15). The engine's weekly path starts from the batch walk-forward
  semantics; `weekly_update` is not an oracle (`docs/00-meta/known-issues.md` §2).
- **Season seams.** Every time-keyed state and every guard uses `(season, week)` and is tested at a
  seam with a canary (lessons-learned LL-14; KI-V2, KI-NEW-W1).
- **Locked snapshots.** A projection locked at an origin is immutable; re-scoring after a stat
  correction creates a new evaluation record against a new data version (G11).
- **Registry.** Promotions are appended as reviewed commits to the versioned registry; the engine
  never rewrites a frozen entry in place.

---

## 7. Validation evidence (template: Validation and promotion)

All numbers recorded with threads pinned to 1 and the `reference/python/requirements.lock` pins.

### 7.1 Oracle tests (all pass at `59bce1d` on Linux, 2026-10-07)

`python3 -m pytest tests/projection tests/scoring tests/validation`: 208 passed in 34.9 s;
validation 137.

| Test file (count) | What it pins | Status for the port |
|---|---|---|
| `test_asof.py` (14) | cutoff split; season-aware slicing; as-of pool; undated intervention raises; market dict pass-through and table slice; interventions after W excluded; watermark and canary; namespace isolation and canary; tripwire pass, canary, post-construction mutation | Port as behavioural parity. **Divergences:** market dict pass-through, week-only fallback, record-list interventions (§4.2) |
| `test_leakage_guards.py` (7) | the four guards and their canaries (§4.2 G1–G4) | Port as Rust-native guards; add G2 seam canary and G5–G13 |
| `test_backtest.py` (7) | origins skip warm-up; parameter validation; finite ratings; incremental == scratch (`rtol = atol = 1e-8`); stride subset; market anchor changes ratings | Port (`rapm-attribution.md` P-5) |
| `test_baselines.py` (8) | persistence (empty at W = 1); season-to-date mean and spread with pooled and 1e-6 fallbacks; last season; market wrapper | Port persistence and season-to-date as Class A; last season as diagnostic |
| `test_metrics.py` (24) | Φ, Φ⁻¹ inverse and range check; accuracy; skill guard; bootstrap coverage, excludes-zero, determinism, custom statistic, validation; NIS; PIT; CRPS at zero spread "collapses to MAE" (tested in the limit, not at `sd = 0`); PICP; PINAW; pinball; Spearman ties; top-N; NDCG | Port (Class A); the degenerate cases diverge (§5.4) |
| `test_tier1.py` (17) | forecast frames; omission rules; tables; skill vs a weak baseline (weekly and ROS); unknown horizon; calibration on planted noise (NIS within ±0.4 of 1, PICP within ±0.10 of 0.80); ROS forecast cases | Diagnostic only |
| `test_tier2.py` (7) | planted predictive; shuffled not predictive; byes drop; thin weeks; no qualifying week; `min_teams`; duplicates | Diagnostic only |
| `test_lineup_sim.py` (13) | draft partition, slots, FLEX-only TE, determinism; lineup rules, OUT, missing projection = 0 but eligible, tie-break; zero margin; better projection wins; antisymmetry; starter-OUT subset | Port exactly (DR-C11) |
| `test_thresholds.py` (7) | promotion arithmetic; freeze-once; copy semantics; `n ≥ 2`; strict `>`; round trip; committed registry empty | Port the semantics; the "empty committed registry" contract is replaced (§4.11) |
| `test_verdict.py` (23) | synth verdict `SHIP_GRID`; H2 positive; Tier 2 on tautological allowed; gates promoted and frozen; loaders; ROS wiring; smoothed-talent wiring; degraded columns | Diagnostic only; the Tier-2 and anti-correlated-baseline fixtures are not evidence |
| `test_sniff.py` (7), `test_report.py` (3) | rank bands; report rendering and strict JSON | Port as smoke checks |

### 7.2 Legacy-synth verdict (`run_verdict` on the `test_verdict.py` fixture; `n = 10,000`, seed 0)

The fixture plants current-season points `8 + 30·ability + N(0, 0.5²)` and an **anti-correlated**
prior season `8 − 30·ability` (`tests/validation/test_verdict.py:39-42`), so last season is wrong by
construction. Every number here is a wiring check on the legacy synth (KI-NEW-Y0).

| Quantity | Value |
|---|---|
| origins / verdict | 10 / `SHIP_GRID` |
| ROS ALL margin vs last season (n = 1,440) | +0.5698 [+0.5366, +0.6041] |
| ROS ALL margin vs persistence / season-to-date | −1.0528 [−1.1278, −0.9775] / −1.2422 [−1.3167, −1.1677] |
| weekly ALL margin vs last season | +0.5598 [+0.5271, +0.5941] |
| weekly ALL calibration (n = 1,296) | PICP 0.7515, PINAW 0.3672, CRPS 1.0736, NIS 1.1452 |
| H2 overall (8 rosters × 10 weeks) | +0.3055 [+0.1681, +0.4570]; win rate 0.5875 |
| Gates promoted and checked in the same run | H1 gate 0.0583 (passed); H2 gate 0.2074 (passed) |

GRID loses to persistence and the season-to-date mean on this fixture, because its current-season
noise is small. The verdict's `SHIP_GRID` comes only from the deliberately wrong last-season baseline.

### 7.3 Real data (historical, non-parity; engine-spec §3.3, §7.14.5)

From `docs/07-archive/cautious-nevermore/real-data-results.md`, run `real-2022-2023-universe-fixed`
(STANDARD scoring, iid intervals, current-season participation): H1 ALL (n = 7,035) vs last season
−0.015 [−0.059, +0.029], vs season-to-date −0.194 [−0.270, −0.120], vs persistence +0.957
[+0.848, +1.068]; H2 +0.848 [+0.232, +1.466], win rate 0.542; per-run gates `h1_ros` +0.0564 (not
cleared) and `h2` +0.3164 (cleared). Calibration figures (weekly NIS 1.00, ROS NIS 1.25, PICP@80
≈ 0.75–0.87) were recorded for the **first** run only. None of these numbers is a parity target or
evidence: biased labels (KI-NEW-I1..I5, KI-NEW-V0a), iid intervals (KI-NEW-V1), current-season
participation (reconcile-code-first C1; reconcile-spec-first R14; proposed — DR-C1), and the H2
forecast map fitted in-sample (KI-NEW-R3).

### 7.4 iid vs week-clustered intervals (legacy synth; §1.3 item 3)

The same samples as §7.2, re-bootstrapped by week cluster (§4.6), `n = 10,000`, seed 0:

| KPI | iid CI (width) | Week-clustered CI (width) | Width ratio |
|---|---|---|---|
| H1 ROS margin vs last season | [+0.5366, +0.6041] (0.0675) | [+0.5303, +0.6119] (0.0817) | 1.21 |
| H2 lineup margin | [+0.1681, +0.4570] (0.2888) | [+0.1553, +0.4712] (0.3159) | 1.09 |

The H1 clusters here are origin weeks, whose ROS targets overlap (§4.6), so even the clustered
interval is a lower bound on the honest width.

### 7.5 Season seam and interventions (§1.3 items 1–2)

- Two seasons of the canonical synth (relabelled 2022 and 2023), `warmup_weeks = 4`: 24 origins,
  23 watermark checks, unchecked origin `(2023, 1)`. Calling the week-only watermark at that seam
  with the prior season's last week (14) raises `LeakageError`, which is why the driver skips it.
- `AsOf(2023, 5).slice_interventions(records)` with records at `(2024, 2)`, `(2023, 9)` and
  `(2022, 17)` keeps only `(2024, 2)`. The DataFrame branch on the same rows keeps only `(2022, 17)`,
  which is correct.

### 7.6 Promotion (engine-spec §8.8)

A candidate is promotable only when: the correctness gate of engine-spec §7.10 passes, including every
§4.2 guard with its canary; the evaluation is not `research_only`; every §4.12 gate item is
evaluable and passes; and the evaluation record carries this spec's protocol version.

### 7.7 Provenance of measurements first recorded in this spec

| Measurement | Method |
|---|---|
| §7.2 verdict values; §7.4 intervals | `run_verdict` on the `test_verdict.py` fixture, `threadpool_limits(1)`; H1 cells rebuilt with origin tags by replicating `tier1_report`'s pairing (verified equal to its `margins` list); H2 margins are roster-major, so week clusters are recovered by position; a cluster bootstrap of the mean resampling whole weeks |
| §7.5 seam | `walk_forward` on two concatenated canonical seasons with a spy on `AsOf.assert_watermark` |
| §7.5 interventions | direct calls on records and on a DataFrame |
| §5.2 `Φ⁻¹`, `Φ`; §5.4 CRPS at `sd = 0` | the oracle functions against SciPy 1.17.1 on dense grids |
| §5.2 `erf` in Rust | compiling `f64::erf` with rustc 1.98.1 (stable) |
| §10.4 examples | oracle functions called directly |

The probe scripts live in this consolidation's scratch area and are scheduled for commit under
`reference/python/tools/investigations/`. Until then these numbers are *recorded, not reproducible
from the repo*.

---

## 8. Known defects and required engine behaviour (template: Known limitations)

The Rust port MUST NOT reproduce any item below. Each correction lands as an approved oracle
correction (proposed — DR-B1) or a deliberate divergence in `reference/python/PARITY.md` (proposed —
DR-B6).

| # | Defect | KI / source | Required behaviour |
|---|---|---|---|
| 1 | iid bootstrap over correlated cells | KI-NEW-V1 | Paired week-clustered bootstrap (§4.6) |
| 2 | Tier-1 drops forecasts without realized rows; ROS targets average only played weeks | KI-NEW-Z70; `tier1.py:258-260`, `:183-187`; reconcile-spec-first R46 | Union pool with inactive = 0 (§4.7) |
| 3 | Watermark week-only, skipped at season seams | KI-V2 | `(season, week)` at every origin (G2) |
| 4 | Record-list interventions season-blind | KI-NEW-Z61; §1.3 item 2 (new) | Season-aware, `announced_at < lock` (G6) |
| 5 | Static market dict passes through `slice_market`; synth market is planted truth | KI-NEW-Z71; `asof.py:124-126`; `backtest.py:136-141` | Market lines published before the lock only (G7) |
| 6 | Current-season participation folded into RAPM at every origin | KI-NEW-Z68; reconcile-code-first C1; reconcile-spec-first R14 | Publication-lag axis; research-only otherwise (§4.1, G5; proposed — DR-C1) |
| 7 | Gates promoted from the run they judge | KI-NEW-Z72; `verdict.py:470-476`; critic X-16 | Disjoint, pre-registered calibration; never for spec thresholds (§4.11) |
| 8 | Registry docstring vs empty-registry test | KI-NEW-Z73; cn-docs §14 item 8 | Committed, versioned registry entries |
| 9 | No PB-MAE, locks, union pool, provider registry, §9.4/§7.8/§7.9 gates | reconcile-spec-first R47, R52, R54 | §4.5, §4.7, §4.12 |
| 10 | Walk-forward universe is the fixed frame: not-yet-seen players get rating 0 and forecasts | KI-NEW-Z67; §1.3 item 5; `slice_pool` unused (KI-V8) | As-of universe; `no_data` (G8) |
| 11 | Tier-2 synth test tautological; grades inverted | critic X-3; KI-NEW-A2, KI-NEW-V2 | Diagnostic until DR-B5 and a points-allowed feed |
| 12 | Weekly H2 forecast is an in-sample affine map of the RAPM rating | KI-NEW-R3 | Engine locked weekly projections (§4.10) |
| 13 | Evaluation seed also seeds model cross-fitting | KI-NEW-Z62; `verdict.py:414-415`, `:438-445` (new) | Separate seeds (§5.3) |
| 14 | Evaluation scored with STANDARD | KI-NEW-Z69; reconcile-code-first C5 | Half-PPR default; profile in the protocol version |
| 15 | Tier-1 calibration spread omits bias (SD around the mean error) | KI-NEW-Z66; `tier1.py:267` (new) | Calibration from the Layer-F predictive distribution; diagnostics declare their spread |
| 16 | Degenerate metric inputs return silent values | KI-NEW-Z65; KI-V4, KI-V5, KI-V6, KI-V12; `metrics.py:111-112` | §5.4 |
| 17 | Bootstrap materializes `n × N` matrices | KI-NEW-Z63; `metrics.py:169-171` (new) | Streaming resamples (§5.1) |
| 18 | Unstable tie handling in top-N and NDCG | KI-NEW-Z64; `metrics.py:308-309`, `:324` (new) | Deterministic tie-break (§5.2) |
| 19 | Postseason rows in labels and origins | KI-NEW-V0a, KI-NEW-I4 | REG-only labels (proposed — DR-C12) |
| 20 | `weekly_update` is not an oracle (V(s) refit, week-only keys, reinit) | KI-NEW-W1..W5, KI-NEW-W4 | Engine incremental path from batch semantics (§6) |

---

## 9. Open decisions

| Decision | Question | Proposed default |
|---|---|---|
| **DR-C5** | Primary metric and gates | PB-MAE primary; α§9.4 thresholds verbatim and pre-registered; week-clustered bootstrap; union pool with inactive = 0; calibrate-then-gate only for unnumbered KPIs on a disjoint period (proposed — DR-C5). Open inside it: `scale_p` (§4.5 proposal), the strongest-baseline definition, tie handling at rank N, baseline fallbacks, the ROS clustering scheme, numeric meanings of "materially worse", "documented recalibration", "material degradation" and "without degrading veterans" |
| **DR-C1** | Participation on the live path | Publication-lag axis; in-season RAPM research-only (proposed — DR-C1) |
| **DR-C4** | Horizons | Weekly primary; H1 no longer a kill criterion (proposed — DR-C4) |
| **DR-C6** | Three-season window in backtests | Per-season blocks; per-origin window (proposed — DR-C6) |
| **DR-C11** | VOR and lineup simulation | Lineup simulation kept as the start/sit metric, VOR internal (proposed — DR-C11) |
| **DR-C12** | Labels, season type | Official stats, REG only (proposed — DR-C12) |
| **DR-B2** | Live oracle in CI vs committed fixtures | Both: committed fixtures are the Rust contract; a Linux oracle job proves they regenerate (proposed — DR-B2) |
| **DR-B5** | Grade sign for Tier 2 | `+E_def` (proposed — DR-B5) |
| **DR-D23** | Imputation for missing provider projections | Provider's lowest published projection at that position and week; counts reported (§4.7) |
| **DR-D24** | Last season in the §9.4 gate set? | Reported as a diagnostic baseline on every scorecard; gate membership open |
| **DR-D5** | Declared publication-time approximation for backfilled history | Open |
| **DR-D2** | Source of timestamped pre-lock lines for backtests | Open; until then market inputs are unavailable in backtests |
| **DR-D16** | Conditional vs unconditional calibration | Open (`state-space-kalman.md` §9) |

---

## 10. Rust port plan (template: Tolerances, Reference examples)

### 10.1 Target crates and work packages (engine-spec §8.1, §9.5; DR-A8 adopted subject to ratification)

| Piece | Crate::module | WP |
|---|---|---|
| `AsOf`, `Lock`, publication timestamps, `research_only` | `domain::asof` | **P1-01** (type), **P1-05** (behaviour) |
| Leakage harness: G1–G13 with canaries; state namespace | `features::asof_store`, `evaluation::leakage` | **P1-05** |
| Walk-forward with locks, window, frozen projections | `evaluation::walk_forward` | **P1-09** |
| Baselines | `evaluation::baselines` | **P1-07** (naive set), **P1-09** |
| Metrics, PB-MAE, Brier, quantile, `Φ`, `Φ⁻¹` | `evaluation::metrics` | **P1-09** |
| Week-clustered bootstrap (streaming) | `evaluation::bootstrap` | **P1-09** |
| Union pool and provider registry | `evaluation::pool`, `evaluation::benchmark` | **P1-09** (pool); Phase 2 (providers) |
| Lineup simulation with internal VOR | `evaluation::lineup` | **P1-09** |
| Tier 1, Tier 2, sniff (diagnostics) | `evaluation::diagnostics` | P1-09 |
| Registry, gates, promotion records, reports | `governance::registry`, `governance::promotion`, `governance::report` | P1-09, P2-07 |
| Two-path with the weekly pipeline | `pipeline` (DR-A8) | P2-01 |

### 10.2 Fixtures (parity-fixture contract §3, §5; synthetic only, DR-A11)

Component `evaluation-and-leakage`:

- metric inputs and outputs, non-degenerate and tie-free, plus the degenerate cases of §5.4 as
  divergence cases;
- bootstrap cases with **injected index matrices** (small `n`) and their expected CIs;
- baseline histories (persistence, season-to-date) and expected forecasts;
- lineup-simulation rosters, projections, actuals, `out` sets, expected rosters, lineups and margins;
- registry promotion and gate-check cases;
- per-origin walk-forward ratings with injected `dv` (shared with `rapm-attribution`).

These stages are `ledger_independent = true` for injected inputs, except the walk-forward ratings,
which follow the RAPM ledger (`rapm-attribution.md` §10.2).

### 10.3 Parity targets (classes per `docs/03-contracts/parity-fixture-contract.md` §6; proposed — DR-B3)

| ID | Target | Class | Criterion |
|---|---|---|---|
| E-1 | `AsOf` slicing on the `test_asof.py` cases where week and lock granularity coincide | behavioural | identical kept-row sets; divergences of §4.2 listed in `PARITY.md` |
| E-2 | Leakage guards G1–G4 and canaries on the synthetic world | behavioural | G1 bit-identical; canaries fire; G4 per `rapm-attribution.md` P-6 |
| E-3 | Persistence and season-to-date baselines (mean and spread, both fallbacks) | A | `≤ 1e-12` abs; absent players exact |
| E-4 | Accuracy, skill, NIS, PIT, CRPS, PICP, PINAW, pinball, Spearman, top-N, NDCG on non-degenerate, tie-free inputs | A | `≤ 1e-12` abs; PICP counts exact |
| E-5 | `Φ` and Acklam `Φ⁻¹` on grids | A | `≤ 1e-12` abs against the oracle (not against an exact inverse) |
| E-6 | Percentile CI given an injected index matrix | A | `≤ 1e-12`; `excludes_zero` exact |
| E-7 | Lineup simulation (13 oracle cases) and the VOR cases it uses | exact | rosters, lineups, margins, win rates and subsets equal; margins `≤ 1e-12` |
| E-8 | Registry semantics (7 oracle cases) | exact | entries and pass/fail equal; gate `≤ 1e-12` |
| E-9 | Tier-1 / Tier-2 tables on injected forecasts (diagnostic mode) | A | `≤ 1e-12` on margins, MAEs and calibration given injected bootstrap indices |
| E-10 | Degenerate metric cases (§5.4) | divergence | Rust returns the declared value or typed error; listed in `PARITY.md` |
| E-11 | PB-MAE, union pool, week-clustered bootstrap, publication-lag guard, seam canary | spec golden | No oracle source. Golden tests per §10.4 and engine-spec §12.2 |

### 10.4 Reference examples (become golden unit tests)

**Metrics** (oracle-verified):

```text
crps_gaussian(actual 0; mean 0, sd 1) = 0.23369497725510913      (= 2φ(0) − 1/√π)
crps_gaussian(actual 1; mean 0, sd 1) = 0.6024413576276163
pinball(q = 0.8): actual 10, quantile 8 → 1.6;   actual 6, quantile 8 → 0.4
spearman([1, 2, 2, 3], [1, 2, 3, 4]) = 0.9486832980505138
ndcg@3(pred [3, 2, 1], actual [1, 2, 3]) = 0.7899980042460358
normal_ppf(0.9) = 1.2815515641401563   (Acklam; exact 1.2815515655446004)
bootstrap_ci([1, 2, 3, 4, 5], n = 10000, seed = 0) = (point 3.0, lower 1.8, upper 4.2, excludes_zero true)   (numpy stream; oracle only)
```

**Registry** (`test_thresholds.py` arithmetic):

```text
samples [1, 2, 3, 4]: mean 2.5, se = sd(ddof 1)/√4 = 0.6454972243679028
gate = 2.5 + 1.959964 · 0.6454972243679028 = 3.765151321861012;  gate_check(3.8) passes, (3.765151321861012) fails
```

**PB-MAE** (spec golden, hand-computable):

```text
scale = {QB 6.0, RB 5.0, WR 4.0, TE 3.0};  MAE_engine = {QB 6.6, RB 4.5, WR 4.4, TE 2.7}
NMAE_engine = {1.10, 0.90, 1.10, 0.90}  → PB-MAE_engine = 1.000
MAE_baseline = {QB 6.0, RB 5.5, WR 4.8, TE 3.3} → NMAE = {1.000, 1.100, 1.200, 1.100} → PB-MAE_b = 1.100
improvement = (1.100 − 1.000)/1.100 = 0.0909…  (passes "≥ 3%")
per-position QB: (1.10 − 1.00)/1.00 = +0.10 → 10% worse at QB  (fails "no core position worse by > 1%")
```

**Week-clustered bootstrap** (spec golden, injected clusters):

```text
paired margins by week: w1 [1, 3], w2 [−1], w3 [2, 2, 2]     (cell-level mean 1.5)
drawn clusters (w1, w1, w3): cells [1, 3, 1, 3, 2, 2, 2] → T = 14/7 = 2.0
drawn clusters (w2, w2, w2): cells [−1, −1, −1]           → T = −1.0
```

**Seam canary** (spec golden): accumulators holding `(2023, 1)` when the origin is `(2023, 1)` MUST
raise; holding `(2022, 18)` at origin `(2023, 1)` MUST NOT.

---

## 11. Superseded-spec mapping

| Superseded section | Requirement | How this spec satisfies it | Gap |
|---|---|---|---|
| alpha-spec §2.4 | Exact three-season window per prediction | §4.3 step 2 | Per-season RAPM blocks (`rapm-attribution.md` §6.4) |
| alpha-spec §4.5 | As-of reconstruction with six timestamps; leakage tests fail the build | §4.1, §4.2 (G1–G13) | Historical publication times are approximated (DR-D5) |
| alpha-spec §7.1 | Rolling-origin protocol | §4.3 | — |
| alpha-spec §7.2 | Thursday/Sunday locks | §4.1 (`Lock`), §4.3 step 3 | Lock timestamps per season are configuration |
| alpha-spec §7.3 | Union player pool; inactive = 0; missing-provider penalty | §4.7 | Policy ratification (DR-D23) |
| alpha-spec §7.4 | PB-MAE with a training-period scale | §4.5 | `scale_p` ratification (DR-C5) |
| alpha-spec §7.5 | Secondary metrics | §4.5 | Accuracy Gap and start/sit accuracy definitions are spec goldens still to write |
| alpha-spec §7.6 | Benchmark provider registry | §2.1, §10.1 | Phase 2 |
| alpha-spec §7.7 | Paired, week-clustered CIs; metrics versioned before results | §4.6; front matter (protocol version) | ROS clustering scheme (DR-C5) |
| alpha-spec §7.8, §7.9, §9.4 | Gates | §4.12 (verbatim thresholds, evaluation procedures) | Open numeric meanings (DR-C5) |
| alpha-spec §7.10 | Correctness before accuracy | §7.6 | — |
| alpha-spec §9.2 | Naive baselines | §4.4 | Rolling-3 and position/depth-chart median are new |
| alpha-spec §12.1 | Player-pool construction; benchmark metric formulas | §4.5, §4.7, §10.3 | — |
| alpha-spec §12.2 | Goldens for PB-MAE, Brier, calibration, promotion decisions | §10.4 | Brier and promotion goldens to add with P1-09 |
| alpha-spec §12.3 | Leakage tests, including "current-season participation unavailable at serve time must not appear in promoted live features" | §4.1 research mode; G5 | — |
| alpha-spec §12.7 | No threshold changes after seeing results | §4.11, §4.12 | — |
| CN validation plan §8–§12 (archived) | Three axes, four guards, Tier 0–2, H1/H2, calibrate-then-gate | §1.2, §4.2, §4.8–§4.11 | Superseded where this spec is stricter |
