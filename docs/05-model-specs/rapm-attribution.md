---
model-spec-id: MS-RAPM-ATTRIBUTION
status: Draft            # Draft | Approved | Superseded
statistical-owner: statistical owner (role defined in engine-spec §1); approval pending
version: 0.1.0 (2026-10-01, written under consolidation WP P0-01)
supersedes: none. First engine-only spec for this component. Replaces the cautious-nevermore
  prose now archived (non-authoritative) under docs/07-archive/cautious-nevermore/.
---

# Model spec: RAPM attribution (Layer 2), market reconciliation (Layer 3), team strength and the GRID fixed point

This is a contract written before implementation. Under the authority order in engine-spec §1.5
(DR-A2, adopted subject to owner ratification) it is an authority-level-3 document. The statistical
owner approves the equations. The implementing agent may not redefine them (alpha-spec §1.3,
superseded; carried into engine-spec §1).

**How to read this spec**

- **Normative words.** MUST, SHOULD and MAY are requirements on the Rust engine. Everything
  described as "the oracle" is the behaviour of the Python reference at
  `reference/python/` (ADR-012; engine-spec §1.7). Every `reference/python/...:LINE` citation
  uses cautious-nevermore @ `59bce1d` line numbers, which are byte-identical in the import.
- **Proposed decisions.** Statements tagged "(proposed — DR-xx)" are defaults recorded in
  `docs/00-meta/decision-register.md`. The owner has not ratified them, so they are not settled.
- **Synthetic generators.**
  - *Legacy synth* is the oracle's generator as imported. Its defenders are drawn from the
    offense's own roster (KI-NEW-Y0; critic G-1).
  - *Fixed synth* is the defender-fixed generator. It is applied in memory by
    `reference/python/tools/investigations/_common.py:apply_defender_fix`.
  - Every team-, defense- or grade-related number measured on the legacy synth is
    **semantically void**. It is quoted only so the record stays traceable.

---

## 1. Purpose and statistical intent (template: Target)

### 1.1 What the component estimates

| Estimand | Definition | Units / support | Status |
|---|---|---|---|
| Player rating `β_p` | Marginal effect of player `p`, while on the field, on per-play situational value `dV = V(s′) − V(s)` (value-model.md). It is jointly adjusted for every other on-field player and for team offense and defense baselines. Offensive players enter `+1`; defenders enter `−1`. A larger `β_p` is better for the player's own side in both cases. | EP per play; real | Implemented (oracle) |
| Team offense effect `E_off[t]` | Gauge-invariant mean fitted offensive contribution on team `t`'s offensive plays: the intercept plus the exposure-weighted on-field offensive ratings (§4.5). | EP per play; real; positive = good offense | **Proposed — DR-B5** |
| Team defense effect `E_def[t]` | The same construction on `t`'s defensive plays. Positive = good defense. | EP per play; real | **Proposed — DR-B5** |
| Team net strength `N[t]` | `E_off[t] + E_def[t]`: the quantity a point spread prices. | EP per play; real | **Proposed — DR-B5** |
| Defensive matchup grade `G[t]` | `+E_def[t]`. Higher means a tougher matchup. | EP per play; real | **Proposed — DR-B5** (the oracle's sign is inverted, KI-NEW-A2) |

The component's outputs are **signals, not projections**. GRID is a signal provider to Layers B,
D and E and to the priors (engine-spec §6.1 / §6.2; proposed — DR-C2). RAPM coefficients are not
fantasy points. RAPM is also never a live in-season dependency (§2.3; proposed — DR-C1).

### 1.2 What it treats as nuisance (load-bearing intent, preserved verbatim)

The oracle's comments state the intent. The Rust port MUST keep it.

- **The offensive line is a nuisance, not an estimand.** `reference/python/backend/grid/layers.py:4-9`:

  > "Every play is an observation; each player is a column (+1 if on the field for the offense,
  > -1 if on defense). Team-offense / team-defense intercepts absorb line + scheme + baseline so
  > individual skill players are NOT credited with the offensive line's work -- this is how we
  > keep OL as a nuisance rather than an estimand. Ridge with a prior MEAN shrinks toward
  > replacement / a Layer-1 seed."

  The real-data plays contract therefore keeps only offensive *skill* players (QB, RB, WR, TE,
  FB) in `off_players` (`reference/python/backend/grid/nflverse_adapter.py:38-40,199-203`).
  Defense is kept as listed. The Rust plays contract (`docs/03-contracts/plays-contract.md`) MUST
  preserve this filter. OL players MUST NOT become design columns.

- **Market reconciliation is a weak anchor and a falsifiable hook.** `layers.py:11-14`:

  > "extra pseudo-observations tie each team's intercept gap to a market-implied team strength
  > (here, synthetic "closing line"). Anchors the otherwise free team-level constant and is a
  > falsifiable hook (does bottom-up player value reconstruct the market?)."

  The phrase "intercept gap" contradicts the implemented row (§4.4, §8). The intent that
  survives is: anchor the weakly identified team level to the market, and keep the anchor
  falsifiable.

- **The fixed point is light coupling, not a second estimator.** `layers.py:25-27`:

  > "Fixed point: Layer 2 -> defender ratings -> Layer 1 opponent adjustment -> (optional)
  > Layer 1 QB credit re-seeds the QB's Layer-2 prior. RAPM already does most
  > opponent/teammate adjustment jointly, so the loop is light here."

- **Situational slices are labelled, not zero-filled.** Players absent from a situation get the
  ridge prior (0). The docstring says callers "should label 'no data' rather than a real zero
  grade" (`layers.py:437-440`).

### 1.3 Identification facts this spec relies on

These were derived from the design and verified on the code. Both are binding for the port.

1. **Sign algebra.**
   - Defenders enter `X` at `−1`, and the defense intercept enters at `−1` (`layers.py:319-324`).
   - A better defense therefore fits a **larger** `β_p` and a **larger** `γ_def[t]`.
   - A team's net strength is offense quality **plus** defense quality.
   - Two oracle statements reason the opposite way and are wrong:
     - the matchup-grade docstring "more negative intercept" (`reference/python/backend/pipeline/weekly_update.py:62-68`);
     - the reported `team_rating = γ_off − γ_def` (`layers.py:426`).
2. **Gauge freedom.**
   - Suppose each of team `t`'s offensive plays lists exactly `k_off` on-field offensive
     players from `t`, and those players appear in no other plays. Then the shift
     `β_p += a` (for every `p` on `t`'s offensive roster) together with `γ_off[t] −= k_off·a`
     leaves every fitted value unchanged. The same holds for defense with `k_def`.
   - Only the ridge penalty selects the split, so raw intercepts are weakly identified. The
     aggregates `E_off` and `E_def` (§4.5) are invariant to the shift.
   - Synth: `k_off = 5` (`synth.py:32`) and `k_def = 7` (`synth.py:33`).
   - Real data: `k_off` varies with personnel (about 5.97 skill players per play in 2023, per
     the cn-docs inventory), and players who change teams break exactness. Invariance is
     therefore approximate on real data.
   - Evidence (fixed synth, `tools/investigations/sign_check.py --synth fixed`): raw
     `γ_off + γ_def` vs planted net gives corr 0.431. The gauge-invariant `E_off + E_def` vs
     exposure-weighted planted net gives corr 0.959.

---

## 2. Position in the engine

### 2.1 Signal stack (engine-spec §6.2)

```text
plays contract (participation) ─┐
V(s) → dV  (value-model.md) ────┴─► Layer 2 RAPM (+ Layer 3 market rows)   ◄── this spec
                                       │  β_p (players), γ_off/γ_def, E_off/E_def/N (proposed)
                                       ├─► Layer-1 opponent adjustment (layer1-credit.md: defender ratings)
                                       ├─► priors / features: offseason talent covariate (cross-league-priors.md,
                                       │     projection-stack.md, engine-spec §6.5, §11.6)
                                       ├─► Layer B team net strength N[t] anchored to market at lock (DR-B5)
                                       └─► Layer E defensive matchup grade G[t] = +E_def[t] (DR-B5)
Kalman (state-space-kalman.md) observes weekly Layer-1/1′ credit, NEVER the cumulative RAPM β (§8, KI-NEW-W3).
```

### 2.2 Layers served (engine-spec §6.1; proposed — DR-C2, DR-C3)

| Consumer | What RAPM provides |
|---|---|
| Layer B (team environment) | `N[t]` as a team-strength input, reconciled to the market line at lock |
| Layer D (efficiency) | Offseason player rating as a talent covariate in EB-shrunk component-rate models (proposed — DR-C3, option (a)) |
| Layer E (matchup) | `G[t] = +E_def[t]` |
| §6.5 priors | `estimate_equivalency` consumes the `ratings` frame (`reference/python/backend/grid/priors.py`; cross-league-priors.md) |
| Layer-1 | Defender ratings for the opponent-adjustment feature (layer1-credit.md §4) |

### 2.3 Live vs offseason (engine-spec §6.3; proposed — DR-C1)

- Participation for season `S` is published **once, after `S`'s postseason**. This is stated in
  alpha-spec §4.1 (superseded), Participation row: "No for current in-season use". It is also
  stated in alpha-spec Appendix A. engine-spec §4.5 adds the publication-lag axis.
- Therefore (proposed — DR-C1):
  - Production RAPM is fitted **only at season boundaries**, on completed seasons whose
    participation is published.
  - Its outputs seed priors and features.
  - In-season, the Kalman observes the participation-free Layer-1′ credit (layer1-credit.md §4.8).
- The oracle's walk-forward folds current-season participation into RAPM at every origin
  (`reference/python/backend/validation/backtest.py:177-199`). Backtests built that way MUST be
  labelled **research-only** (alpha-spec §6.2 RAPM row: "research-only if live feature parity is
  absent"; engine-spec §3.3).

### 2.4 Contracts and neighbouring specs

- Inputs: `docs/03-contracts/plays-contract.md` (participation, teams, `dv`, `season`, `week`,
  `season_type`); identity (engine-spec §4.4); the market line at lock (engine-spec §4.3).
- Outputs: `docs/03-contracts/engine-output-contract.md` (§3.2 below).
- Parity: `docs/03-contracts/parity-fixture-contract.md`; `reference/python/PARITY.md` (correction
  ledger, divergences, tolerance table).
- Neighbours: value-model.md (dV), layer1-credit.md, state-space-kalman.md,
  cross-league-priors.md, synthetic-world.md (planted truth for team and defense),
  projection-stack.md, evaluation-and-leakage.md (walk-forward, two-path, three-season window).

---

## 3. Inputs and outputs (template: Inputs)

### 3.1 Inputs

| Input | Oracle form | Units / type | Null and failure semantics (oracle → required) | As-of rule |
|---|---|---|---|---|
| `off_players` per play | tuple of IDs (`layers.py:266,278`) | GSIS ID strings (real); ints (synth) | Empty tuple allowed. IDs not in the column universe are **dropped with a WARNING** (`layers.py:277-304`; PR #53 C4) → MUST be a typed error at this stage (§5.4) | Participation of season `S` usable only at as-of ≥ its publication timestamp (engine-spec §4.5; proposed — DR-C1) |
| `def_players` per play | tuple of IDs | as above | as above | as above |
| `off_team`, `def_team` | team code (`layers.py:309-314`) | string (real) / int (synth) | A code absent from the **players** frame raises a bare `KeyError` (no guard) → MUST be a typed error | as-of of the play |
| `dv` | float column (`layers.py:327`) | EP per play | Missing column raises bare `KeyError` (KI-G12) → typed contract error. NaN is not checked → MUST be a typed error | V(s) frozen per data version and as-of (value-model.md; KI-NEW-W4) |
| `season`, `week` | ints | — | The oracle RAPM ignores `season` (§6) → Rust keys every block by `(season, week)` | — |
| `season_type` | absent in the oracle contract | — | Postseason plays enter RAPM today (KI-NEW-V0a) → RAPM MUST consume only the season types the plays contract admits | — |
| Column universe `players` | `player_id, team, position` (+`is_starter, ability` synth only) (`layers.py:252-258,417-422`) | — | Columns: players sorted by `str(id)`; teams = `sorted(players.team.unique())`. **The team list comes from the players frame, not the plays.** | Universe = as-of roster/identity snapshot over the fit window (engine-spec §4.4; reconcile-code-first C21) |
| Market `s_t` | `{team: strength}` | **must be in the estimand's units, EP/play net strength** | Oracle absent-team handling differs at three sites (§4.4) → absent teams are simply **not anchored** | Line **as of lock** (engine-spec §4.3; proposed — DR-B5). The spread→EP/play scale is open (DR-D10) |
| `prior_mean_players` | `{pid: m_p}` (`layers.py:373-376`) | EP per play | An unknown pid raises bare `KeyError` → typed error | as-of |
| `lambda_by_pos` | `{position: multiplier}` (`layers.py:381-386`) | dimensionless | Unknown position → multiplier 1 | — |

### 3.2 Outputs (template: Target)

**Oracle outputs** (`run_rapm`, `layers.py:362-428`):

- `beta`: length `nCols` (`layers.py:410`).
- `colidx = {p_col, t_off, t_def, nP, teams, nCols[, interactions]}` (`layers.py:328`).
- `ratings_df[player_id, rating, team, position(, is_starter, ability)]` (`layers.py:419-422`).
- `team_df[team, team_rating = γ_off − γ_def]` (`layers.py:424-427`). This is a defect: KI-NEW-A1.

**Required engine outputs** (shape owned by `docs/03-contracts/engine-output-contract.md`, engine-spec §5.5):

| Field | Definition | Notes |
|---|---|---|
| `player_rating` | `β_p` per player in the window universe | `no_data = true` when the player has zero exposure in the window or slice. Never a silent 0 (`layers.py:437-440`) |
| `exposure_off`, `exposure_def` | play counts per player in the window | needed to interpret `β_p` and to build `E_*` |
| `team_off_effect` | `E_off[t]` | proposed — DR-B5 |
| `team_def_effect` | `E_def[t]` | proposed — DR-B5 |
| `team_net_strength` | `N[t] = E_off[t] + E_def[t]` | proposed — DR-B5. Replaces the oracle's `γ_off − γ_def` |
| `def_matchup_grade` | `G[t] = +E_def[t]`, higher = tougher | proposed — DR-B5. Replaces `−γ_def` |
| `market_residual` | `N[t] − s_t` for anchored teams | the falsifiable hook of `layers.py:13-14` |
| `solver_diagnostics` | see §5.3 | MUST accompany every solve |
| provenance | window seasons, as-of, data version, column-map version, hyperparameters (§4.9) | engine-spec §8.5 / §8.8 |

### 3.3 Explanation fields (template: Explanation fields; engine-spec §6.7)

- **`offseason_rapm_rating`** (EP per play over seasons `S−k..S−1`).
  - Positive means better for the player's side.
  - When it is used as a Layer-D covariate (DR-C3), its contribution is reported in the
    covariate's own units by the consuming model.
- **`team_net_strength` decomposition.**
  - `N[t] = (γ_off[t] + γ_def[t]) + exposure-weighted on-field offensive ratings
    + exposure-weighted on-field defensive ratings`.
  - The intercept share is reported separately from the player share. On real data the
    intercept absorbs most value (KI-NEW-A3), and users must see that.
- **`def_matchup_grade`**.
  - Sign: higher = tougher opponent defense.
  - Units: EP per play.
  - Reported with its window and as-of.

---

## 4. Model and equations (template: Equations, Priors, Constraints)

### 4.1 Design matrix (`build_design`, `layers.py:226-344`)

The rows are the plays `i = 1..n`. The columns are ordered as
`[players sorted by str(id) (nP) | team_off for each sorted team (T) | team_def for each sorted team (T)]`
(`layers.py:252-258`), so `nCols = nP + 2T`.

```text
X[i, col(p)]          = +1   for p ∈ off_players(i)            (layers.py:320)
X[i, col(p)]          = −1   for p ∈ def_players(i)            (layers.py:321)
X[i, t_off[off_team(i)]] = +1                                  (layers.py:322)
X[i, t_def[def_team(i)]] = −1                                  (layers.py:323)
y_i = dv_i                                                     (layers.py:327)
```

Implementation facts the port MUST reproduce, or MUST replace with a typed error:

- **Duplicate entries.** The design is built as CSR from (row, col, val) triplets
  (`layers.py:326`), and duplicates are **summed**. An ID listed twice in one play's tuple yields
  entry ±2.
  - The Rust plays contract MUST reject duplicate IDs within a play.
  - The design builder MUST treat a duplicate as a contract violation, not as weight 2.
- **Unknown IDs.**
  - Fast path when all IDs are known (`layers.py:283-285`).
  - Otherwise the slow path drops the unknown entries and logs `"build_design: dropped %d player
    entries with unknown IDs"` (`layers.py:286-304`). The play's row still exists, with fewer
    nonzeros.
  - Required behaviour: §5.4.
- **Optional WR×CB interaction block** (`layers.py:146-223,334-342`).
  - The CB proxy is `"CB"` when any player has that position; otherwise `"DEF"` (`layers.py:167-168`).
  - Pairs `(wr, cb)` co-occurring on at least `min_pair_plays` plays are kept, sorted by
    `(str(wr), str(cb))`, and appended after the base block with entries `+1`.
  - `colidx["interactions"]` and `nCols` are updated.
  - **`run_rapm` never requests this block** (it calls `build_design(plays, players)` at
    `layers.py:369`), so the interaction penalty branch at `layers.py:393-395` is unreachable
    from any oracle solve path. Interactions are exercised only by
    `tests/grid/test_design_interactions.py`, which builds its own mask.
  - The synth never plants a WR×CB effect: `cb_split` only relabels positions (`synth.py:72-79`).

### 4.2 Objective: the generalized ridge (`_solve_ridge_prior` `layers.py:347-359`; `run_rapm` `:362-410`)

```text
β̂ = argmin_β  ‖y − Xβ‖²
            + w_mkt · Σ_{t ∈ T_mkt} (a_tᵀβ − s_t)²
            + (β − m)ᵀ Λ (β − m)

Λ = λ · diag(μ)
μ_j = π(pos_j)        for player columns (π = lambda_by_pos multiplier, default 1)   (layers.py:380-386)
μ_j = 0.05            for every team_off and team_def column                        (layers.py:387-389)
μ_j = 10.0            for interaction columns (unreachable via run_rapm, §4.1)      (layers.py:393-395)
m_j = prior mean (0 unless prior_mean_players sets it)                              (layers.py:373-376)
a_t = e_{t_off[t]} + e_{t_def[t]}       (oracle Layer-3 row, [+1,+1])               (layers.py:403-405)
```

Normal equations, exactly as the oracle forms them:

```text
A = XᵀX + w_mkt Σ_t a_t a_tᵀ + Λ          (XtX dense: layers.py:397; market: :407; Λ: :352-353)
b = Xᵀy + w_mkt Σ_t a_t s_t + Λ m         (layers.py:398, :408, :354)
A β = b
```

- **Plain ridge is a special case.** The final-build-spec §11.3 (superseded) objective
  `‖y−Xβ‖² + λ‖β‖²` is this objective with `m = 0`, `μ = 1` and `w_mkt = 0`.
- **The objective MUST be stated in this generalized form in engine-spec §6.4.** This amends
  final-build-spec §11.3 by ADR (reconcile-code-first C11). The domain controls (home field,
  garbage time, overtime, minimum exposure) are open parameters (DR-C5, §9).
- **Play weights.** The oracle's play weights are uniform (`W = I`). The Rust primitive MUST
  accept a diagonal `W` (final-build-spec §11.2 "Weighted observations"). Then `XᵀWX` and `XᵀWy`
  replace `XᵀX` and `Xᵀy`. `W = I` MUST reproduce the oracle.

### 4.3 Solver (oracle)

```text
cond = np.linalg.cond(A)                        # 2-norm condition number via SVD   (layers.py:355)
if cond > 1e10:  log WARNING "ill-conditioned … using lstsq fallback"
                 return np.linalg.lstsq(A, b, rcond=None)[0]                       (layers.py:356-358)
else:            return np.linalg.solve(A, b)   # LU                               (layers.py:359)
```

- **When `A` is SPD.** If `λ > 0` and every `μ_j > 0`, then `A` is symmetric positive definite:
  `XᵀX` is PSD, the market term is PSD, and `Λ` is positive diagonal. In that case the `lstsq`
  branch is reachable only through numerical ill-conditioning.
- **Measured conditioning** (2026-10-01; §7.6):
  - `cond(A)` = 3.14e3 on the canonical synth, no market;
  - 5.83e2 on the canonical synth with market;
  - 2.18e3 on real 2023 regular season, full-roster universe.

  The `1e10` fallback therefore never triggers on realistic inputs. It triggers in the test
  `test_solve_ridge_prior_lstsq_fallback` (`tests/grid/test_rapm_robustness.py:10-21`), which
  uses rank-1 `XᵀX` with `λ = 1e-15`.
- **Provenance.** The `lstsq` fallback was added deliberately as audit item C3 in PR #53
  ("Conditioning check + lstsq fallback in RAPM ridge solver"). The Rust replacement is a typed
  failure (§5.4; proposed — DR-B6), and the ADR that adopts it MUST cite PR #53 C3 as the
  decision being reversed (critic G-3).

### 4.4 Layer 3: market pseudo-observations

| Site | Row | Absent team |
|---|---|---|
| `layers.py:400-408` (`run_rapm`) | `[+1 at t_off, +1 at t_def]` (`:404-405`). The comment at `:400` says `(gamma_off[t] - gamma_def[t]) ~ market`, which **contradicts the code** | `market_strength[t]` raises `KeyError` (`:406`) |
| `reference/python/backend/validation/backtest.py:101-112` (`solve_rapm`) | `[+1, +1]` (`:108-109`) | skipped: "no false zero prior" (`:105-106`) |
| `reference/python/backend/pipeline/weekly_update.py:269-276` | `[+1, +1]` (`:272-273`) | **anchored to 0.0** via `market_strength.get(t, 0.0)` (`:274`) |

- **Where market rows enter.** In all three sites the market rows are added to `A` and `b`
  **at solve time**. They are not part of the persisted sufficient statistics.
  - `weekly_update` adds them after `save_accumulators` (`:259` vs `:269-276`).
  - `solve_rapm` copies `XtX` before adding them (`backtest.py:102-103`).
- **Scale.**
  - In the synth harness, `s_t` is the planted legacy `team_strength` (mean of offensive starter
    abilities minus mean of DEF starter abilities, `synth.py:285-294`) plus N(0, 0.01) noise
    (`tests/grid/test_tier0_recovery.py:33-34`). Its SD is 0.0157, in ability units.
  - The estimand it anchors lives in dV units. Measured SD of `E_off + E_def` is 0.268 (legacy)
    and 0.208 (fixed) EP per play, and mean `|γ_off + γ_def|` is 0.169 / 0.118.
  - So the synth anchor is mis-scaled by roughly an order of magnitude, independent of its sign.
  - The anchor weight is `w_mkt = 40` pseudo-plays against roughly 1,400 offensive and 1,400
    defensive plays per team on the canonical synth. Layer 3 is therefore a weak nudge
    (consistent with KI-NEW-A1).

**Required semantics** (proposed — DR-B5; critic X-2 adjudication):

1. The `[+1, −1]` row (the "G1 fix" queued in CN docs) is **rejected**. It only aligns the code
   with the legacy generator's defender bug (KI-NEW-Y0).
2. The market anchors **net strength** `N[t]`, using the gauge-invariant row
   (`n^off_{t,p}`, `n^def_{t,p}` = player `p`'s exposure on `t`'s offensive and defensive plays in
   the fit window; `N_off(t)`, `N_def(t)` = `t`'s offensive and defensive play counts):
   ```text
   a_t = e_{t_off[t]} + e_{t_def[t]}
       + Σ_p (n^off_{t,p} / N_off(t)) e_p
       + Σ_p (n^def_{t,p} / N_def(t)) e_p
   ```
3. `s_t` is the market-implied net strength **using the line as of lock** (engine-spec §4.3),
   converted into EP per play over the window. The conversion is not defined anywhere yet
   (DR-D10).
4. A team without a line at lock is **not anchored**. The oracle's three absent-team behaviours
   MUST NOT be reproduced.
5. Measured effect (fixed synth, single solve; §7.6):
   - the gauge-invariant row gives corr(`E_off+E_def`, exposure-weighted planted net) = 0.9589;
   - with no market the same corr is 0.9587.

   The anchor's value is the reconciliation hook (`market_residual`), not recovery.

### 4.5 Team outputs and the matchup grade

**Oracle.**

- `team_rating[t] = γ_off[t] − γ_def[t]` (`layers.py:426`).
- Matchup grade `= −γ_def[t]`:
  - `weekly_update.py:84`, written as `def_strength = -float(beta[col])`;
  - the docstring at `:62-68`;
  - `backtest.py:48-51,185-186` (`def_grades`).

Both are wrong under §1.3: KI-NEW-A1 and KI-NEW-A2 (CONFIRMED by
`tools/investigations/defsign_planted.py`). On a planted-defense league the ELITE defense gets
`β_def = +0.409` and shipped grade −0.409, while the WEAK defense gets `β_def = −0.382` and
grade +0.382.

**Required** (proposed — DR-B5). `n^off_{t,p}` is player `p`'s count of on-field offensive
plays for team `t` in the window, and `N_off(t)` is `t`'s count of offensive plays (likewise for
defense):

```text
E_off[t] = γ_off[t] + (1/N_off(t)) Σ_{i: off_team(i)=t} Σ_{p ∈ off_players(i)} β_p
         = γ_off[t] + Σ_p (n^off_{t,p} / N_off(t)) β_p
E_def[t] = γ_def[t] + (1/N_def(t)) Σ_{i: def_team(i)=t} Σ_{p ∈ def_players(i)} β_p
N[t]     = E_off[t] + E_def[t]
G[t]     = +E_def[t]                       (higher = tougher)
```

- The exposure counts are part of the persisted window statistics (§6), so `E_*` never
  requires replaying plays.
- Evidence (fixed synth, `sign_check.py --synth fixed`):
  - corr(`E_def`, exposure-weighted planted defense) = 0.943 to 0.945 across market settings;
  - corr(`E_off`, planted offense) = 0.962 to 0.965;
  - the shipped `−γ_def` vs mean DEF-starter ability gives −0.529 (no market), i.e. inverted.
- **Tier-2 has never validated any grade on engine output** (KI-NEW-V2). The Tier-2 synth test is
  tautological: it builds `points_allowed = 20 − 30·grade` from the grades themselves
  (`tests/validation/test_verdict.py:47-51`). The unit tests plant the wrong sign by hand
  (`tests/pipeline/test_weekly_update.py:286-366`).

### 4.6 Defender ratings and opponent adjustment

"Defender ratings" are the `β_p` of players appearing in `def_players`. They are consumed by
Layer 1 as `D_i = Σ_{p ∈ def_players(i)} β_p` (`layers.py:488-494,507`).

`D_i` **omits `γ_def[def_team(i)]`**, so it is gauge-dependent (§1.3). The required per-play
defensive effect is specified in layer1-credit.md §4.3 and §8.

### 4.7 The fixed point `fit()` (`layers.py:591-610`)

```text
prior ← None
for it in 0 .. n_iter−1:                                         (n_iter = 3; layers.py:591,596)
    ratings, team_df, β, colidx ← run_rapm(plays, players, market, λ, w_mkt,
                                           prior_mean_players = prior, lambda_by_pos)   (:597-600)
    rlook ← {player_id: rating}                                  (:601)  # defender ratings
    qb_weekly ← layer1_qb_weekly(plays, rlook, focus_qb)         (:603)  # n_splits=5, seed=0 defaults
    prior ← {focus_qb: mean_w(qb_weekly.qb_credit)}              (:605)  # unweighted mean over weeks
return ratings, team_df, qb_weekly, β, colidx of the LAST iteration   (:609-610)
```

- **It is not a convergence-checked fixed point.** It runs a fixed iteration count with no
  tolerance. The prior computed in the final iteration is discarded.
- **Cost.** Each iteration costs one RAPM solve plus one Layer-1 cross-fit (5 GBM fits), so
  `n_iter = 3` means 15 GBM fits.
- **Scope.**
  - Only `focus_qb` is re-seeded (KI-G14).
  - The re-seed value is a per-play *residual mean* (Layer-1 credit), not a RAPM coefficient,
    so the two scales differ (reconcile-code-first §1.4).
  - Production paths have **no** Layer-1 → Layer-2 coupling: `run_rapm`, `walk_forward`,
    `weekly_update`.
  - `fit()` is used by the demo, Tier-0, the golden master, determinism and calibration only.
- **`verbose=True` is the default** (`layers.py:592`). It computes
  `np.corrcoef(ratings.rating, ratings.ability)` (`:606-608`), so it **fails on real rosters**,
  which carry no `ability` column.
- **Required** (DR-D11).
  - Either the re-seed is generalized to every player with Layer-1 credit and given a
    convergence criterion, or the "fixed point" claim is dropped from engine-spec §6.2 and
    `n_iter` is fixed at 1.
  - Until decided, the Rust port MUST implement `fit` exactly as above for parity, with an
    explicit `n_iter`, and MUST NOT depend on synth-only columns.

### 4.8 Situation RAPM (`run_situation_rapm`, `layers.py:432-484`)

- For each mask from `situations.classify` (`red_zone`, `passing_downs`, `rushing_downs`, and
  `two_minute` only when `quarter_seconds_remaining` is present), the oracle solves an
  independent `run_rapm` on the slice (`layers.py:475-483`). It skips slices with fewer than
  `min_plays = 200` plays (`:479-481`).
- It raises `ValueError` without participation columns (`:469-473`).
- `weekly_update` uses `MIN_PLAYS = 50` with **no market rows** and a mask built inline
  (`weekly_update.py:47,435-480`).
- The situation vocabulary and `two_minute` definition are owned by DR-C13 (proposed: code set
  canonical; `two_minute` via `half_seconds_remaining`; KI-NEW-A4, KI-#57).
- Situation and interaction RAPM are **research-only**, because they need participation
  (alpha-spec §6.2; cn-docs: "Situation RAPM is exactly as blocked as base RAPM without
  participation").

### 4.9 Constants and hyperparameters

Provenance key:

- **hand-set v0**: present at `f3b641f`, the oldest commit in the shallow CN clone (2026-06-21).
  The v0 upload is `6b0eeee` on GitHub only (critic G-3). There is no calibration record.
- **hand-set (PR #N)**: introduced in that PR, with no calibration record.
- **calibrated on the legacy synth**: tuned against gates measured on the defender-bug generator.

| Constant | Value | Defined at | Provenance |
|---|---|---|---|
| `λ` (`lam`) | 120.0 | `layers.py:362` (`run_rapm`), `:592` (`fit`); `backtest.py:39`; `weekly_update.py:41` | hand-set v0. The Tier-0 floors were set below values observed with it (legacy synth). Real-data appropriateness is unknown (KI-NEW-A3, KI-NEW-P4) |
| `μ_team` | 0.05 | `layers.py:388-389`; `backtest.py:41`; `weekly_update.py:266-267,476-477` | hand-set v0 |
| `w_mkt` | 40.0 | `layers.py:362,592`; `backtest.py:40`; `weekly_update.py:42` | hand-set v0 |
| Market row | `[+1, +1]` on `(t_off, t_def)` | `layers.py:404-405`; `backtest.py:108-109`; `weekly_update.py:272-273` | hand-set v0. Required replacement in §4.4 |
| `μ_interaction` | 10.0 | `layers.py:395` (unreachable via `run_rapm`) | hand-set (Phase-4 T7, PR #56 era) |
| `min_pair_plays` | 30 | `layers.py:230` | hand-set (Phase-4 T7) |
| `lambda_by_pos` | none by default | `layers.py:363,381-386` | hand-set design hook (Phase-3 plan, lost: "softer regularization for QB (more volatile) vs RB"; critic G-3). Never passed by a production solver (KI-#32). Test values only (`tests/grid/test_attribution.py:40,51,69`) |
| Condition threshold | 1e10 | `layers.py:356` | hand-set (PR #53 C3) |
| `n_iter` | 3 | `layers.py:591`; `tests/grid/golden_master.py:31` | hand-set v0 |
| Situation `min_plays` | 200 (`run_situation_rapm`); 50 (`weekly_update`) | `layers.py:432`; `weekly_update.py:47` | hand-set (Phase 4) |
| Prior mean `m` | 0, except the `fit` re-seed | `layers.py:373-376,605` | design ("ridge toward replacement") |
| Synth market noise | N(0, 0.01), seed 1 | `tests/grid/test_tier0_recovery.py:33-34`; `golden_master.py:30,69-70` | test harness only |
| Column order | players `sorted(key=str)`; teams `sorted(players.team.unique())` | `layers.py:252,255` | design. Canonical-order PR (Phase-4 T1) |

### 4.10 Priors (template: Priors)

- **Prior family.** The prior is the Gaussian implied by the ridge, `β ~ N(m, σ²Λ⁻¹)` with
  `σ²` unmodelled. RAPM is a penalized point estimator in the oracle. There is no posterior
  variance output.
- **Prior means.**
  - `m = 0` ("replacement"), except the `fit` re-seed (§4.7).
  - Priors from cross-league equivalency are **not** wired into RAPM on any oracle path
    (compare KI-NEW-P2).
- **Required (open).**
  - Whether RAPM should take feeder/rookie prior means from cross-league-priors.md is open
    under DR-C9 and DR-D12.
  - Any posterior-variance output (for example from the diagonal of `A⁻¹` scaled by a residual
    variance estimate) is **not** specified. The Rust port MUST NOT emit one until an approved
    spec revision defines it.

### 4.11 Constraints (template: Constraints)

- `λ > 0` and `μ_j > 0` for every column. Otherwise the configuration is rejected
  (`InvalidPenalty`). Never solve a possibly singular system.
- `w_mkt ≥ 0`, and `s_t` finite. `T_mkt` contains only teams that have a line at lock.
- `X`, `y`, `m` and `s` are finite. Any NaN or Inf is a typed error, never clamped.
- Column-map invariants:
  - player columns precede team columns;
  - the map is versioned;
  - persisted sufficient statistics carry the map, and a solve whose map differs from the
    statistics' map is a typed error (§6).
- `E_off`, `E_def` and `N` are defined only for teams with `N_off(t) > 0` and `N_def(t) > 0`.
  Otherwise the output is `no_data`, never 0.

---

## 5. Algorithm, numerics and determinism (template: Seed policy, Tolerances)

### 5.1 Sizes and storage

| Frame | plays | `nCols` | `nnz(XᵀX)` | Note |
|---|---|---|---|---|
| Canonical legacy synth (`load_synthetic()`) | 16,825 | 312 (288 players + 24 team columns) | 11,892 | dense is trivial |
| Real 2023 regular season, full-roster universe | 33,836 | 3,154 | 285,128 (2.9% dense) | dense `XᵀX` ≈ 80 MB per block |
| Real 2023 regular season, on-field universe only | 33,836 | 1,666 | 285,128 | — |

The engine needs per-season blocks (§6) plus per-situation blocks, so the Rust engine SHOULD
store `XᵀWX` sparse (final-build-spec §11.1: `sprs`). Dense storage MAY be used for the parity
oracle path.

### 5.2 Solvers (P1-06 generalized-ridge primitive)

The Rust primitive MUST provide two solvers. Both solve the same `A β = b` from §4.2.

1. **Dense Cholesky.** This is the test oracle and is used for small `nCols`. `A` is SPD under
   §4.11, so Cholesky failure means numerical breakdown, which is a typed error.
2. **Jacobi-preconditioned conjugate gradient** on sparse `A`. It MUST emit the diagnostics
   `converged, iterations, residual_norm, tolerance, regularization_lambda` (final-build-spec
   §11.3, superseded, kept) plus a condition estimate.

**Measured with SciPy Jacobi-PCG** (relative-residual stopping `‖Aβ−b‖/‖b‖ ≤ tol`; §7.6):

| System | tol | iterations | rel. residual | rel. solution error vs dense |
|---|---|---|---|---|
| Canonical synth, no market | 1e-8 | 118 | 8.9e-9 | 2.1e-7 |
| ″ | 1e-10 | 148 | 4.0e-11 | 1.06e-9 |
| ″ | 1e-12 | 168 | 7.5e-13 | 1.4e-11 |
| ″ | 1e-14 | 188 | 4.6e-15 | 2.0e-13 |
| Canonical synth, market `w = 40` | 1e-10 | 106 | — | 8.8e-10 |
| Real 2023, full-roster universe | 1e-10 | 231 | — | 4.8e-10 |

- The relative solution error is 10 to 24 times the residual tolerance, because it scales with
  `cond(A)`. This bears on the Class B criterion (§10.3).
- Cholesky vs LU agree to 1.1e-14 (synth) and 6.5e-15 (real 2023) relative.

### 5.3 Diagnostics

Every solve MUST return the following, persisted with the output (engine-spec §8.8; final-build-spec §11.3):

- `solver ∈ {cholesky, pcg}`, `converged`, `iterations`, `residual_norm` (relative), `tolerance`;
- `regularization_lambda`, the `μ` summary (per column class), `w_mkt`, `|T_mkt|`;
- `cond_estimate` (cheap estimate, for example Lanczos or Hager; **not** a full SVD, which costs
  20.6 s vs 0.70 s for the solve on real 2023: KI-NEW-A6);
- `n_plays`, `nCols`, `n_unknown_participants` (MUST be 0 to proceed, §5.4), column-map version.

### 5.4 Typed failures (alpha-spec §6.6 rule 4; proposed — DR-B6)

The Rust engine MUST return a typed error, never a silent default, and never fall back to
`lstsq`, for each of:

| Condition | Oracle behaviour | Required |
|---|---|---|
| Ill-conditioned `A` (`cond_estimate` > threshold; threshold value open, DR-B6) | `lstsq` with WARNING (`layers.py:356-358`) | `RapmError::IllConditioned{cond}`. Blocks promotion. Divergence recorded in `reference/python/PARITY.md` |
| PCG not converged within the iteration cap | n/a (dense) | `RapmError::NotConverged{iterations, residual}` |
| Cholesky breakdown | n/a | `RapmError::NotPositiveDefinite` |
| `λ ≤ 0` or any `μ_j ≤ 0` | allowed | `RapmError::InvalidPenalty` |
| Participant ID not in the column universe | dropped with WARNING (`layers.py:286-304`) | `RapmError::UnknownParticipant{count, plays}` at the RAPM stage. Upstream ingestion quarantines bad rows (final-build-spec §9.4 partial acceptance) so the universe and the plays agree before the design is built |
| Team code not in the universe | bare `KeyError` | `RapmError::UnknownTeam` |
| `dv` column missing, or NaN or Inf anywhere | bare `KeyError` (KI-G12) / unchecked | `ContractError` |
| Unknown pid in `prior_mean_players` | bare `KeyError` | `RapmError::UnknownPrior` |
| NaN prior mean, e.g. `fit()` with a `focus_qb` who has no plays: `qb_weekly` is empty, so its mean is NaN (`layers.py:603-605`) | **every** coefficient becomes NaN, because `Λm` is a dense `np.diag(...) @ m` product (`layers.py:352-354`) and `0·NaN = NaN`. No error is raised | `ContractError::NonFinitePrior`. `Λm` is computed element-wise |
| Duplicate ID within one play | entry summed to ±2 | `ContractError::DuplicateParticipant` |
| Accumulator column map ≠ solve column map | silent reinit (KI-NEW-W2) | `StateError::ColumnMapMismatch` (§6) |

### 5.5 Determinism and seed policy

- **No randomness.** The RAPM solve is deterministic. The only RNG in the oracle harness is the
  synth market noise (`default_rng(1)`), which belongs to synthetic-world.md.
- **Column order is part of the contract.** Players are sorted by their canonical string form,
  byte-wise lexicographic. For synth integer IDs this is `str(int)` order, so `"10" < "2"`.
  Teams use the same rule. The Rust port MUST reproduce this order exactly; the golden layouts
  depend on it (`tests/grid/test_layers.py:51-87`).
- **Summation order.** `XᵀWX` and `XᵀWy` are accumulated per `(season, week)` block in
  chronological order, single-threaded on the parity path. Parallel accumulation MAY be used in
  production only if it is shown to stay within Class A tolerance (engine-spec §8.19).
- **In-process evidence.** `tests/grid/test_determinism.py:50-60` asserts two in-process
  `fit()` runs agree to `atol 1e-9` on ratings and team ratings.

---

## 6. Incremental and online behaviour

### 6.1 Oracle accumulators (`layers.py:42-142`)

- `init_accumulators(n)` returns `(0ₙₓₙ, 0ₙ)` (`:62-64`).
- `accumulate` adds `(XᵀX).toarray()` and `Xᵀy` (`:67-76`).
- `fit_from_accumulators` solves with `_solve_ridge_prior` (`:79-87`).
- `save_accumulators` writes `.npz{XtX, Xty, week:[int], player_order: unicode array}` with
  pickle disabled. The path is injectable, and a key exists per situation (`:90-117`, `:45-58`).
- `load_accumulators` returns `(XtX, Xty, last_week, player_order)` or `None` (`:120-142`).

### 6.2 Equivalence properties (keep)

The oracle builds columns from the fixed `players` frame, not from the per-week plays
(`backtest.py:16-20`). Per-week contributions are therefore additive, and the incremental solve
equals the from-scratch solve.

| Property | Asserted | Observed (2026-10-01, threads = 1, legacy synth) |
|---|---|---|
| Two-week accumulation equals a joint build | `atol 1e-10` (`tests/grid/test_incremental.py:38-57`) | — |
| Incremental walk-forward vs from-scratch at the final origin | `rtol = atol = 1e-8` (`tests/validation/test_backtest.py:67-92`) | max abs diff **4.8e-15** |
| Two-path: `walk_forward` vs `weekly_update` after weeks 1..W−1 (W = 8) | `rtol = atol = 1e-6` (`tests/validation/test_leakage_guards.py:150-197`) | **0.0** (bit-identical) |

The Rust engine MUST keep both equivalences: incremental equals batch, and the offline harness
equals the production path. engine-spec §8.19 sets the tolerance, and evaluation-and-leakage.md
owns the harness.

### 6.3 Oracle defects in the incremental path (MUST NOT reproduce)

| Defect | KI | Required |
|---|---|---|
| The watermark is the week only. `if last_week >= week: skip` (`weekly_update.py:240-244`) skips week 1 of a new season forever | KI-NEW-W1 | Watermark keyed by `(season, week)`, with an explicit carry-over policy |
| Any change in roster size or order reinitializes the accumulators, discarding history (`weekly_update.py:245-252,453-464`) | KI-NEW-W2 | Stable, versioned player index. New players → zero-padding of rows and columns. A reorder is a typed error, not a reinit |
| Re-running an earlier week silently no-ops, so stat corrections cannot apply | KI-A9 | Idempotent recompute from `(season, week)`-keyed blocks (replace the block, re-sum) |
| The `walk_forward` watermark guard fires only within a season (`backtest.py:178-181`), and accumulation runs across seasons with no window | KI-V2 | Window per §6.4; watermark checked across seasons |
| `weekly_update` refits V(s) on the whole season (`weekly_update.py:210-214`), leaking future weeks into dV in a backfill, and re-basing earlier blocks' `Xᵀy` | KI-NEW-W4 | V(s) frozen per data version and as-of (value-model.md). A V(s) version change invalidates and rebuilds every block |
| The Kalman is fed the cumulative RAPM `β` with `snaps = 1` (`weekly_update.py:307-309`) | KI-NEW-W3, KI-#24 | The Kalman observes weekly Layer-1/1′ credit (layer1-credit.md; proposed — DR-C10) |
| Absent market teams anchored to 0 (`weekly_update.py:274`) | (new; §8) | §4.4 item 4 |

### 6.4 Required window semantics (proposed — DR-C6, with DR-C1)

- **Sufficient statistics are stored per block.** For each completed season `s`, persist
  `XᵀWX_s`, `XᵀWy_s`, exposure counts `n^off_s`, `n^def_s`, `N_off_s`, `N_def_s`, the
  column-map version, the V(s) version and the data version. Storage follows engine-spec §8.5;
  per proposed DR-C15, SQLite is the source of truth and bulk arrays are registered in its
  manifest.
- **Market rows and prior means are never persisted** in these statistics. They are added at
  solve time.
- **Window.**
  - Production (offseason) RAPM for season `S` uses the exact sum of blocks
    `s ∈ {S−3, S−2, S−1}` (alpha-spec §2.4 preseason rule, superseded; engine-spec §2.4).
  - Under DR-C1, in-season origins `(S, W)` reuse that preseason fit, because season-`S`
    participation is unpublished.
  - A **research-only** mode MAY add in-season blocks `S[1..W−1]` when participation is
    available historically. Every output of that mode MUST carry `research_only = true`, and it
    MUST NOT feed promoted outputs (alpha-spec §12.3: "current-season participation unavailable
    at serve time must not appear in promoted live features").
- **Column-map growth.** The universe is the union of as-of roster snapshots over the window.
  Adding a season zero-pads the earlier blocks into the grown map.

### 6.5 Season-boundary refresh

The offseason refresh runs when season `S−1`'s participation is published (engine-spec §4.5
publication timestamp). It recomputes `β` and `E_*`. It is an ordinary pipeline stage in
engine-spec §8.6, and it is gated by promotion (engine-spec §8.8): diagnostics clean, recovery
floors on the fixed synth, and the market residual reported.

---

## 7. Validation evidence (template: Validation and promotion)

All numbers were recorded with threads pinned to 1 and the `reference/python/requirements.lock`
pins (scikit-learn 1.9.1, numpy 2.4.6, scipy 1.17.1, pandas 3.0.6).

### 7.1 Recovery gates (Tier 0; `tests/grid/test_tier0_recovery.py`, `fit(n_iter=3)`, market seed 1)

| Gate | Floor | Legacy synth (current oracle) | Fixed synth (critic G-1) |
|---|---|---|---|
| Pooled corr(rating, planted ability) | ≥ 0.77 (`:86`) | 0.8025 | 0.8276 |
| QB / RB / WR / TE / DEF | .83 / .70 / .76 / .73 / .73 (`:91`) | .869 / .7433 / .7973 / .7713 / .7653 | .8845 / .7711 / .8599 / .7784 / .7827 |
| Team strength corr(`team_rating`, planted) | ≥ 0.60 (`:99`) | 0.6643. **Void**: it measures `γ_off − γ_def` against the legacy `off − def` target, which is coherent only with the defender bug | 0.6596 against the same, now meaningless, target |

The legacy-synth DEF and team values are void (KI-NEW-Y0). Player recovery is valid on both
synths. The fixed values are the starting point for re-set Class D floors (DR-B3/DR-B4; §10.3).

### 7.2 Estimand evidence for DR-B5 (fixed synth; `tools/investigations/sign_check.py --synth fixed`; single solve, `λ = 120`, `μ_team = 0.05`, `w = 40`)

| Quantity | No market | `[+1,+1]` → legacy target | `[+1,−1]` (rejected) | `[+1,+1]` → net target |
|---|---|---|---|---|
| corr(`γ_off − γ_def`, planted off−def) | +0.684 | +0.689 | +0.736 | +0.688 |
| corr(`γ_off + γ_def`, planted off+def, starters) | +0.431 | +0.509 | +0.452 | +0.542 |
| corr(`γ_def`, mean DEF-starter ability) (shipped grade is the negative of this) | +0.529 | +0.512 | +0.170 | +0.540 |
| corr(`E_off`, exposure-weighted planted off) | +0.963 | +0.962 | +0.965 | +0.962 |
| corr(`E_def`, exposure-weighted planted def) | +0.943 | +0.945 | +0.940 | +0.945 |
| corr(`E_off + E_def`, planted net) | +0.959 | +0.958 | +0.959 | +0.958 |
| player corr | 0.822 | 0.828 | 0.833 | 0.828 |

- The same script on the legacy synth gives `E_def` 0.609 to 0.692 and a raw `γ_def` that looks
  "inverted" (−0.223). All of that is a defender-bug artefact. The CN/CI/CF G1 evidence
  (0.6644→0.7318 and similar) is legacy-only and MUST NOT be cited (critic X-19).
- A further single solve on the fixed synth (§7.6) gives corr(`E_off+E_def`, exposure-weighted
  planted net) = 0.9589 with the gauge-invariant market row of §4.4.
- On the same solve, the oracle `team_rating = γ_off − γ_def` vs the starter net gives only
  0.39 to 0.45.

### 7.3 Structural and numeric tests (all pass at `59bce1d` on Linux)

| Test | What it pins | Status for the port |
|---|---|---|
| `tests/grid/test_layers.py:51-145` | Columns sorted by `str(id)`; stable across row shuffles; `player_order` round-trip | Port (Class A) |
| `tests/grid/test_performance.py:72-170` | Vectorized `build_design` == row-loop reference (shape, values, `y`, `colidx`, row sums); `run_rapm` end to end | Port (Class A). The speed test at `:173-198` is a non-gating perf check (critic G-6) |
| `tests/grid/test_incremental.py:18-93` | Accumulator shape; single and two-week sums; recovery with low ridge; persistence round-trip | Port (Class A / A′) |
| `tests/grid/test_rapm_robustness.py:10-88` | `lstsq` fallback warning; well-conditioned path; unknown-ID drop and the "dropped 2 player entries" log | **Divergence**: Rust returns typed errors (§5.4; `PARITY.md`) |
| `tests/grid/test_attribution.py:23-73` | `run_rapm` on real-like rosters without synth columns; `lambda_by_pos` accepted and changes ratings | Port (Class A′) |
| `tests/grid/test_design_interactions.py:113-325` | Base block unchanged; interactions appended after it; pruning; positional fallback; "planted" recovery asserts only non-empty and finite (reconcile-code-first C25) | Port the structure (Class A). The recovery claim needs a planted effect (DR-B4) |
| `tests/grid/test_layers_situations.py:65-200` | Subset of situations; low-volume skip; columns; independence; participation required | Port (Class A′) |
| `tests/grid/test_determinism.py:50-60` | In-process `atol 1e-9` | Port as a Rust determinism gate |
| `tests/grid/test_golden_master.py` | Layer A truth-anchored (`:69-104`); Layer B ordering (`:108-128`); Layer C numeric (`:152-173`, rtol 1e-5 / atol 1e-6) | **Legacy goldens are not Rust targets.** On the fixed synth, Layer B ×2 and Layer C ×2 fail (critic G-1). Targets are regenerated on the corrected oracle (DR-B1) |
| `tests/validation/test_backtest.py`, `tests/validation/test_leakage_guards.py` | §6.2; future-poisoning bit-identical (`:84-104`); watermark (`:110-121`); tripwire (`:127-145`) | Port (evaluation-and-leakage.md) |
| `tests/pipeline/test_weekly_update.py:286-366` | Matchup-grade sign with hand-planted betas | **Tautological; encodes the wrong sign.** Rewrite truth-anchored (§10.3) |

### 7.4 Real data (historical, non-parity; engine-spec §3.3)

These figures come from the CI inventory's `real_rapm.py` on 2023 regular-season data with the
full-roster universe. They are recorded in `docs/07-archive/cautious-nevermore/real-data-results.md`.

- **Rating scale.** RAPM rating SD is about 0.04 at **every** position (QB 0.042, WR 0.045,
  RB 0.044). On the synth, QB is 0.259 and other positions are 0.07 to 0.09.
- **QB value is absorbed by the intercept.** Top QBs by rating include backups. Corr(starter QB
  rating, own `γ_off`) = 0.13. This is consistent with the starting QB being nearly collinear
  with `t_off` under a 20× lighter intercept ridge (KI-NEW-A3).
- **Conditioning** is benign (cond 2.18e3; §5.1).

No real-data RAPM number is a parity target. Two reasons: ingest bias (KI-NEW-I1..I4,
KI-NEW-V0a) and the participation skew (§2.3).

### 7.5 Promotion (engine-spec §8.8)

A season-boundary RAPM output is promotable only when all of the following hold:

- diagnostics are clean (§5.3, with no typed error);
- the Rust-native Class D recovery floors pass on the fixed synth (§10.3);
- the truth-anchored sign tests pass (§10.3 T-1, T-2);
- the market residuals are reported.

Accuracy gates belong to the consuming layers (engine-spec §9.4).

### 7.6 Provenance of measurements first recorded in this spec

| Measurement | Method |
|---|---|
| `cond(A)`; PCG iterations and errors; Cholesky vs LU; sizes and `nnz`; SVD vs solve timing (§4.3, §5.1, §5.2) | SciPy `cg` with a Jacobi preconditioner and `rtol` relative-residual stopping, on the oracle's own `build_design` matrices; real 2023 from the CI inventory's pinned inputs |
| Observed equivalence diffs (§6.2) | Replicates the two oracle tests and prints max abs diffs instead of asserting |
| Synth market scale (§4.4); gauge-invariant market row and `team_rating` vs net (§4.4, §7.2) | Single solves with the oracle's `_solve_ridge_prior` |

The scripts are scheduled for commit under `reference/python/tools/investigations/` in the same
consolidation PR (P0-01). Until they are committed, these numbers are *recorded, not
reproducible from the repo*.

---

## 8. Known defects and required engine behaviour (template: Known limitations)

The Rust port MUST NOT reproduce any item in this table. Each correction lands first as an
approved oracle correction in the `reference/python/PARITY.md` ledger, in the order G-1 synth →
B-5 convention → grade sign (proposed — DR-B1). Alternatively it is recorded there as a
deliberate divergence (proposed — DR-B6).

| # | Defect | KI / source | Required behaviour |
|---|---|---|---|
| 1 | The synth draws defenders from the offense team, so every team, DEF and grade number is void | KI-NEW-Y0 (critic G-1) | Parity targets come from the corrected synth (synthetic-world.md; proposed — DR-B4) |
| 2 | The Layer-3 row `[+1, +1]` with `team_rating = γ_off − γ_def`, a legacy `off − def` truth, and a comment (`layers.py:400`) that contradicts the code | KI-G1, KI-V1, KI-NEW-A1, KI-NEW-Y1 | §4.4 / §4.5. **Do not** adopt `[+1, −1]` |
| 3 | Matchup grade `−γ_def` (inverted), in three code sites plus docstrings and tautological tests | KI-NEW-A2 | `G[t] = +E_def[t]`. Truth-anchored tests (§10.3) |
| 4 | Intercept-only grades carry no validated signal (Tier-2 ≈ 0 for either sign on synth) | KI-NEW-V2 | Grade = `E_def`. Gate on real points allowed per engine-spec §7.14 |
| 5 | Silent `lstsq` fallback plus a full SVD on every solve | KI-NEW-A6; PR #53 C3 | Typed failure plus a cheap condition estimate (§5.4; proposed — DR-B6) |
| 6 | Unknown participant IDs silently dropped; unknown team or prior IDs raise bare `KeyError`; missing `dv` raises bare `KeyError` | KI-NEW-Z10; PR #53 C4; KI-G12; new | Typed errors (§5.4) |
| 7 | Three different absent-team market behaviours (`KeyError` / skip / anchor to 0) | KI-NEW-Z11; reconcile-code-first §1.12; new for `run_rapm` | Not anchored (§4.4 item 4) |
| 8 | Synth market target in ability units vs estimand in dV units (about 15× scale gap) | KI-NEW-Z12; new (§4.4) | `s_t` expressed in EP/play net strength (DR-D10; synthetic-world.md) |
| 9 | Fixed point re-seeds only `focus_qb`, has no convergence test, and `verbose=True` (the default) needs `ability` | KI-NEW-Z9; KI-G14; new | DR-D11. No dependence on synth-only columns |
| 10 | `lambda_by_pos` never used in production; no WR/TE target-share attribution | KI-#32 | Calibrate or remove (DR-D12) |
| 11 | Real-data rating scale ≈ 0.04 SD at every position; QB value absorbed by `γ_off`; synth-calibrated scale constants | KI-NEW-A3, KI-NEW-P4, KI-NEW-Y2 | A real-data penalty and QB-identifiability decision before any real-RAPM golden (DR-D12). Realistic synth profile (DR-B4) |
| 12 | Interaction block unreachable from `run_rapm`; synth plants no interaction | KI-NEW-Z13; new; reconcile-code-first C25 | Research-only. A planted effect before any recovery claim |
| 13 | Duplicate IDs within a play summed to ±2 | KI-NEW-Z38; new | `ContractError::DuplicateParticipant` |
| 14 | Incremental path: week-only watermark, reinit on roster change, no recompute, cross-season accumulation, V(s) refit leakage | KI-NEW-W1, KI-NEW-W2, KI-A9, KI-V2, KI-NEW-W4 | §6.3 / §6.4 |
| 15 | Cumulative RAPM fed to the Kalman with `snaps = 1` | KI-NEW-W3, KI-#24 (critic X-13) | The Kalman observes weekly Layer-1/1′ credit |
| 16 | Postseason plays enter RAPM | KI-NEW-V0a | Plays-contract `season_type` policy |
| 17 | Current-season participation used at every walk-forward origin (train/serve skew); real H2 +0.848 measured that way | KI-NEW-Z68; reconcile-code-first C1; reconcile-spec-first R14 | Research-only labelling. Publication-lag axis (engine-spec §4.5; proposed — DR-C1) |
| 18 | No schedule (player → opponent) table, so grades are never keyed to opponents | KI-#31 | Grades keyed by `(season, week, def_team)` and joined through the schedule provider contract |
| 19 | A NaN prior mean silently turns the whole `β` into NaN (dense `Λm`), reachable through `fit()` when the focus QB has no plays | KI-NEW-Z8; new | §5.4 `NonFinitePrior` |

---

## 9. Open decisions

| Decision | Question | Proposed default |
|---|---|---|
| **DR-B5** | Team-strength estimand, Layer-3 rows, matchup grade | Net strength from gauge-invariant `E_off + E_def`. The market anchors `N` using the line at lock. `G = +E_def`. Reject `[+1, −1]` (proposed — DR-B5) |
| **DR-C1** | RAPM and participation on the live path | Offseason-only RAPM at season boundaries. In-season Layer-1′. Publication-lag axis (proposed — DR-C1) |
| **DR-C5** | Domain controls of the generalized objective: home field, garbage time, overtime, minimum exposure, play weights | **Open.** To be set in this spec before P1-12 code (proposed — DR-C5). The oracle models none of them; `W = I` |
| **DR-C6** | Three-season window | Per-season blocks, exact window sums (proposed — DR-C6) |
| **DR-B3** | Parity tolerances | Classes A / A′ / B / C / D (proposed — DR-B3). See the Class B refinement in §10.3 |
| **DR-B4** | Synthetic world | Fix defenders, plant net strength, regenerate goldens, realistic profile (proposed — DR-B4) |
| **DR-B6** | Typed failure vs faithful port | Typed failure. The ADR cites PR #53 C3 (proposed — DR-B6). The condition-estimate threshold value is open |
| **DR-B1** | Oracle correction order | G-1 synth → B-5 convention → grade sign → … (proposed — DR-B1) |
| **DR-C2 / DR-C3** | GRID as signal provider; where talent enters Layer D | Layers A–F skeleton. Role-specific talent covariates (proposed — DR-C2, DR-C3) |
| **DR-C13** | Situation vocabulary | Code set canonical; `two_minute` via `half_seconds_remaining` (proposed — DR-C13) |
| **DR-C15** | Storage | SQLite source of truth; bulk blocks registered in a manifest (proposed — DR-C15) |
| **DR-D10** | How a spread (points per game) at lock maps to `s_t` in EP per play over the window, and whether totals also enter (Layer B implied points) | **Open, no default.** Requires a calibration study |
| **DR-D12** | `λ`, `μ_team`, `lambda_by_pos`, and QB identifiability on the real scale. Options: multi-season pooling (DR-C6), a QB-specific penalty or prior, or Layer-1 event credit for QBs | **Open.** Calibrate by rolling origin on completed seasons, pre-registered before results (engine-spec §7) |
| **DR-D11** | Generalize the re-seed to every credited player with a convergence test, or drop the fixed-point claim | **Open.** Parity port as is until decided |

---

## 10. Rust port plan (template: Tolerances, Reference examples)

### 10.1 Target crates and work packages (engine-spec §8.1, §9.5; DR-A8 adopted subject to ratification)

| Piece | Crate::module | WP |
|---|---|---|
| Generalized ridge primitive: dense Cholesky + Jacobi-PCG, diagnostics, typed errors, weights | `models::ridge` | **P1-06** (numerical primitives) |
| Design builder, column map, market rows, `E_*`, grade, situation slices, `fit` | `models::rapm` (uses `domain` plays types and `features` situation masks) | **P1-12** (GRID component port) |
| Per-season sufficient-statistic blocks, column-map versioning, window sums | `models::rapm` + `persistence` | P1-12 (store), P1-02 (schema) |
| Publication-lag gate on participation | `domain` as-of types + `features` store | **P1-05** |
| Walk-forward per-origin ratings, two-path, three-season window | `evaluation` | **P1-09** |
| Season-boundary refresh stage | `pipeline` (replaces `application` in the first engine WP; DR-A8) | P2-01 (operation) |

### 10.2 Fixtures

The fixtures are the synthetic-only parity fixtures (DR-A11) defined by
`docs/03-contracts/parity-fixture-contract.md`. They are exported single-threaded from the
oracle, both legacy and corrected, and sha256-manifested (proposed — DR-B2). The RAPM set is:

- canonical synth `plays` / `players` (and the fixed-synth equivalents);
- the oracle `dv` (injected, so V(s) differences do not cascade);
- design triplets and the column map;
- `XᵀX` and `Xᵀy` per week block;
- `β` for every configuration in §10.3;
- exposure counts;
- per-origin walk-forward ratings;
- the per-iteration Layer-1 residual vectors for `fit` (injected).

### 10.3 Parity targets (proposed — DR-B3 classes)

| ID | Target | Class | Criterion |
|---|---|---|---|
| P-1 | Design: CSR triplets, `y`, column map, `nCols` | A (exact) | Integer equality. `y` identical |
| P-2 | `XᵀX` and `Xᵀy` per block and summed | A | `XᵀX` exact (integer sums). `Xᵀy` ≤ 1e-12 abs |
| P-3 | Dense `β` for: (a) `λ=120`, `μ_team=0.05`, no market; (b) (a) plus market `[+1,+1]` with legacy target, `w=40`; (c) `lambda_by_pos = {QB:0.5, RB:1.5, WR:1.0, TE:1.0}`; (d) prior-mean re-seed; (e) interactions `μ = 10`, `min_pair_plays = 10` on the `cb_split` synth (design called directly); (f) each situation slice | A′ | ≤ 1e-9 relative (‖·‖₂) vs oracle `np.linalg.solve` |
| P-4 | PCG `β` for P-3 (a), (b) and the real-scale stress (synthetic, `nCols ≈ 3e3`) | B | See the refinement below |
| P-5 | Incremental == batch (Rust internal) and vs oracle per-origin ratings with injected `dv` | A (internal), A′ (vs oracle) | Internal ≤ 1e-12 abs. Vs oracle ≤ 1e-9 relative (the oracle asserts 1e-8) |
| P-6 | Two-path: Rust weekly stage == Rust walk-forward at W | A | ≤ 1e-12 abs (the oracle observes 0.0) |
| P-7 | `fit` with injected per-iteration Layer-1 residuals: ratings, team values, `qb_weekly` | A′ | ≤ 1e-9 relative |
| P-8 | Typed-failure divergences: the rank-1 `λ=1e-15` case; unknown-ID case | divergence | Rust returns `IllConditioned` / `UnknownParticipant`. Listed in `reference/python/PARITY.md` |
| P-9 | End to end with the Rust booster and Rust synth | D | Recovery floors **re-set on the fixed synth** by calibrate-below-observed (proposed — DR-B3/DR-B4). Observed fixed values to calibrate from are in §7.1. Floors are not set by this spec |
| P-10 | `E_off`, `E_def`, `N`, `G` from the corrected oracle | A′ | ≤ 1e-9 relative once the oracle correction (DR-B1 ledger, B-5) lands. Until then, formula tests on §10.4 |

**Class B refinement (new finding; proposed amendment to DR-B3).**

- DR-B3's Class B criterion is "‖β_cg − β_py‖/‖β_py‖ ≤ 10 × CG tolerance". It conflates the
  residual tolerance with the solution error.
- Measured on the canonical synth, the solution error is **10.6× to 24×** the relative-residual
  tolerance (§5.2). The criterion therefore fails for a correct CG at every tested tolerance.
- Proposed Class B criterion, all three required:
  1. `converged = true`;
  2. the Rust solution's relative residual on the **oracle's** `(A, b)` is ≤ `tol_cg`;
  3. ‖β_cg − β_dense‖/‖β_dense‖ ≤ 1e-9, with `tol_cg = 1e-12` (measured 1.4e-11 at that
     tolerance), so Class B meets the A′ bound.

**Truth-anchored semantic tests (new; must pass on the fixed synth, Rust-native):**

- **T-1.** sign(corr(`E_def`, planted defense)) > 0 and sign(corr(`G`, planted defense)) > 0.
  On the planted-defense league of `tools/investigations/defsign_planted.py`, the ELITE grade
  must exceed the WEAK grade.
- **T-2.** corr(`N`, planted net strength) > 0, and the strongest planted team is in the
  estimated top 2. This is the golden Layer A pattern, `tests/grid/test_golden_master.py:79-85`,
  re-anchored to net strength.
- **T-3.** Gauge invariance: adding `+a` to every one of team `t`'s offensive players'
  coefficients and `−k_off·a` to `γ_off[t]` leaves `E_off[t]` and every fitted value unchanged
  to ≤ 1e-12 on the synth (`k_off = 5`).
- **T-4.** Absent-market team: removing team `t` from `T_mkt` leaves no row for `t`, and `β`
  equals the solve with that row omitted.

### 10.4 Reference examples (become golden unit tests)

**Setup.**

- Players `a1` (A, QB), `a2` (A, DEF), `b1` (B, QB), `b2` (B, DEF).
- Plays:

  | Play | Offense | Defense | `off_players` | `def_players` | `dv` |
  |---|---|---|---|---|---|
  | 1 | A | B | (a1) | (b2) | 0.5 |
  | 2 | B | A | (b1) | (a2) | −0.2 |
  | 3 | A | B | (a1) | (b2) | 0.1 |

- Columns: `[a1, a2, b1, b2, t_off A, t_off B, t_def A, t_def B]`.
- Design rows: `r1 = r3 = (1, 0, 0, −1, 1, 0, 0, −1)` and `r2 = (0, −1, 1, 0, 0, 1, −1, 0)`
  (oracle output verified).

**Hand derivation** (`λ = 1`, `μ = (1, 1, 1, 1, 0.05, 0.05, 0.05, 0.05)`, `m = 0`, no market).

- Each distinct row `r` has `rᵀΛ⁻¹r = 1 + 1 + 20 + 20 = 42`, and the solution lies along `Λ⁻¹r`.
- For `r1` with two observations (0.5, 0.1): `s₁ = 0.6/(2·42 + 1) = 0.6/85`.
- For `r2` with one observation (−0.2): `s₂ = −0.2/(42 + 1) = −0.2/43`.

```text
β = [ 0.6/85, 0.2/43, −0.2/43, −0.6/85, 12/85, −4/43, 4/43, −12/85 ]
  = [ 0.007058823529412, 0.004651162790698, −0.004651162790698, −0.007058823529412,
      0.141176470588235, −0.093023255813953,  0.093023255813953, −0.141176470588235 ]
oracle team_rating = γ_off − γ_def :  A = 12/85 − 4/43 = 0.048153214774282 ;  B = −4/43 + 12/85 = 0.048153214774282
```

The oracle's `team_rating` cannot tell A from B. Yet A's offense gained +0.3 on average and its
defense allowed −0.2.

**Proposed outputs (§4.5)** for the same `β`:

```text
E_off[A] = 12/85 + 0.6/85 = 12.6/85 = 0.148235294117647     E_def[A] = 4/43 + 0.2/43 = 4.2/43 = 0.097674418604651
E_off[B] = −4.2/43 = −0.097674418604651                      E_def[B] = −12.6/85 = −0.148235294117647
N[A] = +0.245909712722298     N[B] = −0.245909712722298
G[A] = +0.097674418604651 (tougher)      G[B] = −0.148235294117647 (easier)
oracle grade −γ_def:  A = −0.093023255813953,  B = +0.141176470588235   ← inverted (KI-NEW-A2)
```

**Oracle with market** (`market_strength = {A: 0.1, B: −0.1}`, `w_mkt = 40`, `λ = 1`).
`run_rapm` output; Class A′ golden:

```text
β = [ 0.055697409490196, 0.052724183798448, −0.052724183798448, −0.055697409490196,
      0.080378238137256, −0.020913724302329,  0.020913724302327, −0.080378238137254 ]
γ_off[A] + γ_def[A] = 0.101291962439583   (anchored toward 0.1)
```

---

## 11. Superseded-spec mapping

| Superseded section | Requirement | How this spec satisfies it | Gap |
|---|---|---|---|
| alpha-spec §1.2 | "a traditional basketball-style RAPM implementation is not allowed to become a hidden dependency of the live projection path" | §2.3: offseason-only RAPM, publication-lag gate (proposed — DR-C1) | Ratification of DR-C1 |
| alpha-spec §4.1 (Participation row), Appendix A | Participation not for in-season use; published after the postseason | §3.1 as-of rule; §6.4 research-only mode | The provider contract must carry the publication timestamp (P1-03 / P1-05) |
| alpha-spec §2.4 | Exact three-season rule | §6.4 per-season blocks (proposed — DR-C6) | — |
| alpha-spec §6.1 Layers B, E | Team environment; opponent adjustment | §2.2: `N[t]`, `G[t]` (proposed — DR-B5, DR-C2) | Venue, rest and weather (Layer E) are outside RAPM |
| alpha-spec §6.2 (RAPM row) | "Opponent-adjusted team/unit/player effects where participant data supports them; research-only if live feature parity is absent" | §2.3, §6.4 | — |
| alpha-spec §6.6 | Model-spec contents; rule 4 typed failures; rule 3 no silent golden regeneration | This document; §5.4; §10.3 (goldens regenerated only through the DR-B1 ledger) | Computational-complexity statement for declared dimensions: §5.1 sizes only. A benchmark is still due in P1-06 |
| alpha-spec §12.1 / final-build-spec §19.1 | Unit tests for "ridge regression, RAPM construction" | §7.3, §10.3 | — |
| alpha-spec §12.3 (line 1784) | "current-season participation unavailable at serve time must not appear in promoted live features" | §6.4 research-only flag | The leakage test lives in evaluation-and-leakage.md |
| final-build-spec §11.1 | `nalgebra` dense, `sprs` sparse, dimension assertions | §5.1, §5.2 | — |
| final-build-spec §11.2 | Ridge primitive: configurable λ, regularized intercept, **weighted observations**, diagnostics | §4.2 (per-column `μ`, `W`), §5.3 | The oracle has uniform weights. The weight semantics depend on DR-C5 |
| final-build-spec §11.3 | "Document and test the exact optimization objective"; sparse CG with diagnostics; "A numerically failed solve must not silently produce a production model"; preconditioning | §4.2 generalized objective (amendment by ADR; reconcile-code-first C11); §5.2–§5.4 | Domain controls: stints → plays (done); players on court → participation (done); response → `dv` (done); possession weighting, home court, garbage time, overtime and minimum appearances are **open (DR-C5)**; team effects → intercepts (done); intercept treatment → `μ = 0.05` plus Layer 3 (done) |
| final-build-spec §12.1 | Daily "RAPM incremental" branch | **Replaced**: RAPM is a season-boundary stage (§6.5). The daily DAG runs V(s) → dV → Layer-1′ → Kalman (engine-spec §8.6) | — |
| final-build-spec §12.3 | Snapshot and rollback | §6.4: blocks and column map are versioned state | The snapshot list in engine-spec §8 must include RAPM sufficient statistics |
| final-build-spec §0.3 / §12.1 claim "Kalman consumes weekly RAPM" | — | **Rejected** (critic X-13): the Kalman observes Layer-1/1′ credit | — |
