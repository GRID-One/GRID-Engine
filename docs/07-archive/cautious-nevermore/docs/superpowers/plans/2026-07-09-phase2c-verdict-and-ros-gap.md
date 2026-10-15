# Phase 2c — first real verdict + the ROS-projection scoring gap

_Recorded 2026-07-09. Run label: `real-2022-2023-first-run`._

## What was run

First real Phase-2c verdict on nflverse data for **2022–2023** (2 seasons,
100% participation coverage; 70,778 plays):

```
python -m backend.pipeline.ingest_grid   --years 2022 2023
python -m backend.pipeline.data_pipeline  --years 2022 2023
python -m backend.validation.verdict --run-label real-2022-2023-first-run
```

## Verdict: `BASELINE_FALLBACK`

H1 (ROS, kill criterion): GRID vs last-season-actuals margin
**−0.864 [−0.932, −0.796]** — CI firmly on the wrong side of 0. GRID also loses
to season-to-date mean; it only beats trivial persistence (and fails even that
for RB). H2 weekly lineup margin −0.524 [−1.486, +0.461] (indistinguishable
from 0). Tier 2 skipped (no points-allowed feed).

Calibration is reasonable even where the point estimate is weak: weekly
NIS(ALL)=1.00, ROS NIS(ALL)=1.25, PICP@80 ≈ 0.75–0.87.

**Per the roadmap (§Phase 2c done-when + §7 Risks), this is a designed outcome,
not a failure:** "H1 clears the kill criterion → ship GRID-powered; else
documented fallback to baseline for alpha." `compute_valuations` stays on the
last-season baseline for alpha. This note is that documented fallback.

## Known limitation found while recording the verdict (drives the re-run)

The verdict's **ROS (H1) table is scored by the wrong projection.**
`verdict.py` builds a single `forecasts` frame from the **weekly** SV→points
affine map (`tier1.forecast_weekly_points` → `sv_to_points.SVToPointsMap`) and
feeds that same frame to both `tier1_report(horizon="weekly")` and
`tier1_report(horizon="ros")` (verdict.py:191–195). The only projection import
is `sv_to_points` (verdict.py:39); the season stat-line model
(`projection/model.py`), volume model (`projection/volume.py`), and preseason
assembly (`projection/preseason.py`) are never imported.

This contradicts the design. `sv_to_points.py`'s own docstring: *"the
season-long stat-line model (`model.py`): that one drives the ROS/draft
projection, this one the weekly injury-replacement lever."* So H1 is currently
measuring the weekly lever on the ROS horizon, not the volume×efficiency
projection the spec (§4.5) assigns to ROS. The −0.864 fallback therefore
reflects, at least partly, scoring the wrong projection — not a settled verdict
on GRID.

## Decisions

1. **Gate registry NOT frozen on this run.** The spec's rule is that the
   *single first full backtest pass* freezes each KPI's noise-floor gate. That
   pass must score the correct ROS projection, so `provisional_thresholds.json`
   was reset to `{"kpis": {}}`; the corrected re-run will freeze the gates.
2. **Baseline fallback stands for now** (compute_valuations unchanged) — but the
   H1 verdict is treated as *provisional pending the corrected ROS run*.

## Next step (this branch: `phase2c/ros-projection-gap`)

Wire the Phase-2b volume×efficiency projection into the verdict's ROS forecast
path (leakage-safe, per-origin), re-run the verdict, then freeze gates and
re-decide H1.

## The ROS-scoring fix (this branch)

The gap is closed. `verdict.run_verdict` now takes an optional
`stats_history`; when present, the ROS (H1) table is scored by the season
`StatLineModel` (volume×efficiency, `projection/model.py`) instead of the
weekly SV→points map:

* `tier1.forecast_ros_points` — per origin, each rated player's as-of-W volume
  projection (`project_volume`, prior-season shrinkage) is run through a
  per-position `StatLineModel` fit **once** on pre-first-origin player-season
  totals (frozen, mirroring the frozen `sv_map`/V(s)), then scored to per-game
  points. RAPM enters as the `rapm_rating` talent feature; `smoothed_talent`
  and `prior_mean` aren't available from walk-forward origins, so they enter as
  the constant 0.0 the predict side already supplies (fit and forecast share
  one feature space).

A first re-run (`real-2022-2023-ros-corrected`) confirmed the −0.864 was a
scoring artifact — ROS margin moved to −0.015 — but it was **still run on the
broken player universe**: the CLI fed `build_design` a fantasy-skill-only
players frame, so RAPM dropped ~22k participants/origin as unknown IDs (the
`build_design: dropped N ...` warnings). **PR #89** fixed that (full nflverse
roster as the RAPM universe). The trustworthy verdict is the run on both fixes.

## Trustworthy verdict (`real-2022-2023-universe-fixed`, post-#89)

**H1 (ROS kill criterion): `BASELINE_FALLBACK` — margin −0.015 [−0.059,
+0.029].** Essentially identical to the broken-universe number, and that is the
reassuring part: the ROS forecast is volume-dominated (volume×efficiency), with
RAPM only one talent feature, so fixing the RAPM universe barely moves ROS.
ALL n=7035:

| baseline | ALL margin | reading |
|----------|-----------|---------|
| vs persistence      | +0.957 [+0.848, +1.068] | GRID clearly better |
| vs season-to-date   | −0.194 [−0.270, −0.120] | slightly worse |
| vs last-season      | −0.015 [−0.059, +0.029] | tie (kill criterion not cleared) |

Per position **TE clears the kill criterion** (vs last-season +0.064 [+0.011,
+0.118] *and* vs season-mean +0.228 PASS); RB (+0.005) and QB (−0.036) are
ties; WR marginally worse (−0.066). (`DB`/`P` rows appear because the ROS
model fits any position present in `player_stats`; at 15/30 cells they don't
move ALL. **Kept deliberately** — these non-skill rows are slated to feed the
weekly in-season mode, so the ROS model is intentionally position-agnostic.)

**H2 (weekly lineup margin): now PASSES — +0.848 [+0.232, +1.466]**, up from
−0.524 [−1.486, +0.461] on the broken universe (win rate 0.478→0.542). H2 rides
directly on the weekly RAPM forecasts, so the universe fix helped it
materially, and it clears its frozen noise-floor gate (+0.848 > +0.316).

Gate freezing is a **per-run runtime** step (sign-flip null, mean + 1.96·SE);
the committed `provisional_thresholds.json` ships **empty** by contract
(`test_committed_registry_is_valid_and_empty`), so this run's gates
(`h1_ros` +0.0564, `h2` +0.3164) live in the report, not in VCS.

## Decision (settled)

`compute_valuations` stays on the last-season baseline for alpha — the **H1
kill criterion (ROS vs last-season) is a tie, not a win**. But the corrected,
correct-universe result is a live signal, not a dead end: GRID's ROS projection
is at parity with last-season-actuals, beats trivial persistence everywhere,
beats both references for TE, and — the headline — its **weekly lineup value
(H2) is now significantly positive**. Natural follow-ups (not this branch):
feed the Kalman `smoothed_talent` and cross-league `prior_mean` into the ROS
talent features (only `rapm_rating` is live today), add the market/ADP
baseline, and supply a points-allowed feed to unskip Tier 2. (The ROS model
stays position-agnostic on purpose — the non-skill rows feed the weekly
in-season mode.)

## Update — `smoothed_talent` wired into the ROS features (follow-up done)

The first of those follow-ups is done. `verdict._smoothed_talent_pre_first_origin`
now computes each player's **RTS-smoothed end-of-pre-period Kalman talent** (the
§4.5 "best retrospective talent") from their **weekly Layer-1 credit** series
(`layers.layer1_all_players`) over the window strictly before the first forecast
origin, and supplies it as the `smoothed_talent` rate-model feature in **both**
the ROS fit (`_fit_ros_models`) and forecast (`tier1.forecast_ros_points`) — so
it is no longer the dead constant 0.0. It is **frozen** on the pre-first-origin
window and reused at every origin, mirroring the frozen `sv_map`/V(s)/rate-model
discipline: `rapm_rating` is the fast season-to-date talent (updates per origin),
`smoothed_talent` the slow retrospective prior-period talent. Leakage-safe by
construction (pre-window plays + warm-up RAPM as the opponent lookup; a unit test
asserts truncating the input to pre-origin weeks leaves it bit-identical). The
verdict re-run on the frozen snapshot is the dev/nightly step that measures
whether the richer talent vector moves H1; **`prior_mean` stays 0.0** (blocked on
wiring `priors.py` onto real data) and remains a follow-up alongside the
market/ADP baseline and the Tier-2 points-allowed feed.
