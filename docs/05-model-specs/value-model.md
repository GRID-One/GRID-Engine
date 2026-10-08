---
model-spec-id: MS-VALUE-MODEL
status: Draft            # equations below are NOT yet approved by the statistical owner
statistical-owner: statistical owner (role defined in engine-spec §1); approval pending
version: 0.1.0 (2026-10-07, written under consolidation WP P0-01)
supersedes: none. First engine-only spec for this component. Replaces the cautious-nevermore
  prose now archived (non-authoritative) under docs/07-archive/cautious-nevermore/.
---

# Model spec — Situational value V(s), play value dV, and situation masks

This is a contract written before implementation. Under the authority order in engine-spec §1.5
(DR-A2, adopted subject to owner ratification) it is an authority-level-3 document. The statistical
owner approves the equations. The implementing agent may not redefine them (superseded alpha-spec
§6.6; carried into engine-spec §6.8).

**How to read this spec**

- **Normative words.** MUST, SHOULD and MAY are requirements on the Rust engine. "The oracle" is the
  Python reference at `reference/python/` (ADR-012; engine-spec §1.7). Every
  `reference/python/...:LINE` citation uses cautious-nevermore (CN) `59bce1d` line numbers; the
  imported files are byte-identical (`reference/python/MANIFEST.tsv`: `verbatim`).
- **Proposed decisions.** "(proposed — DR-xx)" marks a default recorded in
  `docs/00-meta/decision-register.md`. The owner has not ratified it. `DR-NEW:<slug>` marks a
  decision first raised here (§9).
- **Defect IDs.** `KI-` IDs are entries in `docs/00-meta/known-issues.md`. Findings first made in
  this spec are labelled **new** and listed in §8.2 for registration.
- **Synthetic generators.** *Legacy* is the oracle's generator as imported; its defenders come from
  the offense's own roster (KI-NEW-Y0). *Fixed* is the defender-fixed generator, applied in memory by
  `reference/python/tools/investigations/_common.py:apply_defender_fix`. V(s) itself has no team or
  defense semantics, so legacy V(s) numbers are not void the way legacy team numbers are, but they
  describe the legacy world and are not Rust targets (DR-B1). See `synthetic-world.md`.
- **Measurements.** Every number marked "measured" was produced on 2026-10-07 with
  `OMP/OPENBLAS/MKL_NUM_THREADS=1`, Python 3.11.15 and the `reference/python/requirements.lock` pins
  (scikit-learn 1.9.1, numpy 2.4.6, pandas 3.0.6), against the imported oracle, read-only. Real-data
  numbers use the sha256-pinned nflverse 2023 play-by-play (`pbp_2023`, `bd348473…6776`;
  `reference/python/tools/investigations/fetch_realdata.py`). They are **historical, non-parity**
  evidence (DR-A11; engine-spec §3.3) and are never fixtures.

## 0. Template conformance

This spec follows `docs/99-templates/template-model-spec.md`, extended with numbered sections.

| Template heading | Where it is covered |
|---|---|
| Target | §1.1, §3.3 |
| Inputs | §3.1, §3.2 |
| Equations | §4.1–§4.3 |
| Priors | §4.5 (none) |
| Constraints | §4.6, §5.4 |
| Seed policy | §5.2 |
| Tolerances | §10.3 |
| Reference examples | §10.4 |
| Explanation fields | §3.4 |
| Validation and promotion | §7, §10.3 |
| Known limitations | §8, §9 |

---

## 1. Purpose and statistical intent

### 1.1 What the component estimates

| Estimand | Definition | Units / support | Status |
|---|---|---|---|
| Situational value `V(s)` | `E[drive_points | s]`: the points the offense's current drive eventually scores, given the pre-snap state `s = (down, ydstogo, yardline_100)` | expected points; the label takes values in {0, 3, 7}, so the true conditional mean lies in [0, 7] | Implemented (oracle, `reference/python/backend/grid/value.py:27-61`) |
| Play value `dV` | `V(s′) − V(s)`, offense perspective. For a drive-ending play `s′` is absorbing and its value is the realized terminal value | expected points per play; real | Implemented (`value.py:89-104`) |
| Situation masks | Named boolean predicates over the state | boolean per play | Implemented (`reference/python/backend/grid/situations.py:30-62`) |

V(s) is a **nuisance surface**, not an estimand anyone consumes directly. Its only job is to put every
play on one currency so that attribution (Layer 2 RAPM, Layer 1 credit) and the dynamic layer have a
response variable. It is not a projection, not a fantasy quantity, and not nflfastR expected points
(§1.3).

### 1.2 Statistical intent (load-bearing comments, preserved verbatim)

The Rust port MUST keep this intent.

- **A standard, honest scaffold.** `reference/python/backend/grid/value.py:1-16`:

  > "V(s) = expected net points to the offense before the drive resolves, given the game state
  > s = (down, distance, yardline). We FIT V from data (we never know true EP in reality), then value
  > each play as the change in expected points: dV(play) = V(s') - V(s) where for a drive-ending play
  > s' is an absorbing state with the realized terminal value (7 for TD, 3 for FG, 0 for punt/downs),
  > and for a continuing play s' is the next snap's state. This is deliberately a transparent,
  > standard expected-points scaffold -- the originality in GRID is in attribution (layers) and the
  > dynamic layer, not in re-inventing the value currency. We keep it honest about that."

  "Net points" in that docstring is aspirational: the implemented label never goes negative
  (§3.2, KI-NEW-V0b).

- **The state surface, not the players.** `value.py:31-32`:

  > "We train on every play's (state -> eventual drive points). Player effects average out, so the
  > model learns the state surface, which is what V is."

  This is an assumption, not an identification result: V(s) averages over the population of
  offenses that reach `s`. Better offenses reach better states more often, so V(s) is the
  league-average value of a state, exactly as in any expected-points model. Attribution of
  deviations from that average to players is the job of `rapm-attribution.md` and
  `layer1-credit.md`.

- **Situations use only state columns.** `situations.py:28`: masks use "ONLY the plays-contract
  columns: down, ydstogo, yardline_100". The clock-based `two_minute` mask is the documented
  exception (`situations.py:4-6`).

### 1.3 What V(s) is not

- **Not nflfastR EP.** nflfastR's `ep` is a signed next-score expectation that includes opponent
  scoring and the clock. V(s) has no opponent scoring, no safeties, no clock, no score and no
  timeouts (KI-NEW-V0b). Measured on real 2023 (historical): corr(V(s), `ep`) = 0.963 over 35,474
  rows, but at 1st and 10 at the offense's own 25 (2,175 rows) V(s) is 1.849 against a mean `ep` of
  1.120, a gap of +0.730 EP. The engine MUST NOT substitute `ep` for V(s): nflfastR's model was
  trained on later seasons, which is a scope leak in backtests (proposed — DR-C7).
- **Not a projection output.** dV never appears in the stat-vector contract. It appears only in
  labelled GRID explanation and diagnostic fields (engine-spec §5.5).

### 1.4 Identification facts this spec relies on

1. **Per-drive telescoping.** Within a drive, every continuing play's `V(s′)` is the next play's
   `V(s)`, and the terminal play's `s′` value is the realized terminal value. Therefore

   ```text
   Σ_{i ∈ drive} dV_i = terminal_value(drive) − V(s_first(drive))
   ```

   exactly. Measured max |violation| over all 2,016 canonical drives: 8.9e-16 (legacy and fixed).
   This requires that next-state columns equal the next row's state, which holds on both synthetic
   worlds (measured) and by construction in the nflverse adapter (`nflverse_adapter.py:110-118`).
2. **Level sensitivity sits on terminal plays only.** If V is shifted by a constant `c`, every
   continuing play's dV is unchanged and every terminal play's dV falls by `c`. The level of V(s)
   is therefore anchored by the label scale at drive ends. Any estimator change that moves the level
   of V moves dV only on terminal plays.
3. **Martingale property.** If V is the true value function of a Markov process on `s`, then
   `E[dV | s] = 0`. The real-data dV is close to state-centred (§7.4); the synthetic dV is not, for
   two generator reasons diagnosed in §7.4 and owned by `synthetic-world.md`.

---

## 2. Position in the engine

### 2.1 Signal stack (engine-spec §6.2, Components 1 and 2)

```text
plays contract (as of the fit point; REG only — DR-C12)
   │  state s, drive_points label, terminal flag/value, next state s′
   ▼
V(s) fit  ── once per season on the §2.4 window, frozen (proposed — DR-C6, DR-C7)   ◄── this spec
   │
   ▼
dV = V(s′) − V(s) per play                                                            ◄── this spec
   ├─► Layer 2 RAPM response y (rapm-attribution.md §4.1; layers.py:327)
   ├─► Layer 1 / Layer-1′ context residual target r = dV − g(s, opp) (layer1-credit.md; layers.py:496-519)
   ├─► team-level ridge on dV (engine-spec §6.2)
   ├─► EPA-type GRID features (engine-spec §11.3, §11.6), labelled as GRID dV
   └─► Layer B scoring environment
situation masks (situations.py)                                                       ◄── this spec
   ├─► situation RAPM slices (layers.py:432-484; weekly_update.py:425-480) — research-only
   └─► situational features (engine-spec §11)
```

The Layer-1 context model reuses the V(s) feature list: it is fit on `STATE_COLS` plus the on-field
opponent rating sum (`reference/python/backend/grid/layers.py:38,508`). A change to `STATE_COLS`
therefore changes Layer 1 too.

### 2.2 Oracle call sites

| Caller | Training frame | Effect |
|---|---|---|
| `run_demo.py:43`; `tests/grid/test_tier0_recovery.py:32`; `tests/grid/golden_master.py:68`; `tests/grid/test_determinism.py:33-34`; `tests/grid/test_calibration_synth.py:59` | the full canonical synthetic season | in-sample V(s) for the whole world |
| `backend/validation/backtest.py:165-168` (`walk_forward`) | the first `warmup_weeks = 4` (season, week) slots, **frozen** | every later week's dV uses that frozen model (`:192`) |
| `backend/validation/verdict.py:169-172` | the pre-origin `AsOf` slice | dV for the frozen pre-origin talent |
| `backend/pipeline/weekly_update.py:210-214` | **the whole season** (`load_grid_plays([season])`), refit on every call | leaks weeks after the target week into dV on backfill (KI-NEW-W4) |
| `backend/grid/layers.py:615-617`, `priors.py:176-181`, `statespace.py:422-427` (`__main__` blocks) | full synth | demonstration only; they import `from value import …`, which does not resolve under the package layout |

No oracle production path calls `load_value_model` or passes `cache_path`. Only
`tests/grid/test_performance.py:205-305` exercises persistence.

### 2.3 Contracts and neighbouring specs

- Inputs: `docs/03-contracts/plays-contract.md` §2 (columns), §3 (construction), §8 D-1 to D-4, D-9,
  D-14.
- Outputs: dV currency in `docs/03-contracts/engine-output-contract.md`; artifacts per engine-spec
  §8.5 and §8.8.
- Parity: `docs/03-contracts/parity-fixture-contract.md` §5 (`value-model` rows), §6 (Class C);
  `reference/python/PARITY.md` (e) item 4.
- Neighbours: `rapm-attribution.md`, `layer1-credit.md`, `state-space-kalman.md`,
  `synthetic-world.md`, `evaluation-and-leakage.md`.

---

## 3. Inputs and outputs

### 3.1 Inputs

| Input | Oracle form | Units / domain | Null and failure semantics (oracle → required) | As-of rule |
|---|---|---|---|---|
| `down`, `ydstogo`, `yardline_100` | `STATE_COLS` (`value.py:25`), cast to float (`:49`) | down 1..4; ydstogo ≥ 1; yardline_100 = yards to the opponent goal. Synthetic: ydstogo 1..24 (legacy) / 1..26 (fixed), yardline 1..104 / 1..101. Real 2023: ydstogo 1..40, yardline 1..99 (measured) | Missing column → bare `KeyError`. NaN → passed to the booster as "missing" silently. Out-of-range values (for example the −1 sentinel) are binned to the edge bin silently (§5.4) → typed `ContractError` | the row's play must be at or before the fit point (§6.2) |
| `drive_points` | float label (`value.py:50`) | {0, 3, 7} | Not validated. Any float is accepted → typed `ContractError::InvalidLabel` outside {0, 3, 7} | the drive must be complete at the fit point |
| `terminal` | bool (`value.py:95`) | exactly one True per drive, on its last modelled row | Not validated → typed error on violation | — |
| `terminal_value` | float, read on terminal rows only (`value.py:97`) | {0, 3, 7}; equals `drive_points` on terminal rows; NaN elsewhere | NaN on a terminal row → NaN dV silently → typed error | — |
| `n_down`, `n_ydstogo`, `n_yardline_100` | read on continuing rows only (`value.py:99-102`) | as the state; −1 sentinel on terminal rows | A sentinel on a continuing row is evaluated silently (V(−1, −1, −1) = 7.130 on the canonical synth, measured) → typed error | — |
| `random_state` | int, default 0 (`value.py:27`) | — | Seeds the booster and its early-stopping split (§4.1) | recorded in the artifact |
| `season_type` | **absent** from the oracle contract | — | Postseason rows enter the fit on real data (KI-NEW-V0a; 1,638 rows in 2023) → REG only for the label (proposed — DR-C12) | — |

**Training rows.** The oracle fits on **every row** of the frame it is given, terminal rows
included, with no filter (`value.py:49-55`). The docstring's "non-terminal-irrelevant rows"
(`value.py:29`) does not describe the code.

### 3.2 Label and next-state construction (V-relevant semantics; the plays contract owns the rest)

| Aspect | Synthetic producer (`synth.simulate`) | nflverse producer (`build_plays_contract`) |
|---|---|---|
| Label | 7 on a touchdown; 3 on a field goal (a failed fourth down inside `fg_range_yardline = 35`, with probability 0.82); 0 on a failed fourth down otherwise, and 0 on a drive cut at `max_plays_per_drive = 12` plays (`synth.py:222-237,256-260`) | `{Touchdown: 7, Field goal: 3}[fixed_drive_result]`, every other value 0 through `fillna(0.0)` (`nflverse_adapter.py:36,106`) |
| Terminal row | the play that ends the drive; or the 12th play of a truncated drive | the last pass or run of the drive (`nflverse_adapter.py:110-114`) |
| Next state | the next row's state within the drive (`synth.py:262-271`) | within-drive `shift(-1)` over modelled rows only (`nflverse_adapter.py:110-118`); penalties, kneels and spikes in between are skipped |
| Broadcast | `drive_points` is constant within the drive | same |

Measured terminal outcomes:

- **Canonical synth.** Legacy 2,016 drives: 417 scored 7, 62 scored 3, 1,537 scored 0. Fixed: 399 /
  86 / 1,531. Of the zero-point drives, **486 (legacy) and 586 (fixed) end only because of the
  12-play cap**: 24.1% and 29.1% of all drives (§7.4).
- **Real 2023 (historical).** 6,085 drives. The terminal play's `fixed_drive_result` is Punt 2,338,
  Touchdown 1,296, Field goal 948, Turnover 653, Turnover on downs 368, End of half 232, Missed field
  goal 156, Opp touchdown 76, Safety 18. All but Touchdown and Field goal score 0 for the offense.
  By rows: 0 → 17,833, 7 → 10,007, 3 → 7,634 (also in plays-contract §6.3).

Consequences the port MUST keep in mind:

- On real data a drive ending in a field-goal attempt credits its value (3, or 0 if missed) to the
  last offensive snap before the kick, because kicks are not modelled rows.
- A turnover, punt or turnover on downs ends the drive at value 0. The offense's dV on that play is
  `−V(s)`. Field position handed to the opponent is not valued (no signed EP; DR-C13).
- An opponent return touchdown or a safety scores 0 for the offense, not −7 or −2 (KI-NEW-V0b).
- An unknown future `fixed_drive_result` value would silently score 0 (plays-contract D-3).

### 3.3 Outputs

**Oracle.**

- `fit_value_model` returns a fitted `HistGradientBoostingRegressor`. With `cache_path` it is
  written by `joblib.dump` to that file, silently overwriting (`value.py:57-59`); parent directories
  are created.
- `attach_dv` returns a copy of the frame with a `dv` column (`value.py:107-110`).
- `classify` returns `{name: bool array}` (`situations.py:37-62`).

**Required engine outputs.**

| Output | Definition | Notes |
|---|---|---|
| V(s) artifact | the fitted estimator plus `value_model_version`: estimator id and version, hyperparameters, training window (seasons, season types), data version, row count, seed (if any), content hash | immutable; written atomically (engine-spec §8.5, §8.11; proposed — DR-C15) |
| V on the declared state grid | every observed `(down, ydstogo, yardline_100)` cell of the training frame and the 1st-and-10 line `yardline_100 = 1..99`, each with its training support count | the parity surface (§10.3) and a regression artifact |
| `dv` per play | §4.2, tagged with `value_model_version` | a consumer holding dV from two versions MUST fail with `VersionMismatch` |
| situation masks | §4.3, plus an availability flag per mask | `two_minute` is `Unavailable`, never silently omitted (plays-contract D-1) |
| fit diagnostics | row count, label counts, number of cells, cells with support ≥ the estimator's minimum leaf/cell size, in-sample mean V vs mean label, telescoping max error, range of V | persisted with the artifact |

### 3.4 Explanation fields (engine-spec §6.7)

- `dv` is reported in **GRID expected points per play**. Any EPA-type feature derived from it
  (engine-spec §11.6) MUST be labelled as GRID dV, not as nflfastR EPA.
- `value_model_version` and its training window accompany every explanation that cites dV.
- Situation labels use the canonical names of §4.3. A player with no plays in a situation is reported
  as "no data", never as a zero grade (`layers.py:437-440`).

---

## 4. Model and equations

### 4.1 The V(s) estimator, exactly as the oracle implements it (`value.py:49-55`)

```text
X  = [down, ydstogo, yardline_100] as float64, one row per play     (value.py:49)
y  = drive_points as float64                                        (value.py:50)
model = sklearn.ensemble.HistGradientBoostingRegressor(
            max_depth=4, learning_rate=0.08, max_iter=300,
            min_samples_leaf=120, random_state=random_state)        (value.py:51-54)
model.fit(X, y)                                                     (value.py:55)
V(s) = model.predict([s])
```

Every other hyperparameter is a scikit-learn 1.9.1 default, read from the fitted object (measured):
`loss='squared_error'`, `max_leaf_nodes=31`, `l2_regularization=0.0`, `max_features=1.0`,
`max_bins=255`, `categorical_features='from_dtype'` (no categoricals here), `monotonic_cst=None`,
`interaction_cst=None`, `early_stopping='auto'`, `validation_fraction=0.1`, `n_iter_no_change=10`,
`tol=1e-7`, `scoring='loss'`, `warm_start=False`.

What the booster computes (squared error, unit hessians):

```text
F_0      = mean(y_train)
F_m(x)   = F_{m−1}(x) + η · h_m(x),          η = 0.08
h_m      = a regression tree grown best-first on residuals y − F_{m−1}(X_train):
           depth ≤ 4 (so ≤ 16 leaves; max_leaf_nodes = 31 never binds),
           every leaf holds ≥ 120 training rows,
           leaf value = mean residual in the leaf (l2 = 0)
V(x)     = F_M(x),  M = n_iter_
```

**Binning.** Each feature is binned into at most 255 bins. Every frame in scope has fewer than 255
distinct values per feature (down 4; ydstogo ≤ 40; yardline ≤ 104), so every distinct value gets its
own bin and splits fall between consecutive observed values. A value outside the training range
falls into the edge bin: `V(1, 10, −1) = V(1, 10, 1)` on the canonical synth (measured).

**Early stopping is on for every frame larger than 10,000 rows.** This behaviour is undocumented in
the oracle and in the inventory (new):

- `early_stopping='auto'` enables early stopping exactly when `n_samples > 10_000`.
- The fit then holds out a random 10% validation split, `train_test_split(test_size=0.1,
  random_state=s)`, with `s` drawn from `check_random_state(random_state)`.
- It stops when the validation loss has not improved by more than `tol` over 10 iterations. The
  fitted model keeps `n_iter_` iterations. It does not roll back to the best one.
- Consequently `max_iter = 300` is a cap that the full-world fits never reach, and they train on 90%
  of the rows:

| Frame | Rows | Early stopping | `n_iter_` (seed 0) |
|---|---|---|---|
| Canonical synth, legacy | 16,825 | on | 77 |
| Canonical synth, fixed | 17,752 | on | 53 |
| Real 2023, all rows (historical) | 35,474 | on | 87 |
| Real 2023, REG only (historical) | 33,836 | on | 79 |
| Backtest warm-up, legacy synth weeks 1–4 | 4,784 | **off** | 300 |
| Backtest warm-up, real 2023 weeks 1–4 (historical) | 8,017 | **off** | 300 |

So the same call is two different estimators depending on frame size. The backtest's frozen
warm-up V(s) is fit on all rows for 300 iterations. Every full-season fit is early-stopped on a
seed-dependent split. Across seeds 1–10, `n_iter_` ranges over 47–182 (legacy synth), 51–87
(fixed synth) and 52–96 (real 2023), measured.

### 4.2 Play valuation (`compute_dv`, `value.py:89-104`)

With `T_i` the terminal flag and `tv_i` the terminal value:

```text
dV_i = T_i · tv_i
     + (1 − T_i) · V(n_down_i, n_ydstogo_i, n_yardline_100_i)
     − V(down_i, ydstogo_i, yardline_100_i)
```

- **Scoring play.** `dV = 7 − V(s)` (touchdown) or `3 − V(s)` (field goal).
- **Turnover, punt, turnover on downs, end of half, missed field goal, opponent return score,
  safety, or synthetic truncation.** `dV = 0 − V(s) = −V(s)`.
- **Continuing play.** `dV = V(s′) − V(s)`, with `s′` the next modelled snap of the same drive.
- `V(s)` is evaluated for every row (`:91-92`) and `V(s′)` only for continuing rows (`:99-102`).
  Terminal rows' sentinels are never evaluated, **provided `terminal` is correct**.
- `attach_dv` copies the frame and adds the column (`:107-110`).

Measured dV:

| Frame | mean dV | SD dV | mean dV on terminal rows by value 0 / 3 / 7 |
|---|---|---|---|
| Canonical synth, legacy | 0.0102 | 1.1553 | −1.127 / +1.444 / +1.067 |
| Canonical synth, fixed | 0.0213 | 1.0649 | −1.332 / +1.431 / +1.179 |
| Real 2023, all rows (historical) | −0.0052 | 1.1427 | — |

### 4.3 Situation masks (`situations.py`, exactly as coded)

| Mask | Definition | Defined at |
|---|---|---|
| `red_zone` | `yardline_100 ≤ 20` | `situations.py:31` |
| `passing_downs` | `(down = 3 and ydstogo ≥ 7) or down = 4` | `situations.py:32` |
| `rushing_downs` | `down ≤ 2 and ydstogo ≤ 4` | `situations.py:33` |
| `two_minute` | `quarter_seconds_remaining ≤ 120`, **only if that column exists**; otherwise the key is omitted and `WARNING "two_minute skipped: quarter_seconds_remaining absent"` is logged | `situations.py:23,57-60` |

- The masks overlap (a red-zone third and 8 is in `red_zone` and `passing_downs`) and do not cover
  every play (third and 1–6 is in neither down mask).
- Neither producer emits `quarter_seconds_remaining`, so `two_minute` is absent on every frame the
  oracle builds (KI-NEW-A4; plays-contract D-1). The definition also ignores the quarter, so it would
  include the ends of the first and third quarters.
- Measured counts: canonical legacy `red_zone` 1,478, `passing_downs` 2,897, `rushing_downs` 1,812;
  fixed 1,676 / 2,851 / 1,899; real 2023 5,187 / 4,346 / 3,103 (historical).
- **Not in the engine.** CN documentation listed `goal_line`, `third_and_long` and `fourth_down`
  (cn-docs §14 item 3). They do not exist in code and are **not** situations of this engine. Adding
  any situation requires a revision of this spec (proposed — DR-C13).
- **Required.**
  - The canonical set is the code set: `red_zone`, `passing_downs`, `rushing_downs`, `two_minute`.
    One enum spells them, plural as in code (KI-#57; proposed — DR-C13).
  - `two_minute := half_seconds_remaining ≤ 120`, which removes the quarter defect. The adapter MUST
    emit `half_seconds_remaining` (proposed — DR-C13; plays-contract D-1). The change from the
    oracle's definition is a deliberate divergence listed in `reference/python/PARITY.md`.
  - On a frame without the clock column, `two_minute` is reported `Unavailable`. A consumer that
    requires it gets a typed error, not a missing key.

### 4.4 Constants and hyperparameters

Provenance key: **hand-set v0** = present at `f3b641f`, the oldest shallow-clone boundary in CN
(2026-06-21), with no calibration record; the v0 upload `6b0eeee` exists on GitHub only (critic G-3).
**library default** = scikit-learn 1.9.1, never set by the oracle.

| Constant | Value | Defined at | Provenance |
|---|---|---|---|
| `STATE_COLS` | `["down", "ydstogo", "yardline_100"]` | `value.py:25` | hand-set v0 |
| `max_depth` | 4 | `value.py:52` | hand-set v0 |
| `learning_rate` | 0.08 | `value.py:52` | hand-set v0 |
| `max_iter` | 300 (a cap; §4.1) | `value.py:52` | hand-set v0 |
| `min_samples_leaf` | 120 | `value.py:53` | hand-set v0. Also the support threshold of the Class C grid (parity-fixture-contract §6) |
| `random_state` | 0 | `value.py:27` | hand-set v0 |
| Early stopping | on iff rows > 10,000; 10% split; 10 rounds; tol 1e-7 | sklearn defaults | library default. Never chosen by the oracle |
| `loss`, `max_leaf_nodes`, `l2_regularization`, `max_bins` | squared error, 31, 0.0, 255 | sklearn defaults | library default |
| Label vocabulary | TD 7, FG 3, else 0 | `nflverse_adapter.py:36`; `synth.py:222-237` | hand-set scaffold (`nflverse_adapter.py:20-22`); kept for v1 (proposed — DR-C13) |
| Synthetic FG rule | inside `fg_range_yardline = 35`, probability 0.82 | `synth.py:50,232` | owned by `synthetic-world.md` |
| Synthetic drive cap | 12 plays, scored 0 | `synth.py:49,256-260` | owned by `synthetic-world.md` |
| `red_zone` threshold | ≤ 20 | `situations.py:31` | hand-set (present at the `c1b6748` boundary, 2026-06-22) |
| `passing_downs` | down 3 and ≥ 7, or down 4 | `situations.py:32` | as above |
| `rushing_downs` | down ≤ 2 and ≤ 4 | `situations.py:33` | as above |
| `two_minute` | ≤ 120 s on `quarter_seconds_remaining` | `situations.py:23,58` | as above; replaced (§4.3) |

### 4.5 Priors

None. V(s) is a frequentist state surface with no prior and no shrinkage target beyond the
booster's leaf-size and learning-rate regularization. Any prior introduced by the DR-C7 estimator
(for example smoothing toward a parametric surface) MUST be stated here before implementation.

### 4.6 Constraints

| Constraint | Oracle | Required |
|---|---|---|
| Label in {0, 3, 7} | not checked | `ContractError::InvalidLabel` |
| Finite state, label and terminal value | not checked | `ContractError::NonFinite` |
| Exactly one terminal row per drive, last in order; `terminal_value = drive_points` there | not checked | `ContractError::DriveStructure` |
| No sentinel on a continuing row | not checked; evaluated silently | `ContractError::SentinelOnContinuingRow` |
| V within the label hull [0, 7] | **violated**: canonical synth V ranges over [−0.240, 7.046] on observed states and up to 7.130 on the integer grid; real 2023 V ranges over [−0.435, 6.415] (measured) | the engine estimator MUST produce values in [0, 7] by construction; a value outside is `ValueModelError::OutOfHull`. Never clamped |
| Monotonicity | none imposed. Measured: V(1st and 10) **rises** with distance to goal at 18 (synth) and 17 (real 2023) of 98 one-yard steps; V rises with down at fixed (ydstogo, yardline) in 191 of 1,368 synthetic cells (0 of 1,838 real) | open, part of DR-C7 (§9) |
| State inside the declared support | edge-bin extrapolation | `ValueModelError::OutOfSupport`, unless the estimator spec defines extrapolation |

---

## 5. Algorithm, numerics and determinism

### 5.1 Sizes and cost

| Frame | Rows | Distinct states | States with support ≥ 120 (share of rows) | Fit time, 1 thread |
|---|---|---|---|---|
| Canonical synth, legacy | 16,825 | 2,999 | 14 (11.0%) | about 0.13 s |
| Canonical synth, fixed | 17,752 | 3,003 | 14 (10.6%) | similar |
| Real 2023, all rows (historical) | 35,474 | 3,938 | 56 (32.8%) | — |

V(s) is cheap. Its cost is irrelevant to the daily DAG because it is fit once per season (§6.2).

### 5.2 Determinism and seed policy

- **In process.** Deterministic for a given frame, `random_state` and library build. The oracle's
  determinism gate asserts dV agrees to `atol 1e-9` across two in-process runs
  (`tests/grid/test_determinism.py:50-60`).
- **Threads.** V(s) is **thread-invariant** on the canonical synth: predictions at 1 and 4 OpenMP
  threads differ by 0.0 on every row (measured; scikit-learn builds histograms per feature, so the
  per-bin sums keep a fixed order). `tests/grid/golden_master.py:55-60` says "multi-threaded the
  gradient-boosted V(s) / Layer-1 credit diverge at ~1e-2". The divergence is real, but it does not
  come from V(s): at 4 threads the golden ratings move by up to 9.6e-4 and `qb_credit` by up to
  0.021 (measured), downstream of an identical V(s).
- **Seed.** `random_state` changes V(s) materially, because it selects the early-stopping split and
  so the iteration count (§4.1). Seeds 1–10 against seed 0 (measured):

  | Frame | max \|ΔV\| on states with support ≥ 120 | median over seeds of that max | corr(dV, dV₀) range |
  |---|---|---|---|
  | Canonical synth, legacy | 0.270 | 0.180 | 0.9891–0.9942 |
  | Canonical synth, fixed | 0.200 | 0.088 | 0.9896–0.9931 |
  | Real 2023, all rows (historical) | 0.216 | 0.174 | 0.9940–0.9960 |

  With early stopping disabled (300 iterations, seed 0) the canonical synth moves by up to 0.245
  (legacy) and 0.148 (fixed), with corr(dV) 0.987 and 0.981. A crude empirical cell-mean estimator
  differs from the booster by up to 0.330 (legacy) and 0.362 (fixed) on the 14 supported states.
- **Engine rule.**
  - The engine estimator MUST be deterministic for identical inputs, independent of thread count and
    of input row order. Training rows are sorted by `(season, week, game, play)` key before fitting.
  - It MUST NOT switch behaviour on frame size. Any internal validation or tuning split is
    deterministic, declared and recorded.
  - Any seed is recorded in `value_model_version`. The canonical parity seed for the oracle is
    `random_state = 0` (parity-fixture-contract §4).

### 5.3 Numerical facts the port can rely on

- The telescoping identity (§1.4) holds to rounding for any V. It is a free property test.
- dV is a subtraction of two predictions. Given the predictions, Class A (≤ 1e-12) is exact
  arithmetic.

### 5.4 Typed failures (superseded alpha-spec §6.6 rule 4; proposed — DR-B6)

| Condition | Oracle behaviour | Required |
|---|---|---|
| A state, label or next-state column missing | bare `KeyError` (`value.py:49-50,91,95-101`) | `ContractError::MissingColumn` |
| NaN or Inf in a state, label or terminal value | booster treats NaN state as missing; NaN terminal value gives NaN dV; nothing raised | `ContractError::NonFinite` |
| Label outside {0, 3, 7} | accepted | `ContractError::InvalidLabel` |
| Sentinel −1 on a continuing row, or a wrong `terminal` flag | evaluated silently (V(−1, −1, −1) = 7.130) | `ContractError::SentinelOnContinuingRow` / `DriveStructure` |
| State outside the declared support | edge-bin extrapolation | `ValueModelError::OutOfSupport` |
| Prediction outside [0, 7] | possible (§4.6) | `ValueModelError::OutOfHull` |
| Empty training frame | sklearn `ValueError` | `ValueModelError::EmptyTrainingSet` |
| Training window incomplete (fewer than the §6.2 seasons) | n/a | `ValueModelError::WindowIncomplete` |
| Training rows at or after the fit point | allowed (KI-NEW-W4) | impossible by construction: the fit takes an `AsOf` view (P1-05) |
| dV from two V(s) versions mixed in one block or solve | undetectable | `ValueModelError::VersionMismatch` |
| Artifact write interrupted | `joblib.dump` in place, silent overwrite | temp file + rename, content hash, registered in the SQLite manifest (proposed — DR-C15) |

---

## 6. Incremental and online behaviour

### 6.1 Oracle

V(s) is refit from scratch on every call. There is no continuation, no versioning and no as-of
discipline in the estimator itself; each caller decides the frame (§2.2). The three disciplines in
the oracle disagree:

- `weekly_update` refits on the whole season on every run (`weekly_update.py:213`). On a backfill of
  week W it uses weeks after W, and each run re-bases the dV of every earlier week, which also
  invalidates the RAPM sufficient statistics accumulated from them (KI-NEW-W4;
  `reference/python/PARITY.md` (e) item 4; rapm-attribution.md §6.3).
- `walk_forward` freezes V(s) on the first four slots (`backtest.py:165-168`) — 4,784 rows on the
  synth, so early stopping is off (§4.1).
- `verdict` refits on the pre-origin `AsOf` slice (`verdict.py:169-172`).

### 6.2 Required behaviour (proposed — DR-C6, DR-C7)

1. **Fit once per season, at the season boundary.** The V(s) used for every projection origin in
   season `S` is fitted on the regular-season plays of the three completed seasons `S−3, S−2, S−1`
   (the preseason window of superseded alpha-spec §2.4, carried into engine-spec §2.4) and frozen for
   all of season `S`.
   - It is **not** refit in season on `S−2, S−1, S[1..W−1]`. That window would be permitted by
     §2.4, but refitting would re-base dV and every RAPM block and Layer-1 credit already built on it.
     Freezing uses only pre-season data, so it is strictly inside the §2.4 window.
   - V(s) needs play-by-play only, not participation, so it has no publication-lag dependency
     (engine-spec §4.5).
2. **Versioned and as-of bounded.** The fit consumes an `AsOf` view whose cutoff is the season
   boundary. A leakage test MUST show that poisoning any play of season `S` or later leaves the season-`S`
   artifact bit-identical (evaluation-and-leakage.md; superseded alpha-spec §12.3).
3. **Version propagation.** `value_model_version` is part of the data → feature → model →
   prediction chain (engine-spec §8.8). A new V(s) version invalidates every dV-derived block: RAPM
   sufficient statistics (rapm-attribution.md §6.4), Layer-1 and Layer-1′ credit, Kalman
   observations, and dV features. Mixing versions is a typed error (§5.4).
4. **Backtests replay the production rule.** Each historical origin uses the artifact that the
   season-boundary rule would have produced for its season (two-path equivalence; P1-09). The
   oracle's four-slot warm-up freeze is a research harness and is not ported.
5. **Window completeness.** If the window lacks a season, the fit is refused
   (`WindowIncomplete`). A shorter-window policy needs a revision of this spec.

---

## 7. Validation evidence

### 7.1 Oracle tests touching this component (all pass on Linux at `59bce1d`)

| Test | What it pins | Status for the port |
|---|---|---|
| `tests/grid/test_determinism.py:50-60` | dV identical to `atol 1e-9` in process | Port as a Rust determinism gate, strengthened to bit-identical across thread counts (§5.2) |
| `tests/grid/test_performance.py:205-305` | `cache_path` persistence, round trip, missing-file errors | **Not ported.** Persistence is the engine artifact protocol (§3.3) |
| `tests/grid/test_nflverse_adapter.py:51-100,192-218` | drive-points broadcast; next state and terminal within drive; terminal value; no cross-drive bleed; the contract flows through `fit_value_model` + `compute_dv` with finite dV | Port (plays-contract §13, P1-03), plus the D-3/D-4 typed errors |
| `tests/grid/test_situations.py:38-138` (5 tests) | red-zone threshold; fourth down in `passing_downs`; `two_minute` omitted with a warning when the column is absent; boolean masks of the right length; no input mutation | Port the first, second, fourth and fifth exactly. The third becomes `Unavailable` (§4.3; divergence) |
| `tests/grid/test_layers_situations.py` | situation RAPM slicing | Owned by `rapm-attribution.md` §4.8 |

No oracle test validates V(s) itself against any truth. The synthetic world plants no value
function (§7.5).

### 7.2 V(s) surface (measured; legacy numbers are not Rust targets)

| Frame | V(1st and 10) at yardline 90 / 60 / 30 / 10 | Mean V(s) over rows vs mean label |
|---|---|---|
| Canonical synth, legacy | 1.1171 / 1.8404 / 3.3749 / 5.5746 (reconcile-code-first §1.3 records the same four values) | 1.8341 vs 1.8373 |
| Canonical synth, fixed | 0.7251 / 2.1188 / 2.9669 / 4.5888 | 1.7633 vs 1.7619 |
| Real 2023, all rows (historical) | 1.2452 / 2.4490 / 4.0233 / 4.9078; at yardline 75: 1.8492 | — |
| Real 2023, REG only (historical) | 1.2137 / 2.4648 / 3.9819 / 4.9964; at 75: 1.7960 | — |

The legacy and fixed synthetic surfaces differ by up to 0.99 EP on these four states (at yardline
10), because the defender fix changes the realized plays (synthetic-world.md §8.1). The seed
sensitivity of §5.2 is of the same order as several of these differences.

### 7.3 Class C feasibility (input to DR-B3)

DR-B3 proposes Class C as `corr(dV) ≥ 0.999` **and** `|ΔV| ≤ 0.10` EP on grid cells with support
≥ `min_samples_leaf`. Measured against §5.2:

- The oracle against itself with another seed reaches corr(dV) 0.9891–0.9942 (legacy synth),
  0.9896–0.9931 (fixed synth) and 0.9940–0.9960 (real 2023). **No seed reaches 0.999.**
- Across the three frames the per-seed maximum |ΔV| on supported states has a median of
  0.088–0.180 and a worst case of 0.200–0.270. The 0.10 bound therefore fails for most re-seeds of
  the legacy and real frames and for about half of the fixed ones.
- Only 14 synthetic states (about 11% of rows) have support ≥ 120. The grid criterion covers a small,
  central part of the surface.

So the DR-B3 Class C criterion cannot be met by scikit-learn re-seeded, and a different estimator,
which is what DR-C7 proposes, is a larger perturbation than a seed. §10.3 proposes a V(s)-specific
replacement (DR-D27). `layer1-credit.md` §5.3 reaches the same conclusion for
the Layer-1 context booster.

### 7.4 State-centring of dV, and why the synthetic world breaks it

E[dV | down] for downs 1 / 2 / 3 / 4 (measured; "cell mean" replaces the booster with the exact
empirical mean of `drive_points` in each state, to separate estimator effects from generator
effects):

| Frame | Booster V | Cell-mean V |
|---|---|---|
| Canonical synth, legacy | −0.247 / −0.108 / +0.437 / +0.649 | −0.267 / −0.094 / +0.437 / +0.663 |
| Legacy generator, drive cap raised to 200 plays | −0.091 / −0.127 / +0.158 / +0.344 | −0.109 / −0.120 / +0.164 / +0.363 |
| Legacy generator, `yards_ability_gain = 0` (no player effects) | −0.001 / −0.035 / +0.064 / +0.136 | −0.010 / −0.032 / +0.071 / +0.137 |
| Both changes | +0.020 / −0.018 / +0.003 / −0.027 | −0.006 / −0.004 / +0.015 / −0.004 |
| Real 2023, all rows (historical) | −0.006 / −0.020 / +0.012 / +0.068 | −0.016 / −0.007 / +0.014 / +0.086 |

The booster column for the canonical world matches `layer1-credit.md` §7.3 (−0.247, −0.108, +0.437,
+0.649), which recorded the pattern without diagnosing it. Diagnosis:

1. **It is not the estimator.** The exact cell mean shows the same pattern.
2. **It is the generator, for two reasons.**
   - The 12-play cap scores a drive 0 by play count, not by state, so the process is not Markov in
     `s` (24.1% of legacy drives end that way; §3.2).
   - Lineup quality persists within a drive at the synthetic world's large ability scale, so a
     drive's history predicts its outcome beyond the current state.

   Removing both restores centring to within noise.
3. **Real dV is close to centred** (fourth down +0.068 is the largest), consistent with the
   real-data out-of-fold R² of −0.0005 recorded in `layer1-credit.md` §7.3.

Consequences: synthetic gates over-exercise any state term in Layer 1, and V(s) recovery claims on
the legacy world inherit a non-Markov artefact. The corrected world MUST end drives by football
events, not by a play cap (synthetic-world.md §4.11; proposed — DR-B4).

### 7.5 What has never been validated

- **No recovery gate for V(s).** The synthetic world plants no value function. The corrected
  generator SHOULD emit the true `V*(s)` of its own state process (by exact dynamic programming or a
  declared Monte-Carlo budget for the generator's mean lineup), so that V(s) recovery can be gated
  (synthetic-world.md §4.10; proposed — DR-B4).
- **No out-of-sample accuracy check.** No oracle path measures V(s) on held-out seasons. The DR-C7
  estimator choice MUST be made on held-out-season squared error of the label and on downstream
  recovery, pre-registered before results (engine-spec §7; superseded alpha-spec §12.7).

---

## 8. Known defects and required engine behaviour

### 8.1 Registered defects

The Rust port MUST NOT reproduce any item below.

| # | Defect | KI / source | Required behaviour |
|---|---|---|---|
| 1 | V(s) refit on the whole season on every weekly run, leaking later weeks into dV and re-basing earlier blocks | KI-NEW-W4; `PARITY.md` (e) 4; plays-contract D-14 | §6.2: once per season on the window, frozen, as-of bounded (proposed — DR-C6, DR-C7, DR-B6) |
| 2 | Postseason rows train V(s) | KI-NEW-V0a; plays-contract D-2 | REG only (proposed — DR-C12) |
| 3 | 7/3/0 label: opponent return touchdowns and safeties score 0; no clock or score in the state | KI-NEW-V0b; plays-contract D-3 | Kept for v1 and documented here (proposed — DR-C13). Signed EP or clock/score state is a v2 change of this spec and of the plays contract |
| 4 | An unknown `fixed_drive_result` silently scores 0 | KI-NEW-Z46; plays-contract D-3 | Closed table; unknown value → `SchemaDrift` (P1-03) |
| 5 | A `fixed_drive` spanning a change of possession chains next state across offenses | KI-NEW-Z47; plays-contract D-4 | Quarantine the drive (P1-03) |
| 6 | `two_minute` uses the quarter clock and is never emitted | KI-NEW-A4; plays-contract D-1 | §4.3: `half_seconds_remaining ≤ 120`; `Unavailable` on synthetic frames (proposed — DR-C13) |
| 7 | Situation names plural in code, singular in the schema | KI-#57 | One canonical enum (proposed — DR-C13) |
| 8 | No V(s) versioning; `cache_path` silently overwrites; joblib (pickle) artifact | KI-NEW-Z45; reconcile-spec-first R55; `PARITY.md` (e) 5 | §3.3 artifact; atomic write; no pickle (proposed — DR-C15) |
| 9 | Synthetic state space leaves football (yardline above 99, `ydstogo > yardline_100`) | KI-NEW-Z39; plays-contract D-9 | The V(s) support is declared per profile; the corrected and realistic synthetic profiles are football-valid (synthetic-world.md §4.10–§4.11) |
| 10 | Gradient boosting on GRID's critical path, not deterministic across seeds | DR-C7; reconcile-spec-first C4 | Deterministic in-house estimator behind a `Regressor` trait (proposed — DR-C7, DR-C8; §10.2) |

### 8.2 Defects found in this consolidation (to be registered)

| # | Defect | Evidence | Required behaviour |
|---|---|---|---|
| N-1 (KI-NEW-Z41) | **Hidden early stopping.** For frames over 10,000 rows the booster early-stops on a seeded random 10% split; `max_iter = 300` is never reached on full-season frames, and smaller frames (the backtest warm-up) train a different regime | §4.1 table | §5.2 engine rule: no size-dependent mode switch; any split declared and recorded. Fixtures record `n_iter_` and the split (§10.3) |
| N-2 (KI-NEW-Z74) | **Seed dominates V(s) at the scale of the parity tolerance.** Re-seeding moves V by up to 0.27 EP on supported states and caps corr(dV) at 0.989–0.996 | §5.2, §7.3 | Amend DR-B3 Class C for V(s) (DR-D27; §10.3) |
| N-3 (KI-NEW-Z42) | **V outside the label hull.** Predictions as low as −0.240 (synth) and −0.435 (real) | §4.6 | `OutOfHull` typed failure; the engine estimator is hull-respecting by construction |
| N-4 (KI-NEW-Z44) | **Silent out-of-support evaluation.** A −1 sentinel evaluates to 7.130 | §3.1 | `SentinelOnContinuingRow`, `OutOfSupport` |
| N-5 (KI-NEW-Z32) | **Synthetic dV is not state-centred**, caused by the 12-play cap and persistent lineup quality | §7.4 | Corrected generator ends drives by football events (synthetic-world.md, DR-B4) |
| N-6 (KI-NEW-Z43) | **Non-monotone V.** V(1st and 10) rises with distance at 17–18 of 98 steps; V rises with down in 191 synthetic cells | §4.6 | Input to the DR-C7 estimator choice |
| N-7 (KI-NEW-Z75) | The golden master's thread-divergence comment attributes the ~1e-2 drift to V(s), which is thread-invariant | §5.2 | Documentation correction (oracle unchanged); the Rust determinism gate covers every stage |
| N-8 (KI-NEW-Z77) | Wrong `value.py` line citations in two contracts: plays-contract §2.1–§2.2 cites `value.py:112,157,159,161-164,169-172` and §1 `:87,111-112,151-166`; parity-fixture-contract §5 cites `value.py:153-164` and `:113-116`. `value.py` has 123 lines. The correct sites are `:50` (label), `:51-54` (estimator, `min_samples_leaf` at `:53`), `:89-104` (`compute_dv`: `terminal` `:95`, `terminal_value` `:97`, next state `:99-102`) and `:107-110` (`attach_dv`) | `wc -l`; this spec | Contract owners fix the citations |

---

## 9. Open decisions

| Decision | Question | Proposed default |
|---|---|---|
| **DR-C7** | Which estimator produces V(s)? | A deterministic in-house estimator behind a `Regressor` trait: binned-and-smoothed or monotone-constrained. No nflfastR `ep` (proposed — DR-C7). The choice is made on held-out-season label error and downstream recovery (§7.5). Whether monotonicity in yardline, distance and down is imposed (§4.6, N-6) is part of this decision |
| **DR-C8** | Booster backend, if any booster is used | Pure Rust first; `xgb` optional behind a feature (proposed — DR-C8). If DR-C7 adopts a non-boosted estimator, V(s) needs no booster |
| **DR-C13** | `drive_points` vocabulary; situation set; clock column | 7/3/0 for v1; the code set; `two_minute` on `half_seconds_remaining` (proposed — DR-C13) |
| **DR-C6** | Three-season window for stateful components | V(s) refit per season on the window, frozen in season (proposed — DR-C6; §6.2) |
| **DR-C12** | Season type for labels | REG only (proposed — DR-C12) |
| **DR-B3** | Parity tolerances | Class C does not fit V(s) (§7.3). See DR-D27 |
| **DR-B6** | Typed failure vs faithful port | Typed failures of §5.4; refit-every-run is a divergence (proposed — DR-B6) |
| **DR-D27** | What does V(s) parity mean when the Rust estimator is, by DR-C7, a different estimator, and the oracle is not reproducible against itself under a seed change? | The three-part criterion C-V of §10.3. Thresholds pre-registered by the statistical owner before any Rust V(s) is evaluated |
| **DR-D22** | May nflverse's `ep`/`epa` columns enter features at all (superseded alpha-spec §11.3 lists "EPA and success per opportunity")? DR-C7 rules them out only as V(s) | **Open, no default.** The same scope-leak argument applies: nflfastR's model was trained on later seasons. Until decided, EPA-type features are computed from GRID dV only |

---

## 10. Rust port plan

### 10.1 Homes (engine-spec §8.1; DR-A8 adopted subject to ratification)

| Piece | Crate::module | WP |
|---|---|---|
| `Regressor` trait; the deterministic V(s) estimator primitive once DR-C7 names it; `booster` adapter if DR-C8 needs one | `models::value`, `models::booster` | **P1-06** (numerical primitives: trait and estimator primitive) |
| V(s) stage: as-of fit, artifact, `compute_dv`, version tags, fit diagnostics | `models::value` | **P1-12** (GRID component port) |
| Situation masks and their availability | `features::situations` | P1-12 |
| Plays-contract types, label and drive-structure validation, `half_seconds_remaining`, closed drive-result table | `domain` (types), `ingestion` (producer) | **P1-03** |
| As-of fit view and the leakage test of §6.2 item 2 | `domain` `AsOf` + `features` store | **P1-05** |
| Season-boundary V(s) in walk-forward replay; two-path | `evaluation` | **P1-09** |
| V(s) artifact storage | `persistence` | P1-02 / P1-12 |
| Season-boundary refresh stage | `pipeline` (DR-A8) | P2-01 |

`models` stays synchronous, with no Tokio or SQLx dependency (engine-spec §8.1 rule 3).

### 10.2 The `Regressor` trait and estimator requirements (proposed — DR-C7, DR-C8)

The trait replaces superseded final-build-spec §11.8's `IncrementalBooster` for V(s). V(s) is
refit per season, never continued, so it needs no `continue_training`. An illustrative shape (P1-06
fixes the real one):

```rust
pub trait Regressor {
    type Fitted: FittedRegressor;
    fn fit(&self, x: &FeatureMatrix, y: &[f64], w: Option<&[f64]>) -> Result<Self::Fitted, FitError>;
}
pub trait FittedRegressor {
    fn predict(&self, x: &FeatureMatrix) -> Result<Vec<f64>, PredictError>;
    fn metadata(&self) -> ModelMetadata; // estimator id + version, params, rows, data version, seed
}
```

Whatever estimator DR-C7 selects MUST satisfy:

1. **Deterministic:** bit-identical across thread counts and input row order (§5.2).
2. **No hidden modes:** no frame-size switch; any validation split is deterministic and recorded.
3. **Hull-respecting:** predictions in [0, 7] by construction (§4.6).
4. **Support-aware:** a declared state domain, declared smoothing for sparse states, and a defined
   result (or a typed error) outside the domain.
5. **Versioned:** its parameters and the training window enter `value_model_version`.
6. **Earns its place:** held-out-season label error no worse than the oracle booster on the same
   pre-registered split, and the downstream Class D gates hold (§7.5; superseded alpha-spec §6.2).
7. **Specified before code:** the estimator's equations, constants and reference examples are added
   to §4 of this spec, approved by the statistical owner, before P1-06 implements it.

### 10.3 Parity class and concrete targets

Classes are per `docs/03-contracts/parity-fixture-contract.md` (proposed — DR-B3). Downstream stages
never depend on V(s) parity: RAPM, Layer 1 and the Kalman receive the **oracle's dV** as an injected
input (parity-fixture-contract §5; rapm-attribution.md §10.2).

| ID | Target | Class | Criterion |
|---|---|---|---|
| P-V1 | `compute_dv` given the injected per-row V(s) and V(s′) | A | ≤ 1e-12 abs on every row; NaN positions identical (none on a valid frame) |
| P-V2 | Telescoping identity on the Rust dV, every drive | A (property) | ≤ 1e-12 abs |
| P-V3 | Situation masks on injected plays | exact | `red_zone`, `passing_downs`, `rushing_downs` identical; `two_minute` is `Unavailable` on synthetic frames (divergence) and follows §4.3 on clock-bearing frames |
| P-V4 | Rust V(s) estimator against its own spec reference examples (once DR-C7 adds them to §4) | A (spec golden) | as stated with the examples |
| P-V5 | Rust V(s) against the oracle booster on the canonical **corrected** synth, identical training rows | **C-V** (below) | proposed — DR-D27 |
| P-V6 | No-future-data property | exact | poisoning season-`S` plays leaves the season-`S` artifact bit-identical (§6.2) |
| P-V7 | Thread and row-order invariance | exact | Rust V(s) bit-identical at 1 and N threads and under a row shuffle |
| P-V8 | End to end with the Rust V(s), Rust boosters and Rust synth | D | Class D gates of `synthetic-world.md` §10.3, re-set on the corrected world (proposed — DR-B3, DR-B4) |
| P-V9 | Divergences: typed failures of §5.4; season-boundary freeze instead of refit-every-run; `two_minute` definition | divergence | Listed in `reference/python/PARITY.md`; Rust asserts the typed outcome |

**Fixture.** `fixtures/parity/value-model/canonical-<level>/` carries the training frame by
reference to `synthetic-world/` (`inputs_from`), the oracle V(s) on the declared state grid (§3.3)
with per-state support, per-row V(s) and V(s′), the oracle dV, the seed, the fitted `n_iter_`, the
early-stopping flag, and the **seed envelope**: V on the grid and dV for seeds 1–10, from which the
C-V thresholds' reference values are computed. Legacy cases are audit-only (DR-B1).

**C-V: a V(s)-specific Class C (new finding; proposed amendment to DR-B3).** All four are required:

1. `corr(dV_Rust, dV_oracle) ≥ 0.98` over all rows. The oracle's seed-envelope minimum is 0.9891
   (legacy) and 0.9896 (fixed).
2. On states with support ≥ 120: max |V_Rust − V_oracle| ≤ 0.30 EP. The envelope maximum is 0.270
   (legacy) and 0.200 (fixed). The cell-mean estimator's 0.33–0.36 would fail, by design: an
   estimator that much noisier than the booster in the best-supported states should not pass.
3. P-V4 holds, so the Rust estimator is the one the spec defines, not merely close to sklearn.
4. P-V8 holds with Rust dV in place of the injected dV.

These thresholds sit outside every observed oracle self-perturbation, in the calibrate-below-observed
manner of the oracle's gates. They MUST be pre-registered by the statistical owner before any Rust
V(s) is evaluated against them (engine-spec §7.13.4).

### 10.4 Reference examples (become golden unit tests)

**E-1: compute_dv on a hand-specified V.** One drive, three plays, with V given as a table so the
example is estimator-free:

| Play | s | V(s) | terminal | s′ / terminal value | dV |
|---|---|---|---|---|---|
| 1 | (1, 10, 75) | 1.80 | no | (2, 6, 71) → V = 1.60 | 1.60 − 1.80 = −0.20 |
| 2 | (2, 6, 71) | 1.60 | no | (1, 10, 60) → V = 2.30 | +0.70 |
| 3 | (1, 10, 60) | 2.30 | yes | touchdown, 7 | 7 − 2.30 = +4.70 |

Telescoping: −0.20 + 0.70 + 4.70 = 5.20 = 7 − 1.80.

**E-2: turnover.** A single-play drive at (3, 4, 40) with V = 2.10 ending in a turnover (value 0):
dV = −2.10. The same play ending in a field goal (value 3): dV = +0.90.

**E-3: level shift.** Add 0.5 to every V in E-1: plays 1 and 2 keep −0.20 and +0.70; play 3 becomes
+4.20; the drive sum becomes 4.70 = 7 − 2.30 (§1.4 item 2).

**E-4: situation masks.**

| (down, ydstogo, yardline_100) | `red_zone` | `passing_downs` | `rushing_downs` |
|---|---|---|---|
| (3, 7, 25) | false | true | false |
| (4, 1, 1) | true | true | false |
| (2, 4, 20) | true | false | true |
| (3, 6, 30) | false | false | false |
| (1, 10, 21) | false | false | false |
| (2, 5, 15) | true | false | false |

**E-5: sentinel guard.** A continuing row carrying `(−1, −1, −1)` MUST return
`ContractError::SentinelOnContinuingRow`. The oracle returns V = 7.130 for that state on the
canonical synth.

---

## 11. Superseded-spec mapping

Sections cited as "(superseded)" live in `docs/00-meta/specs/superseded/`.

| Superseded section | Requirement | How this spec satisfies it | Gap |
|---|---|---|---|
| final-build-spec §11.8 | Gradient boosting behind an adapter; `IncrementalBooster` with `continue_training`; `xgb` primary, `linfa-trees` fallback; "the rest of the application must not depend directly on XGBoost-specific types" | §10.2: a `Regressor` trait without continuation, because V(s) is refit per season. Backend per DR-C8 (pure Rust first). The no-vendor-types rule is kept | DR-C7 and DR-C8 ratification |
| final-build-spec §12.1–§12.2 | Daily "Gradient Boosting (continuation / replay)"; three training modes | **Replaced** for V(s): a season-boundary rebuild only (§6.2), the analogue of the "periodic rebuild" mode | — |
| final-build-spec §12.3 | Snapshot and rollback of model files | §3.3 versioned, immutable artifact | Snapshot list in engine-spec §8 must include the V(s) artifact |
| final-build-spec §19.2 | Golden outputs on fixed synthetic data | §10.3 P-V1–P-V7 and the state-grid artifact | — |
| alpha-spec §6.2 (gradient-boosting row) | Boosting for "nonlinear residual correction, interaction effects, availability/workload models, and component-rate models"; "No single method is promoted because it is architecturally required" | V(s) boosting is outside those roles. DR-C7 proposes a deterministic estimator that must earn its place (§10.2 item 6) | — |
| alpha-spec §6.6 | Model-spec contents; rule 1 (implement the documented formula, not a "superficially similar library API"); rule 4 (typed failures); rule 5 (seeded, thread-independent) | This document; §10.2 item 7; §5.4; §5.2 | The estimator's own equations are added at DR-C7 ratification |
| alpha-spec §2.4 | Exact three-season rule | §6.2 preseason-window fit, frozen in season | — |
| alpha-spec §4.5, §12.3 | As-of reconstruction; leakage tests | §6.2 items 2 and 4; P-V6 | Test lives in `evaluation-and-leakage.md` |
| alpha-spec §11.3 | "EPA and success per opportunity" as efficiency features | GRID dV features (§3.4; engine-spec §11.6) | Provider `ep`/`epa` as features: DR-D22 |
| alpha-spec §12.1 | Unit tests for the math core | §7.1, §10.3, §10.4 | — |
