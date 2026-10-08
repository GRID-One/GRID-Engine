# Reconciliation (code-first): working Python GRID engine ↔ GRID-Engine specs

**Author:** inventory agent `reconcile-code-first`. **Date:** 2026-10-01.
**Sources (read-only):** `/home/user/cautious-nevermore` @ `59bce1d` (main), and `/home/user/GRID-Engine` @ `3823478` (branch `claude/grid-engine-consolidation-e7kmh9`). Deleted cautious-nevermore docs were read from history (`git show 5f282cb^:<path>`): the dashboard design spec, `2026-06-23-retrospective-validation-suite.md` (the "validation plan"), `2026-06-23-product-roadmap-to-alpha.md` (the "roadmap"), and `2026-07-09-phase2c-verdict-and-ros-gap.md`.
**Scratch:** `/tmp/claude-0/-home-user/693e74a1-f8af-5256-86e9-2299b8697223/scratchpad/inventory/scratch-reconcile-code-first/` (`cn/` = `git archive` copy, `sign_check.py`, `olddocs/`).

## How I checked this

- I ran `OMP_NUM_THREADS=1 python3 -m pytest tests/grid tests/projection tests/validation tests/scoring` on a `git archive` copy: **377 passed in 138 s.**
- I recomputed every Tier-0 recovery metric live on the canonical synthetic data (§A.2). Each one matches the "observed" comment in `tests/grid/test_tier0_recovery.py`.
- I ran two scripts of my own:
  - `sign_check.py` tests the Layer-3 market sign and the matchup-grade sign (§6, C12/C13).
  - A batch-vs-incremental Kalman comparison (§6, C15).
- Spec line numbers below refer to `alpha-spec.md` (AS) and `final-build-spec.md` (FBS) in GRID-Engine.

---

## 0. Executive summary (read this first)

1. **The Python engine is real, tested and partly validated on real data.** Its parts are:
   - situational value V(s) and dV credit;
   - RAPM over participation, with team intercepts and market pseudo-observations;
   - cross-fitted Layer-1 weekly credit;
   - a 3-state Kalman filter (`[talent, form, scheme_fit]`) with discounting, rust, scheme resets, RTS smoothing and changepoints;
   - feeder→NFL equivalency priors;
   - the synthetic planted-truth contract;
   - a leakage-safe walk-forward validation stack.

   **None of these exist in the GRID-Engine specs as named components.** The specs describe a generic six-layer fantasy-projection system (Layers A–F) and a basketball-flavoured RAPM. The consolidated spec must add the GRID components explicitly (§5).

2. **The biggest conflict is that the specs ban a live dependence on participation, and RAPM needs it.**
   - The ban: AS:75 (§1.2), AS:249 (§4.1), AS:631 (§6.2), AS:1784 (§12.3), AS:1940 (§15).
   - Python's own docstring calls participation "the linchpin of Layer 2 RAPM" (`data_adapters.py`). The FTN participation feed is published only after the postseason.
   - **So the only significantly positive real-data result, H2 = +0.848 [+0.232, +1.466], used current-season participation. That is unavailable when the system runs live: train/serve skew.** It must be re-measured under live-parity constraints before anyone cites it (C1).

3. **Real-data performance does not yet meet the spec's Phase-1 model-quality gate (AS §9.4, AS:1476).** On 2022–2023 nflverse data (70,778 plays):

   | Comparison (ROS margin) | Result | Reading |
   |---|---|---|
   | vs last-season actuals (H1 kill criterion) | −0.015 [−0.059, +0.029] | tie → `BASELINE_FALLBACK` |
   | vs season-to-date mean | −0.194 [−0.270, −0.120] | GRID loses |
   | vs persistence | +0.957 | GRID wins |

   - Season-to-date mean is one of the spec's naive baselines, so this fails "beats every naive baseline".
   - Calibration is reasonable: weekly NIS 1.00, ROS NIS 1.25, PICP@80 between 0.75 and 0.87.
   - Source: `docs/05-current-state.md` and the phase-2c doc.

4. **Five latent defects must not be ported verbatim** (details in §6):
   - **C12 Layer-3 sign convention.** Planted truth, estimate and market row are mutually inconsistent. The docs claim a fix the code does not contain.
   - **C13 Matchup-grade sign.** The reasoning in the code comment is wrong, and no truth-anchored test exists.
   - **C14 Production Kalman observations.** `weekly_update` feeds cumulative RAPM ratings as if they were weekly observations, with snaps fixed at 1.
   - **C15 Batch vs incremental Kalman.** The two disagree whenever an intervention fires (confirmed numerically).
   - **C16 Kalman initialisation looks ahead.** `x0` is set from the first three observations.

5. **Parity strategy.** sklearn's `HistGradientBoostingRegressor` cannot be matched bit-for-bit in Rust (xgb or linfa), and numpy's RNG streams are not reproducible natively. So parity must be **stage-wise, with Python fixtures injected upstream** (§7.1):
   - Rust RAPM, Kalman, priors, metrics and walk-forward are fed Python's `dv`, out-of-fold residuals and split indices, and must match to about 1e-8.
   - V(s), the Layer-1 booster and the Rust synth are gated statistically (Tier-0 floors), not numerically.

6. **Crate homes.** Keep these crates: `domain`, `ingestion`, `identity`, `features`, `models`, `simulation`, `scoring`, `evaluation`, `governance`, `persistence`.
   - Drop `ffi`.
   - Replace `application` with a headless `grid-pipeline` (CLI/orchestration).
   - Add `grid-synth` (planted truth plus Python-fixture loader).

   The full module→crate table is in §7.2.

---

## 1. Component inventory (what the Python code actually estimates)

Every hyperparameter below is copied from code. These are the numbers the Rust port and its model specs must reproduce or explicitly change.

### 1.1 `backend/grid/data_adapters.py`: the swap point and data contract

**Contract** (docstring lines 1–22):

- **`plays`**: one row per play with
  - `play_id, drive_id, week, off_team, def_team`
  - state s = `down, ydstogo, yardline_100`
  - outcome = `yards, points, terminal, terminal_value`
  - s′ = `n_down, n_ydstogo, n_yardline_100`
  - the V(s) label `drive_points`
  - `off_players`, `def_players` (tuples)
- **`players`**: `[player_id, team, position, is_starter, (ability if synthetic)]`
- **`market`**: `{team -> closing-line-implied strength}`, used "ONLY as a team-level anchor, never per play"
- **`college`**: `[player_id, position, feeder_sv, feeder_snaps, is_rookie]`

**Loaders:**

- `load_synthetic(cfg)` defaults to `SynthConfig(yards_noise_sd=3.2, drives_per_team_per_game=12)`. This is **not** the dataclass default; the golden master pins it explicitly.
- `REAL_LOADERS`:
  - `pbp`: a stub that raises `NotImplementedError`. The real path is `nflverse_adapter.load_grid_plays`.
  - `participation`: implemented.
  - **`odds`: raises `NotImplementedError`**, so Layer 3 has never run on real data.
  - **`cfbd`: raises `NotImplementedError`**, so there is no real `feeder_sv`.

### 1.2 `backend/grid/synth.py`: the planted-ground-truth generator

**Rosters and on-field personnel**

- Per team: QB 2, RB 3, WR 5, TE 2, DEF 12, for 24 players per team. The default 12 teams give **288 players**.
- On field: offense QB 1, RB 1, WR 2, TE 1 (OL is a nuisance and is never on the sheet); 7 defenders.
- Ability: `N(0, ABILITY_SD[pos])` with SDs QB 0.090, RB 0.030, WR 0.028, TE 0.022, DEF 0.022.
  - Starters get `+STARTER_BONUS`: QB 0.10, RB 0.03, WR 0.03, TE 0.02, DEF 0.02.
  - Backups get −0.5 × the bonus.
- Rotation:
  - An offensive starter is on the field with probability 0.80 per slot.
  - The defensive base 7 play 70% of snaps; otherwise 7 are drawn at random from the 12.

**Play mechanics**

- Yards per play = `26.0 × (Σoff_abil − Σdef_abil) + N(4.0, yards_noise_sd)`, rounded and clipped to [−8, 60].
- Drives start at `clip(N(75, 8), 60, 95)` yards to goal.
- Outcomes: touchdown = 7; turnover on downs inside the 35 → field goal with p = 0.82, scoring 3, otherwise 0; a drive is cut at 12 plays and scores 0.
- `drive_points` is broadcast to every play in the drive. Terminal plays get `n_* = −1`.
- Schedule: random pairing each week (14 weeks, round-robin by shuffle).
- Canonical size: **16,825 plays**.

**Focus QB (team 0 starter)**

- Ability rises as `0.05 + 0.025·wk` through week 7.
- He misses weeks 8–9 (the backup plays).
- He returns at 0.06 in week 10, then recovers linearly to the peak over 3 weeks.

**Other truth outputs**

- `league_factor = 0.62`.
- `make_college`: `feeder_sv = 0.62·ability + N(0, 0.030)`, feeder snaps drawn from [250, 900), 40% of players are rookies. Seed = `cfg.seed + 99`.
- `cb_split` relabels the first 2 DEF players of each team as "CB". **It plants no WR–CB effect.**

**Defect.** `_team_strength = mean(off starter ability) − mean(DEF starter ability)` (`synth.py:293`). A good defence has *high* ability, because it subtracts from the offence. So this "strength" ranks strong defences *lower*. See C12.

### 1.3 `backend/grid/value.py`: Situational Value

**Estimand:** V(s) = E[drive_points | down, ydstogo, yardline_100], with features `STATE_COLS` only. There is no clock, score or timeouts.

**Model:** `HistGradientBoostingRegressor(max_depth=4, learning_rate=0.08, max_iter=300, min_samples_leaf=120, random_state)`, optionally persisted with joblib.

**Play value:**
- dV = V(s′) − V(s).
- On a terminal play, V(s′) is the realised `terminal_value` (7/3/0).

**Statistical intent (load-bearing docstring).** "Deliberately a transparent, standard expected-points scaffold — the originality in GRID is in attribution (layers) and the dynamic layer."

**Canonical synth values.**
- dV mean 0.0102, sd 1.1553.
- V at 1st-and-10 for yardline 90/60/30/10 = **[1.117, 1.840, 3.375, 5.575]**.

### 1.4 `backend/grid/layers.py`: attribution (Layers 2, 3 and 1, plus the fixed point)

**Layer 2: RAPM design (`build_design`, lines 226–344)**

- Columns are `[players sorted by str(id)…, team_off…, team_def…]`. Teams are `sorted(players.team.unique())`, which is numeric for synth ints and lexicographic for codes.
- Each play row has:
  - +1 for each offensive player on the field, −1 for each defender;
  - +1 at `t_off[off_team]` and −1 at `t_def[def_team]`.
- The response is `y = dv`.
- Unknown IDs are dropped with a warning.
- With `interactions=True`, WR×CB-proxy pair columns are appended. Pairs seen fewer than `min_pair_plays=30` times are pruned. The proxy is CB if present, else DEF.

**Layer 2: the solve (`run_rapm`, lines 362–428, and `_solve_ridge_prior`, 347–359)**

- Objective: `min ||y − Xβ||² + λ Σ_j m_j (β_j − μ_j)² + w_mkt Σ_t (a_tᵀβ − s_t)²`.
- Parameters:
  - λ = 120.
  - Penalty weights m: players 1 (optionally `lambda_by_pos` multipliers), team intercepts 0.05, interaction columns 10.
  - μ = 0, except prior-mean seeds.
  - w_mkt = 40.
- Solver: dense normal equations, `np.linalg.solve`. If `cond(A) > 1e10` it falls back to `lstsq` with a warning; that is a silent fallback (see C11).
- Incremental path: `init_accumulators`, `accumulate` (dense XtX and Xty), `fit_from_accumulators`, and `save/load_accumulators`.
  - Accumulators are stored as `.npz` with `week` and `player_order` (string array, `allow_pickle=False`). The cache path can be injected.
  - A per-situation key writes `rapm_accumulators_{key}.npz`.

**Layer 3: market rows (lines 400–408)**

- Row `a_t` has **+1 at t_off and +1 at t_def**, with target `market_strength[t]`.
- Reported `team_rating = β[t_off] − β[t_def]` (line 426). See C12.
- Statistical intent: "anchors the otherwise free team-level constant and is a falsifiable hook (does bottom-up player value reconstruct the market?)".

**Situation RAPM (`run_situation_rapm`)**

- Runs a separate `run_rapm` per mask from `situations.classify`.
- Requires the participation columns; skips a situation with fewer than `min_plays=200` plays.
- A player absent from a situation gets the ridge-prior 0, which callers must label "no data".

**Layer 1: weekly credit (lines 487–587)**

- The context model g(state, Σ on-field defender ratings) is an HGB with `max_depth=3, lr=0.1, max_iter=200, min_samples_leaf=150`.
- It is cross-fitted with `KFold(5, shuffle=True, random_state=0)` at **play level**, **once per frame**, and is position-agnostic.
- residual = dv − g_oof.
- A player's weekly credit is the mean residual over that player's on-field offensive plays. `snaps` is the count of those plays.
- `layer1_all_players` uses `min_plays=20`. `layer1_all_qbs` swallows exceptions (G6).

**Fixed point (`fit`, lines 591–610)**

- `n_iter=3` loops of: RAPM → ratings lookup → Layer-1 QB credit (opponent adjustment) → the **focus QB's** prior mean becomes his season-mean credit.
- Only the focus QB is re-seeded (G14). The credit's scale (per-play residual) differs from the scale of the RAPM coefficient.

### 1.5 `backend/grid/statespace.py`: the dynamic layer

**State and model**

- State x = [τ talent, f form, s scheme_fit]; F = diag(1, φ = 0.50, φ_s = 0.985); H = [1, 1, 1].
- The observation y_w is weekly credit. R = r_mult · r_scale / max(snaps, 1).

**Predict step**

- `P ← F P Fᵀ`.
- `P[0,0] /= d` (a West–Harrison discount on the **talent variance only**; see C17).
- `P[1,1] += q_form` (0.0008) and `P[2,2] += q_scheme` (0.0002).
- At an intervention week:
  - d = d_spike (0.70);
  - `P[1,1] += 5·q_form` and `P[2,2] += scheme_reset_var` (0.04);
  - rust: `r_mult = 2.0` for `post_event_games = 2` played games (batch path).

**Update step**

- Joseph form: K = P Hᵀ / S, with S = H P Hᵀ + R; `P = (I−KH) P (I−KH)ᵀ + K R Kᵀ`.
- A missing week (`played=False` or NaN) predicts only, so variance grows during absences.

**Outputs**

- The one-step predictive `(total_pred = H·xp, var_total_pred = S)` is **the calibration target**. Using `var_total_filt`, which omits R, is called out as wrong.
- RTS smoother: C = Pf Fᵀ (Pp + 1e-10 I)⁻¹. Returns `tau_smooth, total_smooth, var_tau_smooth, sigma_smooth`.
- Interpretation: filtered = "current form / real-time grade"; smoothed = "best retrospective talent".

**Initial state**

- `x0 = [nanmean(y[:3]), 0, 0]` (**look-ahead**; C16) and `P0 = diag(0.05, 0.02, 0.01)`.

**Per-position `SSParams`**

| Position | d_steady | r_scale |
|---|---|---|
| QB | 0.95 | 0.55 |
| RB | 0.85 | 0.45 |
| WR | 0.90 | 0.40 |
| TE | 0.90 | 0.40 |
| DEF | 0.95 | 0.55 |
| default | 0.90 | 0.40 |

Only QB's `r_scale` has been calibrated: 0.55 gives pooled NIS ≈ 1.24, median per-QB NIS ≈ 0.93 and PICP@80 ≈ 0.78. **Real nflverse defensive positions (CB/LB/DL/S) fall through to the default.**

**`kalman_step` (incremental, vectorised over players)**

- Same equations, plus `scheme_resets`. A reset zeroes μ[2], sets Σ[2,2] = 0.04 and zeroes Σ's cross-covariances with scheme_fit. This happens after predict and before update, and is distinct from an intervention.
- Rust inflation is applied **only in the intervention week**. It differs from the batch path (C15).
- With `return_pred`, it returns the per-player S.

**`detect_changepoints`**

- z = (obs − H F μ) / √S at steady-state parameters; flags |z| > 3.0.
- CUSUM is reserved but not implemented.

**State persistence**

- `KalmanState` is saved to `.npz` with `state_version = 3`. A 2-wide legacy file migrates to 3-wide; any other width returns None. The path can be injected.

### 1.6 `backend/grid/priors.py`: cross-league priors

**`estimate_equivalency`**

- OLS of the NFL `rating` on `feeder_sv` over shared non-rookie players.
- OOS R² comes from a single 70/30 split that uses `default_rng(3).permutation` (issue #48).
- Returns slope, intercept, `oos_r2`, `factor_check` (synth only), `n_shared` and `league_factor`.
- The league factor comes from `LEAGUE_FACTORS`: FBS 0.35, FCS 0.20, UFL/USFL/XFL 0.15, CFL 0.18, unknown 0.25. **It is returned but never applied** (issue #23).
- Canonical synth: slope 1.3159, intercept 0.0043, oos_r2 0.1489, n_shared 183, factor_check 0.678 against a planted 0.62.

**`build_priors`**

- `prior_mean = intercept + slope·feeder_sv`, plus optional step functions:
  - age: < 24 → +0.05; > 30 → −0.05;
  - draft round: 1 → +0.08; 2–3 → +0.03; undrafted → −0.03.
- `prior_sd = PRIOR_SD[pos]`: QB 0.16, RB 0.07, WR 0.08, TE 0.06, DEF 0.07, otherwise 0.10.

**`washout_table`**

- Prior weight after k games = (1/P0) / (1/P0 + k/R).
- This is the static, conjugate form of AS §6.3's `n0 = R/P0` (see E5).
- On synth with R = 0.40/90, the prior falls below 50% weight after 1 game for QB/RB/WR/DEF and after 2 for TE.

**Statistical intent.** "Shared-unit identification… range-restricted… wide prior → high early Kalman gain → washes out fast."

**Not wired.** The priors never reach Kalman `x0/P0` (issue #15). On real data `prior_mean` enters the ROS model as a constant 0.0.

### 1.7 `backend/grid/situations.py`

Masks:

- `red_zone`: `yardline_100 ≤ 20`.
- `passing_downs`: (down 3 and ydstogo ≥ 7) or down 4.
- `rushing_downs`: down ≤ 2 and ydstogo ≤ 4.
- `two_minute`: `quarter_seconds_remaining ≤ 120`, **only if that column exists**. The real adapter does not emit it.

`docs/02-backend-spec.md` also lists goal_line, third_and_long and fourth_down. **Those are not in the code** (C24).

### 1.8 `nflverse_loader.py` and `nflverse_adapter.py`: the real-data plays contract

**Loader**

- Downloads nflverse release parquets: `play_by_play_{y}`, `roster_{y}` and `pbp_participation_{y}` (FTN, 2016–2025, CC-BY-SA, published after the postseason).
- Caches them with `ParquetCache` (mtime TTL 168 h).
- The roster maps `gsis_id` to `player_id`.

**Adapter (`build_plays_contract`)**

- Keeps pass and run plays with a non-null down.
- `drive_id = game_id_fixed_drive`. `drive_points` = {Touchdown: 7, Field goal: 3, everything else: 0} (**safeties and defensive TDs score 0**; C23).
- The terminal row is the last modelled play of the drive. `n_*` comes from shifting within the drive.
- The participation join on (game_id, play_id) normalises int/float/str keys:
  - offensive players are filtered to {QB, RB, WR, TE, FB} through the roster (OL goes into the intercept);
  - defenders are kept as listed (all 11);
  - NA cells become `()`.
- Verdict data: 2022–2023, 70,778 plays, 100% participation coverage.

### 1.9 `backend/projection/*`: volume × efficiency (roadmap §4.5)

**Statistical intent.** "Volume is sticky; GRID measures efficiency only, so GRID enters as a feature, not as the projection."

- **`volume.project_volume`** (empirical Bayes):
  - Per-game usage = w·own + (1−w)·position mean, with w = g/(g+8), where g = games in the most recent *prior* season. `k_shrink=8` is fixed, not learned.
  - Columns: `VOLUME_COLS = [pass_attempts, rush_attempts, targets, receptions]`.
  - Has a manual override hook. Information is cut off as of W.
- **`model.StatLineModel`** (Layer-D seed):
  - Per position, and per (driver, stat) pair:
    - driver `pass_attempts`: completions, passing_yards, passing_tds, interceptions;
    - driver `rush_attempts`: rushing_yards, rushing_tds;
    - driver `receptions`: receiving_yards, receiving_tds.
  - The rate stat/driver is regressed with sklearn `Ridge(alpha=1)` on **standardised** `[rapm_rating, smoothed_talent, prior_mean]`. NaN is treated as 0.
  - Output = max(0, volume × rate).
  - **`fumbles_lost` and `two_point_conversions` are fixed at 0.0.**
- **`sv_to_points.SVToPointsMap`**: a per-position affine fit `points ≈ a + b·credit` (`np.polyfit` degree 1). It is fitted on data as of W, with `min_rows=10` and skipped when the spread is zero. It is the "weekly injury-replacement lever".
- **`features.assemble_talent_features`**: pure assembly that strictly separates *filtered* from *smoothed* talent ("the correct-looking-wrong-semantic bug the golden master exists to catch").
- **`preseason.project_preseason`**: per-game line × 17 games.

### 1.10 `backend/scoring/engine.py` (with `columns.py` and `formats.py`)

- `ScoringConfig{name, rules: {stat: multiplier}}`; `calculate_points = Σ stats[k]·rules[k]`. It is **linear with no offset and not versioned**.
- 14 `SCORING_STAT_COLS`.
- Presets:
  - base: pass yd 0.04, pass TD 4, INT −2, rush/rec yd 0.1, rush/rec TD 6, fumble lost −2, 2PT 2;
  - receptions: 0 (Standard), 0.5 (Half PPR), 1.0 (Full PPR).
- `vor.py` and `format_registry.py` are app-side (VOR ranks and SQLite).

### 1.11 `backend/validation/*`: the trust layer

- **`asof.AsOf(season, week)`**
  - `outcome_cutoff = W−1`, `pregame_cutoff = W`; season-aware tuple mask.
  - Slices: `slice_plays`, `slice_pool`, `slice_market`, `slice_interventions` (an undated intervention raises `LeakageError`), and `assert_watermark`.
  - `TripwireFrame`.
  - `CacheNamespace` refuses any root that overlaps `data/cache`.
  - These cover the three leakage axes in validation-plan §8: temporal, scope and state.
- **`backtest.walk_forward`**
  - V(s) is **frozen** on the first `warmup_weeks=4` slots.
  - Accumulators are incremental per week. At each origin it solves first, then folds the week in.
  - The market is static. `def_grades = −β[t_def]`.
  - The watermark is checked only within a season (V2).
- **`baselines`**: persistence, season-to-date mean (with a pooled-sd fallback for spread), last season, market (report only). All are per-game.
- **`metrics`** (numpy only, closed form):
  - accuracy: MAE, RMSE, bias, `skill = 1 − model/ref`;
  - `bootstrap_ci`: percentile, n = 10,000, **iid resampling**, seed 0, plus an `excludes_zero` flag;
  - calibration: NIS, PIT, Gaussian-closed-form CRPS, PICP and PINAW (always paired), pinball;
  - ranking: Spearman (tie-aware), top-N hit rate, NDCG@k;
  - Acklam `normal_ppf`.
- **`tier1`**
  - Weekly forecast = `sv_map(rating)`. ROS forecast = `StatLineModel(volume_asof, talent)` scored with STANDARD.
  - Reports the paired margin |baseline err| − |GRID err| with a bootstrap.
  - Calibration uses an expanding pool of past errors only, needing at least 20 errors.
  - **Players with no realised row are dropped** (`tier1.py:259`; C7).
- **`tier2`**: the KPI is the mean over weeks of −Spearman(grade, points allowed), with a bootstrap and `min_teams=6`.
- **`lineup_sim`** (H2)
  - Rosters come from a deterministic VOR-greedy snake draft. One greedy `set_lineup` rule is used for both methods.
  - Margin = realised(A lineup) − realised(B lineup), reported overall and for the starter-OUT subset; win rate counts ties as ½.
  - It imports `backend.services.mock_draft.snake_order` and `scoring.vor` (app modules).
- **`thresholds`** (calibrate-then-gate)
  - Gate = mean + 1.959964·SE of a **sign-flip null**.
  - Freeze-once; only `force` overrides.
  - The committed `provisional_thresholds.json` is `{"kpis": {}}`.
- **`verdict.run_verdict`** stitches it all together. The H1 kill criterion is ROS vs last season, giving `SHIP_GRID`, `BASELINE_FALLBACK` or `INCONCLUSIVE`.
  - `_smoothed_talent_pre_first_origin` runs the RTS smoother over weekly Layer-1 credit, frozen before the first origin. A missing week is NaN plus `played=False`.
- **`sniff.check_rank_bands`**: at least 3 elites in the top 5, and none ranked outside the top 24.

### 1.12 `backend/pipeline/weekly_update.py`: the production forward path (engine-relevant defects)

- It accumulates the week and solves RAPM with market rows [+1,+1] using `market.get(t, 0.0)`. That **anchors teams missing from the market to 0**, whereas `backtest.solve_rapm` skips them.
- **The Kalman observation is the cumulative season-to-date RAPM rating** (line 307) with `snaps = np.ones` (line 309, issue #24). The validated path instead uses weekly Layer-1 credit with real snap counts (C14).
- **V(s) is refitted each run on `all_plays` of the season** (line 213). That violates the frozen-V(s) discipline used in validation.
- Per-position `kalman_step` with automatic changepoints, scheme resets from the `coaching_changes` table, and situation passes with `MIN_PLAYS=50`.
- Matchup grade = `−β[t_def]`, with a comment whose sign reasoning is wrong (lines 65 and 84; C13).

---

## 2. What validates each component, and real-data status

| Component | Gate / test (cautious-nevermore) | Canonical value (live, synth) | Real data (docs/05, phase-2c doc) |
|---|---|---|---|
| V(s), dV | `test_performance` (cache), `test_nflverse_adapter::test_output_feeds_grid_value_path`, in-process determinism `test_determinism` (dv to 1e-9) | V(1st&10) = [1.117, 1.840, 3.375, 5.575] | Runs on real data; frozen on warm-up in validation |
| RAPM attribution | `test_tier0_recovery`: pooled ≥ 0.77; per position QB ≥ .83, RB ≥ .70, WR ≥ .76, TE ≥ .73, DEF ≥ .73 | 0.8025; .869/.743/.797/.771/.765 | Runs (after fixing the universe in PR #89); weekly H2 relies on it (C1 caveat) |
| Market reconciliation (L3) | `test_team_strength_recovery` ≥ 0.60; golden Layer A "strongest planted team in estimated top 2" | 0.6643 | **Never run** (`load_odds` is a stub; verdict passes market=None) |
| Layer-1 credit | `test_attribution` (all-players == QB path; context model fitted exactly once), golden `qb_credit` | n/a | Feeds `smoothed_talent` (PR #91; re-run not recorded) |
| Incremental RAPM | `test_incremental` (accumulating A+B == fitting on all), leakage `test_two_path_equivalence` (1e-6) | exact | Used by walk-forward |
| Kalman | Tier 0: total_smooth ≥ 0.92 (0.958), tau_smooth ≥ 0.60 (0.6745), variance widens on injury return (0.00228 → 0.00614), focus NIS ≤ 10 (4.581; **overconfident**); `test_calibration_synth` QB pooled with GroupKFold-by-week residuals: median per-QB NIS ∈ [0.8, 1.25], pooled ∈ [0.8, 1.4], PIT mean ± 0.07, PIT sd ∈ [0.24, 0.34], PICP80 ∈ [0.70, 0.90]; `test_kalman_numerical` (Joseph form, PSD); golden Layer A talent is the most persistent component, injury dip; Layer B steepest drop at index 8 | total_filt corr 0.9867 (higher than smoothed) | Weekly NIS(ALL) 1.00, ROS NIS 1.25, PICP80 0.75–0.87 (fantasy-point space, expanding-error sd, not the Kalman S) |
| Priors | Tier 0: slope ∈ [0.9, 1.8], oos_r2 ≥ 0.05, rookie corr ≥ 0.50; `test_priors` (15) | 1.3159 / 0.1489 / 0.5829 | `prior_mean` is a constant 0 on real data (no CFBD) |
| Situations / situation RAPM | `test_situations`, `test_layers_situations` (structure only) | n/a | Only on historical participation |
| WR–CB interactions | `test_design_interactions` (only asserts non-empty and finite; **no planted effect**) | n/a | Not used |
| Changepoints / scheme reset | `test_changepoint` (z-score units; the demo week is recovered by weekly_update), `test_coaching_changes` | n/a | Not validated (validation plan's P/R ≥ 0.6/0.5 target not implemented) |
| Projection | `test_model::test_beats_last_season_baseline_on_synth`, `test_sv_to_points::test_recovers_planted_affine_map`, `test_volume` (10) | n/a | ROS tie vs last season; TE wins |
| Golden master | `tests/grid/golden/snapshot.npz` (38,764 B); Layers A/B/C, rtol 1e-5 / atol 1e-6, single-threaded (`threadpool_limits(1)`) | 288 ratings, 12 teams, 14-week Kalman arrays | n/a |
| Leakage | `test_leakage_guards` (future-poisoning bit-identical plus a canary, watermark plus canary, tripwire plus canary, two-path equivalence), `test_asof` (14) | exact | Inherited by verdict |

---

## 3. Component → spec mapping table

Legend: ✓ implemented and matches · ◐ partial / different form · ✗ absent.

| Python component | Spec location(s) | Status | Note |
|---|---|---|---|
| `value.py` V(s), dV | AS §11.3 "EPA and success per opportunity" (AS:1711); AS §6.2 gradient boosting (AS:634); FBS §11.8 | ◐ | The spec treats EPA as an **input feature**. GRID builds its own EP-like currency and uses it as the RAPM/Kalman response. No spec section names V(s). → E1 |
| `layers.build_design/run_rapm` | AS §6.2 "RAPM-style sparse effects… research-only if live feature parity is absent" (AS:631); FBS §11.3 (FBS:373–405) | ◐ | FBS's objective is plain ridge with basketball controls. Python uses a generalised objective (prior mean, per-column weights, market rows). Live-parity conflict → C1, C11 |
| Layer-3 market rows | AS §11.1 "point spread and total" (feature); AS §4.3 market context | ✗ (as named) | The spec has no reconciliation layer → E3 / C12 |
| Layer-1 cross-fitted credit | AS Layer D (AS:594); AS §6.2 GBM "nonlinear residual correction" | ◐ | A GBM is used here as a *context model for opponent-adjusted credit*, not for residual correction → E2 |
| Fixed point (`fit`) | none | ✗ | → E2 |
| Situation RAPM / `situations.py` | AS §11.2 red-zone, two-minute, third-down usage (AS:1697–1709); AS Layer E | ◐ | Situations exist as masks and slices; the spec wants them as features |
| WR–CB interactions | AS Layer E matchup (AS:608) | ◐ | Synth plants no interaction; nothing validates it |
| Kalman `[τ, f, s]` | AS §6.2 Kalman "pace, pass tendency, opportunity share, selected efficiency" (AS:632); FBS §11.4 | ◐ | Python applies Kalman only to player **efficiency** in dV currency, not pace/share. The spec does not describe the decomposition, discount, rust or scheme reset → E4 |
| RTS smoother | AS §6.2 fixed-lag RTS (AS:633); FBS §11.5 (full and fixed-lag) | ◐ | Full RTS only; **no fixed-lag** |
| `detect_changepoints`, scheme resets | AS §10.2 "injury-return… role-transfer features" (Phase 2) | ◐ | The mechanism exists; no spec text describes it |
| `priors.py` | AS §2.5, §4.2, §6.3 (AS:640–667), §11.5 | ◐ | Both have position-specific prior variance and decay with evidence. Python lacks q, identity confidence, learned n0 and CFBD; the spec lacks feeder-SV equivalency through shared players → E5, C27 |
| `projection/volume.py` | AS Layer C (AS:584), §6.2 empirical Bayes | ◐ | EB shrinkage of per-player usage. **No team-total simplex constraint**, no snap share, no depth chart |
| `projection/model.py` | AS Layer D (rates), §6.2 ridge | ◐ | Rates come from ridge on GRID talent. No TD-specific shrinkage or probability models. fumbles and 2PT fixed at 0 |
| `sv_to_points.py` | AS §5.4 affine (but stat-vector → points) | conflict | Maps credit straight to points, bypassing the stat vector → C2 |
| `preseason.py` | AS §2.2 horizon | conflict | Season-long product → C3 |
| `scoring/engine.py` | AS §2.3, §5.4 (AS:526–534); FBS §11.7 | ◐ | Linear, with no offset, no versioning and no inverse/serialisation; Standard used as default → C5 |
| Gaussian predictive (mean, S) | AS §5.3 distributions (AS:510); Layer F (AS:612) | ◐ | One-step Gaussian only. No quantiles, P(zero), boom/bust or simulation |
| `asof.py` leakage stack | AS §4.5 (AS:400–420), §12.3 (AS:1779) | ✓ concept / ◐ granularity | Week-level information time vs the spec's source/lock timestamps → E7 |
| `backtest.walk_forward` | AS §7.1 rolling-origin; FBS §13 | ◐ | No three-season window and no locks → C4 |
| `baselines.py` | AS §9.2 naive baselines (AS:1401–1406) | ◐ | Has persistence (= prior-game) and season-to-date. **Missing rolling-3 and position/depth-chart median.** Has last-season and market (not in the spec) |
| `metrics.py` | AS §7.4–7.5, §7.7; FBS §13 | ◐ | No PB-MAE, Brier, median AE, start/sit accuracy, Accuracy Gap or slice bias; bootstrap not week-clustered → C8, C9 |
| `tier1/tier2/lineup_sim/verdict` | AS §7.8, §9.4 gates; §10.2 scorecards | ◐ / conflict | Different metric (paired margin vs PB-MAE), different pool (C7), different gate design (C10) |
| `thresholds.py` | AS §7.7 "metric definitions versioned before…", §12.7 | ◐ | Compatible as pre-registration of a procedure → E9 |
| `golden_master` (A/B/C) | AS §12.2 (AS:1766), §6.6 rule 3; FBS §19.2 | ✓ / richer | Truth-anchored Layer A is beyond the spec → E8 |
| determinism | AS §6.6 rule 5, §9.3 | ✓ | Single-thread freeze; in-process to 1e-9 |
| `KalmanState.save/load` npz, accumulators npz | FBS §11.4 (state in SQLite), §11.6, §12.3, §14 | ◐ | Files, not SQLite; no versioning or rollback → C20 |
| `ParquetCache` | AS §4.1.1 freshness metadata; FBS §8.3 raw retention | conflict | TTL by mtime vs content hash → C20 |
| `nflverse_loader` / `adapter` | AS §4.1, §4.6 provider contracts | ◐ | No schedules, snap counts, depth charts or NGS. Participation is used live → C1 |
| `data_adapters` contract / `synth.py` | AS §12.2 "fixed synthetic football datasets" (AS:1766) | ◐ | The spec mentions synth fixtures only for golden tests; nothing on planted truth or recovery → E6 |
| `weekly_update` | AS §8.6, FBS §12.1 daily pipeline | ◐ / defects | Weekly not daily; no snapshot or rollback; defects C14/C15 |

---

## 4. Gaps: spec requirements with no Python implementation

These are grouped by the spec layer that drives them. Each item is required work for the Rust engine; the Python reference cannot supply a parity target for any of them.

### G-A. Availability (Layer A, AS:557–566; §4.1.2; §5.2)

- Active probability, start probability, snap multiplier if active, and limited-role probability.
- `availability_overrides.csv` import and an `AvailabilityProvider` adapter.
- Conditional vs unconditional projections.
- Active-status Brier score.
- Python's only availability notions are the Kalman's predict-only missing weeks and the H2 `out` sets.

### G-B. Team game environment (Layer B, AS:568–582; §11.1)

- A joint two-team latent model for plays, drives, pass/rush attempts, sacks, TDs by type, red-zone opportunities, pace, neutral pass rate and game script.
- §11.1 features: pace, seconds per play, PROE, no-huddle/shotgun, spread/total, rest/venue/surface/roof.
- The schedules dataset is not ingested.
- The Kalman for pace and pass tendency (AS:632) is absent.

### G-C. Constrained opportunity allocation (Layer C, AS:584–592)

- A softmax/simplex allocator with team-total constraints.
- Snap share, air-yard share, red-zone and goal-line share, depth-chart rank, teammate-vacated opportunity.
- A Kalman over opportunity share.

### G-D. Efficiency detail (Layer D)

- Explicit probability models for completion, catch and TD conversion.
- "High-variance rates, especially touchdowns, strongly shrunk" is not modelled separately.
- Fumbles and 2PT are fixed at 0.0. AS §6.6 rule 4 forbids silent defaults.

### G-E. Matchup and context (Layer E, AS:608)

- Venue, weather, rest, travel, QB and OL adjustments.
- Python's matchup grades are never fed into a projection, and Tier 2 was skipped.

### G-F. Correlated simulation (Layer F, AS:612–624; §5.3)

- A seeded Monte Carlo with shared game/team draws and logical constraints.
- Distribution outputs: median, SD, P10/25/75/90, P(zero or inactive), P(> threshold), boom/bust.
- Re-scoring of draws for multiple leagues.
- Python offers only a Gaussian `N(mean, S)` in dV currency and an expanding-error SD in points.

### G-1. Stat-vector targets (§5.1)

- active_prob, start_prob, snap share, sacks taken, fumbles lost and 2PT as modelled targets, and carries for WR/TE.

### G-2. Scoring profile (§5.4, FBS §11.7)

- An affine transform with an offset, a versioned profile, Half-PPR default, inverse and dimensional validation, and deterministic serialisation.

### G-3. NCAA (AS §4.2, §4.4.3, §6.3)

- CFBD ingestion with a call budget.
- Identity-link tiers and a review queue.
- The q-mixture with position/draft priors and identity confidence.
- n0 learned by position and component; an influence cap; separate role vs efficiency priors.
- Combine and recruiting data.

### G-4. Three-season window (AS §2.4, AS:178–188)

- Python accumulates without a bound (weekly_update) or uses whatever seasons the snapshot holds.

### G-5. Ensemble and stacking (AS §6.4)

- Out-of-fold stacking with constrained weights, and a GBM residual model.
- The "simpler wins when indistinguishable" rule.

### G-6. Explainability payload (AS §6.5)

- None exists. The Kalman components and the prior/evidence split are natural inputs (E4/E5).

### G-7. Fixed-lag RTS (AS:633, FBS §11.5)

### G-8. Persisted EB parameters (FBS §11.6)

- Prior mean/variance, population variance, counts and version. Python's `k_shrink=8` is a fixed constant.

### G-9. `IncrementalBooster` (FBS §11.8)

- Continuation, replay-window and periodic-rebuild modes. Python refits sklearn HGB from scratch every time.

### G-10. RAPM production controls (FBS §11.3)

- Sparse conjugate gradient with diagnostics (converged, iterations, residual_norm, tolerance, λ).
- Observation weights.
- Home-field term (absent in Python).
- Garbage-time filter (absent).
- OT handling (absent).
- Explicit minimum-appearance thresholds (absent from RAPM).

### G-11. Versioning and governance (FBS §10, §12.3–12.4, §13, §14; AS §3.1)

- Feature schema versions.
- The Data → Feature → Model → Prediction lineage.
- Model states TRAINING / VALIDATING / CANDIDATE / PRODUCTION / REJECTED / SUPERSEDED.
- Snapshot and automatic rollback.
- Resumable stages.

### G-12. Evaluation protocol (AS §7.2–7.9)

- Thursday/Sunday locks.
- Union-of-top-N player pool with N: QB 20, RB 40, WR 50, TE 15.
- PB-MAE with a training-period scale, and `improvement_j`.
- Median AE, start/sit accuracy, FantasyPros Accuracy Gap, quantile calibration error, bias by slice, weekly win rate.
- Week-clustered bootstrap.
- Provider registry and benchmark import.
- Rookie/low-evidence scorecards.
- Naive baselines: rolling 3-game mean and position/depth-chart median.

### G-13. Leakage tests (§12.3)

- Stat-correction versioning.
- Post-lock availability injection.
- "Current-season participation must not appear in promoted live features" (AS:1784).

### G-14. Data sources (§4.1)

- Schedules (spread_line, total_line).
- player_stats as **labels**.
- Snap counts, depth charts, NGS, PFR, FTN charting.
- `load_odds` for Layer 3.

### G-15. Feature families §11.1–11.5

- Most are absent: CPOE, YAC, NGS, PFR, churn, QB continuity, depth-chart entropy, model disagreement, combine, recruiting, PPA, market share.

---

## 5. Extras: Python capabilities the consolidated spec must add

Each item below names a suggested spec home and the minimum text it should carry.

### E1. Situational Value currency (new spec section under §6, e.g. "§6.0 GRID value currency")

- V(s) = E[drive points | down, distance, yardline], fitted with a booster.
- dV = V(s′) − V(s), with the realised value on terminal plays.
- Label vocabulary 7/3/0 (C23).
- V(s) is frozen within a season and refitted only at a season boundary on the three-season window.
- Intent text must be kept: "standard EP scaffold; originality is downstream".
- dV is GRID's **efficiency response variable** and also an §11.3 feature.

### E2. Attribution layers

**Layer 2: RAPM over participation**

- The play is the observation; ±1 player columns; team off/def intercepts make OL and scheme a *nuisance, never an estimand* (keep that sentence).
- Generalised ridge objective (C11).
- Accumulators per (season, week) block so that incremental and batch fits are exactly equal.
- Situation slices; optional WR×CB interactions.

**Layer 1: cross-fitted opponent-adjusted credit**

- Context model g(state, opponent defensive strength).
- Grouped-by-week cross-fitting (C18).
- Weekly credit = mean residual over exposures; exposure count → R.

**Fixed point:** RAPM → defender ratings → Layer-1 opponent adjustment → prior re-seed, for all players rather than only the focus QB.

**Placement in the Layer A–F model:** GRID talent is the **Layer D efficiency latent** and a Layer E opponent-strength input.

### E3. Layer-3 market reconciliation

- Pseudo-observations tie a gauge-invariant team strength to the market-implied strength, the closing spread before lock (C12).
- Required as a team-level anchor only, never per play. It is also a falsifiable hook: does bottom-up value reconstruct the market?

### E4. Dynamic layer: Kalman `[talent, form, scheme_fit]`

- The F/H/Q/R equations in §1.5.
- The discount d per position as the single interpretable knob (C17).
- Regime-change interventions: d_spike, extra form and scheme variance, rust R × 2 for N games.
- Scheme reset on a coaching change, distinct from an intervention.
- Predict-only missing weeks (NaN, never 0).
- One-step predictive (mean, S **with R**) as the calibration target; filtered = real-time grade; smoothed = retrospective talent; never alias the two.
- z > 3 innovation changepoint detection.
- Snap-scaled R.
- **Model-spec homes:** `docs/05-model-specs/kalman-player-efficiency.md`, plus a future `kalman-opportunity-share.md` / `kalman-team-pace.md` reusing the same engine.

### E5. Cross-league feeder equivalency

- Shared-player identification of feeder→NFL slope and intercept per feeder league (FBS, FCS, UFL, …).
- Position-specific prior variance.
- The prior enters the Kalman as x0 = prior_mean and P0 = prior_var. This **is** AS §6.3's EB posterior with n0 = R/P0, made dynamic. Keep the washout table as the diagnostic for n0.
- The spec's q-mixture wraps it: θ_ncaa_translated = feeder-SV equivalency (+ CFBD feature translation).

### E6. Synthetic planted-ground-truth contract (AS §12.2 rewrite)

- The fixed data contract (plays/players/market/college) is the swap point.
- A synthetic generator with planted abilities, team strength, a focus-QB trajectory with an injury, and a feeder factor.
- Recovery gates are scale-invariant correlations.
- Canonical config constants: `CANONICAL_SYNTH` (§A.1).
- The contract must be preserved by every engine change.
- Gaps to fix in the Rust synth: plant situational ability, WR–CB interactions, defensive-strength truth with the correct sign, opportunity/volume truth, and availability truth.

### E7. Three-axis leakage design (AS §4.5 / §12.3)

- Axes: temporal, scope and **state** (persistent caches as a global singleton).
- Guards:
  - metamorphic future-poisoning (bit-identical forecast);
  - deliberate-leak canaries for every guard;
  - tripwire frame;
  - accumulator watermark (must also cover season seams; V2);
  - injectable cache namespace;
  - two-path equivalence (offline walk-forward == production incremental path);
  - intervention foreknowledge;
  - player-universe as of W;
  - information-time ≠ week index (outcomes ≤ W−1, pregame ≤ W).

### E8. Three-layer golden master (AS §12.2 / §6.6 rule 3)

- Layer A: truth-anchored semantics that cannot be fixed by regenerating.
- Layer B: ordering.
- Layer C: numeric, rtol 1e-5 / atol 1e-6.
- A single-thread freeze for determinism, and an in-process determinism gate at 1e-9.

### E9. Calibration doctrine and calibrate-then-gate (AS §7.5, §12.7)

- NIS on S, PIT, PICP always paired with PINAW, closed-form CRPS, pinball at the floor and ceiling.
- Conditional vs unconditional (snaps known vs forecast) calibration.
- Regime-conditional calibration: first game back, absence, rookie.
- Sign-flip-null noise-floor gates, frozen once and committed, **only** for KPIs the spec does not fix numerically (C10).

### E10. Volume × efficiency decomposition (AS Layers C/D)

- State outright that GRID talent enters as a **feature** of efficiency, not as the projection.
- This is the empirical lesson of the phase-2c verdict: ROS is volume-dominated.

### E11. Decision-value metric (AS §7.5 "start/sit accuracy")

- The H2 lineup-margin simulation (one slot-filling rule for both methods; starter-OUT subset) as the concrete start/sit metric.

---

## 6. Conflicts, with recommended resolutions

The owner column marks decisions reserved for humans under AS §1.4: **[S]** statistical owner, **[P]** product/architecture owner, **[D]** data/licensing owner.

| # | Conflict | Code fact | Spec fact | Recommended resolution |
|---|---|---|---|---|
| **C1** [P][S] | Live participation dependence | RAPM needs `off/def_players`. `weekly_update` and `walk_forward` use **same-season** participation. H2 +0.848 and Tier-1 weekly were measured that way. | AS:75, 249, 631, 1784, 1940: no live dependence; backtests must not create train/serve skew | **Two-tier GRID.** (a) RAPM and the Layer-1 fixed point are fitted only at **season boundaries** on completed seasons with participation. They yield frozen talent priors and features, and the Kalman x0/P0 for season S. (b) The in-season weekly signal is participation-free: an **involvement-based Layer-1′ credit** (passer/rusher/receiver/target IDs from live PBP; exposure from snap counts; opponent adjustment from team-level defensive strength fitted without player columns). It feeds the Kalman. (c) Backtests must hide current-season participation (add the AS:1784 leakage test). (d) Re-measure H1/H2 under (a)–(c) before citing any GRID real-data win. |
| **C2** [S] | Scoring path | Two paths: the stat line (volume × rate → `calculate_points`) and **SV→points** (credit → points via an affine map). Fantasy uncertainty is a Gaussian on expanding errors. | AS §5.1/§5.4: stat vector first; simulated draws scored affinely | The stat vector is the only output contract. dV/SV stays internal (a Layer D feature). Demote `SVToPointsMap` to an *ensemble candidate / diagnostic* that must prove incremental value (AS:636). Distributions come from Layer F draws. |
| **C3** [P] | Horizon | Weekly plus **ROS (the H1 kill criterion)** plus preseason ×17 | AS §2.2: one week; W1–17 primary; W18 separate | The weekly horizon is the primary engine contract. ROS and preseason become derived products (sums of weekly simulated draws), reported as secondary diagnostics. H1 is no longer the kill criterion; the AS §9.4 gates govern. |
| **C4** [S] | Data window | Accumulators are cumulative with no window. V(s) is frozen on the first 4 slots of the first season. Kalman state carries indefinitely. | AS §2.4: S−2..S plus W−1, or S−1..S−3 preseason | Store per-(season, week) XtX/Xty blocks; the window is the exact sum of in-window blocks (incremental **and** compliant). Refit V(s) per season on the window. At a season boundary, rebuild the Kalman by replaying the window from the prior (the FBS §12.2 "periodic rebuild" analogue). Static metadata is exempt. |
| **C5** [S] | Default scoring | Verdict and history use **STANDARD** | Half-PPR default (AS:165) | Half-PPR default; Standard and Full PPR are built in; profiles become `(weights, offset, version)`. |
| **C6** [S] | Targets | 14 `SCORING_STAT_COLS`; fumbles and 2PT fixed at 0 | AS §5.1 vectors include active prob, start prob, snap share, sacks, fumbles, 2PT | Adopt the spec vectors. Model fumbles and 2PT with EB-shrunk rates. Remove the silent-zero contract (AS §6.6 rule 4). |
| **C7** [S] | Evaluation pool | `tier1.py:259` drops players with no realised row (inactive) | AS:772: inactive after lock = actual 0; union-of-top-N pool | Spec pool. Port Python's `tier1` only as a diagnostic. |
| **C8** [S] | Bootstrap | iid resampling of cells (`metrics.py:170`) | AS:857: bootstrap by week (and by game) | Week-clustered block bootstrap; keep the `excludes_zero` semantics. |
| **C9** [S] | Primary metric | Paired-margin skill vs baselines; H1/H2 | PB-MAE (AS:776–797) | PB-MAE primary; the Python metrics survive as secondary (CRPS, NIS, PICP/PINAW, Spearman, NDCG). |
| **C10** [S] | Gate design | Calibrate-then-gate (sign-flip null mean + 1.96·SE, frozen) | Fixed thresholds: AS §7.8 and §9.4 (≥ 3% vs the strongest naive baseline; no position > 1% worse; 80% coverage 72–88%, then 75–85%) | Spec thresholds govern promotion. Calibrate-then-gate applies only to KPIs without a spec number (H2 margin, Tier 2); the procedure is pre-registered in the registry (E9). |
| **C11** [S][P] | RAPM objective and solver | `min ‖y−Xβ‖² + λΣm_j(β_j−μ_j)² + w_mkt Σ(a_tᵀβ−s_t)²`; dense solve; **silent `lstsq` when cond > 1e10** | FBS:391 `‖y−Xβ‖² + λ‖β‖²`, sparse CG with diagnostics; basketball controls (stints, players on court, home court) | An ADR amends FBS §11.3 to the generalised objective (plain ridge is the special case μ=0, m=1, w=0) with NFL controls: play = observation, participation, dV response, off/def intercepts, home field, garbage time, OT, minimum exposures. Use Jacobi-preconditioned CG with diagnostics; keep dense Cholesky as the test oracle; turn the ill-conditioning fallback into a typed failure/diagnostic. FBS outranks AS, so this needs an explicit ADR or spec edit. |
| **C12** [S] | **Layer-3 sign convention / team strength** | `layers.py:404–405`, `backtest.py:108–109`, `weekly_update.py:272–273` all anchor **γ_off + γ_def**. `team_rating = γ_off − γ_def`. Planted `team_strength = off − def` (`synth.py:293`) rewards bad defences. `docs/02-backend-spec.md` and `04-lessons-learned.md` claim a fix to [+1, −1] that **the code does not contain**. Intercepts are **gauge-dependent**: each team has an exact null direction (its players +a with the intercept −n_onfield·a). **Empirically (sign_check.py):** raw-intercept team correlation 0.46–0.74 vs **gauge-invariant aggregates 0.82–0.92**; the market rows change little. | None (FBS mentions team effects only) | Define gauge-invariant team effects: E_off[t] = γ_off[t] + exposure-weighted Σ on-field offensive ratings; E_def[t] = γ_def[t] + exposure-weighted Σ on-field defensive ratings (positive = good D). Net strength = E_off + E_def. Planted truth = Σoff + Σdef. The market row anchors E_off + E_def, with the player columns weighted by exposure, to the spread-implied net strength. Add a truth-anchored golden Layer-A test. |
| **C13** [S] | Matchup grade sign | `−β[t_def]`; the comment says a stronger defence has a "more negative intercept" (`weekly_update.py:65`). Wrong: defenders enter with −1, so a stronger defence has a *more positive* effect. Empirically on synth corr(γ_def, true D) = −0.22 (gauge artefact); E_def has corr +0.61 to +0.69. The validation plan's Layer-A matchup-grade test was never written. | AS Layer E matchup adjustment | Grade = E_def (higher = tougher). Add a truth-anchored test with planted defensive strength. |
| **C14** [S] | Kalman observation semantics (production) | `weekly_update.py:307–309`: observation = **cumulative** season RAPM rating; snaps = 1; V(s) refitted on the season to date (line 213) | AS §6.2 Kalman online state; AS §4.5 leakage | Port only the validated semantics: observation = **weekly** credit; R from real exposures; V(s) frozen. |
| **C15** [S] | Batch vs incremental Kalman | Rust inflation lasts 2 games in the batch path (`statespace.py:214`) but only in the intervention week in `kalman_step` (`:398`, `:405`). Measured: identical without interventions; **max\|Δtotal_filt\| = 0.0195 and max\|ΔS\| = 0.0229** with one. | FBS §11.4 one filter | Persist `games_since_event` in the state. One Rust filter. Parity test batch == incremental, with and without interventions. |
| **C16** [S] | Kalman initialisation look-ahead | `x0 = nanmean(y[:3])` (`statespace.py:186`): weeks 1–3 forecasts see future data; fixed P0 | AS §4.5 / §6.6 | Set x0 and P0 from the prior (E5) or the position mean, never from observations. Python goldens that depend on it must be regenerated in the reference copy only after an ADR. |
| **C17** [S] | Discount semantics | `P[0,0] /= d` inflates only the talent variance, leaving the cross-covariances alone (not the standard full-P West–Harrison) | AS §6.6 "update equations" | The model spec states the component discount exactly as implemented (or chooses full-P with recalibration). |
| **C18** [S] | Layer-1 cross-fit folds | Production uses play-level `KFold(shuffle)` (`layers.py:512`); the calibration gate uses `GroupKFold` by week | AS §6.6 leakage | Week-grouped folds everywhere. |
| **C19** [P] | Booster | sklearn HGB with full refits | FBS §11.8 `IncrementalBooster` (xgb, linfa fallback) | Use the trait. Parity is functional/statistical, not numeric (§7.1). |
| **C20** [P] | Persistence and caching | `.npz` state, joblib V(s), mtime-TTL parquet cache | FBS §8, §11.4, §11.6, §12.3, §14; AS §4.1.1 | SQLite latent/model states plus versioned artefacts; content-hash raw retention. Do not port `ParquetCache`. |
| **C21** [D] | Identity and player universe | `player_id` = GSIS (real) or int (synth). The RAPM universe is the full roster deduplicated to the latest season (multi-season). Columns sorted by `str(id)`. | AS §4.4 GSIS canonical plus identity links | GSIS canonical; universe from roster snapshots as of W; synth IDs stringified exactly as `str(int)` to keep golden column order. |
| **C22** [D] | Labels | `data_pipeline._aggregate_stats_from_pbp` builds player_stats from PBP with heuristics | AS §4.1 "Player weekly stats… authoritative training labels" | nflverse player_stats as labels; PBP aggregates only as features. |
| **C23** [S] | `drive_points` vocabulary | 7/3/0; safeties and defensive TDs count 0; no negative values | Not in the spec (EPA is a feature) | Keep 7/3/0 in v1 for parity; the model spec must record it; a later ADR may adopt signed EP. |
| **C24** [P] | Situation list | Code: red_zone, passing_downs, rushing_downs, two_minute (conditional) | Docs claim goal_line, third_and_long, fourth_down; AS §11.2 wants features | The code set is canonical. The adapter must emit `quarter_seconds_remaining`. New situations only by spec. |
| **C25** [S] | Validation overclaims | `test_planted_wr_cb_interaction_recovered` asserts only non-empty and finite; Tier-2 "situation RAPM recovery ≥ 0.65" and changepoint P/R targets were never implemented | AS §12.2 | Plant the effects in the Rust synth and gate on real recovery, or drop the claims. |
| **C26** [S] | Tier-0 thresholds | Plan targets (pooled 0.85, team 0.90, NIS ∈ [0.8, 1.25] for the focus QB) vs **implemented floors** (0.77, 0.60, NIS ≤ 10) | none | Rust parity uses the implemented floors; the plan targets are recorded as stretch goals. Focus-QB NIS 4.58 is a known overconfidence. |
| **C27** [S] | Prior construction | `LEAGUE_FACTORS` unused; age/draft step functions; no q; no identity confidence | AS §6.3 | Adopt §6.3 with feeder-SV equivalency as one translated component (E5); continuous age/draft terms (issue #49). |
| **C28** [P] | RAPM status | — | FBS §11.3 "first-class production model" vs AS:631 "research-only if live parity absent" | Under C1, RAPM is production at season boundaries (frozen priors/features) and research-only in-season, which satisfies both documents. |

**Sign-check evidence (C12/C13).** Canonical synth; λ = 120; team intercept mask 0.05; w = 40.

| Market setting | corr(γ_off − γ_def, off − def) | corr(γ_def, true D) | corr(E_off − E_def, …) | corr(E_off + E_def, true off + def) | corr(E_def, true D) |
|---|---|---|---|---|---|
| No market | 0.693 | −0.223 | 0.854 | 0.844 | 0.609 |
| Current [+1, +1] | 0.682 | −0.185 | 0.901 | 0.824 | 0.677 |
| Audit fix [+1, −1] | 0.739 | −0.287 | 0.920 | 0.829 | 0.681 |
| Proposed (off + def target) | — | −0.180 | — | 0.830 | 0.692 |

Player correlation is 0.805 under every market setting.

---

## 7. Rust crate homes and parity targets

### 7.1 Parity strategy (applies to every port)

1. **Import the oracle.**
   - Import `backend/{grid,projection,validation}`, `backend/scoring/{engine,columns,formats,vor}.py`, `tests/{grid,projection,validation,scoring}`, `tests/grid/golden/snapshot.npz` and `run_demo.py` into `reference/python/`.
   - Vendor `backend/services/mock_draft.snake_order` (needed by `lineup_sim`) and `backend/pipeline/_logging` (needed by `backtest._log_subset`).
   - Exclude or adapt the four app-coupled tests: `test_changepoint::test_weekly_update_auto_interventions_recovers_demo`, `test_coaching_changes` (database), `test_leakage_guards::test_two_path_equivalence` (`weekly_update` plus database) and `test_verdict` (lazy database loaders, monkeypatched).
   - Pin the library versions. `requirements.txt` uses `>=`, and the golden is sensitive to the sklearn version.
2. **Fixture exporter (new).** Add `reference/python/tools/export_parity_fixtures.py`. Run single-threaded under `threadpool_limits(1)` and write `fixtures/parity/*.npz|parquet` plus a SHA-256 manifest:
   - canonical synth `plays` / `players` / `gt` / `college`;
   - Python `dv`;
   - V(s) predictions on a state grid;
   - Layer-1 out-of-fold residuals (play-level and week-grouped);
   - the RAPM design triplets, XtX/Xty and β for each configuration;
   - Kalman inputs and outputs for the focus QB and the QB pool;
   - the `estimate_equivalency` split permutation;
   - bootstrap resample indices;
   - walk-forward per-origin ratings.
3. **Stage-wise injection.** Each Rust stage is tested with Python's upstream outputs as inputs. Numeric parity is required wherever the stage is pure linear algebra; statistical parity applies wherever a booster or RNG is involved.
4. **End-to-end Rust-native gates.**
   - With the Rust V(s), the Rust booster and the Rust synth, the Tier-0 floors (§A.2) and the calibration bands must pass.
   - The golden Layer A truth-anchored invariants must pass.
   - Golden Layer C is checked only in fixture-injected mode.
5. Every confirmed defect in §6 (C12–C18) is fixed **in Rust behind a model-spec change**. The oracle keeps its old behaviour, and parity is asserted against an *oracle-patched* variant where needed. The ADR records the deliberate divergence.

### 7.2 Module → crate → parity target

| Python module | Rust home (crate::module) | Parity target |
|---|---|---|
| `grid/data_adapters.py` | `grid-domain::contract` (Plays/Players/Market/College types and validators) + `grid-synth::load` | Schema invariants: columns and types; terminal ⇒ `n_* = −1`; `terminal_value` NaN off-terminal; `drive_points` constant within a drive. Loading the exported synth fixture round-trips hash-identically. |
| `grid/synth.py` | **`grid-synth`** (new crate): `generator`, `fixtures` | (a) The exported canonical synth (16,825 plays; 288 players; 12 teams; 14 weeks; focus-QB τ array) loads exactly. (b) A Rust-native generator with the same config passes the Tier-0 floors and the golden Layer-A invariants. RNG parity is **not** required. (c) Add planted effects missing in Python (C12, C25). |
| `grid/value.py` | `grid-models::value` (V(s) via the `Regressor` / `IncrementalBooster` trait) | `compute_dv` exact (≤ 1e-12) given the same V predictions. Rust V(s) on the canonical synth: grid predictions within ±0.15 of [1.117, 1.840, 3.375, 5.575]; downstream Tier-0 floors hold. Joblib persistence → a versioned artefact. |
| `grid/layers.py` (design, accumulators, solve, market, situations, interactions) | `grid-models::rapm`, built on `grid-models::ridge` (generalised ridge primitive: dense Cholesky oracle plus preconditioned CG with diagnostics) | Given the Python `dv` fixture: identical CSR triplets and column order; XtX/Xty ≤ 1e-12; β relative error ≤ 1e-8 vs the Python dense solve for (λ = 120, m_team = 0.05, w = 40, interactions m = 10). Incremental == batch exactly (`test_incremental`). Ill-conditioning → a typed error, not a silent lstsq. Situation masks reproduce `run_situation_rapm`. |
| `grid/layers.py` (Layer 1, fixed point) | `grid-models::credit` | With the injected out-of-fold residual fixture: weekly credit and snaps exact. `fit()` with injected V(s) and residuals reproduces golden `rating`, `team_rating` and `qb_credit` to rtol 1e-5 / atol 1e-6. Rust-native (booster): `layer1_all_players` == the per-QB path; the context model is fitted exactly n_splits times. Folds are week-grouped (C18). |
| `grid/statespace.py` | `grid-models::statespace` (`kalman`, `rts`, `fixed_lag` [new], `changepoint`, `state` persisted via `grid-persistence`) | Given identical inputs (y, snaps, played, interventions, x0, P0): all `kalman_two_component` outputs ≤ 1e-12 abs. Golden `k_*` arrays to rtol 1e-5 when fed golden `qb_credit`. Batch == incremental with and without interventions (after C15). Joseph symmetry/PSD properties; RTS PSD. On the exported QB-pool residual fixture: median per-QB NIS ∈ [0.8, 1.25], pooled ∈ [0.8, 1.4], PIT mean ± 0.07, PIT sd ∈ [0.24, 0.34], PICP80 ∈ [0.70, 0.90]. Changepoint z-score cases from `test_changepoint`. Scheme reset semantics from `test_coaching_changes`. The x0 look-ahead is removed (C16) in a documented divergence. |
| `grid/priors.py` | `grid-models::priors` + `grid-models::empirical_bayes` | With the injected permutation: slope/intercept ≤ 1e-10, `oos_r2` exact; `build_priors` and `washout_table` exact. Tier 0: slope ∈ [0.9, 1.8] (1.3159), `oos_r2` ≥ 0.05 (0.1489), rookie corr ≥ 0.50 (0.5829). New: x0/P0 handoff to the Kalman; §6.3 q-mixture. |
| `grid/situations.py` | `grid-features::situations` | Masks exact; `two_minute` only when the clock column is present (the adapter must emit it). |
| `grid/cache.py` | not ported → `grid-ingestion::raw` (content-hash retention) + `grid-persistence` | n/a (C20) |
| `grid/nflverse_loader.py` | `grid-ingestion::nflverse` (pbp, roster, participation; plus schedules, player_stats, snap counts) | Normalisation exact on sanitised provider fixtures (AS §4.6); the 9 loader tests. |
| `grid/nflverse_adapter.py` | `grid-ingestion::plays_contract` | Frame equality on a pbp+participation fixture; the 12 adapter tests (drive ID, 7/3/0 mapping, next state, no cross-drive bleed, int/float play_id join, NA → `()`, skill filter, empty contract). |
| `projection/volume.py` | `grid-models::projection::volume` (Layer-C seed; EB) | Exact (k = 8) on the `test_volume` cases; later replaced or extended by the constrained allocator (G-C). |
| `projection/model.py` | `grid-models::projection::rates` (Layer-D seed) | Closed-form standardised ridge (alpha = 1, unpenalised intercept, population-sd scaler as in sklearn) ≤ 1e-10 vs sklearn. `test_model` synth recovery. |
| `projection/features.py` | `grid-features::talent` | Exact assembly; the filtered-vs-smoothed separation is enforced by type. |
| `projection/sv_to_points.py` | `grid-evaluation::diagnostics::sv_map` (not an output path; C2) | OLS ≤ 1e-10 vs `np.polyfit`; `test_sv_to_points`. |
| `projection/preseason.py` | `grid-models::projection::season` (derived product; C3) | Exact (× games). |
| `scoring/engine.py`, `columns.py`, `formats.py` | `grid-scoring::profile` (affine with offset, versioned) | Points exact for the Standard, Half and Full presets on fixtures; offset = 0 reproduces Python. |
| `scoring/vor.py`, `format_registry.py` | Out of the engine. VOR is retained only inside `grid-evaluation::lineup` if H2 is kept. | `test_vor` cases, only if ported. |
| `validation/asof.py` | `grid-domain::asof` (AsOf type) + `grid-evaluation::leakage` (Tripwire, CacheNamespace analogue for scratch state dirs) | The 14 `test_asof` behaviours; fix V8 (array cells). |
| `validation/backtest.py` | `grid-evaluation::walk_forward` | With injected `dv`: per-origin ratings ≤ 1e-8 vs the Python fixture. Leakage canaries: future-poisoning bit-identical, the canary fires, the watermark (including season seams; fixes V2), two-path equivalence ≤ 1e-6. The three-season window per C4. |
| `validation/baselines.py` | `grid-evaluation::baselines` | Exact; add rolling-3 and position/depth-chart median (G-12). |
| `validation/metrics.py` | `grid-evaluation::metrics` | Closed forms ≤ 1e-12 (`normal_ppf` Acklam ≤ 1.15e-9 abs). Bootstrap CI exact given injected indices. Add PB-MAE, Brier, median AE and a week-clustered bootstrap (C8, C9). Fix V4–V6/V12 edge semantics as typed errors. |
| `validation/tier1.py`, `tier2.py` | `grid-evaluation::scorecard` (spec pool and PB-MAE) + `grid-evaluation::legacy_tiers` (diagnostic) | Python tier tables reproduced exactly on synth (diagnostic mode); spec pool behaviour per AS §7.3. |
| `validation/lineup_sim.py` | `grid-evaluation::lineup` (start/sit decision metric, E11) | `test_lineup_sim` (13) exact, including tie handling and the starter-OUT subset. |
| `validation/sniff.py` | `grid-evaluation::sniff` | `test_sniff` exact. |
| `validation/thresholds.py` (+ JSON) | `grid-governance::registry` | `test_thresholds` exact; freeze-once; committed registry. |
| `validation/verdict.py`, `report.py` | `grid-governance::promotion` (model states, FBS §13) + `grid-governance::report` | Synth verdict (`test_verdict` non-DB cases) reproduced. `SHIP_GRID` / `BASELINE_FALLBACK` map onto PRODUCTION / REJECTED decisions under the AS §9.4 gates. |
| `pipeline/weekly_update.py` | `grid-pipeline::weekly` (replaces the `application` crate) | Rebuilt on the validated semantics (C14, C15). Two-path equivalence with `walk_forward`. Snapshot and rollback (FBS §12.3). |
| `pipeline/ingest_grid.py` | `grid-pipeline::snapshot` | Manifest (per-season plays, drives, off/def coverage) exact on a fixture; atomic write (V10). |
| `pipeline/data_pipeline.py`, `compute_valuations.py`, `sync_*`, `health_check.py` | Not ported (app). Label ingest is redone per C22. | n/a |

**Crate list after the pivot:**

- `grid-domain`, `grid-persistence`, `grid-ingestion`, `grid-identity`, `grid-features`, `grid-models`, `grid-simulation`, `grid-scoring`, `grid-evaluation`, `grid-governance`;
- **`grid-synth`** (new);
- **`grid-pipeline`** (new; headless CLI that replaces `application`);
- **drop `ffi`** and `app/` (Flutter).

`grid-models` gains the submodules `ridge`, `affine`, `booster`, `value`, `rapm`, `credit`, `statespace`, `priors`, `empirical_bayes`, `projection`. This mirrors AS §8.1 `models/` plus the GRID additions. `grid-simulation` owns Layer F (G-F); there is no Python parity source for it.

### 7.3 Suggested port order (dependency-driven)

1. `grid-domain`: contract, AsOf, IDs.
2. `grid-synth`: fixture loader.
3. `models::ridge` → `models::rapm` (with injected `dv`).
4. `models::statespace` (with injected credit).
5. `models::priors` and `empirical_bayes`.
6. `evaluation::metrics` and `baselines`.
7. `evaluation::walk_forward` and `leakage`.
8. `models::value` and `credit` (booster; statistical gates).
9. `ingestion` (nflverse adapter on fixtures).
10. `scoring::profile`.
11. Projection seeds.
12. Governance and registry.
13. New spec layers: A, B, C allocator, F simulation.

---

## 8. Decisions needed from owners (summary; details in §6)

1. **[P][S] C1.** Adopt the two-tier GRID: RAPM fitted at season boundaries, a participation-free in-season Layer-1′ credit, and backtests that hide current-season participation. Accept that the existing H2 win is not citable until re-measured.
2. **[S] C12/C13.** Gauge-invariant team strength and matchup grade (E_off/E_def), correct planted truth, and the market row anchoring E_off + E_def. This overrides the documented-but-unimplemented [+1, −1] "fix".
3. **[S][P] C11.** An ADR amending FBS §11.3 to the generalised GRID ridge objective with NFL domain controls (FBS outranks AS, so this is a spec edit or a level-3 ADR plus owner ratification).
4. **[P] C3/C9/C10.** Weekly is the primary horizon, PB-MAE the primary metric and the AS §9.4 fixed gates govern. H1 (ROS vs last season) is demoted to a diagnostic, and calibrate-then-gate is limited to KPIs without a spec threshold.
5. **[S] C14–C17.** Fix the Kalman semantics in Rust and diverge deliberately from the oracle: weekly observations with real exposures; batch == incremental via a persisted `games_since_event`; a prior-based x0/P0 instead of the look-ahead; the component-discount definition.
6. **[S] C4.** Three-season window via per-week accumulator blocks; V(s) refitted per season; Kalman replay at season boundaries.
7. **[P] Parity contract (§7.1).** Stage-wise, fixture-injected numeric parity, with statistical (not numeric) parity for anything that involves a booster or RNG. This requires pinning the Python reference environment.
8. **[S] C23/C24.** Keep 7/3/0 `drive_points` and the code's situation set for v1.

---

## Appendix A: constants the ports must carry

### A.1 Canonical synthetic configuration and run constants

- `CANONICAL_SYNTH = SynthConfig(n_teams=12, weeks=14, seed=7, yards_ability_gain=26.0, yards_noise_sd=3.2, drives_per_team_per_game=12, max_plays_per_drive=12, fg_range_yardline=35, league_factor=0.62, college_noise_sd=0.030, cb_split=False, n_cb_per_team=2)`.
- Market seed 1, market noise sd 0.01.
- `N_ITER=3`; focus intervention at index 9 (week 10).
- Golden layout: 288 players, 12 teams, 14-week Kalman arrays. Tolerances rtol 1e-5 / atol 1e-6. Single-threaded.

### A.2 Live Tier-0 values (recomputed 2026-10-01; all match the test comments)

| Metric | Value | Gate floor |
|---|---|---|
| Pooled attribution | 0.8025 | 0.77 |
| QB | 0.869 (n = 24) | 0.83 |
| RB | 0.7433 (n = 36) | 0.70 |
| WR | 0.7973 (n = 60) | 0.76 |
| TE | 0.7713 (n = 24) | 0.73 |
| DEF | 0.7653 (n = 144) | 0.73 |
| Team strength | 0.6643 | 0.60 |
| Kalman total_smooth | 0.958 | 0.92 |
| Kalman tau_smooth | 0.6745 | 0.60 |
| Kalman total_filt (ungated) | 0.9867 | none |
| Focus-QB NIS | 4.581 | ≤ 10 |
| Filtered variance, week 3 → week 10 | 0.00228 → 0.00614 | must increase |
| Equivalency slope | 1.3159 | [0.9, 1.8] |
| OOS R² | 0.1489 | ≥ 0.05 |
| Rookie prior correlation | 0.5829 | ≥ 0.50 |

### A.3 Hyperparameters

| Component | Hyperparameters |
|---|---|
| V(s) booster | max_depth 4, lr 0.08, max_iter 300, min_samples_leaf 120 |
| Layer-1 booster | max_depth 3, lr 0.1, max_iter 200, min_samples_leaf 150; 5 folds; min_plays 20 |
| RAPM | λ 120; team mask 0.05; w_market 40; interaction mask 10; min_pair_plays 30 |
| Situation RAPM | min_plays 200 (weekly_update uses 50) |
| Ridge fallback | cond threshold 1e10 |
| SSParams (defaults) | φ 0.50, φ_s 0.985, d 0.90, d_spike 0.70, q_form 8e-4, q_scheme 2e-4, scheme_reset_var 0.04, r_scale 0.40, post_event_r_mult 2.0, post_event_games 2 |
| SSParams (per position) | see §1.5 |
| Kalman initial P0 | diag(0.05, 0.02, 0.01) |
| Changepoint | z 3.0 |
| PRIOR_SD | QB .16, RB .07, WR .08, TE .06, DEF .07, otherwise .10 |
| Volume | k_shrink 8 |
| Stat-line rates | ridge alpha 1.0 |
| SV→points | min_rows 10 |
| Bootstrap | n 10,000, α 0.05, seed 0 |
| Gate z | 1.959964 |
| Walk-forward | warmup_weeks 4 |
