# Real-data results of the Python GRID engine (historical, non-parity)

> **HISTORICAL. NON-PARITY. NOT EVIDENCE FOR THIS ENGINE.**
>
> Every number on this page was produced by the cautious-nevermore (CN) Python engine, now the
> reference oracle in `reference/python/`, on real nflverse data. None of them may be cited as evidence
> of this engine's accuracy, used as a parity target, or used to set a gate. The reasons are in §1.
> They are kept so that nobody has to re-derive them, and so that later results can be compared with
> what was measured before. The current statement of the engine's evidence is `engine-spec.md` §3.3,
> and the place of these numbers in validation is §7.14.5.

This page was written for the archive on 2026-10-07 (`docs/07-archive/README.md`). It implements
critic G-7 and records cn-issues §0, §3.6–§3.7 and NEW-A3 / NEW-P4. Both reports are in
`docs/06-sessions/2026-10-01-consolidation-inventory/`.

## 1. Why none of these numbers counts

1. **Biased labels.** CN built its realized fantasy points from its own play-by-play aggregation,
   which is biased against the official nflverse player statistics (§4.1):
   - sacks counted as pass attempts and sack yards netted from passing yards (KI-NEW-I1);
   - return and defensive touchdowns on offensive plays credited to the offence (KI-NEW-I2);
   - fumbles charged to the wrong player, with sack fumbles dropped (KI-NEW-I3);
   - postseason weeks summed into season totals and rates (KI-NEW-I4), and postseason plays left in
     the engine's plays contract (KI-NEW-V0a);
   - two-point conversions never populated and kneels left out of rush attempts (KI-NEW-I5).

   These labels feed the stat-line and volume models, the realized-points history and both
   headline margins.
2. **Confidence intervals that are too narrow.** Every interval below is an **iid percentile
   bootstrap** over correlated cells: player × origin pairs for H1, roster × week cells for H2. The
   cells share weeks and overlapping rest-of-season windows. This engine requires a paired,
   week-clustered bootstrap (KI-NEW-V1; `engine-spec.md` §7.7).
3. **Current-season participation skew.** The walk-forward folded the current season's player
   participation into RAPM at every origin. nflverse publishes a season's participation only after
   its postseason, so no live forecast could have had that input. That is train/serve skew, and this
   engine does not allow it on the live path (proposed — DR-C1; `engine-spec.md` §4.5, §6.3).
   Separately, the H2 forecast map was fitted on pairs from the same warm-up weeks its ratings were
   estimated on (KI-NEW-R3).
4. **A different protocol.** The runs used STANDARD scoring, a margin metric and synthetic rosters.
   This engine uses its own scoring default (`engine-spec.md` §2.3), PB-MAE as the primary metric
   (§7.4) and its own player pool (§7.3).
5. **An incomplete record.** PR #91 added the Kalman `smoothed_talent` feature to the rest-of-season
   model and left the re-run as a developer step. No H1 number with that feature was **ever recorded**,
   in any document or commit, and whether the re-run happened is unknown. `prior_mean` stayed 0.0 in
   every run.

The *direction* of the conclusion is robust. Over all positions, GRID's rest-of-season forecasts beat
only the persistence baseline, not the season-to-date or last-season baselines, and the one passing
headline, H2, rests on the skew of reason 3. GRID has therefore not met the `engine-spec.md` §9.4
model-quality gate. The magnitudes must be
measured again after the ingest corrections and under the participation-publication rule
(critic X-14).

## 2. Provenance

- **Source.** CN's verdict reports (`data/reports/phase2c_verdict.{json,md}`) and the per-run frozen
  gates were **gitignored by design** (PR #88). They were never committed anywhere. The only committed
  transcription is the Phase-2c record, archived here as
  `docs/superpowers/plans/2026-07-09-phase2c-verdict-and-ros-gap.md` (CN `165ccde`). The squash bodies
  of PRs #89–#91 agree with it.
- **Re-verified.** Every verdict number in §3 was checked against that file on 2026-10-07.
- **The real-2023 measurements** in §4 were made by the consolidation inventory on 2026-10-01. The
  scripts are in `reference/python/tools/investigations/`; each docstring carries the recorded output.

## 3. The Phase-2c verdict runs (2022–2023)

### 3.1 Run context

| Item | Value |
|---|---|
| Commands (as recorded) | `python -m backend.pipeline.ingest_grid --years 2022 2023`, then `python -m backend.pipeline.data_pipeline --years 2022 2023`, then `python -m backend.validation.verdict --run-label <label>` |
| Data | nflverse, seasons 2022–2023: 70,778 plays, 100% participation coverage |
| Scoring | STANDARD: receptions 0, otherwise the base rules (`backend/validation/verdict.py:47,563`; `backend/scoring/formats.py`) |
| Warm-up | `warmup_weeks = 4` (`verdict.py:350`); V(s) and the rating→points map frozen on the pre-first-origin window |
| Origins | one per week after the warm-up (`origin_stride = 1`, the CLI default; the recorded commands pass none) |
| H2 rosters | synthetic VOR-greedy snake drafts: 8 teams, 8 rounds, slots QB 1, RB 2, WR 2, TE 1, FLEX 1 (`verdict.py:68,348-349`) |
| Bootstrap | iid percentile, `n = 10,000`, `alpha = 0.05`, `seed = 0` (`verdict.py:355-357`) |
| Gates | each KPI's gate is the mean plus 1.96 standard errors of its own sign-flip null, frozen once per run (PR #87, #88) |

**Metric definitions.**

- **H1 (rest of season).** A cell is one player at one forecast origin. The H1 margin is the mean over
  cells of `|baseline error| − |GRID error|`, in fantasy points per game. The target is the realized
  rest-of-season per-game mean. Positive favours GRID. "Vs last-season" was CN's kill criterion.
- **H2 (weekly lineups).** For each roster and week, both methods fill the same slots with the same
  rule; only the projection differs. The baseline is the last-season weekly average. The margin is
  realized points of GRID's lineup minus realized points of the baseline's. Positive favours GRID. Win
  rate counts ties as one half.

### 3.2 The three recorded runs

| Run label | Recorded | What was wrong with it | H1 vs last season | H2 margin |
|---|---|---|---|---|
| `real-2022-2023-first-run` | 2026-07-09 | Rest of season was scored with the **weekly** SV→points map, not the season stat-line model the design assigns to it; RAPM also ran on a broken player universe | **−0.864** [−0.932, −0.796] | −0.524 [−1.486, +0.461]; win rate 0.478 |
| `real-2022-2023-ros-corrected` | 2026-07-09 | Rest of season scored with the volume × efficiency model, but still on the broken universe: RAPM was fed the skill-position subset and dropped about 22,000 participants per origin as unknown ids | −0.015 (no interval recorded) | not recorded |
| `real-2022-2023-universe-fixed` | 2026-07-09 | Both fixes (PRs #89 and #90). The run CN called "trustworthy" | **−0.015** [−0.059, +0.029] | **+0.848** [+0.232, +1.466]; win rate 0.542 |

First run, as recorded:

- GRID also lost to the season-to-date mean and beat only persistence, failing even that for RB.
- Tier 2 was skipped, because there was no points-allowed feed.
- Calibration: weekly NIS (all positions) 1.00, rest-of-season NIS 1.25, PICP@80 ≈ 0.75–0.87. **These
  calibration figures are stated for the first run only.** No calibration figures were transcribed for
  the later runs.
- The gate registry was reset rather than frozen on this run.

### 3.3 The `real-2022-2023-universe-fixed` run

**H1, all positions (n = 7,035 cells).**

| Baseline | Margin [95% CI, iid] | Reading as recorded |
|---|---|---|
| Persistence (last week) | +0.957 [+0.848, +1.068] | GRID clearly better |
| Season-to-date mean | **−0.194 [−0.270, −0.120]** | GRID slightly worse |
| Last season | −0.015 [−0.059, +0.029] | Tie: the kill criterion was not cleared |

The season-to-date row is the key negative result. CN's consolidated `docs/02-backend-spec.md`
dropped it, and attributed the +0.957 persistence row to TE (cn-docs §14 item 2).

**H1 by position, vs last season.** The source gives an interval for TE only.

| Position | Margin | Reading as recorded |
|---|---|---|
| TE | +0.064 [+0.011, +0.118] | Clears the kill criterion. Also +0.228 vs season-to-date mean, recorded as "PASS" with no interval |
| RB | +0.005 | Tie |
| QB | −0.036 | Tie |
| WR | −0.066 | Marginally worse |

DB and P rows also appear, at 15 and 30 cells. They do not move the all-positions row and were kept
on purpose: the rest-of-season model is position-agnostic so that non-skill rows could later feed a
weekly in-season mode.

**H2.** +0.848 [+0.232, +1.466], win rate 0.542, recorded as a **pass**. No starter-OUT subset figure
was transcribed.

**Per-run frozen gates** (from the report, never in version control):

| Gate | Value | Observed | Recorded outcome |
|---|---|---|---|
| `h1_ros` | +0.0564 | −0.015 | not cleared |
| `h2` | +0.3164 | +0.848 | cleared |

**Verdict.** `BASELINE_FALLBACK`. `compute_valuations` stayed on the last-season baseline.

**CN's interpretation at the time** (recorded, not endorsed): rest of season is volume-dominated,
and RAPM is only one talent feature, so fixing the RAPM universe barely moved H1. H2 rides directly on
weekly RAPM forecasts, so the fix helped it materially.

### 3.4 What was never measured

- H1 with `smoothed_talent` (PR #91): no result was recorded;
- any run with a real `prior_mean`: no real feeder-SV source existed (KI-NEW-P2);
- the market baseline: it was never wired;
- Tier 2 (matchup grade against points allowed): no points-allowed feed, and the grade's sign is
  inverted anyway (KI-NEW-A2);
- calibration of the universe-fixed run, and intervals for most per-position rows (not transcribed).

## 4. Real-2023 measurements from the consolidation inventory (2026-10-01)

All inputs are nflverse 2023 release assets, fetched and sha256-verified by
`reference/python/tools/investigations/fetch_realdata.py`. They are never committed (DR-A11):

| File | sha256 |
|---|---|
| `play_by_play_2023.parquet` | `bd3484731408def6b0ec93225bba2bd7b2c65769ca707a2b9444d891abdc6776` |
| `pbp_participation_2023.parquet` | `b157736972248e6f71e0c5ad6c1010b71375a500a63944dc4956c65de263e5a6` |
| `roster_2023.parquet` | `66dcb7d0e203c41b49e997b93eb55ed77bcc960e5dd0774b7e2426a87cef3c90` |
| `player_stats_2023.parquet` | `94673091ea041bc4bddd7848f813abcf0f9be085540e3f187fb6dd1b1dacf2c9` |

### 4.1 Ingest bias (`cmp_stats.py`)

The oracle's own ingest (`nflverse_loader._normalize_pbp`, then
`data_pipeline._aggregate_stats_from_pbp`) on 2023 play-by-play, weeks ≤ 18, against the official
`player_stats_2023` regular season, inner-joined on player id:

| Stat | Oracle | Official | Difference | Cause |
|---|---|---|---|---|
| Pass attempts | 19,658 | 18,315 | +7.3% | sacks counted (KI-NEW-I1) |
| Passing yards | 119,092 | 128,567 | −7.4% | sack yards netted (KI-NEW-I1) |
| Passing TDs | 814 | 754 | +8% | return TDs credited (KI-NEW-I2) |
| Receiving TDs | 799 | 754 | +6% | return TDs credited (KI-NEW-I2) |
| Rushing TDs | 473 | 470 | +0.6% | KI-NEW-I2 |
| Rush attempts | 14,178 | 14,588 | −2.8% | kneels excluded (KI-NEW-I5) |
| Rushing yards | 61,761 | 61,295 | +0.8% | — |
| Receiving yards | 128,564 | 128,562 | 0.0% | — |
| Fumbles lost | 174 | 256 | −32% | misattribution (KI-NEW-I3) |
| Completions, interceptions, targets, receptions | equal | equal | 0 | — |

Also counted:

- 66 offensive plays with a touchdown that was neither a passing nor a rushing touchdown;
- 1,459 sack rows that carry a passer;
- 1,638 postseason rows left in the normalized frame (KI-NEW-I4).

### 4.2 The real plays contract (`real_contract.py`)

The oracle's adapter on 2023 inputs:

- 35,474 plays, of which **1,638 are postseason** (weeks > 18; KI-NEW-V0a);
- participants missing from the roster: 0;
- offensive skill participants per play: 6 on 34,475 plays and 5 on 913, the rest fewer; 25 plays
  have no QB listed;
- defenders per play: 11 on 35,451 plays, and 9, 10 or 13 on the other 23.

`fixed_drive_result` per play:

| Value | Plays | Drive points |
|---|---|---|
| Punt | 14,862 | 0 |
| Touchdown | 13,188 | 7 |
| Field goal | 10,392 | 3 |
| Turnover | 3,690 | 0 |
| Turnover on downs | 3,263 | 0 |
| End of half | 2,135 | 0 |
| Missed field goal | 1,569 | 0 |
| Opp touchdown | 502 | 0 (KI-NEW-V0b) |
| Safety | 62 | 0 (KI-NEW-V0b) |

### 4.3 RAPM scale and QB identifiability (`real_rapm.py`)

2023 regular season (weeks ≤ 18), full-roster universe, oracle V(s) and `run_rapm`:

- 33,836 plays; dV standard deviation 1.135; 1,602 players took the field.
- Rating standard deviation by position, real against synthetic (legacy generator, KI-NEW-Y0):

  | Position | Real 2023 | Synthetic |
  |---|---|---|
  | QB | 0.0422 | 0.2593 |
  | RB | 0.0441 | 0.0917 |
  | WR | 0.0454 | 0.0811 |
  | TE | 0.0462 | 0.0711 |
  | DB / DL / LB | 0.0411 / 0.0445 / 0.0405 | DEF 0.0794 |

- Team rating SD 0.1038; team-offence intercept SD 0.0718.
- Median offensive snaps per season for skill players with more than 100: 429.
- **QB value is absorbed by the team intercept** (KI-NEW-A3):
  - the highest-rated QBs (rating, snaps) include Matthew Stafford (0.0867, 946), Jake Browning
    (0.0853, 448), Joe Flacco (0.0791, 329) and Aidan O'Connell (0.0559, 618);
  - the lowest include Jalen Hurts (−0.0310, 1,057), Bryce Young (−0.0475, 1,012) and Sam Howell
    (−0.1140, 1,013);
  - the correlation between a starting QB's rating and his own team-offence intercept is 0.13.

  A full-time starter is nearly collinear with the team-offence intercept, whose ridge penalty is
  20 times lighter.
- **Every hand-set scale constant is synthetic-calibrated** (KI-NEW-P4): the prior SDs, the age and
  draft steps, the Kalman initial covariance and the `SSParams` noise values. On the real scale they
  are four to five times too wide.
- The ridge system is well-conditioned: cond(A) = 2.18e3
  (`docs/05-model-specs/rapm-attribution.md` §4.3).

### 4.4 Earlier real-data smoke runs (CN PRs #67 and #68, 2023, manual)

Recorded in the PR bodies, with a different configuration from §4.3:

- V(1st & 10) was 1.25 at the opponent's 90 and 4.91 at the 10; mean dV ≈ 0; 1.96 average drive points.
- Participation coverage was 100%, with 5.97 skill players and 11 defenders per play.
- RAPM "ranks sanely": top QB C.J. Stroud; top WRs CeeDee Lamb, Deebo Samuel and Tank Dell.

The §4.3 measurement, on the full-roster universe and regular season only, ranks QBs very
differently. Neither is a parity target.
