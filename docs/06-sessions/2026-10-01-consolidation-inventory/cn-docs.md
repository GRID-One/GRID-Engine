# Inventory: cautious-nevermore documentation, engine-relevant extraction

Source repo: `/home/user/cautious-nevermore` @ `main` = `59bce1d` (read-only; nothing modified).
Scratch copies of every deleted original (for downstream agents):
`/tmp/claude-0/-home-user/693e74a1-f8af-5256-86e9-2299b8697223/scratchpad/inventory/scratch-cndocs/`
- `orig/*.md`: the 8 docs deleted by PR #92 (byte-identical to `git show 165ccde:<path>`)
- `orig/07-brainstorming-whiteboard.pre-clear.md`: the whiteboard before it was cleared (**only reachable on `origin/docs/consolidate-specs` @ `c33712e`, not from `main`, so it is at risk if that branch is deleted**)
- `orig/superpowers-sdd/phase4-task-{1,3,5,6,7,8,12}-report.md`: deleted Phase-4 SDD reports (`git show 1aeb7ed^:.superpowers/sdd/task-N-report.md`)
- `engine-pr-dedup.txt`: deduplicated squash-commit bodies for engine PRs #63–#91 and #94 (`git log main`). These hold a lot of engine detail that is in **no** doc.

How to read originals: `git -C /home/user/cautious-nevermore show 165ccde:docs/superpowers/<specs|plans>/<file>` (`165ccde` = main immediately before the consolidation squash `379750e`; reachable from `main`).

---

## 0. Headline findings (read first)

1. **The engine knowledge is concentrated in 4 originals plus the commit log, not in the consolidated docs.** `2026-06-23-retrospective-validation-suite.md` (validation design), `2026-06-23-product-roadmap-to-alpha.md` §4.4/§4.5/§7.5 (adapter, projection architecture, compute budgets), `2026-07-09-phase2c-verdict-and-ros-gap.md` (verdict record) and `2026-06-22-fantasy-dashboard-phase4.md` Tasks 1–11 (situations, interactions, 3-state Kalman, changepoints) are the primary sources. PR #92 (consolidation) dropped most of their quantitative content; see §13.
2. **The consolidated docs state aspirational Tier-0 targets as if they were the gates. They are not met.** `docs/02-backend-spec.md:273-282` lists pooled ≥0.85, team ≥0.90, Kalman ≥0.80, NIS ∈[0.8,1.25]. The implemented gates (`tests/grid/test_tier0_recovery.py:84-143`) are floors calibrated *below observed*: pooled ≥0.77 (obs 0.8025), team ≥0.60 (obs **0.6643**), focus-QB NIS ≤10 (obs **4.58**), prior OOS R² ≥0.05 (obs **0.149**), equivalency slope 0.9–1.8 (obs 1.3159 vs planted factor 0.62). Rust parity targets must be the observed baselines, not the doc numbers (§3.6).
3. **The Phase-2c verdict table in `docs/02-backend-spec.md:311-316` has a transcription error and omits the key negative result.** "+0.957 vs persistence" is the **ALL** row, not TE. The consolidated docs drop that overall **GRID loses to season-to-date mean: −0.194 [−0.270, −0.120]** (n=7035). See §4.
4. **The Layer-3 market sign and the matchup-grade sign are an unresolved semantic question, not just a known bug.** G1/V1 (`[+1,+1]` vs `[+1,−1]`) is still unfixed at `59bce1d` (`backend/grid/layers.py:404-405`, `backend/validation/backtest.py:108-109`). Synth plants "team strength" = mean offense ability − mean defense ability (`synth.py:285-294`), which `[+1,−1]` matches. A real closing-line strength is overall quality, though, and **no real market source exists** (`data_adapters.load_odds` raises `NotImplementedError`; neither `verdict` nor `weekly_update` passes a market). So Layer 3 is synth-only, and **G1 cannot have affected the real H1/H2 numbers**. The matchup-grade sign tests (`tests/pipeline/test_weekly_update.py:286-350`) use hand-built betas, not a solve. My synth probe gave corr(β_def, planted defensive toughness) = −0.22 (12 teams; inconclusive because of intercept/starter collinearity). This needs an owner decision plus a truth-anchored test before porting (§14, decisions).
5. **The verdict currency is STANDARD scoring** (`backend/validation/verdict.py:47,551,563`), with 8-team / 8-round VOR-greedy rosters, `H2_ROSTER_SLOTS={"QB":1,"RB":2,"WR":2,"TE":1,"FLEX":1}`, `warmup_weeks=4`, bootstrap `n=10000`, `alpha=0.05`, `seed=0` (`verdict.py:68,345-358`). No doc states this. GRID-Engine's alpha-spec defaults to Half-PPR (§2.3).
6. **Reported "pre-existing failures" are Windows-only.** `docs/10-next-steps-plan.md:73-75` reports golden-master Layer C ×2 and `test_cache.py::test_ttl_expired` failing. On Linux (the declared golden "platform of record", validation-suite §6) at `59bce1d`, the **full suite is 637 passed, 0 failed** (run on a scratch copy with `OMP_NUM_THREADS=1`), golden master included.
7. **Two different "plays" contracts are conflated in the docs.** The GRID engine contract (`data_adapters.py` docstring; produced by `nflverse_adapter.build_plays_contract`) differs from the stats-aggregation contract (`nflverse_loader.PLAYS_CONTRACT_COLS`: passer/receiver/rusher ids, air_yards, td_type, …). `docs/02-backend-spec.md:71-73` presents the latter as "extended fields" of the former. They are not in the engine contract (§2.2).
8. **The Python reference has app couplings that must be vendored.** `backend/validation/lineup_sim.py:32` imports `snake_order` from `backend/services/mock_draft.py:38-46` (app). `verdict.py:539-541` reads realized weekly points from SQLite `player_stats` (`backend.db.connection.get_db`, populated by `backend/pipeline/data_pipeline.py`). `backtest.py:212` imports `backend.pipeline._logging`.

---

## 1. Source inventory and classification

Legend. Classification: E = engine, V = validation, D = data, A = app-only, P = process, M = mixed. Recommendation: **ARCHIVE** = copy verbatim into GRID-Engine for provenance; **EXTRACT** = mine into new engine-only specs and leave the doc in CN; **LEAVE** = stays in CN only.

| # | Source (how to read) | Lines | Class | Engine relevance | Rec. |
|---|---|---|---|---|---|
| 1 | `docs/01-product-roadmap.md` | 216 | M (P/A + engine phases) | Invariants 2–4 (l.28-35), decisions ledger rows (l.39-50), Phase 0/1/2a/2b/2c done-whens (l.54-91), budgets (l.113-166), #8/#9/#11 cross-cutting (l.188-198), risks (l.200-207), data sources and the participation finding (l.209-217) | EXTRACT |
| 2 | `docs/02-backend-spec.md` | 380 | M (≈55% engine) | GRID layers (l.33-85), projection (l.95-110), validation framework (l.207-255), Phase-4 engine evolution (l.257-271), KPI tables (l.273-301), verdicts (l.303-335). **Contains errors**, see §14 | EXTRACT (do not archive as authority) |
| 3 | `docs/03-frontend-spec.md` | 159 | A | None (consumes `kalman_trajectory`, `matchup_grades`, `situation_grades` only) | LEAVE |
| 4 | `docs/04-lessons-learned.md` | 77 | M | Statistical/numerical (l.13-31), validation (l.50-59), process (l.69-77) lessons are engine-relevant; Data-pipeline (l.33-48) partly; Frontend (l.61-67) is app | EXTRACT (engine subset into a "lessons" doc) |
| 5 | `docs/05-current-state.md` | 119 | M (status snapshot) | "What's built" engine/validation list (l.25-55), verdict table (l.91-99). Stale (pre-Stage-0). Contains a local Windows path and GitHub owner (l.9-12) | LEAVE (summarize state in new status doc) |
| 6 | `docs/06-issues-log.md` | 211 | M | Engine audit G1–G14 (l.64-92), P/V (l.170-205), A1/A8/A9 (l.139-152), GitHub bugs #15/#23/#24/#25/#27/#48/#57 and enhancements #32/#42–#49 (l.9-50). Status column is stale ("0 closed" although #25, P1 etc. are fixed) | EXTRACT (engine subset → GRID-Engine issue backlog) |
| 7 | `docs/07-brainstorming-whiteboard.md` (current) | 14 | P | Empty template | LEAVE |
| 7b | **whiteboard pre-clear** `git show c33712e:docs/07-brainstorming-whiteboard.md` | 80 | M | **Engine ideas found nowhere else**: ADP as automated volume signal; smooth age curves; prior_mean for rookies only; promote market baseline to gate; points-allowed proxy from nflverse drive points for Tier 2; unconditional cross-season watermark; Kalman-innovation changepoint detection wired to production; per-position lambda calibration via synth OOS recovery; real-roster H2 | **ARCHIVE** (at-risk branch) |
| 8 | `docs/08-phase-task-context.md` | 174 | M (agent guide) | Known traps (l.117-141), key file map (l.62-106), PR→phase log #63–#91 (l.143-175), plays-contract columns (l.60) | EXTRACT (traps + PR log) |
| 9 | `docs/09-alpha-release-plan.md` | 1072 | A (packaging) | None. Only one engine-adjacent sentence (l.92: participation limitation, WR-vs-CB approximate) | LEAVE |
| 10 | `docs/10-next-steps-plan.md` | 138 | M | Stage-0 triage of engine bugs (l.45-75): P1 fixed, A1/G2 false positives with evidence, G1/V1 deferred pending golden regen. Engine enhancements are post-alpha (l.123) | EXTRACT (defect triage only) |
| 11 | `CLAUDE.md` | 140 | M | Synthetic-data contract (l.44-59), engine architecture (l.61-95), conventions (l.127-140). App layers (l.97-125) | EXTRACT (engine half) |
| 12 | `README.md` | 1 | – | `# cautious-nevermore` placeholder | LEAVE |
| 13 | `sdd/task-8-report.md` | 29 | D/A | Phase-1 Task 8 pipeline automation. Engine-relevant only in its concern that PBP TD/fumble attribution is "best-effort" (l.27) and that the projection = last-season actuals (l.28) | LEAVE |
| 14 | `.superpowers/sdd/task-14-report.md` | 48 | A | Viz-agent ChartSpec/cache | LEAVE |
| 15 | **deleted** `.superpowers/sdd/task-{1,3,5,6,7,8,12}-report.md` (`git show 1aeb7ed^:…`) | ~25–70 each | E (Phase 4) | T1 canonical player order (`key=str`) + accumulator fingerprint; T5 `run_situation_rapm`; T6 situation-keyed accumulators (`MIN_PLAYS=50`); T7 WR–CB interactions (penalty 10.0, synth `cb_split`); **T8 3-state Kalman port (enumerated 2-d sites)**; T12 `kalman_trajectory` table. T3/T12 are mostly DB/API. **Name collision**: the deleted `task-8` (Kalman) is unrelated to the live `sdd/task-8-report.md` (pipeline) | ARCHIVE T1/T5/T6/T7/T8 (small, engine); LEAVE T3/T12 |
| 16 | **orig** `docs/superpowers/specs/2026-06-20-fantasy-football-dashboard-design.md` | 548 | M (≈25% engine) | §3 data contract (l.128-140, original column names), §4 fantasy projection layer (l.188-194), **§5 GRID Engine Evolution by phase (l.196-268)**, §10 budgets (l.512-535). Rest is app | ARCHIVE (provenance: genesis of the engine roadmap) |
| 17 | **orig** `…/plans/2026-06-20-fantasy-dashboard-phase1.md` | 3382 | M (≈10% engine) | Task 4 nflverse loader + ParquetCache (l.607-894: URLs, normalize filter, TTL 168h); **Task 11 vectorized `build_design` + joblib V(s) with GBM hyperparams (l.2570-2704)**; Task 12 orphaned `fantasy_scoring.py` (l.2706-2890); Task 3 scoring presets (l.469-605); Task 6 VOR (l.1200-1360); Phases 2-4 breakdown (l.3294-3382) | LEAVE (extract the listed facts) |
| 18 | **orig** `…/plans/2026-06-21-fantasy-dashboard-phase2.md` | 2434 | A | Draft tools. Only link to the engine: mock-draft AI = VOR + positional need + scarcity + 15% randomness (l.110), and `snake_order` (reused by H2) | LEAVE |
| 19 | **orig** `…/plans/2026-06-22-fantasy-dashboard-phase4.md` | 771 | M (≈50% engine) | **Tasks 1, 2, 4–11 (l.99-424)**, Known Constraints (l.698-713). Tasks 0, 3, 12–19b are DB/API/viz/frontend | ARCHIVE (engine design + honest constraints) |
| 20 | **orig** `…/plans/2026-06-22-phase4-execution-handoff.md` | 102 | P | Merge hygiene; no engine content | LEAVE |
| 21 | **orig** `…/plans/2026-06-23-product-roadmap-to-alpha.md` | 436 | M (≈45% engine) | §2 ledger, §3 invariants, **§4.4 adapter, §4.5 projection architecture**, §5 Phase 0–2c done-whens, §6.5 #4/#8/#9/#11, §7 risks, **§7.5 compute budgets**, §9 interleave | ARCHIVE |
| 22 | **orig** `…/plans/2026-06-23-retrospective-validation-suite.md` | 442 | V (≈95% engine) | Whole doc: tiers, golden master, calibration, leakage, KPIs, H1/H2, adapter, file layout, build order, data-source finding | **ARCHIVE (highest value)** |
| 23 | **orig** `…/plans/2026-07-09-phase2c-verdict-and-ros-gap.md` | 151 | V | Verdict record, decisions, gate values, follow-ups | **ARCHIVE** |
| 24 | engine PR bodies #63–#91, #94 (`git log main`, scratch `engine-pr-dedup.txt`) | ~1500 dedup | E/V | Adapter semantics, real-data smoke numbers, calibration (QB r_scale 0.35→0.55), determinism, golden master design, leakage-guard canaries, missing-week Kalman contract, verdict review rounds | EXTRACT (into an engine changelog / ADRs) |

---

## 2. Engine design (what GRID is), with sources

### 2.1 Identity and statistical intent
- Name: **GRID = Game-state Relative Individual Decomposition**, a player-value estimation engine (`CLAUDE.md:7-8`).
- **Multi-layer fixed point** (`CLAUDE.md:63-77`; `layers.py:1-27` docstring is the canonical intent statement):
  - **Situational value V(s)** (`value.py`): `V(s) = E[drive points | down, distance, yardline]`, gradient boosting; per-play value `dV = V(s') − V(s)`. "Deliberately a standard expected-points scaffold; the originality is downstream."
  - **Layer 2, RAPM** (`layers.py`): ridge with a prior **mean** over participation. Every play is an observation; offense players +1, defense −1; team-offense intercept +1 and team-defense intercept −1 "absorb line + scheme + baseline so individual skill players are NOT credited with the offensive line's work — this is how we keep OL as a nuisance rather than an estimand" (`layers.py:4-9`). Team rating in code = `beta[t_off] − beta[t_def]` (`layers.py:426`).
  - **Layer 3, market reconciliation**: pseudo-observations tie each team's intercept gap to a market-implied strength; "anchors the otherwise free team-level constant and is a falsifiable hook (does bottom-up player value reconstruct the market?)" (`layers.py:11-14`). Real odds source: none (`data_adapters.load_odds` → `NotImplementedError`). Market is "used ONLY as a team-level anchor, never per play" (`data_adapters.py` `load_odds` docstring).
  - **Layer 1, event credit**: cross-fitted, opponent-adjusted residual. A position-agnostic context model `g(state, opponent-defense-rating)` is fit **once** per plays frame (`_cross_fitted_context_residual`), and per-player weekly aggregation is cheap (`layer1_all_players`, PR #80). Its job is to supply the **weekly signal** the state-space layer consumes.
  - **Fixed point**: Layer 2 → defender ratings → Layer-1 opponent adjustment → (optional) QB Layer-1 credit re-seeds the QB's Layer-2 prior. "RAPM already does most opponent/teammate adjustment jointly, so the loop is light." `fit()` re-seeds only the focus QB (issue G14).
  - **State-space** (`statespace.py`): per-player Kalman over `[talent, form, scheme_fit]`, separated by timescale/persistence; West-Harrison discount `d` is "the one interpretable knob"; regime-change weeks drop `d` and inflate R ("rust"); filtered = real-time grade, RTS-smoothed = best retrospective talent; **missing weeks predict without update**, so uncertainty grows during absences (`CLAUDE.md:78-84`).
  - **Cross-league priors** (`priors.py`): estimate feeder→NFL equivalency from shared players, translate a prospect's feeder SV into a position-specific NFL-scale prior (mean + variance) for the Kalman. "Wide prior → high early Kalman gain → fast washout" (`CLAUDE.md:85-88`).
- **Convention to preserve**: "Comments in the engine are unusually load-bearing — they state the statistical intent (what each layer is allowed to estimate vs. treat as nuisance). Keep that intent intact when editing" (`CLAUDE.md:138-140`).

### 2.2 The synthetic-data contract (the most important architectural idea)
- Principle (`CLAUDE.md:44-59`): GRID is validated against **planted ground truth**. `synth.py` generates nflfastR-shaped PBP from hidden abilities/team strengths, and estimators are checked by **correlation** (RAPM recovers a *scaled* ability, so validation is scale-invariant). `data_adapters.py` is **the swap point**: every downstream stage consumes one fixed contract. "When changing any engine stage, preserve this contract." The same text is architectural invariant #3 in roadmap-to-alpha §3.
- **Canonical contract (code docstring, `backend/grid/data_adapters.py:1-23`)**:
  - `plays`: `play_id, drive_id, week, off_team, def_team, down, ydstogo, yardline_100` (state s), `yards, points, terminal, terminal_value` (outcome), `n_down, n_ydstogo, n_yardline_100` (next state s′), `drive_points` (label for V(s)), `off_players (tuple), def_players (tuple)` (participation). `season` is attached when snapshots are loaded (`verdict.py:489-500`). `game_id` is embedded in `drive_id` (`nflverse_adapter.py:98-99`).
  - `players`: `player_id, team, position, is_starter, (ability if synthetic)`.
  - `market`: `{team -> closing-line-implied strength}`.
  - `college`: `player_id, position, feeder_sv, feeder_snaps, is_rookie`.
- Doc variants and discrepancies:
  - `docs/02-backend-spec.md:71` and `docs/08-phase-task-context.md:60` add `game_id, season` to the list. Fine as a superset, but not emitted by the adapter itself.
  - `docs/02-backend-spec.md:73` "Extended fields (fantasy-relevant): pass_attempt, rush_attempt, passer_id, receiver_id, rusher_id, air_yards, yards_after_catch, td_type, fumble, interception" comes from design-spec §3 (orig l.128-140). These belong to the **stats-aggregation** contract (`nflverse_loader.PLAYS_CONTRACT_COLS`, which also has `td_player_id, complete_pass`), **not** the GRID engine contract. Recommend the engine spec define two named contracts: `plays` (engine) and `stat_events` (box-score aggregation).
  - Design-spec §3 "Original" names (`next_down, next_ydstogo, next_yardline_100, yards_gained`) are stale. The code uses `n_*` and `yards`.
- `load_synthetic()` default overrides SynthConfig dataclass defaults: `yards_noise_sd=3.2, drives_per_team_per_game=12`, giving the canonical **16,825-play** dataset used by Tier 0 / run_demo / golden (`data_adapters.py:31-35`; PR #65 body). The golden master pins this explicitly as `CANONICAL_SYNTH`.

### 2.3 Layer parameters (doc-stated; code location for verification)
| Component | Parameter (value) | Source |
|---|---|---|
| V(s) | `STATE_COLS = [down, ydstogo, yardline_100]`; `HistGradientBoostingRegressor(max_depth=4, learning_rate=0.08, max_iter=300, min_samples_leaf=120, random_state=0)`; joblib persistence; **frozen within a season** ("re-fit only on major data changes") | phase1 plan Task 11 (orig l.2655-2672); design §5 Phase 3 (orig l.227); `value.py:25,51-53` |
| RAPM | `lam=120.0`, `w_market=40.0`, team-intercept ridge mask `0.05`, `prior_mean_players` seed, optional `lambda_by_pos` multipliers (uncalibrated, issue #32), interaction columns mask `10.0` | `layers.py:362-410`; `backtest.py:39-41`; deleted T7 report |
| RAPM design | Sparse COO construction (vectorized `build_design`); **canonical column order = `sorted(player_id, key=str)`** (preserves dtype); team cols after players; interaction cols appended after `[players, team_off, team_def]` | phase1 Task 11; phase4 Task 1 (orig l.99-129); deleted T1 report |
| Incremental RAPM | Running `XtX`/`Xty` accumulators (`accumulate` / `fit_from_accumulators`); npz stores `player_order` fingerprint; reinit if `nCols` differs **or** order differs; `last_week` skip guard; per-situation keyed files `rapm_accumulators_{key}.npz` (`key='all'` = legacy) | design §5 Phase 3 (orig l.224-228); phase4 Tasks 1, 6; deleted T1/T6 reports |
| Kalman `SSParams` defaults | `phi=0.50` (form AR1), `phi_scheme=0.985`, `d_steady=0.90`, `d_spike=0.70`, `q_form=0.0008`, `q_scheme=0.0002`, `scheme_reset_var=0.04`, `r_scale=0.40` (R = r_scale/snaps), `post_event_r_mult=2.0`, `post_event_games=2` | phase4 Task 8 (orig l.324-355); `statespace.py:137-147` |
| Kalman per position | QB `d_steady .95, r_scale .55` (calibrated from .35 in PR #66), RB `.85/.45`, WR `.90/.40`, TE `.90/.40`, DEF `.95/.55`. Scheme params position-invariant by design | `statespace.py:121-133`; PR #66 body |
| Kalman structure | `N_STATE=3`; `F = diag(1, phi, phi_scheme)`; `H=[1,1,1]`; P0 = `diag(0.05, 0.02, 0.01)`; x0 talent = `nanmean(y[:3])`; Joseph-form update; RTS smoother; one-step predictive `S = H·P_pred·Hᵀ + R`; versioned npz (`state_version=N_STATE`), legacy 2-comp padded with `[2,2]=0.01` | phase4 Tasks 8–9 (orig l.324-378); deleted T8 report; PR #63 body |
| Scheme reset | `scheme_resets` param: zero `mu[i,2]`, set `sigma[i,2,2]=scheme_reset_var`, zero its cross-covariances; talent/form untouched; driven by hand-curated `coaching_changes` seed (HC/OC → skill positions, DC → DEF). Interventions *also* add `scheme_reset_var` to `P[2,2]` (`statespace.py:203,376`; see §14 item 9) | phase4 Task 10 (orig l.380-402); deleted T8 report |
| Changepoints | `detect_changepoints(state, obs, snaps, params, z_thresh=3.0)`: standardized innovation `z=(obs − H·mu_pred)/sqrt(S)`, flag `|z|>3`; NaN never flagged; run **inside** the per-position loop with that position's `SSParams`; auto set unioned with explicit interventions; CUSUM opt-in (`use_cusum=False`) | phase4 Task 11 (orig l.404-424) |
| Situations | `red_zone: yardline_100 ≤ 20`; `passing_downs: (down==3 & ydstogo≥7) | down==4`; `rushing_downs: down≤2 & ydstogo≤4`; `two_minute: quarter_seconds_remaining ≤ 120` only if the column exists (not in the contract) | phase4 Task 4 (orig l.205-234); `situations.py:30-58`. **`docs/02-backend-spec.md:81` wrongly adds goal_line / third_and_long / fourth_down** |
| Situation RAPM | `run_situation_rapm(min_plays=200)` = `run_rapm` on each masked subframe; **raises if participation columns are absent**; players absent from a situation get ridge-prior 0 and must surface as "no data", not 0. Pipeline uses `MIN_PLAYS=50` | phase4 Task 5; deleted T5/T6 reports |
| WR–CB interactions | `build_design(interactions=False, min_pair_plays=30)`; positional proxy (each on-field WR × each on-field CB, else DEF); heavily ridge-penalized; off by default so dims and golden are untouched; synth `cb_split`/`n_cb_per_team=2` knob | phase4 Task 7 (orig l.300-322); deleted T7 report |
| Layer-1 | 5-fold cross-fit (`KFold`, seeded); the calibration test uses **GroupKFold by week** (stricter; production is play-level) | PR #66 body; `verdict.py:129` (`n_splits=5, seed=0, min_plays=20`) |
| Priors | `estimate_equivalency` regresses NFL `rating` on `feeder_sv` (slope/intercept/oos_r2); `ability` optional (synth-only `factor_check`); `prior_mean = intercept + slope·feeder_sv`; planted feeder `league_factor` 0.62 in synth; `league_factor` "computed and documented but never applied" (issue #23); single random split for OOS R² (issue #48); divide-by-zero when `len(shared)<4` (G8) | PR #82 body; `test_tier0_recovery.py:132-143`; issues log |
| Cache | `ParquetCache(get(key, ttl_hours)/put)`; nflverse PBP TTL **168 h**; ADP/projections 6 h; expired files never deleted (G9); `ttl=0` boundary bug on coarse-mtime filesystems | phase1 Task 4 (orig l.640-680); design §3 (l.106-111); doc 10 l.75 |

### 2.4 Real-data adapter (nflverse → engine `plays` contract), PR #67/#68 bodies + roadmap §4.4
- Was "nearly invisible": scoped as plumbing, actually net-new **L–XL** (drive segmentation, next-state derivation, next-score labeling, participation join), logic that previously existed only in `synth.py` (roadmap §4.4 orig l.80-97; lessons l.7-8).
- Semantics (must match `synth.simulate()` exactly): filter to `play_type ∈ {pass, run}` with non-null `down`; `drive_id = game_id + "_" + fixed_drive`; `drive_points` from `fixed_drive_result` (**Touchdown → 7, Field goal → 3, else 0**), broadcast to every play in the drive; next state via within-drive `shift(-1)`; terminal = last play of the drive with `n_* = −1`; `terminal_value = drive_points` on terminal (NaN otherwise); `points = drive_points` on terminal (0 otherwise).
- Participation: `pbp_participation_{year}.parquet` → `[game_id, play_id, offense_players, defense_players]` (rename `nflverse_game_id → game_id`, fallback `old_game_id`); split on `;`; join on `(game_id:str, play_id:int)`, normalized on both sides because a float/int mismatch silently dropped all participation; pd.NA/NaN/None cells → empty tuple. **`off_players` keeps only offensive skill positions `{QB, RB, WR, TE, FB}`** (OL deliberately excluded, consistent with team intercepts absorbing line). Defense is kept as listed.
- Rosters: correct URL is `releases/download/rosters/roster_{year}.parquet` (`roster_weekly` was a 404); map `gsis_id/full_name → player_id/name`. `gsis_id` matches participation ids.
- **The RAPM player universe must be the full roster (all positions, deduped to the most recent season row, NaN-team rows dropped), not the fantasy-skill subset.** The first real run dropped ~22k participants per origin as unknown ids (PR #89; lessons l.55-56).
- Real-data smoke (2023, manual, not CI): V(1st&10 @ the 90) = 1.25 → 4.91 @ the 10, mean dV ≈ 0, average 1.96 drive points; participation coverage 100% off/def; 5.97 skill + 11 defenders per play; RAPM top QB C.J. Stroud; top WRs Lamb / Deebo Samuel / Tank Dell (PR #67/#68 bodies). Required coverage gate: `off_players/def_players` populated on **≥99%** of run/pass plays (roadmap §5 Phase 1, orig l.190).
- Snapshot: `python -m backend.pipeline.ingest_grid --years …` writes `data/snapshots/grid_plays_{year}.parquet` + `manifest.json` (per-season `n_plays/n_drives/off_coverage/def_coverage` + totals); default years 2019–2024 (`ingest_grid.py:25`); duplicate years rejected; `[]` is a no-op. The raw nflverse parquet stays cached so rebuilds are offline (roadmap §6.5 #4).
- Phase-2c verdict data: 2022–2023, **70,778 plays, 100% participation coverage** (phase2c doc l.7-8).
- Stats path (`nflverse_loader._normalize_pbp`) aliases raw `fumble_lost → fumble`. So the A1 "fumbles counted" finding was a **false positive** (doc 10 l.67); residual naming smell only.

### 2.5 Projection layer (engine-adjacent, `backend/projection/`)
- **Key principle** (roadmap §4.5 orig l.106-108; dropped from consolidated docs): "fantasy points = **volume × efficiency**. Volume (attempts/targets/carries/snaps) is the dominant, sticky driver; GRID's RAPM/Kalman measures **per-play efficiency/talent only**. So GRID cannot *be* the projection — it is the most valuable *feature* in one."
- Structure (decided): **project a stat line, then score** (format-flexible): `projected_stat_line = g(volume/role features, GRID talent features, age, prior, team/role-change signal)`; `projected_points(format) = calculate_points(stat_line, format)` (roadmap §4.5 l.114-119).
- **H1 implication** (l.148-151): last-season actuals already encode volume × efficiency, so GRID's win must come from (a) regressing unsustainable efficiency/TD rate to talent and (b) trajectory/aging, **not** (c) volume/role change, which GRID doesn't model.
- Volume model (`volume.py`, PR #77): empirical-Bayes shrinkage of most-recent **prior-season** per-game usage toward the position per-game mean; weight `g/(g + k_shrink)`, `k_shrink = 8.0`; `VOLUME_COLS = [pass_attempts, rush_attempts, targets, receptions]`; manual ADP/depth-chart override hook (rejects unknown columns); no prior season → empty. Mid-season blending of current-season usage is deferred. **Known weakness: role-changers.**
- Talent features (`features.py`, PR #78): `rapm_rating` (prior-season Layer 2), `smoothed_talent` (genuinely re-runs the RTS smoother, **not** the DB's filtered `kalman_trajectory.talent`, because aliasing filtered for smoothed is the "correct-looking-wrong-semantic" bug class), `prior_mean`/`prior_var` (rookies only; **NaN for non-rookies, meaning "no prior", distinct from 0**).
- Stat-line model (`model.py`, PR #79): per driver, ridge (`alpha=1.0`) of the historical per-unit rate on standardized talent features (one `StandardScaler` per driver); `RATE_STATS` maps each driver to the stats it doesn't supply; zero-volume rows excluded per driver; `fumbles_lost` / `two_point_conversions` deliberately unmodeled (0.0). P1 fix: predict fills present-but-NaN with 0.0 to be train-consistent.
- SV→points map (`sv_to_points.py`, PR #81): per-position affine `weekly_fp ≈ a_pos + b_pos · weekly_credit`; leakage-safe via `AsOf.slice_plays`; positions below `min_rows` or with no credit spread are omitted and `predict` returns 0.0 (no guessed conversion). **This is the weekly lever (H2). The season stat-line model is the ROS/draft lever (H1).** Mis-wiring them produced the −0.864 artifact.
- Preseason (`preseason.py`, PR #83): volume × efficiency → per-game stat line × expected games.
- Orphaned `backend/fantasy_scoring.py` (hand-picked `GRID_SCALING` constants, phase1 Task 12 orig l.2790-2810) is superseded. **Do not port** (doc 08 l.128-129).

### 2.6 Scoring + VOR (needed by verdicts)
- `ScoringConfig{name, rules: stat→multiplier}`; `calculate_points = Σ stats.get(k,0)·mult`; serialization nests `rules` (bug A2 came from reading the wrong level).
- Base rules (phase1 Task 3 orig l.585-600): pass yd 0.04, pass TD 4, INT −2, rush yd 0.1, rush TD 6, rec yd 0.1, rec TD 6, fumbles_lost −2, 2-pt 2; receptions 0 / 0.5 / 1.0 for Standard / Half / Full. **Identical to GRID-Engine alpha-spec §2.3 Half-PPR.** `SCORING_STAT_COLS` has 14 columns (`scoring/columns.py`).
- VOR: replacement = player at 0-based index `slots·num_teams` per position (i.e. rank slots·num_teams + 1); FLEX-only positions get the FLEX replacement level (PR #85); gap-based tiering `assign_tiers(max_gap_pct=0.15)`; tier fragmentation near zero VOR (A5, #47).

---

## 3. Validation design (retrospective-validation-suite + roadmap §5/§7.5 + code)

### 3.1 Purpose and philosophy
- Ground-truth recovery proves the math. The walk-forward backtest answers the product question: "does GRID beat the baselines a fantasy manager already has?" "Retrospective" = **walk-forward backtesting**: at each historical point, predict using only information available then (val-suite §1, l.14-25).
- Both kinds stay (val-suite §3 table, l.60-67): recovery = seeded, exact, hard CI gate; real backtest = noisy, report → promoted gates.
- **Calibrate-then-gate** (val-suite §2 l.43-45): only Tier 0/0.5 are hard CI gates from day one. Real-data KPIs run report-only for one pass to measure the noise floor, then directional KPIs are promoted against the observed baseline distribution. Every "defined bound" is **provisional until measured** (roadmap §6.5 #8).
- **Success ≠ "GRID wins"** (roadmap §5 Phase 2 note, orig l.227-231): "a trustworthy, leakage-free answer + the product reflecting it". Beating last-season actuals is a **floor** ("not broken"). The real value signal is beating **season-to-date mean and market**. "Defining success as 'GRID wins' would create pressure to defeat the leakage guards."

### 3.2 Tiers
- **Tier 0, recovery**: formalized `run_demo.py` correlations (seeded).
- **Tier 0.5, golden master + determinism**: three layers (§3.4) and in-process determinism < 1e-9.
- **Tier 1, real walk-forward**: weekly fantasy points + ROS VOR/rankings, per position × {weekly, ROS}.
- **Tier 2, attribution/matchup**: synth gates now; real once participation (done) **and a points-allowed feed (missing)** exist.

### 3.3 Targets and evaluation population (val-suite §5, l.84-99; dropped from consolidated docs except "no survivorship")
- Realized points = volume × efficiency + TD variance. Score **both** realized points **and** an opportunity target (carries/targets/snaps), reported separately.
- Horizon split = the two levers: weekly is TD-variance-dominated; ROS averages it out.
- No survivorship (freeze the eval set to everyone rosterable as of W). DNP/zero weeks scored two ways (0 vs excluded). Per position always. Ranking metrics are top-N.

### 3.4 Golden master (val-suite §6, l.103-137; PR #65)
- Motivating bug: `matchup_grades` stored mislabeled offensive means. A float freeze would miss it and be "fixed" by regenerating.
- Layer A, schema + **semantic invariants anchored to planted truth** (exact): shapes, dtypes, rating/ability sign per position, team-rating sign, strongest team in top-2, talent more persistent than total, injury-return dip + variance widening. "Cannot be gamed by regenerating."
- Layer B, ordering/membership (exact): top-10 player ranking, team ranking, injury-dip location, played-week set.
- Layer C, numeric `assert_allclose(rtol=1e-5, atol=1e-6)` on ratings, team ratings, weekly QB credit, focus-QB Kalman states; failure prints the largest movers.
- Determinism hazards: pin `random_state` on HistGBR and Layer-1 `KFold`; canonical `player_id` order; `OMP_NUM_THREADS=1` / `threadpool_limits(1)` (**multi-threaded GBM diverges ~1e-2**, doc 08 l.140-141); `requirements.txt` uses `>=`, so record/pin versions.
- **Platform of record = Linux/CI**; cross-platform floats diverge ~1e-6; Layers A/B carry the regression weight. Regenerate with `python -m tests.grid.golden_master` → `tests/grid/golden/snapshot.npz`. "The golden diff in a PR is the feature." Anti-gaming: Tier 0 + Layer A must also pass.

### 3.5 Calibration design (val-suite §7, l.141-193; "GRID's moat")
- Engine fix (done, PR #63): surface the one-step predictive `(mean = H·x_pred, var = S = H·P_pred·Hᵀ + R)`. The filtered variance is circular and omits R ("every coverage number is overconfident by construction"). Also fix the DB write path (`_write_kalman_trajectory` previously persisted 2-component, R-omitted variance). Smoothed covariances are **off-limits** for forecast calibration (they use future weeks).
- **Three distributions**: filtered/nowcast (weak, circular); one-step-ahead (honest weekly target, maps to the injury-replacement lever); multi-step/ROS (variance compounds through predict-without-update, maps to the bench-depth lever).
- **R depends on unknown snaps** (`R = r_scale/snaps`): report **conditional** (given realized snaps, isolates the value model) and **unconditional** (forecasts snaps too). The gap quantifies volume vs value uncertainty. (Dropped from consolidated docs.)
- Metrics: NIS `z=innov/√S`, `mean(z²)≈1` (audits `d` and `r_scale`; deterministic synth gate); PIT histogram shape reading (∪ overconfident, ∩ underconfident, slope = bias, edge spikes = zero inflation); **coverage + sharpness together** (PICP@80/50 paired with PINAW); CRPS closed-form Gaussian as **skill vs a spread-emitting baseline**; pinball at 20th and 80th separately.
- **Aggregation escape**: weekly points are skewed/zero-inflated, so expect Gaussian PIT tail defects. ROS totals (~17-week sums) are near-Gaussian by CLT. So: **ROS → Gaussian coverage/CRPS gates; weekly → quantile/pinball + randomized PIT.**
- Regime-conditional calibration: first-game-back coverage with vs without rust; during-absence variance growth; rookie prior-variance coverage early.
- Implemented synth calibration gates (`tests/grid/test_calibration_synth.py`; PR #66): week-held-out (GroupKFold) innovations pooled over all synth QBs (~310 player-weeks). Median per-QB NIS ∈[0.8,1.25] (obs 0.93), pooled NIS ∈[0.8,1.4] (obs 1.24), PIT mean ≈0.5, PIT sd ≈0.289, PICP@80 ∈[0.70,0.90] (obs 0.78).
- Metrics module facts (PR #72): numpy-only, closed-form; Φ and Acklam Φ⁻¹ implemented (no scipy); tie-aware Spearman; `bootstrap_ci` percentile bootstrap (PR #84), seeded. Known edge bugs V3–V6, V12 (§10).

### 3.6 KPI targets: plan vs implemented (use the "observed" column for Rust parity)
| KPI | Plan target (val-suite §9 / doc 02) | Implemented gate (`test_tier0_recovery.py`) | Observed @59bce1d |
|---|---|---|---|
| Attribution pooled corr(rating, ability) | ≥0.85 | ≥0.77 | 0.8025 |
| Attribution per position | ≥0.70 each | QB .83, RB .70, WR .76, TE .73, DEF .73 | QB .869, RB .743, WR .797, TE .771, DEF .765 |
| Team strength corr | ≥0.90 | ≥0.60 (market-reconciled) | 0.6643 |
| Kalman trajectory | corr(total_filt, true) ≥0.80 | total_smooth ≥0.92; tau_smooth ≥0.60 | 0.958 / 0.6745 |
| Injury detection | dip + recovery (bool) | var(wk10) > var(wk3) | 0.0023 → 0.0061 |
| Prior equivalency | OOS R² ≥0.50 | slope ∈[0.9,1.8]; OOS R² ≥0.05; rookie corr ≥0.50 | slope 1.3159 (planted 0.62); R² 0.149; rookie 0.5829 |
| Filter NIS (focus QB, dV currency) | ∈[0.8,1.25] | ≤10 (one-sided blow-up guard) | **4.58** (overconfident; tuning deferred) |
| Determinism (in-process) | <1e-9 | <1e-9 (dV, ratings, team ratings, weekly QB credit, full predictive-variance trace) | pass |
| Golden C | rtol 1e-5 / atol 1e-6 | same | pass on Linux |
| PIT uniformity (synth) | within tol | PIT mean≈0.5, sd≈0.289 | pass |
- Tier 1 plan targets (val-suite §9 l.293-334; mostly dropped from consolidated docs). Accuracy context (not gated): weekly PPR RMSE ≈ QB 6–7, RB/WR 7–9, TE 5–6; ROS MAE within ~15%. Skill gates: beat persistence >0.10 (CI excl. 0); beat season-mean >0 (CI excl. 0, the "real bar"); **beat last-season >0 (CI excl. 0) = kill criterion**; beat market >0 (stretch/report). Ranking: Spearman ≥0.55 RB/WR, ≥0.65 QB/TE; top-N hit ≥60%; **tier accuracy ≥70%**. H2: overall margin >0 (CI excl. 0) **hard**; starter-OUT subset >0 and ≥ overall; win rate ≥55%. Calibration: ROS PICP@80 ±5pp **hard**; **PICP@50 ±7pp**; **weekly pinball @20/80 ±7pp**; **PINAW ≤ baseline**; CRPS skill >0 (CI excl. 0) **hard**. Regime: injury-return RMSE −10% with rust; return-week PICP ±10pp; **changepoint precision/recall ≥0.6/≥0.5 vs seed; false-positive rate ≤10% on stable players (hard)**; rookie priors beat naive wk 1–4, converge by ~wk 8.
- Tier 2 (val-suite l.336-341): situation-RAPM recovery ≥0.65 (synth); matchup-grade semantics exact sign vs planted defense (synth); real predictive `corr(grade, pts allowed) > 0, CI excl. 0`. Implementation (PR #87): per-week `−spearman(rapm_grade, points_allowed)` aggregated by bootstrap over weeks; rejects `min_teams<2` and duplicate `(season, week, def_team)` rows; inner join drops byes.

### 3.7 Leakage design (val-suite §8, l.197-249; PRs #73, #75, #76)
- Three axes: **temporal** (future rows), **scope** (a global fit spanning time leaks into a local prediction: V(s), VOR replacement, RAPM market anchor, cross-league priors), **state** (persistent caches are a global singleton: cross-fold contamination, *production* contamination, the backtest overwriting the user's Kalman state, non-reentrancy).
- Choke point `AsOf(season, week, mode)`: outcome slices (`slice_plays`, `slice_pool`) keep `≤ W−1`; pre-game slices (`slice_market`, `slice_interventions`) allow `≤ W`. **Season-aware tuple cutoff** (a week-only filter leaks future-season low weeks). Undated intervention records raise. `CacheNamespace` asserts isolation before mkdir. `TripwireFrame` raises on any future-row read (incl. `len()`). "Filter for correctness, poison to verify completeness, tripwire to localize."
- Four guards, each with a deliberate-leak canary ("test the test"): (1) **metamorphic future-poisoning**: forecast W twice with two random futures (weeks ≥ W, state columns shuffled), output must be bit-identical (headline); (2) temporal watermark; (3) tripwire; (4) **two-path equivalence**: offline backtest week-W equals production `weekly_update` after weeks 1..W−1 (with `last_week == W−1` and the persisted `player_order` matching).
- Scope-leak poison checks: V(s) frozen to pre-period or refit ≤ W−1; VOR replacement from the as-of pool; **RAPM market must be the line as published before W**; equivalency frozen to prior seasons.
- **Three GRID-specific leaks**: intervention foreknowledge (filter interventions known as of W; check coaching-seed dates; it leaks *confidence*); information time ≠ week index (cutoff = "available before kickoff of W"; design an information timestamp per source); player-universe leakage (build the universe as of W).
- Known gap V2: the watermark guard is skipped at season boundaries (`backtest.py:173-174`). Whiteboard idea: check `last_slot` against the current origin's cutoff unconditionally.

### 3.8 Walk-forward driver and compute budgets (roadmap §7.5 l.333-362; val-suite §11.5)
- Driver (PR #75): V(s) fit **once** on a reserved warm-up pre-period (`warmup_weeks=4` default) and reused; incremental accumulators in memory; each origin = solve → record ratings + `def_grades` (`−beta[t_def]`) → fold the week in; incremental == from-scratch (tested); `origin_stride` fast mode (logged, never silent); teams absent from the market are skipped (no false zero prior).
- Three distinct budgets ("conflating them is the trap"): **A. CI synth gates < ~60 s per PR** (Tier 0/0.5 + synth calibration + leakage canaries); **B. real backtest < ~10 min dev/nightly, NOT per-PR**, a *design requirement* met by incremental RAPM + frozen V(s) (avoids ~126 GBM refits) + cheap per-player Kalman + **sampled H2 bootstrap with logged caps** + fast-iteration subset mode ("if naive refit-per-origin, B is hours"); **C. shipped app**: weekly incremental update seconds–low minutes, first-run setup network-bound (~0.5–1 GB download), **ingest peak RAM ~1–2 GB**, disk < ~2 GB. Measured locally on Linux: the full CN pytest suite (637 tests, incl. app) runs in 154 s single-threaded.
- Design-spec performance targets (§10 orig l.525-535) that are engine-relevant: **RAPM full solve (vectorized) < 1 s**; **weekly pipeline refresh < 60 s**. Storage: nflverse 5 yr Parquet 500 MB–1 GB; SQLite ~50 MB.

### 3.9 H1 / H2 definitions (val-suite §2 l.46-58, §9 l.260-269, l.308-316)
- **H1 (bench-depth / ROS), kill criterion**: GRID beats last-season actuals on ROS skill. Full north star: beats both season-to-date mean and last-season on ROS skill for RB/WR/TE, ROS PICP@80 within ±5pp, and the injury-return and rookie-prior mechanisms each reduce error. Verdict enum `SHIP_GRID / BASELINE_FALLBACK / INCONCLUSIVE` (PR #88).
- **H2 (injury replacement / weekly)**: identical roster and identical slot-filling rule for both methods; only the projection source differs (GRID as-of weekly forecast vs last-season weekly average). Margin = Σ(GRID lineup actual) − Σ(baseline lineup actual), bootstrapped over rosters × weeks; reported overall and on the starter-OUT subset; tie-aware win rate. Rosters are **synthetic VOR-greedy snake drafts** (deterministic; reuses `calculate_vor` + `snake_order`).
- **Gate promotion rule** (roadmap §5 Phase 2c l.221-225; `thresholds.py`): the *single first full backtest pass* sets each directional KPI gate at `baseline_mean + 1.96·SE(baseline)` where the baseline is that KPI's **sign-flip null** (seeded); then **frozen**. Re-promotion is a no-op; `force=True` is the reviewable recalibration path. The committed `provisional_thresholds.json` ships `{"kpis": {}}` by contract (`test_committed_registry_is_valid_and_empty`), so run gates live in the report. **Contradiction**: the `thresholds.py` docstring says the registry "IS committed … the gates are part of the spec once frozen" (§14).

### 3.10 Baselines (PR #74)
Persistence (last week); season-to-date mean **with per-player spread** (pooled fallback when <2 games; 1e-6 floor when unavailable), used as the CRPS reference; last-season per-game average (= what `compute_valuations` ships; the H1 reference); market (external ADP/ESPN, report-only, **never wired**). All return `PointForecast(mean, sd?)`.

---

## 4. Verdict results (Phase 2c, real 2022–2023) — corrected record
Source: phase2c doc (orig l.1-151) and PR #88–#91 bodies. Commands: `python -m backend.pipeline.ingest_grid --years 2022 2023`; `python -m backend.pipeline.data_pipeline --years 2022 2023`; `python -m backend.validation.verdict --run-label <label>`.

| Run label | H1 ROS vs last-season | H2 weekly margin | Note |
|---|---|---|---|
| `real-2022-2023-first-run` | −0.864 [−0.932, −0.796] | −0.524 [−1.486, +0.461]; win rate 0.478 | ROS scored by the **weekly SV→points map** (wrong projection); also lost to season-to-date; failed even persistence for RB. Gate registry reset, not frozen |
| `real-2022-2023-ros-corrected` | −0.015 | – | ROS via volume × efficiency, but still the broken player universe |
| `real-2022-2023-universe-fixed` (post-#89, **trustworthy**) | **−0.015 [−0.059, +0.029] → BASELINE_FALLBACK (tie)** | **+0.848 [+0.232, +1.466] PASS**; win rate 0.542; clears frozen gate +0.3164 | ALL n=7035 |

ALL-row margins (trustworthy run): vs persistence **+0.957 [+0.848, +1.068]**; vs season-to-date **−0.194 [−0.270, −0.120]** (GRID slightly worse); vs last-season −0.015 [−0.059, +0.029].
Per position vs last-season: **TE +0.064 [+0.011, +0.118]** (also vs season-mean +0.228 PASS, so TE clears the kill criterion); RB +0.005; QB −0.036; WR −0.066. DB/P rows exist (15/30 cells) **deliberately**: the ROS model is position-agnostic so non-skill rows can feed a weekly in-season mode.
Calibration: weekly NIS(ALL) = 1.00; ROS NIS(ALL) = 1.25; PICP@80 ≈ 0.75–0.87. Tier 2 skipped (no points-allowed feed).
Per-run frozen gates (in the report, not VCS): `h1_ros` +0.0564, `h2` +0.3164.
Interpretation (phase2c l.89-94, l.120-132): ROS is **volume-dominated**, with RAPM one talent feature, so fixing the RAPM universe barely moved ROS but materially helped H2 (H2 rides directly on weekly RAPM forecasts). Decision: `compute_valuations` stays on the last-season baseline for alpha.
Follow-up done (#91): `smoothed_talent` = RTS-smoothed end-of-pre-period Kalman talent from weekly Layer-1 credit, **frozen** on the pre-first-origin window and fed to both ROS fit and forecast. **The H1 re-run with this feature was never recorded** (no doc or commit reports a post-#91 H1 number). `prior_mean` is still 0.0 in the verdict path.

---

## 5. Engine phase history (for a GRID-Engine history / ADR record)
- **Design era (2026-06-20 → 06-22)**: design spec §5 "GRID Engine Evolution". Phase 1: real loaders, vectorized `build_design`, joblib V(s), cached design matrix. Phase 2: no engine change. Phase 3: incremental accumulators, Kalman append, all-QB credit, WR/TE target-share attribution (**never built**, #32), position-specific lambda (infrastructure only), priors expansion with draft capital/age and separate FBS/FCS/UFL equivalency (**never built**; #49 step functions). Phase 4 (plan 2026-06-22; tests 257 → 313+): T1 canonical order, T2 matchup-grade semantics (defense intercept, sign-flipped), T4 situations, T5 situation RAPM, T6 situation accumulators, T7 WR–CB interactions, T8 3-state Kalman, T9 npz migration, T10 coaching resets, T11 changepoints.
- **Roadmap-to-alpha era (2026-06-23 → 07-17)**, PR → phase (doc 08 l.143-175):
  - Phase 0 (#63 predictive `(mean,S)`, honest trajectory band, injectable cache paths; #64 determinism + Tier 0; #65 golden master; #66 synth calibration + QB `r_scale` 0.35→0.55). Suite 384 → 411.
  - Phase 1 (#67 adapter; #68 participation loader + `load_grid_plays` + roster URL/gsis fixes; #69 `weekly_update` on real data + `run_rapm` real-data-tolerant; #70 `ingest_grid` snapshot; #71 sniff gate). → 438.
  - Phase 2a (#72 metrics; #73 AsOf/CacheNamespace/Tripwire; #74 baselines; #75 walk-forward; #76 leakage guards). → 499.
  - Phase 2b (#77 volume; #78 talent features; #79 stat-line model; #80 per-position weekly Layer-1 via shared cross-fit; #81 SV→points; #82 priors on real data with `ability` optional; #83 preseason + `compute_valuations` injectable projection). → 551.
  - Phase 2c (#84 `bootstrap_ci`; #85 H2 lineup sim; #86 Tier-1 runner; #87 Tier-2 + threshold registry; #88 verdict runner + report → 602; #89 universe fix; #90 ROS via volume × efficiency + data-quality surfacing; #91 smoothed_talent → 628).
  - Stage 0 (#94, 2026-07-17): P1 fixed (NaN → 0.0 in `RateModel.predict`); A1, G2 shown to be false positives; G1/V1 deferred to a dedicated engine PR with golden regen.
- Current: 637 tests pass on Linux @59bce1d.
- Not done (engine): G1/V1 fix; `prior_mean` on real data in the verdict path; market baseline; Tier-2 points-allowed feed; WR/TE target-share attribution; lambda-by-position calibration; real-data situation grades producer (#57); per-week projection producer (A4); DEF/K stat aggregation (A7); weekly in-season participation.

---

## 6. Decisions ledger and invariants (engine-relevant rows only)
From roadmap-to-alpha §2 (orig l.31-49) and §3 (l.51-64); consolidated `docs/01-product-roadmap.md:28-50`.
- Core projections are GRID-powered: a fitted stat line (GRID talent features + volume model) replaces the last-season baseline. **The verdict reversed this for alpha (fallback), but the architecture stands.**
- Participation: free via nflverse/FTN through 2025; live-weekly is a research track.
- The real-data GRID path (adapter) is net-new and gates all of Phase 2.
- H2 weekly signal: per-position weekly Layer-1 credit + SV→points map are in scope.
- Validation is a **recurring gate**, green at every phase boundary.
- Headlines: H1 = beat last-season on ROS skill (kill criterion); H2 = weekly lineup point margin.
- Timeline stance "GRID-powered or bust", quality over date; baseline-interim decoupling considered and **rejected** (roadmap §9 l.431-436). App-specific; record only as history.
- Long-term native: "evaluate true-native (Flutter/Avalonia) only if feel falls short" (app; superseded by the GRID-Engine Rust decision).
- Engine invariants worth keeping verbatim: **#2 "Engine is UI-agnostic. The GRID engine never moves to accommodate a UI; the client/server seam stays clean."** **#3 "The synthetic-data contract is preserved (`data_adapters.py`) through every engine change."** **#4 "Validation green before any phase is considered done."** #6 sub-clause: "generated caches are reproducible, never hand-migrated"; on an engine-format bump, invalidate and rebuild from locally cached raw parquet, offline (`state_version` mechanism, roadmap §6.5 #4).
- Validation ⟷ product interleave (roadmap §9 l.403-420): the walk-forward harness **is** the projection model's train/eval harness; the Tier-1 ROS target model **is** the stat-line projection model; `lineup_sim` is reusable for start/sit. The only pure-validation tax is the leakage guards, Tier 0/0.5 and metrics/report scaffolding.

---

## 7. Data sources, licensing, attribution
- nflverse (no auth, free): PBP `https://github.com/nflverse/nflverse-data/releases/download/pbp/play_by_play_{year}.parquet`; rosters `…/download/rosters/roster_{year}.parquet` (corrected in PR #68); participation `pbp_participation_{year}.parquet`. Participation: per-play `offense_players`/`defense_players` (GSIS ids, 11/side, 100% populated), "free for 2016–2025", **transitioned to FTN Data from 2023**, published **once per year after the postseason, not in-season** (val-suite §15 l.432-442; roadmap/doc 01 l.217). Consequence: RAPM is real for draft-prep/prior-season priors and validation; **live in-season RAPM needs a paid feed**. Supports GRID-Engine alpha-spec §1.2 l.75 (RAPM must not be a hidden dependency of the live path).
- Attribution (roadmap §6.5 #11; doc 01 l.197-198): docs state "nflverse data is CC-BY-SA 4.0; FTN participation (2023+) must credit **'FTN Data via nflverse'**"; required, not polish. **Verify against nflverse's current license statements** when writing `docs/04-providers/nflverse/access-and-license.md` (the CN docs assert CC-BY-SA for all nflverse data; licensing may differ per dataset). The 2016–2022 participation provenance is not documented in CN.
- Not in nflverse: closing odds (Layer 3; no source chosen); CFBD college PBP for feeder SV (`load_cfbd` stub, needs API key); coaching/coordinator changes (hand-curated `backend/db/data/coaching_changes_2025.json`, ~10–20 rows/season); true WR–CB alignment (paid PFF/SIS; NGS alignment columns suggested); game clock for two-minute (`quarter_seconds_remaining`, available in nflverse PBP but not carried in the contract).
- ESPN/Sleeper: league sync, ADP/projections (app; ADP is the only engine-relevant piece, as the market baseline / volume override).
- Research list for weekly participation (val-suite §14 l.423-430): ESPN/NFL GameCenter feeds, NGS endpoints, paid (SportsRadar, Genius Sports, SIS/TruMedia, PFF), community/scraped, or approximate lineups from snap counts + depth charts.

---

## 8. Known traps and lessons learned (engine; consolidate into one doc)
From `docs/04-lessons-learned.md`, `docs/08-phase-task-context.md:117-141`, PR bodies:
1. **Verify the real-data path produces the contract the engine expects**; don't trust a docstring that says "stubs sketched" (lessons l.7-8). `weekly_update` "looked like it ran" because a broad try/except swallowed `KeyError`.
2. Market pseudo-obs sign (G1/V1), duplicated in `layers.py` and `backtest.py.solve_rapm`: **when a bug exists in one path, check every sibling path that duplicates the pattern** (lessons l.15-16). See §14 for the deeper semantic question.
3. Variance of a sum needs covariance: `var_total = H Σ Hᵀ`, `H=[1,1,1]`, not `σ00+σ11`. Appeared in 3 places. **Tests asserting `> 0` do not catch wrong values; assert the expected value** (lessons l.18-19, l.74-75).
4. NaN contracts: `dict.get(k, d)` handles absent, not present-NaN keys (P1); `max(0.0, NaN) == 0.0` (P2); float truthiness `x or 0.0` (G7). Rust analogue: make NaN/None explicit in types (Option/typed errors), consistent with alpha-spec §6.6 rule 4.
5. **Missing-week Kalman input contract**: an absent week must be `played=False` **and** `y=NaN`, because the talent init is `nanmean(y[:3])` and reads `y` without consulting `played`. Seeding absences with 0.0 biased a leading-absence RB's smoothed talent 1.012 → 0.959 (PR #91 round 2).
6. **Filtered vs smoothed aliasing** is the canonical "correct-looking wrong semantic" bug: never use the DB filtered `talent` where RTS-smoothed is meant (PR #78).
7. **Wiring must match the module's own docstring**: ROS ↔ stat-line model, weekly ↔ SV→points map (lessons l.52-53).
8. **The RAPM universe must include all participants**, not only scored players (lessons l.55-56).
9. Canonical column order plus an order fingerprint in persisted accumulators; dimension-only guards silently corrupt on reorder (phase4 T1).
10. Thread determinism: GBM diverges ~1e-2 multi-threaded; set `OMP_NUM_THREADS=1` + `threadpool_limits(1)` for exact tests (doc 08 l.140-141).
11. Column semantics: verify against the nflverse data dictionary (`fumble` vs `fumble_lost`; lessons l.21-22, though the actual pipeline aliased correctly, so the A1 finding was a false positive).
12. Defense-in-depth guards must cover the edge they are needed for (watermark at season seams, V2).
13. Process: subagent audit findings are self-reports, so verify before acting (lessons l.71-72). Several audit CRITICALs were false positives (A1, G2). Read tests to distinguish bugs from intentional design.
14. Identifiability caveat (phase4 Known Constraints l.709): talent vs scheme_fit is weakly identified under additive `H=[1,1,1]`; mitigated by `q_scheme ≪ q_form` and moving scheme_fit materially only on explicit coaching resets; "validate smoothed scheme_fit on synth before trusting it."
15. Situation RAPM is exactly as blocked as base RAPM without participation; masking on state does not remove the requirement (phase4 Known Constraints l.702).

---

## 9. Risks (engine subset; roadmap §7 l.311-331)
- GRID may fail H1. This is "a feature of the plan, not a failure"; honest fallback. **Realized**: tie.
- Volume/role is the dominant driver and GRID doesn't model it; the volume model plus override hook is the mitigation. **Realized**: ROS volume-dominated.
- nflverse schema drift on first real-data contact; front-load it.
- The two-path leakage guard depends on `weekly_update` running on real data (now resolved).
- In-season RAPM staleness: draft-time RAPM (prior-season participation) doesn't refresh weekly and must be presented alongside the weekly-updating Kalman.
- Biggest residual risk (doc 10 l.134): the audit was static, and a real-data backtest sniff may surface more correctness issues.

---

## 10. Open defects relevant to the engine (from `docs/06-issues-log.md`, re-triaged against code/docs)
- **Live in code @59bce1d**: G1 `layers.py:404-405` / V1 `backtest.py:108-109` (market row `[+1,+1]`; semantics open, §14); G3 `synth.py:292` `_team_strength` excludes CB-relabeled players; G4 RTS `Ps[w]` PSD not guaranteed (`statespace.py:206-209`); G6 bare except in `layer1_all_qbs`; G8 priors divide-by-zero; G9 cache never deletes expired files / `ttl=0` boundary; G10 row-wise `df.apply` for `td_type`; G12/G14 minor; P3–P5/P7 projection NaN/missing-column crashes; V2 cross-season watermark gap; V3–V6, V11, V12 metrics edge cases (scalar `normal_cdf`, CRPS NaN at sd=0, NIS var≤0, PINAW zero range, `spearman` 0 vs NaN); V7–V10 sniff/asof/ingest robustness (V10 non-atomic snapshot write); A8 1-D Kalman state not handled in `weekly_update`; A9 re-running an earlier week silently no-ops; #24 `weekly_update` snap counts hardcoded to ones (defeats `R = r_scale/snaps`); #23 priors `league_factor` never applied; #48 single random split for OOS R²; #15 priors not wired into week-1 Kalman cold start; #57 `situation_grades` has no producer + plural/singular naming.
- **Fixed / false positive** (the issues log still lists them as open): P1 (fixed #94); A1, G2 (false positives); #25 matchup-grade mislabel (fixed in Phase-4 T2; sign itself is in question, §14); G5/W12 `var_total` (fixed in #63 for the write path).
- **Enhancement backlog (engine research)**: #32 WR/TE target-share attribution + position-specific lambda; #42 matrix factorization on RAPM residuals; #44 hierarchical Bayesian priors for cross-league equivalency; #45 feature embedding for matchup edges; #46 Bayesian DLM time-varying observation variance; #49 smooth age/draft adjustments (whiteboard: quadratic age by position); #43 copula correlation (app roster context, but relevant to GRID-Engine's Layer F correlated simulation); #31 weekly schedule/opponent table (needed for matchup grades and Tier 2).

---

## 11. Forward plans for the engine (consolidated from all sources)
1. **Close the ROS gap** (phase2c l.120-151): re-run the verdict post-#91 to measure smoothed_talent's effect; wire `prior_mean` on real data (blocked on `priors.py` real wiring + a CFBD feeder SV pass); add the market/ADP baseline (whiteboard: consider promoting market to a gate, since last-season is a weak bar).
2. **Unskip Tier 2**: supply a points-allowed feed. Whiteboard proposal: derive weekly points allowed from nflverse PBP `drive_points` grouped by defense team × week, with no new source.
3. **Real-roster H2**: replace synthetic VOR rosters with real post-draft rosters (draft-day → week-1 window). App/league-sync dependent; may not survive the engine-only pivot.
4. **Weekly in-season participation research** (§7 list) to enable live RAPM.
5. **Filter calibration**: focus-QB NIS 4.6 → ~1 (tune `r_scale`/`d`); calibrate RB/WR/TE `r_scale` now that per-position weekly Layer-1 exists (`statespace.py:126-127` comment).
6. **Changepoint detection wired to production** using Kalman innovations, replacing reliance on hand-curated coaching seeds (whiteboard; Tier 0.5/changepoint KPIs exist in plan only).
7. **Position-specific lambda** calibrated by maximizing per-position OOS recovery on synth (whiteboard; #32).
8. Volume model: ADP as an automated role-change signal; mid-season blending of current-season usage; smooth age curves.
9. Fix G1/V1 with a golden regen on Linux, bundled with a truth-anchored Layer-A test for team-intercept and matchup-grade signs.
10. DEF/K stat aggregation (A7) and a per-week projection producer (A4) if the engine owns weekly outputs (it will under GRID-Engine's weekly scope).

---

## 12. App-only content to leave behind (explicit)
Frontend (all of `docs/03-frontend-spec.md`, design-system Stage 1, phase1 Tasks 9/10/13, phase4 Tasks 0/17–19b); dashboard UX and pages (Dashboard, Rankings UI, DraftRoom, Workbench, InSeason, StartSit, WaiverWire, TradeCenter); packaging (`docs/09-alpha-release-plan.md` entire; conda-constructor/pywebview/Tauri; zip-serve; Windows Task Scheduler `scripts/setup_scheduler.ps1`; roadmap Phases 3–5; cross-cutting #4 app-upgrade (except the cache-rebuild principle), #10 acceptance script); league sync and adapters (ESPN cookies, Sleeper, `League` abstraction, `sync_leagues`, `sync_trade_history`, live-draft WebSocket); draft tools (phase2 plan entire; mock draft except `snake_order`, draft grade); trades (`trade_model` RSV/scarcity/roster-context/trajectory, finder, analyzer, regret tracker; these *consume* Kalman variance/trajectory but are not engine); viz agent (phase4 Tasks 13–16, Task 14 report, ChartSpec, `viz_query_cache`, Anthropic model config); FastAPI routes and the SQLite app schema (`leagues`, `rosters`, `draft_history`, `saved_views`, `viz_query_cache`, `scoring_formats` registry, A3); PostHog analytics (#93); audit sections W1–W20, A2–A6, A10–A18, F1–F8; `docs/05` repo/workstation notes; execution handoff; capacity/IED sizing.
Note: the app tables `kalman_trajectory`, `matchup_grades`, `situation_grades`, `valuations`, `projections` are **engine output sinks**. GRID-Engine should define engine output contracts instead of porting these SQLite tables verbatim.

---

## 13. Content dropped by the PR #92 consolidation (exists only in originals or commit log)
Verified by diffing the originals against docs 01–08 (and 09/10):
- Roadmap §4.5 **projection-architecture rationale** ("GRID cannot be the projection…", the H1 implication (a)/(b)/(c), the structural formula). Doc 02 keeps module descriptions only.
- Roadmap §5 Phase-2 note **"H1 is a floor, not proof; success ≠ GRID wins"**.
- Roadmap §7.5 detail: the three-budget distinction rationale, ~126 GBM refits, sampled H2 bootstrap with logged caps, ingest peak RAM 1–2 GB, weekly-update target, fast-iteration mode.
- Roadmap Phase 0/1 done-when specifics: `_write_kalman_trajectory` fix, `KalmanState.save/load` signature change, cross-platform ~1e-6 note, **participation ≥99% coverage gate**, per-season row-count bounds, baseline-measurement task contents.
- Roadmap risks: nflverse schema drift, two-path guard dependency; validation ⟷ product interleave table.
- Val-suite §5 target choice (opportunity target, DNP scored two ways); §6 Layer-A examples, determinism hazards list, update workflow and anti-gaming backstop; §7 conditional vs unconditional calibration, PIT shape reading, aggregation escape, pinball floor/ceiling, regime-conditional calibration; §8 three axes detail, poisoning mechanism, scope-leak poison checks, information timestamp per source, two-path guard; §9 **Tier-0 injury detection and prior-equivalency KPIs, Tier-0.5 PIT, Tier-1 accuracy context, tier accuracy, H2 starter-OUT and win-rate KPIs, PICP@50, pinball, PINAW, all regime-mechanism KPIs (incl. changepoint P/R and false-positive rate)**; §10 adapter column list; §11 proposed file layout; §12 build order; §13 sign-off; §14 weekly-participation candidate list; §15 `load_participation` mechanics (split on `;`, join game+play_id).
- Phase2c: ALL-row table (vs persistence +0.957, **vs season-to-date −0.194**), n=7035, run labels, first-run H2 −0.524, frozen gate values (+0.0564/+0.3164), the sign-flip-null mechanism, "registry ships empty by contract", DB/P rows kept deliberately.
- Design spec §5 per-phase engine evolution (incremental pipeline flow, "value model stays fixed within a season", priors expansion: draft capital/age, FBS/FCS/UFL); §3 original contract names.
- Phase-4 plan engine **parameters and procedures** (all of §2.3's Kalman/situation/interaction/changepoint numbers, npz migration rules, Known Constraints table). Doc 02 l.257-271 keeps only a 10-line summary.
- **Whiteboard (pre-clear)**: all engine ideas (§11 items 2, 6, 7, 8 and the V2 fix).
- Commit-log-only: adapter semantics (TD 7/FG 3, `n_*=−1`, skill-only `off_players` incl. FB), real-2023 smoke numbers, roster URL fix, QB `r_scale` calibration numbers, the missing-week NaN contract, verdict defaults (STANDARD, 8 teams/rounds, warm-up 4, n=10000).

---

## 14. Doc-vs-code discrepancies and errors (must not propagate)
1. `docs/02-backend-spec.md:273-282` Tier-0 table = aspirational, not the implemented gates (§3.6).
2. `docs/02-backend-spec.md:311-316`: "+0.957 vs persistence" attributed to TE (it is the ALL row); the ALL vs season-to-date loss is omitted.
3. `docs/02-backend-spec.md:81` lists situations `goal_line, third_and_long, fourth_down` that do not exist in `situations.py`.
4. `docs/02-backend-spec.md:73` conflates the stats contract with the engine `plays` contract (§2.2).
5. `docs/05-current-state.md:29` "Cross-league priors … wired to real data (Phase 2b)": true for `estimate_equivalency` (PR #82), but `prior_mean` is 0.0 in the verdict path and priors are not in `weekly_update` cold start (#15). There is no real feeder-SV source (`load_cfbd` stub).
6. `docs/10-next-steps-plan.md:73-75` "pre-existing failures": not reproducible on Linux (637/637 pass). Windows/cross-platform artifacts. The claim that #80 drifted the golden ~1e-2 is not borne out on the platform of record.
7. Market sign (G1/V1), deeper than the docs state:
   - Design-matrix signs make a tough defense's intercept *positive*: column = −1, and a tough defense lowers dV.
   - Synth defines planted team strength as `mean(off ability) − mean(def ability)` (`synth.py:285-294`), so `beta_off − beta_def` and a `[+1,−1]` market row are self-consistent **with synth's definition**.
   - A real closing-line strength measures overall quality, which under these signs is `beta_off + beta_def`. The "known trap" assertion is therefore only correct relative to synth's chosen semantics.
   - Matchup grade = `−beta[t_def]`, "higher = tougher" (`weekly_update`, `backtest.OriginResult.def_grades`). It is tested only on hand-built betas that assume elite defense ⇒ negative beta.
   - My synth probe (direct `run_rapm`, no market): corr(β_def, planted mean DEF ability) = **−0.22** over 12 teams. Collinearity between team intercepts and always-on starters makes intercepts weakly identified, so this is inconclusive.
   - With market rows: `[+1,+1]` gives corr(bo−bd, planted) 0.696; `[+1,−1]` gives 0.758. That modestly supports the doc's fix *for synth semantics*.
   - Action: define "team strength" and intercept semantics explicitly in the engine spec, and add a Layer-A truth-anchored test for both signs before porting.
8. `thresholds.py` docstring (gates committed once frozen) vs the empty-registry contract test and the phase2c record (gates per-run in the report).
9. Scheme reset vs intervention (resolved from code): **both** apply. An intervention spikes the discount (`d_spike`) **and** adds `scheme_reset_var` to `P[2,2]` (`statespace.py:197-203` in `kalman_two_component`, `:376` in `kalman_step`). An explicit `scheme_resets` entry (coaching change) zeroes `mu[i,2]`, sets `sigma[i,2,2]=scheme_reset_var` and zeroes its cross-covariances without touching talent/form (`:381-387`). Phase-4 plan T10's "distinct from interventions" holds only in that direction; the Rust model spec must encode both.
10. `data_adapters.py` module docstring still says "real loaders below are stubs". Only `load_participation` is real. The real PBP path is `nflverse_adapter.load_grid_plays`, not `load_nflfastr_pbp` (still `NotImplementedError`).

---

## 15. Mapping to GRID-Engine (where this content should land)
GRID-Engine context: `alpha-spec.md` (weekly-only, three-season rule §2.4, Layers A–F §6.1, model specs required in `docs/model-specs/` per §6.6, template `docs/99-templates/template-model-spec.md`), provider contracts in `docs/04-providers/<provider>/` (nflverse README is "not yet written"; P1-03), ADRs in `docs/02-adr/`.

Recommended placement:
- `docs/archive/cautious-nevermore/` (verbatim, read-only, with a README giving source repo, commit `165ccde`/`c33712e`/`1aeb7ed^`, and a "superseded by" pointer). Archive: val-suite, roadmap-to-alpha, phase2c verdict, design spec, phase4 plan, whiteboard pre-clear, Phase-4 SDD reports T1/T5/T6/T7/T8. Optionally `CLAUDE.md` (engine half) and `docs/04`/`docs/08` as context. Add a banner warning that app sections are out of scope and that doc-02 numbers in §14 are wrong. **Do not archive** docs 03, 05, 09, the phase1/phase2 plans, the handoff, or the SDD task-14 report (app/process; doc 05 has personal workstation paths).
- `docs/model-specs/`, one per GRID component, filled from §2.3–§2.5 with **observed parity values** from §3.6: `situational-value.md` (V(s)), `rapm-layer2.md` (+ Layer 3, with the open sign decision), `event-credit-layer1.md`, `statespace-kalman.md` (3-state, SSParams, missing-week contract, predictive S, npz versioning), `cross-league-priors.md`, `situations.md`, `matchup-interactions.md` (flag approximate), `changepoint.md`, `volume.md`, `stat-line.md`, `sv-to-points.md`, `scoring.md`.
- A validation spec (e.g. `docs/03-validation/` or a new top-level spec), rewritten from val-suite §§2–9 + roadmap §7.5 + the implemented gates. Keep calibrate-then-gate, three leakage axes, the GRID-specific leaks, the three-layer golden master, and the two-budget CI vs nightly split.
- `docs/04-providers/nflverse/`: URLs, participation mechanics, FTN attribution (verify license), cadence (once after postseason), schema gotchas (gsis_id, `nflverse_game_id`, float/int `play_id`, `fumble_lost`), coverage gate ≥99%. Add a `docs/04-providers/odds/` placeholder (Layer 3 has no source) and use the existing `cfbd` for feeder SV.
- ADRs: (a) Python reference as oracle, with parity tolerances; (b) Layer-3 / intercept sign semantics; (c) which Tier-0 numbers gate the Rust port; (d) RAPM's role given no in-season participation (aligns with alpha-spec §1.2); (e) ROS horizon status vs weekly-only alpha-spec.
- Rust parity strategy (recommendation): sklearn `HistGradientBoostingRegressor` (V(s), Layer-1 context model) cannot be bit-matched in Rust. Make Layer-A/B invariants and Tier-0 recovery correlations the primary parity gates for GBM-dependent stages. Use Layer-C `rtol 1e-5` only for closed-form stages (ridge/RAPM solve, Kalman/RTS, affine maps, metrics) fed with **Python-exported fixtures of dV / weekly credit**, so GBM differences don't cascade.
- Reconciliation notes vs alpha-spec:
  - CN Half-PPR = alpha-spec §2.3, but the CN verdict ran in STANDARD.
  - CN data window 2019–2025 vs the alpha-spec three-season rule (CN's frozen pre-period V(s) and prior-season talent features already fit a lookback model).
  - CN volume × efficiency maps to alpha-spec Layer C (opportunity) × Layer D (efficiency).
  - CN's verdict that ROS is volume-dominated supports the alpha-spec's opportunity-first decomposition.
  - CN calibration (one-step predictive S, PICP/PINAW/CRPS/pinball) maps to alpha-spec §5.3 distribution outputs.
  - CN's AsOf information-time cutoff = alpha-spec §4.5 timestamps.
  - CN priors (feeder-SV equivalency, wide prior → fast washout) map to alpha-spec §6.3 (EB posterior with n0).
  - CN's "honest fallback" and H1-is-a-floor discipline map to alpha-spec §3.2 claim discipline.

## 16. Python reference import notes (from docs + spot checks)
- Engine package set: `backend/grid`, `backend/projection`, `backend/validation`, `backend/scoring` (`engine`, `formats`, `columns`, `vor`; **not** `format_registry`, which is SQLite/app), parts of `backend/pipeline` (`ingest_grid`, `_logging`; `weekly_update` is the production forward path the two-path guard needs, but it is DB-coupled). Tests: `tests/grid`, `tests/validation`, `tests/projection`, `tests/scoring` (subset), `tests/pipeline/test_weekly_update.py`, `tests/pipeline/test_weekly_situations.py` (DB-coupled).
- Couplings to vendor or cut: `lineup_sim → services.mock_draft.snake_order` (9 lines); `verdict → backend.db.connection.get_db` + SQLite `player_stats` (from `data_pipeline`); `backtest → pipeline._logging`; `weekly_update` → SQLite `matchup_grades`/`kalman_trajectory`/`coaching_changes` + `db/data/coaching_changes_2025.json`.
- Golden files: `tests/grid/golden/snapshot.npz` (Linux-valid at `59bce1d`); `backend/validation/provisional_thresholds.json` (empty by contract).
- Determinism env for the oracle: `OMP_NUM_THREADS=1`, `threadpool_limits(1)`, pinned library versions (CN `requirements.txt` uses `>=`; pytest not listed).
