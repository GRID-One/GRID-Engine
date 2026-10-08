---
model-spec-id: MS-CROSS-LEAGUE-PRIORS
status: Draft            # equations below are NOT yet approved by the statistical owner
statistical-owner: repository owner (holds every engine-spec §1.4 role today)
version: 0.1.0
supersedes: none (first engine model spec for this component)
---

# Model spec — Cross-league (feeder → NFL) priors

Consolidation work package **P0-01** (ADR-011 "Engine-only pivot", ADR-012 "Python reference
oracle"). This is a contract-level model spec (engine-spec §1.5). Its equations bind the Rust
implementation once the statistical owner approves them (superseded alpha-spec §6.6).

**How to read this spec.**

- **Oracle behaviour.** Sections 3–7 describe the Python reference oracle exactly as implemented at
  cautious-nevermore (CN) `59bce1d`. Code is cited as `reference/python/<path>:<line>`; the line
  numbers equal CN `59bce1d` (`reference/python/MANIFEST.tsv`: `backend/grid/priors.py` verbatim).
  The oracle is executable evidence, not authority (engine-spec §1.7).
- **Engine requirements** use MUST/SHOULD/MAY. Differences from the oracle are deliberate divergences
  (§8; `reference/python/PARITY.md`).
- **Decision tags.** "(proposed — DR-xx)" marks a default that waits on the owner
  (`docs/00-meta/decision-register.md`). `KI-` IDs are in `docs/00-meta/known-issues.md`.
- **Inventory reports.** "critic", "cn-issues", "cn-docs", "reconcile-code-first" and
  "reconcile-spec-first" are the consolidation inventory, kept verbatim in
  `docs/06-sessions/2026-10-01-consolidation-inventory/`.
- **Companion spec.** The Kalman layer these priors seed is `state-space-kalman.md` (cited as
  "SSK §x").

## 0. Template conformance

| Template heading | Where it is covered |
|---|---|
| Target | §3.1 |
| Inputs | §3.2, §3.3 |
| Equations | §4.1–§4.4 (oracle), §6.2 (engine form) |
| Priors | the whole document |
| Constraints | §4.6, §8.3 |
| Seed policy | §5.2 |
| Tolerances | §10.3 |
| Reference examples | §10.4 |
| Explanation fields | §3.5 |
| Validation and promotion | §7, §10.3 |
| Known limitations | §8, §9 |

---

## 1. Purpose and statistical intent

The component gives a player with little or no NFL evidence a **prior on the NFL talent scale**, built
from his production in a feeder league. The oracle docstring states two jobs
(`reference/python/backend/grid/priors.py:1-17`):

1. **Estimate the feeder→NFL equivalency from shared players**, the players observed in both a feeder
   league and the NFL, by regressing NFL rating on feeder situational value (SV). This is "the same
   shared-unit identification as the opponent/teammate fixed point: shared units link otherwise-incomparable
   pools".
2. **Translate a prospect's feeder SV into an NFL-scale prior (mean, variance)** for the state-space
   layer. "The variance is POSITION-SPECIFIC: wide where college predicts the NFL poorly (QB), tight
   where it translates cleanly. A wide prior → high early Kalman gain → the prior washes out fast."

**What it may estimate.** A per-league affine map `E[NFL talent | feeder SV]` and a position-specific
prior variance expressing translation fidelity. The docstring intends one slope and intercept per
feeder league and NCAA tier (`priors.py:16-17`). The oracle implements a single pooled feeder league.

**What it does not estimate, and must say so.**

- **Selection (range restriction).** The map is identified only from players good enough to reach the
  NFL. The oracle "report[s] out-of-sample predictive fit honestly" (`priors.py:9-10`) but does not
  correct the bias. The engine MUST report the shared-player population the map was fit on, and MUST
  NOT present the map as valid outside that population's feeder-SV range without an approved
  correction.
- **Volume or role.** This is an **efficiency** prior. Alpha-spec §6.3 (superseded): "strong college efficiency
  does not guarantee NFL volume". Role priors belong to the Layer-C spec.
- **College fantasy points** are never inserted into an NFL projection (alpha-spec §4.2.3 (superseded)).

**Status.** The component is **synthetic-only**. Verified at `59bce1d`:

- No real feeder-SV source exists. `load_cfbd` raises `NotImplementedError`
  (`reference/python/backend/grid/data_adapters.py:68-74`), and the `college` frame comes only from the
  synthetic `make_college` (`reference/python/backend/grid/synth.py:297-317`).
- `estimate_equivalency` and `build_priors` are called only inside `backend/grid` (exports and the
  `__main__` self-test), by `run_demo.py:82-83`, and by tests.
- The projection layer accepts a `priors_df` (`reference/python/backend/projection/features.py:116-145`),
  but no production caller passes one. The verdict hard-codes `prior_mean = 0.0`
  (`reference/python/backend/validation/verdict.py:280`).
- PR #82 only made `estimate_equivalency` tolerate a missing `ability` column (`priors.py:83-99`).

CN's documented claim that priors are "wired to real data" is false (cn-issues §1, KI-NEW-P2).

## 2. Position in the engine

| Aspect | Content |
|---|---|
| Engine-spec section | §6.5 NCAA / rookie prior. Also §6.4 (empirical Bayes; affine mapping) and §6.1 Layer D (efficiency) |
| Eligibility | Superseded alpha-spec §2.5 (low-evidence definition: `years_exp ≤ 2`, below a position opportunity threshold, position change, undrafted or late-added). The oracle has only a static `is_rookie` flag |
| Upstream | NCAA data: `docs/04-providers/cfbd/` (CFBD; not implemented). Identity linking (engine-spec §4.4; alpha-spec §4.4.3 (superseded) tiers). A feeder SV pass ("reduced Layer-1/Layer-2 machinery within college", `data_adapters.py:68-72`) built on `value-model.md`, `layer1-credit.md`, `rapm-attribution.md`. The NFL rating target: RAPM season rating or Kalman end-of-season smoothed talent (`priors.py:83-90`) |
| Downstream | SSK §6.5 (`x0`/`P0`; **not wired in the oracle**, KI-#15). `projection-stack.md` (`prior_mean`, `prior_var` features; NaN means "no prior", `features.py:52-57`). Explainability "NCAA prior contribution" (engine-spec §6.7) |
| Contracts | `docs/03-contracts/engine-output-contract.md` (prior fields); `docs/03-contracts/parity-fixture-contract.md` |

**Scope.** Engine-spec §2 (superseded alpha-spec §2.1) covers QB/RB/WR/TE. IDP is out of scope, so the oracle's
`DEF` prior SD (`priors.py:26`) is not ported. Feeder leagues: **NCAA only** (proposed — DR-C9).
The UFL/USFL/XFL/CFL entries are documented here but unused.

## 3. Inputs and outputs

### 3.1 Target

Two targets:

- `prior_mean_i`: the expected NFL talent of player i on the scale of the regression target.
- `prior_var_i`: its variance.

Units: dV per play on the NFL **rating** scale. In the oracle that is the RAPM coefficient (`run_rapm`/`fit` `rating`).

This is **not** the scale of the Kalman state, which is weekly Layer-1 credit (SSK §3.1, §6.5). On
the legacy synth the per-position slope of season credit on RAPM rating is 1.19–1.40 (SSK §7.3).

### 3.2 Inputs

**`college_df`.** The data-adapter contract (`data_adapters.py:15`):

| Column | Type | Meaning | Null semantics (oracle) |
|---|---|---|---|
| `player_id` | id | join key (synth: int) | — |
| `position` | str | QB/RB/WR/TE (DEF in synth) | Unknown position → `prior_sd = 0.10` silently (`priors.py:148`) |
| `feeder_sv` | float | feeder-league situational value, dV/play in feeder units | NaN propagates to `prior_mean` |
| `feeder_snaps` | int | feeder exposure | **unused** by the oracle |
| `is_rookie` | bool | excluded from the equivalency fit; receives a prior | Must be bool: `~` is applied (`:98`) |
| age column (optional, name passed as `age_col`) | int | age | NaN gives 0 adjustment (comparisons are false). A named column that is absent is silently skipped (`:144`) |
| draft-round column (optional, `draft_round_col`) | int or None | 0 = undrafted | None or NaN gives 0. A missing column is silently skipped (`:146`) |

**`nfl_ratings`.** `player_id`, `rating`, and optional `ability` (synth only, used for `factor_check`)
(`priors.py:96-99`).

**`league_type`.** One of FBS/FCS/UFL/USFL/XFL/CFL/unknown. Any other string gets the `unknown` factor
silently (`:117`).

**Synthetic generator** (`make_college`, `synth.py:297-317`):

- `feeder_sv = league_factor·ability + N(0, college_noise_sd²)`;
- `feeder_snaps ~ U{250, …, 899}`;
- `is_rookie ~ Bernoulli(frac_rookies)`;
- RNG `default_rng(cfg.seed + 99)`.

Canonical values: `league_factor = 0.62`, `college_noise_sd = 0.030`, `frac_rookies = 0.4`, seed 7. That gives
288 rows (every synthetic player, DEF included), 105 rookies and 183 shared players.

### 3.3 As-of and publication constraints

The oracle has **no as-of handling**. The equivalency is a global fit across whatever shared players are
passed. Required engine behaviour:

- **Fit window.** The equivalency used for a forecast in season S MUST be fit only on NFL ratings that
  existed as of the forecast: completed seasons inside the engine-spec §2.4 window. Its feeder inputs
  MUST carry CFBD retrieval timestamps (engine-spec §4.5). This follows the cn-docs §3.7 scope-leak
  rule "equivalency frozen to prior seasons".
- **Static metadata.** A prospect's own feeder SV, draft round and age are static metadata and may predate the window
  (alpha-spec §2.4 (superseded)). The **map** applied to them may not.
- **Identity links.** An identity link that is ambiguous MUST NOT produce a prior (alpha-spec §4.4.3 (superseded): "a false
  positive is worse than a missing NCAA prior"). A link correction creates a new link version and MUST
  rebuild the affected priors.

### 3.4 Outputs

`estimate_equivalency` returns (`priors.py:118-126`):

| Key | Meaning |
|---|---|
| `slope` | full-sample OLS slope |
| `intercept` | full-sample OLS intercept |
| `oos_r2` | out-of-sample R² from one 70/30 split |
| `factor_check` | synth-only sanity slope, or None |
| `n_shared` | number of shared players |
| `league_type` | as passed |
| `league_factor` | from the table; returned but unused |

`build_priors` returns one row per input row, rookies **and** non-rookies, with columns `[player_id, position,
is_rookie, feeder_sv, prior_mean, prior_var, prior_sd]` (`:150-151`).

`washout_table` returns one row per position with columns `prior_sd`, `g1…g12` (rounded to 2 decimals),
and `games_to_<50%` (`:154-171`).

### 3.5 Explanation fields

Supplied to the engine output contract (engine-spec §6.7):

- `prior.mean` and `prior.var` (Kalman scale);
- the source (feeder league, link version, equivalency version);
- `q` (engine form, §6.2);
- the **current prior weight**, computed exactly from the filter (§4.4.2), not from the conjugate table.

The superseded alpha-spec §6.5 requires "NCAA prior contribution, if any".

---

## 4. Model and equations (as implemented)

### 4.1 Equivalency (`reference/python/backend/grid/priors.py:71-126`)

```text
shared = college_df[¬is_rookie]  ⋈_inner  nfl_ratings  on player_id            (:96-99)
x_i = feeder_sv_i,  y_i = rating_i,  n = |shared|
(a, b) = OLS of y on [1, x] over all of shared       (sklearn LinearRegression)  (:113)

Out-of-sample R² (one split):                                                     (:103-111)
  π   = numpy default_rng(3).permutation(n)          # depends on the row order of `shared`
  cut = floor(0.7·n);   T = π[:cut];   V = π[cut:]
  (a_T, b_T) = OLS on T
  oos_r2 = 1 − Σ_{i∈V} (y_i − a_T − b_T·x_i)² / Σ_{i∈V} (y_i − ȳ_V)²            # ȳ_V = test-set mean

factor_check = slope of polyfit(ability → feeder_sv, deg 1)   if 'ability' present else None   (:115-116)
league_factor = LEAGUE_FACTORS.get(league_type, LEAGUE_FACTORS["unknown"])   # returned, never applied
```

### 4.2 Translation (`reference/python/backend/grid/priors.py:129-151, 41-68`)

```text
prior_mean_i = a + b·feeder_sv_i + A(age_i)·[age_col given and present] + D(round_i)·[draft_round_col given and present]
A(age)   = +0.05 if age < 24;  −0.05 if age > 30;  0 otherwise
D(r)     =  0 if r is None;  −0.03 if r == 0 (undrafted);  +0.08 if r == 1;  +0.03 if r ≤ 3;  0 otherwise
           (a NaN round gives 0, because every comparison is false)
prior_sd_i  = PRIOR_SD[position_i]  (0.10 if absent)
prior_var_i = prior_sd_i²
```

`league_factor` does **not** enter `prior_mean` (KI-#23). The variance is a hand-set table. It is not derived
from the equivalency residual (KI-NEW-P1), and it does not depend on `feeder_snaps`, identity confidence
or draft status.

### 4.3 Hand-off to the Kalman layer (intended; not implemented)

GitHub issue #15 (KI-#15) proposes:

```text
x0 = [prior_mean, 0, 0]
P0 = diag(prior_var, ·, ·)
```

for a player's first observed week. Nothing implements it. The engine form is SSK §6.5 and §6.2 below.

### 4.4 Washout

#### 4.4.1 Oracle diagnostic (`reference/python/backend/grid/priors.py:154-171`)

```text
P0 = prior_sd²;   w_k = (1/P0) / (1/P0 + k/R),   k ∈ games = (1, 2, 4, 6, 8, 12)
games_to_<50% = first k in `games` with w_k < 0.5 (unrounded w), else None
```

This is a static, one-component, conjugate-normal weight: no process noise, no form or scheme component.
Its answer depends entirely on the assumed per-game observation variance R, and the oracle uses two
inconsistent values (critic G-9, X-18):

| Caller | R | QB | RB | WR | TE | DEF |
|---|---|---|---|---|---|---|
| `priors.py:197` (`__main__`, v0) | 0.40/90 = 0.00444 | 1 | 1 | 1 | 2 | 1 |
| `run_demo.py:91` | 0.12² = 0.0144 | 1 | 4 | 4 | 6 | 4 |

Entries are games to < 50% prior weight. Neither value is derived from `SSParams` and exposure.

#### 4.4.2 What the implemented filter actually does (measured in this consolidation)

The weight that the filtered talent puts on the prior mean after k played games can be computed exactly
by linearity. Run `kalman_two_component` with `y ≡ 0` and `x0 = [1, 0, 0]`; then `tau_filt[k−1]` is the
weight. Setup:

- `SSParams.from_position(pos)`;
- `P0 = diag(PRIOR_SD[pos]², 0.02, 0.01)`, where the form and scheme entries are the oracle's init values;
- 60 snaps every week.

The conjugate formula is shown with the same R for comparison:

| Position | R = r_scale/60 | Kalman: w after 1 / 2 / 4 / 6 / 8 / 12 games | Games to < 50% (Kalman) | Conjugate: w after 1 / 2 / 4 / 6 / 8 / 12 | Games to < 50% (conjugate) |
|---|---|---|---|---|---|
| QB | 0.00917 | 0.480 / 0.391 / 0.318 / 0.286 / 0.267 / 0.243 | 1 | 0.264 / 0.152 / 0.082 / 0.056 / 0.043 / 0.029 | 1 |
| RB | 0.00750 | 0.801 / 0.727 / 0.636 / 0.581 / 0.543 / 0.495 | 12 | 0.605 / 0.434 / 0.277 / 0.203 / 0.161 / 0.113 | 2 |
| WR | 0.00667 | 0.759 / 0.685 / 0.604 / 0.558 / 0.527 / 0.485 | 11 | 0.510 / 0.342 / 0.207 / 0.148 / 0.115 / 0.080 | 2 |
| TE | 0.00667 | 0.848 / 0.794 / 0.727 / 0.686 / 0.654 / 0.607 | > 12 | 0.649 / 0.481 / 0.316 / 0.236 / 0.188 / 0.134 | 2 |
| DEF (out of scope) | 0.00917 | 0.828 / 0.769 / 0.704 / 0.668 / 0.643 / 0.605 | > 12 | 0.652 / 0.483 / 0.319 / 0.238 / 0.190 / 0.135 | 2 |

**Why the two disagree.** Under `H = [1, 1, 1]` the innovation is shared among talent, form and
scheme_fit in proportion to their predicted variances. For RB/WR/TE, `PRIOR_SD²` (0.0036–0.0064) is
smaller than the form and scheme initial variances (0.02, 0.01), so most of the early evidence is absorbed
by form and scheme, not by talent.

"Wide prior → fast washout" holds for QB only. For the other positions the talent prior is
**sticky** for most of a season. The washout speed is therefore governed by `P0_form`/`P0_scheme`
as much as by `PRIOR_SD`, which makes those statistical-owner parameters (SSK §6.5;
DR-D19).

The engine MUST compute the prior-weight diagnostic from the actual filter as above. The conjugate
`washout_table` is not ported as a diagnostic of record.

### 4.5 Constants

| Constant | Value | Defined at | Provenance |
|---|---|---|---|
| `PRIOR_SD` | QB 0.16, RB 0.07, WR 0.08, TE 0.06, DEF 0.07 | `priors.py:26` | v0 upload `6b0eeee` (2026-06-21), hand-set, no derivation recorded. The "specialists tightest" comment (`:25`) is stale (TE is tightest). Synthetic units (KI-NEW-P4) |
| fallback prior SD | 0.10 | `priors.py:148` | v0, hand-set |
| `LEAGUE_FACTORS` | FBS 0.35, FCS 0.20, UFL 0.15, USFL 0.15, XFL 0.15, CFL 0.18, unknown 0.25 | `priors.py:30-38` | Phase-3 Task 5 "priors expansion" (CN `c58aebd`, 2026-06-21). Hand-set; the governing Phase-3 plan was never committed (critic G-3). **Unused** (KI-#23) |
| age steps | +0.05 below 24, −0.05 above 30 | `priors.py:41-52` | Phase-3 Task 5 (`c58aebd`), hand-set (KI-#49) |
| draft steps | round 1 +0.08, rounds 2–3 +0.03, undrafted −0.03, rounds 4+ 0 | `priors.py:55-68` | Phase-3 Task 5 (`c58aebd`), hand-set (KI-#49) |
| split seed | 3 | `priors.py:104` | v0 |
| train fraction | 0.7 | `priors.py:106` | v0 |
| washout games | (1, 2, 4, 6, 8, 12); rounding 2 dp | `priors.py:155, :166` | v0 |
| synthetic `league_factor` | 0.62 | `synth.py:51` | synthetic generator (planted truth) |
| synthetic `college_noise_sd` | 0.030 | `synth.py:52` | synthetic generator |
| synthetic `frac_rookies` | 0.4 | `synth.py:298` | synthetic generator |

On real 2023 data the RAPM rating SD is about 0.04 at every position (cn-issues NEW-A3/P4). On that
scale `PRIOR_SD` (QB 0.16) and the ±0.05 / +0.08 steps are 1–4 rating SDs. None of these constants
may be used on real data unless re-derived (KI-NEW-P4).

### 4.6 Constraints

The oracle validates none of these. The engine MUST validate them and raise a typed error (§8.3); it
MUST NOT apply a silent fallback.

- Shared-player count ≥ a declared minimum (KI-G8).
- Non-degenerate feeder-SV variance.
- Finite inputs.
- Unique `player_id` in both frames.
- A known position class and a known feeder league.
- `prior_var > 0`.
- Prior and Kalman state on the same declared scale (§6.2).

---

## 5. Algorithm, numerics and determinism

### 5.1 Numerics

The equivalency is a one-regressor OLS: closed form, O(n). There are no numerical risks beyond
degenerate inputs.

Failure behaviour (oracle; KI-G8):

| n_shared | Result |
|---|---|
| 0 or 1 | sklearn `ValueError`: the training split is empty (measured with well-formed frames; cn-issues reports a `KeyError` for n = 0 in its setup) |
| 2–3 | `oos_r2 = −inf` or NaN: the test set has one point, so its variance is zero |

Duplicate ids in `nfl_ratings` silently duplicate rows of `shared`. PR #91 reported duplicate `player_id`s
perturbing results at about 1e-6 (critic G-3).

### 5.2 Determinism and seed policy

The only randomness is the split permutation: NumPy PCG64 with seed 3. It depends on the **row order** of
`shared`, which is the `college_df` order after filtering and merging.

The Rust port MUST NOT re-implement NumPy's permutation algorithm. Split indices (or fold assignments
under DR-C9 / KI-#48 k-fold) are an **input** recorded in the fixture and in the equivalency's version
record. Everything else is deterministic.

---

## 6. Incremental behaviour and the engine form

### 6.1 Oracle

There is no state, persistence or versioning. Each call refits. Priors are static per prospect and never
updated within a season; mid-season evolution is the KI-#46 enhancement. There is no hand-off to the
filter (KI-#15).

### 6.2 Engine form (proposed — DR-C9)

The proposed form is alpha-spec §6.3 (superseded), with the oracle's feeder-SV equivalency as **one translated
component**:

```text
θ_ncaa_translated(i) = a_pos + b_pos · feeder_sv_i
    # (a_pos, b_pos): equivalency fit as-of on shared NCAA→NFL players
    # (§3.3), per position class; NCAA only.
    # Further translated components MAY come from alpha-spec §4.2.3 (superseded)
    # features, each with its own equivalency.
θ_prior(i)           = q_i · θ_ncaa_translated(i) + (1 − q_i) · θ_position_draft_prior(i)
σ²_prior(i)          = residual variance of the equivalency (KI-NEW-P1), inflated for undrafted,
                       transferred, position-converted or identity-uncertain players (alpha-spec §6.3 (superseded))
Kalman hand-off       x0 = [θ_prior on the Kalman credit scale, 0, 0],
                      P0 = diag(σ²_prior, P0_form, P0_scheme)        (SSK §6.5)
                      ⇒ the filter update is the dynamic EB posterior with n0 = R/σ²_prior
n0 (equivalently the σ²_prior scale) learned by position through rolling-origin validation (alpha-spec §6.3 (superseded))
NCAA influence capped for components with weak translation evidence
```

**The parts of the oracle the proposed default ports.**

- The shared-player equivalency, made per-position, as-of and per-league-ready.
- The `(mean, variance)` hand-off.

**What it does not port.**

- The `LEAGUE_FACTORS` table. Feeder leagues other than NCAA are documented, unused.
- The age and draft step functions. They go back to the statistical owner and are not ported as-is (KI-#49).
- `PRIOR_SD` as a constant. The variance is derived as above.
- The conjugate `washout_table` (§4.4).

**Still open under DR-C9.**

- The definition of `q` (identity confidence, NCAA sample size, role comparability, opponent and
  conference adjustment quality, draft and combine agreement).
- The form of `θ_position_draft_prior`.
- The influence cap.
- How `feeder_sv` itself is computed from CFBD play-by-play. The `load_cfbd` docstring sketches a reduced
  Layer-1/Layer-2 pass with within-NCAA opponent adjustment; alpha-spec §4.2.3 (superseded) lists
  box-score features instead.

**Lifecycle requirements.**

- The equivalency is estimated once per season boundary.
- It is persisted as a versioned EB/affine parameter record: prior mean, prior variance, population
  variance, sample counts, shrinkage parameters, version (final-build-spec §11.6 (superseded)).
- Priors are immutable for a season except through an identity-link version change.
- The prior is applied to the Kalman state only at the player's first slot in the evidence window, or at
  a window re-initialisation under DR-C6.

---

## 7. Validation evidence

Every synthetic number labelled **legacy** comes from the oracle's legacy generator, which draws defenders
from the offense's own team (KI-NEW-Y0; critic G-1). Legacy numbers are history, not targets.
**Defender-fixed** numbers come from the one-line generator fix (critic G-1).

All values were re-measured in this consolidation: `load_synthetic()`, `fit(n_iter=3)`, market seed 1,
single-threaded. They match critic G-1.

### 7.1 Tier-0 gates (`tests/grid/test_tier0_recovery.py`)

| Metric | Gate | Legacy | Defender-fixed |
|---|---|---|---|
| equivalency slope | 0.9 ≤ slope ≤ 1.8 (`:137`) | 1.3159 | 1.2119 |
| equivalency intercept | — | 0.0043 | 0.0053 |
| OOS R² | ≥ 0.05 (`:138`) | 0.1489 | 0.2134 |
| `factor_check` (planted 0.62) | ungated | 0.6779 | 0.6779 |
| `n_shared` / rookies | — | 183 / 105 | 183 / 105 |
| rookie corr(prior_mean, planted ability) | ≥ 0.50 (`:143`) | 0.5829 | 0.5829 |

**The rookie gate does not test the estimator.** Tier 0 calls `build_priors` without the age or draft columns.
So `prior_mean = a + b·feeder_sv`, and for any `b > 0`:

```text
corr(prior_mean, ability) = corr(feeder_sv, ability) on rookies = 0.5829
```

That number is a property of the synthetic college generator (`college_noise_sd`, `league_factor`). This is
why it is identical on both generators. `factor_check` likewise depends only on the generator (0.6779 on
both).

Of the five Tier-0 prior numbers, only the slope band and the OOS R² exercise `estimate_equivalency`.
The rookie gate guards only the sign of the slope. A class-D target set MUST include an
estimator-sensitive gate (§10.3).

### 7.2 Exploratory per-position equivalency

Not a gate. `estimate_equivalency` run separately per position on the canonical synth:

| Position | n_shared | Legacy slope / OOS R² / rookie corr | Defender-fixed slope / OOS R² |
|---|---|---|---|
| QB | 13 | 2.404 / −0.133 / 0.852 | 2.257 / −0.319 |
| RB | 27 | 1.414 / 0.511 / 0.529 | 1.278 / 0.321 |
| WR | 36 | 1.139 / 0.597 / 0.406 | 0.868 / 0.286 |
| TE | 17 | 0.416 / −0.078 / 0.467 | 0.235 / −0.223 |
| DEF | 90 | 0.874 / 0.055 / 0.542 | 0.965 / 0.085 |

Two observations:

- **Slopes differ by position by up to a factor of 6.** The pooled regression mixes position-specific
  rating scales. QB rating SD is 0.272 against 0.07–0.09 elsewhere.
- **Per-position fits are unstable at these n.** The single 70/30 split gives a negative OOS R² for
  QB and TE (KI-#48).

Residual SD of the pooled legacy equivalency by position: QB 0.189, RB 0.071, WR 0.070, TE 0.067, DEF 0.074.
These are within about 20% of `PRIOR_SD`, which is consistent with deriving the variance from the
residual (KI-NEW-P1). It is not evidence that `PRIOR_SD` was derived that way.

### 7.3 Unit tests (all pass on Linux at `59bce1d`)

**`tests/grid/test_priors.py` (15):**

- age steps (`:17-30`);
- draft steps (`:34-52`);
- `LEAGUE_FACTORS` orderings (`:56-64`);
- `build_priors` with age, with draft round, and with a missing column silently ignored (`:80-113`).
  The engine reverses that last behaviour (§8.3);
- the real-data unblock without `ability` (`:134-145`);
- synth path unchanged (`:148-154`);
- equivalency feeds `build_priors`, with `prior_mean` monotone in `feeder_sv` (`:157-173`).

**`tests/projection/test_features.py`:** the prior lookup tests (`:85-104`) and NaN-for-no-prior defaults
(`:117-133`).

### 7.4 Real-data evidence

None. There is no feeder-SV source and no caller (§1).

---

## 8. Known defects and required engine behaviour

### 8.1 Registered defects (`docs/00-meta/known-issues.md`)

| KI | Defect (oracle) | Required engine behaviour |
|---|---|---|
| KI-G8 | Small shared pools break the OOS computation (§5.1) | Typed `InsufficientSharedPlayers` with a declared minimum n per position and league |
| KI-#48 | Single 70/30 split, seed 3, unstable by about ±0.15 | k-fold or repeated split with injected fold assignments (DR-C9) |
| KI-#23 | `league_factor` computed and documented but never applied | NCAA-only per-league equivalency estimated from data. The constant table is not ported (DR-C9) |
| KI-#49 | Age and draft step functions, hand-set | Returned to the statistical owner; not ported as-is (DR-C9) |
| KI-NEW-P1 | `prior_var` is a hand-set table, not derived from the equivalency residual | Variance derived from the equivalency (§6.2) |
| KI-NEW-P2 | Priors are not wired to any real-data path; the verdict uses `prior_mean = 0.0` | Wired through SSK §6.5 and the feature store (P1-05/P1-07) |
| KI-NEW-P4 | Scale constants are synth-unit, 4–5× too wide on real data | Data-derived per data version |
| KI-#15 | Priors never reach the Kalman `x0`/`P0` | SSK §6.5 |
| KI-NEW-P3 | The `__main__` self-test (`priors.py:174-197`) fails under `python -m` | DROP |
| KI-NEW-Y0 | Equivalency numbers above measured on the defender-bug synth | Targets re-set on the fixed synth (DR-B4) |
| KI-#44 | Hierarchical equivalency (enhancement) | Research; subsumes KI-#23 and KI-#48 |

### 8.2 Defects found in this consolidation (to be registered)

| # | Defect | Required engine behaviour |
|---|---|---|
| P-1 (KI-NEW-Z24) | **Scale mismatch.** The prior is on the RAPM-rating scale, the Kalman state on the Layer-1-credit scale (credit-on-rating slope 1.19–1.40 on the legacy synth). Issue #15's proposed `x0 = prior_mean` would mis-scale the prior | The equivalency target MUST be the same quantity as the Kalman talent state (for example end-of-season smoothed talent, which `priors.py:83-90` already allows), or an explicit, versioned scale map (DR-D19) |
| P-2 (KI-NEW-Z25) | The Tier-0 rookie-prior gate is invariant to the estimated equivalency (§7.1) | Class-D targets include an estimator-sensitive gate (§10.3) |
| P-3 (KI-NEW-Z26) | `washout_table` is a conjugate approximation with inconsistent R and contradicts the implemented filter (§4.4) | Filter-derived diagnostic only |
| P-4 (KI-NEW-Z27) | The regression is pooled across positions, while the variance is position-specific and the rating scale differs by position (§7.2) | Per-position-class equivalency or an explicit position term (DR-C9) |
| P-5 (KI-NEW-Z28) | Silent fallbacks: unknown position gives 0.10, unknown league gives 0.25, NaN age or draft gives 0, a named but absent column is skipped | Typed failures (§8.3) |
| P-6 (KI-NEW-Z29) | No as-of handling; the equivalency is fit on all shared players regardless of time | §3.3 |
| P-7 (KI-NEW-Z30) | Duplicate `player_id`s silently duplicate regression rows | Typed `DuplicatePlayerId` |
| P-8 (KI-NEW-Z31) | `feeder_snaps` is ignored; exposure does not affect prior variance or the regression weights | Statistical owner decides weighting (DR-C9) |

### 8.3 Typed failures (proposed — DR-B6)

- `InsufficientSharedPlayers`
- `DegenerateFeederVariance`
- `NonFiniteInput`
- `DuplicatePlayerId`
- `UnknownPositionClass`
- `UnknownFeederLeague`
- `MissingRequiredColumn`
- `ScaleMismatch` (prior scale ≠ Kalman scale)
- `AmbiguousIdentityLink` (no prior issued; recorded for the review queue)

Each blocks promotion of the affected artifact. Each is a deliberate divergence listed in
`reference/python/PARITY.md`.

---

## 9. Open decisions

| Decision | Proposed default | Effect |
|---|---|---|
| **DR-C9** NCAA / feeder prior form | Alpha-spec §6.3 (superseded) form with feeder-SV equivalency as one translated component (needs a CFBD play-by-play pass). NCAA only. UFL/USFL/XFL/CFL `LEAGUE_FACTORS` documented but unused. Age and draft step functions returned to the statistical owner, not ported as-is | §6.2. Open inside it: `q`, `θ_position_draft_prior`, influence cap, `feeder_sv` construction, exposure weighting |
| DR-C10 Kalman semantics bundle | Prior-based `x0`/`P0` | SSK §6.5 |
| DR-C6 Three-season window | Kalman re-initialised from the windowed prior on replay | Which seasons enter the equivalency fit; when the prior is re-applied |
| DR-C1 Live vs offseason | Offseason participation RAPM seeds priors and features | The NFL rating target for the equivalency |
| DR-B4 Synthetic world | Fix defenders and regenerate targets | Class-D targets (§10.3). The synthetic college world should also plant range restriction and position-specific translation |
| DR-A11 Fixture licensing | Synthetic-only parity fixtures | No CFBD payload enters the parity fixtures |
| DR-B6 Typed failures | Typed failure; oracle unchanged | §8.3 |
| **DR-D19** | — (open) | §8.2 P-1, SSK §6.5. The scale on which the prior is expressed, and `P0_form`/`P0_scheme` for prior-seeded players (they govern washout speed, §4.4.2) |

---

## 10. Rust port plan

### 10.1 Homes

| Concern | Crate |
|---|---|
| Equivalency (affine fit with injected splits), translation, EB variance, versioned parameter record | `crates/models` (`grid-models`): `priors`, `empirical_bayes`, affine utility |
| CFBD adapter | `crates/ingestion` (provider contract `docs/04-providers/cfbd/`) |
| NCAA→NFL linking, `q` inputs, review queue | `crates/identity` |
| Prior features as-of | `crates/features` |
| Rookie / low-evidence scorecards | `crates/evaluation` |
| Synthetic college fixtures | `synth` crate (DR-A8) |

### 10.2 Work packages

| WP | Scope |
|---|---|
| **P1-07** (§6.1 Layers A–D + §6.3 NCAA/rookie priors) | Owns the engine form (§6.2) |
| **P1-12** (GRID component port) | Ports the oracle equivalency and translation for parity (§10.3 class A) and the Kalman hand-off (SSK §6.5) |
| **P1-04** | NCAA adapter and identity linking |
| **P1-05** | As-of prior features and leakage tests (§3.3) |
| **P1-09** | Rookie / low-evidence scorecards (superseded alpha-spec §9.2 evaluation foundation) |

### 10.3 Parity class and concrete targets

Classes are per `docs/03-contracts/parity-fixture-contract.md` (proposed — DR-B3). Fixtures are
synthetic only.

| ID | Fixture | Compared outputs | Class |
|---|---|---|---|
| PF-PR-01 | Canonical synthetic `college` frame plus `ratings` from the corrected oracle, with the exported split indices `T`/`V` | `slope`, `intercept`, `oos_r2`, `n_shared`, `factor_check` | **A** (≤ 1e-12 abs; one-regressor closed form) |
| PF-PR-02 | `tests/grid/test_priors.py` frames (`_make_college_df`, `_shared_college_and_ratings`) and §10.4 | `prior_mean`, `prior_var`, `prior_sd` with and without age and draft | **A** (exact). Required only if the step functions survive DR-C9; otherwise the translation without steps |
| PF-PR-03 | Filter-derived prior weight (§4.4.2) on the PRIOR_SD × position grid | weights at k = 1…12 | **A** against the oracle `kalman_two_component` with explicit `x0`/`P0` |
| PF-PR-04 | Defender-fixed synthetic world after DR-B4 | (a) slope band and OOS R² (currently 1.2119 / 0.2134); (b) an estimator-sensitive rookie gate, for example corr(prior_mean, planted ability) **relative to** corr(feeder_sv, planted ability), or a mean-calibration error of the translated prior; (c) per-position checks once the synthetic world plants position-specific translation | **D** (floors calibrated below the fixed-synth observed values). The legacy 0.5829 rookie corr is a generator property, not a target |

**Not targets.**

- `washout_table` output (replaced, §4.4).
- `LEAGUE_FACTORS` (unused).
- Any real-data number.

### 10.4 Reference examples (oracle `build_priors`, exact)

Inputs: `equiv = {slope: 0.5, intercept: 0.01}`, `age_col='age'`, `draft_round_col='draft_round'`.

| Player | Inputs | Computation | Output |
|---|---|---|---|
| x | QB, feeder_sv 0.2, age 22, round 1 | 0.01 + 0.5·0.2 + 0.05 + 0.08 | `prior_mean = 0.24`, `prior_var = 0.0256`, `prior_sd = 0.16` |
| y | position "K", feeder_sv −0.1, age 31, round 0 | 0.01 − 0.05 − 0.05 − 0.03 | `prior_mean = −0.12`, `prior_var = 0.0100`, `prior_sd = 0.10` (the silent fallback the engine replaces with `UnknownPositionClass`) |

Kalman-implied prior weight for RB with snaps 60 (§4.4.2): 0.801 after 1 game, 0.495 after 12.

---

## 11. Superseded-spec mapping

| Superseded section | What this component satisfies | Gap |
|---|---|---|
| alpha-spec §6.3 (superseded) NCAA prior formulation | A translated feeder prior with position-specific variance. The Kalman hand-off is the dynamic EB posterior (`n0 = R/σ²`) | No `q`, no `θ_position_draft_prior` blend, no learned `n0`, no influence cap, no variance widening for undrafted, transfer or identity-uncertain players, no role/efficiency split; not wired (KI-#15). §6.2 is the proposed engine form |
| alpha-spec §4.2, §4.2.1, §4.2.2 (superseded) CFBD source, datasets, call budget | — | Not implemented (`load_cfbd` stub). `docs/04-providers/cfbd/` is a placeholder for P1-04 |
| alpha-spec §4.2.3 (superseded) NCAA features by position | — | None implemented. The `college` frame is synthetic-only with one `feeder_sv` column. The oracle's feeder-SV approach needs a CFBD **play-by-play** GRID pass beyond §4.2.3's box-score features (DR-C9) |
| alpha-spec §4.4.3 (superseded) NCAA-to-NFL matching tiers | — | Absent (synthetic ids join exactly). §3.3 and §8.3 add the "no prior on ambiguous link" rule |
| alpha-spec §2.5 (superseded) low-evidence definition; continuous decay | Decay through the filter, in principle | Eligibility is a static `is_rookie` flag; none of the §2.5 criteria is implemented |
| alpha-spec §11.5 (superseded) rookie/low-evidence features | Draft round and age (as step adjustments) | Pick, combine, college market share, usage/PPA, conference adjustment, recruiting, position conversion, identity confidence, prior variance as a feature: absent (`prior_var` exists as a feature in `features.py`) |
| alpha-spec §6.2 (superseded) EB and affine rows | Affine college→NFL translation | EB prior parameters are not persisted or estimated |
| alpha-spec §6.5 (superseded) "NCAA prior contribution" | — | Not computed. §3.5 specifies it from the filter |
| (no superseded section) USFL/XFL/UFL/CFL feeder leagues | — | The superseded specs name only NCAA (CFBD). The oracle's broader feeder set is documented here only and is unused (NCAA only, DR-C9) |
| final-build-spec §11.6 (superseded) EB persisted parameters | — | Nothing persisted. §6.2 lifecycle requirement |
| final-build-spec §11.7 (superseded) affine mapping | The feeder map is affine | No generic affine type, inverse, or serialization in the oracle |
