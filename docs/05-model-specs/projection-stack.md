---
model-spec-id: MS-PROJECTION-STACK
status: Draft            # Draft | Approved | Superseded
statistical-owner: statistical owner (role defined in engine-spec §1); approval pending
version: 0.1.0 (2026-10-07, written under consolidation WP P0-01)
supersedes: none. First engine-only spec for this component. Replaces the cautious-nevermore
  roadmap §4.5 prose now archived (non-authoritative) under docs/07-archive/cautious-nevermore/.
---

# Model spec: projection stack (volume, stat-line rates, talent features, SV→points map, preseason assembly, scoring)

This is a contract written before implementation. Under the authority order in engine-spec §1.5
(DR-A2, adopted subject to owner ratification) it is an authority-level-3 document. The statistical
owner approves the equations. The implementing agent may not redefine them (alpha-spec §1.3,
superseded; carried into engine-spec §1).

**How to read this spec**

- **Normative words.** MUST, SHOULD and MAY are requirements on the Rust engine. Everything
  described as "the oracle" is the behaviour of the Python reference at `reference/python/`
  (ADR-012; engine-spec §1.7). Every `reference/python/...:LINE` citation uses cautious-nevermore @
  `59bce1d` line numbers, which are byte-identical in the import for every file cited here.
- **Proposed decisions.** Statements tagged "(proposed — DR-xx)" are defaults recorded in
  `docs/00-meta/decision-register.md`. The owner has not ratified them, so they are not settled.
  `DR-NEW:<slug>` marks a decision raised after the critic's consolidation; it has no register
  entry yet unless one is cited.
- **Scope.** `reference/python/backend/projection/*` in full, and the scoring engine
  `reference/python/backend/scoring/{engine,formats,columns}.py`. `scoring/vor.py` and
  `scoring/format_registry.py` are oracle-only: they are imported because the oracle's closure needs
  them (critic X-5, X-20) and are not ported, except VOR inside the lineup simulation
  (`evaluation-and-leakage.md` §4.10; proposed — DR-C11).
- **Synthetic generators.** *Legacy synth* is the oracle's generator as imported. Its defenders are
  drawn from the offense's own roster (KI-NEW-Y0; critic G-1). The projection modules do not read
  the synth's defence directly, but every number below that passes through RAPM ratings or the
  verdict's walk-forward was measured on the legacy synth and is labelled so.

---

## 1. Purpose and statistical intent (template: Target)

### 1.1 What the stack estimates

| Estimand | Definition | Units / support | Status |
|---|---|---|---|
| Per-game usage `v̂_{i,c}` | Projected per-game count of usage driver `c ∈ {pass_attempts, rush_attempts, targets, receptions}` for player `i`, shrunk toward the position mean | count per game, ≥ 0 | Implemented (oracle), `volume.py`. Partial Layer C seed |
| Per-unit rate `r̂_{i,(c,s)}` | Projected stat `s` per unit of driver `c` (e.g. passing yards per attempt), as a function of GRID talent features | stat per unit of `c`, real (unconstrained in the oracle) | Implemented (oracle), `model.py`. Partial Layer D seed |
| Per-game stat line | The 14 `SCORING_STAT_COLS` per game: volume copied through, the rest `v̂ · r̂` | box-score units per game | Implemented (oracle) |
| Season stat line | Per-game line × an expected games count (17) | box-score units per season | Implemented (oracle), `preseason.py`. A derived product (proposed — DR-C4) |
| Fantasy points | Scoring-profile transform of a stat line | points | Implemented (oracle) as a linear map with no offset and no version |
| Weekly points from credit | Per-position affine map `a_pos + b_pos · credit` from GRID currency to weekly fantasy points | points per game | Implemented (oracle), `sv_to_points.py`. **Diagnostic only** in the engine (proposed — DR-C2) |

The stack's outputs are **fantasy-relevant projections**. GRID's signals (RAPM rating, state-space
talent, the cross-league prior) enter only as **covariates** (proposed — DR-C2, DR-C3). They are
never re-labelled as fantasy points.

### 1.2 Statistical intent (load-bearing comments, preserved)

The oracle's comments state the intent. The Rust port MUST keep it.

- **Volume dominates, and GRID measures efficiency only.** `reference/python/backend/projection/volume.py:3-7`:

  > "Fantasy points = volume x efficiency. Volume (attempts / targets / carries / receptions) is
  > what most of the points ride on and it is far more predictable than week-to-week efficiency,
  > yet GRID's RAPM/Kalman measure efficiency only. So volume gets its own projection here; the
  > GRID talent features (a later module) supply the efficiency side, and the fitted stat-line
  > model combines them."

- **GRID is a feature, not the projection.** `model.py:15-20`: each downstream stat is
  "``volume_driver * fitted_rate(talent_features)``" and "This is exactly §4.5's "GRID enters as a
  *feature*, not as the projection itself" — efficiency, not volume." The CN roadmap states the same
  principle (cn-docs §2.5): "GRID cannot *be* the projection — it is the most valuable *feature* in
  one." This is the intent DR-C2 and DR-C3 carry into Layers C and D.
- **Project a stat line, then score it.** `model.py:6-9`: "project a stat line, then score it
  through any format (one projection, many scoring rule sets)." The engine keeps this ordering: the
  stat vector is the contract, and scoring is a downstream affine transform (engine-spec §5.4).
- **Where GRID can win.** CN roadmap §4.5 (cn-docs §2.5, "H1 implication"): last-season actuals
  already encode volume × efficiency, so GRID's advantage must come from regressing unsustainable
  efficiency and touchdown rates toward talent, and from trajectory, **not** from volume or role
  change, which GRID does not model. The engine's evaluation of GRID members (engine-spec §6.6
  requirement 5) tests exactly this.
- **Filtered and smoothed talent are different quantities.** `features.py:9-17`: the smoothed
  talent "is **not** the same number as the DB's `kalman_trajectory.talent` column, which is the
  *filtered* (online, causal) state ... conflating filtered and smoothed state is exactly the kind
  of correct-looking-wrong-semantic bug the engine's golden master exists to catch". Lessons-learned
  LL-12. §1.3 item 2 refines what this means at the end of a window.
- **Missing is not zero.** `features.py:53-57`: "Missing ``prior_mean``/``prior_var`` are ``NaN``
  ... rather than 0.0, so a downstream model can distinguish "no prior" from "prior of zero"." The
  oracle then erases that distinction in `model.py:82` and `:138` (KI-P1). The engine keeps the
  distinction end to end (LL-11).
- **No calibrated conversion means no projection.** `sv_to_points.py:45-46`: positions without a
  fitted map "fall back to 0.0 — "no calibrated conversion" is surfaced as no projection, not a
  guess." The oracle returns a number (0.0); the engine returns an explicit absence (§5.4).
- **Overrides are the role-change mitigation.** `volume.py:16-18`: "A manual **override hook** lets
  ADP / depth-chart knowledge replace a player's projected usage — the documented mitigation for
  offseason role changes (§4.5), the one place naive prior-usage is weakest."
- **Leakage-safe by construction.** `volume.py:20-22` and `sv_to_points.py:17-19`: only data from
  before the forecast point is read, through the `AsOf` cutoffs (`evaluation-and-leakage.md` §4.1).
- **Fast talent and slow talent.** `verdict.py:145-148`: "``rapm_rating`` is the fast,
  season-to-date talent that updates per origin, ``smoothed_talent`` the slow retrospective
  prior-period talent, so freezing the latter is the design, not a shortcut."

### 1.3 Facts this spec relies on

These were derived from the code and verified by measurement on 2026-10-07 (§7.6). They are binding
for the port.

1. **The oracle's volume weight is an empirical-Bayes posterior-mean weight.**
   - With per-game counts `y_{i,g} | λ_i ~ Poisson(λ_i)` and a position prior
     `λ_i ~ Gamma(α, β)`, the posterior mean is `(α + Σ_g y_{i,g}) / (β + g_i)`
     `= (g_i/(g_i+β)) · ȳ_i + (β/(g_i+β)) · (α/β)`.
   - That is the oracle's `w = g/(g + k_shrink)` (`volume.py:105`) with `k = β` and target `α/β`.
   - Under a normal–normal model the same form holds with `k = σ²_within / τ²_between`.
   - So "learning the shrinkage" (KI-NEW-R2) means estimating **one** parameter per position and
     driver. The functional form can stay, and parity at a fixed `k` stays exact (§10.3 P-2).
2. **The verdict's `smoothed_talent` is the filtered end-of-window talent.**
   - `features.assemble_smoothed_talent` returns `tau_smooth[-1]` (`features.py:111`).
   - The RTS pass starts from the last filtered state: `xs[-1] = xf_s[-1]; Ps[-1] = Pf_s[-1]`
     (`reference/python/backend/grid/statespace.py:239`). So `tau_smooth[-1] == tau_filt[-1]`
     and `var_tau_smooth[-1]` is the filtered talent variance at the last week.
   - Measured: the absolute difference is **exactly 0** over 200 random series with gaps and
     interventions (§7.6).
   - Consequence for the engine. The ROS feature is a **filtered** quantity, so it is admissible in
     live records under the "filtered or predictive only" rule of
     `docs/03-contracts/engine-output-contract.md` §4.2. It is causal only if the filter's
     initialisation is causal. The oracle initialises from `nanmean(y[:3])` inside the window
     (`statespace.py:186`; KI-#15). The engine computes it with the forward filter alone, from a
     prior-based `x0`/`P0` (proposed — DR-C10), and names it `end_of_window_filtered_talent`.
3. **Receptions are projected as volume, and catch rate is not modelled.**
   - `receptions` is a `VOLUME_COLS` entry (`volume.py:34`) and the driver of receiving yards and
     touchdowns (`model.py:57`).
   - `targets` is projected (`volume.py:34`) but drives nothing: it is not a `RATE_STATS` key
     (`model.py:54-58`) and it is not scored (`formats.py:3-17`).
   - So the oracle's receiving production has no talent effect on catches. Alpha-spec §6.1 puts
     catch probability in Layer D. The engine follows alpha: receptions are
     `targets × catch_probability` (§4.8).
4. **Rates are per player-season ratio estimates, fitted unweighted.**
   - `y = sub[stat] / sub[vol_col]` on season totals (`model.py:142`), so each row is
     `Σ stat / Σ volume` for one player-season.
   - Every row has weight 1, whatever its volume (KI-NEW-R1).

---

## 2. Position in the engine

### 2.1 Layers A–F (engine-spec §6.1; proposed — DR-C2)

| Layer (engine-spec §6.1) | What the oracle has | Status | Engine requirement |
|---|---|---|---|
| A. Availability and role eligibility | Nothing. Every projected player gets the full per-game line, and preseason multiplies by 17 games (`preseason.py:30`) | **Absent** | New work, no parity target. `p_active`, `p_start`, snap multiplier, `p_limited_role` (engine output contract §3.3) |
| B. Team game environment | Nothing in `projection/`. Team intercepts exist only in RAPM (`rapm-attribution.md` §4.5) | **Absent** | New work. Team net strength anchored to the market line at lock is a GRID input (proposed — DR-B5) |
| C. Opportunity allocation | `volume.project_volume`: EB shrinkage of prior-season per-game usage toward the position mean, plus an override hook | **Partial** | §4.8.1. No team totals, no simplex allocator, no snap share, no in-season data, games miscounted (KI-NEW-R2) |
| D. Efficiency | `model.fit_stat_line_model`: per-position, per-(driver, stat) standardized ridge of the per-unit rate on `[rapm_rating, smoothed_talent, prior_mean]` | **Partial** | §4.8.2. Unweighted (KI-NEW-R1), no TD shrinkage rule, no catch rate, fumbles and two-point conversions fixed at 0 |
| E. Matchup and context | Nothing in `projection/`. The matchup grade lives in RAPM, with an inverted sign (KI-NEW-A2) | **Absent** | `rapm-attribution.md` §4.5 grade `G = +E_def` (proposed — DR-B5), plus venue, rest and weather, as new work |
| F. Correlated simulation | Nothing. The only uncertainty is an evaluation-side Gaussian spread (`evaluation-and-leakage.md` §4.8) | **Absent** | New work (P1-08). Every distribution, quantile and P(zero) comes from Layer-F draws (engine-spec §5.3) |
| Scoring (engine-spec §5.4) | `scoring/engine.calculate_points`: `Σ rules[k] · stats[k]` | Implemented, linear, unversioned, no offset | §4.7. Affine profile with offset and version; Half-PPR default |

### 2.2 Data flow (oracle)

```text
weekly player_stats (box-score rows) ──► volume.project_volume ─────────────► per-game usage (4 drivers)
RAPM ratings_df (layers.run_rapm) ─┐                                             │
weekly credit series (Layer 1) ────┼─► features.assemble_talent_features ──┐     │
priors.build_priors (synth only) ──┘    (rapm_rating, smoothed_talent,     │     │
                                         prior_mean, …)                    ▼     ▼
player-season totals + features ──► model.fit_stat_line_model ──► StatLineModel.project_stat_line
                                                                       │ per-game 14-column line
                                                       preseason ×17 ◄─┤
                                                                       ▼
                                                     scoring.calculate_points(line, config) ──► points
(credit, weekly points) pairs ──► sv_to_points.fit_sv_to_points ──► weekly points (diagnostic)
```

### 2.3 Horizons (engine-spec §2.2; proposed — DR-C4)

- **Oracle.** Three horizons with three different producers:
  - weekly: the SV→points affine map on the walk-forward RAPM rating (`tier1.forecast_weekly_points`,
    `verdict.py:403-404`);
  - rest of season (ROS): the stat-line model, per game (`games = 1`, `tier1.py:106`, `:168`);
  - preseason: per-game line × 17 (`preseason.py:59`).
- **Engine (proposed — DR-C4).**
  - The weekly stat vector is the primary contract (`docs/03-contracts/engine-output-contract.md`
    §3.1).
  - ROS and preseason quantities are **derived sums of weekly Layer-F draws**, labelled with a typed
    `horizon` field. They are never produced by a separate model, and never by a `week = 0`
    sentinel (KI-A4).
  - Mis-wiring the horizon to the wrong producer produced CN's −0.864 artefact (LL-12;
    `docs/07-archive/cautious-nevermore/real-data-results.md` §3.2). Each output horizon therefore
    names the model that produced it.

### 2.4 Contracts and neighbouring specs

- Inputs: official weekly player statistics as labels (engine-spec §4.7; proposed — DR-C12); the
  GRID signals of `rapm-attribution.md`, `layer1-credit.md`, `state-space-kalman.md` and
  `cross-league-priors.md`, materialized as GRID-derived features (engine-spec §11.6).
- Outputs: `docs/03-contracts/engine-output-contract.md` §3 (stat vectors, conditional and
  unconditional, distributions, scoring) and §4.2 (GRID signal block).
- Parity: `docs/03-contracts/parity-fixture-contract.md` (component `projection-stack` and
  `scoring`); `reference/python/PARITY.md`.
- Evaluation: `evaluation-and-leakage.md` (as-of slicing, baselines, metrics, the H1/H2 diagnostics
  that consume this stack).

---

## 3. Inputs and outputs (template: Inputs)

### 3.1 Inputs

| Input | Oracle form | Units / type | Null and failure semantics (oracle → required) | As-of rule |
|---|---|---|---|---|
| Weekly usage history | `player_stats`-shaped frame, one row per player-week, with `VOLUME_COLS`, `position`, `season`, `week` (`volume.py:63-68`) | counts | A missing `VOLUME_COLS` column crashes `groupby.agg` (KI-P3) → typed `ContractError`. Missing weeks are absent rows, so "games" = weeks with a row (KI-NEW-R2) → games from snaps or participation | Rows with `(season, week) ≤ (S, W−1)` (`volume.py:70-72`). Engine: also published before the lock (engine-spec §4.5) |
| Player-season training frame | one row per (player, season): drivers, downstream stats, `TALENT_FEATURE_COLS` (`model.py:125-131`) | season totals; features in GRID currency | A missing stat or feature column raises `KeyError` (KI-P4) → typed error. NaN features become 0.0 (`model.py:138`; KI-P1) → explicit missing indicator | Labels and features from the training window only (engine-spec §2.4) |
| `ratings_df` | `layers.run_rapm` output, `[player_id, rating, …]` (`features.py:72-76`) | EP per play | Players absent → 0.0 in `to_frame` (`features.py:64`) → `no_data` | Offseason RAPM only, from seasons whose participation is published (proposed — DR-C1) |
| Weekly credit series | `{pid: {y, snaps, played, interventions?}}` (`features.py:82-86`) | EP per snap; snaps | Absent week MUST be `played = False` **and** `y = NaN` (`features.py:88-93`) | Strictly before the origin (`verdict.py:150-153`) |
| `priors_df` | `priors.build_priors` output, `[player_id, prior_mean, prior_var]` (`features.py:116-126`) | rating scale | `None` or empty → no priors. Wired to no real data (KI-NEW-P2) | Prior seasons only (`cross-league-priors.md`) |
| `(credit, points)` pairs | `[season, week, position, credit, fantasy_points]` (`sv_to_points.py:66-69`) | EP per snap; points | NaN passes the spread guard and reaches `polyfit` (KI-P5) → typed error | `AsOf.slice_plays` (`sv_to_points.py:74`) |
| Overrides | `{pid: {col: value}}` (`volume.py:67-68`) | per-game counts | Unknown column → `ValueError` (`volume.py:110-115`). Values are not validated → typed check for finite, ≥ 0 | Timestamped operator input, part of the prediction snapshot (engine output contract §3.3) |
| Scoring profile | `ScoringConfig{name, rules}` (`engine.py:6-17`) | points per unit | Unknown stat keys are silently ignored; missing stats count 0 (`engine.py:22-23`) → typed validation (engine-spec §5.4) | Versioned, immutable profile |

### 3.2 Outputs

**Oracle outputs.**

- `VolumeProjection{per_game: {pid: {col: value}}, games_prior: {pid: int}}` (`volume.py:37-47`).
- `StatLineModel{rates: {(driver, stat): RateModel}}` (`model.py:86-90`); `project_stat_line` →
  `{col: value}` over the 14 columns (`model.py:92-108`).
- `TalentFeatures` and its frame `[player_id, rapm_rating, smoothed_talent, smoothed_var,
  prior_mean, prior_var]` (`features.py:35-69`).
- `SVToPointsMap{coeffs: {position: (intercept, slope)}}` (`sv_to_points.py:36-51`).
- `project_preseason` → `{pid: {col: season total}}` (`preseason.py:33-60`).
- `calculate_points` → `float` (`engine.py:20-24`).

**Required engine outputs** (shape owned by `docs/03-contracts/engine-output-contract.md`,
engine-spec §5.5): the per-position stat vector of engine-spec §5.1, in conditional and
unconditional form, with Layer-F distribution summaries. The table maps each alpha-spec §5.1
component onto the oracle.

| Stat-vector component (engine output contract §3.2) | Oracle column | Oracle status | Required |
|---|---|---|---|
| `active_probability`, `start_probability` (QB), `offensive_snap_share` (RB/WR/TE) | — | absent | Layer A / Layer C (new) |
| `pass_attempts` | `pass_attempts` (volume) | EB-shrunk prior-season per-game count; label counts sacks (KI-NEW-I1) | Layer B pass attempts × Layer C dropback share; official definition, sacks excluded |
| `completions` | `completions` = attempts × rate | linear ridge rate, unbounded, so `completions > pass_attempts` is possible | Layer D completion probability on a probability scale; ≤ attempts per draw |
| `passing_yards`, `passing_tds`, `interceptions` | rate stats on `pass_attempts` | linear ridge rates, clamped at 0 | Layer D yards-per-attempt; strongly shrunk TD and INT rates (alpha-spec §6.1 Layer D) |
| `sacks_taken` | — | absent | Layer B sacks × allocation (new) |
| `rushing_attempts` / carries | `rush_attempts` (volume) | label excludes kneels (KI-NEW-I5) | Layer C carry share × Layer B rush attempts; official definition |
| `rushing_yards`, `rushing_tds` | rate stats on `rush_attempts` | as above | Layer D |
| `targets` | `targets` (volume) | projected but drives nothing (§1.3 item 3) | Layer C target share × Layer B pass attempts |
| `receptions` | `receptions` (volume) | EB-shrunk count, no talent effect | `targets × catch_probability` (Layer D); ≤ targets per draw |
| `receiving_yards`, `receiving_tds` | rate stats on `receptions` | as above | Layer D yards per target or per reception (rate-base choice under DR-D21) |
| `fumbles_lost` | `fumbles_lost` | **fixed at 0.0** (`model.py:22-24`, `:99`); label misattributed (KI-NEW-I3) | **Modelled**, strongly shrunk per-touch rate (engine-spec §6.4.7) |
| `two_point_conversions` | `two_point_conversions` | **fixed at 0.0**; label never populated (KI-NEW-I5) | **Modelled**, strongly shrunk, tied to team scoring draws (proposed — DR-C12) |

The stat-vector asymmetries (QB has no snap share; RB/WR/TE have no start probability; QB has no
receiving components) are carried from alpha-spec §5.1 and are open
(DR-D8; DR-D7).

### 3.3 Explanation fields (template: Explanation fields; engine-spec §6.7)

- **Role and opportunity:** projected share means and the EB weight on own history (`g/(g+k)` in
  the oracle's form), so a reader sees how much a small sample was pulled toward the position.
- **Efficiency drivers:** per-component contribution of each GRID covariate, in the component's own
  units (for example yards per attempt per unit of dropback talent). Signs follow the covariate.
  Contributions are predictive associations, never causes (engine output contract §4.1).
- **NCAA prior contribution:** the prior covariate's contribution, or null when the player has no
  prior. "No prior" and "a prior of 0" are reported differently (§1.2).
- **Override:** whether an operator override replaced a usage projection, with its timestamp.
- **GRID signal block** (engine output contract §4.2): filtered or predictive values only; the
  end-of-window talent of §1.3 item 2 qualifies because it is a filtered value.

---

## 4. Model and equations (template: Equations, Priors, Constraints)

### 4.1 Volume model (`volume.project_volume`, `volume.py:50-119`)

Let `S` be the forecast season and `W` the forecast week. For player `i` and driver
`c ∈ VOLUME_COLS = [pass_attempts, rush_attempts, targets, receptions]` (`volume.py:34`):

```text
H        = { rows : season < S  or  (season = S and week ≤ W − 1) }              (volume.py:70-72)
for each (i, s) in H:  g_{i,s} = |{distinct week}|,  V_{i,s,c} = Σ_weeks c        (volume.py:78-82)
                       x_{i,s,c} = V_{i,s,c} / g_{i,s}                            (volume.py:83-84)
s*(i)    = max{ s < S : (i, s) ∈ H }          most recent PRIOR season            (volume.py:91-94)
pos(i)   = first position of i in season s*(i)                                    (volume.py:81)
m_{p,c}  = mean_{j : pos(j)=p} x_{j,s*(j),c}  over every player's own s*(j)        (volume.py:97)
w_i      = g_i / (g_i + k),   g_i = g_{i,s*(i)},   k = k_shrink = 8.0             (volume.py:55,103,105)
v̂_{i,c}  = w_i · x_{i,s*(i),c} + (1 − w_i) · m_{pos(i),c}                          (volume.py:108-109)
v̂_{i,·}  ← override_i  (key by key, after validation of the keys only)            (volume.py:110-116)
```

Facts the port MUST reproduce in oracle-compatible mode, or replace as stated:

- **Current-season rows are read but not used.** They stay in `H` but the baseline is the prior
  season only (`volume.py:86-90`): "Mid-season blending of current-to-date usage is a deferred
  extension." At `(S, W > 1)` the projection is therefore the same as at `(S, 1)`
  (`tests/projection/test_volume.py:64-70`). Required: §4.8.1.
- **Empty cases.** No rows, or no prior season, give an empty projection, not an error
  (`volume.py:74-75`, `:92-93`). Required: an explicit `no_history` status per player.
- **The shrinkage target mixes seasons.** `m_{p,c}` averages each player's *own* most recent prior
  season, so a player who last played years earlier contributes that stale season to the current
  target. The mean is also unweighted by games: a one-game player counts as much as a 17-game
  player (`volume.py:97`). Required: the target is estimated on the §2.4 window, exposure-weighted
  (§4.8.1).
- **Position fallback.** A player whose position is missing from `m` uses his own rate as target
  (`volume.py:108`), which silently sets `w = 1`. Required: typed error for an unknown position.
- **Games.** `g = nunique(week)` over stat rows (`volume.py:79`) drops active weeks with no stat
  row, which inflates backups' per-game rates (KI-NEW-R2). Postseason rows are included when the
  history has them (KI-NEW-I4).

### 4.2 Rate model (`model.fit_stat_line_model`, `model.py:117-146`)

Fit per position (callers fit one `StatLineModel` per position, `model.py:26-28`). For each driver
`c` in `RATE_STATS` (`model.py:54-58`):

```text
RATE_STATS = { pass_attempts: [completions, passing_yards, passing_tds, interceptions],
               rush_attempts: [rushing_yards, rushing_tds],
               receptions:    [receiving_yards, receiving_tds] }

D_c  = { player-season rows with c > 0 }                                         (model.py:135)
X    = D_c[TALENT_FEATURE_COLS].fillna(0.0)     TALENT_FEATURE_COLS =
       [rapm_rating, smoothed_talent, prior_mean]                                (model.py:48,138)
μ_j  = mean_n X_{nj};   σ_j = sqrt(mean_n (X_{nj} − μ_j)²)   (population sd, ddof 0)
σ_j  ← 1   if σ_j = 0  (scikit-learn StandardScaler zero-variance rule)          (model.py:139)
Z    = (X − μ) / σ        one scaler per driver, shared by that driver's stats     (model.py:139-140)
for each stat s of c:
  y_n = D_c[s]_n / D_c[c]_n                     per player-season ratio           (model.py:142)
  (β̂, b̂) = argmin_{β,b} ‖y − b·1 − Zβ‖² + α‖β‖²,   α = 1.0, intercept unpenalized (model.py:121,143-144)
         ⇔ β̂ = (Z_cᵀ Z_c + α I)⁻¹ Z_cᵀ (y − ȳ),   b̂ = ȳ − z̄ᵀβ̂,   Z_c = Z − 1 z̄ᵀ
predict(t) = b̂ + β̂ᵀ ((x(t) − μ) / σ),
  x(t)_j = t.get(feature_j, 0.0), then NaN → 0.0, +inf → 0.0, −inf → 0.0           (model.py:81-83)
```

- `z̄` is zero up to rounding, because `Z` is standardized on the same rows. The closed form above
  matches scikit-learn's `Ridge(alpha, fit_intercept=True)` on predictions to 2.1e-16 relative
  (§5.1).
- **A constant feature contributes nothing.** In the verdict path `prior_mean` is the constant 0.0
  (`verdict.py:280`). Its scale becomes 1, its standardized column is all zeros, and its
  coefficient is exactly 0 (measured, §7.6).
- **Present-but-NaN guard (CN PR #94, audit finding P1).** `dict.get(c, 0.0)` returns the default
  only for absent keys. A non-rookie's `prior_mean` is a present NaN (`features.py:67`), which used to
  crash `Ridge.predict` through `StandardScaler`. The fix maps NaN and ±inf to 0.0 at predict time to
  match the fit side's `fillna(0.0)` (`model.py:75-82`; regression tests
  `tests/projection/test_model.py:139-170`). It is train-consistent, but 0.0 still conflates "no
  prior" with "prior = 0" (KI-P1). Required: §4.8.2.
- **Zero-volume rows.** Excluded from that driver's fits only, because "a zero-volume row has an
  undefined per-unit rate, not a zero rate" (`model.py:129-131`). If no row has volume for a
  driver, that driver gets no rate models (`model.py:136-137`).

### 4.3 Stat-line assembly and scoring (`model.py:92-114`)

```text
line[col]    = 0.0                                for every col in SCORING_STAT_COLS (14)  (model.py:99)
line[c]      = max(0, v̂_c)                         for c in VOLUME_COLS                    (model.py:100-101)
line[s]      = max(0, line[c] · predict_{c,s}(t))  for (c, s) in RATE_STATS; 0 if no model (model.py:102-107)
points(line) = Σ_{k ∈ rules} rules[k] · line.get(k, 0)                                     (engine.py:20-24)
```

- `fumbles_lost` and `two_point_conversions` stay 0.0: "low-magnitude, rarely-driven stats left at
  0.0 (unmodeled)" (`model.py:22-24`).
- **Silent clamping.** A negative predicted rate is clamped to 0 (`model.py:107`). Alpha-spec §6.6
  rule 4 makes an impossible stat a typed failure. Required: rates are produced on their support by
  construction (§4.8.2), and any out-of-support value is a typed error.

### 4.4 Talent-feature accessor (`features.py`) and its verdict wiring

`assemble_talent_features(ratings_df, weekly_obs, positions, priors_df)` is pure assembly
(`features.py:129-145`):

```text
rapm_rating[pid]                   = ratings_df.rating                              (features.py:72-76)
(smoothed_talent, smoothed_var)[pid] = (tau_smooth[T−1], var_tau_smooth[T−1]) of
      kalman_two_component(y, snaps, played, interventions, SSParams.from_position(pos))
      with default SSParams when pid has no position                               (features.py:99-113)
(prior_mean, prior_var)[pid]       = priors_df rows; {} when None or empty          (features.py:116-126)
to_frame: ids = sorted(union of rapm, smoothed, prior_mean keys, key=str)          (features.py:59-61)
          rapm_rating, smoothed_talent default 0.0;
          smoothed_var, prior_mean, prior_var default NaN                           (features.py:62-69)
```

- By §1.3 item 2, `smoothed_talent` equals the filtered talent at the last week of the series, and
  `smoothed_var` the filtered talent variance there.
- `smoothed_var` and `prior_var` are carried in the frame but are **not** in
  `TALENT_FEATURE_COLS`, so no rate model uses the uncertainty of any feature.

**Verdict wiring (CN PR #91).** `verdict._smoothed_talent_pre_first_origin`
(`reference/python/backend/validation/verdict.py:127-191`) turned `smoothed_talent` on for the ROS
model. Before it, "only ``rapm_rating`` was ever supplied — the other two entered as the constant
0.0 ... so two thirds of GRID's efficiency signal was dead weight" (`verdict.py:133-138`).

1. `pre = AsOf(first origin).slice_plays(plays)` (`verdict.py:169`).
2. V(s) is **refit on `pre`** (`verdict.py:172`). This is a different V(s) from the walk-forward's,
   which is fit on the warm-up slots only (`backtest.py:166-168`). One verdict therefore uses two
   V(s) models.
3. Weekly Layer-1 credit per season through `layer1_all_players(grp, ratings_lookup, player_ids,
   n_splits=5, seed, min_plays=20)`, with the first origin's RAPM ratings as the opponent lookup
   (`verdict.py:173-187`). Per-season grouping prevents week-number collisions across seasons
   (`verdict.py:153-158`).
4. Each player's `(season, week) → (credit, snaps)` is aligned onto the window's full chronological
   timeline. An absent week is `played = False`, `y = NaN`, `snaps = 0` (`verdict.py:206-234`).
5. `assemble_smoothed_talent` gives the per-player value (`verdict.py:189-190`). Players absent
   from it get 0.0 in both fit and forecast (`verdict.py:278-279`; `tier1.py:161-162`).

Fit and forecast share one feature space:

- `rapm_rating` in the **fit** is the first origin's rating (`verdict.py:276-277`). In the
  **forecast** it is the as-of rating of each origin (`tier1.py:161`). The fit therefore pairs
  pre-first-origin player-season labels (prior seasons plus warm-up) with a rating estimated from
  the warm-up weeks only. Train and serve use the same column, but not the same quantity (§8 row 9).
- `prior_mean` is the constant 0.0 in both (`verdict.py:280`; absent from the forecast row,
  `tier1.py:161-162`, so `predict` defaults it to 0.0). Priors are wired to no real data
  (KI-NEW-P2).
- The Layer-1 credit that feeds `smoothed_talent` is an on-field-unit plus-minus. For RB/WR/TE it
  mostly measures the team offence (`layer1-credit.md` §8 row 10; DR-D13).

### 4.5 SV→points affine map (`sv_to_points.py`)

```text
hist = AsOf(S, W).slice_plays(history)           (season, week) ≤ (S, W−1)         (sv_to_points.py:74)
for each position p with n_p ≥ min_rows (10) and ptp(credit_p) ≠ 0:                (sv_to_points.py:77-83)
   (b_p, a_p) = polyfit(credit_p, points_p, 1)   ordinary least squares            (sv_to_points.py:84)
predict(p, x) = a_p + b_p · x   if p fitted, else 0.0                              (sv_to_points.py:42-51)
```

- **Why affine.** "a position's baseline scoring floor (``a_pos``) and its points-per-unit-credit
  slope (``b_pos``) both differ" (`sv_to_points.py:10-15`).
- **What the verdict feeds it.** The column named `credit` holds the **RAPM rating** of the first
  origin, not Layer-1 credit (`verdict.py:89-95`). The map is fitted on pairs from the same
  warm-up weeks the rating was estimated on, so the fit is in-sample within the pre-period
  (KI-NEW-R3). "Credit" denotes Layer-1/1′ only in the engine (`layer1-credit.md` §8 row 18).
- **Engine role (proposed — DR-C2).** The map outputs fantasy points, so it cannot populate the
  stat vector or its draws (engine-spec §6.6, "Not members"). It is a **diagnostic and an ensemble
  candidate only**. It becomes a member only if this spec is amended to define how it enters the
  stat vector, and it then proves incremental value (engine-spec §6.6 requirement 5).

### 4.6 Preseason assembly (`preseason.project_preseason`, `preseason.py:33-60`)

```text
season_line[i][col] = games · project_stat_line(v̂_i, talent_row_i)[col],   games = DEFAULT_GAMES = 17
```

- A player needs both a projected volume and a model for his position; otherwise he is omitted
  (`preseason.py:54-57`). Rookies without usage history are absent unless an override supplies
  volume (`preseason.py:14-17`).
- A missing talent row defaults to `{}`, so every feature predicts as 0.0 (`preseason.py:58`).
- The constant 17 is a full modern regular season; it ignores byes, availability and week 18.
- **Engine (proposed — DR-C4).** Preseason is the sum over weeks 1–17 of weekly Layer-F draws,
  including Layer A availability, reported as a derived product. `DEFAULT_GAMES` is a parity
  constant only (§10.3 P-7).

### 4.7 Scoring engine (`scoring/engine.py`, `formats.py`, `columns.py`)

**Oracle.**

- `ScoringConfig{name, rules: {stat: multiplier}}`; `to_json` = `json.dumps({"name", "rules"})` in
  insertion order (`engine.py:6-17`). Identity by that text is key-order-sensitive (KI-NEW-C1).
- `calculate_points(stats, config) = Σ_{k ∈ rules} stats.get(k, 0) · rules[k]` (`engine.py:20-24`).
  Linear, no offset, no version, no validation of keys.
- `SCORING_STAT_COLS` is the 14-column list of `columns.py:3-18`.
- Presets (`formats.py:3-17`):

  | Rule | Standard | Half PPR | Full PPR |
  |---|---|---|---|
  | `passing_yards` | 0.04 | 0.04 | 0.04 |
  | `passing_tds` | 4 | 4 | 4 |
  | `interceptions` | −2 | −2 | −2 |
  | `rushing_yards`, `receiving_yards` | 0.1 | 0.1 | 0.1 |
  | `rushing_tds`, `receiving_tds` | 6 | 6 | 6 |
  | `fumbles_lost` | −2 | −2 | −2 |
  | `two_point_conversions` | 2 | 2 | 2 |
  | `receptions` | 0.0 | 0.5 | 1.0 |

  These equal the alpha-spec §2.3 weights exactly (cn-docs §2.6; reconcile-spec-first R24).
- **The oracle's evaluations score with STANDARD**, not the alpha-spec Half-PPR default
  (`verdict.py:47`, `:427`, `:563`; reconcile-code-first C5).

**Required (engine-spec §2.3, §5.4).**

```text
fantasy_points = w_profile · stat_vector + b_profile
profile = (weights over the position's stat vector, offset b, profile_id, version, content_hash)
```

- **Default profile: Half-PPR** (alpha-spec §2.3). Standard and PPR are built in. All three have
  offset 0, which reproduces the oracle exactly (§10.3 P-1).
- **Content hash** over a canonical serialization: sorted keys, fixed numeric formatting
  (engine-spec §6.4.8; KI-NEW-C1).
- **Validation.** An unknown stat name, a dimension mismatch or a non-affine rule is a typed error
  (engine-spec §5.4; DR-D6). Missing stats are not silently 0: the profile is
  applied to complete stat-vector draws.
- **Domain.** Profiles apply to stat-vector draws and summaries, never to ratings, credit or `dV`.
- **Re-scoring.** The same Layer-F draws are re-scored for any number of profiles without
  re-running the football model (engine output contract §3.7).
- **Oracle-only modules.** `scoring/vor.py` (value over replacement, tiers) and
  `scoring/format_registry.py` (SQLite format store) are not ported. The engine emits no VOR, tier or
  ranking output. VOR survives only as roster construction inside the lineup simulation
  (`evaluation-and-leakage.md` §4.10; proposed — DR-C11). The format registry's content-dedup
  idea survives as the profile content hash.

### 4.8 Required engine model (normative; proposed — DR-C2, DR-C3)

The oracle seeds Layers C and D. It does not define them. The engine model below is the target for
P1-07. Every equation family here needs statistical-owner approval of its open parameters before
code (engine-spec §6.8; Definition of Ready).

#### 4.8.1 Layer C opportunity

1. **Exposure.** Games are counted from snaps or participation (active weeks), never from weeks with
   a stat row (KI-NEW-R2). A week with zero snaps while active is an observation of zero usage.
2. **Shrinkage (proposed — DR-C5, DR-C3).** Per-game usage and per-snap shares are
   empirical-Bayes posterior means of the §1.3 item 1 form, with the strength estimated, not fixed:
   - **Proposed default estimator: Gamma–Poisson method of moments**, per position, per driver, on
     the training window, exposure-weighted:

     ```text
     m̂   = Σ_i Y_i / Σ_i g_i                                   pooled per-game mean
     s²  = Σ_i g_i (ȳ_i − m̂)² / Σ_i g_i                         exposure-weighted between-player variance
     τ̂²  = s² − m̂ · n / Σ_i g_i                                 minus Poisson sampling variance (n players)
     k̂   = m̂ / τ̂²     if τ̂² > 0                                  prior "pseudo-games" β
     v̂_i = (Y_i + k̂ m̂) / (g_i + k̂)                              = w_i ȳ_i + (1 − w_i) m̂,  w_i = g_i/(g_i + k̂)
     ```

     `τ̂² ≤ 0` (no detectable between-player variation) is a typed `DegeneratePool` error, never an
     implicit `k = ∞`.
   - **Alternative:** normal–normal EB with `k = σ²_within / τ²_between` (engine-spec §6.4.7 names
     both candidates). The choice is recorded in this spec before P1-07 is Ready.
   - The estimated `(m̂, k̂)` per position and driver is a persisted, versioned EB parameter record
     (engine-spec §6.4.7).
3. **In-season data.** The evidence window is engine-spec §2.4: season-to-date `S[1..W−1]` plus
   `S−1, S−2` in-season; `S−1..S−3` for preseason and week 1. Older seasons may be down-weighted by
   a recency discount whose value is set under DR-C5. The oracle's "prior season only" rule is a
   parity mode, not the engine rule.
4. **Allocation.** Shares obey team totals: per team and role, unconstrained logits feed a
   softmax/simplex allocator with roster-aware renormalization when players are inactive (alpha-spec
   §6.1 Layer C). Share overflow is a typed failure (engine output contract §3.9).
5. **Overrides** stay: a timestamped operator override replaces a projected usage or share, is
   validated (finite, in support, known key), is part of the prediction snapshot, and is reported in
   the explanation (§3.3).
6. **Receptions are not volume.** Layer C projects targets. Receptions come from Layer D catch
   probability (§1.3 item 3).

#### 4.8.2 Layer D efficiency

1. **Components** (alpha-spec §6.1 Layer D): completion probability, yards per attempt, catch
   probability, yards per target or reception, yards per carry, touchdown conversion probability
   per opportunity type, interception rate, fumble rate per touch, two-point conversion rate.
2. **Covariates (proposed — DR-C3, option (a)).**
   - role-specific GRID talent (dropback, carry, target): filtered talent and form with their
     predictive variance (`state-space-kalman.md`; `layer1-credit.md` Layer-1′);
   - the prior-season offseason RAPM rating (`rapm-attribution.md` §2.2; proposed — DR-C1);
   - the cross-league prior (`cross-league-priors.md`), with an explicit missing indicator.
   - Every covariate has a missing indicator. A missing covariate is never 0.0 (KI-P1).
3. **Model family (DR-D21).** Each component is an EB-shrunk position rate times a
   covariate effect, fitted with exposure weights:
   - probability components (completion, catch, TD conversion, INT, fumble, two-point):
     binomial GLM with logit link, trials = exposure;
   - yardage components: weighted least squares with weight = exposure, or a Gamma GLM with log link.
   - The oracle's unweighted standardized ridge (`model.py:134-145`) MUST NOT be reproduced in the
     engine path (KI-NEW-R1). It survives as a parity mode only (§10.3 P-3).
4. **Shrinkage.** Touchdown rates are strongly shrunk and "are not allowed to follow short hot
   streaks without opportunity support" (alpha-spec §6.1 Layer D). Fumbles lost and two-point
   conversions are modelled with strongly shrunk rates and are never fixed at 0 (engine-spec §6.4.7).
5. **Support.** Every rate is produced on its support by its link function. No silent clamp.

#### 4.8.3 Ensemble member 1: the transparent recency-weighted baseline (engine-spec §6.6)

engine-spec §6.6 assigns its weighting scheme to this spec. Proposed form
(DR-D25):

```text
for each player i, stat-vector component c, origin (S, W):
   G      = the player's active games in the engine-spec §2.4 window, ordered by recency
   age_g  = number of the team's game weeks between game g and the origin (byes not counted)
   ŷ_{i,c} = Σ_{g∈G} ρ^{age_g} y_{i,c,g} / Σ_{g∈G} ρ^{age_g},        0 < ρ ≤ 1
```

- No fitted GRID input. One `ρ` per position and component class (usage, efficiency).
- `ρ`, the minimum game count before the baseline is emitted, and the season-boundary treatment are
  open; they are set by rolling-origin validation on a calibration period disjoint from evaluation
  (`evaluation-and-leakage.md` §4.11).

#### 4.8.4 Stacking level (DR-D20)

Proposed default for the owner: stack the **Layer C share means and the Layer D rate means**
per position, with ridge-estimated non-negative weights that sum to 1, on out-of-fold
rolling-origin predictions (engine-spec §6.6 requirements 2 and 4). Stacking component means after
multiplication is the alternative. Layer F stays downstream of the stacked quantities.

### 4.9 Constants and hyperparameters

Provenance key: **hand-set (PR #N)** means introduced in that CN PR with no calibration record;
PR numbers are from cn-docs §2.5 and the PR bodies (critic G-3).

| Constant | Value | Defined at | Provenance |
|---|---|---|---|
| `VOLUME_COLS` | `[pass_attempts, rush_attempts, targets, receptions]` | `volume.py:34` | design (PR #77) |
| `k_shrink` | 8.0 | `volume.py:55` | hand-set (PR #77). Docstring says "learned" (`volume.py:12-14`); it is a constant (KI-NEW-R2) |
| `TALENT_FEATURE_COLS` | `[rapm_rating, smoothed_talent, prior_mean]` | `model.py:48` | design (PR #79) |
| `RATE_STATS` | §4.2 | `model.py:54-58` | design (PR #79) |
| Ridge `alpha` | 1.0 | `model.py:121` | hand-set (PR #79). Never tuned |
| Feature scaling | `StandardScaler` (population sd; zero variance → 1), one per driver | `model.py:139` | design (PR #79) |
| Missing / non-finite feature value | 0.0 (fit `fillna`; predict `nan_to_num`) | `model.py:138`, `:82` | fix for audit finding P1 (CN PR #94, `ccc1ffe`) |
| `min_rows` (SV map) | 10 | `sv_to_points.py:62` | hand-set (PR #81) |
| Degenerate-spread rule | `ptp(credit) == 0` exactly | `sv_to_points.py:82` | hand-set (PR #81); exact equality (KI-P7) |
| `DEFAULT_GAMES` | 17 | `preseason.py:30` | design (PR #83) |
| Smoothed-talent Layer-1 cross-fit | `n_splits = 5`, `seed` = the verdict seed (0), `min_plays = 20` | `verdict.py:128-130`, `:414-415` | hand-set (PR #91) |
| Smoothed-talent filter | `SSParams.from_position(pos)`; default `SSParams()` otherwise | `features.py:103` | `state-space-kalman.md` §4 constants |
| Scoring weights | §4.7 table | `formats.py:3-17` | CN phase-1 Task 3 (cn-docs §2.6); equal to alpha-spec §2.3 |
| Verdict scoring profile | STANDARD | `verdict.py:47`, `:427`, `:563` | hand-set (PRs #88, #90); differs from the alpha default (reconcile-code-first C5) |

### 4.10 Priors (template: Priors)

- **Volume.** The prior is the position mean `m_{p,c}` with strength `k` pseudo-games (§1.3 item 1).
  In the oracle `k = 8` is fixed and the target mixes seasons (§4.1). Required: §4.8.1.
- **Rates.** The ridge is a Gaussian prior on the standardized coefficients, `β ~ N(0, σ²/α · I)`,
  with an unpenalized intercept; it shrinks toward the pooled position rate. There is no prior on the
  rate itself and no exposure-dependent shrinkage. Required: EB-shrunk position rates (§4.8.2).
- **Cross-league prior.** Accepted as a covariate (`features.py:20-23`) but never supplied on real
  data (KI-NEW-P2). Its form is `cross-league-priors.md` (proposed — DR-C9).

### 4.11 Constraints (template: Constraints)

| Invariant | Oracle | Required |
|---|---|---|
| Counts and yards ≥ 0 | clamped silently (`model.py:101`, `:107`) | produced on support; a violation is a typed `ImpossibleStat` |
| `completions ≤ pass_attempts` | not enforced; a linear rate above 1 breaks it | per draw (engine output contract §3.9) |
| `receptions ≤ targets` | holds by linearity of the shrinkage (each column shrinks toward its own position mean, and each player's receptions ≤ targets), **unless an override sets one column** | per draw; enforced structurally by `receptions = targets × catch_probability` |
| Shares sum ≤ 1 per team and role | no shares | typed share-overflow failure |
| Inactive ⇒ zero vector | no availability | per draw |
| Finite values | NaN and inf mapped to 0.0 at predict | typed `ContractError::NonFinite` |

---

## 5. Algorithm, numerics and determinism (template: Seed policy, Tolerances)

### 5.1 Rate-model numerics (measured 2026-10-07; §7.6)

| Comparison | Fixture | Result |
|---|---|---|
| Closed-form standardized ridge (§4.2) vs scikit-learn `Ridge`: **predictions** | `tests/projection/test_model.py::_history` (300 rows), `alpha ∈ {1e-6, 1, 10}` | ≤ 2.14e-16 relative |
| Same: **coefficients and intercept** | same fixture, where `smoothed_talent ≡ rapm_rating` exactly | 7.88e-9 relative at `alpha = 1e-6`; 6.96e-15 at `alpha = 1`; 1.72e-16 at `alpha = 10` |
| Same: coefficients | same rows with `smoothed_talent` decorrelated (`+N(0, 0.5²)`) | ≤ 4.58e-16 relative at every `alpha` |
| `np.polyfit(x, y, 1)` vs closed-form OLS | 500 synthetic points | slope 5.3e-16, intercept 1.6e-16 relative |

- **Coefficients are not identified under collinearity.** The test fixture plants
  `smoothed_talent = rapm_rating` (`test_model.py:36`). With exactly collinear columns, only their
  sum is identified, and at small `alpha` the split depends on rounding. Predictions are unaffected.
  Parity targets for the rate model are therefore **predictions on a declared evaluation grid**, and
  coefficients only on fixtures with a well-conditioned design (§10.3 P-3).
- scikit-learn's `Ridge` uses `solver = "auto"` here (inspected on the fitted model). The engine uses
  the P1-06 generalized-ridge primitive with an unpenalized intercept by centring (engine-spec
  §6.4.3).

### 5.2 Determinism and seed policy

- `projection/` itself has **no randomness**. Every fit is closed form.
- The only stochastic input is `smoothed_talent`, through the Layer-1 cross-fit fold assignment
  (`KFold(shuffle, random_state = seed)`, `layer1-credit.md`).
- **Seed sensitivity (measured, legacy synth, 144 players).** Changing the cross-fit seed from 0 to
  1 moves `smoothed_talent` by up to **0.0487**, against a cross-player SD of **0.198** (about a
  quarter of an SD); corr 0.9968. Thread count 1 vs 4 changed nothing on this machine (max |Δ| = 0).
  CN PR #91 described the path as "more numerically sensitive" and reported a duplicate-`player_id`
  perturbation of about 1e-6 (critic G-3).
- **Seed coupling (oracle defect).** `run_verdict` passes one `seed` to both the bootstrap and the
  Layer-1 cross-fit behind `smoothed_talent` (`verdict.py:414-415`, `:438-445`). Changing the
  evaluation seed changes the forecast being evaluated. Required: model seeds and evaluation seeds
  are separate, recorded inputs (engine-spec §6.8 rule 5); a model seed is part of the model version.

### 5.3 Typed failures (alpha-spec §6.6 rule 4; proposed — DR-B6)

| Condition | Oracle behaviour | Required |
|---|---|---|
| Missing `VOLUME_COLS` column | `groupby.agg` crash (KI-P3; `volume.py:78-81`) | `ContractError::MissingColumn` |
| Missing stat or feature column in the training frame | bare `KeyError` (KI-P4; `model.py:134-145`) | `ContractError::MissingColumn` |
| Partial label schema (absent or all-null stat columns) | filled with 0.0 and a `warnings.warn` (`verdict.py:263-274`) | `LabelError::DegradedColumns{cols}` before any fit. A run with degraded labels is not promotable |
| NaN or inf feature | coerced to 0.0 (`model.py:82`, `:138`) | missing indicator for absent covariates; `NonFinite` for any other non-finite value |
| NaN credit or points in the SV map | `ptp(NaN) == 0` is False, so `polyfit` runs on NaN (KI-P5) | `ContractError::NonFinite` |
| Near-zero credit spread | exact `== 0` check (KI-P7) | tolerance-based degenerate-design error |
| SV-map prediction for an unfitted position | returns 0.0 (`sv_to_points.py:48-49`) | `None` (explicit absence) |
| Unknown position in the volume model | own rate used as target (`volume.py:108`) | `ContractError::UnknownPosition` |
| Negative predicted rate or count | clamped to 0 (`model.py:101`, `:107`) | `ImpossibleStat` |
| Override with unknown column | `ValueError` (`volume.py:110-115`) | kept, as a typed error; also non-finite or negative values |
| Degenerate EB pool (`τ̂² ≤ 0`, too few players) | n/a (fixed `k`) | `DegeneratePool` |
| Scoring: unknown stat key or non-affine rule | key ignored (`engine.py:22-23`) | typed error (engine-spec §5.4) |

---

## 6. Incremental and online behaviour

### 6.1 Oracle

- Every projection function refits from its input frame on each call. Nothing is persisted.
- The verdict **freezes** three things on the pre-first-origin window and reuses them at every
  origin: the SV→points map (`verdict.py:79-95`), the ROS rate models (`verdict.py:237-282`) and
  `smoothed_talent` (`verdict.py:127-191`). "Mirroring the frozen ``sv_map``/V(s)/rate-model
  discipline" (`verdict.py:144-145`).
- Volume is re-projected per origin (`verdict.py:422-425`), but because it reads only the prior
  season, it is constant within a season.

### 6.2 Required

- **Season boundary.** EB parameter records (Layer C `(m̂, k̂)`, Layer D position rates and
  shrinkage) and the rate-model coefficients are re-estimated on the engine-spec §2.4 window at a
  season boundary or periodic rebuild, and persisted as versioned artifacts (engine-spec §6.4.7,
  §8.5; proposed — DR-C6, DR-C15).
- **Game week.** Usage EB posteriors update incrementally with the completed week
  (`(Y_i + k̂ m̂)/(g_i + k̂)` is a running sum), under the as-of and publication-lag rules of
  engine-spec §4.5. The GRID covariates update through their own components (`state-space-kalman.md`
  §6).
- **Freezing in backtests.** A model frozen on a pre-period and reused across origins is allowed
  only when the frozen artifact is fit strictly before the first origin, and the artifact's version
  is recorded with every projection (engine-spec §8.9).

---

## 7. Validation evidence (template: Validation and promotion)

Numbers recorded with threads pinned to 1 and the `reference/python/requirements.lock` pins
(Python 3.11.15; scikit-learn 1.9.1, numpy 2.4.6, scipy 1.17.1, pandas 3.0.6).

### 7.1 Oracle tests (all pass at `59bce1d` on Linux, 2026-10-07)

`python3 -m pytest tests/projection tests/scoring tests/validation`: **208 passed in 34.9 s**
(projection 44, scoring 27, validation 137), isolation guard 0 violations.

| Test file (count) | What it pins | Status for the port |
|---|---|---|
| `tests/projection/test_volume.py` (10) | shrinkage arithmetic (`16/24·10 + 8/24·6`), small-sample pull, most-recent prior season, future-season exclusion, prior-season-only baseline mid-season, empty cases, override validation and replacement, per-position targets | Port as the oracle-compatible mode (Class A). The engine default replaces the fixed `k` and the prior-season-only rule (§4.8.1) |
| `tests/projection/test_model.py` (11) | planted YPA recovery to ±0.15 at `alpha = 1e-6`; every `RATE_STATS` pair fitted; 14-column emission; volume pass-through; zero volume → zero; fumbles and two-point always 0; zero-volume rows excluded; present-but-NaN guard; scoring integration; synth skill > 0 vs last season | Port the assembly and NaN-guard cases as parity (Class A′ on predictions). **Do not port** "unmodeled stats are always zero" as an engine requirement (§3.2) |
| `tests/projection/test_features.py` (12) | RAPM lookup; smoothed talent equals a direct `kalman_two_component` call; position params; missing weeks; independence; priors lookup; frame defaults; sorted union of ids | Port (exact assembly; Class A for the filter given injected inputs) |
| `tests/projection/test_sv_to_points.py` (7) | planted affine recovery; per-position independence; future-season exclusion; current-season weeks before `W`; `min_rows`; zero spread; unknown position → 0.0 | Port as a diagnostic (Class A′). **Divergence:** unknown position → `None` |
| `tests/projection/test_preseason.py` (4) | × games scaling; omission without model or volume; missing talent row | Port as the parity mode of the derived product (Class A) |
| `tests/scoring/test_engine.py` (6) | Standard QB, Half-PPR WR, PPR WR, empty stats, custom config, JSON round trip | Port (Class A; the first parity port) |
| `tests/scoring/test_vor.py` (21) | VOR, replacement level, FLEX, tiers | Oracle-only. Only the cases used by the lineup simulation's roster construction are ported (`evaluation-and-leakage.md` §10) |

### 7.2 Semantic gaps in the tests

- The "beats last season on synth" test (`test_model.py:190-229`) plants next season's actuals from
  the same formula and talent, so it checks wiring, not skill.
- No test checks rate support (`completions ≤ attempts`), exposure weighting, or any stat-vector
  component the oracle lacks.
- No test exercises `prior_mean` with a non-constant value on any path that reaches the verdict.

### 7.3 Verdict-level evidence (legacy synth; `evaluation-and-leakage.md` §7)

On the oracle's synth verdict fixture, the weekly and ROS forecasts beat the deliberately
anti-correlated last-season baseline (ROS ALL margin +0.570, n = 1,440) and lose to persistence
(−1.053) and to the season-to-date mean (−1.242). The fixture plants current-season points with
noise SD 0.5, so persistence is near-perfect. These are wiring checks, not projection evidence.

### 7.4 Real data (historical, non-parity; engine-spec §3.3)

From `docs/07-archive/cautious-nevermore/real-data-results.md` (run `real-2022-2023-universe-fixed`,
STANDARD scoring, iid CIs):

- ROS (H1) via volume × efficiency, all positions, n = 7,035: vs last season −0.015
  [−0.059, +0.029]; vs season-to-date mean −0.194 [−0.270, −0.120]; vs persistence +0.957
  [+0.848, +1.068].
- CN's reading: ROS is volume-dominated and RAPM is one talent feature, so fixing the RAPM universe
  barely moved H1.
- The first run scored ROS with the weekly SV→points map instead and gave −0.864 [−0.932, −0.796].
- **No H1 number with `smoothed_talent` (PR #91) was ever recorded.** `prior_mean` was 0.0 in
  every run.

None of these is a parity target or evidence: the labels are biased (KI-NEW-I1..I5, KI-NEW-V0a), the
CIs are iid (KI-NEW-V1), and the weekly path used current-season participation (proposed —
DR-C1).

### 7.5 Promotion (engine-spec §8.8)

Layer C and D components are promotable only as part of a projection whose evaluation passes the
pre-registered gates of engine-spec §9.4 under PB-MAE, the union player pool and the week-clustered
bootstrap (`evaluation-and-leakage.md`; proposed — DR-C5). Each GRID covariate must show incremental
value against the same component family without it (engine-spec §6.6 requirement 5).

### 7.6 Provenance of measurements first recorded in this spec

| Measurement | Method |
|---|---|
| Ridge closed form vs scikit-learn; polyfit vs OLS (§5.1) | Closed form of §4.2 against the oracle's fitted `RateModel` on the `test_model.py` fixture, `alpha ∈ {1e-6, 1, 10}`, with and without a decorrelated `smoothed_talent` |
| Constant-feature scale and coefficient (§4.2) | Inspection of the fitted `StandardScaler` and `Ridge` |
| `tau_smooth[-1] == tau_filt[-1]` (§1.3 item 2) | 200 random series, lengths 3–19, ~20% missing weeks, 0–2 interventions, all four position parameter sets |
| Smoothed-talent seed and thread sensitivity (§5.2) | `verdict._smoothed_talent_pre_first_origin` on the canonical legacy synth at the first walk-forward origin, `threadpool_limits` 1 vs 4, seed 0 vs 1 |
| Worked examples (§10.4) | Oracle functions called directly |

The probe scripts live in this consolidation's scratch area. They are scheduled for commit under
`reference/python/tools/investigations/` with the other inventory evidence. Until then, these numbers
are *recorded, not reproducible from the repo*.

---

## 8. Known defects and required engine behaviour (template: Known limitations)

The Rust port MUST NOT reproduce any item in this table. Each correction either lands first as an
approved oracle correction in the `reference/python/PARITY.md` ledger (proposed — DR-B1), or is
recorded there as a deliberate divergence (proposed — DR-B6).

| # | Defect | KI / source | Required behaviour |
|---|---|---|---|
| 1 | Rate regressions unweighted: a one-attempt player weighs as much as a 600-attempt player | KI-NEW-R1 | Exposure-weighted fits or count GLMs (§4.8.2; DR-D21) |
| 2 | "Games" = weeks with a stat row; `k_shrink = 8` fixed although documented as learned; postseason rows included | KI-NEW-R2, KI-NEW-I4 | Games from snaps or participation; `k` estimated (§4.8.1); REG-only labels (proposed — DR-C12) |
| 3 | NaN and inf features become 0.0, conflating "no prior" with "prior = 0" | KI-P1 (residual of the PR #94 fix) | Explicit missing indicators |
| 4 | `fumbles_lost` and `two_point_conversions` fixed at 0.0 | KI-NEW-Z55; `model.py:22-24`; reconcile-code-first C6; KI-NEW-I5 | Modelled with strongly shrunk rates (§4.8.2) |
| 5 | Receptions projected as volume; catch rate unmodelled; targets drive nothing | KI-NEW-Z54; §1.3 item 3 | `receptions = targets × catch_probability` |
| 6 | Shrinkage target averages each player's own last prior season (stale seasons of departed players), unweighted by games | KI-NEW-Z52; §4.1 (new) | Window-estimated, exposure-weighted target |
| 7 | Current-season usage never used (prior season only) | KI-NEW-Z53; `volume.py:86-90`; reconcile-spec-first R28 | Engine-spec §2.4 window with in-season data |
| 8 | Silent clamping of negative rates; unbounded linear probabilities | KI-NEW-Z56; `model.py:101`, `:107` | Rates on support; typed `ImpossibleStat` |
| 9 | ROS fit pairs prior-season labels with the first-origin rating while the forecast uses each origin's rating | KI-NEW-Z57; `verdict.py:276-277` vs `tier1.py:161` (new) | One feature definition shared by fit and serve, as-of the label's own period |
| 10 | Weekly forecast = affine map of the RAPM rating, fitted in-sample within the pre-period; the column is named `credit` | KI-NEW-Z59; KI-NEW-R3; `layer1-credit.md` §8 row 18 | Weekly stat vector from Layers A–F; SV map diagnostic only (proposed — DR-C2) |
| 11 | `smoothed_talent` documented as smoothed but equal to the filtered endpoint, with a look-ahead initialisation inside the window | KI-NEW-Z51; §1.3 item 2 (new); KI-#15 | Named and computed as end-of-window filtered talent from a prior-based `x0`/`P0` (proposed — DR-C10) |
| 12 | One seed drives both evaluation resampling and model cross-fitting | KI-NEW-Z62; §5.2 (new) | Separate, recorded model and evaluation seeds |
| 13 | Two V(s) models in one verdict (walk-forward warm-up vs `smoothed_talent` pre-window) | KI-NEW-Z58; `verdict.py:172` vs `backtest.py:166-168` (new) | One versioned V(s) per data version and as-of (`value-model.md`) |
| 14 | Priors wired to no real data; `prior_mean` hard-coded 0.0 | KI-NEW-P2 | Cross-league prior covariate per `cross-league-priors.md` (proposed — DR-C9) |
| 15 | No availability, team environment, allocator, matchup, simulation, quantiles, conditional/unconditional split | reconcile-spec-first R21–R31 | Layers A–F (P1-07, P1-08) |
| 16 | Season-total `week = 0` sentinel for the preseason product | KI-A4 | Typed `horizon` field (proposed — DR-C4) |
| 17 | Scoring: no offset, no version; identity by key-order-sensitive JSON; evaluations use STANDARD | KI-NEW-Z69; KI-NEW-C1; reconcile-code-first C5 | Versioned affine profile with content hash; Half-PPR default |
| 18 | Missing columns crash with bare errors; NaN reaches `polyfit`; exact-zero spread check; SV map returns 0.0 for unknown positions | KI-NEW-Z60; KI-P3, KI-P4, KI-P5, KI-P7 | §5.3 typed failures |
| 19 | Labels reconstructed from play-by-play are biased | KI-NEW-I1..I5 | Official nflverse weekly stats as labels (proposed — DR-C12) |

---

## 9. Open decisions

| Decision | Question | Proposed default |
|---|---|---|
| **DR-C2** | Layers A–F skeleton with GRID as signal provider | Yes; this stack is the Layer C/D seed (proposed — DR-C2) |
| **DR-C3** | Where GRID talent enters Layer D | Role-specific talent as covariates in EB-shrunk per-component rate models (proposed — DR-C3, option (a)) |
| **DR-C4** | Horizons | Weekly primary; ROS and preseason derived from weekly draws (proposed — DR-C4) |
| **DR-C5** | EB estimator and shrinkage strength (`n0`, `k`); recency discount | Gamma–Poisson method of moments for usage (§4.8.1), compared against normal–normal in this spec before P1-07 (proposed — DR-C5) |
| **DR-C12** | Labels and season type | Official nflverse weekly stats, REG only, two-point conversions modelled (proposed — DR-C12) |
| **DR-C11** | VOR scope | VOR internal to the lineup simulation only (proposed — DR-C11) |
| **DR-D21** | Binomial/Poisson GLMs vs exposure-weighted least squares per Layer D component; yards per target vs per reception as the receiving rate base | **Open.** Proposed: binomial GLM for probability components, exposure-weighted least squares or Gamma GLM for yardage; decided on rolling-origin evidence before P1-07 |
| **DR-D25** | `ρ`, minimum games and season-boundary handling for ensemble member 1 (§4.8.3) | **Open.** Form proposed in §4.8.3 |
| **DR-D20** | Stack shares and rates, or component means | Shares and rates, non-negative weights summing to 1 (§4.8.4) |
| **DR-D7**, **DR-D8** | The alpha §5.1 asymmetries | Open (engine output contract §12) |
| **DR-D13** | Whether RB/WR/TE credit measures the player or the unit | Open (`layer1-credit.md`) |
| **DR-D6** | Threshold bonuses in profiles | Reject with a typed error until decided |

---

## 10. Rust port plan (template: Tolerances, Reference examples)

### 10.1 Target crates and work packages (engine-spec §8.1, §9.5; DR-A8 adopted subject to ratification)

| Piece | Crate::module | WP |
|---|---|---|
| Affine scoring profile, presets, content hash, re-scoring of draw matrices | `scoring::profile` | **P1-06** |
| Generalized ridge with unpenalized intercept (standardized rate models) | `models::ridge` | **P1-06** |
| EB primitive (posterior mean, parameter record, estimators) | `models::empirical_bayes` | **P1-06** |
| Affine fit (SV map, diagnostic) | `models::affine` + `evaluation::diagnostics::sv_map` | P1-06 (primitive), P1-09 (diagnostic) |
| Layer C usage and shares (oracle-compatible mode + engine model) | `models::projection::volume` | **P1-07** |
| Layer D rate models | `models::projection::rates` | **P1-07** |
| Talent-feature accessor (filtered vs smoothed enforced by type) | `features::talent` (GRID-derived features, engine-spec §11.6) | P1-07, inputs from P1-12 |
| Preseason and ROS derived products | `models::projection::season` | **P1-08** |
| Layers A, B, E and F | `models`, `simulation` | P1-07 (A, B), P1-08 (E, F) |

### 10.2 Fixtures (parity-fixture contract §3; synthetic only, DR-A11)

Component `projection-stack` and component `scoring`:

- stat-line, usage and profile cases from the oracle tests above, exported as fixtures;
- the rate-model training frames, the fitted standardization (`μ`, `σ`), coefficients, intercepts
  and **predictions on a declared evaluation grid** of talent rows;
- injected Kalman inputs (`y`, `snaps`, `played`, `x0`, `P0`) for the talent endpoint, shared with
  `state-space-kalman` cases.

Each case is `ledger_independent = true` (parity-fixture contract §7 item 4): no correction-ledger
entry changes these stages' semantics for injected inputs.

### 10.3 Parity targets (proposed — DR-B3 classes, as defined in `docs/03-contracts/parity-fixture-contract.md` §6)

| ID | Target | Class | Criterion |
|---|---|---|---|
| P-1 | Standard, Half-PPR and PPR presets on the `test_engine.py` cases and on synthetic stat lines; offset 0 | A | `max |Δ| ≤ 1e-12` |
| P-2 | Volume EB at fixed `k = 8`, prior-season-only rule (oracle-compatible mode), on the `test_volume.py` cases and a synthetic multi-season frame | A | `max |Δ| ≤ 1e-12`; `games_prior` exact |
| P-3 | Standardized ridge rate models | A′ | **Predictions** on the evaluation grid: `‖r − p‖∞/‖p‖∞ ≤ 1e-9`. Coefficients and intercept only on fixtures whose standardized design has full column rank, declared in the case description; the collinear `test_model.py` fixture is predictions-only (§5.1) |
| P-4 | Stat-line assembly with injected rates and volumes, including clamp positions and zero-volume rows | A | `≤ 1e-12`; which entries are clamped matches exactly |
| P-5 | Talent-feature frame assembly; end-of-window talent from injected filter inputs | exact / A | ids, defaults and NaN positions exact; values `≤ 1e-12` |
| P-6 | SV→points OLS (diagnostic) | A′ | `≤ 1e-9` relative; omitted positions exact |
| P-7 | Preseason `× DEFAULT_GAMES` parity mode | A | `≤ 1e-12` |
| P-8 | Typed-failure divergences of §5.3 | divergence | Rust returns the typed error; listed in `reference/python/PARITY.md` |
| P-9 | Engine Layer C/D models, Layers A, B, E, F, distributions | spec golden | No oracle source. Golden tests on the stat-vector synthetic world (engine-spec §12.2; proposed — DR-B4) |

### 10.4 Reference examples (become golden unit tests)

**Scoring** (oracle-verified):

| Stat line | Standard | Half PPR | Full PPR |
|---|---|---|---|
| WR: 6 receptions, 90 receiving yards, 1 receiving TD | 15.0 | 18.0 | 21.0 |
| QB: 300 passing yards, 3 passing TD, 1 INT, 20 rushing yards, 1 fumble lost | 22.0 | 22.0 | 22.0 |

**Volume EB** (`k = 8`, AsOf(2023, 1); three WRs in 2022 with 16, 16 and 2 games at 10, 2 and 6
targets per game, and 7, 1 and 3 receptions per game):

```text
position means: targets 6.0, receptions 11/3
A: targets 16/24·10 + 8/24·6 = 8.666666666667     receptions 16/24·7 + 8/24·11/3 = 5.888888888889
B: targets 16/24·2  + 8/24·6 = 3.333333333333     receptions                      1.888888888889
C: targets  2/10·6  + 8/10·6 = 6.0                receptions  2/10·3 + 8/10·11/3 = 3.533333333333
games_prior: A 16, B 16, C 2
```

**Standardized ridge** (one feature `x = [−1, 0, 1]`, rates `y = [4, 6, 8]`, `alpha = 1`):

```text
σ = sqrt(2/3),  z = x/σ,  Σz² = 3
β̂_z = Σ z(y − ȳ) / (Σz² + α) = 4.898979485566 / 4 = 1.224744871392
b̂   = 6.0
predict(x) = 6 + (β̂_z/σ)·x = 6 + 1.5x   →  predict(1) = 7.5, predict(2) = 9.0
(OLS slope 2 is shrunk by Σz²/(Σz² + α) = 3/4)
```

**SV→points** (12 WR rows, weeks 1–4, credit `[0, 1, 2]` per week with points `[5, 7, 9]`):

```text
AsOf(2023, 5): 12 rows ≥ min_rows → (a, b) = (5.0, 2.0); predict(WR, 0.5) = 6.0; predict(RB, 0.5) = 0.0 (oracle)
AsOf(2023, 4): 9 rows < min_rows  → WR omitted
```

---

## 11. Superseded-spec mapping

| Superseded section | Requirement | How this spec satisfies it | Gap |
|---|---|---|---|
| alpha-spec §2.3 | Half-PPR default; Standard and PPR built in; custom scoring as a versioned affine map | §4.7 | Non-affine rules (DR-D6) |
| alpha-spec §2.4 | Exact three-season window | §4.8.1 item 3; §6.2 | The oracle reads only the prior season |
| alpha-spec §5.1 | Stat-vector-first targets | §3.2 coverage table | Availability, snap share, sacks, fumbles, two-point conversions are new work |
| alpha-spec §5.2, §5.3 | Conditional/unconditional; distributions | §2.1 (Layer A, F) | No oracle source; P1-08 |
| alpha-spec §5.4 | Affine scoring transform; re-score draws | §4.7 | Draw matrices arrive with Layer F |
| alpha-spec §6.1 Layers C, D | Constrained allocation; shrunk efficiency rates | §4.8.1, §4.8.2 | Allocator and GLM families are new work |
| alpha-spec §6.2 (Empirical Bayes, Affine mapping, Ridge rows) | Rate shrinkage; stat-vector→points; small-sample rate models | §1.3 item 1; §4.2; §4.7; §4.8 | EB estimator choice (DR-C5) |
| alpha-spec §6.4 | Ensemble member 1 | §4.8.3 | Weights open (DR-D25) |
| alpha-spec §6.5 | Explanation fields | §3.3 | — |
| alpha-spec §6.6 rule 4 | Typed failures for NaN, impossible stats | §5.3 | — |
| alpha-spec §12.1 / §12.2 | Unit tests for scoring transforms, EB updates; goldens for stat-line means and scoring | §7.1, §10.3, §10.4 | Spec goldens for Layers A–F await the stat-vector synthetic world (DR-B4) |
| final-build-spec §11.6 | Persisted EB prior parameters | §4.8.1 item 2; §6.2 | — |
| final-build-spec §11.7 | Affine primitive with serialization | §4.7; §10.1 | — |
| CN roadmap §4.5 (archived) | volume × efficiency; GRID as a feature | §1.2; DR-C2, DR-C3 | — |
