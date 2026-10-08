---
contract: grid.plays
contract-version: 0 (legacy, as implemented by the oracle) and 1 (Rust target)
status: Draft            # Draft | Approved | Superseded
authority-level: 3       # engine-spec.md §1.5
semantics-owner: Statistical owner
provider-mapping-owner: Data/Licensing owner
work-packages: P1-01 (domain types), P1-03 (nflverse adapter), P1-05 (as-of views), P1-12 (GRID port)
---

# Contract — GRID engine input frames (`plays`, `players`, `market`, `college`)

This is the data contract every GRID engine stage consumes: the "swap point" between the
synthetic world and real football. It covers four frames. **`plays`** has one row per modelled
snap. **`players`** is the player universe. **`market`** is the team-level anchor.
**`college`** holds feeder-league seasons.

Two versions are specified:

- **v0 (legacy)** is the contract *exactly as implemented* by the Python reference oracle at
  `cautious-nevermore@59bce1d`, imported under `reference/python/` (ADR-012). The code is the
  source of truth for v0. Every statement below cites it as
  `reference/python/<path>:<line>`; line numbers equal `cautious-nevermore@59bce1d`. Parity
  fixtures exported from the oracle carry v0 frames
  ([parity-fixture-contract.md](parity-fixture-contract.md)).
- **v1 (Rust target, Draft)** keeps the v0 semantics. It adds the season, as-of and
  publication-lag metadata that v0 lacks (§9). It also closes the contract defects in §8, or
  makes each one an explicit typed failure. P1-01 and P1-03 implement v1 (§11).

Where this document says the oracle *does* something, that was verified against code or by
running it. The runs used a scratch copy of the oracle, threads pinned to 1, the canonical
synthetic world (`load_synthetic()`), and the real 2023 nflverse inputs listed in §6.3. Where
it says v1 *MUST* do something, that is a requirement on the Rust port. Requirements that
depend on an open owner decision are tagged *(proposed — DR-xx)*; see
[decision-register.md](../00-meta/decision-register.md).

The **stats-aggregation shape** (`nflverse_loader.PLAYS_CONTRACT_COLS`) is a different
contract and is **not** part of `grid.plays` (§10).

---

## 1. Producers and consumers

| Producer (v0) | Where | Status |
|---|---|---|
| Synthetic generator `simulate()` + `make_college()` via `load_synthetic()` | `reference/python/backend/grid/synth.py:162-282`, `:297-317`; `data_adapters.py:31-35` | The default. Used by every recovery gate and golden master. `load_synthetic()` overrides two `SynthConfig` defaults: `yards_noise_sd=3.2`, `drives_per_team_per_game=12` (`data_adapters.py:32`). The result is the canonical world: 16,825 plays, 288 players, 12 teams, 14 weeks |
| nflverse adapter `build_plays_contract()` / `load_grid_plays()` | `reference/python/backend/grid/nflverse_adapter.py:43-147` | The real-data path. It bypasses the documented swap point: `data_adapters.REAL_LOADERS["pbp"]` still raises `NotImplementedError` (`data_adapters.py:39-46`; KI-NEW-V0c) |
| Real `players` frame | `reference/python/backend/validation/verdict.py:509-530` (from `nflverse_loader.load_rosters`) | Full nflverse roster, all positions, de-duplicated to each player's latest season row, null-team rows dropped |
| Real `market` | none. `data_adapters.load_odds` raises `NotImplementedError` (`data_adapters.py:61-65`) | Layer 3 has never run on real data |
| Real `college` | none. `data_adapters.load_cfbd` raises `NotImplementedError` (`data_adapters.py:68-73`) | Synthetic only; see [cfbd/README.md](../04-providers/cfbd/README.md) and DR-C9 |

| Consumer (v0) | Reads | Where |
|---|---|---|
| Value model V(s), dV | `down, ydstogo, yardline_100, drive_points` (fit); `terminal, terminal_value, n_down, n_ydstogo, n_yardline_100` (dV) | `value.py:87,111-112,151-166` |
| Layer 2 RAPM design | `off_players, def_players, off_team, def_team`, plus the derived `dv`; `players.player_id, players.team` (and `players.position` for `lambda_by_pos` and the WR–CB interaction block) | `layers.py:226-344,362-428` |
| Layer 3 market rows | `market[team]` | `layers.py:400-408`; `validation/backtest.py:101-112`; `pipeline/weekly_update.py:269-276` |
| Layer 1 credit | `down, ydstogo, yardline_100, dv, week, off_players, def_players` | `layers.py:488-587` |
| Situations | `down, ydstogo, yardline_100`; `quarter_seconds_remaining` if present | `situations.py:23-60` |
| Priors | `college.player_id, position, feeder_sv, is_rookie`; ratings `player_id, rating` | `priors.py:96-151` |
| As-of slicing | `week`, `season` if present, `off_players`, `def_players` | `validation/asof.py:41-114` |
| Snapshot manifest | `drive_id`, `off_players`, `def_players` (coverage) | `pipeline/ingest_grid.py:29-39` |

Nothing downstream of the adapter reads `play_id`, `yards` or `points`. They are carried for
traceability and for the generator's own bookkeeping.

---

## 2. Frame `plays`

One row per **modelled** play: a pass or run with a non-null down. Kickoffs, punts, field-goal
attempts, extra points, two-point tries, kneels, spikes and no-plays are not rows
(`nflverse_adapter.py:91-92`). Consumers MUST access columns by name. Column *order* is not part
of the contract: the synthetic and adapter frames order the columns differently.

### 2.1 Columns

"Synth" is `simulate()` output and "real" is `build_plays_contract()` output; both dtypes were
observed by running the code. The v1 type is the Rust canonical type (§11).

| Column | v0 dtype synth / real | v1 type | Units and domain | Null | Semantics | Producer | Consumers |
|---|---|---|---|---|---|---|---|
| `play_id` | int64 / float64 (integral) | `u32` inside `PlayKey { game_id, play_id }` | Id. Synth: 0..N−1, globally unique, in generation order. Real: the nflverse `play_id`, unique **within a game only** (2023: not globally unique, unique per `game_id`) | never | Play identity | `synth.py:241-251`; `nflverse_adapter.py:128` | participation join only (`nflverse_adapter.py:195-196`) |
| `drive_id` | int64 / str | `DriveKey { game_id, drive_seq: u16 }` | Synth: 1..2016, globally unique. Real: `f"{game_id}_{int(fixed_drive)}"`, e.g. `2023_01_ARI_WAS_1` | never | Drive identity; it scopes next-state and `drive_points` | `synth.py:187`; `nflverse_adapter.py:97-99` | next-state grouping; `ingest_grid.py:36` |
| `week` | int64 / int64 | `u8` | Synth 1..14. Real 1..22: **postseason weeks 19–22 are included** (KI-NEW-V0a) | never | Week index within the season | `synth.py:242`; `nflverse_adapter.py:130` | Layer 1 weekly aggregation (`layers.py:535,584`); `AsOf` slices |
| `off_team` / `def_team` | int64 / str | `TeamKey` | Synth 0..11. Real: nflverse team abbreviations (32 in 2023) | never | Possession team and defending team | `synth.py:182,243`; `nflverse_adapter.py:131-132` | `build_design` team columns (`layers.py:309-314`); a value missing from `players.team` raises `KeyError` |
| `down` | int64 / int64 | `u8` | 1..4 | never (real rows with null down are dropped) | State s | `synth.py:217`; `nflverse_adapter.py:101` | V(s); situations |
| `ydstogo` | int64 / int64 | `u8` | ≥ 1. Synth 1..24; real 1..40 | never | State s: yards to a first down | `synth.py:217`; `nflverse_adapter.py:102` | V(s); situations |
| `yardline_100` | int64 / float64 (integral) | `u8` | Yards to the opponent goal line. Real 1..99. **Synth 1..104** (§8, D-9) | never | State s | `synth.py:188,239`; `nflverse_adapter.py:103` | V(s); `red_zone` |
| `yards` | float64 (integral) / float64 (integral) | `i16` | Yards gained on the play. Synth clipped to [−8, 60] (observed −8..23); real `yards_gained` (2023: −24..92) | never | Play outcome | `synth.py:214-215`; `nflverse_adapter.py:136` | none downstream |
| `points` | float64 / float64 | derived from `next` (§11) | {0, 3, 7} | never | `drive_points` on the terminal row, 0.0 elsewhere | `synth.py:221-237`; `nflverse_adapter.py:121` | none downstream |
| `terminal` | bool / bool | derived from `next` | — | never | True on the **last modelled row of the drive** and only there | `synth.py:222-260`; `nflverse_adapter.py:114` | `compute_dv` (`value.py:157`) |
| `terminal_value` | float64 / float64 | derived from `next` | {0, 3, 7} on terminal rows | **NaN on every non-terminal row** | Realized value of the absorbing state s′ | `synth.py:220-237,259`; `nflverse_adapter.py:120` | `compute_dv` reads terminal rows only (`value.py:159`) |
| `n_down`, `n_ydstogo` | int64 / int64 | `NextState::Continue(PlayState)` | as `down`, `ydstogo`; **−1 sentinel on terminal rows** | never (sentinel instead) | State s′ = the next modelled row of the same drive | `synth.py:262-270`; `nflverse_adapter.py:110-117` | `compute_dv` non-terminal rows (`value.py:161-164`) |
| `n_yardline_100` | int64 / float64 | as above | as `yardline_100`; −1 on terminal rows | never | as above | as above | as above |
| `drive_points` | float64 / float64 | `DrivePoints` enum | {0, 3, 7}: Touchdown 7, Field goal 3, everything else 0 | never | The drive's eventual points, broadcast to every row of the drive. **This is the V(s) training label** | `synth.py:191,222-237,264`; `nflverse_adapter.py:36,106` | `fit_value_model` (`value.py:112`) |
| `off_players` | tuple of int / tuple of str | `Participation` (§11) | Player ids. Synth: on-field QB, RB, WR, WR, TE. Real: participation `offense_players` **filtered to `{QB, RB, WR, TE, FB}`** through the roster position map (`nflverse_adapter.py:40,198-203`). OL never appears: the offense intercept absorbs it | `()` when participation is absent or the row is unmatched | On-field offensive estimands | `synth.py:246`; `nflverse_adapter.py:123-125,175-214` | `build_design`; Layer 1 masks; `AsOf.slice_pool`; coverage stats |
| `def_players` | tuple of int / tuple of str | `Participation` | Synth: 7 ids — **drawn from the offense's own roster** (KI-NEW-Y0). Real: participation `defense_players`, kept as listed (2023: 11 ids on 35,451 of 35,474 rows) | `()` as above | On-field defenders | `synth.py:123-128,193,247`; `nflverse_adapter.py:210` | `build_design` (−1 entries); Layer 1 opponent rating sum (`layers.py:488-494`) |

### 2.2 Derived stage columns (not source columns)

These columns are attached by engine stages. They are not part of what a producer must emit.

| Column | Attached by | Meaning |
|---|---|---|
| `dv` | `value.attach_dv` (`value.py:169-172`) | dV = V(s′) − V(s), offense perspective, expected-points units. `build_design` reads it as the response y (`layers.py:327`), and raises a bare `KeyError` when it is absent (KI-G12) |
| `season` | `verdict._load_snapshot` (`verdict.py:498-500`) | The season tag added when per-season snapshots are concatenated. **Not emitted by either producer.** `AsOf` switches to the season-aware tuple cutoff only when this column exists (`asof.py:41-55`). Without it the cutoff is week-only |
| `_resid` | Layer 1 (`layers.py:531,576`) | The out-of-fold context residual. Internal; it is the Layer-1 injection point in parity fixtures |

---

## 3. Construction semantics

### 3.1 Synthetic producer (`synth.simulate`)

The [synthetic-world model spec](../05-model-specs/synthetic-world.md) owns the planted truth.
For the contract, the generator's behaviour is:

- **Schedule.** A random pairing each week: n_teams/2 games, each team on offense once per game
  (`synth.py:151-159,181-182`). There is no bye.
- **Drives.** Each drive starts at `yardline_100 ~ clip(N(75, 8), 60, 95)`, 1st and 10
  (`:188-189`). Each play draws an on-field unit with rotation (`_pick_onfield`, `:107-128`).
  Yards = `26·(off ability − def ability) + N(4, yards_noise_sd)`, rounded and clipped to
  [−8, 60] (`:214-215`).
- **Terminal logic** (`:222-237`). New yardline ≤ 0 → touchdown, value 7. Otherwise a first down
  resets to 1st and 10. Failing on 4th down → field goal (value 3) with probability 0.82 if inside
  `fg_range_yardline=35`, else value 0. A drive that reaches `max_plays_per_drive=12` without
  ending is marked terminal on its last play with value 0 (`:256-260`). There are no punts,
  turnovers, safeties or penalties.
- **Next state** is attached after the drive ends (`:262-271`).
- **Injury substitution.** While the focus QB is injured (weeks 8–9, team 0), the backup QB is
  prepended to `off_players` (`:204-210`).

### 3.2 nflverse producer (`nflverse_adapter.build_plays_contract`)

1. **Filter** to `play_type ∈ {pass, run}` and non-null `down` (`:91-92`). There is **no
   `season_type` filter** (KI-NEW-V0a).
2. **Drive segmentation** (`:97-99`). Rows are sorted by `(game_id, fixed_drive, play_id)` and
   `drive_id = game_id + "_" + fixed_drive`. Within-drive chronology is the `play_id` order.
3. **Label** (`:36,106`). `drive_points = {Touchdown: 7, Field goal: 3}[fixed_drive_result]`.
   Every other value maps to 0 through `fillna(0.0)`. In the 2023 pbp (all play types) that
   includes Punt, Turnover, Turnover on downs, End of half, Missed field goal, Opp touchdown and
   Safety. **An unknown future vocabulary value would also silently become 0.**
4. **Next state** (`:110-118`). A within-drive `shift(-1)` of `(down, ydstogo, yardline_100)`.
   A row with no successor is terminal and gets −1 sentinels. The successor is the next
   *modelled* row, so a penalty, kneel or spike in between is skipped over.
5. **Terminal value and points** (`:120-121`). `terminal_value = drive_points` on terminal rows
   and NaN elsewhere; `points = drive_points` on terminal rows and 0 elsewhere. The terminal row
   is the last pass or run of the drive. A drive that ends in a field-goal attempt therefore
   credits its value to the last offensive snap before the kick.
6. **Participation join** (`:175-214`). The join keys are `(game_id: str, play_id: int64)`,
   normalized on **both** sides, because pbp `play_id` is float and a dtype mismatch once dropped
   every row. Cells are split on `;`. `None`, NaN and `pd.NA` become `()` (`:159-172`).
   Offensive ids are kept only if the roster maps them to `{QB, RB, WR, TE, FB}`. Defensive ids
   are kept as listed. Unmatched rows get `()`. A duplicate `(game_id, play_id)` key in the
   participation frame would raise an ambiguous-truth error (KI-NEW-V0d).
7. **Roster map** (`:56-60`; `nflverse_loader.py:110-128`). nflverse `gsis_id` becomes
   `player_id`, the same id space as participation. The map is `dict(zip(...))` over all roster
   rows, so a player who appears more than once keeps his last listed position.

---

## 4. Frame `players`

| Column | v0 synth / real | v1 type | Semantics | Notes |
|---|---|---|---|---|
| `player_id` | int64 0..287 / str (GSIS, e.g. `00-0033873`) | `PlayerKey` | Universe key. **Canonical design-column order is ascending by the string form** (`sorted(..., key=str)`, `layers.py:252`), so synthetic ids order as `"0", "1", "10", "100", …` | Unique. Every participant id SHOULD be present. Unknown ids are dropped with a warning only (`layers.py:280-294`); on real data the first run dropped about 22k participants per origin (PR #89) |
| `team` | int64 / str | `TeamKey` | Team columns are `sorted(players.team.unique())` (`layers.py:255`). That is **native** order: numeric for synthetic integers, lexicographic for abbreviations | Teams present in `players` but absent from `plays` still get (ridge-only) intercept columns |
| `position` | str {QB, RB, WR, TE, DEF; CB when `cb_split`} / nflverse roster position | `Position` (v1 vocabulary open: §8 D-12) | Used by `lambda_by_pos` (`layers.py:381-386`) and the WR–CB proxy (`layers.py:167-173`) | Synthetic defenders are one undifferentiated `DEF` label |
| `is_starter` | bool / **absent** | not in v1 `PlayersFrame` | Synthetic bookkeeping only | Optional in v0 (`layers.py:417-418`) |
| `ability` | float64 / **absent** | moved to `SyntheticTruth` | Planted truth | MUST NOT reach an estimator. In v0 only `fit(verbose=True)`, validation and the `__main__` blocks read it (`layers.py:607`) |

The real frame is built from **all seasons' rosters, de-duplicated to each player's latest row**
(`verdict.py:525-529`). That universe can include players signed after an origin week. v1
builds the universe **as of** the projection time (§9; player-universe leakage per
engine-spec §4.5).

## 5. `market`

| Aspect | v0 |
|---|---|
| Shape | `{team -> float}`. `AsOf.slice_market` also accepts a per-week DataFrame filtered on `week` (and `season`) (`asof.py:116-127`); its other column names are unspecified |
| Synthetic producer | Callers build it as `gt["team_strength"][t] + N(0, 0.01)` with `default_rng(1)` (`tests/grid/golden_master.py:69-70`; `layers.py:619-620`). `load_synthetic()` does **not** return a market |
| Units and estimand | Legacy planted "team strength" = mean starter offense ability − mean starter DEF ability (`synth.py:285-294`). The Layer-3 pseudo-row is `[+1 on t_off, +1 on t_def]` (`layers.py:403-405`), which constrains β_off + β_def, while `team_rating = β_off − β_def` (`layers.py:426`). The three are mutually inconsistent (KI-G1, KI-NEW-A1) |
| Missing team | **Three different behaviours:** `run_rapm` raises `KeyError` (`layers.py:406`); `backtest.solve_rapm` skips the team (`backtest.py:104-106`); `weekly_update` anchors the team to **0.0** (`weekly_update.py:274`), a false zero prior |
| Real producer | none |

**v1 (proposed — DR-B5).** The market anchors **net** team strength (offense plus defense
quality). It uses the quote **as published at or before the projection lock**. That is the
market-line lock rule of engine-spec §4.3: a closing line is post-lock for any game after the
lock. A team with no eligible quote gets **no** anchor row. A 0.0 default is forbidden
(superseded alpha-spec Appendix D #8; root `CLAUDE.md` "Prohibited"). The quote record carries
`season`, `week`, `team`, `strength` (with declared units), `quoted_at` and `source`. No odds
provider is selected. Mapping a spread or total to the intercept scale is a model-spec item in
[rapm-attribution.md](../05-model-specs/rapm-attribution.md).

## 6. `college`

| Column | v0 (synthetic only) | v1 | Semantics |
|---|---|---|---|
| `player_id` | int64 | `PlayerKey` | Joins to NFL ratings |
| `position` | str | `Position` | Selects `PRIOR_SD[position]` (`priors.py:26,148`) |
| `feeder_sv` | float64 | `f64` | Synthetic: `league_factor·ability + N(0, college_noise_sd)` with `league_factor=0.62`, `college_noise_sd=0.030` (`synth.py:312`). Real: a feeder-league situational value from a CFBD pass; **no producer exists** |
| `feeder_snaps` | int64 in [250, 900) | `u32` (optional) | **Produced but never read** by any consumer (verified by search) |
| `is_rookie` | bool (105 of 288 in the canonical world) | `bool` | Rookies are excluded from the equivalency fit and receive priors (`priors.py:98`) |

Synthetic `make_college()` gives **every** player a feeder season, using RNG seed
`cfg.seed + 99` (`synth.py:308`). `build_priors` optionally reads caller-named `age` and
`draft_round` columns (`priors.py:129-147`), where draft round 0 means undrafted and an absent
value means unknown. Those step functions are not ported as-is (proposed — DR-C9).

### 6.1 Synthetic truth bundle (`gt`)

`simulate()` also returns the planted truth (`synth.py:274-281`). It is **not** part of the input
contract; it exists only for validation:

- `ability {player_id → float}`;
- `team_strength {team → float}` (legacy off − def definition);
- `focus_qb` (int);
- `focus_tau` (float[weeks], NaN = missed game);
- `league_factor` (0.62);
- `players` (the players frame).

The [synthetic-world spec](../05-model-specs/synthetic-world.md) defines the corrected planted
net strength (proposed — DR-B4, DR-B5).

### 6.2 Invariants verified on the canonical synthetic world (v0)

- 16,825 plays; 2,016 drives; one terminal row per drive, always the last.
- Terminal values: 7 → 417 drives, 3 → 62, 0 → 1,537.
- `drive_points` is constant within each drive. `points == terminal_value == drive_points` on
  terminal rows. `terminal_value` is NaN on every non-terminal row. Every terminal row carries
  the `(−1, −1, −1)` sentinel.
- `off_team ≠ def_team` on every row. `def_players` has 7 ids on every row.
- Violations of real-football invariants (§8, D-8 to D-10):
  - `off_players` contains a **duplicated id on 245 plays**:
    - 215 WR (both WR slots drew the same backup);
    - 30 QB (the injured-focus substitution prepends the backup QB when he was already on the
      field; that gives a 6-entry tuple).
  - `yardline_100` reaches **104** (10 rows above 99).
  - `ydstogo > yardline_100` on 361 rows.
  - Every `def_players` id belongs to `off_team` (KI-NEW-Y0).

### 6.3 Invariants verified on real 2023 nflverse data (v0 adapter)

Inputs, by sha256:

| File | sha256 |
|---|---|
| `play_by_play_2023` | `bd348473…6776` |
| `pbp_participation_2023` | `b1577369…e5a6` |
| `roster_2023` | `66dcb7d0…3c90` |

The investigation scripts are in `reference/python/tools/investigations/`; the parquet files are
not committed (DR-A11).

- 35,474 plays and 6,085 drives. 1,638 rows are postseason.
- `yardline_100` is in 1..99 and integral. `ydstogo ≤ yardline_100` on every row.
- `drive_points` counts: 0 → 17,833 rows, 7 → 10,007, 3 → 7,634.
- No duplicate ids within a side; no duplicate `(drive_id, play_id)`.
- Participation is populated on 100% of rows. Offense tuple sizes:
  - 6 ids on 34,475 rows;
  - 1–5 ids on the remaining 999 rows.
- **One drive spans a change of possession:** `2023_12_PIT_CIN`, `fixed_drive` 8 holds 3 CIN
  plays followed by 5 PIT plays (§8, D-4).

---

## 7. Validation rules (v1)

A v1 frame is validated when it is constructed, and every violation is a typed `ContractError`
(§11). Unknown or malformed critical data MUST NOT be coerced to a default (superseded
alpha-spec Appendix D #8).

**Core rules (every producer):**

1. Rows within a drive are chronological. `next` is the following row of the same drive, or
   `Terminal`.
2. Exactly one terminal row per drive, and it is the last row.
3. `drive_points` is constant within a drive and in the v1 vocabulary {0, 3, 7} *(proposed —
   DR-C13)*. On the terminal row, `points` and `terminal_value` equal it.
4. `off_team ≠ def_team`. `off_team` and `def_team` are constant within a drive.
5. `down ∈ 1..=4`, `ydstogo ≥ 1`, `yardline_100 ≥ 1`.
6. Every `off_team`/`def_team` value is in `players.team`. Every participant id is in
   `players.player_id`, or the frame fails with `UnknownParticipant { count }`. Dropping unknown
   ids is allowed only under an explicit, logged run policy *(proposed — DR-B6)*.
7. Keys are unique: `player_id` within `players`; `(game_id, play_id)` within `plays`.

**Real-data profile (nflverse and the corrected synthetic world):**

1. `yardline_100 ≤ 99` and `ydstogo ≤ yardline_100`.
2. Ids are distinct within a side.
3. Offensive ids belong to `off_team`, and defensive ids to `def_team`, on the as-of roster for
   that week.
4. For any season that feeds RAPM, participation is populated on ≥ 99% of rows. This is the
   cautious-nevermore Phase-1 coverage gate; 2022–2023 measured 100%.

The **legacy synthetic profile** (v0 fixtures) is exempt from real-data rules 1–3 and is flagged
`profile = synthetic-legacy` in fixture metadata. The corrected generator SHOULD satisfy them
(proposed — DR-B4).

---

## 8. Known contract defects (v0) and v1 disposition

| # | Defect (v0) | Evidence | v1 disposition |
|---|---|---|---|
| D-1 | **`two_minute` depends on a column the contract never carries.** `situations.py:23,57-60` reads `quarter_seconds_remaining`, which neither producer emits, so the mask is always absent and only a warning is logged. The definition also ignores the quarter, so it would include the ends of Q1 and Q3 (KI-NEW-A4). On real 2023 pass/run rows, `quarter_seconds_remaining ≤ 120` marks 6,227 rows and `half_seconds_remaining ≤ 120` marks 4,111 | critic.md C-13; KI-NEW-A4 | The adapter MUST emit `qtr` and `half_seconds_remaining` (`GameClock`). `two_minute := half_seconds_remaining ≤ 120` *(proposed — DR-C13)*. Synthetic frames have no clock, so `two_minute` is reported as `Unavailable`, never silently omitted. The change from the oracle's definition is a `PARITY.md` divergence |
| D-2 | No `season_type`, so postseason rows enter V(s), RAPM and the walk-forward | KI-NEW-V0a (1,638 rows in 2023) | `season_type` is required. Engine policy is regular season only for training labels, and the plays-side policy is stated in the model specs *(proposed — DR-C12)* |
| D-3 (KI-NEW-Z46) | 7/3/0 vocabulary: defensive scores and safeties count 0; an unknown `fixed_drive_result` becomes 0 | KI-NEW-V0b; `nflverse_adapter.py:106` | Keep 7/3/0 for v1 *(proposed — DR-C13)*. Map `fixed_drive_result` through a **closed** table in the nflverse normalization map; an unknown value is a typed `SchemaDrift` error. A signed-EP label would be a v2 major change |
| D-4 (KI-NEW-Z47) | A `fixed_drive` can span a change of possession; the adapter chains next-state across offenses and broadcasts one result to both | §6.3 (2023, one drive). **Not in the inventories; reported as new** | MUST NOT chain next-state across an `off_team` change. Such a drive is quarantined (excluded from V(s) and RAPM and listed in the data-quality report) unless the nflverse normalization map defines a reliable split *(proposed — P1-03 decision)* |
| D-5 (KI-NEW-Z48) | "Participation unavailable" is indistinguishable from "empty": absent participation or an unmatched row yields `()`, and RAPM then fits a team-intercept-only design without error. `run_situation_rapm` checks only that the columns exist (`layers.py:469-473`) | `nflverse_adapter.py:183-186,211-213` | `Participation::{Listed, Unmatched, NotAvailable}` plus frame-level availability (§9). Participation-dependent stages reject frames whose participation is not available as of the lock (DR-C1) |
| D-6 | Duplicate participation keys raise an ambiguous-truth error | KI-NEW-V0d | Typed `DuplicateKey` error |
| D-7 | The swap point is stale: `REAL_LOADERS["pbp"]` raises while the real path is `load_grid_plays`; odds and CFBD are stubs | KI-NEW-V0c | One source trait (`PlaysSource`) with synthetic and nflverse implementations. Absent sources are typed `SourceUnavailable`, not stubs |
| D-8 (KI-NEW-Z38) | The synthetic generator emits duplicated ids in `off_players` (245 plays). The planted yards count the player's ability twice, and `build_design` sums the duplicate COO entries into a **+2** design value | §6.2. **New** | Rust parity on legacy fixtures MUST treat participation as a **multiset** (sum duplicates) to reproduce v0. The real-data profile forbids duplicates. The corrected generator SHOULD not produce them (DR-B4) |
| D-9 (KI-NEW-Z39) | The synthetic state space leaves football: `yardline_100` up to 104, `ydstogo > yardline_100`, no safeties | §6.2. **New** | V(s) state support differs between synthetic and real data; [value-model.md](../05-model-specs/value-model.md) states the support. The legacy profile is exempt from validation |
| D-10 | Synthetic defenders are drawn from the offense's own team | KI-NEW-Y0 (critic.md G-1) | Fixed in the corrected synthetic world (correction #1 in `reference/python/PARITY.md`, DR-B1). Real-data rule 3 enforces the invariant |
| D-11 | `market` units, estimand and missing-team behaviour are inconsistent (§5) | KI-G1, KI-NEW-A1; reconcile-code-first.md §3 | §5 v1 rules *(proposed — DR-B5)* |
| D-12 (KI-NEW-Z49) | Producer dtypes drift: `play_id`/`yardline_100` are float in real data and int in synthetic; ids are str in real and int in synthetic; `drive_id` is str vs int. Position vocabularies differ (`DEF` vs nflverse positions). After a Parquet round-trip participation cells become arrays, which breaks `AsOf.slice_pool` | KI-V8; §2.1 | v1 canonical types (§11). Position enum and synthetic↔real mapping are owned by [rapm-attribution.md](../05-model-specs/rapm-attribution.md) (open) |
| D-13 (KI-NEW-Z40) | Truth columns (`ability`, `is_starter`) live in the input frame | §4 | `SyntheticTruth` is a separate type that estimator code cannot reach |
| D-14 | V(s) is fitted on the whole season's plays in `weekly_update` | KI-NEW-W4 (`weekly_update.py:210-214`) | Consumers fit only on the as-of view (§9) |
| D-15 | Cache and snapshot writes are non-atomic, and cache keys collide after sanitizing | KI-G9, KI-V10 | Artifact persistence is content-addressed and written by temp file plus rename (engine-spec §8; DR-C15) |

---

## 9. As-of and publication-lag metadata (v1 additions)

v0 has no notion of *when* a row became knowable. `AsOf` filters on `(season, week)` only
(`asof.py:41-55`), and only when a `season` column happens to exist. The Python walk-forward
therefore folds current-season FTN participation into RAPM at every origin. That participation
is published **only after the postseason**
([nflverse/README.md](../04-providers/nflverse/README.md)), so this is a train/serve skew.
v1 adds the following *(proposed — DR-C1)*.

**Row-level (required):**

- `season: u16`
- `season_type: SeasonType`
- `game_id: GameId` (explicit; v0 embeds it in `drive_id`)
- `clock: Option<GameClock { qtr: u8, half_seconds_remaining: u16 }>`
- `participation: Participation` (§11)

**Frame-level `PlaysFrameMeta` (required):**

- `contract: ContractVersion`.
- `producer`:
  - `Synthetic { generator_version, config_hash, seeds, profile }`, or
  - `Nflverse { adapter_version, normalization_map_version }`.
- `sources: Vec<SourceSnapshot>`, one per dataset partition (pbp, participation, roster) and
  season. Each records the freshness fields of superseded alpha-spec §4.1.1 and the
  timestamps of engine-spec §4.5:
  - `source_name`, `source_version`;
  - `source_timestamp` (publication), `retrieved_at`, `available_at`;
  - `ingestion_run_id`, content `sha256`, `row_count`, `schema_version`, `validation_status`.
- `participation_availability: Vec<(season, ParticipationStatus)>`:
  - `Published { at, provenance: Ngs | Ftn }`, or
  - `NotPublished`, or
  - `Synthetic`.
- Coverage statistics: off/def coverage per season, as written by `ingest_grid.py:29-39`.

**As-of view.** `AsOf { season, week, lock_at }` replaces the week-index-only cutoff. A view
exposes a row's outcome columns only if both hold:

- `(season, week) < (AsOf.season, AsOf.week)` (the season-aware tuple rule of `asof.py`);
- the row's partition was available at `lock_at`: `available_at ≤ lock_at`.

The same view exposes participation for season *s* only if
`participation_availability(s) = Published { at ≤ lock_at }`. In-season participation is
therefore unavailable at every in-season lock. The live path uses the participation-free
Layer-1′ signal *(proposed — DR-C1; engine-spec §6.3)*.

Pre-game inputs (market quotes, interventions, availability) are admitted by their own
`quoted_at`/`observed_at ≤ lock_at`. A record with no timestamp is a typed error, as the oracle
already raises for undated interventions.

---

## 10. Not this contract: the stats-aggregation shape

`nflverse_loader.PLAYS_CONTRACT_COLS` (`nflverse_loader.py:18-26`) has the columns
`game_id, play_id, week, off_team, def_team, down, ydstogo, yardline_100, yards_gained,
pass_attempt, rush_attempt, passer_id, receiver_id, rusher_id, air_yards, yards_after_catch,
td_type, fumble, interception, td_player_id, complete_pass`. Despite its name it is a
**box-score aggregation** shape, produced by `_normalize_pbp` (`:47-82`). Its only consumer is
`data_pipeline._aggregate_stats_from_pbp`, which writes the SQLite `player_stats` table.

- It drops the drive columns the GRID contract needs (`nflverse_loader.py:50-51`).
- It carries the confirmed ingest biases KI-NEW-I1 to KI-NEW-I5: sacks counted as attempts;
  sack yards netted from passing yards; return touchdowns credited to the offense; fumbles
  mis-attributed; postseason summed in; two-point conversions never populated.
- The cautious-nevermore backend spec presented these as "extended fields" of the engine
  contract (cn-docs.md §2.2). They are not.

v1 does **not** port this shape. Training labels come from official nflverse weekly player
statistics, regular-season weeks only, with official stat definitions *(proposed — DR-C12;
engine-spec §4.7)*. PBP-derived aggregates may appear only as features or cross-checks, under
their own contract (`grid.stat_events`, not yet written). They never extend `grid.plays`.

---

## 11. Rust implementation (v1)

The types below are illustrative; P1-01 fixes the names. `grid-domain` (`crates/domain`) has
**no external dependencies** (docs/CLAUDE.md), so timestamps are a domain newtype and
serialization lives outside the domain crate.

```rust
// grid-domain (P1-01)
pub struct ContractVersion { pub major: u16, pub minor: u16 }

pub enum PlayerKey { Gsis(String), Synthetic(u32) }   // Ord = by string form (layers.py:252)
pub enum TeamKey { Nfl(String), Synthetic(u16) }      // Ord = native: numeric / lexicographic (layers.py:255)
pub struct GameId(pub String);
pub struct UtcInstant(pub i64);                       // seconds since the Unix epoch, UTC
pub enum SeasonType { Regular, Post }

pub struct PlayState { pub down: u8, pub ydstogo: u8, pub yardline_100: u8 }
pub enum DrivePoints { Zero, FieldGoal, Touchdown }   // 0 / 3 / 7 (proposed — DR-C13)
pub enum NextState { Continue(PlayState), Terminal }  // replaces terminal, n_*, the -1 sentinels
pub struct GameClock { pub qtr: u8, pub half_seconds_remaining: u16 }
pub enum Participation {
    Listed { offense: Vec<PlayerKey>, defense: Vec<PlayerKey> }, // order kept; multiset for legacy synth
    Unmatched,      // participation published, but no row for this play
    NotAvailable,   // not published as of the lock, or never produced
}
pub struct PlayRecord {
    pub game_id: GameId, pub play_id: u32, pub drive_seq: u16,
    pub season: u16, pub season_type: SeasonType, pub week: u8,
    pub off_team: TeamKey, pub def_team: TeamKey,
    pub state: PlayState, pub clock: Option<GameClock>,
    pub yards: i16, pub next: NextState, pub drive_points: DrivePoints,
    pub participation: Participation,
}
pub struct PlaysFrame { pub meta: PlaysFrameMeta, pub plays: Vec<PlayRecord> }
// PlayersFrame, MarketQuote, CollegeFrame and SyntheticTruth follow §§4-6.
// ContractError: UnsupportedVersion, MissingField, InvalidValue, NonTerminalLastRow,
//   MultipleTerminals, DrivePointsNotConstant, PossessionChangeWithinDrive, UnknownTeam,
//   UnknownParticipant, DuplicateParticipant, DuplicateKey, SchemaDrift,
//   ParticipationUnavailable, ClockUnavailable, MissingTimestamp, SourceUnavailable.
```

The v0 columns `terminal`, `terminal_value`, `points` and `n_*` are **derived** from
`next` + `drive_points`. A columnar v0 view, used by the parity harness and exports, reproduces
them exactly, NaN and −1 sentinels included.

- **P1-03** implements the nflverse producer in `grid-ingestion`. It follows §3.2 with the D-2,
  D-3, D-4 and D-6 fixes, from retained raw partitions (superseded alpha-spec §4.1.1;
  engine-spec §4.5).
- The synthetic producer moves to the `synth` crate once DR-A8 adds it. Until then, the parity
  harness reads oracle-exported v0 frames
  ([parity-fixture-contract.md](parity-fixture-contract.md)).
- A `from_legacy_v0` upgrade maps a v0 synthetic frame to v1. It sets `season` (declared by the
  fixture manifest), `season_type = Regular`, `clock = None`, participation `Listed`, and
  `producer = Synthetic { profile: synthetic-legacy }`. It never guesses for real data.

---

## 12. Versioning and compatibility

- **Contract id `grid.plays`.** v0 is frozen: it describes `cautious-nevermore@59bce1d` and
  changes only if an erratum in this document is found. v1 is Draft until P1-01 is approved.
- **MAJOR bump.** Removing or renaming a field; changing a unit, a vocabulary or a definition
  (signed-EP `drive_points`, the `two_minute` rule, the terminal-row rule); or tightening a rule
  so that previously valid frames are rejected.
- **MINOR bump.** Adding an optional field whose absence has a documented meaning.
- **Reading.** A reader MUST reject a MAJOR version it does not implement
  (`UnsupportedVersion`). It MUST NOT "make the parser flexible" (superseded alpha-spec §4.6).
- **Persisted derivatives.** Snapshots, accumulators, fixtures and model states record the
  contract version they were built from. After a MAJOR change they are **rebuilt from retained
  raw data, never hand-migrated** (cautious-nevermore invariant #6).
- **Process.** A contract change needs:
  - an ADR or an approved work-package decision;
  - an updated version line in this document;
  - fixtures regenerated with a reviewed semantic explanation
    ([parity-fixture-contract.md](parity-fixture-contract.md) §8);
  - synchronized tests.

## 13. Tests that pin this contract

- **Oracle.** `reference/python/tests/grid/test_nflverse_adapter.py` (12 tests: drive id, 7/3/0
  mapping and broadcast, next state, terminal value, no cross-drive bleed, dropped rows,
  participation join and skill filter, missing cells, absent participation, empty input,
  `load_grid_plays`, value-path feed). Also `test_nflverse_loader.py` (9) and
  `test_situations.py` (5).
- **Rust (required by P1-01/P1-03).**
  - Every core and real-data rule in §7 has a passing case and a failing case.
  - Every §8 v1 disposition has a regression test.
  - `from_legacy_v0` round-trips every oracle-exported synthetic v0 fixture exactly.
  - The nflverse adapter reproduces the oracle's 12 adapter cases on synthetic, nflverse-shaped
    toy frames. They are hand-built, like the oracle's tests. Real-data fixtures need a
    Data/Licensing ruling (DR-A11; [access-and-license.md](../04-providers/nflverse/access-and-license.md)).
