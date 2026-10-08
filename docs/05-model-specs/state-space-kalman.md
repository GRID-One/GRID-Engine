---
model-spec-id: MS-STATESPACE-KALMAN
status: Draft            # equations below are NOT yet approved by the statistical owner
statistical-owner: repository owner (holds every engine-spec §1.4 role today)
version: 0.1.0
supersedes: none (first engine model spec for this component)
---

# Model spec — State-space layer (per-player Kalman: talent, form, scheme_fit)

Consolidation work package **P0-01** (ADR-011 "Engine-only pivot", ADR-012 "Python reference
oracle"). This is a contract-level document: engine-spec §1.5 places model specs above the work
package, the tests and the code. The equations here bind the Rust implementation, and an
implementing agent MUST NOT redefine them (superseded alpha-spec §6.6, carried into engine-spec §6).

**How to read this spec.**

- **Oracle behaviour.** Sections 3–7 describe the Python reference oracle **exactly as implemented** at
  cautious-nevermore (CN) `59bce1d`. Code is cited as `reference/python/<path>:<line>`; those line
  numbers equal CN `59bce1d` (`reference/python/MANIFEST.tsv` lists both files as `verbatim`).
  The oracle is executable evidence, not authority (engine-spec §1.7).
- **Engine requirements.** Sections 6, 8 and 10 use MUST/SHOULD/MAY. Where an engine requirement
  differs from the oracle, the difference is a deliberate divergence. It is recorded in §8 and in
  `reference/python/PARITY.md`.
- **Decision tags.** "(proposed — DR-xx)" marks a default that waits on the owner
  (`docs/00-meta/decision-register.md`). It is not settled.
- **Defect IDs.** `KI-` IDs are entries in `docs/00-meta/known-issues.md`.
- **Indexing.** Week indices are 0-based (`w = 0 … W−1`), as in the code.
- **Inventory reports.** "critic", "cn-issues", "cn-docs", "reconcile-code-first" and
  "reconcile-spec-first" refer to the consolidation inventory, kept verbatim in
  `docs/06-sessions/2026-10-01-consolidation-inventory/`. The inventory's line numbers point at
  CN `59bce1d`.

## 0. Template conformance

This spec follows `docs/99-templates/template-model-spec.md`, extended with numbered sections.

| Template heading | Where it is covered |
|---|---|
| Target | §3.1 |
| Inputs | §3.2, §3.3 |
| Equations | §4.1–§4.6 |
| Priors | §4.7, §6.5 and `cross-league-priors.md` |
| Constraints | §4.9, §8.3 |
| Seed policy | §5.3 |
| Tolerances | §10.3 |
| Reference examples | §10.4 |
| Explanation fields | §3.5 |
| Validation and promotion | §7, §10.3 |
| Known limitations | §8, §9 |

---

## 1. Purpose and statistical intent

The layer turns a noisy weekly per-player efficiency signal into a dynamic latent state. That state
is separated **by timescale and persistence** into three components
(`reference/python/backend/grid/statespace.py:1-21`):

| Component | Symbol | Dynamics (as implemented) | Meaning |
|---|---|---|---|
| talent | τ | Random walk (F = 1). Variance is inflated by a discount factor every step. | Slow, persistent ability |
| form | f | AR(1) with φ = 0.50 | Transient, mean-reverting deviation |
| scheme_fit | s | Near-random-walk AR(1) with φ_s = 0.985 and very small innovation | Changes materially only at a regime change (coaching or scheme) |

The observation is `y_w = τ_w + f_w + s_w + noise`, with noise variance scaled by exposure (snaps).

**What the layer may estimate.** Per-player efficiency in the GRID value currency: dV, expected
drive points per on-field offensive snap. It produces three products, and they MUST NOT be
aliased for one another (`reference/python/backend/projection/features.py:9-18`):

1. **Filtered state** (data through week w). This is the real-time grade, and it is causal.
2. **One-step predictive** `N(H·x⁻_w, S_w)`, with `S_w` including the observation noise R. This is the honest forecast
   distribution, and calibration MUST be scored against it, never against the post-update filtered variance
   (`reference/python/backend/grid/statespace.py:206-213`; PR #63).
3. **RTS-smoothed state** (data through the end of the series). This is the "best retrospective talent". It uses
   future weeks, so it MUST NOT be used for forecast calibration or as a forecast-time feature of
   any week inside its own smoothing window.

**What the layer treats as nuisance or leaves to other layers.**

- **Volume and opportunity** belong to Layer C. Exposure enters only as measurement precision
  (`R = r_scale/snaps`).
- **Availability** belongs to Layer A. A week the player did not play is **missing data**, not a zero: the filter
  predicts without updating, so uncertainty grows during an absence
  (`reference/python/backend/grid/statespace.py:20-21`).
- **Offensive line, scheme baseline and opponent strength** are absorbed upstream: by the RAPM
  team intercepts (`rapm-attribution.md`), and by the Layer-1 context model that removes situation and
  opponent-defence effects (`layer1-credit.md`). This layer does not re-estimate them.
- **Fantasy points** are not produced here. The latent state is a Layer-D covariate (`projection-stack.md`).

**The one interpretable knob.** Process noise is governed by the West–Harrison discount d:
"d≈1 means very stable (QB talent), lower d means responsive"
(`reference/python/backend/grid/statespace.py:11-14`). At known regime-change weeks d is
temporarily dropped and R is inflated for the first games back. The code calls this R inflation
"rust"; in this spec "rust" in lower case always means that R inflation, never the Rust language.

**Identifiability caveat (carried from the Phase-4 plan's Known Constraints; cn-docs §8 item 14).**
Under the additive `H = [1, 1, 1]`, talent and scheme_fit are only weakly identified. The design
mitigates this with `q_scheme ≪ q_form` and by moving scheme_fit materially only on explicit coaching
resets. **The decomposition has never been validated against planted components.** The synthetic
world plants one current-ability arc for the focus QB, not separate τ/f/s
(`reference/python/backend/grid/synth.py:86-105`). Engine outputs MUST NOT present scheme_fit or form as
validated estimands until `synthetic-world.md` plants them and §10.3 class-D gates pass.

## 2. Position in the engine

| Aspect | Content |
|---|---|
| Engine-spec layer | §6.1 Layer D (efficiency latent). GRID is a signal provider, not the projection (proposed — DR-C2). GRID talent enters Layer D as role-specific covariates in EB-shrunk rate models (proposed — DR-C3) |
| Signal stack | §6.2 GRID signal stack: V(s) → dV → Layer-1 weekly credit → **this layer** → features. Offseason RAPM is not this layer's observation (§3.1) |
| Live vs offseason | §6.3. The validated observation (Layer-1 credit) is participation-dependent. In-season it MUST be replaced by a participation-free Layer-1′ credit (proposed — DR-C1) |
| Statistical methods | §6.4 Kalman filtering, full RTS and fixed-lag RTS (fixed-lag is not in the oracle, §6.4 below) |
| Priors | §6.5. `cross-league-priors.md` supplies `x0`/`P0` for low-evidence players. **Not wired in the oracle** (KI-#15) |
| Ensemble | §6.6 member "Kalman latent-state model" (superseded alpha-spec §6.4 item 3). It has not been scored on real data as a forecast; see §7.4 |
| Upstream | `value-model.md` (dV), `layer1-credit.md` (weekly credit y and snaps), `cross-league-priors.md` (x0, P0), interventions (Layer A injury return; coaching-change provider; changepoint detector §4.6) |
| Downstream | `projection-stack.md` (`smoothed_talent`, filtered talent/form features), `docs/03-contracts/engine-output-contract.md` (trajectory and predictive band, engine-spec §5.5), `evaluation-and-leakage.md` (NIS, PIT, PICP, as-of rules) |
| Contracts | `docs/03-contracts/plays-contract.md` (indirectly, via credit), `engine-output-contract.md`, `parity-fixture-contract.md` |

**Scope.** Engine-spec §2 (superseded alpha-spec §2.1) scopes the engine to QB/RB/WR/TE, with IDP
out of scope. The oracle's `DEF` parameter row
(`reference/python/backend/grid/statespace.py:132`) describes synthetic defenders and is **not
ported**.

Alpha-spec §6.2 also lists Kalman states for pace, pass tendency and opportunity share. Those are
other instances of the same machinery, and each needs its own model spec. This spec covers only
the player-efficiency instance.

## 3. Inputs and outputs

### 3.1 Target (observation)

`y_w` is the player's **weekly Layer-1 credit**: the mean, over the player's on-field offensive
snaps in week w, of the cross-fitted context residual `dv − g_oof(state, opp_def_rating_sum)`.

- **Units:** dV (expected drive points) per snap.
- **Support:** real-valued.
- **Code:** `reference/python/backend/grid/layers.py:522-538` (focus QB) and `:561-588`
  (any position, Phase 2b PR #80). Defined in `layer1-credit.md`.

This is the observation of the **validated** paths: the goldens, Tier 0, calibration, and the verdict's
`smoothed_talent` (`reference/python/backend/validation/verdict.py:127-191`).

The incremental production path feeds a different quantity: the cumulative season-to-date RAPM
coefficient, with `snaps = 1` (`reference/python/backend/pipeline/weekly_update.py:307-309`). That is a
defect (KI-NEW-W3, KI-#24; critic X-13 adjudicates against final-build-spec's reading). The engine
observation MUST be weekly credit with real exposure (proposed — DR-C10).

### 3.2 Inputs

**Batch filter** `kalman_two_component(y, snaps, played, weeks=None, interventions=None, params=None, x0=None, P0=None)`
(`reference/python/backend/grid/statespace.py:168-175`):

| Input | Type / shape | Units | Null semantics |
|---|---|---|---|
| `y` | float[W] | dV/snap | `NaN` on a did-not-play week. The update is skipped unless `played[w]` **and** `isfinite(y[w])` (`:222`) |
| `snaps` | float[W] ≥ 0 | on-field snaps (count) | Used only through `max(snaps, 1.0)`. A missing week still gets a predictive R from the floor |
| `played` | bool[W] | — | `False` → predict-only |
| `interventions` | set of 0-based week indices | — | Indices ≥ W are silently ignored |
| `params` | `SSParams` | — | Default `SSParams()` (WR/TE/default values) |
| `x0`, `P0` | float[3], float[3×3] | dV/snap, (dV/snap)² | Default: look-ahead `x0` and fixed `P0` (§4.7; KI-#15) |
| `weeks` | unused | — | Accepted and ignored |

**Missing-week input contract (load-bearing).** An absent week MUST be `played=False` **and**
`y=NaN`. The default `x0` reads `y[:3]` without consulting `played`. Seeding absences with 0.0 biased a
leading-absence RB's smoothed talent from 1.012 to 0.959 (PR #91 round 2; cn-docs §8 item 5). See
`reference/python/backend/validation/verdict.py:206-233` and
`reference/python/backend/projection/features.py:89-93`.

**Incremental step** `kalman_step(state, obs, snaps=None, params=None, interventions=None, scheme_resets=None, return_pred=False)`
(`reference/python/backend/grid/statespace.py:329-352`):

| Input | Type / shape | Null semantics |
|---|---|---|
| `state` | `KalmanState(mu (n,3), sigma (n,3,3), player_ids)` | — |
| `obs` | float[n], aligned to `state.player_ids` | `NaN` → predict-only. Only `isnan` is tested (`:403`), so ±inf is **not** treated as missing (contrast `:222`) |
| `snaps` | float[n] | Defaults to ones (`:359-360`) |
| `interventions` | set of player ids | Unknown ids are silently ignored (`:363`) |
| `scheme_resets` | set of player ids (coaching change) | Unknown ids are silently ignored (`:384`) |

### 3.3 As-of and publication constraints

- **Observation timing.** `y_w` is an outcome. It is usable for forecasts of week W only when
  `w ≤ W−1` (engine-spec §4.5; `reference/python/backend/validation/asof.py:84-90`).
- **Publication lag.** Layer-1 credit as implemented needs participation: `off_players` selects the
  player's snaps, and `def_players` builds the opponent sum (`reference/python/backend/grid/layers.py:488-495, 532, 581`).
  nflverse participation is published once, after the postseason (critic G-5). On the live path the
  observation therefore MUST be the participation-free Layer-1′ credit (proposed — DR-C1).
  Participation-dependent credit is admissible only for completed, published seasons. Backtests
  MUST NOT use current-season participation before its publication timestamp.
- **Interventions are pre-game signals.** An intervention dated week W is usable for week W
  (`reference/python/backend/validation/asof.py:128-146`; undated records raise `LeakageError`).
  Coaching changes are keyed by `week_effective`
  (`reference/python/backend/pipeline/weekly_update.py:321-330`). That is the effective week, not
  the time the change became known. The engine MUST carry a knowledge timestamp per intervention record
  (engine-spec §4.3 coaching changes; §4.5 publication lag).
- **Changepoint flags are outcome-derived.** They are computed from `y_w` itself (§4.6), so they
  are not pre-game signals (see §8.2 D-8).

### 3.4 Outputs

Batch outputs (`reference/python/backend/grid/statespace.py:247-258`). All are float[W] unless noted:

| Key | Definition | Product |
|---|---|---|
| `tau_filt`, `form_filt`, `scheme_filt` | components of x⁺_w | filtered |
| `total_filt` | `H·x⁺_w` | filtered current ability |
| `var_total_filt` | `H·P⁺_w·Hᵀ`. Excludes R. MUST NOT be used as a forecast band | filtered |
| `total_pred` | `H·x⁻_w` | one-step predictive mean |
| `var_total_pred` | `S_w = H·P⁻_w·Hᵀ + R_w` | one-step predictive variance |
| `tau_smooth` | first component of x^s_w | smoothed talent |
| `total_smooth` | `H·x^s_w` | smoothed current ability |
| `var_tau_smooth` | `P^s_w[0,0]` | smoothed talent variance |
| `sigma_smooth` | float[W,3,3], all of `P^s_w` | smoothed covariances |

Not returned: smoothed form or scheme separately, and the variance of `total_smooth`.

Incremental outputs: the updated `KalmanState`, plus `pred_var` float[n] (`S_i`) when
`return_pred=True` (`:347-351, :396-399, :414-416`). `detect_changepoints` returns `set[str]` (`:268`).

### 3.5 Explanation fields

The component contributes the following to the per-player explanation payload (engine-spec §6.7).
Field names are fixed by `docs/03-contracts/engine-output-contract.md`. Values are in dV per snap, and
positive means above league-average efficiency.

- Filtered talent, form and scheme_fit, plus the predictive mean and `S`. These feed "recent NFL
  evidence contribution" and "uncertainty drivers".
- Smoothed talent and its variance (retrospective; labelled as such).
- Regime flags for the week: intervention applied, scheme reset applied, changepoint z-score.
- Prior contribution: the weight still carried by `x0` in the filtered talent. It is computable
  exactly by linearity (run the filter with `y ≡ 0` and `x0 = e₁`; see `cross-league-priors.md` §4.5).
  This replaces the conjugate `washout_table` approximation.

---

## 4. Model and equations (as implemented)

### 4.1 State, transition and observation

```text
x_w = [τ_w, f_w, s_w]ᵀ                         N_STATE = 3            (statespace.py:31)
F   = diag(1, φ, φ_s)                                                   (statespace.py:155-161)
H   = [1, 1, 1]                                                         (statespace.py:165)
y_w = H·x_w + ε_w,   ε_w ~ N(0, R_w),   R_w = r_mult_w · r_scale / max(snaps_w, 1)
```

### 4.2 Predict step (batch, `reference/python/backend/grid/statespace.py:195-204`)

```text
x⁻_w = F·x⁺_{w−1}
P⁻_w = F·P⁺_{w−1}·Fᵀ
d_w  = d_spike if w ∈ I else d_steady
P⁻_w[0,0] ← P⁻_w[0,0] / d_w                    # discount on TALENT VARIANCE ONLY
P⁻_w[1,1] ← P⁻_w[1,1] + q_form
P⁻_w[2,2] ← P⁻_w[2,2] + q_scheme
if w ∈ I:  P⁻_w[1,1] += 5·q_form;  P⁻_w[2,2] += scheme_reset_var;  g ← 0
```

`x⁺_{−1} = x0` and `P⁺_{−1} = P0`, so week 0 is itself a predict step from `(x0, P0)`. `I` is the set of
intervention weeks, and `g` is the event counter `games_since_event`, initialised to 99 (`:192`).

**The discount is a component discount, not the textbook full-matrix West–Harrison discount.**
Only `P[0,0]` is divided by d. The talent cross-covariances `P[0,1]` and `P[0,2]` are not. The
implied talent process-noise variance is `P⁻[0,0]·(1/d − 1)`. It is multiplicative in the current
talent variance, so during an absence the talent variance grows geometrically by `1/d` per
predict step: QB ×1.0526, RB ×1.1765, WR/TE/default ×1.1111. The engine spec states the discount
exactly as implemented (proposed — DR-C10; reconcile-code-first C17). Any switch to the full-P form
is a model change and requires recalibration.

### 4.3 One-step predictive (`reference/python/backend/grid/statespace.py:206-219`)

```text
r_mult_w = post_event_r_mult  if g < post_event_games  else 1.0
R_w      = r_mult_w · r_scale / max(snaps_w, 1.0)
m_w      = H·x⁻_w
S_w      = H·P⁻_w·Hᵀ + R_w
```

This is computed every week, including did-not-play weeks. There, `snaps_w = 0`, so R comes from
the floor (`R = r_mult·r_scale`). That makes `S` on a missing week dominated by an arbitrary R.
In the golden, `var_total_pred` is 0.404 and 0.406 at the two injury weeks, against about 0.009–0.06
elsewhere. §8.2 D-9 covers this.

`S_w` uses the **realized** `snaps_w`, so it is the predictive **conditional on realized exposure**.
An ex-ante forecast needs projected exposure (§9, DR-D16).

### 4.4 Update step (`reference/python/backend/grid/statespace.py:221-233`)

```text
if played[w] and isfinite(y[w]):
    K     = P⁻_w·Hᵀ / S_w
    x⁺_w  = x⁻_w + K·(y_w − m_w)
    P⁺_w  = (I − K·H)·P⁻_w·(I − K·H)ᵀ + K·R_w·Kᵀ        # Joseph form (PR #53 C1)
    g     ← g + 1
else:
    x⁺_w = x⁻_w ;  P⁺_w = P⁻_w                           # predict-only; variance grows
```

**Event-counter ("rust") semantics.**

- `g` is set to 0 inside the predict step of an intervention week.
- `g` is incremented only on updated (played) weeks.
- R is therefore inflated (×`post_event_r_mult`) on the first `post_event_games` **played** weeks
  counted from the intervention week.
- If the intervention week itself is missing, the inflation waits until the player plays.
- A second intervention resets `g`.

### 4.5 RTS smoother (`reference/python/backend/grid/statespace.py:237-245`; PR #53 C2)

```text
x^s_{W−1} = x⁺_{W−1};   P^s_{W−1} = P⁺_{W−1}
for w = W−2 … 0:
    P̃       = P⁻_{w+1} + 1e-10·I                         # jitter, used in BOTH lines below
    C_w     = P⁺_w·Fᵀ·P̃⁻¹                                # computed as solve(P̃ᵀ, (P⁺_w·Fᵀ)ᵀ)ᵀ
    x^s_w   = x⁺_w + C_w·(x^s_{w+1} − x⁻_{w+1})
    P^s_w   = P⁺_w + C_w·(P^s_{w+1} − P̃)·C_wᵀ
```

Interventions affect the smoother only through the stored `P⁻`. There is no explicit symmetrization.

### 4.6 Incremental step, scheme reset and changepoint detection

**`kalman_step`** (`reference/python/backend/grid/statespace.py:365-411`) is vectorized over players.
It applies the same predict, predictive and Joseph update with these differences:

1. **Intervention inflation.** `σ⁻[0,0] /= d_steady`, then `σ⁻[0,0] *= d_steady/d_spike` (`:368, :374`).
   This is algebraically `/d_spike`, but not bitwise equal to the batch `/d_spike`.
2. **Post-event R multiplier ("rust").** `r_mult = post_event_r_mult` **only in the call where the player is in
   `interventions`** (`:398, :405`). There is no event counter, so the post-event inflation lasts one
   week instead of `post_event_games` played weeks (KI-NEW-S1).
3. **Scheme reset** (`scheme_resets`, Phase-4 Task 10; `:381-389`). It is applied after predict and
   before update:

   ```text
   μ⁻[2] = 0
   Σ⁻[2,2] = scheme_reset_var    (assignment)
   Σ⁻[2,:2] = Σ⁻[:2,2] = 0
   ```

   Talent and form are untouched. When a player has both an intervention and a reset, the reset
   **overwrites** the intervention's `+scheme_reset_var` on `Σ⁻[2,2]`. The talent and form spikes
   remain. The test name "combined effect is additive" (`tests/grid/test_coaching_changes.py:169-176`)
   is therefore accurate only for talent and form.
4. **Predictive variance** `pred_var_i = H·Σ⁻_i·Hᵀ + r_mult_i·r_scale/max(snaps_i,1)` is taken from the
   **post-reset** prior covariance (`:391-399`).
5. **No default look-ahead `x0`.** State comes from `KalmanState.init`: `μ = 0`,
   `Σ = diag(0.05, 0.02, 0.01)` (`:47-51`).

**Interventions in the batch path vs scheme resets.** An intervention spikes d, adds `5·q_form`, and
also adds `scheme_reset_var` to `P[2,2]` (`:201-204`). An explicit scheme reset zeroes the
scheme_fit mean and its cross-covariances (`:381-389`). The batch path has no scheme-reset input.
Both behaviours are part of the model (cn-docs §14 item 9).

**`detect_changepoints`** (`reference/python/backend/grid/statespace.py:261-326`; Phase-4 Task 11,
CN `c1b6748`):

```text
μ⁻_i = F·μ_i
Σ⁻_i = F·Σ_i·Fᵀ;  Σ⁻_i[0,0] /= d_steady;  Σ⁻_i[1,1] += q_form;  Σ⁻_i[2,2] += q_scheme
R_i  = r_scale / max(snaps_i, 1)                       # r_mult always 1
z_i  = (obs_i − H·μ⁻_i) / sqrt(H·Σ⁻_i·Hᵀ + R_i)
flag i  iff  obs_i is not NaN and |z_i| > z_thresh     (z_thresh = 3.0)
```

The detector uses steady-state predict with no interventions and no scheme resets. `use_cusum` is
reserved and inert (`:294-295`).

In production wiring, auto-flags are unioned with manual interventions **and applied in the same
week's `kalman_step`**, per position, with that position's `SSParams`
(`reference/python/backend/pipeline/weekly_update.py:377-394`). The commit message's claim that
detection "S/z match the update params exactly" holds only for players without an intervention
or a reset.

### 4.7 Initial state and priors (as implemented)

```text
x0 (batch default) = [nanmean(y[0:3]), 0, 0]    (0 if y[0:3] has no finite value)   statespace.py:182-187
P0 (batch default) = diag(0.05, 0.02, 0.01)                                          statespace.py:188
state init (incremental) μ = 0, Σ = diag(0.05, 0.02, 0.01)                           statespace.py:47-51
```

The batch default `x0` reads weeks 1–3, so the filtered and predictive values for weeks 0–2 use future
observations (KI-#15). Cross-league priors are **not** wired into either path (KI-#15, KI-NEW-P2;
`cross-league-priors.md` §6). The engine form is in §6.5.

### 4.8 Constants and hyperparameters

Provenance key:

- **v0**: the original upload `6b0eeee` (2026-06-21), written in a Claude.ai sandbox with no
  calibration record. Values verified against the v0 `statespace.py`, which is two-component.
- **hand-set**: chosen without a recorded derivation.

| Symbol / field | Value | Defined at | Provenance |
|---|---|---|---|
| `N_STATE` | 3 | `statespace.py:31` | Phase-4 Task 8 (PR #56); v0 had 2 |
| φ `phi` (form AR(1)) | 0.50 | `statespace.py:138` | v0, hand-set |
| φ_s `phi_scheme` | 0.985 | `statespace.py:139` | Phase-4 Task 8 (PR #56), hand-set |
| `d_steady` (default) | 0.90 | `statespace.py:140` | v0, hand-set |
| `d_spike` | 0.70 | `statespace.py:141` | v0, hand-set |
| `q_form` | 0.0008 | `statespace.py:142` | v0, hand-set |
| `q_scheme` | 0.0002 | `statespace.py:143` | Phase-4 Task 8 (PR #56), hand-set |
| `scheme_reset_var` | 0.04 | `statespace.py:144` | Phase-4 Tasks 8/10 (PR #56), hand-set |
| `r_scale` (default) | 0.40 | `statespace.py:145` | v0, hand-set |
| `post_event_r_mult` | 2.0 | `statespace.py:146` | v0, hand-set |
| `post_event_games` | 2 | `statespace.py:147` | v0, hand-set |
| intervention form spike | `5·q_form` (literal 5) | `statespace.py:202, :375` | v0, hand-set; not an `SSParams` field |
| snaps floor | 1.0 | `statespace.py:215, :320, :399, :406` | v0 |
| `games_since_event` init | 99 | `statespace.py:192` | v0 |
| P0 / init Σ | diag(0.05, 0.02, 0.01) | `statespace.py:50, :188` | v0 (0.05, 0.02) plus Task 8 (0.01); hand-set, synth units (KI-NEW-P4) |
| default x0 | `[nanmean(y[:3]), 0, 0]` | `statespace.py:186` | v0; look-ahead defect KI-#15 |
| RTS jitter | 1e-10·I | `statespace.py:241` | PR #53 C2 (v0 used `inv` without jitter) |
| `z_thresh` | 3.0 | `statespace.py:266` | Phase-4 Task 11, hand-set |
| legacy-migration scheme variance | 0.01 | `statespace.py:71` | Phase-4 Task 9, hand-set; not ported (§10) |

**Per-position overrides.** `SSParams.from_position(pos)` (`statespace.py:121-133, :149-152`) is
PR #53 W7. Any label not in the table silently gets the defaults.

| Position | `d_steady` | `r_scale` | Provenance |
|---|---|---|---|
| QB | 0.95 | **0.55** | `d` hand-set (PR #53). `r_scale` was hand-set to 0.35 in PR #53, then **calibrated to 0.55 in PR #66 on the legacy synthetic world** (KI-NEW-Y0, critic G-1 (c)) |
| RB | 0.85 | 0.45 | hand-set (PR #53) |
| WR | 0.90 | 0.40 | hand-set (PR #53) |
| TE | 0.90 | 0.40 | hand-set (PR #53) |
| DEF | 0.95 | 0.55 | hand-set (PR #53). Synthetic defenders only; out of engine scope (§2) |
| any other label | 0.90 | 0.40 | silent default. Real nflverse labels such as `FB` fall here (reconcile-code-first §1.5) |

All scale constants (P0, q's, r_scale, `scheme_reset_var`) are in **synthetic** dV units. On real
2023 data, RAPM rating SD is about 0.04 at every position, against 0.26 for QBs on synth (KI-NEW-P4).
They MUST be re-derived per data version before any real-data use (proposed — DR-B4, DR-C10).

### 4.9 Constraints (oracle validates none of them)

The engine MUST validate the following and raise a typed error (§8.3) when one is violated. It
MUST NOT clamp.

- `0 < d_spike ≤ d_steady ≤ 1` and `0 ≤ φ < 1`.
- `0 ≤ φ_s ≤ 1`, `q_* ≥ 0`, `scheme_reset_var ≥ 0`, `r_scale > 0`, `post_event_r_mult ≥ 1`,
  `post_event_games ≥ 0`.
- P0 symmetric positive-definite.
- Every `P⁻`, `P⁺`, `P^s` symmetric PSD within tolerance; `S_w > 0`; `snaps ≥ 0`.
- Observations are finite or explicitly missing.

---

## 5. Algorithm, numerics and determinism

### 5.1 Complexity

| Operation | Cost |
|---|---|
| Batch (one player) | O(W) with 3×3 algebra; RTS adds O(W) |
| Incremental (one week) | O(n_players), with no cross-player coupling |
| `detect_changepoints` | O(n) |
| Persisted state | 12 floats per player in the oracle, plus ids |

### 5.2 Numerical behaviour (measured on the oracle)

- **Joseph form** keeps covariances symmetric and PSD over 50-week adversarial runs (snaps ∈ {1, 5, 200},
  gaps, two interventions) (`tests/grid/test_kalman_numerical.py:22-38, :65-81`).
- **RTS smoothed covariance PSD** is not guaranteed in theory (KI-G4). It was not reproduced
  in 3,000 random stress runs: minimum eigenvalue 0.0, maximum asymmetry 2e-17
  (`reference/python/tools/investigations/psd.py`).
- **Conditioning of the RTS solve.** Measured in this consolidation by instrumenting `np.linalg.solve`
  on the oracle:

  | Series | max cond(P̃) |
  |---|---|
  | Golden focus-QB series | 34.6 |
  | Adversarial series (`test_kalman_numerical.py:11-19`) | 81.9 |
  | Near-singular test (`:53-62`: d=0.999, q_form=1e-8, r_scale=0.001, snaps=1000) | 3.3e5 |

### 5.3 Determinism and seed policy

The filter, smoother and detector use **no randomness**. Given identical inputs the oracle is
deterministic. Without interventions and with the same `x0`/`P0`, `kalman_step` reproduces
`kalman_two_component`'s filtered totals **bitwise**: measured max |Δ| = 0 on the `kparity`
series. The seeds that matter are upstream, in the Layer-1 cross-fit (`layer1-credit.md`).

The engine MUST:

- fix the floating-point operation order of the 3×3 algebra;
- parallelize only across players, with no cross-player reductions (engine-spec §8.19).

### 5.4 Failure behaviour (oracle)

The oracle raises no typed errors:

- `W = 0` raises `IndexError` at `xs[-1]`.
- NaN in `snaps` or in parameters propagates silently.
- `r_scale ≤ 0` can make `S ≤ 0`.
- ±inf observations poison `kalman_step` but are skipped as missing by the batch filter.

The required behaviour is in §8.3.

---

## 6. Incremental and online behaviour

### 6.1 As implemented: two filter cores

| Core | Used by |
|---|---|
| `kalman_two_component` (batch filter plus smoother) | golden, Tier 0, calibration, `features.assemble_smoothed_talent`, verdict `smoothed_talent` |
| `kalman_step` (incremental) | `weekly_update.run` only |

### 6.2 State persistence (`KalmanState`, `reference/python/backend/grid/statespace.py:34-118`)

**Save** (`:53-67`):

- `np.savez` of `mu`, `sigma`, `player_ids` (unicode array) and `state_version = 3`.
- Default path `data/cache/kalman_state.npz`, relative to the **current working directory** (`:28`).
  The path is injectable (PR #63).
- The write is non-atomic, in place.
- `SSParams`, `games_since_event` and any (season, week) key are **not** persisted.

**Load** (`:85-118`):

| File state | Result |
|---|---|
| File absent | `None` |
| `mu` width 3 | Loaded as-is, even without `state_version` (Task 9 fix, PR #56) |
| `mu` width 2 | Migrated: zero scheme mean, `Σ[2,2] = 0.01`, zero cross-covariances (`:69-83`) |
| Any other width | `None` |
| 1-D `mu` | `IndexError` at `mu.shape[1]` (KI-A8 residual) |

`state_version` is written but never read. Sigma shape, id count and finiteness are not
checked. `allow_pickle` stays False (PR #63 review).

**Production wiring** (`reference/python/backend/pipeline/weekly_update.py:292-409`):

1. `KalmanState.load()` from the default path. `None` means **silent re-initialization of every
   player**. PR #56 calls "corrupt → reinit" deliberate.
2. If `player_ids` differ, the state is rebuilt: carried players keep their state, new players get
   init state, and **players absent from this week's column universe are dropped from the saved
   state** (`:296-305`).
3. The observation is the cumulative RAPM coefficient for every player, including non-players, with
   `snaps = 1` (KI-NEW-W3, KI-#24).
4. Coaching changes become scheme resets (`:313-350`): `coaching_changes` rows with
   `season = S AND week_effective = W` map HC/OC to the team's QB/RB/WR/TE and DC to DEF.
5. For each position: `detect_changepoints`, union with manual interventions, then
   `kalman_step(..., return_pred=True)`.
6. `ks.save()` (`:398`), then the trajectory upsert with `var_total = pred_var` (PR #63; `:105-170, :409`).

The accumulators have no season key, so week 1 of a new season is skipped forever (KI-NEW-W1). On
real data the changepoint z cannot exceed about 0.05 with `snaps = 1`, so auto-interventions never
fire (KI-#24).

### 6.3 Measured batch-vs-incremental divergence (KI-NEW-S1)

Measurements are with identical `x0 = 0` and `P0 = diag(0.05, 0.02, 0.01)`. Batch and incremental
are identical until the intervention week, then diverge:

| Scenario | max \|Δ total_filt\| | max \|Δ S\| | Source |
|---|---|---|---|
| `kparity`: W=12, y=0.1+N(0,0.05²) seed 0, snaps 40, intervention at 5, `SSParams()` | 0.00934 | 0.0100 | `reference/python/tools/investigations/kparity.py`; re-measured |
| Same, snaps 10 | 0.01046 | 0.0400 | this consolidation |
| Unknown scenario (script not preserved) | 0.0195 | 0.0229 | reconcile-code-first C15 |
| Golden focus-QB weekly credit (legacy synth), intervention at 9, `SSParams()` | **0.0580** | 0.00404 | this consolidation |
| Same, `SSParams.from_position("QB")` | 0.0585 | 0.00556 | this consolidation |
| Any of the above **without** interventions | 0 (bitwise) | 0 | this consolidation |
| Batch with default look-ahead `x0` vs incremental zero init, no intervention, `kparity` series | 0.0136 (week 0) | 0 | this consolidation |

The divergence is scenario-dependent. On the canonical trajectory it is about 0.06 dV/snap.

### 6.4 Required engine behaviour (proposed — DR-C10)

1. **One filter core.** Batch filtering MUST be a left fold of the incremental step, and
   `games_since_event` MUST be part of the persisted per-player state. The **batch semantics**
   ("rust" R inflation for `post_event_games` played weeks) are the target. `kalman_step`'s one-week
   inflation is not ported.
2. **Observation.** Weekly credit with R from real exposure (§3.1). Cumulative RAPM coefficients MUST
   NOT be fed as observations.
3. **Initial state.** `x0`/`P0` come from a prior (§6.5). They MUST NOT be computed from any observation
   of the same series.
4. **State record.** The persisted state is keyed by (player, season, week). It holds
   `x`, `P`, `games_since_event`, the `SSParams` version, the prior provenance and the input-data version.
   Storage is SQLite with bulk artifacts registered in a manifest (proposed — DR-C15). Writes MUST be
   atomic: a new version, never an in-place overwrite (final-build-spec §12.3, §12.4 (superseded)).
5. **Time index.**
   - The filter advances exactly one predict step per NFL game-week slot on the player's timeline.
   - A run with no newly completed game-week MUST NOT advance the filter. This is the
     final-build-spec inventory key finding 4: per-day prediction would inflate talent variance by
     `1/d` per day.
   - Bye weeks and absences are predict-only slots.
   - Catch-up MUST process each missed week in order.
6. **Player universe.** A player leaving the as-of universe MUST NOT lose his state. State is carried
   forward and simply not updated.
7. **Corrections.** A corrected observation for an earlier week MUST trigger a recompute from that
   week forward (KI-A9). In production this is bounded by fixed-lag smoothing (§6.4.1).

#### 6.4.1 Fixed-lag smoothing (new; not in the oracle)

Final-build-spec §11.5 (superseded) requires a fixed-lag mode. With lag L at time t, the fixed-lag
backward pass over indices `t−L … t` MUST equal full RTS over observations `0 … t` at those indices.
This holds exactly, because the backward recursion uses only the stored `x⁺`, `P⁺`, `x⁻`, `P⁻`.

Indices `< t−L` keep the smoothed value last computed while inside the window (`x^s_{k|k+L}`).
L is a hyperparameter to be fixed in this spec before P2-03 starts (open — DR-C5). Parity is in §10.3.

### 6.5 Initial state from priors (engine form; proposed — DR-C10, DR-C9)

```text
x0 = [μ_prior, 0, 0]
P0 = diag(σ²_prior, P0_form, P0_scheme)
```

- `(μ_prior, σ²_prior)` come from `cross-league-priors.md` for players eligible for a feeder
  prior.
- Otherwise they come from a position/draft population prior estimated as-of from the window
  (superseded alpha-spec §6.3 `θ_position_draft_prior`).

This makes the update the dynamic form of alpha-spec §6.3's EB posterior with `n0 = R/σ²_prior`
(reconcile-spec-first R40).

Two facts constrain the choice:

- **Scale.** The equivalency target in the oracle is the RAPM **rating**. The Kalman state is on the
  **Layer-1 credit** scale. These are not the same scale. On the legacy synth the per-position slope of
  season credit on rating is 1.19–1.40, with credit SD 1.3–1.8× rating SD (§7.3). The prior MUST
  be expressed on the Kalman state's scale (DR-D19).
- **Washout speed.** `P0_form` and `P0_scheme` (oracle 0.02 and 0.01) dominate how fast a talent prior
  washes out. They are statistical-owner parameters, not defaults
  (`cross-league-priors.md` §4.5).

### 6.6 Three-season window (proposed — DR-C6)

Engine-spec §2.4 (superseded alpha-spec §2.4) limits evidence to S−2…S (W > 1) or S−3…S−1
(preseason and week 1).

**As implemented.**

- The batch path smooths whatever series the caller passes. The verdict passes the pre-first-origin
  window (`reference/python/backend/validation/verdict.py:127-233`).
- The timeline is the sorted set of (season, week) slots (`:194-203`). A season boundary is therefore
  **one ordinary predict step**, with no offseason transition.
- The production state carries indefinitely, with no window and no season key.

**Proposed default (DR-C6).** In production the Kalman carries forward with its discount. On
backtest replay it is re-initialised from the windowed prior.

Whatever is ratified, production and replay MUST produce the same state for the same as-of
(two-path equivalence; `evaluation-and-leakage.md`). Under the proposed default that holds only if
production also re-initialises at the same boundary. The season-boundary transition itself is
undefined: the number of predict steps across the offseason, or a separate offseason discount
(DR-D17).

---

## 7. Validation evidence

Every synthetic number below that is labelled **legacy** was measured on the oracle's legacy generator.
That generator draws defenders from the offense's own team (KI-NEW-Y0; critic G-1). Legacy numbers are
history, not Rust targets. **Defender-fixed** values come from the one-line generator fix (critic
`scratch-critic/cnfix`; `reference/python/tools/investigations/_common.py` `apply_defender_fix`).

All runs use `OMP/OPENBLAS/MKL_NUM_THREADS=1`, `load_synthetic()`, `fit(n_iter=3)` and market seed 1.
Every value was re-measured in this consolidation and matches critic G-1 where both exist.

### 7.1 Tier-0 state-space gates

The focus QB has a planted rise, misses weeks 8–9, and returns at index 9 with intervention `{9}`.
The fixture uses **`SSParams()`, not QB params**.

| Metric | Gate (`tests/grid/test_tier0_recovery.py`) | Legacy | Defender-fixed |
|---|---|---|---|
| corr(total_smooth, planted) | ≥ 0.92 (`:105`) | 0.9580 | 0.9760 |
| corr(tau_smooth, planted) | ≥ 0.60 (`:106`) | 0.6745 | 0.6538 |
| corr(total_filt, planted) | none | 0.9867 | 0.9323 |
| var_total_filt healthy week 3 → return week 10 | must increase (`:113-116`) | 0.00228 → 0.00614 | 0.00233 → 0.00738 |
| focus-QB predictive NIS (n=12, includes week 0) | ≤ 10.0 (`:128`) | 4.581 | **8.371** |
| focus-QB NIS with QB params (ungated) | — | 4.825 | 8.470 |

The NIS gate only catches blow-ups. The filter is about 4.6× overconfident on this trajectory under
default params (KI-NEW-S2).

**What the planted truth can test.** The truth is a single current-ability arc, so `tau_smooth`
correlation measures smoothed talent against a series that contains a transient dip. No τ/f/s
component is planted separately (§1).

### 7.2 Synthetic calibration gates (Tier 0.5; PR #66)

Setup (`tests/grid/test_calibration_synth.py`):

- The QB params `SSParams.from_position("QB")` are used.
- Residuals come from a GroupKFold-by-week cross-fit.
- 24 QBs, 310 QB-weeks.
- Week 0 is excluded (`:97`).

| Metric | Gate | Legacy | Defender-fixed |
|---|---|---|---|
| median per-QB NIS | [0.8, 1.25] (`:117`) | 0.9266 | 1.2198 (passes; margin 0.03) |
| pooled NIS | [0.8, 1.4] (`:124`) | 1.2369 | **1.8289 (FAILS)** |
| PIT mean | \|·−0.5\| < 0.07 (`:129`) | 0.5158 | 0.5145 |
| PIT sd | [0.24, 0.34] (`:136`) | 0.2956 | 0.3112 |
| PICP@80 | [0.70, 0.90] (`:143`) | 0.7774 | 0.7452 |

**Exploratory QB `r_scale` sweep.** `d_steady = 0.95`, same fixture, pooled / median / PICP@80.
This is **not a calibration and not a proposed value**. It is input to the DR-B4 recalibration.

| r_scale | Legacy | Defender-fixed |
|---|---|---|
| 0.35 | 1.784 / 1.351 / 0.681 | 2.644 / 1.753 / 0.642 |
| 0.55 | 1.237 / 0.927 / 0.777 | 1.829 / 1.220 / 0.745 |
| 0.75 | 0.951 / 0.707 / 0.839 | 1.404 / 0.936 / 0.800 |
| 0.85 | 0.854 / 0.632 / 0.868 | 1.259 / 0.839 / 0.826 |
| 1.00 | 0.741 / 0.546 / 0.881 | 1.091 / 0.726 / 0.852 |

Two inconsistencies in the oracle's own comments:

- The in-code comment at `statespace.py:122-125` says 0.35 gave "NIS≈1.6, PICP@80≈0.71". That
  **does not reproduce at `59bce1d`**: it gives 1.784 / 0.681. The 0.55 figures do reproduce.
- The test docstring says pooled NIS "observed ~1.21" (`test_calibration_synth.py:121-122`), while
  the module comment says ≈1.24. Measured: 1.2369.

### 7.3 Other evidence

| Evidence | Result |
|---|---|
| Golden master (`tests/grid/test_golden_master.py`) | **Layer A:** talent is the most persistent component (`:88-92`); injury dip and variance widening (`:95-105`). **Layer B:** steepest filtered drop at index 8 (`:122-128`). **Layer C:** `k_total_filt`, `k_total_smooth`, `k_tau_smooth`, `k_var_total_filt`, `k_total_pred` and `k_var_total_pred` at rtol 1e-5 / atol 1e-6, single-threaded (`:168-173`). The Layer-C arrays include the KI-#15 look-ahead and the legacy synth. Under the defender fix, `test_layerC_qb_weekly_and_kalman` fails (critic G-1) |
| Additive-change baseline (`tests/grid/test_phase0_prereqs.py:56-119`) | Five legacy keys frozen at rtol 1e-12 for a benign series (seed 11, W=18, intervention {6}). This is the most precise existing closed-form fixture |
| Unit tests (102 tests, all pass on Linux at `59bce1d`) | `test_kalman_numerical.py` (7), `test_incremental.py` (20 Kalman plus 5 accumulator), `test_changepoint.py` (5), `test_phase0_prereqs.py` (9), `test_coaching_changes.py` (4: 2 kalman_step, 1 schema, 1 DB-coupled `weekly_update`), `test_calibration_synth.py` (6), `test_tier0_recovery.py` (8), `test_golden_master.py` (11), `test_priors.py` (15), `tests/projection/test_features.py` (12) |
| 3-state vs 2-state regression | `test_two_component_recovery_unchanged` (`test_incremental.py:407-438`): baseline 0.7272, measured 0.7543 |
| Changepoint | Unit tests only, in z-score units. `test_weekly_update_auto_interventions_recovers_demo` calls `kalman_step` and `detect_changepoints` directly, not `weekly_update`. The planned changepoint precision/recall target (≥ 0.6 / ≥ 0.5; validation plan §9) was never implemented (reconcile-code-first C25) |
| Look-ahead effect (KI-#15) | Default `x0` vs `x0 = 0` on the golden focus-QB series (legacy, measured on the pre-L0 golden under the AVX-512 kernel): Δ`total_pred[0]` = 0.1776 (that is `x0` itself; 0.1720 on the L0 golden); Δ`total_filt` = 0.0095 at week 0, decaying to about 3e-4 by week 13; total_smooth corr 0.9580 vs 0.9571, tau 0.6745 vs 0.6738, NIS 4.581 vs 4.694 |
| Scale of observation vs RAPM rating | Legacy synth, snap-weighted season credit regressed on RAPM rating: QB slope 1.217 (credit SD 0.349 vs rating 0.272), RB 1.399, TE 1.283, WR 1.189. Correlations 0.66–0.95 |

### 7.4 Real-data status

- The filtered Kalman state has **never** been scored as a forecast on real data. The weekly forecast
  is `sv_to_points(RAPM rating)`, and ROS uses frozen pre-origin `smoothed_talent`
  (reconcile-spec-first C3; KI-NEW-R3).
- The post-#91 H1 re-run with `smoothed_talent` was never recorded (cn-docs §4).
- The real-data numbers are historical, non-parity (critic G-7, X-14).

---

## 8. Known defects and required engine behaviour

### 8.1 Registered defects (`docs/00-meta/known-issues.md`)

| KI | Defect (oracle) | Required engine behaviour |
|---|---|---|
| KI-#15 | Batch default `x0 = nanmean(y[:3])` looks ahead (`statespace.py:186`). Priors are not wired into either path | The Rust port MUST NOT reproduce the look-ahead. `x0`/`P0` come from §6.5. Parity fixtures pass `x0` explicitly (§10.3) |
| KI-NEW-S1 | Batch and incremental cores diverge after interventions. `games_since_event` is not persisted (§6.3) | One core; persisted `games_since_event`; batch == fold(incremental) (§6.4) |
| KI-NEW-W3 | `weekly_update` observes cumulative RAPM for every player, including non-players | MUST NOT reproduce. Weekly credit, NaN for non-play |
| KI-#24 | `snaps = 1` makes R 30–60× too large on real scale, so changepoints never fire | MUST NOT reproduce. R from real exposure |
| KI-NEW-W1 | No season key on state or accumulators | State keyed by (season, week) |
| KI-A8 | 1-D state gives `IndexError`. Unknown width returns `None`, and the caller silently re-initialises (deliberate per PR #56) | Typed `StateSchemaMismatch` / `StateCorrupt`; no silent re-init (proposed — DR-B6). The ADR records that it reverses PR #56's "corrupt → reinit" design |
| KI-G4 | RTS `P^s` PSD not guaranteed; no symmetrization | Symmetrize every stored covariance. A typed `NonPsdCovariance` if the minimum eigenvalue is below −1e-12 relative to the trace |
| KI-NEW-S2 | Tier-0 and golden use `SSParams()` for the focus QB. The NIS gate (≤ 10) is far from calibrated | Re-gate on the fixed synth with the parameters actually shipped (DR-B4) |
| KI-NEW-P4 | Scale constants are synth-unit, 4–5× too wide for real data | Data-derived scale per data version, recorded with the model version |
| KI-NEW-Y0 | QB `r_scale = 0.55` and the NIS bands were calibrated on the defender-bug synth | Recalibrate on the fixed synth before freezing (DR-B4) |
| KI-NEW-D1 | `backend/db/data/coaching_changes_2025.json` holds wrong or implausible rows | Never a fixture or provider input. Coaching changes come from a sourced provider contract with knowledge timestamps (engine-spec §4.3) |
| KI-NEW-P3 | The `__main__` self-test uses bare `from synth import …` and fails under `python -m` (`statespace.py:419-448`) | DROP; not ported |
| KI-#46 | R and d are not time-varying (enhancement) | Research backlog. Do KI-NEW-S1 and KI-NEW-S2 first |

### 8.2 Defects found in this consolidation (to be registered)

| # | Defect | Required engine behaviour |
|---|---|---|
| D-1 (KI-NEW-Z18) | ±inf observations are treated as missing by the batch filter (`isfinite`, `:222`) but poison `kalman_step` (`isnan`, `:403`) | Typed `NonFiniteObservation`. Only an explicit missing marker means missing |
| D-2 (KI-NEW-Z19) | Unknown position labels silently get default parameters (`:149-152`) | Explicit position-class map, QB/RB/WR/TE only; typed `UnknownPositionClass` |
| D-3 (KI-NEW-Z15) | On a universe change, `weekly_update` drops the state of players not in the current universe (`weekly_update.py:296-305`) | Carry state forward (§6.4 item 6) |
| D-4 (KI-NEW-Z16) | The persisted state lacks `SSParams`, `games_since_event`, (season, week) and data version. `state_version` is never read. The default path is CWD-relative and the write non-atomic | §6.4 item 4 |
| D-5 (KI-NEW-Z20) | No input or parameter validation (§4.9); `W = 0` gives `IndexError` | Typed `InvalidParams`, `EmptySeries` |
| D-6 (KI-NEW-Z21) | Scheme reset overwrites (does not add to) the intervention's scheme inflation (§4.6 item 3). The test name says "additive" | Specify explicitly in the engine. Default to the oracle semantics: the reset sets `Σ[2,2] = scheme_reset_var` |
| D-7 (KI-NEW-Z22) | Intervention inflation is computed as `/d_steady·(d_steady/d_spike)` incrementally and `/d_spike` in batch, which is not bitwise equal | One core computes `/d_w` once |
| D-8 (KI-NEW-Z17) | Auto-detected changepoints are applied in the **same** week, so the persisted predictive `S` for that week (`pred_var`, written to the trajectory) is computed **after** looking at `y_w`. The band is inflated exactly when \|z\| > 3, which biases NIS and coverage. Dormant on real data because of KI-#24 | The published one-step predictive for week w MUST be the pre-detection `(m_w, S_w)`. Whether a detected changepoint alters the week-w update or only week w+1 is open (DR-D18) |
| D-9 (KI-NEW-Z23) | Predictive `S` on did-not-play weeks uses `R = r_scale` from the snaps floor (golden 0.404/0.406) | Do not publish a predictive for a did-not-play week as if it were a forecast of an observation. Mark it not-applicable, or condition on projected exposure (DR-D16) |
| D-10 (KI-NEW-Z26) | The washout narrative "wide prior → fast washout" is computed with a conjugate one-component formula and an obs variance (`run_demo.py:91`, 0.12²) that is inconsistent with `SSParams`. The actual filter washes out the talent prior far more slowly for RB/WR/TE (`cross-league-priors.md` §4.5) | Diagnostic derived from the actual filter |

### 8.3 Typed failures (proposed — DR-B6)

The Rust component MUST return typed errors and MUST NOT fall back silently:

- `InvalidParams`
- `EmptySeries`
- `NonFiniteObservation`
- `NonFiniteState`
- `NonPositiveInnovationVariance` (S ≤ 0)
- `NonPsdCovariance`
- `UnknownPositionClass`
- `StateSchemaMismatch` / `StateCorrupt` (load)
- `PlayerUniverseMismatch` (obs and state misaligned)
- `TimeIndexViolation` (non-monotone or duplicate (season, week))

Each failure blocks promotion of the run (engine-spec §8.8). These are deliberate divergences
from the oracle, listed in `reference/python/PARITY.md`.

---

## 9. Open decisions

| Decision | Proposed default | Effect on this spec |
|---|---|---|
| DR-C10 Kalman semantics bundle | Weekly credit observation with real exposure; one core with persisted `games_since_event`; prior-based `x0`/`P0`; state keyed by (season, week); component discount documented as implemented | §3.1, §4.2, §6.4, §6.5 (proposed) |
| DR-C1 Live observation | Participation-free Layer-1′ credit in-season. Participation RAPM offseason only | §3.3 |
| DR-C2 / DR-C3 Role of GRID | Signal provider. Role-specific talent as Layer-D covariates | §2. Whether the scalar state splits by role (dropback / carry / target) is open under DR-C3 |
| DR-C6 Three-season window | Carry with discount in production; re-initialise from the windowed prior on replay | §6.6, plus the two-path equivalence requirement |
| DR-C5 Gates and open parameters | PB-MAE primary. FB's open parameters (fixed-lag L, auto-rollback thresholds) set in model specs before code | §6.4.1 L; NIS bands |
| DR-C9 Prior form | `cross-league-priors.md` | §6.5 |
| DR-C15 Storage | SQLite source of truth plus a manifest of bulk artifacts | §6.4 item 4 |
| DR-B1 Oracle correction ledger | Synth defenders → convention → grade sign → **causal Kalman init** → ingest | Legacy goldens are audit-only (§10.3) |
| DR-B3 Parity tolerances | Classes A / A′ / B / C / D | §10.3 |
| DR-B4 Synthetic world | Fix defenders, re-calibrate QB NIS bands, plant τ/f/s separately, add a "realistic" profile | §7, §1 caveat |
| DR-B6 Typed failures vs faithful port | Typed failure; oracle unchanged; divergences in `PARITY.md` | §8.3 |
| **DR-D17** | — (open) | Transition across the offseason: number of predict steps, an offseason discount, or re-init (§6.6) |
| **DR-D16** | — (open) | Which `S` is published ex-ante (projected snaps) vs scored ex-post (realized snaps); conditional vs unconditional calibration (validation plan §7) |
| **DR-D18** | — (open) | Same-week vs next-week application; `z_thresh`; precision/recall gate; pre-detection predictive (D-8) |
| **DR-D19** | — (open) | Prior expressed on the Kalman credit scale; `P0_form` / `P0_scheme` for prior-seeded players (§6.5) |

---

## 10. Rust port plan

### 10.1 Homes

| Concern | Crate | Module or artifact |
|---|---|---|
| Generic Kalman primitive (predict, predictive, Joseph update, full RTS, fixed-lag RTS), generic over a static state dimension. `SMatrix<f64, N, N>` (final-build-spec §11.1 (superseded)) | `crates/models` (`grid-models`) | `statespace::{kalman, rts, fixed_lag}` |
| GRID 3-state instance: position-class parameters, interventions, scheme reset, changepoint detector | `crates/models` | `statespace::grid` |
| Persisted state records and versioning | `crates/persistence` (SQLite) | — |
| Calibration metrics (NIS, PIT, PICP) | `crates/evaluation` | — |
| Weekly orchestration | `crates/application` → `pipeline` plus a `grid-cli` binary (DR-A8) | — |
| Synthetic fixtures | `synth` crate (DR-A8) | Committed fixtures are **synthetic only** (DR-A11) |

The legacy two-component `.npz` migration (`statespace.py:69-83`) is **not ported**: no legacy Rust
state exists.

### 10.2 Work packages

| WP | Scope from this spec |
|---|---|
| **P1-06** Numerical primitives | Generic predict/update (Joseph), predictive (m, S), full RTS with the 1e-10 jitter, fixed-lag RTS, and typed failures. Parity: classes A and A′ below, plus the property tests ported from `test_kalman_numerical.py` |
| **P1-12** GRID component port | 3-state instance, `SSParams` by position class, intervention and post-event ("rust") counter, scheme reset, changepoint detector, persisted state (§6.4), Tier-0 Kalman gates, golden `k_*` regeneration |
| **P1-05** Feature store and leakage harness | As-of slicing of interventions with knowledge timestamps; exposure of the `smoothed_talent` / filtered features; two-path equivalence for the Kalman |
| **P1-07** Layers A–D, priors, baselines | Prior handoff (§6.5) |
| **P1-09** Backtest and metrics | Calibration gates (NIS, PIT, PICP) on the backtest; regime-conditional calibration |
| **P2-03** Advanced state updates / fixed-lag | Production fixed-lag smoothing, correction replay, time-varying R/d research (KI-#46) |

### 10.3 Parity class and concrete targets

Classes are per `docs/03-contracts/parity-fixture-contract.md` (proposed — DR-B3). Inputs are
stage-isolated: Rust receives **Python-exported weekly credit series**, so no booster sits on the
Kalman parity path.

Every fixture MUST record:

- `x0` and `P0` explicitly;
- the full `SSParams`;
- `cond(P̃)` per smoother step.

The fixture exporter computes `nanmean(y[:3])` where it reproduces legacy numbers and stores it as an
explicit `x0`. Rust therefore never implements the look-ahead.

| ID | Fixture (inputs) | Compared outputs | Class |
|---|---|---|---|
| PF-SS-01 | Benign series `test_phase0_prereqs.py` (seed 11, W=18, intervention {6}, snaps 40) with the exported `x0` | all §3.4 keys (incl. `sigma_smooth`) | **A** (≤ 1e-12 abs element-wise) |
| PF-SS-02 | Golden focus-QB credit and snaps series from `tests/grid/golden/snapshot.npz` (intervention {9}, `SSParams()`), exported `x0 = 0.17201694537356657` (the golden after ledger entry L0; the pre-L0 legacy golden gave 0.17763474167048995); and the same with QB params | all §3.4 keys | **A**. Closed-form parity on a fixed numeric series is independent of the synth's validity; the legacy *recovery* numbers are not targets |
| PF-SS-03 | Adversarial series from `test_kalman_numerical.py` (seeds 7, 13, 55, and 42 benign) | all keys plus symmetry/PSD properties | **A** |
| PF-SS-04 | Near-singular series (`test_kalman_numerical.py:53-62`; cond 3.3e5) | smoother outputs | **A′** (≤ 1e-9 rel); filter outputs **A** |
| PF-SS-05 | Incremental sequences: `test_incremental.py` seeds 77/99, `test_coaching_changes.py` two-player states (reset, intervention, both), changepoint states from `test_changepoint.py` | μ, Σ, `pred_var`, flag sets and z | **A** (flags exact) |
| PF-SS-06 | `kparity` series with and without an intervention | Rust batch == Rust fold(step) == oracle **batch** (`kalman_two_component`) | **A**. The oracle `kalman_step` with an intervention is NOT a target (KI-NEW-S1) |
| PF-SS-07 | Fixed-lag L ∈ {1, 2, 4} over PF-SS-01..03 at every t | fixed-lag vs Rust full RTS over 0…t, indices t−L…t | **A** (internal consistency; no oracle counterpart) |
| PF-SS-08 | Tier-0 focus QB and QB calibration pool on the **defender-fixed** synth after DR-B4 | total_smooth / tau_smooth recovery; variance widening; NIS, PIT and PICP bands | **D** (floors re-set on the fixed synth, calibrate-below-observed). Current fixed values: 0.976 / 0.6538 / 0.00233→0.00738; pooled NIS 1.829 fails today |

**Not targets.**

- Legacy golden `k_*` arrays and the legacy Tier-0 values (audit only, DR-B1).
- `weekly_update` outputs.
- Any real-data number.

### 10.4 Reference examples (hand-checkable; oracle values to 10 significant digits)

All three examples use one player, `KalmanState.init`, `SSParams()` and `kalman_step(..., return_pred=True)`.

**K1: observation `obs = 1.0`, `snaps = 50`.**

Predict:

- `P⁻ = diag(0.05/0.9, 0.02·0.25 + 0.0008, 0.01·0.985² + 0.0002) = diag(0.0555555556, 0.0058, 0.00990225)`
- `R = 0.008`
- `S = 0.0792578056`

Update result:

- μ = [0.7009474356, 0.0731789123, 0.1249372214]
- Σ = [[0.0166140314, −0.0040654951, −0.0069409567], [−0.0040654951, 0.0053755623, −0.0007246359], [−0.0069409567, −0.0007246359, 0.0086650904]]

**K2: same as K1, with intervention `{a}`.**

- μ = [0.4854766033, 0.0666073900, 0.3391692476]
- S = 0.1471308214

**K3: missing observation (`obs = NaN`, `snaps = 0`).**

- μ = [0, 0, 0]
- diag(Σ) = [0.0555555556, 0.0058, 0.00990225]
- S = 0.4712578056. Note the floor R = 0.40 (§8.2 D-9).

---

## 11. Superseded-spec mapping

Sections cited as "alpha-spec §x (superseded)" or "final-build-spec §x (superseded)" live in
`docs/00-meta/specs/superseded/`.

| Superseded section | What this component satisfies | Gap |
|---|---|---|
| alpha-spec §6.2 (superseded), Kalman row | "selected efficiency components": the player-efficiency latent | Pace, pass tendency and opportunity-share streams are not implemented. Each needs its own spec reusing §10.1's primitive |
| alpha-spec §6.2 (superseded), fixed-lag RTS row | — | Not implemented. Specified in §6.4.1 (P1-06 / P2-03) |
| alpha-spec §6.3 (superseded) EB posterior | With `x0`/`P0` from priors, the update is the dynamic analogue (`n0 = R/σ²`) | Not wired (KI-#15). `n0` is not learned |
| alpha-spec §6.4 (superseded), ensemble member 3 | Latent state exists | Not a validated forecast component on real data (§7.4) |
| alpha-spec §6.5 (superseded) explainability | Talent, form, uncertainty and regime flags (§3.5) | Prior contribution not computed in the oracle |
| alpha-spec §6.6 (superseded) rules 1–8 | This spec; typed failures in §8.3 | Rule 4 is violated by the oracle (§5.4) |
| alpha-spec §2.4 (superseded) three-season rule | — | Production state carries indefinitely; DR-C6 |
| alpha-spec §2.5 (superseded) low-evidence decay | A continuous Kalman washout, in principle | Prior not wired |
| alpha-spec §4.5 (superseded) leakage | Missing-week semantics, as-of interventions | Look-ahead `x0` (KI-#15); same-week changepoint predictive (D-8) |
| alpha-spec §12.1–§12.3 (superseded) tests | Unit (Joseph, RTS), golden, as-of | Golden embeds legacy defects; leakage test for intervention knowledge time missing |
| final-build-spec §11.4 (superseded) | Online `x`/`P` state; daily batch of sequential steps | Persisted as `.npz`, not SQLite; transition model not persisted; time index ambiguity resolved in §6.4 item 5 |
| final-build-spec §11.5 (superseded) | Full RTS | Fixed-lag absent (§6.4.1) |
| final-build-spec §11.1 (superseded) | — | Statically sized matrices are a Rust requirement (§10.1) |
| final-build-spec §12.3–§12.4 (superseded) snapshot, rollback, resumability | — | In-place overwrite; no rollback (§6.4 item 4) |
| final-build-spec §19.1–§19.2 (superseded) | Unit tests and golden exist | Re-baseline on the corrected oracle (DR-B1) |
