---
model-spec-id: MS-SYNTHETIC-WORLD
status: Draft            # generator semantics below are NOT yet approved by the statistical owner
statistical-owner: statistical owner (role defined in engine-spec §1); approval pending
version: 0.1.0 (2026-10-07, written under consolidation WP P0-01)
supersedes: none. First engine-only spec for this component. Replaces the cautious-nevermore
  prose now archived (non-authoritative) under docs/07-archive/cautious-nevermore/.
---

# Model spec — Synthetic world (planted ground truth and the recovery-gate contract)

This is a contract written before implementation. Under the authority order in engine-spec §1.5
(DR-A2, adopted subject to owner ratification) it is an authority-level-3 document. It defines the
planted-truth generator that every GRID estimator is validated against (engine-spec §6.9, §7.13).
The statistical owner approves the generative model. The implementing agent may not redefine it
(superseded alpha-spec §6.6; carried into engine-spec §6.8).

**How to read this spec**

- **Normative words.** MUST, SHOULD and MAY are requirements on the Rust engine. "The oracle" is the
  Python reference at `reference/python/` (ADR-012; engine-spec §1.7). Every
  `reference/python/...:LINE` citation uses cautious-nevermore (CN) `59bce1d` line numbers; the
  imported files are byte-identical (`reference/python/MANIFEST.tsv`: `verbatim`).
- **Proposed decisions.** "(proposed — DR-xx)" marks a default recorded in
  `docs/00-meta/decision-register.md`; it is not settled. `DR-NEW:<slug>` marks a decision first
  raised here or in a sibling spec (§9).
- **Defect IDs.** `KI-` IDs are entries in `docs/00-meta/known-issues.md`. Findings first made here
  are labelled **new** (§8.3).
- **Two generators.**
  - *Legacy* is `backend/grid/synth.py` as imported. Its defenders are drawn from the offense's own
    roster (KI-NEW-Y0; critic G-1).
  - *Fixed* is the one-line defender fix applied **in memory** by
    `reference/python/tools/investigations/_common.py:apply_defender_fix`; `synth.py` on disk is never
    modified. It keeps the legacy planted `team_strength` (off − def).
  - Every team-, defense-, grade- or market-related number measured on the legacy generator is
    **semantically void**. It is quoted only so the record stays traceable (DR-B1).
- **Measurements.** Numbers marked "measured" were produced on 2026-10-07 with
  `OMP/OPENBLAS/MKL_NUM_THREADS=1`, Python 3.11.15 and the `reference/python/requirements.lock` pins
  (numpy 2.4.6, scikit-learn 1.9.1, pandas 3.0.6), read-only against the imported oracle. The Tier-0,
  golden and calibration figures reproduce the critic G-1 table and `reference/python/PARITY.md` (c)
  exactly: `python3 -m tools.investigations.tier0_legacy_vs_fixed` and
  `python3 -m pytest -p tools.investigations.pytest_defender_fix …` (5 failed, 22 passed) were re-run
  on that date.

## 0. Template conformance

This spec follows `docs/99-templates/template-model-spec.md`, extended with numbered sections.

| Template heading | Where it is covered |
|---|---|
| Target | §1.2 (what is planted), §3.2–§3.4 |
| Inputs | §3.1 (configuration) |
| Equations | §4.1–§4.8 (as implemented); §4.10–§4.12 (required) |
| Priors | §4.6 (the planted feeder prior); priors of the estimators are in their own specs |
| Constraints | §4.9, §8.5 |
| Seed policy | §4.8, §5.2, §6 |
| Tolerances | §10.3 |
| Reference examples | §10.4 |
| Explanation fields | none. The world emits no user-facing explanation, and its truth never reaches an estimator (§3.4) |
| Validation and promotion | §7, §10.3 |
| Known limitations | §8, §9 |

---

## 1. Purpose and statistical intent

### 1.1 Why a planted world (load-bearing comments, preserved verbatim)

The Rust port MUST keep this intent.

- `reference/python/backend/grid/synth.py:4-7`:

  > "The whole point of this module: produce play-by-play that is generated from known, hidden player
  > abilities and team strengths, so the estimators in the rest of the package can be validated by
  > checking whether they recover the planted truth. Real data gives you no ground truth; synthetic
  > data does."

- `synth.py:12-23`, the three design choices that make recovery testable:

  > "A player's marginal effect flows through YARDS GAINED, which is linear in the net on-field
  > ability gap. Better players -> more yards -> better field position / more points -> higher dV.
  > RAPM then recovers a *scaled* version of ability, so validation is by correlation
  > (scale-invariant), not by matching units exactly."
  >
  > "Substitutions (starters ~80% of snaps, backups rotate) create the lineup variation that adjusted
  > plus-minus needs to separate teammates."
  >
  > "One designated QB ("focus" QB) has a time-varying weekly ability: a rising trajectory, an injury
  > that costs two games, a diminished return, and a recovery -- so the state-space (Kalman) layer has
  > a real latent trajectory + intervention to recover."

- `synth.py:30-31`: offense personnel is "skill positions only; OL is a team-level nuisance, never an
  estimand, consistent with the design".
- **The swap point.** `reference/python/backend/grid/data_adapters.py:2-22`: "The rest of GRID
  consumes a small, fixed contract" (`plays`, `players`, `market`, `college`); `load_synthetic()`
  "returns this contract with planted ground truth"; real loaders fill the same contract and "the
  identical downstream pipeline runs on real football". CN's `CLAUDE.md` adds: "When changing any
  engine stage, preserve this contract." The Rust contract is `docs/03-contracts/plays-contract.md`.
- **Calibrate, then gate.** `tests/grid/test_tier0_recovery.py:8-14`: thresholds are "CALIBRATED BELOW
  THE OBSERVED BASELINE … not set to the plan's aspirational targets … Each gate sits a margin below
  the measured value so it passes comfortably today and fails only on a real recovery regression."
- **Truth anchors cannot be regenerated away.** `tests/grid/test_golden_master.py:7-12,25-26`: Layer A
  "CANNOT be gamed by regenerating the golden file, because it is anchored to truth, not to the last
  run", and "Layer A + the Tier 0 recovery gates are truth-anchored and must ALSO pass, so a
  regenerate cannot turn a real regression green."

### 1.2 What the current world plants, and what it does not

| Planted quantity | How (§4) | Validated by |
|---|---|---|
| Player ability `a_p` by position | `N(0, σ_pos²)` plus a starter bonus or a backup penalty | Tier-0 pooled and per-position attribution; golden Layer A sign |
| Lineup rotation | offense starter on a slot with probability 0.80; defensive base unit on 70% of snaps | identification of teammates (no direct gate) |
| Focus-QB weekly trajectory `τ_w` | rise, two missed weeks, diminished return, recovery | Tier-0 Kalman trajectory, injury variance and NIS; golden Layer A/B/C Kalman arrays |
| Team strength (legacy definition `off − def`) | from starter abilities | Tier-0 team gate; golden team ranking; the harness market |
| Feeder-league factor 0.62 and feeder SV | `0.62·a_p + N(0, 0.03²)` | Tier-0 equivalency slope, out-of-sample R², rookie prior |

**Not planted.** Offensive-line effects (by design); situational ability; WR×CB interactions
(`cb_split` only relabels positions, `synth.py:72-79`); scheme changes; changepoints other than the
focus-QB injury; pass/run split, targets, carries, shares and per-player touchdowns; availability
other than the focus QB's two weeks; a market in the generator (the test harness builds it, §4.7); a
clock, score, punts, turnovers or safeties; a true value function V*(s); multiple seasons. A recovery
claim about any of these is not supported by this world (engine-spec §7.13.2, §7.13.6).

### 1.3 Why recovery is measured by scale-invariant correlation

The map from planted ability to an estimated rating passes through four stages, none of which
preserves units:

```text
a_p ──×26 (yards per unit net ability)──► yards ──V(s), nonlinear in field position──► dV
    ──ridge λ=120 with team intercepts (penalty mask 0.05) and exposure-dependent shrinkage──► β_p
```

So RAPM recovers `c·a_p` with an unknown, position-dependent `c`. Measured slope of rating on
ability, `fit(n_iter=3)` Tier-0 configuration:

| Position | Legacy: rating SD / ability SD / slope | Fixed: rating SD / slope |
|---|---|---|
| QB | 0.272 / 0.083 / 2.83 | 0.236 / 2.51 |
| RB | 0.092 / 0.037 / 1.87 | 0.090 / 1.89 |
| WR | 0.081 / 0.033 / 1.96 | 0.079 / 2.05 |
| TE | 0.072 / 0.025 / 2.23 | 0.059 / 1.85 |
| DEF | 0.080 / 0.025 / 2.42 | 0.073 / 2.25 |

Hence the gates compare by Pearson correlation, pooled and within position. Correlation is invariant
to the unknown positive affine map, and it is what the estimator can honestly be asked to recover. Its
limits are explicit:

- **Blind to magnitude.** A gate that passes says nothing about whether ratings are on the real-data
  scale (about 0.04 EP per play SD at every position on real 2023; §4.11, KI-NEW-A3). Magnitude
  questions go to the calibration gates, which standardise by the filter's own predictive variance,
  and to real-data validation.
- **Pooled correlation mixes positions with different `c`.** The pooled gate is therefore partly a
  between-position statistic. The per-position gates are the primary ones.
- **Not every gate is scale-invariant.** The equivalency slope gate (`[0.9, 1.8]`) is in rating units
  per feeder unit. It reads 1.3159 (legacy) against a planted factor of 0.62 because the target is a
  rating, about twice the ability scale, not the ability itself (`test_tier0_recovery.py:132-138`).

---

## 2. Position in the engine

### 2.1 Data flow

```text
SynthConfig ─► simulate() ─► plays, players, gt ─┐          make_college(players, gt) ─► college
                                                 │
            harness: market_t = gt.team_strength_t + N(0, 0.01²), default_rng(1)
                                                 ▼
V(s) → dV (value-model.md) → RAPM fit() + Layer 1 (rapm-attribution.md, layer1-credit.md)
     → focus-QB Kalman (state-space-kalman.md) → equivalency + priors (cross-league-priors.md)
                                                 ▼
            recovery statistics vs gt ─► Tier-0 gates, golden Layer A/B/C, calibration bands
```

### 2.2 Consumers in the oracle

- **The four gate files** (27 tests): `tests/grid/test_tier0_recovery.py` (8),
  `tests/grid/test_golden_master.py` (11, with `tests/grid/golden_master.py` and
  `tests/grid/golden/snapshot.npz`), `tests/grid/test_calibration_synth.py` (6) and
  `tests/grid/test_determinism.py` (2; one of them synthetic).
- **Unit and integration tests** that build frames with `load_synthetic`, `simulate` or
  `make_college`: `tests/grid/test_attribution.py`, `test_design_interactions.py` (`cb_split`),
  `test_layers_situations.py`, `test_performance.py`, `test_priors.py`, `test_grid_import.py`,
  `tests/pipeline/test_weekly_update.py`, `tests/validation/test_backtest.py`,
  `test_leakage_guards.py` and `test_verdict.py`.
- **`run_demo.py`**, which prints the same recovery numbers (documentation; its state-space lines
  depend on the thread count, `reference/python/README.md`).
- **The investigation scripts** in `reference/python/tools/investigations/` that take `--synth`.

### 2.3 Contracts and neighbouring specs

- `docs/03-contracts/plays-contract.md`: §3.1 (synthetic producer), §6.1 (truth bundle), §6.2
  (verified invariants), §8 D-8, D-9, D-10 and D-13.
- `docs/03-contracts/parity-fixture-contract.md`: the `synthetic-world` component; the canonical
  world is stored once and referenced by other cases (§3); synthetic-only fixtures (§8, DR-A11).
- engine-spec §6.9 (the synthetic-world validation contract), §7.12 and §7.13.
- Neighbours: every GRID model spec, and `evaluation-and-leakage.md` (the recovery harness lives in
  the `evaluation` crate, engine-spec §8.1).

---

## 3. Inputs and outputs

### 3.1 Configuration (`SynthConfig`, `synth.py:41-54`; module constants `synth.py:32-38`)

| Field | Dataclass default | `load_synthetic()` (`data_adapters.py:32`) | `CANONICAL_SYNTH` (`tests/grid/golden_master.py:42-48`) | Meaning |
|---|---|---|---|---|
| `n_teams` | 12 | 12 | 12 | teams |
| `weeks` | 14 | 14 | 14 | weeks, no byes |
| `seed` | 7 | 7 | 7 | world seed |
| `yards_ability_gain` | 26.0 | 26.0 | 26.0 | yards per unit net ability |
| `yards_noise_sd` | 4.2 | **3.2** | 3.2 | per-play yards noise SD |
| `drives_per_team_per_game` | 10 | **12** | 12 | drives per side per game |
| `max_plays_per_drive` | 12 | 12 | 12 | drive cap (§4.3) |
| `fg_range_yardline` | 35 | 35 | 35 | field-goal range |
| `league_factor` | 0.62 | 0.62 | 0.62 | planted feeder→NFL slope |
| `college_noise_sd` | 0.030 | 0.030 | 0.030 | feeder noise SD |
| `cb_split` | False | False | False | relabel DEF slots as CB |
| `n_cb_per_team` | 2 | 2 | 2 | CBs per team when `cb_split` |

`load_synthetic()` does **not** use the dataclass defaults; the golden pins the overrides explicitly
(`golden_master.py:34-41`). The canonical world is `CANONICAL_SYNTH`.

Harness constants outside the generator: market noise `N(0, 0.01²)` with `default_rng(1)`;
`fit(n_iter=3)`; focus-QB intervention at index 9 (`golden_master.py:30-32`).

### 3.2 Outputs (oracle)

- `simulate(cfg)` returns `(plays, players, gt)` (`synth.py:162-282`). `plays` and `players` follow
  the plays contract (§2, §4 of that contract). `gt` is
  `{ability, team_strength, focus_qb, focus_tau, league_factor, players}` (`synth.py:274-281`).
- `make_college(players, gt, cfg)` returns `[player_id, position, feeder_sv, feeder_snaps,
  is_rookie]` (`synth.py:297-317`).
- `load_synthetic(cfg)` returns `{plays, players, gt, college, cfg}` and **no market**
  (`data_adapters.py:31-35`).
- The truth columns `ability` and `is_starter` also sit in the `players` input frame (plays-contract
  D-13).

### 3.3 The canonical world (measured)

| Fact | Legacy | Fixed |
|---|---|---|
| Plays / players / drives / games | 16,825 / 288 / 2,016 / 84 | **17,752** / 288 / 2,016 / 84 |
| Players by position | QB 24, RB 36, WR 60, TE 24, DEF 144 | identical |
| Drives scoring 7 / 3 / 0 | 417 / 62 / 1,537 | 399 / 86 / 1,531 |
| Drives ended only by the 12-play cap (scored 0) | 486 (24.1%) | 586 (29.1%) |
| Plays whose defenders all belong to `off_team` / `def_team` | 100% / 0% | 0% / 100% |
| Plays with a duplicated offensive id (WR / QB) | 245 (215 / 30) | 258 (231 / 28) |
| Offensive tuple length 5 / 6 | 16,795 / 30 | 17,724 / 28 |
| Max `yardline_100`; rows above 99; rows with `ydstogo > yardline_100` | 104; 10; 361 | 101; 1; 389 |
| Starter on-field share, own offense (QB / RB / WR / TE) | 0.788 / 0.798 / 0.799 / 0.797 | 0.793 / 0.789 / 0.800 / 0.798 |
| DEF starter share of its unit's snaps | 0.876 | 0.877 |
| Rookies in `college` | 105 | 105 |
| Focus QB id; static `gt.ability`; mean `τ` over played weeks | 0; 0.1001; 0.1538 | identical |
| `players`, abilities, `team_strength`, `focus_tau`, schedule, `college` | — | **identical to legacy** (§4.8) |

### 3.4 Required outputs (Rust)

- **A separate `SyntheticTruth` type** that estimator code cannot reach (plays-contract D-13). The
  `models` crate MUST NOT depend on `synth`; only `evaluation` and test code read the truth.
- **The truth bundle** contains:
  - per-player static ability;
  - the per-play generating contributions `A_off,i` and `A_def,i` actually used to draw the yards;
  - per-player *effective* ability over the player's snaps (§4.10 G-4);
  - per-team planted offense, defense and net quality (§4.10 G-2);
  - weekly truth for time-varying players;
  - the planted market with its declared units and seed (§4.10 G-5);
  - the planted feeder factor;
  - V*(s) when the profile defines one (§4.10 G-8).
- **Provenance:** generator version, profile name, configuration and seed, carried into every
  fixture manifest (`provenance.profile`, `seeds`; parity-fixture-contract §4).

---

## 4. Model and equations (as implemented)

### 4.1 Rosters and abilities (`_make_players`, `synth.py:57-83`)

For team `t = 0…n_teams−1`, position `pos` in the order QB, RB, WR, TE, DEF, and depth index
`k = 0…ROSTER[pos]−1`, with player ids assigned sequentially from 0:

```text
starter(p) = k < OFF_ONFIELD[pos]      for QB (1), RB (1), WR (2), TE (1)            (synth.py:63)
           = k < DEF_ONFIELD = 7       for DEF                                        (synth.py:65-66)
a_p = ε_p + b_pos          if starter(p)                                              (synth.py:67-69)
    = ε_p − 0.5·b_pos      otherwise                                                  (synth.py:70-71)
ε_p ~ N(0, σ_pos²), one draw per player in roster order
```

| Position | `ROSTER` | on field | `σ_pos` (`ABILITY_SD`) | `b_pos` (`STARTER_BONUS`) |
|---|---|---|---|---|
| QB | 2 | 1 | 0.090 | 0.10 |
| RB | 3 | 1 | 0.030 | 0.03 |
| WR | 5 | 2 | 0.028 | 0.03 |
| TE | 2 | 1 | 0.022 | 0.02 |
| DEF | 12 | 7 | 0.022 | 0.02 |

With `cb_split`, the first `n_cb_per_team` DEF players of each team are relabelled `CB`. Their
ability distribution and on-field count are unchanged (`synth.py:72-79`).

### 4.2 Schedule (`_round_robin`, `synth.py:151-159`)

Each week the team list is shuffled and consecutive pairs play: `n_teams/2` games per week, 84 games
in the canonical world, no byes. It is not a true round robin; pairings can repeat. In each game both
teams take a turn on offense (`synth.py:181-182`), each with `drives_per_team_per_game` drives. With
an odd `n_teams` one team would sit out each week silently (§8.5).

### 4.3 Drives and plays (`simulate`, `synth.py:162-282`)

```text
drive start:  yardline_0 = int(clip(N(75, 8²), 60, 95)),  down = 1,  togo = 10          (:188-189)
for play j = 1 … max_plays_per_drive (12):                                               (:192)
    (off_pl, def_pl) = _pick_onfield(tidx[off_team])        # LEGACY: both from off_team (:193; KI-NEW-Y0)
    A_off = Σ_{p ∈ off_pl} a_p        (focus QB: τ_week instead of a_p; §4.4)            (:195-210)
    A_def = Σ_{p ∈ def_pl} a_p                                                           (:211)
    yards = clip(round(26·(A_off − A_def) + N(4, σ_y²)), −8, 60)                         (:212-215)
    new = yardline − yards
    if new ≤ 0:                       terminal, value 7 (touchdown)                      (:222-224)
    elif yards ≥ togo:                1st and 10 at max(1, new)   (no goal-to-go)        (:226-228,239)
    else:                             down+1, togo = max(1, togo − yards)                (:229-230)
        if down+1 > 4:
            if new ≤ 35 and U < 0.82: terminal, value 3 (field goal)                     (:231-234)
            else:                     terminal, value 0 (turnover on downs)              (:235-237)
if the drive did not end within 12 plays: its last play is terminal, value 0             (:256-260)
drive_points broadcast to every play; next state = the next play's state; (−1, −1, −1) on terminal (:262-271)
```

- `round` is Python's `round` on a float64, which rounds **half to even** (`round(2.5) = 2`,
  `round(3.5) = 4`; measured).
- `U` is drawn only when `new ≤ 35`; the condition short-circuits.
- The expected yards of a play before rounding and clipping is `4 + 26·(A_off − A_def)`.
- There are no punts, field-goal attempts on earlier downs, turnovers, penalties, safeties or clock.
  A loss behind the offense's own goal line is not a safety, so `yardline_100` can exceed 99.

**On-field selection (`_pick_onfield`, `synth.py:107-128`).**

- **Offense.** For each slot (QB, RB, WR, WR, TE), draw `U`. If `U < 0.80` or the position has no
  backups, take `starters[slot mod n_starters]`. Otherwise take a backup chosen uniformly *with
  replacement across slots*, so both WR slots can draw the same backup (the duplicate ids of §3.3;
  plays-contract D-8).
- **Defense.** Draw `U`. If `U < 0.7`, take the first seven of the defensive pool sorted by
  `is_starter`, which is the seven starters. Otherwise take seven of the twelve uniformly without
  replacement.
- The team index sorts by `is_starter` with pandas' default, non-stable sort (`synth.py:137,145`). The
  order among starters is therefore an implementation detail of the pinned pandas/numpy versions. It
  matters only for the draw tape (§10.3 S-2), not for the distribution.

### 4.4 The focus QB (`synth.py:86-104,170-210`)

The focus QB is the starting QB of team 0 (`synth.py:171-172`; id 0 in the canonical world). His
weekly true ability, weeks 1–14:

```text
τ_w = 0.05 + 0.025·w                          w ≤ 7    → 0.075, 0.100, …, 0.225
τ_w = NaN (did not play)                      w ∈ {8, 9}
τ_10 = 0.06                                   diminished return
τ_w = 0.06·(1 − f) + 0.225·f,  f = min(1, (w − 10)/3)    w ≥ 11 → 0.115, 0.170, 0.225, 0.225
```

- **On every snap he plays,** his contribution to `A_off` is `τ_week`, never his static draw
  (`synth.py:196-201`).
- **While injured** (`off_team = 0` and `τ_week` is NaN):
  - his contribution is skipped;
  - backup QB `backups[0]`'s ability is added;
  - the focus id is removed and the backup is prepended to `off_pl` (`:204-210`).
- **Duplicate backup.** If the backup had already been drawn into the QB slot (about 20% of snaps),
  he is listed twice and his ability is counted twice: 30 legacy plays, 28 fixed (plays-contract D-8).
- **Static draw never used.** The static draw `gt["ability"][focus_qb]` = 0.1001 is never used to
  generate a play, but it is what the Tier-0 and golden gates compare his rating with (§8.3 N-2).

### 4.5 Legacy planted team strength (`_team_strength`, `synth.py:285-294`)

```text
T_t = mean_{p starter, pos ∈ {QB, RB, WR, TE}, team t} a_p  −  mean_{p starter, pos = DEF, team t} a_p
```

- `T_t` uses the static focus-QB draw for team 0.
- With `cb_split` it omits the CB-labelled starters (KI-G3).
- It is coherent only with KI-NEW-Y0 (§8.1). Canonical SD: 0.0157.

### 4.6 Feeder league (`make_college`, `synth.py:297-317`)

With a separate generator `default_rng(seed + 99)` (seed 106 in the canonical world), for every
player in `gt["ability"]` order:

```text
feeder_sv   = 0.62·a_p + N(0, 0.030²)
feeder_snaps ~ uniform integer in [250, 900)      (never read by any consumer; plays-contract §6)
is_rookie   = U < 0.4                              (105 of 288 in the canonical world)
```

Every player has a feeder season. The focus QB's feeder value uses his static draw.

### 4.7 Market (test harness, not the generator)

`market_t = T_t + N(0, 0.01²)`, drawn with `default_rng(1)` in team order 0…11
(`tests/grid/test_tier0_recovery.py:33-34`; `golden_master.py:69-70`; `run_demo.py:44-45`; the
determinism and calibration fixtures). It is in **ability units**: planted SD 0.0157, with noise
0.0154. The quantity it anchors in RAPM is in dV units (EP per play), so it is mis-scaled by roughly
an order of magnitude, independent of its sign (rapm-attribution.md §4.4).

### 4.8 Random streams and their order

| Stream | Seed | Draws, in order |
|---|---|---|
| World | `default_rng(seed)` | (1) 288 normals for abilities; (2) one shuffle per week for the schedule; (3) per drive one normal for the start; per play the `_pick_onfield` draws (five offense uniforms plus one integer per backup slot; one defense uniform plus a seven-of-twelve choice on rotation), one normal for yards, and one uniform for a field-goal decision when in range |
| College | `default_rng(seed + 99)` | per player: one normal, one integer, one uniform |
| Market (harness) | `default_rng(1)` | one normal per team |

The defender fix inserts a second `_pick_onfield` call into step (3) of the world stream. Measured
consequence:

- **Unchanged:** players, abilities, `team_strength`, `focus_tau`, the schedule (all draws before the
  first play) and the separately seeded college frame. The first drive's start (83) is unchanged.
- **Changed:** every draw after the first play's selection. The second drive already starts at 62
  instead of 71, and the world has 17,752 plays instead of 16,825.

The fix's extra draws are discarded (the unused offense of `def_team` and the unused defense of
`off_team`). They change the stream, not the distribution of what is used.

### 4.9 Constants and provenance

Provenance key: **hand-set v0** = present at `f3b641f`, the oldest shallow-clone boundary in CN
(2026-06-21), with no calibration record (the v0 upload `6b0eeee` exists on GitHub only; critic
G-3). The defender bug is present at that boundary (`synth.py:181` there).

| Constant | Value | Defined at | Provenance |
|---|---|---|---|
| On-field offense / defense | QB 1, RB 1, WR 2, TE 1 / 7 | `synth.py:32-33` | hand-set v0 |
| Roster depth | QB 2, RB 3, WR 5, TE 2, DEF 12 | `synth.py:35` | hand-set v0 |
| `ABILITY_SD`, `STARTER_BONUS`; backup penalty 0.5·bonus | §4.1 table | `synth.py:37-38,71` | hand-set v0; "interpretable-ish as EPA/play marginal contribution" (`:36`) |
| Starter slot probability; DEF base-unit probability | 0.80; 0.7 | `synth.py:117,124` | hand-set v0 |
| Yards: gain, mean, noise SD, clip | 26.0; 4.0; 3.2 canonical (4.2 default); [−8, 60] | `synth.py:46-47,214-215`; `data_adapters.py:32` | hand-set v0 |
| Drive start | `clip(N(75, 8²), 60, 95)` | `synth.py:188` | hand-set v0 |
| Drive cap; drives per side per game | 12; 12 canonical (10 default) | `synth.py:48-49`; `data_adapters.py:32` | hand-set v0 |
| Field-goal range and probability | 35; 0.82 | `synth.py:50,232` | hand-set v0 |
| Focus-QB trajectory | base 0.05, slope 0.025, out weeks 8–9, return 0.06, 3-week recovery | `synth.py:92-103` | hand-set v0 |
| Feeder factor, noise, snaps, rookie share | 0.62; 0.030; [250, 900); 0.4 | `synth.py:51-52,298,312-314` | hand-set v0 |
| `cb_split`, `n_cb_per_team` | False, 2 | `synth.py:53-54,72-79` | Phase-4 Task 7 (absent at `f3b641f`) |
| World / college / market seeds | 7 / 7 + 99 / 1 | `synth.py:45,308`; test harness | hand-set v0 |
| Canonical overrides pinned | `CANONICAL_SYNTH` | `golden_master.py:42-48` | CN PR #65 (golden master) |
| Tier-0 floors | §7.1 | `test_tier0_recovery.py:86-143` | CN PR #64; calibrated below the legacy seed-7 values |
| QB `r_scale` 0.55 and the calibration bands | — | `statespace.py:121-133`; `test_calibration_synth.py` | CN PR #66; **tuned on the legacy world** (§8.1) |

### 4.10 Required: the corrected canonical world (proposed — DR-B4, DR-B5; `PARITY.md` ledger entry 1)

The corrected canonical world is the legacy generative model with the following changes. G-1 is the
only one that changes any draw.

- **G-1. Defenders from `def_team`.** The oracle correction SHOULD be exactly the measured patch:
  two `_pick_onfield` calls, offense from `tidx[off_team]` and defense from `tidx[def_team]`
  (`_common.py:175-179`). Its effects are the "fixed" columns throughout this spec and critic G-1,
  so they are already measured and reviewed. A different code shape (separate offense and defense
  selection) is equivalent in distribution but changes the stream again, so it would need its own
  measurements.
- **G-2. Planted net strength = offense quality + defense quality** (proposed — DR-B5). Defenders
  subtract from the yards of the offense they face, so a larger defensive ability is a better
  defense, and a spread prices `off + def` (critic X-2; rapm-attribution.md §1.3).
  - **Primary (exposure-weighted):**

    ```text
    Q_off(t) = mean over t's offensive plays i of A_off,i
    Q_def(t) = mean over t's defensive plays i of A_def,i
    N*(t)    = Q_off(t) + Q_def(t)
    ```

    `A_off,i` and `A_def,i` are the generating sums of §4.3, including the focus QB's `τ` and doubled
    duplicates. This is what produced the yards, and it is the truth that the gauge-invariant
    `E_off`, `E_def` and `N` of rapm-attribution.md §4.5 estimate.
  - **Secondary (starter means):** `mean(offense starters) + mean(DEF and CB starters)`. This is the
    one-line replacement of `_team_strength`.
  - **Measured on the fixed world:**
    - corr(primary, secondary) = 0.975;
    - realized season point margin vs the primary 0.841, vs the secondary 0.855, vs the legacy
      `off − def` 0.754.
  - **Measured on the legacy world:** margin vs `off − def` 0.719, vs the secondary 0.427.
  - `gt` emits `Q_off`, `Q_def` and `N*` (both forms) as **new keys**. Ledger entry 1 keeps the legacy
    `team_strength` key, labelled void, so the unchanged team gate still runs (§8.3 N-4). Ledger
    entry 2 moves the team, defense-effect and grade gates, and the harness market, to the primary.
- **G-3. CB counted in defensive aggregates** (KI-G3).
- **G-4. Effective truth for time-varying players.** Recovery gates compare season ratings with
  `a_eff(p)`, the mean of the generating ability over the player's snaps. For the focus QB that is
  the snap-weighted mean of `τ`. The static draw is kept and labelled unused. Measured QB-position
  recovery correlation, with the focus QB's truth taken as his static draw vs as the plain mean of
  `τ` over played weeks: 0.869 vs 0.897 (legacy), 0.8845 vs 0.9089 (fixed). G-2 to G-4 add truth
  keys and change no draw.
- **G-5. Market in declared units.** From ledger entry 2, `market_t = κ·N*(t) + N(0, σ_m²)`, with
  `κ`, `σ_m` and the seed declared in the truth bundle. Until DR-D10 fixes the
  spread → EP-per-play conversion, the harness keeps `κ = 1` (ability units) and `σ_m = 0.01` and
  labels them.
  No Layer-3 recovery claim is made from that anchor; its value is the residual hook
  (rapm-attribution.md §4.4 item 5).
- **G-6. Duplicates stay, in this profile.** The canonical corrected world keeps the legacy
  selection rule, duplicates included (258 plays). Removing them changes the stream and the
  distribution. Fixture-injected parity treats participation as a multiset (plays-contract D-8). The
  realistic profile forbids duplicates (§4.11).
- **G-7. Regenerate and re-set.** These follow DR-B1's rules: a failing test first, a model-spec note,
  and a golden regeneration with a reviewed semantic explanation.
  - Regenerate `tests/grid/golden/snapshot.npz`.
  - Re-record the observed values with every ledger entry. Re-set the Tier-0 floors once, after the
    last entry that moves a Tier-0 statistic (entries 1, 2 and 4 of `PARITY.md` (b)), over a seed
    ensemble (§7.6; DR-D26).
  - Recalibrate the QB `r_scale` and the NIS bands (KI-NEW-S2; the exploratory sweep is in
    state-space-kalman.md §7.2).
- **G-8. V\* (SHOULD).** The generator SHOULD emit the true value function of its own state process
  for the league-average lineup, by exact dynamic programming or a declared Monte-Carlo budget, so
  that V(s) recovery can be gated (value-model.md §7.5). The 12-play cap makes V\* depend on the play
  count, not only on `s` (value-model.md §7.4). In this profile V\* is therefore defined on
  `(s, plays elapsed)`. The realistic profile removes the cap.

### 4.11 Required: the "realistic" profile (proposed — DR-B4; KI-NEW-Y2, KI-NEW-A3)

The canonical world is easiest exactly where real data is hardest: its QB effect is about six times
the real one (KI-NEW-Y2) and its starters rotate out on about 20% of snaps. A second profile MUST
exist in the Rust parity suite before GRID components are trusted on real-data scale questions. Its
requirements:

| # | Requirement | Current canonical world | Real reference (historical, non-parity) |
|---|---|---|---|
| R-1 | A healthy starting QB on at least about 99% of his team's offensive snaps; other positions' rotation set from real snap-share summaries | QB starter share 0.788 (legacy) / 0.793 (fixed) | starting QB about 100% of team offensive snaps (cn-issues NEW-A3) |
| R-2 | Planted ability scaled so that the fitted RAPM rating SD is about 0.04 EP per play **at every position** | rating SD QB 0.272 / 0.236, RB 0.092 / 0.090, WR 0.081 / 0.079, TE 0.072 / 0.059, DEF 0.080 / 0.073 (Tier-0 configuration, legacy / fixed; §1.3). cn-issues reports synth QB 0.259 from a single `run_rapm` | real 2023 single-season RAPM: QB 0.042, RB 0.044, WR 0.045 (`tools/investigations/real_rapm.py`; KI-NEW-A3, KI-NEW-P4) |
| R-3 | About six offensive skill ids and eleven defenders per play; no duplicate ids | five and seven; duplicates on 1.5% of plays | six ids on 34,475 of 35,474 plays; eleven defenders on 35,451 (plays-contract §6.3) |
| R-4 | A football-valid state machine: `yardline_100` in 1…99 with safeties; goal-to-go; punts and field-goal attempts as fourth-down decisions; turnovers; end of half; no play-count cap | no safety, no goal-to-go, no punts or turnovers, a 12-play cap ending 24–29% of drives | — |
| R-5 | League shape: 32 teams, byes, several seasons with persistent player identities | 12 teams, 14 weeks, one season | 32 teams (plays-contract §2.1) |
| R-6 | The identifiability stress of KI-NEW-A3 is present: the starting QB is nearly collinear with his team's offense intercept | absent (QB rotates) | corr(starter QB rating, own `γ_off`) = 0.13 on real 2023 |

- Parameter values derived from real-data summaries are recorded as historical evidence. No byte of
  real data enters a fixture (DR-A11).
- The realistic profile has no oracle counterpart unless a Python reference is added through the
  correction ledger. Its gates are Rust-native Class D floors, calibrated below observed on the
  profile itself. They are expected to be far below the canonical floors. Documenting that drop is
  the purpose of the profile.

### 4.12 Required before Layers A–F: the stat-vector world (proposed — DR-B4, option 3)

The current world plants a per-play efficiency signal only. It cannot validate Layers A, B, C or F
(engine-spec §6.1), the stat-vector targets (§5.1), PB-MAE or predictive distributions (engine-spec
§7.13.6; reconcile-spec-first §6.4). Before any Layer A–F gate is written (P1-07, P1-08), a revision
of this spec MUST define a stat-vector world with at least:

| Layer (engine-spec §6.1) | Planted truth | Recovery target |
|---|---|---|
| A availability | per player-week active/inactive and start probability; injury hazards and return effects; status reports with information timestamps | active and start probabilities; Brier and calibration |
| B team environment | plays per game (pace), pass rate, scoring environment, net strength `N*`; a market spread and total **quoted at a lock timestamp** | team volume; implied points; the market-line lock rule (engine-spec §4.3) |
| C opportunity | snap, target, carry and red-zone shares with persistence and drift, summing to the team totals | shares; the sum-to-team constraint |
| D efficiency | per-opportunity rates (catch rate, yards per target or carry, TD rate) driven by the same latent abilities as the play-level world | component rates; where GRID talent enters (DR-C3) |
| E matchup | defensive strength by unit (pass and run) | matchup grade `+E_def`, higher = tougher (DR-B5) |
| F simulation and coupling | a joint game model: shared team plays, game-script coupling between score state and pass rate, teammate share competition, QB–receiver co-movement | simulated correlations; stat-vector quantiles |
| Scoring | stat vectors scored by every built-in profile | exact (Class A) |
| Time and identity | several seasons; rookies with feeder seasons (cross-league-priors.md); coaching changes with announcement timestamps (DR-D1); participation published only after each postseason (DR-C1); an optional stat-correction stream | the three-season window (DR-C6); publication-lag and leakage tests (P1-05); scheme resets (state-space-kalman.md) |

- **One truth.** The play-level GRID world MUST be generated from the same latent abilities as the
  stat vectors, so that GRID signals and Layers A–F are validated against one truth.
- **Truth bundle.** The bundle carries per player-week expected stat vectors and distribution
  parameters, so that the superseded alpha-spec §12.2 goldens can be computed against truth: team
  volume, shares, stat means, quantiles, scoring, PB-MAE, Brier and calibration, and promotion
  decisions.
- **Not set here.** This spec sets no distribution or parameter value for this world. The revision
  that adds them is approved by the statistical owner before P1-07 starts.

---

## 5. Algorithm, numerics and determinism

### 5.1 Size and cost

The canonical world is generated in about 0.5 s (measured, one thread). As fixtures, the legacy
world is about 4 MB at 8-byte widths (parity-fixture-contract §3).

### 5.2 Seeds and determinism (oracle)

- The world is deterministic for a given configuration and seed **under the pinned numpy and pandas
  versions**. numpy `Generator` method streams (`normal`, `integers`, `choice`, `shuffle`) are tied
  to the numpy version, and the starter order depends on pandas' sort (§4.3). A
  `requirements.lock` bump is therefore an oracle change (`reference/python/PARITY.md` (g)).
- Seeds of the canonical validation run: world 7, college 106, market 1. Downstream estimators have
  their own seeds: V(s) and Layer-1 boosters `random_state = 0`, Layer-1 `KFold(random_state=0)`, and
  the priors' equivalency split `default_rng(3)`. These are not part of the world but they move the
  gate values (value-model.md §5.2; layer1-credit.md §5.3).
- Yards use round-half-to-even (§4.3). A replay of the oracle's draws must use the same rounding.

### 5.3 Why Rust need not match numpy's random streams, and what is matched instead

**The Rust generator MUST NOT be required to reproduce numpy's PCG64 streams.** Four reasons:

1. **Coupling without benefit.** Bit-matching would bind the Rust generator to the internals of
   numpy's `Generator` methods, which change with the numpy version, and to pandas' sort, for no
   statistical gain (critic X-17; DR-B2).
2. **Every correction already breaks the stream.** The defender fix alone changes every play (§4.8).
   A stream-matching requirement would be broken by the first approved correction.
3. **A gate that passes for one stream only is a gate on luck.** Across synth seeds 1–11, every
   non-canonical seed fails at least one canonical Tier-0 floor, on both generators (§7.6). A Rust
   stream is, statistically, just another seed.
4. **Numeric parity does not need it.** Downstream stages get exported oracle frames by stage-isolated
   injection (parity-fixture-contract §5), so no numeric target depends on regenerating the world.

**What is matched instead:**

- **(a) The exported worlds, byte-exact.** `synthetic-world/canonical-legacy` and, after ledger
  entry 1, `canonical-corrected` are committed fixtures. Rust loads them exactly (S-1).
- **(b) The generative model, exactly, by draw-tape replay.** The oracle exporter records each draw's
  *result* in order:
  - the normal values;
  - the uniform values;
  - the chosen backup index;
  - the chosen seven defenders;
  - the shuffled week order.

  It does this by wrapping the generator inside the exporter process; `synth.py` is never edited.
  The Rust generator takes its randomness through an injectable draw source. Fed the tape, it MUST
  reproduce the oracle's `plays`, `players`, `gt` and `college` exactly (S-2; proposed —
  DR-D28). This tests every equation, constant, branch and rounding rule of §4
  without reimplementing numpy's algorithms, because Rust consumes results, not raw bits.
- **(c) The distribution, statistically.** The Rust-native generator, with its own seeded PRNG, must
  pass world-level checks (S-4) and the Class D recovery gates (S-5) over a declared **seed
  ensemble**, against floors re-set on the corrected oracle's ensemble (§7.6;
  DR-D26).

### 5.4 Numerical facts

- The world has no floating-point accumulation beyond sums of at most 13 abilities per play.
- The parity issues are all in the draws, not in arithmetic. Once draws are injected, every output
  is exact.

---

## 6. Incremental behaviour

The oracle's world is static: one call, one season, one stream. There is no incremental generation.
The engine needs more:

1. **Multi-season worlds** with persistent player identities, for the three-season window (DR-C6),
   the season-boundary refresh (rapm-attribution.md §6.5), publication lag (DR-C1) and walk-forward
   replay (P1-09).
2. **Stream isolation (MUST).** Each component draws from its own substream, derived
   deterministically from `(world seed, component, season, game)`. Components are rosters, schedule,
   each game, college, market and availability. A local change to one component then perturbs only
   that component. Adding a season or a profile option never reshuffles earlier seasons. This is
   the structural fix for the KI-NEW-Y0 situation, where one corrected line moved every number in
   the world. A property test asserts it (S-7).
3. **Deterministic extension.** Generating seasons `1…S+1` reproduces seasons `1…S` bit-exactly.

---

## 7. Validation evidence (the gates that consume the world)

### 7.1 Tier 0: recovery (`tests/grid/test_tier0_recovery.py`, 8 tests)

Canonical world, `fit(n_iter=3)`, market seed 1.

| Gate | Floor (line) | Legacy (seed 7) | Fixed (seed 7) |
|---|---|---|---|
| Pooled corr(rating, ability) | ≥ 0.77 (`:86`) | 0.8025 | 0.8276 |
| QB / RB / WR / TE / DEF | ≥ .83 / .70 / .76 / .73 / .73 (`:91`) | .8690 / .7433 / .7973 / .7713 / .7653 | .8845 / .7711 / .8599 / .7784 / .7827 |
| Team corr(`γ_off − γ_def`, legacy `off − def`) | ≥ 0.60 (`:99`) | 0.6643. **Void** (coherent only with the bug) | 0.6596, against a target that is wrong on this world |
| Kalman total_smooth / tau_smooth corr | ≥ 0.92 / ≥ 0.60 (`:105-106`) | 0.9580 / 0.6745 | 0.9760 / 0.6538 |
| Variance widens at the injury return | strict (`:113-116`) | 0.00228 → 0.00614 | 0.00233 → 0.00738 |
| Focus-QB NIS | ≤ 10 (`:128`) | 4.581 | **8.371** |
| Equivalency slope / OOS R² | [0.9, 1.8] / ≥ 0.05 (`:137-138`) | 1.3159 / 0.1489 | 1.2119 / 0.2134 |
| Rookie prior corr | ≥ 0.50 (`:143`) | 0.5829 | 0.5829 |

All 8 pass on both generators at seed 7. Pointed at the corrected truth of §4.10, the unchanged team
gate would measure the wrong estimand against the right truth: corr(`γ_off − γ_def`, planted net,
secondary form) = 0.465 on the fixed world (0.566 legacy), below the 0.60 floor. The team gate
therefore moves to net strength only in ledger entry 2, together with the estimand (E_off + E_def
against `N*`; rapm-attribution.md §7.2 measures 0.959). Entry 1 leaves it on the legacy key (§8.4).

### 7.2 Tier 0.5: golden master (`tests/grid/golden_master.py`, `tests/grid/test_golden_master.py`, 11 tests)

**What the golden holds.**

- **Snapshot:** 17 arrays (`golden_master.py:95-116`):
  - ids and order: `player_id`, `position`, `team`, `qb_week`;
  - Layer C numerics: `rating`, `ability`, `team_rating`, `planted_team`, `qb_credit`, `k_total_filt`,
    `k_total_smooth`, `k_tau_smooth`, `k_var_total_filt`, `k_total_pred`, `k_var_total_pred`;
  - truth anchors: `focus_tau`, `focus_played`.
- **Layer A (5 tests, truth-anchored):**
  - shapes and dtypes;
  - rating sign, pooled and per position;
  - team sign, with the strongest planted team in the estimated top 2;
  - talent more persistent than total;
  - the injury dip and variance widening.
- **Layer B (3 tests, ordering):** top-10 players; team order; the steepest drop at index 8.
- **Layer C (3 tests, numeric):** id arrays identical; ratings and team ratings; QB credit and the
  Kalman arrays at `rtol 1e-5`, `atol 1e-6`.

**Measured movement.**

| Condition | Result |
|---|---|
| Legacy, threads = 1 | identical to the committed golden: max \|Δ\| = 0.0 on every array |
| Legacy, threads = 4 | ids, top-10 and team order unchanged; `rating` 9.6e-4, `team_rating` 3.2e-4, `qb_credit` 0.021, `k_total_filt` 0.0185, `k_total_pred` 0.0173, `k_total_smooth` 0.0101, `k_tau_smooth` 0.0093; variances 0.0. Layer C would fail. V(s) itself is thread-invariant (value-model.md §5.2) |
| Fixed, threads = 1 | **4 of 11 fail**: `test_layerB_top_player_ranking` (top-10 overlap 7 of 10), `test_layerB_team_ranking_order`, `test_layerC_player_and_team_ratings`, `test_layerC_qb_weekly_and_kalman`. Max \|Δ\|: `rating` 0.171, `team_rating` 0.265, `qb_credit` 0.317, `k_total_filt` 0.247, `k_total_pred` 0.239, `k_total_smooth` 0.180, `k_tau_smooth` 0.123, `k_var_total_filt` 1.24e-3, `k_var_total_pred` 1.39e-3. `ability`, `planted_team`, `focus_tau`, `focus_played` and all id arrays are identical (§4.8). All Layer A tests pass |

### 7.3 Tier 0.5: synthetic calibration (`tests/grid/test_calibration_synth.py`, 6 tests)

The setup is QB parameters `SSParams.from_position("QB")`, week-grouped cross-fit residuals, 24 QBs,
310 QB-weeks, and week 1 dropped (`:97`). Values from state-space-kalman.md §7.2, reproduced by the
2026-10-07 plugin run:

| Gate | Band (line) | Legacy | Fixed |
|---|---|---|---|
| Sample | ≥ 20 QBs, ≥ 200 weeks (`:107-110`) | 24, 310 | 24, 310 |
| Median per-QB NIS | [0.8, 1.25] (`:117`) | 0.9266 | 1.2198 |
| Pooled NIS | [0.8, 1.4] (`:124`) | 1.2369 | **1.8289 (fails)** |
| PIT mean | within 0.07 of 0.5 (`:129`) | 0.5158 | 0.5145 |
| PIT SD | [0.24, 0.34] (`:136`) | 0.2956 | 0.3112 |
| PICP@80 | [0.70, 0.90] (`:143`) | 0.7774 | 0.7452 |

`r_scale = 0.55` for QBs was tuned against these bands on the legacy world (test docstring
`test_calibration_synth.py:15`; CN PR #66; 0.35 before).

### 7.4 Tier 0.5: determinism (`tests/grid/test_determinism.py`, 2 tests)

- Two in-process runs of world → V(s) → dV → `fit` agree to `atol 1e-9` on dV, ratings, team ratings
  and weekly QB credit (`:50-60`).
- A seeded 12-week `kalman_step` sequence agrees to 1e-12 (`:63-83`).
- Both pass on both generators.
- The guarantee is in-process only (`:8-12`).

### 7.5 World-level semantics (measured)

| Statistic (12 teams, season totals) | Legacy | Fixed |
|---|---|---|
| corr(realized point margin, legacy `off − def`) | 0.719 | 0.754 |
| corr(realized point margin, planted net, secondary form) | 0.427 | **0.855** |
| corr(points scored, own DEF starter quality) | **−0.337** | −0.049 |
| corr(points allowed, own DEF starter quality) | **+0.482** | **−0.600** |
| corr(`team_rating = γ_off − γ_def`, planted net, secondary form) | 0.566 | 0.465 |

The legacy rows are the defender bug made visible:

- a team's own defenders suppress its own scoring;
- its "defense" has no bearing on what it allows, so the correlation even has the wrong sign;
- the off − def target is the right description of that broken world.

### 7.6 Seed-to-seed spread of the Tier-0 statistics (measured; new)

Same configuration as §7.1, generator seed varied (1–11; 7 is canonical). Min / median / max:

| Statistic (floor) | Legacy | Fixed | Fixed seeds failing the floor |
|---|---|---|---|
| Pooled (≥ 0.77) | 0.654 / 0.803 / 0.859 | 0.646 / 0.828 / 0.855 | 1 (seed 8) |
| QB (≥ 0.83) | 0.559 / 0.835 / 0.915 | 0.571 / 0.857 / 0.924 | 4 (6, 8, 9, 10) |
| RB (≥ 0.70) | 0.644 / 0.786 / 0.874 | 0.716 / 0.848 / 0.927 | 0 |
| WR (≥ 0.76) | 0.782 / 0.848 / 0.869 | 0.797 / 0.861 / 0.892 | 0 |
| TE (≥ 0.73) | 0.460 / 0.724 / 0.876 | 0.363 / 0.751 / 0.850 | 5 (1, 4, 6, 10, 11) |
| DEF (≥ 0.73) | 0.689 / 0.767 / 0.810 | 0.712 / 0.745 / 0.835 | 1 (5) |
| Team vs legacy target (≥ 0.60) | −0.128 / 0.599 / 0.776 | −0.352 / 0.575 / 0.740 | 6 |
| Kalman total_smooth (≥ 0.92) | 0.908 / 0.958 / 0.985 | 0.622 / 0.934 / 0.976 | 5 (2, 3, 5, 9, 10) |
| Kalman tau_smooth (≥ 0.60) | 0.620 / 0.675 / 0.695 | 0.566 / 0.654 / 0.699 | 1 (9) |
| Focus-QB NIS (≤ 10) | 3.08 / 4.52 / 7.33 | 4.03 / 5.56 / 8.64 | 0 |
| Equivalency slope ([0.9, 1.8]) | 0.769 / 1.540 / 2.464 | 0.933 / 1.680 / 2.475 | 5 (3, 4, 5, 6, 10) |
| OOS R² (≥ 0.05) | 0.076 / 0.308 / 0.618 | −0.015 / 0.370 / 0.642 | 1 (9) |
| Rookie prior (≥ 0.50) | 0.539 / 0.679 / 0.721 | 0.539 / 0.679 / 0.721 | 0 |

- **On both generators, every one of the ten non-canonical seeds fails at least one Tier-0 floor.**
  Only seed 7, on which the floors were calibrated, passes all eight.
- The floors are calibrated below one realization, not below the generator. The world's play count
  also varies with the seed, from 16,390 to 19,014 plays (fixed).
- Consequences:
  - A Rust-native generator cannot be held to single-seed floors (§5.3).
  - Re-setting the floors on the corrected world MUST use an ensemble statistic (proposed —
    DR-D26).
    - The default: gate the **ensemble median** over a declared seed set at a calibrated-below
      floor.
    - Optionally also gate a declared low quantile, for per-seed robustness.
    - The oracle's seed-7 world stays the fixture-injected golden.
- The probe was run with `fit(n_iter=3)` and every downstream seed fixed. The estimators' own seeds
  add further spread (value-model.md §5.2; layer1-credit.md §5.3).

### 7.7 Aspirational targets (stretch goals, not gates)

CN's documentation stated targets that were never implemented or met. They are recorded here only
as stretch goals (engine-spec §7.13.2) and MUST NOT be cited as gates or parity targets:

- pooled attribution ≥ 0.85;
- team strength ≥ 0.90;
- Kalman trajectory ≥ 0.80;
- focus-QB NIS in [0.8, 1.25];
- prior OOS R² ≥ 0.50;
- situation-RAPM recovery ≥ 0.65;
- changepoint precision ≥ 0.6 and recall ≥ 0.5;
- WR–CB interaction recovery. The oracle test asserts only a non-empty, finite result
  (reconcile-code-first C25).

None of the last three effects is planted.

---

## 8. Known defects and required engine behaviour

### 8.1 KI-NEW-Y0 in full: defenders drawn from the offense team (CRIT, P0; ledger entry 1)

**Mechanism.**

- `synth.py:193` reads `off_pl, def_pl = _pick_onfield(tidx[off_team], rng)`.
- `_pick_onfield` returns both lists from the single team index it is given (`synth.py:107-128`).
- Every play's "defenders" are therefore the offense's own DEF players: 100% of the 16,825 canonical
  plays, and 0% from `def_team` (§3.3).
- The bug is present at the oldest visible commit (§4.9).

**Why `_team_strength` looks right in the bug world.** In the bug world:

- a team's yards are `26·(own offense − own defense)`;
- a team's defensive plays are faced by the *opponent's* own defenders.

So the realized margin of team `t` is driven by `off_t − def_t`: margin vs `off − def` 0.719, vs net
0.427 (§7.5). The planted `T_t = off − def` (`synth.py:285-294`) is the right description of that
world, and only of that world (KI-NEW-Y1; critic X-2).

It follows that:

- the `t_def` intercept never sees a planted signal;
- "DEF recovery" (0.7653) measures recovery of an own-offense-suppressing ability, not of defense;
- the Tier-0 team gate (0.6643) measures `γ_off − γ_def` against the bug's own target;
- the harness market anchors `γ_off + γ_def` to `off − def`, which is inconsistent in either world.

The `[+1, −1]` Layer-3 row queued in CN's documents would make the code agree with the bug. It is
rejected (DR-B5; critic X-2, X-19).

**What the fix changes.** The fix makes two `_pick_onfield` calls. The world stream shifts after the
first play's selection (§4.8):

- players, abilities, `team_strength`, `focus_tau`, the schedule and college are unchanged;
- every play changes: 17,752 plays instead of 16,825; outcomes 399 / 86 / 1,531; 258 duplicate
  plays instead of 245.

A one-line patch is therefore not a correction by itself. The corrected world needs its own
planted-strength definition, its own golden and re-set gates (§4.10).

**Which gates and goldens move.**

| Suite | On the fixed world |
|---|---|
| Tier 0 (8) | all values move (§7.1); all 8 pass at seed 7 |
| Golden master (11) | 4 fail (§7.2); `qb_credit` moves by up to 0.317 |
| Calibration (6) | 1 fails: pooled NIS 1.829 against [0.8, 1.4] (§7.3) |
| Determinism (2) | pass |
| `run_demo.py` and the "observed" comments in the gate files | every printed or quoted value moves |

**Calibration tuned on the bug.**

- CN PR #66 raised the QB `r_scale` from 0.35 to 0.55 against the calibration bands on the legacy
  world.
- On the fixed world the median per-QB NIS (1.2198) sits 0.03 inside its band and the pooled NIS
  fails.
- Focus-QB NIS reads 8.371 against the ≤ 10 blow-up guard (KI-NEW-S2).
- The exploratory `r_scale` sweep on both worlds is in state-space-kalman.md §7.2. It is input to
  the DR-B4 recalibration, not a proposed value.

**Required.**

- The Rust generator draws defenders from `def_team`.
- Its planted truth is net quality (G-2).
- Its golden and Class D floors are set on the corrected world (G-7).
- No legacy number becomes a Rust target (DR-B1).

### 8.2 Other registered synthetic-world defects

| ID | Defect | Status | Required behaviour |
|---|---|---|---|
| **KI-NEW-Y1** | Planted team strength is `mean(off starters) − mean(DEF starters)` | HIGH; fixed together with KI-NEW-Y0 (critic X-2) | G-2 (net = off + def, exposure-weighted primary) |
| **KI-NEW-Y2** | QB effect about 6× real; starters rotate out on about 20% of snaps | MOD | Realistic profile (§4.11) |
| **KI-G3** | With `cb_split`, CB starters are omitted from the team defensive mean (`synth.py:292`) | LOW (only `cb_split` worlds) | G-3 |
| **KI-G7** | `drive_points or 0.0` float truthiness (`synth.py:260`) | FALSE-POSITIVE (a no-op) | — |
| **KI-G11** | `__main__` divides by `terminal.mean()` (`synth.py:324-326`) | FALSE-POSITIVE (dead code) | not ported |
| **KI-NEW-S2** | The focus-QB NIS ≤ 10 gate is a blow-up guard; the QB calibration was tuned on the legacy world | MOD | §7.3, G-7 |
| plays-contract D-8; KI-NEW-Z38 | Duplicate offensive ids (245 legacy plays); the duplicated ability is counted twice in the yards and summed to +2 in the design | new in plays-contract | Multiset semantics on the canonical profiles (G-6); forbidden in the realistic profile (R-3) |
| plays-contract D-9; KI-NEW-Z39 | `yardline_100` up to 104; `ydstogo > yardline_100`; no safeties | new in plays-contract | Realistic profile R-4; V(s) support declared per profile (value-model.md §3.1) |
| plays-contract D-13; KI-NEW-Z40 | Truth columns live in the input frame | new in plays-contract | `SyntheticTruth` type (§3.4) |
| reconcile-code-first C25; KI-NEW-Z13 | `cb_split` plants no WR×CB effect; the interaction "recovery" test asserts only non-empty and finite | — | No interaction claim until an effect is planted (§4.12) |
| rapm-attribution.md §8 #8; KI-NEW-Z12 | The harness market is in ability units while its estimand is in dV units | new in rapm-attribution.md | G-5; DR-D10 |

### 8.3 Defects found in this consolidation (to be registered)

| # | Defect | Evidence | Required behaviour |
|---|---|---|---|
| N-1 (KI-NEW-Z32) | **The 12-play cap is non-Markov.** 24.1% (legacy) and 29.1% (fixed) of drives end by play count, scored 0 regardless of state. Together with persistent lineup quality this makes synthetic dV not state-centred (E[dV \| down 4] = +0.649 legacy), unlike real data | value-model.md §7.4 (cap removed: +0.344; player effects removed as well: −0.027) | G-8 on the canonical profiles; R-4 removes the cap |
| N-2 (KI-NEW-Z33) | **Focus-QB truth mismatch.** The gates compare his rating with a static draw (0.1001) that generated none of his plays; his plays used `τ` (mean 0.1538) | §4.4; QB corr 0.869 vs 0.897 | G-4: effective truth |
| N-3 (KI-NEW-Z34) | **Single-seed floors.** Every non-canonical seed fails at least one Tier-0 floor on both generators | §7.6 | DR-D26 |
| N-4 (KI-NEW-Z35) | **Truth and estimand must move together.** Pointing the unchanged team gate (`γ_off − γ_def`) at net truth gives corr = 0.465 < 0.60 | §7.1 | Entry 1 adds the net-truth keys and keeps the legacy key; entry 2 moves gate, estimand and market together |
| N-5 (KI-NEW-Z36) | **No value-function truth** is planted, so V(s) has never been validated against truth | value-model.md §7.5 | G-8 (SHOULD) |
| N-6 (KI-NEW-Z37) | **The schedule silently idles a team** for odd `n_teams`, and is not a round robin | `synth.py:151-159` | Typed config error; the realistic profile uses a real schedule shape (R-5) |

### 8.4 Required corrections and their order (proposed — DR-B1, DR-B4, DR-B5)

The order is binding. Each later step is measured on the world the previous step produced (critic
B-1, X-4).

1. **Oracle ledger entry 1** (one reviewed commit in `reference/python/`; `MANIFEST.tsv` status
   `patched:<ID>`):
   - G-1 exactly as measured;
   - the new truth keys of G-2 to G-4, with `team_strength` kept and labelled void;
   - G-5 labelling of the market's units;
   - the golden regenerated with a semantic note (the four failing tests of §7.2 are its failing
     tests first);
   - the Tier-0 and calibration observed values re-recorded.

   The corrected Tier-0 floors and the QB calibration (G-7) are re-set only after entries 2 and 4
   and the seed-ensemble decision.
2. **Oracle ledger entry 2** (DR-B5):
   - the team gate becomes E_off + E_def against `N*`, and the golden team anchor follows;
   - the harness market is built on `N*` (G-5), and market rows anchor net strength using the line
     at lock;
   - truth-anchored Layer-A tests on the corrected world (rapm-attribution.md §10.3 T-1, T-2).
3. **Ledger entry 3**, the matchup-grade sign, then entries 4 and 5 (`PARITY.md` (b)).
4. **Rust `canonical-corrected` generator** (P1-12):
   - the same generative model as the corrected oracle;
   - draw-tape replay exact (S-2);
   - ensemble Class D floors (S-5).
5. **Rust `realistic` profile** (§4.11), with its own calibrated-below-observed floors, before any
   real-data-scale claim.
6. **Rust stat-vector world** (§4.12), spec revision first, before the Layer A–F gates of P1-07 and
   P1-08.

### 8.5 Typed failures of the Rust generator (proposed — DR-B6)

| Condition | Oracle behaviour | Required |
|---|---|---|
| Odd `n_teams`, or a roster smaller than the on-field count | one team silently idles each week; `starters[slot mod n]` reuse | `SynthConfigError::InvalidLeague` / `InvalidRoster` |
| A probability outside [0, 1], or a negative SD | accepted | `SynthConfigError::InvalidParameter` |
| A profile or seed absent from the manifest | n/a | `SynthConfigError::Unversioned` |
| Truth reachable from estimator code | `ability` sits in `players` | impossible by type (§3.4) |
| Draw tape exhausted or of the wrong kind during replay | n/a | `TapeError::{Exhausted, KindMismatch{expected, found}}` |

---

## 9. Open decisions

| Decision | Question | Proposed default |
|---|---|---|
| **DR-B4** | How is the synthetic world corrected and extended? | §4.10 now; §4.11 and §4.12 before Layers A–F (proposed — DR-B4) |
| **DR-B5** | Team-strength estimand, Layer-3 rows, grade sign | Planted net = off + def quality (G-2); gates on E_off + E_def and `+E_def` (proposed — DR-B5) |
| **DR-B1** | Oracle correction order | §8.4 (proposed — DR-B1) |
| **DR-B3** | Parity tolerances | Class D floors re-set on the corrected world (proposed — DR-B3), over a seed ensemble (next row) |
| **DR-D26** | Single-seed floors fail on every other seed (§7.6). What statistic do Class D gates use? | Gate the median over a declared generator-seed ensemble, calibrated below the corrected oracle's ensemble median, optionally with a declared low-quantile floor. The ensemble and margins are pre-registered by the statistical owner when the floors are re-set. The seed-7 world stays the fixture-injected golden |
| **DR-D28** | How is the Rust generator proven to implement §4 exactly without matching numpy streams? | Draw-tape replay (§5.3 (b)): the exporter records draw results by wrapping the generator in its own process; Rust replays them through an injectable draw source |
| **DR-D10** | The spread → EP-per-play conversion for the market anchor (raised in rapm-attribution.md) | Open, no default. Until decided, the synthetic market stays in ability units and is labelled (G-5) |
| **DR-C1** | Participation publication lag | The stat-vector world carries a publication timestamp per season (§4.12) |
| **DR-C9** | Feeder leagues | NCAA only for alpha. The synthetic feeder stays one league (`league_factor` 0.62) unless DR-C9 adds more |
| **DR-D1** | Dated coaching-change records | The stat-vector world plants changes with announcement timestamps, so the as-of rule can be tested on synthetic data first |

---

## 10. Rust port plan

### 10.1 Homes (engine-spec §8.1; DR-A8 adopted subject to ratification)

| Piece | Crate::module | WP |
|---|---|---|
| Fixture loader for the exported worlds (`from_legacy_v0`) | `synth::legacy_v0` | **P1-01** (crate skeleton and loader) |
| Corrected canonical generator: the §4 model with G-1 to G-8 and an injectable draw source | `synth::generator`, `synth::profiles::canonical_corrected` | **P1-12** |
| Draw-tape replay | `synth::tape`; the exporter under `reference/python/tools/parity/` | P1-12 (exporter per parity-fixture-contract §9) |
| `SyntheticTruth` and the truth bundle | `synth::truth` | P1-01 (type), P1-12 (content) |
| Committed synthetic fixtures; multi-season and publication-lag worlds for the leakage harness | `fixtures/parity/synthetic-world/`; `synth::profiles` | **P1-05** |
| Realistic profile | `synth::profiles::realistic` | P1-12, before any real-data-scale claim |
| Stat-vector world | `synth::stat_vector` | before **P1-07** / **P1-08** gates (spec revision first) |
| Recovery harness and Class D gates | `evaluation::recovery` | P1-12 (GRID gates); **P1-09** (backtest on multi-season worlds) |
| Planted changepoints and regime truth for advanced state updates | `synth::profiles` | **P2-03** |

- `synth` is a numerical crate. It stays synchronous, with no Tokio or SQLx dependency (engine-spec
  §8.1 rule 3).
- `models` MUST NOT depend on `synth`. A structural guard SHOULD enforce it.
- The PRNG is seeded, its algorithm is fixed by this spec, and it is recorded in the generator
  version. Choosing a crate for it is a dependency decision that needs written justification and
  owner approval (root `CLAUDE.md`).

### 10.2 What the Rust generator inherits and what it does not

| Inherits exactly | Does not inherit |
|---|---|
| Every equation, constant and branch of §4.1–§4.6, including round-half-to-even and the multiset duplicates of the canonical profiles | numpy's streams (§5.3) |
| The truth semantics of §4.10 | the legacy `team_strength` as a gate target |
| The canonical configuration `CANONICAL_SYNTH` | a market inside the truth without declared units |
| | the static focus-QB draw as his gate truth |

### 10.3 Parity class and concrete targets

Classes are per `docs/03-contracts/parity-fixture-contract.md` (proposed — DR-B3).

| ID | Target | Class | Criterion |
|---|---|---|---|
| S-1 | Load `synthetic-world/canonical-legacy` and, after entry 1, `canonical-corrected` | exact | every column, id order, NaN position and v0 sentinel round-trips (parity-fixture-contract §5 first row) |
| S-2 | Draw-tape replay of the oracle generator (legacy and corrected) | exact | Rust `plays`, `players`, truth and `college` equal the oracle's (proposed — DR-D28) |
| S-3 | Deterministic sub-functions on given inputs: the focus-QB trajectory; legacy `T_t`; `Q_off`, `Q_def`, `N*` from a given world; the drive state machine on given yards and uniforms | exact | §10.4 examples plus the oracle's own values on the canonical world |
| S-4 | World-level distribution of the Rust-native generator over the seed ensemble against the corrected oracle's ensemble: plays per world, outcome shares, truncation share, starter shares, duplicate rate, defender-team share (= 1.0 exactly), state-support extremes | D (world) | Each ensemble mean within a band set from the oracle's own seed spread, pre-registered with the Class D floors |
| S-5 | Recovery gates with the Rust V(s), Rust estimators and Rust-native generator | D | Floors re-set on the corrected oracle's seed ensemble, calibrated below observed (proposed — DR-B3, DR-B4, DR-D26). Starting points: the fixed columns of §7.1 and §7.6. **No floor is set by this spec** |
| S-6 | Golden Layer A truth anchors on every Rust-native world in the ensemble | exact (sign, membership) | as `test_golden_master.py` Layer A, with the team anchor on `N*` and E_off + E_def |
| S-7 | Stream isolation and deterministic extension (§6) | exact | changing one component's parameters leaves every other component bit-identical; seasons `1…S` are unchanged when `S+1` is added |
| S-8 | Typed failures of §8.5 | divergence | Rust asserts the typed outcome |

**Fixture-injected mode.** Golden Layer B and C are checked only with the oracle's exported world and
the oracle's injected intermediates, at the golden's own tolerances (engine-spec §7.13.5). The
legacy golden is audit-only (DR-B1).

### 10.4 Reference examples (become golden unit tests)

**W-1: focus-QB trajectory, weeks 1–14** (exact):
`[0.075, 0.100, 0.125, 0.150, 0.175, 0.200, 0.225, NaN, NaN, 0.060, 0.115, 0.170, 0.225, 0.225]`.
Mean over played weeks: 0.15375.

**W-2: drive state machine** (given yards; `fg_range_yardline = 35`):

| State before | Yards | Draw for field goal | Result |
|---|---|---|---|
| (1, 10, 75) | 12 | — | (1, 10, 63) |
| (3, 2, 30) | 1 | — | (4, 1, 29) |
| (4, 1, 29) | 0 | U = 0.50 | terminal, field goal, value 3 |
| (4, 1, 29) | 0 | U = 0.90 | terminal, value 0 |
| (4, 3, 50) | 1 | not drawn (out of range) | terminal, value 0 |
| (2, 5, 4) | 4 | — | terminal, touchdown, value 7 (new yardline 0) |
| (1, 10, 95) | −8 | — | (2, 18, 103): above 99, no safety (D-9) |
| (1, 10, 8) | 3 | — | (2, 7, 5) |

**W-3: yards rounding and clipping.**

- `26·net + noise` = 2.5 → 2 (half to even); 3.5 → 4; −8.6 → −9 → clipped to −8; 61.2 → 61 → 60.

**W-4: planted team truth on a toy world.**

- Setup: two teams A and B and one play each way.
  - A's offense on the field sums to 0.30 and B's defense on the field to 0.10.
  - B's offense sums to 0.20 and A's defense to 0.25.
- Results:
  - Legacy-style starter means with identical inputs give `T_A = 0.30 − 0.25 = 0.05` and
    `T_B = 0.20 − 0.10 = 0.10`.
  - Net (§4.10 G-2): `N*_A = 0.30 + 0.25 = 0.55` and `N*_B = 0.20 + 0.10 = 0.30`.
  - The expected yards margins: A gains `4 + 26·(0.30 − 0.10) = 9.2` and allows
    `4 + 26·(0.20 − 0.25) = 2.7`, so A is the stronger team, as `N*` says and `T` does not.

**W-5: defender-team share.** On every Rust-native world, every `def_players` id belongs to
`def_team` (share 1.0 exactly) and no id belongs to both sides.

---

## 11. Superseded-spec mapping

Sections cited as "(superseded)" live in `docs/00-meta/specs/superseded/`.

| Superseded section | Requirement | How this spec satisfies it | Gap |
|---|---|---|---|
| alpha-spec §12.2 | "Fixed synthetic football datasets must produce known or tolerance-bounded: team volume projections, player shares, stat-line means, quantiles, fantasy scoring, PB-MAE, Brier and calibration metrics, model promotion decisions" | §4.12 stat-vector world and its truth bundle | The current world covers none of these lists. The §4.12 revision comes before P1-07 |
| alpha-spec §9.2 | "synthetic football fixtures … before dependent implementation begins" | §10.1: loader in P1-01, committed worlds in P1-05, corrected generator in P1-12 | — |
| alpha-spec §6.6 rules 2–3 | A failing test before a numerical fix; goldens never regenerated without an approved model-spec change | §8.4 step 1 (the failing test and semantic note per ledger entry); §10.3 fixture-injected mode | — |
| alpha-spec §6.6 rule 5 | Seeded randomness; parallelism must not change results | §5.2, §6 stream isolation, S-7 | — |
| alpha-spec §2.4, §4.5, §12.3 | Three-season rule; as-of reconstruction; leakage tests (including "current-season participation unavailable at serve time") | §4.12 (several seasons, publication timestamps), §6 | The current world is one season |
| alpha-spec §6.1 | Layers A–F | §4.12 maps each layer to planted truth | Parameters not yet defined |
| final-build-spec §19.2 | Golden outputs on fixed synthetic datasets; CI fails on silent shifts | §7.2, §10.3 S-1, S-2, S-6 | Re-baselined on the corrected world (DR-B1) |
| final-build-spec §11.1–§11.8 | Statistical primitives | Validated through this world's recovery gates (§7) | — |
