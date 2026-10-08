# cautious-nevermore: engine known-issues backlog (verified against `main` @ 59bce1d)

This is an inventory report for the GRID-One/GRID-Engine consolidation. It covers
every issue log entry, GitHub issue and audit finding that touches engine code
(`backend/grid`, `backend/projection`, `backend/validation`, `backend/scoring`, the engine
parts of `backend/pipeline`, and the nflverse ingest). I checked the status of each one
against the code, not against the log. Where I could, I backed the claim with a script run
on synthetic data or on the real 2023 nflverse data.

- Repo checked: `/home/user/cautious-nevermore` @ `59bce1d`. This is a **shallow clone**:
  the boundary commit is `c1b6748` (2026-06-22). Fixes that landed before that date can't be dated.
- Scratch evidence (repro scripts, all runnable from that dir with `python3 <script>`):
  `/tmp/claude-0/-home-user/693e74a1-f8af-5256-86e9-2299b8697223/scratchpad/inventory/scratch-cnissues/`
  - `sign_test.py`, `sign_test2.py`: market-row conventions vs. recovery (G1)
  - `defsign_planted.py`: planted-defense test of the matchup-grade sign (NEW-A2)
  - `defgrade_test.py`, `defgrade_test2.py`: Tier-2 KPI on engine-produced grades
  - `cmp_stats.py`: CN stat aggregation vs. official nflverse `player_stats` 2023 (NEW-I1..I5)
  - `real_contract.py`, `real_rapm.py`: real 2023 plays contract plus RAPM scale (NEW-A3, NEW-P4)
  - `rollover_test.py`, `reinit_test.py`: weekly_update season rollover and roster-change reinit (NEW-W1, NEW-W2)
  - `kparity.py`: batch `kalman_two_component` vs. incremental `kalman_step` divergence (NEW-S1)
  - `psd.py`: RTS covariance PSD stress test (G4)
  - `patched/`: a copy with only G1 fixed (`layers.py:405` → `-1.0`), used to measure the golden/Tier-0 impact
  - `realdata/`: 2023 nflverse pbp, participation, roster, official player stats
  - `pytest_engine.txt`: engine suites at 59bce1d on Linux. **458 passed, 0 failed.**

Legend:

- **Status:** OPEN, FIXED (with commit), FALSE-POSITIVE, PARTIAL, UNVERIFIABLE.
- **Sev:** my re-graded severity. **CRIT** means it corrupts engine numbers on the main path. **HIGH**, **MOD** and **LOW** follow from that.
- **(a) Oracle:** does this bug contaminate something we would freeze as a Python-oracle parity target?
  - **GOLDEN:** the synth golden master and Tier-0 gates bake it in.
  - **REAL:** real-data outputs (verdict, ingest).
  - **INCR:** only the incremental `weekly_update` path.
  - **NONE:** no oracle impact.
- **(b) Rust:** what it means for the port.
  - **MUST-NOT-REPRODUCE:** the port must not copy this bug.
  - **DESIGN-INPUT:** a design or convention choice the port has to make explicitly.
  - **TYPED-FAILURE:** the port should turn it into an explicit typed error (alpha-spec §6, "Implementation rules" item 4: NaN, singular-system and similar paths are explicit typed failures).
  - **DROP:** the code shouldn't be ported at all.

---

## 0. Headline findings (read this first)

1. **The real-data box-score ingest is biased.** This has not been logged anywhere. Running CN's own `_normalize_pbp` and
   `_aggregate_stats_from_pbp` on the 2023 nflverse pbp and comparing with the official nflverse `player_stats`
   (REG season) gives:

   | Stat | CN | Official | Error |
   |---|---|---|---|
   | pass_attempts | 19,658 | 18,315 | +7.3%, because sacks are counted |
   | passing_yards | 119,092 | 128,567 | −7.4%, because sack yards are netted |
   | passing_tds | 814 | 754 | +8%, because return TDs are credited |
   | receiving_tds | 799 | 754 | +6% |
   | fumbles_lost | 174 | 256 | −32%, because of mis-attribution |

   Postseason weeks 19–22 are also stored and summed into season totals. These numbers feed the stat-line model
   fit, volume model, realized-points history, H1/H2 verdict and `compute_valuations`. **No real-data verdict or
   valuation number from CN is a trustworthy parity target.** See §3.7.
2. **G1/V1 (Layer-3 market sign) is still OPEN.** It is in **three** places, not two. The third is `weekly_update.py:273`.
   The sign is also only half of the problem: the whole team-strength convention is inconsistent.
   - Fixing just the row sign to `[+1,−1]`, as the log says, raises canonical-synth team recovery from **0.6644 to 0.7318**.
     It also breaks 4 golden-master tests (Layer B ×2, Layer C ×2).
   - The golden master and the Tier-0 team gate currently freeze the buggy behaviour.
   - The convention itself needs an owner decision. See §3.1, G1.
3. **NEW, CONFIRMED: the matchup-grade sign is inverted.** The code computes `rapm_grade = −β[t_def]`
   (`weekly_update.py:84`, `backtest.py:185-186`). With the `−1` defensive encoding, a *stronger* defence has a *larger*
   `β[t_def]`. In a planted-defence test, the ELITE defence got the *lowest* grade (−0.409) and the WEAK defence the
   highest (+0.382). The unit tests hard-code the wrong assumption with hand-planted betas.
4. **The incremental `weekly_update` path is not a usable oracle.** I confirmed these failures by running it:
   - **Season rollover skips forever.** The accumulators are keyed by week only. 2025 week 1 after 2024 week 18 returns `skipped: True`.
   - **Any roster change silently reinitialises the RAPM accumulators.** One new signing drops all prior weeks.
   - The Kalman filter is fed *cumulative* RAPM as if it were an independent weekly observation, for every player, including players who didn't play.
   - `snaps = 1` makes R 30–60× too large. Changepoint z-scores can't exceed about 0.1, so auto-interventions never fire on real data.
   - `kalman_step` and `kalman_two_component` diverge after an intervention.

   The Rust incremental design should start from the batch walk-forward semantics, not from this code.
5. **Status corrections to CN's own docs:**
   - There are **39** live GitHub issues, not 48. All are open. #50 is missing from the log.
   - The golden-master Layer C and `test_cache` "pre-existing failures" (`docs/10-next-steps-plan.md`) are **Windows-only**. All 458 engine tests pass on Linux at 59bce1d.
   - A1, G2, G7 and G11 are false positives. G3 is MINOR, not CRITICAL.
   - "Cross-league priors wired to real data" (`docs/05-current-state.md`) is **false**. Nothing outside `backend/grid` calls `build_priors` or `estimate_equivalency`, and the verdict hard-codes `prior_mean = 0.0` (`verdict.py:280`).
   - The audit snapshot predates some fixes: A6 was fixed in #85 on 2026-07-01, and G5/W12 in #63 on 2026-06-28, both before the "2026-07-13" audit.
6. **The real data scale is nothing like the synth scale.** Real 2023 single-season RAPM rating SD is about 0.04–0.046 at *every*
   position (QB 0.042). Synth RAPM SD is QB 0.259 and other positions 0.07–0.09. Every hand-set scale constant is synth-calibrated:
   `PRIOR_SD`, the age/draft adjustments, `KalmanState.init` P0, and the `SSParams` q/r values. The spread between QBs on real data
   also looks absorbed by the team-offense intercept, because a full-time starter is nearly collinear with it and the team ridge is
   20× lighter. See NEW-A3 and NEW-P4.

---

## 1. Source reconciliation

| Source | Claim | Reality |
|---|---|---|
| `docs/06-issues-log.md` header | "48 open, 0 closed" GitHub issues | `list_issues` on Seismic-Fate/cautious-nevermore returns **39 open, 0 closed** (#15, #16, #21–#52, #57–#59, #61, #62). The log's tables list 38. **#50 "Trade finder combinatorial explosion" is absent from the log** (it's app-only). |
| `06` audit totals | 11 CRIT / 54 MOD / 24 MINOR | The tables enumerate fewer. **Missing IDs:** G13; W4; W16 (only referenced via the #28 cross-ref "season.py:75"); 9 of 19 Application-logic MODERATE (the table says "key items"); 1 of 5 A-MINOR. No audit source survives in git history (`git log --all --diff-filter=D` shows only the `docs/superpowers/*` plans/specs). **These are UNVERIFIABLE.** |
| `06` line numbers | e.g. G6 `layers.py:519`, G12 `:307`, G14 `:539` | The audit ran on an older tree. Current lines are 555, 327 and 591–610. The current lines are cited below. |
| `10-next-steps-plan.md` Stage 0 | Golden master Layer C ×2 failing, blamed on #80; `test_cache::test_ttl_expired` failing | **Both pass on Linux** (`pytest_engine.txt`). The Layer-C drift is cross-platform GBM variance (Windows). It is not #80 drift: `tests/grid/golden/snapshot.npz` was last written in #65 (`1642892`) and still matches at 1e-5 on Linux. The TTL failure is Windows mtime granularity. |
| `04-lessons-learned.md` | "fumbles_lost counts ALL fumbles" | False. `nflverse_loader.py:78` reads `fumble_lost`. The *real* defect is attribution (NEW-I3). |
| `05-current-state.md` | "Cross-league priors … wired to real data (Phase 2b)" | False. #82 (`2690d72`) only made `estimate_equivalency` tolerate a missing `ability` column. There is no real feeder-SV source (`data_adapters.load_cfbd` raises NotImplementedError) and no caller. |
| `05` headline verdict | H1 −0.015 tie; H2 +0.848 [+0.232, +1.466] | These numbers were recorded *before* #91 (smoothed_talent wiring). A post-#91 re-run was never recorded (deleted plan `2026-07-09-phase2c-verdict-and-ros-gap.md`, "Update" section). They also rest on the biased stats (§3.7) and iid-bootstrap CIs (NEW-V1). Treat them as **not reproducible parity targets**. |

---

## 2. Scope split

**Engine items in this backlog:**
- **GitHub (16, plus #43/#45 as research candidates = 18 of the 39):** #15, #23, #24, #25, #27, #31, #32, #33, #41, #42, #44, #46, #47, #48, #49, #57.
- **Audit (all G\*, P\*, V\*, plus):** A1, A3, A4 (engine output contract only), A5, A6, A7, A8, A9, A15, and W12 (it reads the engine's Kalman state).
- **NEW findings from this pass:** 36, prefixed NEW-.

**Dropped as app-only, listed here so the omission is visible:**

- **GitHub:** #16, #21, #22, #26, #28, #29, #30, #34, #35, #36, #37, #38, #39 (ops; its engine-relevant piece is V10), #40, #50, #51, #52, #58, #59, #61, #62.
- **Audit:** W1, W2, W3, W5, W6, W7, W8, W9, W10, W11, W13, W14, W15, W17, W18, W19, W20 (plus the unlisted W4/W16), A2, A10, A11, A12, A13, A14, A16, A17, A18, F1–F8.

---

## 3. Backlog, by engine area

### 3.1 Attribution: RAPM, Layer 3, Layer 1 (`backend/grid/layers.py`, `backend/validation/backtest.py`, `backend/pipeline/weekly_update.py`)

| ID | Location (current) | Finding | Status / evidence | Sev | (a) Oracle | (b) Rust |
|---|---|---|---|---|---|---|
| **G1 / V1** (+3rd site) | `layers.py:400-408`; `backtest.py:101-112`; **`weekly_update.py:269-276`** (not in log) | The Layer-3 market pseudo-row is `[+1 on t_off, +1 on t_def]`, but `team_rating = β_off − β_def` (`layers.py:426`) and the synth truth is `mean_off − mean_def` (`synth.py:285-294`). So the anchor constrains `β_off+β_def` toward an `off−def` target. | **OPEN.** On canonical synth (`load_synthetic()`, market seed 1, `fit(n_iter=3)`): team corr is 0.6644 as shipped and **0.7318 with `[+1,−1]`**; player corr is unchanged (0.8026 → 0.8036). Across 5 seeds (`sign_test2.py`), mean team corr is 0.580 as shipped vs. 0.644 fixed. The shipped anchor is sometimes *worse than no market* (seed 11: 0.671 vs. 0.743 in `sign_test.py`). Applying the fix in `patched/` fails `test_golden_master.py::test_layerB_top_player_ranking`, `test_layerB_team_ranking_order`, `test_layerC_player_and_team_ratings` and `test_layerC_qb_weekly_and_kalman` (qb_credit Δ up to 0.0125). No real market is fed anywhere (`data_adapters.load_odds` is a stub; `verdict.main` and `weekly_update` pass `None`), so **only synth outputs are affected**. | HIGH | **GOLDEN.** Golden Layer B/C and the Tier-0 `team_corr ≥ 0.60` gate (`tests/grid/test_tier0_recovery.py:99`, observed 0.6643) freeze the bug. | MUST-NOT-REPRODUCE + DESIGN-INPUT (see the convention note under this table) |
| **NEW-A1** | `layers.py:426` + `synth.py:285-294` | The team-strength *convention* is semantically inverted for real markets. The play model is `… + β_off[off_team] − β_def[def_team]`, so a larger `β_def` means a better defence. A real spread implies net strength `β_off+β_def`. CN defines team strength as off − def, so a great defence *lowers* "team strength". The log's `[+1,−1]` fix makes the code self-consistent with the synth, but would anchor the wrong quantity once a real closing-line market is wired. | **OPEN** (design). Option A (net strength: truth `off+def`, `team_rating = β_off+β_def`, keep the `[+1,+1]` row) gives 5-seed mean 0.536 vs. no-market 0.403 for the same target. That shows an aligned anchor helps. With `w_market=40` against roughly 800–3,500 plays per team intercept, Layer 3 is a weak nudge, not a "reconciliation". | HIGH | GOLDEN | DESIGN-INPUT. Owner decision #1. |
| **NEW-A2** | `weekly_update.py:62-66` (docstring), `:84` (`def_strength = −β[t_def]`); `backtest.py:48-51,185-186` (`def_grades`); tests `tests/pipeline/test_weekly_update.py:286-370` | **The matchup grade sign is inverted.** The docstring reasons "stronger defence = more negative intercept because defenders enter X with −1", which has the algebra backwards. | **CONFIRMED** (`defsign_planted.py`, 20k plays, planted team-level defence effects −0.5/0/+0.5): ELITE `β_def = +0.409` → shipped grade **−0.409**; WEAK `β_def = −0.382` → grade **+0.382**. The tests pass only because they plant `β_ELITE = −3` by hand rather than estimating it. | HIGH | REAL/INCR. It writes `matchup_grades` and Tier-2 `def_grades`. | MUST-NOT-REPRODUCE. Grade = `+β_def`, or better the total defensive effect (see NEW-V2). |
| **NEW-A3** | `layers.py:378-389` (team mask 0.05 vs. player 1.0) | On real data the starting QB is about 100% of team offense snaps, so his column is nearly collinear with `t_off`. With a 20× lighter ridge on the intercept, the intercept absorbs the starters' value, and part-time backups get the residual. | **PLAUSIBLE, with evidence** (`real_rapm.py`, 2023 REG, full-roster universe): QB rating SD is 0.042, the same as WR 0.045 and RB 0.044, while synth QB is 0.259. Top QBs by rating include Browning, Flacco and O'Connell. Hurts and Young are near the bottom. corr(starter QB rating, own `t_off`) = 0.13. | HIGH (for real-data use) | REAL | DESIGN-INPUT. Needs multi-season pooling, a QB-specific prior/ridge, or Layer-1 event credit for QBs. Validate on real data before freezing any real-RAPM golden. |
| **G6 / #27** | `layers.py:541-557` (`layer1_all_qbs`, bare `except Exception: pass` at 555-556) | Silently drops QBs whose fit raises. | **OPEN.** Dead in production: only tests call it. `verdict.py:40,180` uses `layer1_all_players`. | LOW | NONE | DROP the function, or a TYPED-FAILURE plus log. |
| **G12** | `layers.py:327` (`y = plays["dv"]…`) | A missing `dv` gives a bare KeyError. | **OPEN** (trivial contract precondition). | LOW | NONE | TYPED-FAILURE (a contract-validation error). |
| **G14** | `layers.py:591-610` (`fit`) | The fixed point re-seeds the Layer-2 prior only for `focus_qb`. | **OPEN**, by design ("light coupling"). `fit()` is synth/demo/golden-only. Production paths (`run_rapm`, `walk_forward`, `weekly_update`) have *no* Layer-1→Layer-2 coupling at all. | MOD | GOLDEN (golden uses `fit`) | DESIGN-INPUT. Either generalize the re-seed to all players with Layer-1 credit, or drop the "fixed point" claim from the spec. |
| **#32** | `weekly_update.py:278`, `backtest.py:114` (no `lambda_by_pos`); no WR/TE target-share code | Position-specific λ exists in `run_rapm(lambda_by_pos=)` (`layers.py:381-386`) but is never passed by production solvers. WR/TE attribution is pure on-field plus-minus. | **OPEN.** Note `fit_from_accumulators` callers build their own mask and bypass `lambda_by_pos` entirely. | MOD | REAL | DESIGN-INPUT |
| **#57** | `situations.py:30-34` (plural `passing_downs`/`rushing_downs`) vs. `db/schema.sql:148` (singular CHECK) | Vocabulary mismatch. There is no `situation_grades` producer. | **OPEN.** | LOW | NONE | DESIGN-INPUT: one canonical situation enum in the engine. |
| **NEW-A4** | `situations.py:57-58` | `two_minute = quarter_seconds_remaining ≤ 120` ignores `qtr`, so it would include the ends of Q1/Q3. The plays contract (`nflverse_adapter.py:127-146`) never emits the column, so on real data the mask is always absent. | **OPEN** (latent). | LOW | NONE | MUST-NOT-REPRODUCE (use `half_seconds_remaining` or `qtr ∈ {2,4}`). |
| **NEW-A5** | `layers.py:507-519` | The Layer-1 "cross-fitted" residual uses `def_sum` from RAPM ratings fitted on *all* plays, including the held-out fold. The context feature is in-sample, so this is not a full cross-fit. | **OPEN** (minor leakage in a nuisance feature). | LOW | GOLDEN (qb_credit) | DESIGN-INPUT (fold-wise ratings or a frozen prior). |
| **NEW-A6** | `layers.py:355-358` | `np.linalg.cond(A)` (an SVD) runs on every solve before `solve`. It is O(n³) extra on about 2.5k columns, and the lstsq fallback at cond > 1e10 is silent apart from a log line. | **OPEN** (perf/robustness). | LOW | NONE | Use Cholesky with typed singular failure. |
| #42 | — | MF on RAPM residuals (enhancement) | Not started. | — | — | Research backlog |

### 3.2 Incremental weekly pipeline (`backend/pipeline/weekly_update.py`)

This whole path is the in-season engine loop. Its findings compound; the summary is that it can't serve as an oracle.

| ID | Location | Finding | Status / evidence | Sev | (a) | (b) |
|---|---|---|---|---|---|---|
| **NEW-W1** | `:237-244`; `layers.py:90-117` (accumulators store `week` only) | Season rollover: `if last_week >= week: skip` has no season key, so week 1 of a new season is skipped forever until the cache is deleted. The Kalman state is not season-keyed either. | **CONFIRMED** (`rollover_test.py`): 2024 wk18 is solved, then 2025 wk1 returns `{'rapm_solved': False, 'skipped': True}`. | CRIT (in-season) | INCR | MUST-NOT-REPRODUCE (key state by (season, week) plus an explicit carry-over policy). |
| **NEW-W2** | `:245-252` (and situation pass `:453-464`) | Any change in roster size or order reinitialises the accumulators, discarding all previous weeks. nflverse season rosters grow weekly with signings, so in-season RAPM collapses to the current week. | **CONFIRMED** (`reinit_test.py`): one added player gives "dimension mismatch (16 vs 17) — reinitialising". The P0 diagonal stays 60 instead of 120. | CRIT (in-season) | INCR | MUST-NOT-REPRODUCE (stable player index; grow XtX by zero-padding). |
| **NEW-W3** | `:280-309` | The Kalman observation is the **cumulative season-to-date RAPM coefficient**, not a weekly signal. Successive "observations" are highly autocorrelated, which violates the independent-noise assumption. Every player in `players_df` gets an observation each week (none are NaN), so players who didn't play are shrunk toward their RAPM prior with growing confidence. That contradicts the documented missing-week semantics (`statespace.py:20-21`). The batch validation path (`verdict.py:126-200`) correctly uses weekly Layer-1 credit plus `played` masks. | **OPEN** (by reading). | HIGH | INCR | MUST-NOT-REPRODUCE. Port the batch semantics: weekly Layer-1 credit as y, snaps as precision, NaN for non-play. |
| **#24** | `:307-309` (`snaps = np.ones`) | R = r_scale/1 = 0.40–0.55, while ratings are about 0.04 SD. | **OPEN. Re-graded LOW → HIGH.** With P0 = diag(.05,.02,.01), S ≈ 0.63, so the gain on total is about 0.13 at week 1, and changepoint z ≈ 0.04/0.79 ≈ 0.05. `detect_changepoints` (`:381`) can never reach `z_thresh = 3`, so auto-interventions are dead on real data. | HIGH | INCR | MUST-NOT-REPRODUCE |
| **#15** | `statespace.py:183-188` (`talent_init = nanmean(y[:3])`); `KalmanState.init` `statespace.py:46-51` (zeros) | Cold start: the batch filter's week-1 *filtered* estimate peeks at weeks 2–3 (a lookahead). Priors are not wired into either path. | **OPEN.** The lookahead is baked into the golden `k_total_filt`/`k_total_pred` (`tests/grid/golden_master.py:88-111`) and into the verdict's smoothed talent (the RTS output is retrospective, so that use is benign). | HIGH | GOLDEN (filtered/predictive series) | MUST-NOT-REPRODUCE in the filtered path (use x0 from a prior, or a diffuse init). |
| **A9** | `:240-244` | Re-running an earlier week silently no-ops, so stat corrections can't be applied. | **OPEN.** Compounds with NEW-W1. | MOD | INCR | DESIGN-INPUT (idempotent recompute from week-keyed deltas). |
| **A8** | `:127-135`; `statespace.py:103-118` | "1D Kalman state not handled". | **FALSE-POSITIVE.** `load()` only returns widths 2 or 3 (a width-2 state is migrated) and `init` is always 3. Residual: a 1-D `mu` on disk raises IndexError at `mu.shape[1]` (`statespace.py:103`) instead of returning None. | LOW | NONE | TYPED-FAILURE on state-file schema mismatch. |
| **G5** | `:105-141` | `var_total` omitted scheme_fit and covariances. | **FIXED** in #63 (`9d30abe`, 2026-06-28). It now uses predictive S, with `sigma[:w,:w].sum()` as the fallback. | — | — | Keep: persist predictive S. |
| **W12** | `backend/api/routes/stats.py:258-268` | Same as G5, but in the API fallback. | **FIXED** in #63. | — | — | — |
| **#25** | `:54-98` | Filed as "offensive ratings stored as def_team grades". | **FIXED as filed.** The code now reads `t_def` intercepts; the fix predates the shallow boundary `c1b6748`. The GitHub issue is stale. **Superseded by NEW-A2 (sign inverted).** | — | — | See NEW-A2 |
| **#31** | — | There is no schedule (player → opponent) table, so matchup grades are never keyed to opponents. | **OPEN.** nflverse schedules are the natural source. | MOD | NONE | DESIGN-INPUT (an ingestion crate artifact). |
| **#33** | `:105-170` | `trajectory_factor` stub. | **PARTIAL.** The engine side (per-week Kalman snapshots) exists as the `kalman_trajectory` table. The trade side is app. | LOW | NONE | — |
| **NEW-W4** | `:210-214` | When called without frames, V(s) is fit on `load_grid_plays([season])`, i.e. the *whole* season. In backfill that leaks future weeks into dV. | **OPEN.** | MOD | INCR | MUST-NOT-REPRODUCE (as-of V(s), as `backtest.py:165-168` does). |
| **NEW-W5** | `weekly_update.py:488`, `data_pipeline.py:262`, `compute_valuations.py:221` (`write_health(status="ok")`) | Health is reported "ok" unconditionally, even when every year failed. | **OPEN** (ops). | LOW | NONE | Evidence records with non-zero exit. |

### 3.3 State-space (`backend/grid/statespace.py`)

| ID | Location | Finding | Status / evidence | Sev | (a) | (b) |
|---|---|---|---|---|---|---|
| **NEW-S1** | `kalman_two_component` `:192-233` (rust inflation for `post_event_games=2` via `games_since_event`) vs. `kalman_step` `:397-411` (inflation only in the intervention week; the state carries no `games_since_event`) | The batch and incremental filters disagree after interventions. | **CONFIRMED** (`kparity.py`, same x0/P0, intervention at week 6): identical through week 6, then \|Δ total_filt\| = 0.0093, 0.0039, 0.0037 … With the default x0, the `nanmean(y[:3])` init vs. zeros makes them differ from week 1. | MOD | INCR vs. GOLDEN | MUST-NOT-REPRODUCE. One filter core with persisted event counters. |
| **G4** | `:237-245` (RTS `Ps[w] = Pf + C(Ps₊ − Pp₊)Cᵀ`); `Pp_next += 1e-10·I` | Smoothed covariance isn't guaranteed PSD. | **OPEN in theory; not reproduced.** 3,000 random stress runs (`psd.py`, R from 1e-4/4000 to 0.4, gaps, interventions) gave min eigenvalue 0.0, max asymmetry 2e-17, and no negative `var_tau_smooth`. | LOW | NONE | TYPED-FAILURE plus symmetrize. Prefer a stable form, e.g. a square-root or Joseph-style smoother. |
| **NEW-S2** | `tests/grid/test_tier0_recovery.py:125-128` | Focus-QB predictive **NIS = 4.58** (observed) against a gate of `≤ 10.0`. Under default `SSParams()` the filter is about 4.6× overconfident on the canonical synth trajectory. The gate is too loose to be a meaningful parity target. (The QB r_scale = 0.55 calibration in `_POSITION_PARAMS` is not used by Tier-0/golden, which run `SSParams()`.) | **OPEN** (calibration weakness). | MOD | GOLDEN | DESIGN-INPUT. Re-gate after calibration, and freeze the NIS value itself as the parity number. |
| **#46** | `SSParams` | Time-varying R/d (DLM) | Enhancement | — | — | Research |

### 3.4 Cross-league priors (`backend/grid/priors.py`)

| ID | Location | Finding | Status / evidence | Sev | (a) | (b) |
|---|---|---|---|---|---|---|
| **G8** | `:103-111` | Small shared pools break the OOS R² computation. | **CONFIRMED.** n=2 or 3 gives `oos_r2 = −inf` (divide-by-zero warning). n=0 gives KeyError; n=1 gives a sklearn ValueError. | MOD | NONE (synth n is large) | TYPED-FAILURE (min-n guard) |
| **#48** | `:103-111` | A single 70/30 split with seed 3 is unstable (±0.15). | **OPEN.** | LOW | GOLDEN-adjacent (Tier-0 `oos_r2 ≥ 0.05`, observed 0.149) | DESIGN-INPUT (k-fold) |
| **#23** | `:93-94` docstring vs. `:143` | `league_factor` is computed and documented but never applied. | **OPEN.** | MOD | NONE | DESIGN-INPUT. Per-league slopes estimated from data, not a constant table. |
| **#49** | `:41-68` | Age/draft step functions (±0.05 at 24/30; round-1 +0.08). | **OPEN.** Also see NEW-P4: on the real rating scale these are more than 1–2 SD. | MOD | NONE | DESIGN-INPUT |
| **NEW-P1** | `:148-149` | `prior_var` is a hand-set `PRIOR_SD[pos]²`, not derived from the equivalency residual variance. | **OPEN.** | MOD | NONE | DESIGN-INPUT |
| **NEW-P2** | `verdict.py:280`; `features.py:20-23,116-126` | Priors are not wired to any real-data path. `prior_mean` is always 0.0 in the verdict, and #15 is unaddressed. | **OPEN** (doc claim false, see §1). | MOD | REAL | — |
| **NEW-P3** | `priors.py:174-197`, `layers.py:613-631`, `statespace.py:419-448`, `value.py:113-123`, `synth.py:320-328` | The `__main__` self-tests use `from synth import …`, which fails under `python -m`. | **OPEN** (dead code). | LOW | NONE | DROP |
| **NEW-P4** | `priors.py:26,41-68`; `statespace.py:46-51,121-147`; `_POSITION_PARAMS :128-133` | Scale constants are calibrated on synth units: QB ability SD 0.09, RAPM SD about 0.26. Real RAPM SD is about 0.04 for all positions (`real_rapm.py`), so P0 (talent SD 0.22), `PRIOR_SD` QB 0.16, and the ±0.05 / +0.08 adjustments are 4–5× too wide or large for real data. | **OPEN.** | HIGH (real) | REAL | DESIGN-INPUT. Make scale constants data-derived per run and record them in the model spec. |
| #44 | — | Hierarchical Bayesian equivalency | Enhancement | — | — | Research |

### 3.5 Synthetic generator (`backend/grid/synth.py`)

| ID | Location | Finding | Status | Sev | (a) | (b) |
|---|---|---|---|---|---|---|
| **G3** | `:292` (`tp.position == "DEF"`) | With `cb_split=True`, CBs are excluded from the team defensive mean. | **OPEN. Re-graded CRIT → MINOR.** It only affects `cb_split=True` configs (interaction tests). The default and the golden use `cb_split=False`. | LOW | NONE | MUST-NOT-REPRODUCE (use `isin(["DEF","CB"])`). |
| **G7** | `:260` (`drive_points or 0.0`) | Float truthiness. | **FALSE-POSITIVE.** It is a no-op, because `drive_points` is always 0.0 on a stalled drive. It's a style issue only. | — | — | — |
| **G11** | `:324-326` | `__main__` divides by `terminal.mean()`. | **FALSE-POSITIVE** in practice: every drive has a terminal play. It's dead code. | — | — | DROP |
| **NEW-Y1** | `:285-294` | The team-strength definition is `off − def` (see NEW-A1). | **OPEN** (design). | HIGH | GOLDEN | DESIGN-INPUT |
| **NEW-Y2** | `:37-38` (`ABILITY_SD`, `STARTER_BONUS`); `_pick_onfield :107-128` (starters ~80%) | The synth QB effect is about 6× the real effect, and starters rotate out 20% of snaps, unlike real QBs. That makes the synth easier than real data exactly where NEW-A3 bites. | **OPEN.** | MOD | GOLDEN | DESIGN-INPUT. Add a "realistic" synth profile, e.g. QB ~99% snaps and scale matched to real RAPM, to the Rust parity suite. |

### 3.6 Value model and plays-contract adapter (`backend/grid/value.py`, `nflverse_adapter.py`, `data_adapters.py`)

| ID | Location | Finding | Status | Sev | (a) | (b) |
|---|---|---|---|---|---|---|
| **NEW-V0a** | `nflverse_adapter.py:91-92` (filters only `play_type ∈ {pass,run}` and `down` notna) | There is **no `season_type` filter**, so postseason plays (weeks 19–22; 1,638 rows in 2023) enter RAPM, V(s), the walk-forward origins and the verdict. | **CONFIRMED** (`real_contract.py`). The GRID-Engine alpha-spec scores weeks 1–17 (`alpha-spec.md:160`). | MOD | REAL | MUST-NOT-REPRODUCE (an explicit season_type policy). |
| **NEW-V0b** | `nflverse_adapter.py:36,106` | `drive_points` maps "Opp touchdown" (502 plays in 2023) and "Safety" (62) to 0, not negative. V(s) ignores defensive scoring, and the V(s) state has no clock or score. | **OPEN.** Documented as a scaffold (`:20-22`). | MOD | REAL | DESIGN-INPUT (the EP target definition in the model spec). |
| **NEW-V0c** | `data_adapters.py:39-46,76-77` | The `REAL_LOADERS["pbp"]` stub raises NotImplementedError, while the real path (`nflverse_adapter.load_grid_plays`) bypasses the documented "swap point". | **OPEN** (stale contract). | LOW | NONE | DESIGN-INPUT (a single loader trait). |
| **NEW-V0d** | `nflverse_adapter.py:205-213` | `part.loc[k]` with duplicate (game_id, play_id) keys returns a DataFrame, and `_split_ids` would then raise on an ambiguous `pd.isna(Series)`. | **OPEN** (latent; 2023 had no duplicates). | LOW | NONE | TYPED-FAILURE |

### 3.7 nflverse box-score ingest (`backend/grid/nflverse_loader.py`, `backend/pipeline/data_pipeline.py`)

All the numbers in this section come from `cmp_stats.py`: CN's `_normalize_pbp` plus `_aggregate_stats_from_pbp` on `play_by_play_2023.parquet` (weeks ≤ 18), compared with the official `player_stats_2023.parquet` (REG), inner-joined on player_id.

| ID | Location | Finding | Status / evidence | Sev | (a) | (b) |
|---|---|---|---|---|---|---|
| **NEW-I1** | `data_pipeline.py:50-62`; `nflverse_loader.py:66` | `pass_attempts` counts sacks (nflverse `pass_attempt` includes sacks), and `passing_yards = Σ yards_gained` nets sack yardage. | **CONFIRMED.** 19,658 vs. 18,315 (+7.3%), and 119,092 vs. 128,567 (−7.4%). 1,459 sack rows carry a passer. Completions, INTs, targets and receptions match exactly. | CRIT | REAL | MUST-NOT-REPRODUCE |
| **NEW-I2** | `nflverse_loader.py:73-77` (`td_type` from `touchdown`), consumed at `data_pipeline.py:57,73,89` | `touchdown` = *any* TD on the play, so pick-sixes and fumble-return TDs on offensive plays are credited as passing/receiving/rushing TDs. | **CONFIRMED.** passing_tds 814 vs. 754, receiving_tds 799 vs. 754, rushing_tds 473 vs. 470. There were 66 offensive plays with `touchdown=1` but neither `pass_touchdown` nor `rush_touchdown`. | CRIT | REAL | MUST-NOT-REPRODUCE (use `pass_touchdown`/`rush_touchdown` and `td_player_id`). |
| **G10** | same lines | Row-wise `df.apply` for `td_type`. | **OPEN** (perf). The real defect at these lines is NEW-I2. | LOW | NONE | — |
| **A1** | `nflverse_loader.py:78`; `data_pipeline.py:97-114` | "fumbles_lost counts ALL fumbles". | **FALSE-POSITIVE** as filed: the column is sourced from `fumble_lost` (also the #94 triage). | — | — | — |
| **NEW-I3** | `data_pipeline.py:101-105` | Fumble *attribution* is wrong: `fumbler = rusher if rush else receiver`. Sack fumbles (no receiver) are dropped, and QB fumbles on pass plays are charged to the targeted receiver. | **CONFIRMED.** fumbles_lost 174 vs. 256 official (−32%), with 47 players mismatched. | HIGH | REAL | MUST-NOT-REPRODUCE (use `fumbled_1_player_id` + `fumble_lost`). |
| **NEW-I4** | `nflverse_loader.py:53-54` (no `season_type` filter) → `player_stats` weeks 19–22 → `compute_valuations.py:44-66` (`SUM` over all weeks of the season), `verdict.py:533-562` (history) | Postseason games are summed into season totals and per-game rates, which inflates playoff teams' players. | **CONFIRMED** (1,638 postseason rows in the normalized 2023 pbp). | HIGH | REAL | MUST-NOT-REPRODUCE |
| **NEW-I5** | `data_pipeline.py:34-139`; `scoring/columns.py` | `two_point_conversions` is never populated (always 0). Rush attempts are −2.8% (14,178 vs. 14,588) because kneels are excluded. | **CONFIRMED.** | MOD | REAL | DESIGN-INPUT (stat definitions in the spec). Owner decision #3. |
| **G2** | `nflverse_loader.py:81` | `complete_pass` scalar 0 when the column is missing. | **FALSE-POSITIVE.** The scalar broadcasts to the existing index; real nflverse always ships the column (#94 triage agrees). | — | — | — |
| **A15** | `data_pipeline.py:181` | `week=0` is not rejected. | **OPEN** (trivial; pbp never has week 0). | LOW | NONE | TYPED-FAILURE |
| **A7** | `compute_valuations.py:65` (`IN ('QB','RB','WR','TE','K')`) | There is no DEF, and K is selected but nothing aggregates K stats, so K and DEF projections are empty. | **OPEN** (a missing feature: no DST/K aggregator). | MOD | NONE | DESIGN-INPUT (alpha-spec scopes K/DST to Phase 2, scored separately; `alpha-spec.md:154`). |
| **G9** | `cache.py:16-23` (`get`), `:25-28` (`put`) | Expired files are never deleted, and there's no stale-while-revalidate. Also (NEW): `put` is non-atomic, so a crash mid-write leaves a "fresh" partial parquet, and `_path` key sanitizing collides (`"a/b"` and `"a_b"` map to the same file). | **OPEN.** The TTL `>` boundary test passes on Linux. | LOW | NONE | MUST-NOT-REPRODUCE (temp file + rename; content-addressed keys). |
| **NEW-I6** | `tests/grid/test_nflverse_loader.py:47,62` | Tests write to the global `/tmp/test_cache` instead of `tmp_path`. | **OPEN** (test hygiene). | LOW | NONE | — |

### 3.8 Projection (`backend/projection/*`, `backend/fantasy_scoring.py`)

| ID | Location | Finding | Status / evidence | Sev | (a) | (b) |
|---|---|---|---|---|---|---|
| **P1** | `model.py:74-83` | A NaN `prior_mean` crashed `Ridge.predict`. | **FIXED** in `ccc1ffe` (squashed into #94 / `91ae204`, 2026-07-17): `nan_to_num → 0.0`. Residual design issue: 0.0 conflates "no prior" with "prior = 0", and the fit side `fillna(0.0)` (`:138`) does the same. | — | — | DESIGN-INPUT (an explicit missing-indicator feature). |
| **P2 / P6** | `fantasy_scoring.py:32` (`max(0.0, NaN) → 0.0`), `:57` | NaN handling. | **OPEN, but the module is orphaned.** Only `tests/test_fantasy_scoring.py` imports it. | LOW | NONE | **DROP** (don't port; delete in the reference). |
| **P3** | `volume.py:78-81` | A missing `VOLUME_COLS` column crashes `groupby.agg`. | **OPEN** (latent; callers fill the columns). | LOW | NONE | TYPED-FAILURE |
| **P4** | `model.py:134-145` | A missing stat or feature column gives a KeyError. | **OPEN** (latent; `verdict._fit_ros_models` fills `SCORING_STAT_COLS`). | LOW | NONE | TYPED-FAILURE |
| **P5** | `sv_to_points.py:84` (`np.polyfit`) | NaN credit or points: `np.ptp(NaN) == 0` is False, so polyfit runs on NaN. | **OPEN.** | MOD | NONE | TYPED-FAILURE |
| **P7** | `sv_to_points.py:82` | Exact `== 0` spread check; near-zero spread still produces a degenerate slope. | **OPEN.** | LOW | NONE | Tolerance-based guard |
| **NEW-R1** | `model.py:134-145` | The per-unit rate regression is **unweighted**: `y = stat/volume` from 1-attempt players weighs the same as from 600-attempt players. It's heteroscedastic, so low-volume noise dominates. | **OPEN.** | HIGH | REAL | MUST-NOT-REPRODUCE (WLS by volume, or a Poisson/binomial GLM). |
| **NEW-R2** | `volume.py:79` (`games = nunique(week)` over `player_stats` rows); docstring `:12-14` | "Games" counts only weeks with a stat row, so active-but-no-touch weeks are dropped and backups' per-game rates are inflated. The docstring says the shrinkage weight is "learned", but `k_shrink = 8.0` is a constant. Postseason rows are included (NEW-I4). | **OPEN.** | MOD | REAL | MUST-NOT-REPRODUCE (games from participation/snaps; fit k by EB). |
| **NEW-R3** | `verdict.py:78-93` + `tier1.py:67-95` | The weekly H2 GRID forecast is `affine(RAPM rating)`. The map is fit on (first-origin rating, pre-origin weekly points) pairs, and the warm-up weeks' points are explained by a rating estimated from those same weeks, so the fit is in-sample within the pre-period. The Kalman filter is **not** used in the weekly verdict forecast; it only feeds ROS `smoothed_talent`. | **OPEN** (methodology). | MOD | REAL | DESIGN-INPUT |
| **A4** | `compute_valuations.py:208` (writes `week=0`) vs. `trades/trade_model.py:57-67` | There is no per-week projection producer; the season total is stored under a `week=0` sentinel. | **OPEN.** Deferred in #94 as Tier-B. Engine-relevant only as the projection output contract. | LOW | NONE | DESIGN-INPUT (a typed horizon field, not a sentinel). |

### 3.9 Validation (`backend/validation/*`)

| ID | Location | Finding | Status / evidence | Sev | (a) | (b) |
|---|---|---|---|---|---|---|
| **NEW-V1** | `lineup_sim.py:187-210`, `tier1.py:296-304`, `metrics.py:134-180` | H1/H2 CIs are **iid percentile bootstraps** over correlated cells. H2 pools every (roster × week) margin, and rosters share realized weekly outcomes. H1 pools (player × origin) pairs, and successive ROS errors overlap. The CIs are too narrow. GRID-Engine requires **week-clustered** paired bootstrap (`alpha-spec.md:856-857,882`). | **OPEN.** | HIGH | REAL (the headline H2 "PASS" CI) | MUST-NOT-REPRODUCE |
| **NEW-V2** | `tests/validation/test_tier2.py:32` (planted grades); `tier2.py` | Tier-2 has never been exercised on engine-produced grades. On synth, the Tier-2 KPI from walk-forward `def_grades` is about 0 for *either* sign, with CIs that include 0 (`defgrade_test.py`, 3 seeds). An intercept-only grade carries no validated signal, and adding on-field defender ratings didn't help on synth (`defgrade_test2.py`). | **OPEN.** | MOD | REAL | DESIGN-INPUT (define the grade as total defensive effect; gate on real points-allowed). |
| **V2** | `backtest.py:178-181` | The watermark guard only fires within the same season. | **OPEN.** Also note the walk-forward accumulates across seasons with no decay (`:191-198`). | MOD | REAL | MUST-NOT-REPRODUCE (check across seasons; multi-season weighting is a design input). |
| **V3** | `metrics.py:34-37` | `normal_cdf` uses `np.vectorize(erf)`. | **PARTIAL.** A scalar returns `np.float64`, not a 0-d array, so that part is a false positive. The slowness is real. | LOW | NONE | — |
| **V4** | `metrics.py:209-220` | `crps_gaussian` with `sd=0` gives NaN (should be \|actual−mean\|). | **CONFIRMED.** | MOD | NONE | TYPED-FAILURE or a closed-form limit |
| **V5** | `metrics.py:192-201` | `nis` with `var ≤ 0` gives inf. | **CONFIRMED.** | MOD | NONE | TYPED-FAILURE |
| **V6** | `metrics.py:248-250` | `pinaw` returns 0.0 when the actual range is 0. | **OPEN.** | LOW | NONE | TYPED-FAILURE / NaN |
| **V12** | `metrics.py:296-298` | `spearman` returns 0.0 (should be NaN) with no variance; the Tier-2 KPI averages these. | **CONFIRMED.** | LOW | NONE | NaN plus exclusion |
| **V7** | `sniff.py:33-44,73` | Non-numeric or None ranks raise TypeError. | **OPEN.** | LOW | NONE | TYPED-FAILURE |
| **V8** | `asof.py:92-114` (`slice_pool`) | After a parquet round-trip, participation cells are `numpy.ndarray` and the code raises TypeError. | **CONFIRMED.** Latent: only tests call `slice_pool`. | MOD | NONE | MUST-NOT-REPRODUCE (typed participation lists). |
| **V9** | `ingest_grid.py:29-39` | `len()` on None/NaN participation. | **OPEN, latent.** The adapter always emits tuples. | LOW | NONE | — |
| **V10** | `ingest_grid.py:63-81` | Non-atomic snapshot writes; the manifest is written last. | **OPEN.** | MOD | NONE | MUST-NOT-REPRODUCE (temp + rename, manifest hash). |
| **V11** | `baselines.py:73-102` | An empty history returns `sd={}` rather than None. | **OPEN.** | LOW | NONE | Typed Option |
| **V1** | `backtest.py:101-112` | See G1. | **OPEN.** | — | — | — |
| **#41** | — | Testing gaps. | **Engine part mostly FIXED.** `tests/grid/test_layers.py`, `test_kalman_numerical.py`, `test_tier0_recovery.py`, `test_golden_master.py` and others now exist. There is still no coverage tooling, and no end-to-end real-data test. | LOW | — | — |

### 3.10 Scoring (`backend/scoring/*`)

| ID | Location | Finding | Status | Sev | (a) | (b) |
|---|---|---|---|---|---|---|
| **A3** | `format_registry.py:36-83` | `resolve_or_create_format` returned the wrong id on a name collision. | **FIXED** in `ffb3074`/`868cfa6` (squashed into #94 / `91ae204`). | — | — | Residual (NEW-C1) |
| **NEW-C1** | `format_registry.py:54,60` (`config.to_json()` string equality) | Config identity is the raw JSON string, so it is key-order-sensitive and two equal rule sets can create duplicates. | **OPEN.** | LOW | NONE | Canonical (sorted) serialization plus a content hash. |
| **A5 / #47** | `vor.py:92-119` (`denom = abs(prev_vor) if prev_vor != 0 else 1.0` at `:112`) | Tiers fragment near zero and negative VOR. | **OPEN.** | MOD | NONE | MUST-NOT-REPRODUCE (absolute plus relative gap). |
| **A6** | `vor.py:47-71` | FLEX replacement was skipped for positions not in `roster_slots`. | **FIXED** in #85 (`64216d6`, 2026-07-01), before the audit date. | — | — | — |
| **NEW-C2** | `vor.py:38` vs. `CLAUDE.md` ("rank `slots*num_teams`") | Replacement is at 0-based index `slots*num_teams`, i.e. rank +1. The code and its own docstring agree; the CN `CLAUDE.md` is off by one. | Doc nit | LOW | NONE | State it precisely in the spec. |

### 3.11 Research and enhancement items to carry, not port

| ID | Item | Target crate (GRID-Engine) | Note |
|---|---|---|---|
| #42 | Matrix factorization on RAPM residuals | `models` | Post-MVP. |
| #43 | Copula / correlated outcomes (filed against trade_model) | `simulation` | Engine-relevant for joint lineup and roster distributions. |
| #44 | Hierarchical equivalency priors | `models` | Fixes #23/#48 holistically. |
| #45 | Player × defence embedding for matchup edges (filed against trade_model) | `models` / `features` | Depends on #31 and on a validated matchup signal (NEW-V2). |
| #46 | Time-varying R / discount (DLM) | `models` | Do NEW-S1/S2 first. |
| #49 | Continuous age/draft curves | `models` | Must respect the NEW-P4 scale. |

---

## 4. Implications for the reference oracle (`reference/python/`)

**Trust map.** These outputs can be frozen as parity targets as-is:

- The **synth Tier-0 player recovery numbers**: overall 0.8026; QB 0.869, RB 0.743, WR 0.797, TE 0.771, DEF 0.765. G1 barely moves them (0.8036 when fixed).
- RTS/smoothed synth outputs.
- `metrics.py` on non-degenerate inputs.
- Scoring arithmetic (`engine.calculate_points`).

These outputs **must not** be frozen as-is:

- Golden Layer B/C team ratings and qb_credit (G1, G14, NEW-A5).
- Golden `k_total_filt`/`k_total_pred` (the #15 lookahead).
- The Tier-0 team gate (G1).
- Anything from `weekly_update` (§3.2).
- Every real-data number: the H1/H2 verdict, `valuations`, `player_stats` (§3.7, NEW-V1).

**Recommended oracle procedure:**

1. Import `reference/python/` at 59bce1d *unchanged* as `legacy`.
2. Apply a minimal, documented "oracle-fix" patch set (G1/NEW-A1 per the decision below, NEW-A2, #15 init, NEW-I1..I4) as separate reviewed commits. Each one should come with a failing test first (alpha-spec §6, "Implementation rules" item 2).
3. Regenerate the golden with a written model-spec change for each patch. Silent golden regeneration is not allowed (§6 item 3).

Keep both goldens. Rust targets the fixed one, and the diff between the two is the documented divergence list.

**GBM caveat.** The golden passes at 1e-5 on Linux only because V(s) and Layer-1 use sklearn `HistGradientBoostingRegressor` under `threadpool_limits(1)`
(`tests/grid/golden_master.py:51-62`); it fails on Windows. Rust will not bit-match sklearn HGB, so parity tests should be **stage-isolated**:
feed Python-produced `dv` into Rust RAPM, and Python-produced weekly credit into the Rust Kalman filter. End-to-end comparison should use
recovery correlations, not 1e-5.

---

## 5. Recommended fix order for downstream agents

1. **Ingest correctness** (NEW-I1..I4, NEW-V0a): fixes the real-data foundation.
2. **G1 / NEW-A1** convention decision, then G1 in all 3 sites, then a golden regen with a model-spec note.
3. **NEW-A2** matchup sign (all four sites: `weekly_update.py:84`, `backtest.py:185-186`, the docstrings, and the tests at `test_weekly_update.py:286-370`).
4. **#15** prior/diffuse init in the filtered path.
5. NaN/degenerate guards (P5, V4, V5, V8, G8), either as typed failures in Rust or as guards in the reference.
6. Design inputs for the Rust incremental loop: NEW-W1..W4, #24, NEW-S1, NEW-W3.
7. Methodology: NEW-V1 (week-clustered bootstrap), NEW-R1 (weighted rates), NEW-A3 and NEW-P4 (real-data scale and QB identifiability).
