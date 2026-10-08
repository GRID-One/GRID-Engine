# Spec-first reconciliation: GRID-Engine specs vs the Python GRID engine

Author: spec-first inventory agent. This is an independent second opinion; a separate agent did
the code-first pass, and I did not read its report before writing this one.

Sources read:
- `/home/user/GRID-Engine` @ `3823478` (branch `claude/grid-engine-consolidation-e7kmh9`):
  `alpha-spec.md` (2186 lines, byte-identical to `docs/00-meta/specs/alpha-spec.md`),
  `final-build-spec.md` (818 lines), `docs/99-templates/template-model-spec.md`, `docs/CLAUDE.md`,
  `docs/04-providers/*/README.md`, ADR-001/006, and `crates/*/src/lib.rs`. All twelve crates are
  doc-comment stubs and no model spec exists yet. I skimmed the review branches `origin/claude/grid-alpha-adversarial-review-ikkjuk`
  and `origin/claude/adversarial-review-p1-00-l32emp`. They review P1-00 bootstrap and CI only and
  contain no engine-semantics findings.
- `/home/user/cautious-nevermore` @ `59bce1d`: `backend/grid/*`, `backend/projection/*`,
  `backend/validation/*`, `backend/scoring/*`, engine parts of `backend/pipeline/*`, `tests/{grid,projection,validation,scoring}`,
  `docs/05-current-state.md`, `docs/06-issues-log.md`, `docs/04-lessons-learned.md`,
  `docs/10-next-steps-plan.md`, `docs/02-backend-spec.md`. I also read deleted docs from git history:
  `git show 379750e^:docs/superpowers/plans/2026-06-23-retrospective-validation-suite.md`
  (cited as "validation plan"), `...-product-roadmap-to-alpha.md` (cited as "roadmap", §4.5 especially),
  and `...2026-07-09-phase2c-verdict-and-ros-gap.md`.

Evidence I produced (all in `scratch-specfirst/`, with neither repo touched):
- **Engine test suite:** I exported a copy of `cautious-nevermore` HEAD to `scratch-specfirst/cn` and ran
  `OMP_NUM_THREADS=1 python3 -m pytest tests/grid tests/projection tests/validation tests/scoring tests/test_fantasy_scoring.py`
  in it. Result: **380 passed in 142 s**. The Layer-C golden master passes on Linux single-thread, so the
  failures that `docs/10-next-steps-plan.md` reports are Windows or multithread drift.
- **Experiment 1:** `scratch-specfirst/sign_exp.py` and `sign_exp2.py`, which test the team-strength sign convention (§4 C1).
- **Experiment 2:** a one-line synthetic-generator fix in `scratch-specfirst/cnfix/backend/grid/synth.py`. Experiment 1 was rerun on that copy (§4 C1).
- **Experiment 3:** `scratch-specfirst/def_sign.py`, which tests the sign of the defensive matchup grade on both synth variants (§4 C2).

Status legend: **SAT** means satisfied by the reference (file and function cited). **PARTIAL** means part is
there and the missing part is named. **ABSENT** means not there. **CONTRA** means the reference does something
different; each one says which side is right and why.

---

## 1. Executive summary

1. **Coverage is narrow.**
   - The two specs describe a six-layer stat-vector projection engine:
     - Layer A, availability
     - Layer B, team environment
     - Layer C, opportunity allocation
     - Layer D, efficiency
     - Layer E, matchup
     - Layer F, correlated simulation
   - Around those layers the specs require versioned, point-in-time, promotion-governed operation.
   - The Python reference is a strong *talent-attribution* engine:
     - V(s), giving dV
     - RAPM with a market anchor
     - Kalman `[talent, form, scheme_fit]` with RTS smoothing
     - cross-league priors
   - It also has a thin projection layer (EB volume × ridge rates) and a good validation and leakage harness.
   - Of the six spec layers it covers part of C and D and a research-grade E. It has nothing for A, B or F,
     and nothing for distributions, versioning, snapshot/rollback or promotion.
2. **The reference's own real-data evidence does not yet clear the spec's Phase-1 model gate.**
   - On real data the H1 ROS margin vs last-season actuals is **−0.015 [−0.059, +0.029]** (a tie).
   - It is −0.194 vs season-to-date mean (`docs/05-current-state.md`, phase2c doc).
   - alpha-spec §9.4 requires beating **every** naive baseline by PB-MAE and the strongest by **≥3%**.
   - The reference therefore fails the spec's gate today. That is not a reason to drop GRID, but GRID's value
     has to be re-proven inside the spec's Layers A–F rather than assumed.
3. **The reported H2 "PASS" (+0.848 weekly lineup margin) is not a valid live claim under the spec's as-of rules.**
   - Walk-forward RAPM uses *current-season* FTN participation.
   - nflverse publishes that participation once, after the postseason (validation plan §15; alpha-spec §4.1 row
     "Participation … No for current in-season use … cannot create train/serve skew"; §12.3).
   - The bootstrap is also iid over roster-weeks, not week-clustered as §7.7 requires.
4. **The synthetic oracle has a structural bug.**
   - `synth.py:193` draws the on-field defenders from the **offensive** team: `_pick_onfield(tidx[off_team], rng)`.
   - I measured 100% of synth plays with defenders from the offense team and 0% from `def_team`.
   - As a result the defence and matchup side of GRID (team-defence intercepts, Layer-3 team strength, matchup
     grades) has never been validated against a correct planted truth.
   - The "fix" queued in `docs/10-next-steps-plan.md` for G1/V1 (market row `[+1,−1]`) would cement that bug.
     The right fix is in the synthetic generator plus the team-rating definition (§4 C1).
5. **The defensive matchup grade sign is inverted.**
   - `backtest.py:185` and `weekly_update._upsert_matchup_grades` store `rapm_grade = −β_def` labelled
     "higher = tougher".
   - Empirically, −β_def correlates **+0.41** (current synth) and **+0.59** (fixed synth) with points allowed,
     so a higher grade means an easier matchup.
   - The synth Tier-2 test cannot catch this: it builds `points_allowed` from the grades themselves
     (`tests/validation/test_verdict.py:47-51`).
6. **The production and validation paths feed the Kalman different signals.**
   - The backtest and goldens feed weekly Layer-1 credit.
   - `weekly_update.py:307-309` feeds the **cumulative season-to-date RAPM rating** as if it were an independent
     weekly observation, with `snaps = np.ones`.
   - The Kalman filtered state is also not used in any evaluated forecast. The weekly forecast is
     `a + b·RAPM` (`tier1.forecast_weekly_points`), and ROS uses only frozen pre-origin smoothed talent.
7. **Recommendation:**
   - Keep GRID as the **talent and efficiency signal source inside Layers D and E**, consistent with roadmap §4.5:
     "GRID cannot *be* the projection — it is the most valuable *feature* in one".
   - Generalize GRID's Kalman/discount/intervention machinery into the spec's Kalman layer for opportunity
     shares and team pace (§6.2).
   - Re-cast Layer-3 market reconciliation as the Layer-B team-environment anchor, using lines as of lock.
   - Split GRID into a participation-free **live path** and a participation-dependent **offseason RAPM prior**.
   - Before porting, **fix the oracle** (synth defenders, sign conventions, Kalman init look-ahead, silent
     fallbacks), regenerate its goldens, then freeze the corrected oracle as the Rust parity target.
   - Make parity stage-wise, because the reference's V(s) and Layer-1 use sklearn `HistGradientBoostingRegressor`,
     which Rust cannot reproduce bit-for-bit.

---

## 2. The engine the specs require (engine-only extraction)

What follows strips out every Flutter, FFI, Windows packaging, UI, installer and AI-process requirement.
Section numbers refer to `alpha-spec.md` (prefix "α") and `final-build-spec.md` (prefix "F").

### 2.1 Pipeline stages

```
E0 Acquire      once-daily fetch → raw retained + hashed (F§8.3, F§9, α§4.1.1)
E1 Normalize    validate, quarantine bad rows, PARTIAL_SUCCESS, typed failure states (F§9.2–9.5)
E2 Commit       data version (idempotent per ingestion_run_id) (F§9.3, F§14)
E3 Identity     gsis_id canonical; source link table; NCAA tiered matching (α§4.4)
E4 As-of        per-source timestamps; lock snapshots; three-season window (α§2.4, §4.5, §7.2)
E5 Features     deterministic, versioned feature schema (F§10; α§11 families)
E6 Layers A–F   availability → team env → opportunity → efficiency → matchup → simulation (α§6.1)
   + methods    ridge, RAPM, Kalman, RTS/fixed-lag, EB, affine, boosting (α§6.2, F§11)
   + priors     NCAA prior q-blend + EB posterior (α§6.3)
   + ensemble   OOF-stacked components with constrained weights (α§6.4)
E7 Score        affine stat-vector → points; re-score the same draws (α§5.4, F§11.7)
E8 Evaluate     rolling-origin backtest, player pool, PB-MAE + secondary metrics, paired
                week-clustered bootstrap, gates (α§7.1–7.11)
E9 Govern       candidate → validate → promote/reject; snapshot/rollback; versioning chain;
                durable resumable stages (F§12.3–16, α§10.3)
```

### 2.2 Inputs (α§4)

- **nflverse datasets** (α§4.1 table): PBP, player weekly stats, team weekly stats, schedules (with
  spread and total), players, rosters, depth charts, snap counts, NGS, PFR advanced, FTN charting,
  participation (historical only), and injuries (unavailable post-2024).
- **Freshness metadata** per source (α§4.1.1):
  `source_name, source_version, source_timestamp, retrieved_at, ingestion_run_id, content hash, row count,
  schema version, validation status`.
  - "A feature is eligible only if its source timestamp is earlier than the projection lock."
- **Availability fallback** (α§4.1.2): `availability_overrides.csv`
  (`player_id,season,week,status,practice_status,expected_active_probability,expected_snap_multiplier,source_note,observed_at`).
  - Missing data must raise uncertainty and never be read as healthy.
  - OUT/IR/PUP/suspension/bye/inactive force p_active = 0.
- **NCAA (CFBD)** (α§4.2): rosters, season and game stats, usage, PPA, recruiting, team context, draft picks.
  - Call-budget policy applies (α§4.2.2).
  - Per-position feature lists are in α§4.2.3.
  - "College fantasy points are never inserted directly."
- **Context** (α§4.3): weather, inactives, practice participation, OL changes, market spread and total.
  - Manual versioned imports are allowed in Phase 1.

### 2.3 Identity (α§4.4)

- `gsis_id` is canonical.
- Use the `player_identity_links` table with the columns listed at α§4.4.2.
- NCAA→NFL matching uses five tiers.
- Ambiguous matches are never auto-promoted ("a false positive is worse than a missing NCAA prior").
- A correction creates a new link version and triggers a feature rebuild.

### 2.4 As-of and leakage (α§2.4, §4.5, §7.2, §12.3)

**Three-season window:**
- For W>1, the window is season-to-date through W−1 plus S−1 and S−2.
- For preseason and Week 1, it is S−1, S−2 and S−3.
- Static metadata may predate the window.

**Timestamps to record:**
- source publication
- retrieval
- feature computation
- projection
- benchmark
- kickoff

**Thursday and Sunday locks** are configurable and persisted. A post-lock run is a new version and never
an overwrite.

**Leakage tests must fail the build for** (α§4.5, §12.3):
- later-week stats
- final status
- future depth charts
- postgame participation
- later stat corrections
- a season summary that includes the target

**Further rules:**
- Current-season participation that is unavailable at serve time must not appear in promoted live features.
- A stat correction creates a new data version.

### 2.5 Output contract (α§5)

**Stat vector per position** (α§5.1):
- **QB:** active probability, start probability, attempts, completions, passing yards, passing TD, INT,
  sacks taken, rush attempts, rushing yards, rushing TD, fumbles lost, 2-pt.
- **RB:** active probability, snap share, carries, rushing yards, rushing TD, targets, receptions,
  receiving yards, receiving TD, fumbles lost, 2-pt.
- **WR/TE:** active probability, snap share, targets, receptions, receiving yards, receiving TD, carries,
  rushing yards, rushing TD, fumbles lost, 2-pt.

**Conditional and unconditional** (α§5.2): both projections are persisted.

**Distribution per player × week × scoring profile** (α§5.3):
- mean, median, sd
- P10, P25, P75, P90
- floor and ceiling definitions
- P(zero or inactive)
- P(> threshold)
- boom and bust

**Scoring** (α§5.4): `fantasy_points = w · stat_vector + offset`. The same simulated draws are re-scored
across profiles. Half-PPR is the default (α§2.3, verbatim weights). Standard and PPR are built in, and
custom profiles are versioned affine transforms.

### 2.6 Modeling system (α§6, F§11)

**Layer A — availability** (α§6.1). Predicts:
- p_active
- p_start
- snap multiplier if active
- P(limited role)

It uses roster status, depth chart, snaps, missed time, injury state and teammate availability.

**Layer B — team game environment.** A *joint* team/game distribution of:
- plays, drives, pass and rush attempts, sacks
- TDs by type, red-zone trips
- pace and neutral pass tendency, game script

The two teams share correlated latents.

**Layer C — opportunity allocation.** Shares of dropbacks, designed rushes, carries, targets, air yards,
red-zone and goal-line opportunities. Unconstrained logits go through a **softmax/simplex plus
roster-aware normalization**, and team totals act as constraints.

**Layer D — efficiency.** Rates:
- completion
- yards per attempt
- catch rate
- yards per target and per reception
- yards per carry
- TD conversion
- fumble rate

"High-variance rates, especially touchdowns, are strongly shrunk."

**Layer E — matchup and context.** Opponent, venue, surface, rest, travel, weather, QB, OL and game script,
all as of lock.

**Layer F — seeded correlated Monte Carlo.**
- Carries sum to team rushes, targets sum to team targets.
- Completions ≤ attempts and receptions ≤ targets.
- TDs align with team scoring.
- Inactive players produce 0.
- Depth-chart scenarios are coherent.
- Use 5,000 draws per game in Phase 1, with a deterministic seed per prediction version.

**Method map** (α§6.2):

| Method | Intended use |
|---|---|
| Ridge | baselines and stacking |
| RAPM | "where participant data supports them; **research-only if live feature parity is absent**" |
| Kalman | pace, pass tendency, opportunity share, efficiency |
| Fixed-lag RTS | revision after stat corrections |
| EB | priors and TD/rate shrinkage |
| Affine | college→NFL translation and scoring |
| GBM | residuals and availability |

"No single method is promoted because it is architecturally required."

**NCAA prior** (α§6.3):
```
theta_prior = q·theta_ncaa + (1−q)·theta_pos_draft
theta_post  = (n0·theta_prior + n_eff·theta_nfl)/(n0 + n_eff)
```
- `n0` is learned per position and component by rolling-origin validation.
- The influence cap applies where translation is weak.
- Role and efficiency are separated.

**Ensemble** (α§6.4) has five components:
1. recency baseline
2. hierarchical/ridge
3. Kalman
4. GBM residual
5. optional sparse adjusted-effect (RAPM)

Stacking uses out-of-fold weights only, constrained, and the simpler model wins ties.

**Explainability** (α§6.5) must cover:
- team environment
- role and opportunity
- availability adjustment
- matchup adjustment
- NCAA prior contribution
- recent-evidence contribution
- top drivers
- uncertainty drivers
- delta vs the prior published projection

Language must be predictive, not causal.

**Model-spec discipline** (α§6.6): a model spec comes before code. Required behaviours:
- typed failures for NaN, singular systems, non-convergence, invalid probabilities, share overflow and impossible stats
- seeded and recorded randomness
- no hyperparameter selection on benchmark weeks
- independent numerical review

**F§11 primitives:**
- **Linear algebra:** nalgebra/sprs with dimension assertions.
- **Ridge** (F§11.2): arbitrary X, λ, regularized intercept, weights, diagnostics. Dense uses Cholesky; sparse uses CG.
- **RAPM** (F§11.3):
  - Explicit objective `argmin ||y−Xβ||² + λ||β||²`.
  - Sparse CG with diagnostics `converged, iterations, residual_norm, tolerance, regularization_lambda`.
  - "A numerically failed solve must not silently produce a production model."
- **Kalman** (F§11.4): x/P persisted, with daily batch predict/update.
- **RTS** (F§11.5): a full mode and a fixed-lag mode.
- **EB** (F§11.6): persist prior mean and variance, population variance, counts, shrinkage parameters and version.
- **Affine** (F§11.7): A, b, inverse, dimension validation, deterministic serialization.
- **Boosting** (F§11.8): `IncrementalBooster` trait (load_or_init, continue_training, predict, save, metadata).
  `xgb` is primary and a hand-rolled `linfa-trees` booster is the fallback.

### 2.7 Incremental learning, versioning and governance (F§12–16, α§10.3)

**Daily pipeline** (F§12.1, α§8.6). The listed order is:
1. features
2. snapshot model state
3. Kalman + RAPM-incremental + EB
4. GBM continuation/replay
5. validation
6. promote or reject

**GBM modes** (F§12.2): continuation, replay-window and periodic rebuild.

**Snapshot and rollback** (F§12.3): snapshot first. Degenerate output (NaN, divergence) triggers an
automatic rollback and leaves production untouched.

**Resumability** (F§12.4, §15): durable stages and jobs, with the job fields listed at F§15.

**Model states** (F§13): `TRAINING, VALIDATING, CANDIDATE, PRODUCTION, REJECTED, SUPERSEDED`. Metrics are
persisted per model.

**Version record** (F§14): `model_id, model_version, model_type, feature_schema_version,
data_snapshot_version, training_run_id, training_start/end, hyperparameters, random_seed,
validation_metrics, build version`. The chain is `Data → Feature → Model → Prediction`, and an incremental
update creates a new version.

**Crash rules** (F§16): never partially promote; never overwrite production before validation; resume;
treat artifacts as immutable.

**Live invariants** (α§10.3): a lock is immutable; promotion is serialized; competitor projections never
enter training.

### 2.8 Evaluation (α§7)

**Rolling origin** (α§7.1): reconstruct the snapshot at lock, then fit, freeze, score and persist.

**Player pool** (α§7.3) is the union of the top N by our model, by each provider and by actual results:
- N = QB 20, RB 40, WR 50, TE 15
- byes are excluded
- an inactive player after lock **stays in the pool with 0**
- a surprise player is included
- a missing provider projection gets a documented penalty

**Primary metric — PB-MAE** (α§7.4):
- Per position: `NMAE_p = MAE_p/scale_p`, with the scale fixed from the training period only.
- `PB-MAE = mean over QB, RB, WR, TE`.

**Secondary metrics** (α§7.5):
- raw MAE and RMSE by position
- MedAE
- Spearman
- start/sit accuracy
- Accuracy Gap
- active Brier score
- CRPS
- 50% and 80% coverage
- quantile calibration
- bias by slice
- weekly win rate

**Statistics** (α§7.7): paired errors; **bootstrap by week (optionally game-within-week)**; 95% CIs;
metrics versioned before results are seen.

**Gates:**
- **Phase-1 model gate** (α§9.4):
  - beat every naive baseline on PB-MAE
  - beat the strongest naive baseline by ≥3%
  - no core position >1% worse
  - NCAA priors help on low-evidence players or get disabled
  - 80% intervals cover 72–88%
- **Phase-2 competitive gate** (α§7.8).
- **Claim gate** (α§7.9).
- **Correctness gate** (α§7.10).

**Naive baselines** (α§9.2):
- prior-game fantasy points
- rolling three-game average
- season-to-date average
- position/depth-chart median

### 2.9 Testing (α§12, F§19), engine-relevant parts only

- **Unit tests:**
  - scoring
  - pool
  - share normalization
  - simulation invariants
  - NCAA prior
  - identity
  - Kalman predict/update
  - fixed-lag
  - EB
  - ridge
  - sparse solver diagnostics
  - metric formulas
- **Golden synthetic tests:**
  - team volume
  - shares
  - stat-line means
  - quantiles
  - scoring
  - PB-MAE
  - Brier and calibration
  - promotion decisions
- **Leakage tests** (α§12.3).
- **Integration tests:**
  - raw → SQLite → features → model → projection
  - NCAA → prior → projection
  - override → new version
  - benchmark → lock → score
  - crash per stage
- **Failure tests** (α§12.5, F§19.4).
- **Anti-shortcut list** (α§12.7).

---

## 3. Requirement-by-requirement reconciliation

`cn/` below means `/home/user/cautious-nevermore/`. Line numbers are at `59bce1d`.

### 3.1 Inputs, identity, ingest

| ID | Requirement (spec ref) | Status | Evidence / gap |
|---|---|---|---|
| R01 | nflverse PBP ingest (α§4.1) | **SAT** | `cn/backend/grid/nflverse_loader.py` (`load_raw_pbp`, URLs at lines 5-15); `nflverse_adapter.build_plays_contract` turns PBP into the GRID plays contract (drive segmentation, next state, `drive_points`). |
| R02 | Player weekly stats as authoritative labels (α§4.1) | **CONTRA** | Labels are re-derived from PBP in `cn/backend/pipeline/data_pipeline._aggregate_stats_from_pbp`. Per the nflfastR dictionary `pass_attempt` includes sacks, so attempts are inflated and `passing_yards` include sack losses. Plays with `down` NA are dropped in `_normalize_pbp`, so 2-pt conversions are never counted (always 0). Laterals are ignored. **Spec is right:** use nflverse `player_stats` weekly as labels, versioned with the correction window, and keep the PBP aggregate as a cross-check. |
| R03 | Rosters, participation (α§4.1) | **SAT** | `load_rosters`, `load_participation` (FTN, 2016–2025). |
| R04 | Schedules + spread/total, depth charts, snaps, NGS, PFR, FTN charting (α§4.1) | **ABSENT** | None are loaded. The market loader `data_adapters.load_odds` raises NotImplementedError. |
| R05 | Freshness metadata per source (α§4.1.1) | **PARTIAL** | `ingest_grid.py` writes `manifest.json` with row counts and participation coverage only. It has no content hash, source_version, retrieved_at, run id or schema version. Snapshots are "idempotent (overwrites in place)", so they are not immutable. `cache.ParquetCache` is mtime and TTL only. |
| R06 | Availability override import; missing ≠ healthy (α§4.1.2) | **ABSENT** | No availability inputs. The closest analogue is Kalman "predict without update" for absent weeks (`statespace.kalman_two_component`; `verdict._weekly_obs_on_timeline`, which sets absent weeks to `y=NaN, played=False`). That grows talent variance but does not produce p_active. |
| R07 | CFBD adapter + call budget (α§4.2) | **ABSENT** | `data_adapters.load_cfbd` raises NotImplementedError, so there is no real `feeder_sv` source. |
| R08 | Idempotent ingest per run id (F§9.3) | **PARTIAL** | `data_pipeline` upserts rows (ON CONFLICT / INSERT OR REPLACE), so it is idempotent but has no run id. `weekly_update.run` is "idempotent" only by skipping when `last_week >= week` (`weekly_update.py:240`). That key has no season, so the first week of a new season is skipped after the prior season reached week ≥ 1 (the check at `:240` runs before the dimension check; cf. issue-log A9). A dimension or order mismatch silently reinitialises the accumulators and drops all history (`:247, :252`). |
| R09 | Partial-success and typed failure states (F§9.4-9.5) | **ABSENT** | `weekly_update.py:215-219` catches any exception, logs a warning and returns `rapm_solved=False`. `layers.layer1_all_qbs:555` has `except Exception: pass` (issue #27). |
| R10 | `gsis_id` canonical (α§4.4.1) | **SAT (NFL)** | Roster and participation ids are GSIS (`verdict._load_players_frame`). |
| R11 | Identity link table, NCAA tiers, review (α§4.4.2-3) | **ABSENT** | — |

### 3.2 As-of, leakage, windows

| ID | Requirement | Status | Evidence / gap |
|---|---|---|---|
| R12 | Outcome vs pre-game information cutoffs (α§4.5) | **SAT (week-granular)** | `cn/backend/validation/asof.py` `AsOf.outcome_cutoff` (W−1), `pregame_cutoff` (W). Slicing is season-aware (`_asof_keep_mask`). |
| R13 | Per-source publication timestamps; feature eligible iff `source_ts < lock` (α§4.1.1, §4.5) | **ABSENT → causes a live leak** | AsOf keys on the week index only. The plan itself said "design the accessor with an information-timestamp per source now" (validation plan §8). This was never done. The concrete consequence is R14. |
| R14 | Current-season participation must not enter live/promoted features (α§4.1 row, §1.2, §12.3) | **CONTRA** | `validation/backtest.walk_forward` folds current-season weeks (with FTN participation) into RAPM at every origin. That participation is published once, after the postseason (validation plan §15; roadmap §7 "In-season RAPM staleness"). So H2 (+0.848) and the weekly Tier-1 tables score information that was not available at lock. In a live season `weekly_update` would find no participation file for S (it is published only after the postseason), and the broad `except` at `:215` would turn that into a silent "no PBP data" skip. **Spec is right.** Re-run with participation for season S gated to "available after S's postseason", and treat in-season RAPM as research-only per α§6.2. |
| R15 | Metamorphic future-poisoning, tripwire, watermark, two-path tests (α§12.3) | **SAT (strong asset)** | `cn/tests/validation/test_leakage_guards.py` (poisoning is bit-identical, with a canary for each guard). Gaps: the watermark skips season seams (`backtest.py:180`, issue-log V2). Two-path equivalence holds only for injected frames, because `weekly_update.py:213` refits V(s) on `all_plays` for the whole season, which leaks future weeks when replaying history, while the backtest freezes V(s). |
| R16 | Three-season window (α§2.4) | **ABSENT** | RAPM accumulators grow without bound (`layers.accumulate`; `backtest.walk_forward`; `weekly_update`). The real run used 2 seasons, which is why it complied. |
| R17 | Thursday/Sunday lock snapshots, immutable (α§7.2, §10.3) | **ABSENT** | — |
| R18 | Stat corrections create a new data version (α§12.3) | **ABSENT** | — |
| R19 | Intervention foreknowledge prevented | **SAT** | `AsOf.slice_interventions`; coaching-change resets keyed by `week_effective` (`weekly_update.py`). |
| R20 | Filter is causal (implied by α§4.5 for any "filtered" estimate) | **CONTRA (minor)** | `statespace.kalman_two_component:186` sets `talent_init = nanmean(y[:3])` when `x0` is absent, so week-1 and week-2 filtered and predictive values use y[1..2]. The calibration test drops only week 1 (`tests/grid/test_calibration_synth.py:97`). **Fix:** initialise from the prior (`x0`/`P0` from priors or a position mean), never from future y. |

### 3.3 Output contract

| ID | Requirement | Status | Evidence / gap |
|---|---|---|---|
| R21 | Stat-vector-first (α§5.1) | **PARTIAL** | `cn/backend/scoring/columns.py` `SCORING_STAT_COLS` (14 box-score stats); `projection/model.StatLineModel.project_stat_line` produces the stat line and scores it afterwards. **Missing:** active and start probability, snap share, sacks taken. `fumbles_lost` and `two_point_conversions` are "deliberately unmodeled" (0.0) (`model.py:22,53`). The weekly forecast path (`tier1.forecast_weekly_points` → `sv_to_points`) maps rating straight to points, which skips the stat vector and contradicts α§5.1 for weekly. |
| R22 | Conditional + unconditional (α§5.2) | **ABSENT** | — |
| R23 | Distribution outputs (α§5.3) | **ABSENT as model output** | The only uncertainty is (a) the Kalman predictive variance S in dV currency (`kalman_step(return_pred=True)`) and (b) a pooled past-error Gaussian sd computed *inside evaluation* (`tier1.tier1_report`). No quantiles, P(zero), P(>thr) or boom/bust. |
| R24 | Scoring as versioned affine transform (α§5.4, F§11.7) | **PARTIAL** | `scoring/engine.calculate_points` is linear `Σ w·stat`. There is no offset, no version id and no inverse or serialization contract. `formats.py` STANDARD/HALF/FULL match α§2.3 weights exactly. |
| R25 | Re-score the same draws (α§5.4) | **ABSENT** | There are no draws. |

### 3.4 Modeling layers

| ID | Requirement | Status | Evidence / gap |
|---|---|---|---|
| R26 | Layer A availability (α§6.1) | **ABSENT** | Interventions and changepoints (`statespace.detect_changepoints`, `post_event_r_mult` "rust") are regime-change machinery for *efficiency*, not availability. |
| R27 | Layer B team environment, joint and correlated (α§6.1) | **ABSENT** | Team off/def intercepts in RAPM are opponent-adjusted team *efficiency* in EP/play. There is no plays, pace or pass-rate model and no game coupling. |
| R28 | Layer C opportunity allocation with simplex and team constraints (α§6.1) | **PARTIAL (weak)** | `projection/volume.project_volume` does EB shrinkage of per-game usage toward the position mean, with weight `g/(g+k_shrink)` and `k_shrink=8.0` fixed rather than learned, plus an override hook. **Missing:** team totals, softmax/simplex, roster-aware normalization, depth chart, snap share. It uses prior-season only ("Mid-season blending … deferred", `volume.py` comment), so **in-season weekly volume is absent**. |
| R29 | Layer D efficiency with strong TD shrinkage (α§6.1) | **PARTIAL** | `projection/model.fit_stat_line_model` fits a per-position, per-(volume, stat) ridge of the per-unit rate on `[rapm_rating, smoothed_talent, prior_mean]` (`alpha=1.0`, standardized). The ridge shrinks toward the mean rate, but there is no explicit EB TD shrinkage, no opportunity-support rule and no fumble-rate model. GRID enters here, which matches the roadmap §4.5 principle. |
| R30 | Layer E matchup (α§6.1) | **PARTIAL / research; sign bug** | Situation RAPM (`layers.run_situation_rapm`), WR×CB interactions (`build_design(interactions=True)`), team-defence intercepts as matchup grades. **Sign is inverted** (§4 C2). Tier-2 has never been run on real data (`05-current-state.md`: "Skipped (no points-allowed feed)"). Context such as venue and weather is absent. |
| R31 | Layer F seeded correlated simulation (α§6.1) | **ABSENT** | `validation/lineup_sim.py` is an evaluation harness, not a projection simulator. |
| R32 | Ridge primitive with diagnostics (F§11.2) | **PARTIAL** | `layers._solve_ridge_prior` is dense `np.linalg.solve` with a prior mean and a per-column penalty mask. It has no diagnostics and no weights. `projection/model` uses sklearn `Ridge`. |
| R33 | RAPM sparse CG, diagnostics, no silent failed solve (F§11.3) | **CONTRA** | The solve is dense normal equations. When `cond > 1e10` it **logs a warning and returns `lstsq`** (`layers.py:356-358`), and that behaviour is tested as intended (`tests/grid/test_rapm_robustness.py::test_solve_ridge_prior_lstsq_fallback`). **Spec is right:** an ill-conditioned or non-converged solve must be a typed failure that blocks promotion. Port as sparse CG plus the diagnostics struct. The domain controls in F§11.3 (garbage time, OT, minimum appearances, possession weighting) are **ABSENT**; the play weight is uniform. |
| R34 | Incremental RAPM (F§12.1) | **SAT** | `layers.init_accumulators/accumulate/fit_from_accumulators`. Additive equivalence is tested (`tests/grid/test_incremental.py::test_accumulate_two_weeks_equals_full`). |
| R35 | Kalman online state persisted (F§11.4) | **PARTIAL** | `statespace.KalmanState` (`mu`, `sigma`, `player_ids`, npz) and `kalman_step` (vectorized predict, Joseph-form update). It is not persisted to SQLite, and the file is overwritten in place with no version. The production observation is wrong (§4 C3). |
| R36 | RTS full + fixed-lag (F§11.5, α§6.2) | **PARTIAL** | Full RTS is in `kalman_two_component` (PSD jitter `1e-10`). **Fixed-lag is absent.** |
| R37 | EB with persisted prior parameters (F§11.6) | **PARTIAL** | The volume EB has a fixed `k_shrink`. The prior SDs are constants (`priors.PRIOR_SD`), and so is `KalmanState.init` `diag(0.05,0.02,0.01)` (`statespace.py:50`). Nothing is persisted or versioned. |
| R38 | Affine primitive (F§11.7) | **PARTIAL** | `projection/sv_to_points` fits per-position `(a, b)` with `np.polyfit`. Priors use an affine feeder→NFL map. There is no generic affine type. |
| R39 | Boosting behind an incremental trait (F§11.8, §12.2) | **PARTIAL** | sklearn `HistGradientBoostingRegressor` for V(s) (`value.fit_value_model`) and the Layer-1 context model g (`layers._cross_fitted_context_residual`). There is no continuation, replay or trait. Note that this is a GBM *on the critical path of GRID itself*, not the "residual" role α§6.2 assigns. |
| R40 | NCAA prior q-blend + EB posterior with learned n0 (α§6.3) | **PARTIAL / different form** | `grid/priors.py`: `estimate_equivalency` (OLS of NFL rating on `feeder_sv` over shared players, single 70/30 split, issue #48) and `build_priors` (`prior_mean = a + b·feeder_sv + age/draft step adjustments`, `prior_var = PRIOR_SD[pos]²`). The Kalman update with prior variance P0 is algebraically the spec's EB posterior with `n0 = R/P0`, so the form is compatible. **Missing:** q (identity confidence, sample size, comparability); learned n0 (PRIOR_SD is hand-set); position/draft-prior blend; role vs efficiency separation; `LEAGUE_FACTORS` is "computed but never applied" (issue #23). Not wired into Kalman init (issue #15). On real data `prior_mean` is a constant 0.0 (`verdict._fit_ros_models`). |
| R41 | Ensemble with OOF stacking (α§6.4) | **ABSENT** | There is one model per path. |
| R42 | Explainability fields (α§6.5) | **ABSENT** | Talent, form and scheme_fit trajectories exist (`kalman_trajectory` table via `weekly_update._write_kalman_trajectory`) and could feed the "recent evidence" and "uncertainty" fields. |
| R43 | Typed numeric failures (α§6.6.4) | **CONTRA** | Examples of silent fallbacks: the `lstsq` fallback (R33); NaN→0.0 in `RateModel.predict` (`nan_to_num`); `SVToPointsMap.predict` returns 0.0 for an unfitted position; `except Exception: pass` (R09); `max(1.0, snaps)` floors. Some of these are documented design choices, but they all violate α§6.6.4 and α§12.7 ("converting failures into defaults"). |
| R44 | Seeded + recorded randomness (α§6.6.5) | **PARTIAL** | Seeds are fixed (`random_state=0`, `seed=0`, `KFold(shuffle, random_state)`) and determinism is gated in-process at <1e-9 (`tests/grid/test_determinism.py`), single-thread golden (`threadpool_limits(1)`). Seeds are not recorded in any version record. |

### 3.5 Evaluation

| ID | Requirement | Status | Evidence / gap |
|---|---|---|---|
| R45 | Rolling-origin weekly backtest (α§7.1) | **SAT (mechanics)** | `validation/backtest.walk_forward`: frozen pre-period V(s), incremental accumulators, `origin_stride` logged. **Gaps:** no lock snapshots, no three-season window, no projection freezing or persistence beyond the in-memory `OriginResult`. |
| R46 | Player pool = union of top-N by model / provider / actual; inactive stays with 0 (α§7.3) | **CONTRA** | `tier1.tier1_report:259` drops any forecast without realized rows (`if row.player_id not in realized: continue`). Inactive players are removed, which is survivorship bias and also contradicts validation plan §5 ("No survivorship … score injuries/benchings as the low outcomes they were"). Skill is computed on "paired cells", not a pre-declared union pool. **Spec is right.** |
| R47 | PB-MAE primary (α§7.4) | **ABSENT** | There is per-position MAE (`tier1`), but no `scale_p` normalization or PB-MAE. The primary decision metric is the H1 ROS margin vs last-season (`verdict.run_verdict`). |
| R48 | Secondary metrics (α§7.5) | **PARTIAL** | `validation/metrics.py` has MAE, RMSE, bias, Spearman, top-N, NDCG, CRPS (Gaussian), PICP+PINAW (any level), PIT, pinball and NIS, and `lineup_sim` has a win rate. **Missing:** MedAE, start/sit accuracy, Accuracy Gap, active Brier, bias slices beyond position. |
| R49 | Paired, **week-clustered** bootstrap, 95% CI (α§7.7) | **CONTRA** | `metrics.bootstrap_ci:170` resamples individual cells iid. Tier-1 margins are per player-origin. For `horizon="ros"`, adjacent origins share most of the realized ROS window, so the cells are heavily dependent. H2 resamples roster-weeks iid while rosters share players' realized points. The CIs are therefore too narrow. **Spec is right.** |
| R50 | Naive baselines (α§9.2) | **PARTIAL** | `validation/baselines.py` has persistence (= prior-game FP ✓), season-to-date (✓, with sd), last-season (extra) and market (wrapper). **Missing:** rolling 3-game average and position/depth-chart median. |
| R51 | Metrics/thresholds versioned before results (α§7.7, §12.7) | **CONTRA (design)** | `validation/thresholds.py` "calibrate-then-gate" promotes gates as `mean + 1.96·SE` of a sign-flip null computed **from the same run being judged** (`verdict.run_verdict`, `_sign_flip_null`). The committed registry ships empty (`provisional_thresholds.json`). Freeze-once is good practice, but it freezes after seeing the data. **Spec is right** for the primary gate: pre-register the α§9.4 thresholds. The noise-floor null is acceptable only as an extra significance test, or on a designated calibration period disjoint from evaluation. |
| R52 | Phase-1 model gate (α§9.4) | **NOT MET** | The real H1 margin is a tie with last-season, and the reference is worse than season-to-date (−0.194 [−0.270, −0.120]). The gate requires beating every naive baseline by ≥3%. Calibration is reported as PICP@80 ≈ 0.75–0.87 (inside the 72–88% band) and ROS NIS 1.25. |
| R53 | Correctness gate before accuracy (α§7.10) | **PARTIAL** | The leakage suite is strong (R15), but R14 and R46 invalidate the current real-data numbers. |
| R54 | Benchmark registry, provider comparison, claim gates (α§7.6-7.9) | **ABSENT** | `baselines.market` is a wrapper only. |

### 3.6 Versioning, governance, durability

| ID | Requirement | Status | Evidence / gap |
|---|---|---|---|
| R55 | Data→Feature→Model→Prediction version chain (F§14) | **ABSENT** | No ids. `fit_value_model(cache_path=…)` "silently overwrites any previously cached model" (docstring, `value.py`). |
| R56 | Feature schema version (F§10) | **ABSENT** | `TALENT_FEATURE_COLS` is a module constant. |
| R57 | Snapshot before update + auto-rollback on degenerate output (F§12.3, §16) | **ABSENT** | `KalmanState.save` and `save_accumulators` overwrite in place. `KalmanState.load` returns `None` on an unknown width, which reinitialises silently. |
| R58 | Model states and promotion (F§13) | **ABSENT** | The verdict *renders* SHIP_GRID or BASELINE_FALLBACK and "is rendered, not acted on" (`verdict.py` docstring). |
| R59 | Resumable stages / job ledger (F§12.4, §15) | **ABSENT** | — |
| R60 | Structured diagnostics with ids (F§17) | **PARTIAL** | `pipeline/_logging.get_logger` writes prose logs, `health_check.write_health` writes JSON, and there are no run ids. |

### 3.7 Testing

| ID | Requirement | Status | Evidence / gap |
|---|---|---|---|
| R61 | Golden numerical tests on fixed synthetic data (α§12.2, F§19.2) | **PARTIAL (strong for talent)** | `tests/grid/test_golden_master.py` + `golden_master.py` + `golden/snapshot.npz`. Layer A covers semantic invariants anchored to planted truth, Layer B covers orderings, and Layer C covers numerics at rtol 1e-5 and atol 1e-6. The goldens cover ratings, team ratings, QB weekly credit and the focus-QB Kalman trajectory (288 players, 12 teams, 14 weeks). **Missing** from α§12.2: team volume, shares, stat means, quantiles, scoring, PB-MAE, Brier, promotion decisions. |
| R62 | Recovery gates (synthetic-truth contract) | **SAT (but oracle bug)** | `tests/grid/test_tier0_recovery.py` floors, with observed values in brackets: pooled ≥0.77 (0.80); QB ≥0.83, RB ≥0.70, WR ≥0.76, TE ≥0.73, DEF ≥0.73; team ≥0.60 (0.66); Kalman total ≥0.92, tau ≥0.60; rookie prior ≥0.50; equivalency slope 0.9–1.8, OOS R² ≥0.05; NIS ≤10. `tests/grid/test_calibration_synth.py` checks QB-pooled NIS in [0.8, 1.4], median in [0.8, 1.25], PIT, and PICP@80 in [0.70, 0.90]. The DEF and team gates are measured against a mis-specified generator (§4 C1). |
| R63 | Unit tests per primitive (α§12.1, F§19.1) | **PARTIAL** | Kalman Joseph-form PSD and RTS PSD (`test_kalman_numerical.py`), incremental equivalence, and design-matrix vectorization parity (`test_performance.py`). **Missing:** share normalization, sim invariants, identity, fixed-lag, EB posterior. |
| R64 | Failure tests (α§12.5) | **ABSENT** (beyond a few robustness tests) | — |

**Counts across the 64 rows:**

| Status | Rows |
|---|---|
| SAT or SAT-with-caveat | ≈ 10 |
| PARTIAL | ≈ 24 |
| ABSENT | ≈ 20 |
| CONTRA | 10 (R02, R14, R20, R33, R43, R46, R49, R51, and C1/C2/C3 below) |

---

## 4. Contradictions the specs don't enumerate (engine semantics)

These sit inside GRID itself, where the specs are silent or generic. Each one needs a decision before
the reference can be called an oracle.

### C1 — Synthetic defence and team-strength estimand (blocks parity and the G1/V1 fix)

**What the code does.**
- `cn/backend/grid/synth.py:193` is `off_pl, def_pl = _pick_onfield(tidx[off_team], rng)`. The function
  returns both offence and defence from one team index, so **the on-field "defence" is the offence's own
  defenders**. I measured this with a small script: defenders belong to the offence team on 100% of plays
  and to `def_team` on 0%.
- The planted team strength `_team_strength` (`synth.py:293`) is `mean(off starters) − mean(def starters)`.
  That is coherent only with this bug, because a team's own good defenders lower its own offence's yards.
- RAPM's `team_rating = β_off − β_def` (`layers.py:426`) matches that convention.
- The Layer-3 market row anchors `β_off + β_def` (`layers.py:404-405`, `backtest.py:108-109`,
  `weekly_update.py:272-273`), which does not match it.

**Evidence.** I ran `sign_exp.py` and `sign_exp2.py` with `OMP_NUM_THREADS=1`.

On the current synthetic generator:

| Measure | corr |
|---|---|
| corr(γ_off−γ_def, planted off−def) | 0.693 |
| corr(γ_off+γ_def, planted off+def) | 0.471 |
| realized team point margin vs planted off+def | 0.427 |
| realized team point margin vs planted off−def | 0.719 |

With defenders drawn from `def_team` (one-line patch):

| Measure | corr |
|---|---|
| realized margin vs planted **net** (off+def) | **0.855** |
| realized margin vs off−def | 0.754 |
| γ_off+γ_def vs planted net, no market | 0.431 |
| γ_off+γ_def vs planted net, with market anchored on net, `[+1,+1]` | **0.542** |
| γ_off−γ_def (the current `team_rating`) vs planted net, with that market | 0.412 |

Player recovery also improves with the patch:

| Position | corr with fixed synth |
|---|---|
| QB | 0.887 |
| RB | 0.767 |
| WR | 0.851 |
| TE | 0.775 |
| DEF | 0.782 |

**What is right.**
- Under the design convention, with offence +1 and defence −1 in `build_design`, a stronger defence has a
  **larger** β_def.
- A team's net strength, the thing a spread prices, is therefore **γ_off + γ_def**, plus the corresponding
  on-field player sums.
- The market row `[+1,+1]` is correct. What is wrong is `team_rating` (it should be `γ_off + γ_def`) and the
  synthetic generator (defenders should come from `def_team`, and planted strength should be off + def quality).
- The queued "1-char fix" to `[+1,−1]` (`docs/10-next-steps-plan.md`; `docs/04-lessons-learned.md`
  "Market reconciliation sign error") would make the code self-consistent with the buggy generator and should
  **not** be ported.
- The intercept-only team rating is weak in either convention, at about 0.4–0.5. A team-strength readout should
  add starters' ratings, or be defined from team-level RAPM.

**Statistical owner's decision.** The team-strength estimand, the fixed synthetic generator, and
regenerated goldens.

### C2 — Matchup grade sign inverted

**What the code does.**
- `backtest.py:185` and `weekly_update._upsert_matchup_grades` store `−β_def` as "higher = tougher". The
  docstring reasons that "a stronger defence (more negative intercept, because defenders enter X with −1)".
- That reasoning is backwards. Because the column enters with −1, a stronger defence fits a *more positive*
  coefficient.

**Evidence.** I ran `def_sign.py`.

| Synth | corr(β_def, planted def quality) | corr(−β_def, points allowed) |
|---|---|---|
| current | −0.22 (meaningless, because of C1) | +0.41 |
| fixed | +0.53 | +0.59 |

So the stored grade is an "easiness" score.

**Why the tests miss it.** The Tier-2 synth test is tautological: it builds `points_allowed = 20 − 30·grade`
from the grades themselves (`tests/validation/test_verdict.py:47-51`).

**What is right.** Store `+β_def` (or `−` of opponent-adjusted points allowed). Gate it with a Layer-A
semantic test against planted defence quality on the **fixed** synthetic generator.

### C3 — What the Kalman observes in production vs validation

**What the code does.**
- In the backtest, the goldens and the calibration tests, the Kalman observes **weekly Layer-1 credit**: the
  mean cross-fitted residual per snap, with `R = r_scale/snaps`.
- In `weekly_update.py:307-309` it observes `obs = ratings_arr` (the cumulative season-to-date RAPM solve)
  with `snaps = np.ones(n_players)` (issue #24).
- A cumulative estimate fed as a fresh observation every week double-counts evidence and breaks R's meaning.
  The variance band persisted to `kalman_trajectory` is then not calibrated.

**What is right.** The weekly Layer-1 credit, which is what the state-space docstring and the calibration were
built on. This is a bug, not a decision.

Also note that **no evaluated forecast uses the filtered Kalman state**:
- The weekly forecast is `sv_to_points(RAPM rating)`.
- ROS uses frozen pre-origin smoothed talent.

So the spec's "Kalman latent-state model" component (α§6.4) has never been scored on real data.

### C4 — GBM inside GRID's core vs α§6.2 roles

- V(s) and the Layer-1 context model `g` are sklearn HGBR. α§6.2 places GBM as residual, availability or
  component-rate models.
- This is not a semantic contradiction, but it creates two problems:
  - Rust parity cannot be bit-exact, because the reference also needs `threadpool_limits(1)`.
  - The F§11.8 "continuation/replay" story does not fit a frozen V(s).
- V(s) has 3 integer state features, about 4 × 30 × 99 cells. A deterministic estimator (binned and smoothed,
  or a monotone-constrained boosted model implemented in-house) is feasible and better for determinism.
- **Owner decision**, together with whether nflfastR's published `ep` column may substitute for V(s). It is a
  scope-leak risk in backtests, because the EP model was trained on later seasons.

---

## 5. Proposed unified engine architecture (engine-only consolidated spec)

### 5.1 Principles

1. **Spec layers are the skeleton and GRID provides signals.** Layers A–F (α§6.1) stay the structure.
   GRID components become named signal providers with model specs. This is the roadmap §4.5 conclusion
   ("fantasy points = volume × efficiency … GRID cannot *be* the projection").
2. **There are two GRID paths, split by data availability (α§1.2, §6.2):**
   - **GRID-Live** is participation-free and runs weekly in-season. It uses PBP-identifiable involvement:
     the passer, rusher, target or receiver, and the sacked QB. Opponent adjustment comes from team-level
     ridge (team off/def intercepts need no participation).
   - **GRID-Offseason** is participation RAPM: full on-field ±1 RAPM, refreshed once per year when FTN
     participation for season S is published. Its outputs are prior-season talent ratings that seed the
     Kalman preseason prior and feed Layer-E research.
   - Neither path may use participation for season S before its publication timestamp.
3. **The Kalman machinery is generic.** GRID's `[talent, form, scheme_fit]` with discount `d`, intervention
   spikes, "rust" R-inflation, predict-without-update and Joseph-form update becomes the engine's single
   state-space framework, instantiated per stream (§5.2).
4. **The market reconciles at the team level, as of lock.** Layer 3 generalizes to Layer B. The spread and
   total *at lock* (α§4.3, §11.1) become pseudo-observations on team net strength and team implied points.
   No closing lines are used, and competitor *projections* stay evaluation-only (α§10.3).
5. **Currency.** GRID efficiency lives in EP/play (dV). Conversion to stat components happens in Layer D rate
   models and never through a direct rating→points map. The weekly `sv_to_points` shortcut is retired from
   the production path and kept only as a diagnostic.

### 5.2 Layer mapping

| Spec layer | Unified design | GRID source (reference) | New work |
|---|---|---|---|
| Shared currency (feature) | **V(s) EP model** → `dV` per play. Also used in α§11.1 and §11.3 EPA features. | `grid/value.py` `fit_value_model/compute_dv`; `nflverse_adapter.build_plays_contract` | Deterministic estimator (C4); frozen per data version; model spec |
| B — team environment | Team-week state-space (Kalman) for pace, plays, neutral PROE, drives, TD rates. Team net strength and implied points come from team-level ridge on dV with **market-at-lock pseudo-obs** (generalized Layer 3). Game-level correlated latents. | `layers.run_rapm` team intercepts plus the market rows (sign per C1); `statespace` framework | Plays, pace and PROE models; joint game distribution; model spec |
| A — availability | Rule layer (override and status → p_active = 0/1) plus a GBM or logistic for questionable/doubtful/limited and snap multiplier. Missingness raises variance. | Concepts only: interventions and rust, `detect_changepoints` | Entire layer; override import (α§4.1.2) |
| C — opportunity | Per-player share states (target, carry, snap, air-yard, RZ share) as **Kalman streams** with discount and interventions, plus EB toward depth-chart and position priors, then a softmax/simplex allocator against Layer-B team totals with roster-aware renormalization when players are inactive. | `projection/volume.project_volume` (EB shrink, override hook); `statespace` framework | Share Kalman; simplex allocator; in-season blend; depth chart and snap inputs |
| D — efficiency | Per-component rate models (comp%, YPA, catch%, YPT, YPC, TD conversion, fumble rate), each = EB-shrunk position rate × f(GRID talent). TD conversion heavily shrunk (α§6.1). GRID inputs: GRID-Live filtered `talent` and `form` (EP/play, by role: dropback/carry/target), the GRID-Offseason prior-season RAPM rating, and the NCAA/draft prior. | `projection/model.StatLineModel` (ridge rates on talent features); `statespace.kalman_step`; `layers.layer1_all_players` | Role-specific Layer-1 credit; EB TD shrinkage; per-component model specs |
| E — matchup/context | Opponent defence adjustments from team-level ridge (`+β_def`, C2) by position and role. Situation and WR×CB RAPM stay **research-only** (participation). Venue, rest and weather adjustments when available pre-lock. | `run_situation_rapm`, `build_design(interactions=True)`, def intercepts | Sign fix; real Tier-2 validation; context features |
| NCAA / low-evidence prior (α§6.3) | Kalman **initial state** `(x0, P0)` = q-blend of translated feeder SV and a position/draft prior. P0 = `σ²/n0`, so the Kalman update *is* the α§6.3 EB posterior. `n0` is learned by rolling origin. Role vs efficiency priors separated. | `grid/priors.py` (affine equivalency, `PRIOR_SD`, `washout_table`) | CFBD adapter; feeder-SV college pass; q; learned n0; identity confidence |
| F — simulation | Seeded per-game Monte Carlo: draw game latents (B) → team totals → availability (A) → shares (C, Dirichlet/multinomial around allocator means) → per-player efficiency draws (D, variance from Kalman predictive S mapped through rates) → stat draws with hard constraints → fantasy points by affine scoring. | none | Entire layer |
| Ensemble (α§6.4) | Components = recency baseline, ridge/hierarchical, **GRID-Kalman stat model**, GBM residual, optional RAPM-offseason component; OOF-stacked with constrained weights. | none | Entire |
| Scoring (α§5.4) | Versioned affine `(w, b, profile_id)` applied to draw matrices. | `scoring/engine.calculate_points`, `formats.py` | Offset, version, draw-matrix API |
| Evaluation (α§7) | Port the reference harness (`asof`, `backtest`, `baselines`, `metrics`, leakage guards) and **change**: union player pool with inactive = 0 (R46), PB-MAE (R47), week-clustered bootstrap (R49), pre-registered thresholds (R51), per-source information timestamps (R13/R14), three-season window (R16). Keep H1/H2 as secondary diagnostics. | `backend/validation/*`, `tests/validation/test_leakage_guards.py` | As listed |
| Governance (F§12-16) | Run ledger with stage status; immutable versioned artifacts (V(s), accumulators per season, Kalman states per week, rate models); candidate → validate → promote; auto-rollback on NaN, divergence or invariant violation. | none (in-place npz overwrite) | Entire |

### 5.3 Three-season window implementation (α§2.4) for GRID

- **RAPM accumulators are additive.** Store `XtX_s, Xty_s` **per season**. The design at (S, W) is
  `Σ_{s∈{S−2,S−1}} A_s + A_S[1..W−1]`, and the preseason design is `Σ_{S−3..S−1}`.
- This is exact, cheap, and removes the unbounded growth (R16).
- The column universe must be the as-of player pool (`AsOf.slice_pool`), not the full roster frame. Today
  `walk_forward` builds columns from the fixed `players` frame.
- **Kalman:** the state carries forward, and its discount implements forgetting. Spec conformance means the
  state is re-initialised from the windowed prior at S−3 for replays. This needs the owner's ruling (§8, D5).

### 5.4 Output contract with GRID as the talent signal (engine DTOs, no FFI)

These types live in `crates/domain` and are serialized via serde to Parquet or JSON. Units are explicit.

```text
PlayerWeekProjection {
  ids:      prediction_version_id, model_version_id, feature_schema_version, data_snapshot_id,
            scoring_profile_versions[], seed, lock_type{THU,SUN,OPERATIONAL}, lock_ts, computed_ts
  key:      season, week, gsis_id, position, team, opponent, game_id
  availability: { p_active, p_start, snap_mult_if_active, p_limited,
                  source{override|provider|model|missing}, observed_at }          # Layer A
  stats_conditional:   { <component>: {mean, sd, q10, q25, q50, q75, q90} }       # α§5.1 vector per position
  stats_unconditional: { same }                                                    # α§5.2
  fantasy[profile_id]: { mean, median, sd, p10, p25, p75, p90, p_zero_or_inactive,
                         p_exceed{thr:prob}, p_boom, p_bust }                      # α§5.3
  draws_ref: { matrix_id, n_draws }                                               # re-score w/o rerun (α§5.4)
  grid: {                                                                          # GRID signal, EP/play
    role_talent{dropback|carry|target}: {filtered_mean, filtered_var, predictive_var},
    form, scheme_fit,
    offseason_rapm{rating, season, n_plays} | null,
    prior{mean, var, q, n0, weight_now} | null,                                    # NCAA/draft prior
    regime_flags{intervention, scheme_reset, changepoint_z}
  }
  explanation: { team_env{plays, pass_rate, implied_points}, role{share means},
                 availability_delta, matchup_delta, prior_contribution, recent_evidence_contribution,
                 top_drivers[±], uncertainty_drivers[], delta_vs_prior_version }      # α§6.5
}
TeamGameEnvironment { ids…, plays, drives, pass_att, rush_att, sacks, td_pass, td_rush, rz_trips,
                      pace, neutral_proe, implied_points, net_strength, shared_latent_ids }
```

**Invariants** (typed failures):
- Σ shares ≤ 1 per team-role.
- receptions ≤ targets and completions ≤ attempts per draw.
- p ∈ [0,1].
- Quantiles are monotone.
- An inactive draw produces the zero vector.

### 5.5 Rust crate mapping (trimmed workspace)

Keep `domain, persistence, ingestion, identity, features, models, simulation, scoring, evaluation,
governance`.

**Replace** `application` with an engine orchestration crate (`pipeline`) and a CLI binary (`grid-cli`):
`ingest`, `build-features`, `fit`, `project --season --week --lock`, `backtest`, `promote`, `rollback`.

**Drop** `ffi`, `app/`, `toolchains/flutter.version`, and `flutter_rust_bridge` from `[workspace.dependencies]`.

**Add a `synth` crate.** It is a Rust port of the *fixed* synthetic generator, and the recovery gates need
it in Rust. It must eventually plant stat-vector truth: volume shares, availability and TD conversion (§6).

Inside `models`, map the GRID modules onto the α§8.1 submodules:

| α§8.1 submodule | GRID content |
|---|---|
| `value` (new) | V(s) |
| `rapm` | design, sparse CG, accumulators per season, market pseudo-obs |
| `kalman` + `rts` | statespace, fixed-lag |
| `empirical_bayes` | priors, volume shrink, TD shrink |
| `ridge` | — |
| `boosting` | trait |
| `ensemble` | — |

Layer-1 credit belongs in `features` (it is a derived per-player-week signal), or in `models::attribution`.

---

## 6. Oracle and parity strategy

The reference is to become `reference/python/` and serve as the executable oracle. Six steps follow.

1. **Tag as-imported, then correct.** Import at `59bce1d` unchanged and tag it. Then apply an **oracle-
   correction ledger** in Python first, so that Rust parity targets are spec-correct:
   - C1 synthetic defenders, team_rating, and the market row kept at `[+1,+1]`
   - C2 grade sign
   - C3 Kalman observation
   - R20 causal init
   - R33 typed failure instead of the lstsq fallback
   - R09 removing the `except: pass`
   - R46 inactive = 0
   - R49 clustered bootstrap
   - R14 participation publication gating

   Regenerate `tests/grid/golden/snapshot.npz` under the documented "reviewed semantic explanation" rule
   (α§6.6.3). Do not port known bugs and then "fix" them in Rust, because parity would become meaningless.
2. **Stage-wise goldens, not end-to-end.** Export intermediate artifacts from the corrected oracle on the
   canonical seed:
   - `plays + dV` Parquet → Rust RAPM must match β to rtol 1e-8 (CG tolerance 1e-12). This is independent of
     the GBM.
   - Weekly `(y, snaps, played, interventions, params)` → Rust Kalman, RTS and fixed-lag must match
     filtered, smoothed and predictive values to 1e-12.
   - `college + ratings` → priors must match to 1e-12.
   - Volume and rate inputs → the EB and ridge outputs must match to 1e-10.
3. **GBM-dependent stages** (V(s), Layer-1 g) get **functional parity**:
   - Rust V(s) on a fixed state grid within ±0.05 EP of the oracle (or of a spec-defined deterministic estimator).
   - corr(dV_rust, dV_py) ≥ 0.999.
   - The Tier-0 recovery floors and Layer-A semantic invariants must pass in Rust as-is.

   The recovery floors are implementation-independent and are the real cross-language contract.
4. **Synthetic truth must expand before Layers A–F are ported.** The current generator plants only player
   ability, team strength, one QB trajectory and a feeder league. It has no pass/run split, no targets or
   carries, no availability and no TDs by player, so it cannot be an oracle for Layers A, B, C or F, for
   PB-MAE, or for distributions. The spec's α§12.2 goldens need a planted volume/share/TD/availability world.
5. **The leakage suite ports as-is**, as a spec-level contract: poisoning, tripwire, watermark (fixed at season
   seams) and two-path. It is the best part of the reference.
6. **Determinism contract:**
   - single-thread bit-identical in-process (<1e-9 today)
   - cross-platform tolerance documented per stage
   - Rayon reductions in a fixed order (or Kahan-summed), so parallelism does not change published numbers (α§6.6.5)

---

## 7. Keep, downgrade, defer, drop (engine-only repo)

**Must stay (engine properties):**
- determinism and recorded seeds (α§6.6.5, F§14)
- the version chain Data→Feature→Model→Prediction (F§14)
- the feature schema version (F§10)
- as-of rules, per-source timestamps, the three-season window and leakage tests (α§2.4, §4.5, §12.3)
- idempotent ingest with raw retention, content hashes and run ids (F§8.3, §9.3, α§4.1.1)
- typed failure states and partial-success quarantine (F§9.4-9.5)
- snapshot, rollback, and candidate validation before promotion; never partially promote (F§12.3, §13, §16)
- typed numerical failures (α§6.6.4)
- the stat-vector-first output with distributions, conditional and unconditional (α§5)
- versioned affine scoring (α§5.4)
- the model-spec-before-code rule (α§6.6)
- golden, property, recovery and leakage tests (α§12.1-12.3, F§19.1-19.2)
- immutable lock snapshots (α§7.2, §10.3); without an app these become *engine run records*
- competitor projections never used as features (α§10.3, §14)

**Downgrade:**
- **Durable job queue** (F§15) and crash recovery (F§16) become a run ledger with per-stage status and atomic
  stage commits. Windows-restart and app-crash specifics are dropped.
- **Daily scheduler and catch-up** (F§9.1, §9.6; α§10.2) become CLI commands an external scheduler calls. The
  once-per-day fetch cap stays as a policy in the ingest layer.
- **Commands, queries and events** (α§8.2-8.4) become the CLI and library API plus structured run events (F§17).
- **Performance targets** (α§13, F§20) keep only the pipeline budgets as *provisional* benchmarks:
  - daily incremental under 10 min
  - all-player simulation under 90 s
  - three-season rebuild under 45 min
  - memory under 4 GB normal and 8 GB rebuild

  UI latency targets are dropped. "Reference Windows machine" becomes "declared reference machine" (owner decision D9).
- **SQLite schema groups** (α§8.5) are pared to the engine tables. Bulk analytical data may move to Parquet
  with an SQLite manifest; this is the owner's call because F§8 makes SQLite the source of truth.
- **The identity review queue UI** (α§4.4.3 rule 4, §10.2) becomes an exported review artifact plus a CLI
  approve/reject.

**Defer (keep the interface, implement later):**
- the benchmark provider registry and imports (α§7.6)
- the competitive and claim gates (α§7.8-7.9)
- live shadow weeks (α§10.4)
- K/DST (α§2.1)
- weather and OL adapters (α§4.3)
- GBM continuation/replay modes (F§12.2), until a GBM component earns its place (α§6.2 last line)

**Drop:**
- Flutter and FRB (α§5.5, §12.6, F§4-6, F§19.5)
- installer, signing, MSVC and Windows credential store (F§3, §18 bullets 6; α§14)
- resource modes (F§7)
- application lifecycle (F§22)
- UI deliverables (α§9.2 "Native UI", §10.2 UI additions, §17)
- the α§3.2 product wording

The AI-process apparatus (α§1.3-1.6, §8.7-8.12, App. B-E) is not engine semantics. Keeping, slimming or
dropping it is a product owner decision (D10).

---

## 8. Decisions that need the statistical owner (or the product/architecture owner)

**D1. Team-strength estimand and Layer-3 convention (C1).**
- Adopt net = γ_off + γ_def (market row `[+1,+1]`).
- Fix `synth.py:193` so defenders come from `def_team`, and fix planted strength to off + def.
- Regenerate the goldens.
- Reject the queued `[+1,−1]` change.

**D2. Live-path policy for participation-dependent RAPM (R14).**
- Ratify GRID-Live (participation-free involvement credit + team-level ridge) vs GRID-Offseason RAPM.
- Declare the current H2 +0.848 and weekly Tier-1 numbers **not valid as live evidence** until re-run with
  publication-time gating.
- Alternative: approve approximating on-field lineups from snap counts plus depth charts.

**D3. Where GRID talent enters the stat vector (Layer D).**
- Option (a): role-specific talent as covariates in EB-shrunk per-component rate models (recommended; matches
  roadmap §4.5).
- Option (b): structural per-component Kalman states.
- Option (c): the scalar dV-talent → rates mapping as today.
- Also decide whether the Kalman scalar should split by role (dropback, carry, target).

**D4. Primary metric and gate pre-registration (R47, R51).**
- Adopt PB-MAE with training-period `scale_p`, the union pool with inactive = 0, a week-clustered bootstrap,
  and α§9.4 thresholds fixed in advance.
- Demote H1/H2 to secondary diagnostics.
- Calibrate-then-gate becomes allowed only on a disjoint calibration period.

**D5. Three-season window semantics for state-space models (§5.3).**
- RAPM per-season accumulator blocks are mechanical.
- For the Kalman, choose between (i) discounting carried state as sufficient, or (ii) re-initialising from the
  S−3 prior on replay.

**D6. GBM on GRID's critical path (C4).**
- V(s) estimator: a deterministic in-house model, `xgb` behind the trait, or nflfastR `ep` (scope-leak caveat).
- Layer-1 context model `g`: keep cross-fitted GBM vs ridge/GAM.

**D7. NCAA prior form (R40).**
- Keep GRID's feeder-SV approach, which needs a CFBD *play-by-play* GRID pass beyond α§4.2.3's box-score
  features, or adopt α§4.2.3 box-score features with an affine translation.
- Define q, learn n0, and set the influence cap.
- The reference's age and draft step functions (issue #49) need approval or replacement.

**D8. Synthetic world scope before porting Layers A–F (§6.4).**
- Approve building a stat-vector synthetic generator: volume, shares, TDs, availability and game coupling.
- It then becomes the golden source for α§12.2.

**D9. Engine-only verification authority** (product/architecture owner).
- alpha-spec §8.11 makes Windows `verify.ps1` authoritative.
- For an engine library, Linux CI may be authoritative, with Windows as a secondary job.

**D10. Fate of the α§1.3-1.6 and §8.7-8.12 AI-governance apparatus** in an engine-only repo
(product/architecture owner).

**D11. The α§9.4 Phase-1 model gate stays verbatim**, even though the reference's best real result is a tie
with last-season. The recommendation is to keep it, since lowering a gate after seeing results is exactly what
α§12.7 prohibits. The owner should acknowledge that GRID is currently below it.

Items with an obvious default that need no owner decision:
- C2 sign fix
- C3 Kalman observation
- R20 causal init
- R33 typed failure
- R09 exception swallowing
- R02 labels from nflverse `player_stats`
- R46 inactive = 0
- R49 clustered bootstrap
- the season-keyed accumulator watermark (R08)

---

## 9. Suggested order for implementation agents (spec-first view)

1. **Write the model specs** in `docs/05-model-specs/` from `docs/99-templates/template-model-spec.md`
   (α§6.6) **before** any Rust. In order, with their sources:
   1. V(s) EP model — `grid/value.py`
   2. GRID attribution: team-level ridge, offseason RAPM and market-at-lock — `grid/layers.py`, with the C1/C2 corrections
   3. Layer-1 involvement credit — `layers.layer1_all_players`, re-scoped participation-free
   4. Generic state-space — `grid/statespace.py`, with the C3 and R20 fixes and fixed-lag added
   5. Low-evidence prior — `grid/priors.py` plus α§6.3
   6. Volume/share allocator — `projection/volume.py` plus the α§6.1 C simplex
   7. Efficiency rates — `projection/model.py` plus TD EB
   8. Scoring affine — `scoring/*`
   9. Evaluation protocol — `validation/*` plus α§7

   Each spec carries the reference file and function, the reference tests that become goldens, and the
   corrections ledger entries.
2. **Correct the Python oracle** (§6.1) and regenerate the goldens. Export the stage-wise golden fixtures (§6.2).
3. **Port the primitives**, which can run in parallel:
   - ridge with diagnostics, sparse CG RAPM, affine
   - Kalman, RTS and fixed-lag
   - EB
   - the leakage harness (`AsOf`, poisoning, tripwire)
   - the metrics, with PB-MAE and the clustered bootstrap added
4. **Port GRID**: V(s), attribution and state-space, gated by stage-wise parity plus the Rust synthetic Tier-0
   recovery floors.
5. **Build the new spec layers** (A, B, C-allocator, F), the ensemble, governance and versioning. These have
   no reference, so they get spec-only goldens from the expanded synthetic world (D8).
