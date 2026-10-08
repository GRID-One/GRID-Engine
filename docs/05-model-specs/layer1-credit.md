---
model-spec-id: MS-LAYER1-CREDIT
status: Draft            # equations below are NOT yet approved by the statistical owner
statistical-owner: repository owner (holds every engine-spec §1.4 role today)
version: 0.1.0 (2026-10-07, written under consolidation WP P0-01)
supersedes: none (first engine model spec for this component). Replaces the cautious-nevermore
  prose now archived (non-authoritative) under docs/07-archive/cautious-nevermore/.
---

# Model spec — Layer 1: cross-fitted per-play credit (offseason tier) and the participation-free Layer-1′ credit (live tier)

Consolidation work package **P0-01** (ADR-011 "Engine-only pivot", ADR-012 "Python reference
oracle"). This is a contract-level model spec. Engine-spec §1.5 (DR-A2, adopted subject to owner
ratification) places model specs above the work package, the tests and the code. The equations
bind the Rust implementation once the statistical owner approves them. An implementing agent MUST
NOT redefine them (superseded alpha-spec §6.6, carried into engine-spec §6.8).

**How to read this spec.**

- **Oracle behaviour.** Sections 3–7 describe the Python reference oracle **exactly as
  implemented** at cautious-nevermore (CN) `59bce1d`. Code is cited as
  `reference/python/<path>:<line>`. `reference/python/MANIFEST.tsv` lists every cited file as
  `verbatim`, so the line numbers equal CN `59bce1d`. The oracle is executable evidence, not
  authority (engine-spec §1.7).
- **Engine requirements** use MUST, SHOULD and MAY. Where a requirement differs from the oracle,
  the difference is a deliberate divergence. It is recorded in §8 and in
  `reference/python/PARITY.md`.
- **Decision tags.** "(proposed — DR-xx)" marks a default that waits on the owner
  (`docs/00-meta/decision-register.md`). It is not settled. `DR-NEW:<slug>` marks a decision
  raised by this spec that the register does not yet hold.
- **Defect IDs.** `KI-` IDs are entries in `docs/00-meta/known-issues.md`.
- **Synthetic generators.**
  - *Legacy synth* is the oracle's generator as imported. Its defenders are drawn from the
    offense's own roster (KI-NEW-Y0; critic G-1). On the legacy synth the Layer-1 "opponent"
    feature therefore sums the ratings of the **offense's own** defenders. Every legacy number
    about the opponent adjustment is **semantically void** and is quoted only for traceability.
  - *Fixed synth* is the defender-fixed generator, applied in memory by
    `reference/python/tools/investigations/_common.py:apply_defender_fix`. It keeps the legacy
    planted `team_strength`.
- **Real-data numbers** are nflverse 2023, weeks ≤ 18 (all `REG`). They are **historical,
  non-parity** evidence (DR-A11; engine-spec §3.3). They were measured with an in-sample V(s) and
  an in-sample season RAPM, so they describe scale and structure only.
- **Inventory reports.** "critic", "cn-issues", "cn-docs", "reconcile-code-first" and
  "reconcile-spec-first" are the consolidation inventory, kept verbatim in
  `docs/06-sessions/2026-10-01-consolidation-inventory/`.
- **Companion specs.** `rapm-attribution.md` is cited as "RAPM §x" and `state-space-kalman.md` as
  "SSK §x".

## 0. Template conformance

This spec follows `docs/99-templates/template-model-spec.md`, extended with numbered sections in
the house format of `rapm-attribution.md`.

| Template heading | Where it is covered |
|---|---|
| Target | §1.1, §3.2 |
| Inputs | §3.1 |
| Equations | §4.1–§4.8 |
| Priors | §4.10 |
| Constraints | §4.11 |
| Seed policy | §5.2 |
| Tolerances | §5.3, §10.3 |
| Reference examples | §10.4 |
| Explanation fields | §3.3 |
| Validation and promotion | §7, §10.3 |
| Known limitations | §1.3, §8, §9 |

---

## 1. Purpose and statistical intent (template: Target)

### 1.1 What the component estimates

| Estimand | Definition | Units / support | Status |
|---|---|---|---|
| Context residual `r_i` | `dv_i − ĝ(x_i)`: the play's situational value above what a context model `g` of game state and opponent defence predicts, with `ĝ` fitted **out of fold** (§4.4) | EP per play; real | Implemented (oracle) |
| Weekly credit `c_{p,w}` | Mean of `r_i` over the plays in week `w` on which player `p` is listed in `off_players` (§4.5) | EP per play; real | Implemented (oracle), QB since v0, any position since PR #80 |
| Exposure `n_{p,w}` | The number of those plays. It sets the Kalman measurement precision `R = r_scale / max(n, 1)` (SSK §4.1) | count ≥ 1 | Implemented (oracle). Called `snaps` in the code; it counts listed pass/run plays, not official snaps |
| Layer-1′ role credit `c′_{p,ρ,w}` | Participation-free per-role credit for roles ρ ∈ {dropback, carry, target} identified from play-by-play (§4.8) | EP per play; real | **Proposed — DR-C1**; definition open (DR-D15). No oracle counterpart |

The outputs are **signals, not projections**. They are the observation of the state-space layer
(SSK §3.1) and, in the offseason tier, the re-seed of the RAPM fixed point (RAPM §4.7). They are
never fantasy points (engine-spec §6.2; proposed — DR-C2).

### 1.2 What it treats as nuisance (load-bearing intent, preserved verbatim)

The oracle's comments state the intent. The Rust port MUST keep it.

- **Layer 1 supplies the weekly signal, and the context model is shared.**
  `reference/python/backend/grid/layers.py:16-23`:

  > "Layer 1 (event credit): a cross-fitted, opponent-adjusted residual. Its job is to supply the
  > *weekly* signal the state-space layer consumes. The context model g(state,
  > opponent-defense-rating) is position-agnostic, so it is fit ONCE per plays frame and shared
  > across every player's weekly aggregation (`_cross_fitted_context_residual`);
  > `layer1_qb_weekly` backs onto it for a single QB, `layer1_all_players` generalizes it to any
  > set of players (roadmap Phase 2b) -- tractable at RB/WR/TE scale, unlike refitting per player."

- **The context model never sees who is on offense.** `layers.py:500-503`:

  > "g is position-agnostic (conditions on situational state + aggregate on-field
  > opponent-defense rating only, never who is on offense), so this is computed ONCE per plays
  > frame and shared across every player's weekly aggregation"

- **Credit is value above context; exposure is precision.** `layers.py:525-528`:

  > "Context model g(state, opp_def_rating) is trained out-of-fold on ALL offensive plays; the
  > residual dv - g_oof is the value above what state + opponent strength explain. The focus QB's
  > weekly credit = mean residual on his on-field offensive snaps; snaps -> measurement precision
  > for the Kalman."

- **The fixed point is light coupling.** `layers.py:25-27` and `:602-605`:

  > "Fixed point: Layer 2 -> defender ratings -> Layer 1 opponent adjustment -> (optional) Layer 1
  > QB credit re-seeds the QB's Layer-2 prior. RAPM already does most opponent/teammate adjustment
  > jointly, so the loop is light here."

  > "# re-seed QB prior with his season Layer-1 credit (light coupling)"

**Nuisance:** game state and opponent defence. **Estimand:** per-play value above that context,
per player-week. **Not modelled here:** volume and opportunity (Layer C), availability (Layer A),
and the dynamics (SSK §1).

### 1.3 What the credit is relative to, and what it is not

These facts were derived from the code and verified by measurement (§7). They bind the port and
every consumer.

1. **Relative to the league-average context expectation.** `ĝ` is fitted on every play of the
   frame, whichever team has the ball. The credit is therefore relative to the frame's average
   offense in the same state against an opponent of the same strength. The out-of-fold mean
   residual is near zero: −1.2e-3 (legacy) and −9.3e-5 (fixed) EP per play.
2. **Not teammate-adjusted: it is an on-field-unit plus-minus.** Every offensive player listed on
   play `i` receives the same `r_i`. Two players with identical on-field play sets get identical
   credit. This is an algebraic property of §4.5, not an approximation. Measured consequences:
   - Fixed synth: a starter's season credit correlates 0.961–0.967 with the summed planted
     ability of his team's starting offense, against 0.337–0.620 with his own planted ability
     (RB/WR/TE). Starters' within-team credit SD is 0.043 against 0.245 overall (§7.4).
   - Real 2023: a team's most-used QB and WR have weekly credits that correlate at a median 0.909
     across 31 teams. The QB's season credit correlates 0.974 with his team's mean residual
     (§7.7).
   - RAPM separates teammates. Layer 1 does not. The oracle docstring claims only
     "opponent-adjusted" and leaves teammate adjustment to RAPM (`layers.py:25-27`).
   - For QBs the unit signal is mostly the QB's own (synth QB ability SD 0.090 against 0.022–0.030
     for other positions, `reference/python/backend/grid/synth.py:37`). For RB/WR/TE the weekly
     credit is mostly the team offense. Whether the engine keeps the unit estimand or adjusts for
     teammates is open (DR-D13; §9).
3. **Not a RAPM coefficient.** Credit is a per-play residual mean. A RAPM rating is a ridge
   coefficient. Their scales differ: legacy-synth season credit regresses on rating with slope
   1.19–1.40 by position (SSK §7.3). The fixed-point re-seed copies one into the prior of the other
   (RAPM §4.7; §4.6).
4. **Not causal in time on the batch path.** `ĝ` is fitted on the whole frame and the opponent
   ratings come from a RAPM on the whole frame. The credit for week `w` therefore depends on plays
   from weeks after `w`. This is admissible for a completed season. It is not admissible as a live
   observation (§6, §8 item 8).
5. **Participation-dependent.** Membership reads `off_players`
   (`layers.py:533, 549, 580`). The opponent feature reads `def_players` (`layers.py:493`). Free
   participation is published once, after the postseason (critic G-5). The oracle's Layer 1 is
   therefore an offseason-tier quantity (§1.4; proposed — DR-C1).

### 1.4 Two tiers (proposed — DR-C1)

| Tier | When | Credit | Opponent feature | Context model |
|---|---|---|---|---|
| **Offseason** | After season `S`'s participation is published (engine-spec §4.5) | Layer 1: on-field credit over listed plays (§4.5), corrected per §8 | Gauge-invariant per-play defensive effect from the offseason RAPM, fold-wise or frozen (§4.3) | Cross-fitted on the completed-season window (§4.4) |
| **Live** | Each completed game-week of season `S` | Layer-1′: per-role involvement credit (§4.8) | Team-level defensive effect from the live-tier team ridge, as of the start of the week (§4.8.3) | Frozen, fitted on completed seasons; no in-season refit (§4.8.2) |

Participation-dependent credit MUST NOT feed any live or locked projection (superseded alpha-spec
§12.3: "current-season participation unavailable at serve time must not appear in promoted live
features"). A historical backtest that computes Layer 1 from in-season participation MUST label
every output `research_only = true` (engine-spec §6.3).

---

## 2. Position in the engine

### 2.1 Signal stack (engine-spec §6.2)

```text
plays contract (participation) ─┐                       OFFSEASON TIER
V(s) → dV  (value-model.md) ────┼─► RAPM β, γ (RAPM §4) ──► per-play defensive effect δ_i (§4.3)
                                └─► Layer 1: r_i = dV − ĝ_oof(s, δ) ──► c_{p,w}, n_{p,w}   ◄── this spec
                                        ├─► fixed-point re-seed of the QB prior (RAPM §4.7; §4.6)
                                        └─► retrospective Kalman / RTS on completed seasons (SSK §3.1)
plays contract (involvement roles) ─┐                   LIVE TIER
V(s) → dV ──────────────────────────┼─► team ridge E_def (engine-spec §6.2, Component 5)
                                    └─► Layer-1′: r′_i = dV − g′(s, δ^team) ──► c′_{p,ρ,w}, n_{p,ρ,w}   ◄── this spec
                                                └─► Kalman one step per player-role-week (SSK §6.4)
```

### 2.2 Consumers

| Consumer | What it takes | Oracle site |
|---|---|---|
| State-space layer (SSK §3.1) | `y_w = c_{p,w}`, `snaps_w = n_{p,w}`, `played_w = true`; absent weeks `y = NaN`, `played = false` | Golden and Tier-0: `reference/python/tests/grid/golden_master.py:75-90`, `tests/grid/test_tier0_recovery.py:53-60`, `run_demo.py:64-69`. Calibration: `tests/grid/test_calibration_synth.py:64-97`. Verdict: `reference/python/backend/validation/verdict.py:173-190`, `:206-234` |
| RAPM fixed point (RAPM §4.7) | `mean_w c_{focus,w}` as the focus QB's prior mean | `layers.py:601-605` |
| ROS talent feature `smoothed_talent` (projection-stack.md; engine-spec §11.6) | RTS-smoothed end-of-window talent over weekly credit | `verdict.py:127-191` (PR #91) |
| `sv_to_points` | Its docstring says it maps weekly Layer-1 credit to points (`reference/python/backend/projection/sv_to_points.py:3-9`) | **Not true on the verdict path.** `_fit_points_map` fills the `credit` column with the first origin's **RAPM rating** (`verdict.py:79-95`; KI-NEW-R3). In this spec "credit" means Layer-1/1′ only |

`weekly_update` does **not** compute Layer 1. It feeds the cumulative RAPM coefficient with
`snaps = 1` to the Kalman (`reference/python/backend/pipeline/weekly_update.py:280-309`). That is
a defect (KI-NEW-W3, KI-#24; critic X-13). The validated observation is weekly credit with real
exposure (proposed — DR-C10).

### 2.3 Contracts and neighbouring specs

- Inputs: `docs/03-contracts/plays-contract.md` (`off_players`, `def_players`, `week`, `season`,
  `season_type`, state columns, and `dv` from `value-model.md`). Layer-1′ needs involvement roles
  that the v1 plays contract does not yet carry (§3.1, §8 item 21).
- Outputs: `docs/03-contracts/engine-output-contract.md` §4.2 (GRID signal block) and the
  `layer1_credit` table (engine-spec §8.5).
- Parity: `docs/03-contracts/parity-fixture-contract.md` §5 (rows `layer1-credit` weekly credit
  and fixed point) and §6; `reference/python/PARITY.md`.
- Neighbours: `value-model.md` (dV), `rapm-attribution.md` (defender ratings, fixed point),
  `state-space-kalman.md` (observation), `synthetic-world.md` (planted roles, DR-B4),
  `evaluation-and-leakage.md` (as-of, two-path, publication lag), `projection-stack.md` (Layer D
  role talent, DR-C3).

---

## 3. Inputs and outputs (template: Inputs, Target)

### 3.1 Inputs

**Oracle signatures** (`layers.py:497, 522, 541, 561`):

- `_cross_fitted_context_residual(plays, ratings_lookup, n_splits=5, seed=0)`
- `layer1_qb_weekly(plays, ratings_lookup, focus_qb, n_splits=5, seed=0)`
- `layer1_all_qbs(plays, ratings_lookup, qb_ids, n_splits=5, seed=0)`
- `layer1_all_players(plays, ratings_lookup, player_ids, n_splits=5, seed=0, min_plays=20)`

| Input | Oracle form | Units / type | Null and failure semantics (oracle → required) | As-of rule |
|---|---|---|---|---|
| `down`, `ydstogo`, `yardline_100` | `STATE_COLS` (`reference/python/backend/grid/value.py:25`), cast to float (`layers.py:508`) | integer-valued | NaN passes to the booster, which accepts NaN features → MUST be a typed `ContractError` at contract validation (plays-contract §11) | play's own state |
| `dv` | `plays["dv"]` (`layers.py:509`) | EP per play | Missing column: bare `KeyError`. NaN: sklearn raises `ValueError: Input y contains NaN` (verified) → typed `ContractError` | V(s) frozen per data version and as-of (value-model.md; KI-NEW-W4) |
| `off_players` | tuple per play; membership by `pid in t` (`layers.py:533, 549, 580`) | player ids | Duplicates collapse to one membership (legacy synth has 245 plays with duplicated ids, plays-contract D-8). `()` (participation absent or unmatched) silently yields no credit → MUST be rejected unless participation is `Listed` (plays-contract D-5) | participation of season `S` only at as-of ≥ its publication timestamp (engine-spec §4.5; proposed — DR-C1) |
| `def_players` | tuple per play, iterated in the sum (`layers.py:493`) | player ids | Duplicates would count twice (none in either synth or in 2023; verified). Same `()` hazard as above | as above |
| `ratings_lookup` | `{player_id: rating}`; read with `.get(p, 0.0)` (`layers.py:492-493`) | EP per play | **A defender missing from the lookup silently contributes 0.0** → MUST be an explicit coverage policy with a counted, typed failure (§5.4) | ratings MUST be out of fold or frozen before the frame (§4.3; KI-NEW-A5) |
| `week` | int, the aggregation key (`layers.py:535, 584`) | — | **`season` is ignored**: a multi-season frame merges equal week numbers (§8 item 6). The verdict works around it by calling per season (`verdict.py:176-182`) | — |
| `season_type` | absent from the oracle contract | — | Postseason rows enter (KI-NEW-V0a) → Layer 1 consumes only the season types the plays contract admits (proposed — DR-C12) | — |
| `player_ids` / `qb_ids` / `focus_qb` | lists / id | — | Unknown or absent ids are skipped silently (`layers.py:550-551, 581-582`); `layer1_qb_weekly` returns an empty frame for an absent QB, and `fit()` then turns it into a NaN prior (RAPM §5.4) → explicit `no_data` and a typed error where a value is required | — |
| `n_splits`, `seed`, `min_plays` | ints | — | Not validated. `n_splits > n` raises inside sklearn → typed `InvalidParams` | — |
| **Layer-1′ only:** involvement roles per play | not in the oracle contract. nflverse pbp carries `passer_player_id`, `rusher_player_id`, `receiver_player_id`, `sack`, `qb_scramble` (verified on 2023, §7.7) | GSIS ids, flags | MUST be added to the plays contract as an optional, versioned field (plays-contract §12 MINOR bump); absence is `NotAvailable`, never `()` | pbp is published in season |
| **Layer-1′ only:** snap counts | not in the oracle. Provider: PFR via nflverse, keyed by `pfr_player_id`, needs a PFR→GSIS crosswalk (`docs/04-providers/nflverse/README.md:76`); licence unverified (`docs/04-providers/nflverse/access-and-license.md:40`) | count per player-game | `SourceUnavailable` when absent; never a default of 1 (KI-#24) | `available_at ≤ lock_at` (engine-spec §4.5) |

### 3.2 Outputs

**Oracle outputs.**

- `_cross_fitted_context_residual` returns `y − oof`, a float array aligned to the frame's rows
  (`layers.py:519`). It is stored as the internal column `_resid` (`layers.py:531, 576`;
  plays-contract §2.2 row `_resid`).
- `layer1_qb_weekly` returns a frame `[week, qb_credit, snaps]`, one row per week with at least one
  listed play, sorted by week (`layers.py:535-537`).
- `layer1_all_players` returns `{player_id: frame[week, credit, snaps]}` for players with at least
  `min_plays` listed plays in the **whole frame** (`layers.py:578-587`). Ineligible players are
  absent from the dict.
- `layer1_all_qbs` returns `{qb_id: frame[week, qb_credit, snaps]}` and drops QBs whose fit raises
  (`layers.py:552-556`; KI-G6).

**Required engine output** (the `layer1_credit` record; engine-spec §8.5; field names fixed by
`docs/03-contracts/engine-output-contract.md`):

| Field | Definition | Notes |
|---|---|---|
| key | `(player_id, season, week, tier, role)` | `role = on_field` for Layer 1; `dropback`, `carry` or `target` for Layer-1′ |
| `credit` | `c_{p,w}` (§4.5) or `c′_{p,ρ,w}` (§4.8) | EP per play |
| `exposure` | `n_{p,w}` or `n_{p,ρ,w}` | Never defaulted. `exposure = 0` means no observation: the record is `no_data`, not a zero credit |
| `no_data` | true when the player has no qualifying event that week, or is below the eligibility rule | Consumers map it to `played = false, y = NaN` (SSK §3.2 missing-week contract) |
| `context_model_version` | hash of the fitted context model(s), the feature schema and the training window | engine-spec §8.8 |
| `opponent_source` | the rating source and version that produced the opponent feature (§4.3) | — |
| `fold_id` (offseason tier) | the fold that scored each play; persisted with the residuals | audit and parity surface |
| `research_only` | true whenever participation of a season was used before its publication timestamp | engine-spec §6.3 |
| provenance | data version, V(s) version, as-of, seeds, booster backend and version (DR-C8) | engine-spec §8.5, §8.8 |

### 3.3 Explanation fields (template: Explanation fields; engine-spec §6.7)

- **`recent_credit`** per role: the last completed week's credit and exposure, labelled with tier
  and role. Units EP per play; positive means above the league-average context expectation.
- **Sign convention for the opponent adjustment**: a tougher opponent defence lowers the context
  expectation, so the same raw dV earns more credit. The explanation reports
  `mean_w(ĝ(s, δ) − ĝ(s, δ̄))` over the player's plays as "schedule adjustment", where `δ̄` is the
  frame mean of the opponent feature.
- **Unit-credit warning.** While DR-D13 is open, an explanation MUST NOT describe
  Layer-1 credit of an RB, WR or TE as the player's individual efficiency. It is the efficiency of
  the offensive unit while he was on the field (§1.3 item 2).
- Credit itself is not shown as a projection driver. The Kalman state that observes it is
  (SSK §3.5).

---

## 4. Model and equations (template: Equations, Priors, Constraints)

### 4.1 Notation

- A **frame** is the set of plays `i = 1..n` the caller passes. In the oracle it is a whole
  season, a slice before an origin, or a single-season group (`verdict.py:179`).
- `s_i = (down_i, ydstogo_i, yardline_100_i)`.
- `O_i` is the set of ids in `off_players(i)`. `Δ_i` is the tuple `def_players(i)`.
- `ρ(p) = ratings_lookup.get(p, 0.0)`.
- `week_i` is the week number. The oracle ignores `season_i`.

### 4.2 Context model `g` (oracle, `layers.py:507-518`)

```text
D_i = Σ_{p ∈ Δ_i} ρ(p)                                   (layers.py:493, :507)
x_i = (down_i, ydstogo_i, yardline_100_i, D_i) ∈ ℝ⁴      (layers.py:508)
y_i = dv_i                                               (layers.py:509)
```

`g` is sklearn `HistGradientBoostingRegressor` (`layers.py:514-516`) with squared-error loss:

```text
F_0     = mean of y over the rows the booster trains on   (sklearn baseline for squared error; verified.
          With early stopping active these are the fold's rows minus its validation split)
F_m     = F_{m−1} + 0.1 · h_m,     m = 1..M,  M = n_iter_ ≤ 200
h_m     = regression tree on histogram-binned x (≤ 255 bins per feature), fitted to the
          negative gradient, max_depth 3, ≤ 31 leaves, ≥ 150 samples per leaf, L2 penalty 0
ĝ(x)    = F_M(x)
```

**Early stopping is on, implicitly.** The oracle sets `max_iter=200` but leaves
`early_stopping='auto'`. In scikit-learn 1.9.1, `'auto'` means "on when the training sample has
more than 10,000 rows" (`sklearn/ensemble/_hist_gradient_boosting/gradient_boosting.py:534`,
`self.do_early_stopping_ = n_samples > 10_000`). Each canonical training fold has 13,460 (legacy),
14,201–14,202 (fixed) or 27,068–27,069 (real 2023) rows, so early stopping is active:

- 10% of the training fold is held out (`validation_fraction = 0.1`), split with a seed derived
  from `random_state`;
- training stops when none of the last 10 validation scores beats the score 11 iterations back
  by more than `tol = 1e-7` (`n_iter_no_change = 10`, `scoring = 'loss'`);
- observed `n_iter_` per fold: 200, 170, 146, 136, 144 (legacy); 66, 62, 58, 72, 47 (fixed);
  25, 98, 61, 48, 83 (real 2023).

The effective model complexity is therefore set by noise in an internal split. The estimator also
changes discontinuously at 10,000 training rows: smaller frames (training folds of 10,000 rows or fewer, for
example a four-week canonical-synth pre-window of about 4,800 plays) run all 200 iterations on
100% of the fold. §8 item 9 makes this explicit.

### 4.3 The opponent feature: oracle `D_i` and the required per-play defensive effect

**Oracle.** `D_i` sums the current ratings of the listed defenders. In `fit()` the ratings are the
RAPM ratings of the same iteration, fitted on **all** plays of the frame (`layers.py:597-603`). In
the verdict they are the first origin's RAPM ratings over the same pre-window
(`verdict.py:173`). Both are in-sample to the plays being scored (KI-NEW-A5).

**Sign.** Defenders enter the RAPM design at −1, so a larger `β_p` is a better defender
(RAPM §1.3). A larger `D_i` is a tougher opponent and should lower the context expectation.
Measured on the fixed synth, corr(`D_i`, state-only residual) = −0.170, the expected sign. The
legacy value (−0.258) is void: there `D_i` sums the offense's own defenders.

**Defects of `D_i`.**

1. **Gauge dependence.** `D_i` omits `γ_def[def_team(i)]`. When every defensive play of team `t`
   lists exactly `k_def` of `t`'s defenders, RAPM's fit is invariant to `β_p += a` for those
   defenders together with `γ_def[t] −= k_def·a` (RAPM §1.3), but `D_i` moves by `k_def·a`. Only
   the ridge penalty fixes the split.
2. **In-sample ratings.** On real 2023 the in-sample feature gives the context model an
   out-of-fold R² of 0.0144. With fold-wise RAPM ratings (each fold's ratings fitted on its
   training plays only) the R² is **−0.0184**. The whole apparent opponent signal on real data is
   in-sample leakage (§7.7). On the fixed synth the same change moves R² from 0.1343 to 0.1245
   and the focus-QB weekly credit by up to 0.024.
3. **Silent coverage.** A defender absent from the lookup contributes 0.0 (`layers.py:493`).

**Required (offseason tier).** The opponent feature MUST be the gauge-invariant per-play
defensive effect

```text
δ_i = γ_def[def_team(i)] + Σ_{p ∈ Δ_i} β_p
```

whose exposure-weighted team mean is `E_def[def_team(i)]` (RAPM §4.5). `β` and `γ_def` MUST be
either:

- (a) **fold-wise**: fitted on the training plays of the fold that scores `i`, so
  `ĝ^{(−k)}` and `δ` share one training set; or
- (b) **frozen**: the previous completed window's offseason RAPM, which is out of sample for the
  frame.

The choice between (a) and (b) is DR-D14. Coverage MUST be total: every id
in `Δ_i` has a rating in the declared source, or the play fails with
`CreditError::UncoveredDefender` (§5.4). Under (b), a defender new to the league has no frozen
rating. The policy for that case (prior mean 0 with a recorded count, or the team effect alone) is
part of the same decision.

**Required (live tier).** No participation exists, so `δ^team_i = E_def^{team}[def_team(i)]` from
the live-tier team ridge on dV (engine-spec §6.2, Component 5), estimated through week
`week_i − 1` (§4.8.3).

### 4.4 Cross-fitting (oracle, `layers.py:511-519`)

```text
fold map  k(i) ∈ {1..K}  from  KFold(n_splits=K, shuffle=True, random_state=seed)  over row positions 0..n−1
ĝ^{(−k)}  = g fitted on {(x_i, y_i) : k(i) ≠ k},   random_state = seed
r_i       = y_i − ĝ^{(−k(i))}(x_i)
```

- `K = 5` and `seed = 0` by default (`layers.py:497`).
- Folds are **play-level** and shuffled. Plays of the same week, drive and player fall in
  different folds.
- The calibration gate instead holds out whole weeks: `GroupKFold(n_splits=5)` grouped by `week`
  (`tests/grid/test_calibration_synth.py:70-78`). Its docstring calls this "stricter here than
  production layer1_qb_weekly's play-level CV" (`:49-53`).
- The fold assignment depends on the **row order** of the frame. The Rust port MUST take it as an
  explicit input for parity (§10.2) and MUST define it as a function of play keys in production
  (§5.2).

**Required** (DR-D14): week-grouped folds (reconcile-code-first C18), the
opponent feature of §4.3, early stopping explicit (§8 item 9), and the fold map persisted with the
residuals.

### 4.5 Weekly aggregation (oracle, `layers.py:533-536, 578-587`)

```text
P(p, w)   = { i : week_i = w  and  p ∈ O_i }
n_{p,w}   = |P(p, w)|                                     ("snaps", the group size of _resid)
c_{p,w}   = (1 / n_{p,w}) · Σ_{i ∈ P(p,w)} r_i            ("credit" / "qb_credit", the group mean)
N_p       = Σ_w n_{p,w}
eligible  ⇔ N_p ≥ min_plays (= 20)                        (layer1_all_players, :581)
```

- Rows exist only for weeks with `n_{p,w} ≥ 1`. A week with one listed play yields a credit from a
  single residual. The Kalman weights it through `R = r_scale / 1`.
- Membership is a set test, so a duplicated id in one play counts once.
- `layer1_qb_weekly` applies no threshold. `layer1_all_qbs` pre-filters `N_p ≥ 20` before calling
  it and refits the context model per QB (`layers.py:548-553`).
- `layer1_all_players` is behaviour-preserving for QBs: for the same `n_splits` and `seed` it
  returns frames equal to `layer1_qb_weekly`'s, renamed (`tests/grid/test_attribution.py:113-131`).
  It fits the context model exactly `n_splits` times regardless of the player count (`:164-181`).
- **Eligibility looks ahead.** `N_p` counts the whole frame, so on a whole-season frame whether
  week 3 is reported depends on later weeks. Required: eligibility is evaluated as of the record's
  week, and an ineligible week is reported as `no_data`, not omitted (§8 item 7).
- **Season collision.** Grouping is by `week` only. Measured: a frame split into two seasons with
  overlapping week numbers returns 7 focus-QB rows instead of 12 (§7.4). Required: key by
  `(season, week)`.

### 4.6 Season mean and the fixed-point re-seed (oracle, `layers.py:603-605`)

```text
m_focus = (1 / |W_focus|) · Σ_{w ∈ W_focus} c_{focus,w}      (unweighted mean of weekly means)
prior_mean_players = { focus_qb : m_focus }                   (enters RAPM at the next iteration)
```

- The mean is **not** snap-weighted: it weights a 1-play week like a 100-play week. In the §10.4
  example it gives 0.0278 where the snap-weighted mean is 0.0667.
- `m_focus` is a residual mean on the credit scale. It becomes the prior **mean** of a ridge
  coefficient on the rating scale (RAPM §4.7, §4.10; reconcile-code-first §1.4).
- A focus QB with no listed plays yields an empty frame, `m_focus = NaN`, and every RAPM
  coefficient becomes NaN at the next solve without an error (RAPM §5.4, `NonFinitePrior`).
- Measured iteration behaviour (§7.6): the loop is not converged at `n_iter = 3`. The scope of the
  re-seed (focus QB only, KI-G14), its scale mapping and its convergence criterion are
  DR-D11 (RAPM §4.7).

### 4.7 Mapping to the Kalman observation (SSK §3.1, §4.1)

For player `p` on the timeline of completed game-weeks `w = 0..W−1` (0-based in SSK):

```text
y_w      = c_{p,w}          if a record exists and no_data = false;   NaN otherwise
snaps_w  = n_{p,w}          if a record exists;                       0 otherwise
played_w = (record exists and no_data = false)
R_w      = r_mult_w · r_scale / max(snaps_w, 1)                       (SSK §4.1)
```

- This is the batch path the oracle validates: golden master, Tier 0, calibration and the
  verdict's `smoothed_talent` (`verdict.py:206-234` builds exactly these arrays per
  `(season, week)` slot).
- The missing-week contract is load-bearing: an absent week MUST be `played = false` **and**
  `y = NaN` (SSK §3.2).
- The engine MUST NOT observe the cumulative RAPM coefficient, and MUST NOT default exposure to 1
  (KI-NEW-W3, KI-#24; proposed — DR-C10).
- Layer-1′ maps the same way per role stream, with `n_{p,ρ,w}` as exposure (§4.8.5). Which
  exposure the predictive band uses ex ante is DR-D16 (SSK §9).

### 4.8 Layer-1′: participation-free involvement credit (proposed — DR-C1; open — DR-D15)

Nothing in this subsection exists in the oracle. It is the proposed specification of the live-tier
observation. Every free choice is listed in §4.8.6 and §9. The Rust port MUST NOT promote
Layer-1′ outputs until the statistical owner ratifies the definition.

#### 4.8.1 Roles (from play-by-play, available in season)

On the plays the contract admits (pass and run plays with a down, `nflverse_adapter.py:91-92`;
regular season, proposed — DR-C12):

| Role ρ | Event set for player `p` in week `w` | 2023 counts (weeks ≤ 18; §7.7) |
|---|---|---|
| `dropback` | pass plays with `passer_player_id = p`, sacks included; plus run plays with `qb_scramble = 1` and `rusher_player_id = p` | 19,658 pass plays; all 1,410 sacks carry `passer_player_id`; 1,035 scrambles are `play_type = run` with the QB as rusher and no passer |
| `carry` | run plays with `rusher_player_id = p` and `qb_scramble = 0` | — |
| `target` | pass plays with `receiver_player_id = p` | 17,483 of 19,658 pass plays have a receiver; of the 2,175 without one, 1,410 are sacks |

No 2023 play has both a passer and a rusher. The "sacked QB" of DR-C1 is the `dropback` event of a
sack. Scrambles go to `dropback` because the play began as a dropback. Whether they form their own
role is open (§4.8.6).

#### 4.8.2 Context model `g′` (frozen per data version)

```text
x′_i  = (down_i, ydstogo_i, yardline_100_i, δ^team_i)
g′    = the DR-C7 estimator fitted on all admitted plays of the completed seasons in the window
        (S−3 .. S−1, engine-spec §2.4; proposed — DR-C6), with δ^team computed as in §4.8.3
        on those seasons
r′_i  = dv_i − g′(x′_i)          for every play of season S
```

- `g′` is fitted once at the season boundary, versioned (`context_model_version`), and **not
  refitted in season**. Season-`S` plays are then out of sample by construction, so no in-season
  cross-fitting is needed, and the credit of week `w` cannot depend on later weeks.
- Replaying a completed season in a backtest uses the `g′` that was frozen at that season's start,
  so production and replay agree (two-path equivalence, §6.2).
- Any in-season refit schedule would be a change of this spec, with a cross-fit design under
  DR-D14.

#### 4.8.3 Opponent adjustment

`δ^team_i = E_def^{team}[def_team(i)]` from the live-tier team-level ridge on dV with off/def team
intercepts and no player columns (engine-spec §6.2, Component 5; RAPM §4.5 sign: positive is a
better defence), **as of the start of week `week_i`**. That is, it is estimated from plays of weeks
`< week_i`, so the adjustment is a pre-game quantity. The week-1 value comes from the preseason
state of that ridge (DR-C6). The team ridge's market rows at lock (DR-B5) are part of Component 5,
not of this spec.

#### 4.8.4 Attribution (proposed default)

Each role event receives the **full** play residual `r′_i` in its own role stream. A completed pass
gives `r′_i` to the passer's `dropback` stream and the same `r′_i` to the receiver's `target`
stream. Nothing is split.

Rationale:

- Role streams are separate observations of separate latent states (proposed — DR-C3). A split
  share would be a scale constant that the role-specific Kalman parameters absorb anyway.
- The full-residual rule has no free parameter.
- What it does not do is remove the co-involved player's contribution: a target's credit still
  carries the passer's quality, and a carry's carries the blocking. That is the same estimand
  question as §1.3 item 2 (DR-D13).

#### 4.8.5 Weekly observation and exposure

Two variants are specified. DR-C1's recommended default says "exposure from snap counts". This
spec finds that the two readings of that phrase define different estimands, so it raises the
choice as DR-D15 instead of picking silently.

```text
E(p, ρ, w)      = event set of §4.8.1
(per-event)     c′_{p,ρ,w} = (1 / |E|) · Σ_{i ∈ E} r′_i,      exposure n_{p,ρ,w} = |E|
(per-snap)      c″_{p,ρ,w} = (1 / snaps_{p,w}) · Σ_{i ∈ E} r′_i,  exposure snaps_{p,w}   (snap counts, §3.1)
```

- **Per-event** is efficiency per opportunity. Its precision is set by the event count. Volume
  stays in Layer C, as SSK §1 requires ("Exposure enters only as measurement precision"). Snap
  counts are still needed: for Layer C, and for the `played` flag of each stream (a player active
  with zero targets has `no_data` in the target stream, not a zero).
- **Per-snap** folds the involvement rate (volume) into the efficiency observation. Its sampling
  variance is not `∝ 1/snaps` when events per snap vary, so `R = r_scale / snaps` would be
  misspecified.
- Real 2023 prototype (§7.7, non-parity): the two variants are equivalent for QBs (season-level
  correlation with Layer-1 credit 0.901 per snap, 0.900 per event) and differ for receivers.

Until the decision, the Rust port implements both behind an explicit enum. Neither is promoted.

#### 4.8.6 Open parameters of Layer-1′ (DR-D15)

- Per-event vs per-snap observation (§4.8.5).
- Scrambles as `dropback` (default) or their own role. Treatment of two-point tries, spikes and
  kneels: they are already excluded by the adapter's `play_type` filter. Of the kept 2023 plays,
  484 carry a penalty flag. Whether they stay is open.
- Whether a target on an interception or an incompletion is attributed to the receiver. The
  default is yes, all targets.
- The eligibility rule: none by default; exposure carries the weight.
- The opponent as-of rule (week `w − 1` by default) and the week-1 preseason value.
- Whether `g′` adds features beyond `(s, δ^team)`. The default is no, to stay position-agnostic.

### 4.9 Constants and hyperparameters

Provenance key, as in RAPM §4.9:

- **hand-set v0**: present at `f3b641f`, the oldest commit in the shallow CN clone (2026-06-21),
  with no calibration record;
- **hand-set (PR #N)**: introduced in that PR, with no calibration record;
- **library default**: not set by the oracle, so it is whatever the pinned scikit-learn 1.9.1
  (`reference/python/requirements.lock`) uses.

| Constant | Value | Defined at | Provenance |
|---|---|---|---|
| Context features | `down, ydstogo, yardline_100, D_i` | `layers.py:507-508`; `value.py:25` | hand-set v0 |
| Booster | `HistGradientBoostingRegressor`, squared error | `layers.py:514` | hand-set v0 (DR-C7, DR-C8 open) |
| `max_depth` | 3 | `layers.py:514` | hand-set v0 |
| `learning_rate` | 0.1 | `layers.py:514` | hand-set v0 |
| `max_iter` | 200 (an upper bound; see early stopping) | `layers.py:515` | hand-set v0 |
| `min_samples_leaf` | 150 | `layers.py:515` | hand-set v0 |
| `random_state` | `seed` (default 0) | `layers.py:516` | hand-set v0 |
| `early_stopping` | `'auto'` → on when training rows > 10,000 | sklearn `gradient_boosting.py:534` | library default |
| `validation_fraction`, `n_iter_no_change`, `tol`, `scoring` | 0.1, 10, 1e-7, `'loss'` | sklearn defaults | library default |
| `max_leaf_nodes`, `max_bins`, `l2_regularization` | 31, 255, 0.0 | sklearn defaults | library default |
| Folds | `KFold(n_splits=5, shuffle=True, random_state=seed)`, play-level | `layers.py:512`; `n_splits=5` at `:497, 522, 541, 561`; `verdict.py:129` | hand-set v0 |
| Calibration-gate folds | `GroupKFold(n_splits=5)` by week | `tests/grid/test_calibration_synth.py:70-72` | hand-set (PR #66) |
| `min_plays` | 20 (whole frame) | `layers.py:561`; `verdict.py:129` | hand-set (PR #80) |
| `layer1_all_qbs` threshold | 20 | `layers.py:550` | hand-set (PR #10, Phase 3 Task 4; plan lost, critic G-3) |
| Re-seed statistic | unweighted mean of weekly credit, focus QB only | `layers.py:605` | hand-set v0 |
| Unknown defender rating | 0.0 | `layers.py:493` | hand-set v0 |
| Aggregation key | `week` only | `layers.py:535, 584` | hand-set v0 (defect, §8 item 6) |
| Layer-1′ constants | none fixed | — | to be set by DR-D15 |

### 4.10 Priors (template: Priors)

Layer 1 has no prior and no shrinkage. It is a residual aggregator. Shrinkage of the weekly signal
happens downstream, in the Kalman, through `R = r_scale / exposure` and the prior `x0`/`P0`
(SSK §6.5; `cross-league-priors.md`). Layer 1 MUST NOT apply its own shrinkage. That would
double-count precision in the filter.

### 4.11 Constraints (template: Constraints)

- `r_i` finite for every admitted play. Otherwise `CreditError::NonFiniteResidual`, never clamped.
- `exposure ≥ 1` on every record with `no_data = false`. `credit` is undefined, not 0, when
  exposure is 0.
- The fold map partitions the frame: every play is scored exactly once, by a model that did not
  train on it. In the offseason tier the opponent feature of a scored play MUST come from a source
  that did not train on that play either (§4.3).
- Credit for `(season, week)` in the live tier MUST be invariant to any play of a later week
  (§10.3 T-L1-4).
- Each persisted record carries the context-model, opponent-source and V(s) versions it was
  computed from. A record whose versions differ from the current promoted ones is stale, never
  silently mixed (engine-spec §8.8).

---

## 5. Algorithm, numerics and determinism (template: Seed policy, Tolerances)

### 5.1 Cost

| Frame | plays | Training rows per fold | Booster fits |
|---|---|---|---|
| Canonical legacy synth | 16,825 | 13,460 | 5 per frame; 15 per `fit(n_iter=3)` |
| Canonical fixed synth | 17,752 | 14,201–14,202 | as above |
| Real 2023, weeks ≤ 18 | 33,836 | 27,068–27,069 | 5 |

- Aggregation is linear in plays per player in the oracle (a Python `apply` per player,
  `layers.py:580`). The Rust port SHOULD build the player → plays index once per frame
  (one pass over `O_i`).
- `layer1_all_qbs` refits the context model for each QB: `5 × (number of QBs)` fits. Its two tests
  are among the slowest in the closure (python-closure inventory). It is not ported (§8 item 1).
- Layer-1′ in season costs one prediction pass per completed week and no fit.

### 5.2 Determinism and seed policy

- **Seeds.** One integer `seed` drives three things: the fold shuffle (`layers.py:512`), the
  booster's `random_state` (`:516`), and through it the early-stopping validation split. The
  canonical seed is 0 (parity-fixture-contract §4: "Layer-1 `KFold(random_state=0)` and GBM
  `random_state=0`").
- **Threads.** The oracle is deterministic only single-threaded. `golden_master.py:55-59` records
  that "multi-threaded the gradient-boosted V(s) / Layer-1 credit diverge at ~1e-2". The golden
  runs under `threadpool_limits(1)`. The determinism gate does not pin threads itself; its
  docstring asks for `OMP_NUM_THREADS=1`, and it asserts that two in-process runs agree on
  `qb_credit` to `atol 1e-9` (`tests/grid/test_determinism.py`).
- **Reproduction.** With the lock pins, Linux x86_64 and threads = 1, the live `qb_credit`
  equals the committed golden exactly (max abs diff 0.0; §7.2).
- **Engine rule.**
  - The fold map MUST be a deterministic function of play keys and the seed, for example a hash
    of `(season, week)` for week-grouped folds. It MUST NOT depend on frame row order.
  - The booster MUST be deterministic for a given seed regardless of thread count. Any
    parallelism uses fixed-order reductions (superseded alpha-spec §6.6 rule 5).
  - The seed, the fold map and the booster backend and version enter `context_model_version`.

### 5.3 Sensitivity envelope of the oracle's own cross-fit

Before choosing a Rust tolerance, this spec measured how far the oracle moves under perturbations
that change nothing statistically: the fold seed, the fold scheme, early stopping and the fold
count. Each row compares against the shipped configuration (`KFold(5)`, seed 0). Canonical synth,
`fit(n_iter=3)` ratings, threads = 1, 2,014 offensive player-weeks (§7.8).

| Perturbation | corr(r, r₀) | Weekly credit: corr / RMS Δ / max \|Δ\| | Min per-position season-credit corr | Max \|Δ season credit\| |
|---|---|---|---|---|
| Fixed synth, KFold seeds 1–4 | 0.9966–0.9974 | 0.9982–0.9989 / 0.0158–0.0204 / 0.091–0.119 | 0.9997–0.9998 | 0.0153–0.0224 |
| Fixed synth, `GroupKFold` by week | 0.9973 | 0.9977 / 0.0241 / 0.106 | 0.9994 | 0.0264 |
| Fixed synth, early stopping off (200 iterations) | 0.9961 | 0.9971 / 0.0259 / 0.125 | 0.9994 | 0.0230 |
| Fixed synth, 10 folds | 0.9982 | 0.9992 / 0.0136 / 0.064 | 0.9998 | 0.0143 |
| Legacy synth, KFold seeds 1–4 | 0.9936–0.9941 | 0.9961–0.9965 / 0.0245–0.0257 / 0.130–0.155 | 0.9978–0.9989 | 0.0232–0.0299 |
| Legacy synth, `GroupKFold` by week | 0.9940 | 0.9954 / 0.0279 / 0.148 | 0.9985 | 0.0254 |
| Legacy synth, early stopping off | 0.9976 | 0.9980 / 0.0184 / 0.105 | 0.9967 | 0.0284 |
| Legacy synth, 10 folds | 0.9958 | 0.9974 / 0.0210 / 0.101 | 0.9981 | 0.0261 |

The weekly credit SD is 0.338 (fixed) and 0.290 (legacy). The season credit SD is 0.269 and 0.197.

**Consequence for parity.** The DR-B3 Class C criterion, `corr(dV) ≥ 0.999` plus a grid-cell
bound, was written for V(s). It is not satisfiable for the Layer-1 residual: the oracle against
itself with another seed reaches only 0.9936–0.9974. A Rust booster is a larger perturbation than a
seed change. §10.3 therefore proposes a Layer-1-specific Class C, set below this envelope.

### 5.4 Typed failures (superseded alpha-spec §6.6 rule 4; proposed — DR-B6)

| Condition | Oracle behaviour | Required |
|---|---|---|
| `dv` missing or non-finite | bare `KeyError` / sklearn `ValueError` | `ContractError::MissingField` / `NonFiniteValue` at contract validation |
| State feature non-finite | passed to the booster as a missing value | `ContractError::NonFiniteValue` |
| Participation not `Listed` for a play in an offseason-tier frame | `()` → no credit, silently | `ContractError::ParticipationUnavailable` (plays-contract D-5) |
| A defender id without a rating in the declared source | contributes 0.0 | `CreditError::UncoveredDefender{count, plays}`, unless the DR-D14 policy names an explicit substitute. The substitution count is then reported |
| Fewer plays than folds, or a fold with no training rows | sklearn `ValueError` | `CreditError::InsufficientData` |
| Booster fit or prediction fails | propagates; swallowed in `layer1_all_qbs` (KI-G6) | `CreditError::ContextModelFailed`. Never swallowed |
| A requested player with no qualifying event | omitted, or an empty frame | a `no_data` record. Where a value is required (the fixed-point re-seed), `CreditError::NoEvidence` |
| Non-finite residual or credit | unchecked | `CreditError::NonFiniteResidual` |
| Frame spans seasons without season keys | weeks merged silently | impossible by construction: records are keyed `(season, week)` |
| Layer-1′: snap counts or involvement roles unavailable | n/a | `SourceUnavailable`. The week is not computed. Exposure is never defaulted |
| Context-model or opponent-source version mismatch on read | n/a | `StateError::VersionMismatch` |

Each failure blocks promotion of the run (engine-spec §8.8). These are deliberate divergences,
listed in `reference/python/PARITY.md` (e).

---

## 6. Incremental and online behaviour

### 6.1 Oracle

Layer 1 is **batch-only** in the oracle.

- Every call refits the context model on the frame it receives and persists nothing. It has no
  accumulator, watermark or state file.
- The incremental production path, `weekly_update`, does not compute Layer 1 at all. It feeds the
  cumulative RAPM coefficient to the Kalman instead (§2.2; KI-NEW-W3).
- The verdict computes credit once on the frozen pre-first-origin window and reuses it at every
  origin (`verdict.py:127-191`, PR #91).

### 6.2 Required engine behaviour

1. **Offseason tier (season boundary).**
   - When season `S−1`'s participation is published, recompute Layer 1 for the completed window
     with the §4.3 and §4.4 requirements. Write versioned records keyed `(player, season, week)`.
   - It is an ordinary pipeline stage (engine-spec §8.6), gated by promotion (engine-spec §8.8).
2. **Live tier (each completed game-week).**
   - Compute `r′_i` for that week's plays only, with the frozen `g′` and the as-of opponent
     effect. Append the week's records.
   - A run with no newly completed game-week computes nothing (SSK §6.4 item 5).
3. **Corrections.** A corrected play, a corrected snap count or a corrected V(s) version
   invalidates and recomputes the affected weeks' records. Downstream, the Kalman recomputes from
   the first affected week (SSK §6.4 item 7; KI-A9).
4. **Two-path equivalence.** The weekly stage's records for `(S, w)` MUST equal, exactly, a batch
   recomputation of Layer-1′ for `(S, w)` at the same as-of with the same frozen `g′`
   (`evaluation-and-leakage.md`; engine-spec §8.19). The frozen-`g′` design makes this hold by
   construction: no in-season refit means no path-dependent model.
5. **Window** (proposed — DR-C6). `g′` trains on the three completed seasons before `S`
   (engine-spec §2.4). The offseason-tier Layer-1 frame for season `s` defaults to that season alone, as
   the verdict computes it per season (`verdict.py:179-182`). Pooling the context model over the
   window is part of DR-D14. The opponent source follows §4.3 (a) or (b).

---

## 7. Validation evidence (template: Validation and promotion)

Runs used the `reference/python/requirements.lock` pins (scikit-learn 1.9.1, numpy 2.4.6,
pandas 3.0.6), Python 3.11, Linux x86_64, `OMP/OPENBLAS/MKL_NUM_THREADS = 1` and
`threadpool_limits(1)`. Synthetic runs used `load_synthetic()`, `fit(n_iter=3)` and market seed 1,
as the oracle's gates do.

### 7.1 Oracle tests that pin Layer 1 (all pass at `59bce1d` on Linux)

| Test | What it pins | Status for the port |
|---|---|---|
| `tests/grid/test_attribution.py:113-131` | `layer1_all_players` equals `layer1_qb_weekly` for a QB (same seed and folds) | Port as a Rust internal-consistency test |
| `tests/grid/test_attribution.py:134-161` | Non-QB credit finite, `snaps > 0`; a player below `min_plays` is absent | Port, with `no_data` records instead of absence |
| `tests/grid/test_attribution.py:164-181` | The context model is fitted exactly `n_splits` times for any number of players | Port (the cost property) |
| `tests/grid/test_attribution.py:78-108` | `layer1_all_qbs` returns a dict and skips sparse QBs | **Not ported** (§8 item 1) |
| `tests/grid/test_golden_master.py:168-170` | `qb_credit` against `golden/snapshot.npz` at rtol 1e-5 / atol 1e-6 | Python-to-Python golden only. Legacy, not a Rust target (§7.2) |
| `tests/grid/test_determinism.py` | Two in-process runs agree on `qb_credit` to `atol 1e-9` | Port as a Rust determinism gate |
| `tests/grid/test_tier0_recovery.py:103-128` | Kalman recovery on focus-QB weekly credit | SSK §7.1 |
| `tests/grid/test_calibration_synth.py:107-143` | NIS, PIT and PICP on QB weekly credit from week-grouped folds | SSK §7.2. Fails 1 of 6 on the fixed synth |

No oracle test checks the opponent adjustment's sign. None checks the credit's relation to planted
ability except through the focus QB's Kalman trajectory, and none checks RB/WR/TE credit beyond
finiteness.

### 7.2 Golden `qb_credit`

- Legacy: the live run reproduces the committed 12 values exactly. Weeks 1–7 and 10–14:
  0.0115, 0.1773, 0.3442, 0.5057, 0.5751, 0.6838, 0.7427, −0.0045, 0.4500, 0.5112, 0.7491, 0.7550
  (rounded), with exposures 99, 97, 105, 97, 88, 82, 84, 116, 99, 87, 75, 81.
- `golden/snapshot.npz` stores `qb_week` and `qb_credit` but **not the exposures**. The exposures
  above come from the live run. A Kalman fixture built from the golden must export them from the
  run, not read them from the `.npz` (§8 item 20).
- Fixed synth: the same run gives values that differ from the legacy golden by up to **0.3165**
  (critic G-1: "qb_credit Δ up to 0.317"). Week 4, for example, is 0.1997 against 0.5057. The
  legacy golden is audit-only (DR-B1).

### 7.3 Context model fit

| Quantity | Legacy synth | Fixed synth | Real 2023 (non-parity) |
|---|---|---|---|
| var(dv) | 1.3346 | 1.1340 | dv SD 1.135 (`real_rapm.py` header) |
| Out-of-fold R² of `ĝ` (shipped) | 0.2548 | 0.1343 | 0.0144 |
| R² with state features only | 0.1190 | 0.1008 | −0.0005 |
| R² added by the opponent feature | 0.1358 (void: own defenders) | 0.0335 | 0.0149 (in-sample leakage, next row) |
| R² with fold-wise opponent ratings | 0.2499 | 0.1245 | **−0.0184** |
| SD of `D_i` | 0.2060 | 0.1825 | 0.1179 |
| Mean OOF residual | −1.18e-3 | −9.3e-5 | — |
| `n_iter_` per fold (early stopping on) | 200, 170, 146, 136, 144 | 66, 62, 58, 72, 47 | 25, 98, 61, 48, 83 |

Three observations bear on DR-C7:

- **On real data the state part of `g` does nothing.** dV is already state-centred: an
  out-of-fold R² of −0.0005 is what the martingale property of V(s) predicts. On the synth, dV is
  not state-centred. E[dv | down] is −0.247, −0.108, +0.437, +0.649 for downs 1–4 (legacy) and
  −0.216, −0.042, +0.376, +0.556 (fixed). So the synth exercises the state part of `g` in a way real
  data does not. This spec records the observation and does not diagnose its cause. It is an input
  to `synthetic-world.md` and `value-model.md` (DR-B4).
- **Per-play R² is the wrong yardstick for the opponent term.** Its per-play SD is about 0.05 EP
  (real team-level defensive effect SD 0.0492) against a dV SD of 1.135, so even a correct
  adjustment explains little per-play variance. It still moves weekly credit over 30–60 plays. The
  choice of `g` MUST be made on weekly-credit recovery against a planted opponent effect, and on
  out-of-sample forecasting (DR-C7), not on R².
- A crude linear candidate (ridge on binned state plus `D_i`) gives R² 0.1031 (fixed) and 0.1471
  (legacy), with residual correlation to the GBM of 0.9842 and 0.9456. That is a probe, not a
  candidate evaluation.

### 7.4 Recovery of planted ability, and the unit-credit property

Season credit is the snap-weighted mean of weekly credit, for all 144 offensive synth players (all
pass `min_plays = 20`). "RAPM" is the `fit()` rating.

| Position | Legacy: corr(credit, ability) / corr(RAPM, ability) | Fixed: same | Fixed: SD credit / SD RAPM | Median weekly exposure (fixed) |
|---|---|---|---|---|
| QB (24) | 0.884 / 0.869 | 0.888 / 0.885 | 0.364 / 0.236 | 40 |
| RB (36) | 0.730 / 0.743 | 0.659 / 0.771 | 0.257 / 0.090 | 13 |
| WR (60) | 0.654 / 0.797 | 0.497 / 0.860 | 0.246 / 0.079 | 16 |
| TE (24) | 0.637 / 0.771 | 0.464 / 0.778 | 0.233 / 0.059 | 38 |
| Pooled | 0.748 / 0.818 | 0.641 / 0.846 | — | — |

| Unit-credit evidence (starters) | Legacy | Fixed |
|---|---|---|
| RB: corr(credit, own ability) / corr(credit, team starting-offense ability sum) | 0.613 / 0.864 | 0.620 / 0.967 |
| WR: same | 0.300 / 0.849 | 0.337 / 0.967 |
| TE: same | 0.663 / 0.836 | 0.480 / 0.961 |
| Starter QB vs starter WR weekly credit, median corr over 12 teams (min) | 0.824 (0.537) | 0.870 (0.237) |
| Starters' season-credit SD: within team / overall | 0.054 / 0.130 | 0.043 / 0.245 |

Reading:

- Layer 1 recovers QB ability as well as RAPM does, because the QB dominates the unit.
- For RB/WR/TE it is much worse than RAPM on the fixed synth, and it tracks the team offense.
  The weekly Kalman observation for those positions, and the verdict's RB/WR/TE
  `smoothed_talent`, therefore measure mostly team offense (§1.3 item 2; DR-D13).
- **Season collision**: splitting the canonical frame into two "seasons" of 7 weeks with
  renumbered weeks returns 7 focus-QB rows instead of 12 (§4.5).

### 7.5 Cross-fit design (fixed synth unless stated)

| Change | corr(r, shipped) | Focus-QB weekly credit max \|Δ\| | R² |
|---|---|---|---|
| `GroupKFold` by week | 0.9973 (legacy 0.9940) | 0.046 (legacy 0.045) | 0.1340 (legacy 0.2594) |
| Fold-wise opponent ratings (KI-NEW-A5) | 0.9977 (legacy 0.9912) | 0.024 (legacy 0.101) | 0.1245 (legacy 0.2499) |
| Early stopping off | 0.9961 (legacy 0.9976) | — | — |
| Seeds 1–4 | 0.9966–0.9974 (legacy 0.9936–0.9941) | 0.018–0.022 (legacy 0.016–0.037) | — |

On the synth these design changes are of the same size as a seed change. On real data the
fold-wise correction removes the entire apparent opponent signal (§7.3). Week-grouped folds are
also the condition under which the calibration gate's innovations are genuinely held out
(`test_calibration_synth.py:49-53`).

### 7.6 The fixed point (`fit`, `n_iter = 3`)

| Iteration | Legacy: re-seed `m_focus` / focus rating | Fixed: same |
|---|---|---|
| 0 | 0.364462 / 0.357616 | 0.346223 / 0.306432 |
| 1 | 0.457843 / 0.550419 | 0.356890 / 0.490154 |
| 2 | 0.458421 / 0.599819 | 0.358644 / 0.495814 |

| Change between iterations | Legacy | Fixed |
|---|---|---|
| 1 vs 0: max \|Δ qb_credit\| / max \|Δ rating\| | 0.110 / 0.193 | 0.036 / 0.184 |
| 2 vs 1: same | 0.017 / 0.049 | 0.018 / 0.0057 |

The loop has not converged at 3 iterations. The re-seed moves the focus QB's rating by 0.24
(legacy) and 0.19 (fixed) from iteration 0, which is about one QB rating SD (0.272 and 0.236). That is
"light coupling" for the other players, not for the focus QB (DR-D11).

### 7.7 Real data (2023, weeks ≤ 18; historical, non-parity)

Inputs: the oracle adapter on the pinned nflverse 2023 files (`fetch_realdata.py`), an in-sample
V(s), and an in-sample full-season RAPM as `ratings_lookup`. 33,836 plays, all `REG`, no empty
`off_players`. 528 players have credit.

| Position (≥ 200 exposures) | n | Median weekly exposure | SD season credit | SD RAPM | corr(credit, RAPM) |
|---|---|---|---|---|---|
| QB | 48 | 60 | 0.0900 | 0.0380 | 0.518 |
| RB | 68 | 29 | 0.0931 | 0.0485 | 0.482 |
| WR | 142 | 42 | 0.0912 | 0.0481 | 0.541 |
| TE | 79 | 30 | 0.0880 | 0.0452 | 0.505 |

| Structure | Value |
|---|---|
| Most-used QB's share of team offensive plays, median (min) | 0.857 (0.311) |
| Most-used WR's share, median | 0.817 |
| Weekly credit, most-used QB vs most-used WR of the same team, median corr (31 teams) | **0.909** |
| corr(QB season credit, team mean residual) | **0.974** |
| Context R², in-sample vs fold-wise opponent ratings | 0.0144 vs −0.0184 |

Season credit has the same SD, about 0.09, at every position. That fits a shared team-level signal.

**Layer-1′ prototype.** This tests feasibility only. It uses the per-event and per-snap variants of
§4.8.5 with roles combined per player, and participation play counts stand in for PFR snap
counts. `g′` is fitted in sample on 2023 with a team-only ridge (λ = 1) as `δ^team`; that is
neither frozen nor as-of.

| Position | n | Involvements per exposure (median) | corr(per-snap, Layer 1) | corr(per-event, Layer 1) | corr(per-snap, RAPM) |
|---|---|---|---|---|---|
| QB | 47 | 0.638 | 0.901 | 0.900 | 0.495 |
| RB | 65 | 0.443 | 0.714 | 0.612 | 0.397 |
| WR | 133 | 0.120 | 0.508 | 0.461 | 0.449 |
| TE | 63 | 0.099 | 0.297 | 0.217 | 0.201 |

- `g′` R² −0.0004. corr(`r′`, `r`) = 0.9915.
- QB weekly, at least 20 exposures (578 QB-weeks): corr(per-snap Layer-1′, Layer 1) = 0.763.
- Split-half reliability (odd vs even weeks) of the season value:
  - QB: Layer-1′ 0.578, Layer 1 0.591;
  - WR: Layer-1′ 0.243, Layer 1 0.508.

  Layer 1's higher WR reliability is what a persistent team signal would produce. Stability is not
  validity of an individual estimand.

### 7.8 Provenance of measurements first recorded in this spec

| Measurement | Method |
|---|---|
| §4.2 early-stopping rule and defaults; HGBR baseline | Read from the installed scikit-learn 1.9.1 source; baseline checked numerically against the training mean |
| §5.3, §7.3–§7.6 synth numbers | Instrumented copy of `fit()` (the same `run_rapm` and `layer1_qb_weekly` calls), oracle `_cross_fitted_context_residual` (the probe's own cross-fit matched it with max abs diff 0.0), and variants that change one factor each. Legacy and fixed in separate processes. The fold-wise variant refits `run_rapm` on each fold's training plays with the market rows and without the focus-QB re-seed prior |
| §7.3 real fold-wise row | `run_rapm` refitted on each fold's training plays, no market rows (as the shipped real-data fit) |
| §7.7 real-data numbers | The oracle adapter as in `tools/investigations/real_rapm.py`; Layer 1 via `layer1_all_players`; involvement fields joined from pbp on `(game_id, play_id)` (33,836 of 33,836 matched) |
| §10.4 aggregation example | The oracle's `layer1_all_players` and `layer1_qb_weekly` with `_cross_fitted_context_residual` replaced by the injected residual vector |

The probe scripts are held in the consolidation session scratchpad and are handed over for commit
under `reference/python/tools/investigations/` (critic G-2 precedent). Until they are committed,
these numbers are *recorded, not reproducible from the repo*.

---

## 8. Known defects and required engine behaviour (template: Known limitations)

The Rust port MUST NOT reproduce any item in this table. Each correction lands either as an
approved oracle correction in the `reference/python/PARITY.md` ledger (proposed — DR-B1) or as a
recorded deliberate divergence (proposed — DR-B6).

| # | Defect (oracle) | KI / source | Required behaviour |
|---|---|---|---|
| 1 | `layer1_all_qbs` swallows any exception with `except Exception: pass` (`layers.py:555-556`) and refits the context model per QB | KI-G6 (alias KI-#27) | **Not ported** (decision-register "Defaults applied" item 4). Any multi-player call fits once and raises `CreditError::ContextModelFailed` |
| 2 | Opponent feature from ratings fitted on all plays, held-out fold included. On real 2023 the in-sample feature's entire apparent signal is leakage (R² 0.0144 → −0.0184) | KI-NEW-A5 (this spec adds the real-data measurement) | Fold-wise or frozen ratings (§4.3; DR-D14) |
| 3 | Play-level shuffled folds in production; week-grouped only in the calibration gate | KI-NEW-A5; reconcile-code-first C18 | Week-grouped folds keyed by play identity, with the fold map persisted (§4.4, §5.2) |
| 4 | Opponent feature `D_i` omits `γ_def`, so it is gauge-dependent | KI-NEW-Z5; RAPM §4.6; new | `δ_i = γ_def[def_team(i)] + Σ β_p` (§4.3) |
| 5 | A defender without a rating contributes 0.0 silently | KI-NEW-Z4; new | Total coverage or a declared, counted substitute (§5.4) |
| 6 | Aggregation by `week` only: multi-season frames merge seasons silently (7 rows instead of 12 in §7.4) | KI-NEW-Z2; new (the verdict works around it, `verdict.py:176-182`) | Key every record by `(season, week)` |
| 7 | Eligibility `N_p ≥ min_plays` counted over the whole frame; ineligible players are absent rather than flagged | KI-NEW-Z6; new | As-of eligibility; `no_data` records (§3.2, §4.5) |
| 8 | Batch credit for week `w` depends on later weeks through `ĝ` and the ratings. The synth Kalman calibration and Tier-0 gates are therefore not causal one-step tests | KI-NEW-Z7; new | Offseason or retrospective use only. The live tier uses frozen `g′` and as-of `δ^team` (§4.8, §6.2). Calibration gates that claim one-step semantics use causal credit (SSK §7) |
| 9 | Early stopping enabled implicitly by sklearn's `n_samples > 10,000` rule; `n_iter_` varies 25–200 between folds and the algorithm changes at 10,000 rows | KI-NEW-Z3; new | The booster configuration is explicit and complete: either a fixed iteration count or an explicitly declared early-stopping rule, with `n_iter` recorded per fold in the model version |
| 10 | Credit is an on-field-unit plus-minus. For RB/WR/TE it mostly measures the team offense (§7.4, §7.7), yet the verdict uses it as player `smoothed_talent` | KI-NEW-Z1; new | Explicit estimand decision (DR-D13). Until then, outputs carry `role = on_field` and explanations follow §3.3 |
| 11 | `weekly_update` feeds the Kalman cumulative RAPM with `snaps = 1` | KI-NEW-W3, KI-#24 (critic X-13) | Weekly credit with real exposure; NaN and `played = false` for no evidence (§4.7; proposed — DR-C10) |
| 12 | Fixed-point re-seed: focus QB only, unweighted mean of weekly means, credit scale into a rating prior, NaN prior when the QB has no plays | KI-NEW-Z8; KI-G14; RAPM §5.4 | DR-D11; `CreditError::NoEvidence` / `ContractError::NonFinitePrior` |
| 13 | Legacy synth: the opponent feature sums the offense's own defenders | KI-NEW-Y0 (critic G-1) | No legacy Layer-1 value is a Rust target. Parity uses `-corrected` cases (DR-B1) |
| 14 | Postseason plays enter the frame | KI-NEW-V0a | Season-type policy of the plays contract (proposed — DR-C12) |
| 15 | Same-season participation used at every walk-forward origin and in the verdict's pre-window | KI-NEW-Z68; reconcile-code-first C1; DR-C1 | Publication-lag gate; `research_only` labelling (§1.4) |
| 16 | `dv` and state not validated (`KeyError`, sklearn `ValueError`, NaN features accepted) | KI-NEW-Z4; KI-G12 (RAPM side); new | Typed contract errors (§5.4) |
| 17 | Synthetic dV is not state-centred (E[dv \| down = 4] = +0.649 legacy, +0.556 fixed); real dV is (state R² −0.0005). Synth gates over-exercise the state part of `g` | KI-NEW-Z32; new | Input to `synthetic-world.md` and `value-model.md` (DR-B4); no action in this component |
| 18 | `sv_to_points` documents a credit→points map, but the verdict fits it on RAPM ratings in a column named `credit` | KI-NEW-Z59; KI-NEW-R3 | "Credit" denotes Layer-1/1′ only. The map is a projection-stack question (DR-C4, DR-C5) |
| 19 | `fit(verbose=True)`, the default, needs the synth-only `ability` column (`layers.py:606-608`) | KI-NEW-Z9; RAPM §4.7 | No dependence on synth-only columns |
| 20 | `golden/snapshot.npz` lacks the exposures of `qb_credit` | KI-NEW-Z76; new | Layer-1 and Kalman fixtures export exposures from the run (§10.2) |
| 21 | The plays contract has no involvement roles, and snap counts have no provider contract, PFR→GSIS crosswalk or licence ruling | KI-NEW-Z50; new; `docs/04-providers/nflverse/README.md:76`; `access-and-license.md:40` | Contract MINOR addition and provider contract before Layer-1′ (P1-03); licence ruling by the Data/Licensing owner |

---

## 9. Open decisions

| Decision | Question | Proposed default |
|---|---|---|
| **DR-C1** | Participation dependence on the live path | Two tiers: Layer 1 offseason only; Layer-1′ in season (§1.4) (proposed — DR-C1) |
| **DR-C7** | Context-model estimator: cross-fitted GBM vs ridge/GAM | Decided on recovery evidence (proposed — DR-C7). This spec adds: decide on weekly-credit recovery against a planted opponent effect and on out-of-sample forecasting, not per-play R² (§7.3) |
| **DR-C8** | Booster backend | Pure-Rust first (proposed — DR-C8). The backend and version enter `context_model_version` |
| **DR-C10** | Kalman observation | Weekly credit with real exposure (proposed — DR-C10); §4.7 |
| **DR-C3** | Role-specific talent in Layer D | Per-role Layer-1′ streams (dropback, carry, target) feed role talent (proposed — DR-C3) |
| **DR-C6** | Window | `g′` trains on the three completed seasons before `S` (§6.2 item 5) |
| **DR-B3** | Parity tolerances | Classes A / A′ / B / C / D (proposed — DR-B3), plus the Layer-1 Class C amendment of §10.3 |
| **DR-B4** | Synthetic world | Fixed defenders, then the stat-vector world that plants passer, rusher and target roles and an opponent effect, so Layer-1′ and the DR-C7 choice can be tested |
| **DR-B6** | Typed failures | §5.4 (proposed — DR-B6) |
| **DR-D14** | Fold scheme, opponent-rating source (fold-wise vs frozen) and its coverage policy, early-stopping control, fold count, seed policy, and the offseason frame (single season vs window pooling) | Week-grouped folds, fold-wise ratings, early stopping explicit, `K = 5`, seed 0 recorded, single-season frame. Same slug as engine-spec §6.2 Component 6 |
| **DR-D13** | Is Layer-1 (and Layer-1′) credit an on-field-unit quantity or a teammate-adjusted individual one? Candidates: keep unit credit and label it; subtract co-involved players' offseason ratings, e.g. `r_i − Σ_{q ∈ O_i, q ≠ p} β_q`; or restrict to Layer-1′ role events | **Open, no default.** The evidence is §7.4 and §7.7. It must be decided before RB/WR/TE credit is promoted as an individual signal |
| **DR-D15** | Layer-1′ observation (per event vs per snap), role set, scrambles, penalties, attribution, opponent as-of rule, `g′` features and refit schedule | §4.8 defaults: per-role full-residual attribution, frozen `g′`, opponent as of week `w − 1`. The per-event vs per-snap choice is **open** (it reads DR-C1's "exposure from snap counts" two ways) |
| **DR-D11** | Re-seed set, credit→rating scale mapping, convergence criterion | Open (RAPM §4.7). Parity port of `fit` as is until decided. engine-spec §6.2 Component 7 calls the same decision "DR-D11" |
| **DR-D16** | Ex-ante exposure for the published predictive band | Open (SSK §9) |
| **DR-D19** | The prior on the Kalman credit scale | Open (SSK §9); §1.3 item 3 |

---

## 10. Rust port plan (template: Tolerances, Reference examples)

### 10.1 Target crates and work packages (engine-spec §8.1, §9.5; DR-A8 adopted subject to ratification)

| Piece | Crate::module | WP |
|---|---|---|
| `Regressor` trait and the booster used by the context model (deterministic, explicit configuration) | `models::boosting` (DR-C7, DR-C8) | **P1-06** (numerical primitives) |
| Context residual (cross-fit, fold map, opponent feature), weekly aggregation, eligibility, typed errors | `models::credit` (uses `domain` plays types and the `models::rapm` outputs) | **P1-12** (GRID component port) |
| Fixed-point coupling with RAPM (`fit`) | `models::rapm` + `models::credit` | P1-12 |
| Layer-1′: roles, frozen `g′`, as-of team opponent effect, per-role streams | `models::credit` (roles from `domain`) | P1-12, after DR-D15 |
| Involvement roles in the plays contract; snap-count provider; PFR→GSIS crosswalk | `domain`, `ingestion`, `identity` | **P1-03** (nflverse ingestion); identity per engine-spec §4.4 |
| `layer1_credit` records, versions, invalidation on correction | `persistence`; materialized as features by `features` (engine-spec §11.6) | P1-02 (schema), **P1-05** (feature store, as-of and publication-lag gate) |
| Leakage tests (future poisoning, participation lag), two-path equivalence | `evaluation` + `features` | **P1-05** |
| Walk-forward with per-origin frozen context models | `evaluation` | **P1-09** |
| Per-role Kalman streams; correction replay | `models::statespace` | P1-12; **P2-03** (advanced state updates) |
| Role talent into Layer D | `models` (projection stack) | **P1-07** (DR-C3) |
| Planted roles and opponent effect in the synthetic world | `synth` (DR-A8) | P1-12 for the corrected world; the stat-vector world that plants roles before the Layer A–F gates (proposed — DR-B4) |
| Weekly Layer-1′ stage | `pipeline` and `grid-cli` (DR-A8) | P2-01 (operation) |

### 10.2 Fixtures (`docs/03-contracts/parity-fixture-contract.md`)

Fixtures are synthetic only (DR-A11), exported single-threaded and sha256-manifested
(proposed — DR-B2), under `fixtures/parity/layer1-credit/`. Each case has `-legacy` and
`-corrected` variants where the ledger touches it (DR-B1).

- The canonical plays and `dv`, referenced from `synthetic-world/` and `value-model/` via
  `inputs_from`.
- The ratings lookup used for the opponent feature, and the per-play `D_i`.
- The fold map `k(i)` as an integer array (KFold seed 0). Also the week-grouped variant once
  DR-D14 is ratified.
- The out-of-fold residual `r` (`_resid`) per configuration.
- Weekly credit, exposure and week sets for all offensive players (`layer1_all_players`) and for
  the focus QB (`layer1_qb_weekly`).
- For `fit(n_iter=3)`: the residual vector, `m_focus`, ratings and `qb_weekly` **for each
  iteration** (parity-fixture-contract §5: the defender-rating feature changes per iteration).
- The exposures of the golden `qb_credit` (§8 item 20).
- Manifest `params` record `n_splits`, `seed`, `min_plays` and the booster's full effective
  configuration, including `early_stopping='auto'` and the observed `n_iter_` per fold.

### 10.3 Parity targets

| ID | Target | Class | Criterion |
|---|---|---|---|
| P-L1-1 | Membership sets `P(p,w)`, exposures `n_{p,w}`, week sets, eligibility under the oracle rule | exact | Integer and set equality (duplicates counted once) |
| P-L1-2 | Weekly credit `c_{p,w}` from the **injected** residual vector | A | ≤ 1e-12 abs (parity-fixture-contract §6: "credit means") |
| P-L1-3 | Opponent feature `D_i` from the injected ratings lookup; `δ_i` from injected `β`, `γ` once the corrected oracle provides it | A | ≤ 1e-12 abs |
| P-L1-4 | Context-model residual from the Rust booster, with injected `x_i`, `dv` and **injected fold map** | C-L1 (below) | Rust-native vs oracle |
| P-L1-5 | `fit(n_iter=3)` with injected per-iteration residuals: ratings, team values, `qb_weekly`, `m_focus` | A′ (ratings, ≤ 1e-9 rel); A (`qb_weekly`, `m_focus`) | Stage-isolated, as parity-fixture-contract §5 |
| P-L1-6 | Kalman on weekly credit | — | Owned by SSK §10.3 (PF-SS-02); requires the exported exposures |
| P-L1-7 | End to end with the Rust V(s), the Rust booster and the Rust corrected synth: Kalman Tier-0 floors and calibration bands on credit | D | Floors re-set on the fixed synth, calibrated below observed (proposed — DR-B3, DR-B4). Not set by this spec |
| P-L1-8 | Divergences: `(season, week)` keying on a two-season frame; `no_data` records; typed failures of §5.4; no `layer1_all_qbs` | divergence | Listed in `reference/python/PARITY.md` (e). Rust asserts the typed outcome |
| P-L1-9 | Layer-1′ | spec golden (A) + D | No oracle counterpart. §10.4 examples are Class A goldens once DR-D15 is ratified. Recovery on the stat-vector synth (DR-B4) is Class D |

**Layer-1 Class C (C-L1): proposed amendment to DR-B3.** The DR-B3 Class C text targets V(s) and
fails for the oracle against itself on Layer 1 (§5.3). Proposed criterion: on the canonical
**fixed** synth, with injected `x`, `dv` and fold map, the Rust context model passes all four of:

1. `corr(r_Rust, r_oracle) ≥ 0.99`. The oracle's self-perturbation minimum is 0.9961 (fixed) and
   0.9936 (legacy).
2. Weekly credit over all offensive player-weeks: corr ≥ 0.99 **and** RMS Δ ≤ 0.05 EP per play.
   The envelope is ≥ 0.9971 and ≤ 0.0259 (fixed); ≥ 0.9954 and ≤ 0.0279 (legacy). 0.05 is about
   15% of the weekly credit SD.
3. Per-position season-credit correlation ≥ 0.995. The envelope minimum is 0.9994 (fixed) and
   0.9967 (legacy).
4. The P-L1-7 Class D gates hold.

These thresholds sit below every observed oracle self-perturbation, in the calibrate-below-observed
manner of the oracle's own gates. They MUST be pre-registered by the statistical owner before any
Rust result is seen (superseded alpha-spec §12.7). A Rust booster that fails them is a decision
request, not a reason to loosen them.

**Truth-anchored and property tests (Rust-native; new).**

- **T-L1-1 unit identity.** Two players with identical on-field play sets receive identical credit
  and exposure, exactly. This documents the estimand while DR-D13 is open.
- **T-L1-2 opponent sign.** On the fixed synth, the fitted context model's partial dependence on
  the opponent feature is non-increasing on average, and corr(`δ_i`, state-only residual) < 0.
  The observed oracle value with `D_i` is −0.170.
- **T-L1-3 centring.** \|mean OOF residual\| ≤ 0.01 EP per play on the canonical synth. Observed
  −1.2e-3 (legacy) and −9.3e-5 (fixed).
- **T-L1-4 as-of causality (live tier).** Layer-1′ records for `(S, w)` are bit-identical when
  plays of weeks > `w` are added, removed or altered (future poisoning; P1-05 harness).
- **T-L1-5 season keying.** A two-season frame with overlapping week numbers yields distinct
  `(season, week)` records whose values equal the per-season computation.
- **T-L1-6 cross-fit hygiene.** No play is scored by a model, or by an opponent-rating source,
  that trained on it (§4.11). This is checked by fold-membership audit on the persisted fold map.

### 10.4 Reference examples (become golden unit tests)

**E-1. Weekly aggregation with injected residuals.** Verified by running the oracle's
`layer1_all_players(min_plays=2)` and `layer1_qb_weekly` with the residual vector injected.

| Play | week | `off_players` | `def_players` | injected `r_i` |
|---|---|---|---|---|
| 0 | 1 | (q1, w1) | (d1, d2) | 0.5 |
| 1 | 1 | (q1, w2) | (d1) | −0.1 |
| 2 | 1 | (q1, w1, w1) | (d1, zz) | 0.3 |
| 3 | 2 | (q1, w1) | (d2) | −0.4 |
| 4 | 2 | (q2, w1) | (d2) | 0.2 |
| 5 | 2 | (q1, w2) | (d1) | 0.1 |
| 6 | 3 | (q1) | (d1) | 0.0 |

```text
q1: week 1 credit (0.5 − 0.1 + 0.3)/3 = 0.233333333333333, snaps 3
    week 2 credit (−0.4 + 0.1)/2     = −0.15,             snaps 2
    week 3 credit 0.0,                                    snaps 1
w1: week 1 (0.5 + 0.3)/2 = 0.4, snaps 2   (play 2 lists w1 twice: counted once)
    week 2 (−0.4 + 0.2)/2 = −0.1, snaps 2
w2: week 1 −0.1, snaps 1;  week 2 0.1, snaps 1      (N = 2 ≥ min_plays = 2: eligible)
q2: absent (N = 1 < 2)     d1: absent (never in off_players)
fit() re-seed for q1: (0.233333 − 0.15 + 0.0)/3 = 0.027777777777778   (unweighted, oracle)
            snap-weighted alternative: (0.7 − 0.3 + 0.0)/6 = 0.066666666666667
an absent focus QB: empty frame, mean = NaN  (→ CreditError::NoEvidence in Rust)
```

Rust required form of the same example: `q2` and `d1` produce `no_data` records instead of being
absent, and every record is keyed `(season, week)`.

**E-2. Opponent feature.** Ratings `{d1: 0.2, d2: −0.1, q1: 0.05}`, same plays:

```text
D = [0.1, 0.2, 0.2, −0.1, −0.1, 0.2, 0.2]       (oracle; play 2's unknown "zz" contributes 0.0)
```

Required: play 2 raises `CreditError::UncoveredDefender{count: 1, plays: [2]}` unless a substitute
policy is declared (§4.3).

**E-3. Gauge invariance of `δ` (required form).** One team `B` defends every play, with `k_def = 2`
listed defenders `b1`, `b2`. With `β_b1 = 0.10`, `β_b2 = 0.05` and `γ_def[B] = 0.20`,
`δ = 0.35` on every play. The RAPM-equivalent shift `β_b1, β_b2 += 0.03`, `γ_def[B] −= 0.06` leaves
`δ = 0.35`, while the oracle's `D` moves from 0.15 to 0.21.

**E-4. Layer-1′ attribution (proposed default, §4.8.4–§4.8.5).** Week `w`, team A on offense.
Frozen `g′` gives the residuals below.

| Play | Type | passer | rusher | receiver | scramble | `r′_i` |
|---|---|---|---|---|---|---|
| 1 | pass, complete | QB_A | — | WR_A | 0 | 0.6 |
| 2 | pass, sack | QB_A | — | — | 0 | −1.2 |
| 3 | run | — | RB_A | — | 0 | 0.1 |
| 4 | run, scramble | — | QB_A | — | 1 | 0.4 |
| 5 | pass, incomplete | QB_A | — | TE_A | 0 | −0.5 |

```text
QB_A dropback: events {1, 2, 4, 5}; per-event credit (0.6 − 1.2 + 0.4 − 0.5)/4 = −0.175, exposure 4
WR_A target:   {1};  credit 0.6,  exposure 1
TE_A target:   {5};  credit −0.5, exposure 1
RB_A carry:    {3};  credit 0.1,  exposure 1
per-snap variant with QB_A snaps = 5: (0.6 − 1.2 + 0.4 − 0.5)/5 = −0.14, exposure 5
```

This example becomes a golden only after DR-D15 is ratified.

---

## 11. Superseded-spec mapping

| Superseded section | Requirement | How this spec satisfies it | Gap |
|---|---|---|---|
| alpha-spec §1.2 | RAPM-style models must not become a hidden dependency of the live path | §1.4: participation Layer 1 is offseason only; Layer-1′ reads no participation (proposed — DR-C1) | Ratification of DR-C1 |
| alpha-spec §4.1 (Participation row), Appendix A | Participation not for in-season use | §3.1 as-of rule; §8 item 15; `research_only` | The provider contract must carry the publication timestamp (P1-03, P1-05) |
| alpha-spec §6.1 Layer D | Player efficiency rates | Layer-1′ role streams feed role-specific talent (proposed — DR-C3) | Layer-D models are `projection-stack.md` |
| alpha-spec §6.1 Layer E | Opponent adjustment | §4.3 and §4.8.3: opponent strength enters as context | The estimand of the adjustment depends on DR-B5 |
| alpha-spec §6.2 (Gradient boosting row) | Boosting for "nonlinear residual correction, interaction effects, availability/workload models, and component-rate models" | The Layer-1 context model is a fourth role, a nuisance model for a residual. Kept only as a DR-C7 candidate, and it must earn its place: "Each component must prove incremental out-of-sample value" | DR-C7 |
| alpha-spec §6.2 (Kalman row) | Online state for "selected efficiency components" | §4.7: credit is that efficiency observation | — |
| alpha-spec §6.6 | Model-spec contents: target, as-of inputs, equations, constraints, split rules, seeds, tolerances, complexity, examples, training/serving parity, explanation fields | §1–§5, §3.3, §5.1, §6.2 item 4, §10.4 | The complexity statement covers the declared synthetic and 2023 sizes only |
| alpha-spec §6.6 rule 1 | "implement the documented formula, not substitute a superficially similar library API" | §4.2 states every effective booster setting, including the library defaults the oracle inherited | — |
| alpha-spec §6.6 rule 4 | Typed failures | §5.4 | — |
| alpha-spec §6.6 rule 5 | Seeded, deterministic under parallelism | §5.2 | — |
| alpha-spec §12.3 | "current-season participation unavailable at serve time must not appear in promoted live features" | §1.4; T-L1-4 | The test lives in `evaluation-and-leakage.md` (P1-05) |
| alpha-spec §12.7 | Thresholds versioned before results are seen | §10.3 C-L1 floors to be pre-registered | — |
| final-build-spec §11.8 | `IncrementalBooster` with continuation | **Not applicable.** A cross-fitted nuisance model is refitted whole. The live-tier `g′` is frozen per season, which is the §12.2 "periodic rebuild" mode | The trait shape is DR-C8 |
| final-build-spec §12.1 | Daily DAG with parallel "Kalman update" and "RAPM incremental" | **Replaced** (critic X-13): V(s) → dV → Layer-1′ → Kalman in season; RAPM and Layer 1 at the season boundary (§6.2) | — |
| final-build-spec §12.2 | Continuation, replay-window and periodic-rebuild training modes | Periodic rebuild only, at season boundaries (§4.8.2, §6.2) | — |
| final-build-spec §12.3 | Snapshot and rollback of model state | `context_model_version` and versioned `layer1_credit` records (§3.2) | The snapshot list in engine-spec §8 must include the context models |
| final-build-spec inventory §0.3, D1 | "the Kalman filter consumes the weekly RAPM / Layer-1 output" | **Corrected** (critic X-13): the Kalman consumes weekly Layer-1/1′ credit, never RAPM; Layer 1 is itself participation-dependent, hence Layer-1′ | — |
