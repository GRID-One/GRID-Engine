"""Tier-1 skill + calibration runner (roadmap Phase 2c; validation plan §7, §9).

The **H1 headline** (bench-depth / ROS, the kill criterion): GRID beats the
**last-season-actuals** baseline on rest-of-season skill, "> 0, CI excludes 0";
beating season-to-date-mean and the market is the real value bar.  This module
turns the walk-forward's per-origin GRID ratings into scoreable per-player
fantasy forecasts and renders the per-position skill + calibration tables the
verdict rests on:

* :func:`forecast_weekly_points` — ratings from
  :func:`~backend.validation.backtest.walk_forward` (as-of-W, leakage-safe by
  construction) through the per-position SV->points map
  (:mod:`backend.projection.sv_to_points`) into a tidy per-player-week
  fantasy-point forecast frame.
* :func:`tier1_report` — scores any such forecast frame against realized
  points: **skill** vs each baseline in :mod:`backend.validation.baselines`
  (persistence / season-to-date mean / last-season), stated per §9 as a
  *paired* per-cell margin ``|baseline error| - |GRID error|`` bootstrapped via
  :func:`~backend.validation.metrics.bootstrap_ci` (mean > 0 with the CI
  excluding 0 = GRID beats that baseline, significantly); and **calibration**
  (PICP + PINAW paired, CRPS, NIS) using an expanding, strictly-past-errors
  spread so the probabilistic scores are as leakage-safe as the point ones.
  Tables are per position plus an ``ALL`` aggregate.

Horizon-flexible like the baselines (per-game currency): ``horizon="weekly"``
scores each origin against that week's realized points (the H2-adjacent weekly
signal); ``horizon="ros"`` scores against the realized rest-of-season per-game
mean — the H1 target.  The real numbers come from the Phase-2c verdict run on
the frozen snapshot; CI runs this on synth only.
"""
from __future__ import annotations

import numpy as np
import pandas as pd

from backend.projection.sv_to_points import SVToPointsMap
from backend.validation import baselines as B
from backend.validation.asof import AsOf
from backend.validation.metrics import (
    bootstrap_ci,
    crps_gaussian,
    nis,
    picp,
    pinaw,
    skill_score,
)

_ALL = "ALL"


def default_baselines() -> dict:
    """The §9 baseline set GRID is scored against, name -> forecaster.

    Each is a :mod:`backend.validation.baselines` callable
    ``(history, asof) -> PointForecast``; ``last_season`` is the H1
    kill-criterion reference.  ``market`` is report-only and needs an external
    projection, so it is not part of the history-derived default set — the
    verdict runner wires it separately when an ADP feed is present.
    """
    return {
        "persistence": B.persistence,
        "season_to_date_mean": B.season_to_date_mean,
        "last_season": B.last_season,
    }


def forecast_weekly_points(
    origins: list,
    sv_map: SVToPointsMap,
    positions: dict,
) -> pd.DataFrame:
    """GRID's weekly fantasy-point forecast frame from walk-forward ratings.

    ``origins`` is :func:`~backend.validation.backtest.walk_forward`'s
    :class:`~backend.validation.backtest.OriginResult` list (each rating is
    as-of that origin, so the leakage safety is inherited); ``positions`` maps
    ``player_id -> position``.  Each rated player is pushed through the
    per-position affine SV->points map.  Players with an unknown position or a
    position without a fitted map are omitted (no calibrated conversion means
    no forecast, not a 0.0 guess — mirroring ``SVToPointsMap.predict``'s
    contract).  Returns ``[season, week, player_id, position, forecast]``.
    """
    rows = []
    for o in origins:
        for pid, rating in o.ratings.items():
            pos = positions.get(pid)
            if pos is None or pos not in sv_map.coeffs:
                continue
            rows.append({
                "season": o.season, "week": o.week,
                "player_id": pid, "position": pos,
                "forecast": sv_map.predict(pos, rating),
            })
    return pd.DataFrame(rows, columns=["season", "week", "player_id",
                                       "position", "forecast"])


def forecast_ros_points(
    origins: list,
    models: dict,
    volume_by_origin: dict,
    positions: dict,
    scoring_config,
    *,
    smoothed: dict | None = None,
    games: int = 1,
) -> pd.DataFrame:
    """GRID's ROS forecast from the volume x efficiency stat-line model (§4.5).

    This is the ROS-horizon counterpart to :func:`forecast_weekly_points`.  The
    design assigns ROS/draft projection to the season stat-line model
    (:mod:`backend.projection.model`), NOT the weekly SV->points affine map
    (which is the weekly injury-replacement lever) — see
    ``sv_to_points.py``'s docstring.  Per origin, each rated player's per-game
    stat line is ``StatLineModel.project_stat_line(volume_asof, talent_row)``
    scored through ``scoring_config`` to per-game fantasy points, so it is
    directly comparable to the per-game ROS baselines (``games=1``).

    ``talent_row`` carries the same GRID talent features the models were fit on:
    the origin's ``rapm_rating`` (per-origin, as-of-W) and — when ``smoothed`` is
    supplied — ``smoothed_talent`` (the frozen pre-first-origin RTS-smoothed
    talent, ``{player_id: value}`` from
    :func:`~backend.validation.verdict._smoothed_talent_pre_first_origin`,
    defaulting to 0.0 for players absent from it).  Passing the identical values
    the fit saw keeps fit and forecast on one feature space; omitting
    ``smoothed`` reproduces the ``rapm_rating``-only behaviour.

    ``models`` maps position -> fitted `StatLineModel` (frozen pre-first-origin,
    mirroring the frozen sv_map / V(s)); ``volume_by_origin`` maps
    ``(season, week) -> {player_id: {volume_col: per-game value}}`` (or a
    `VolumeProjection`) — the as-of-W usage projection, so the ROS forecast is
    leakage-safe by construction.  ``games`` scales the per-game forecast to a
    multi-week horizon; it is 1 (per-game, comparable to the per-game ROS
    baselines) everywhere today and reserved for a future weeks-remaining ROS.

    Omission contract (two distinct cases):

    * **No model object** for a player's position (``models.get(pos) is None``),
      no projected volume this origin, or an unknown position -> the player is
      dropped, never scored as a 0.0 guess (mirrors `forecast_weekly_points`).
    * A **present-but-empty** `StatLineModel` (a position whose `RATE_STATS`
      volume was always 0, e.g. DB/P) passes the ``is not None`` check and
      yields a genuine 0.0 forecast row.  This is intentional — those non-skill
      rows are kept to feed the weekly in-season mode — so the model is
      deliberately position-agnostic, not silently swallowing a bad guess.

    Returns ``[season, week, player_id, position, forecast]``.
    """
    smoothed = smoothed or {}
    rows = []
    for o in origins:
        vol = volume_by_origin.get((o.season, o.week))
        if vol is None:
            continue
        per_game = vol.per_game if hasattr(vol, "per_game") else vol
        for pid, rating in o.ratings.items():
            pos = positions.get(pid)
            model = models.get(pos)
            if pos is None or model is None or pid not in per_game:
                continue
            talent_row = {"rapm_rating": float(rating),
                          "smoothed_talent": float(smoothed.get(pid, 0.0))}
            pts = model.project_fantasy_points(
                per_game[pid], talent_row, scoring_config)
            rows.append({
                "season": o.season, "week": o.week,
                "player_id": pid, "position": pos,
                "forecast": pts * games,
            })
    return pd.DataFrame(rows, columns=["season", "week", "player_id",
                                       "position", "forecast"])


def _realized(history: pd.DataFrame, season: int, week: int, horizon: str) -> dict:
    """Per-player realized per-game points for one origin, by horizon.

    ``weekly`` = week-W points; ``ros`` = mean over weeks >= W of the origin
    season (per-game, so it is directly comparable to the per-game baselines —
    no remaining-games scaling enters the skill comparison).
    """
    if horizon == "weekly":
        sub = history[(history["season"] == season) & (history["week"] == week)]
    elif horizon == "ros":
        sub = history[(history["season"] == season) & (history["week"] >= week)]
    else:
        raise ValueError(f"horizon must be 'weekly' or 'ros', got {horizon!r}")
    return {pid: float(g["points"].mean()) for pid, g in sub.groupby("player_id")}


def tier1_report(
    forecasts: pd.DataFrame,
    history: pd.DataFrame,
    *,
    horizon: str = "weekly",
    baselines: dict | None = None,
    level: float = 0.80,
    min_sd_history: int = 20,
    keep_margins: bool = False,
    n: int = 10000,
    alpha: float = 0.05,
    seed: int = 0,
) -> dict:
    """Per-position Tier-1 skill + calibration tables for a forecast frame.

    ``forecasts`` is ``[season, week, player_id, position, forecast]`` (e.g.
    from :func:`forecast_weekly_points`); ``history`` is the tidy realized
    per-game points frame the baselines consume
    (``[player_id, season, week, points]``).  For every origin ``(season,
    week)`` present in ``forecasts``, realized points are taken per
    ``horizon`` and each baseline forecasts through the same
    :class:`~backend.validation.asof.AsOf` cutoff, so GRID and the references
    see identical information.

    Skill is scored on **paired** cells (player has both a GRID and that
    baseline's forecast): the per-cell margin ``|baseline error| - |GRID
    error|`` is bootstrapped (``margin``; mean > 0 with ``excludes_zero`` =
    GRID beats the baseline — the H1 gate when the baseline is
    ``last_season`` and ``horizon="ros"``), alongside the aggregate
    :func:`~backend.validation.metrics.skill_score` on the paired MAEs.
    Baselines with no paired cells for a position are omitted from that
    position's table rather than reported on a different sample.

    Calibration scores the Gaussian ``N(forecast, sd)`` with ``sd`` = the
    expanding standard deviation of that position's forecast errors from
    strictly earlier origins (never the current one), so the probabilistic
    metrics stay one-step-honest; cells before ``min_sd_history`` errors have
    accumulated are skipped, and a position that never reaches it reports
    ``calibration: None``.

    Returns ``{position: {n_cells, mae, baselines: {name: {n_pairs,
    mae_grid, mae_baseline, skill, margin}}, calibration: {n_cells, picp,
    pinaw, crps, nis} | None}}`` plus an ``ALL`` row aggregated over
    positions.  ``keep_margins=True`` additionally attaches the raw paired
    margin list to each baseline entry — the verdict runner uses it to
    promote each KPI's noise-floor gate (sign-flip null) into the
    calibrate-then-gate registry.
    """
    if baselines is None:
        baselines = default_baselines()

    abs_err: dict = {}          # bucket -> [ |grid error| ]
    sd_pool: dict = {}          # position -> [ signed grid errors, past origins ]
    paired: dict = {}           # bucket -> {baseline: ([abs base], [abs grid])}
    calib: dict = {}            # bucket -> {"actual": [], "mean": [], "sd": []}

    origins = sorted({(int(s), int(w)) for s, w in
                      forecasts[["season", "week"]].itertuples(index=False)})
    for season, week in origins:
        asof = AsOf(season=season, week=week)
        realized = _realized(history, season, week, horizon)
        base_fc = {name: fn(history, asof) for name, fn in baselines.items()}
        sub = forecasts[(forecasts["season"] == season)
                        & (forecasts["week"] == week)]

        new_errors: dict = {}   # this origin's signed errors, folded in AFTER scoring
        sd_cache: dict = {}     # position -> sd for THIS origin (pool is frozen
                                # within an origin, so compute it once, not per row)
        for row in sub.itertuples(index=False):
            if row.player_id not in realized:
                continue
            actual = realized[row.player_id]
            err = float(row.forecast) - actual

            # calibration first, from strictly-past errors only
            if row.position not in sd_cache:
                pool = sd_pool.get(row.position, [])
                sd_cache[row.position] = (float(np.std(pool, ddof=1))
                                          if len(pool) >= min_sd_history else None)
            sd = sd_cache[row.position]
            if sd is not None and sd > 0:
                for bucket in (row.position, _ALL):
                    c = calib.setdefault(bucket,
                                         {"actual": [], "mean": [], "sd": []})
                    c["actual"].append(actual)
                    c["mean"].append(float(row.forecast))
                    c["sd"].append(sd)

            for bucket in (row.position, _ALL):
                abs_err.setdefault(bucket, []).append(abs(err))
                for name, fc in base_fc.items():
                    if row.player_id not in fc.mean:
                        continue
                    b_abs = abs(fc.mean[row.player_id] - actual)
                    pb, pg = paired.setdefault(bucket, {}).setdefault(name, ([], []))
                    pb.append(b_abs)
                    pg.append(abs(err))
            new_errors.setdefault(row.position, []).append(err)

        for pos, errs in new_errors.items():
            sd_pool.setdefault(pos, []).extend(errs)

    report: dict = {}
    for bucket, errs in abs_err.items():
        entry: dict = {"n_cells": len(errs), "mae": float(np.mean(errs)),
                       "baselines": {}, "calibration": None}
        for name, (pb, pg) in paired.get(bucket, {}).items():
            mae_g, mae_b = float(np.mean(pg)), float(np.mean(pb))
            margins = [b - g for b, g in zip(pb, pg, strict=True)]
            entry["baselines"][name] = {
                "n_pairs": len(margins),
                "mae_grid": mae_g,
                "mae_baseline": mae_b,
                "skill": skill_score(mae_g, mae_b),
                "margin": bootstrap_ci(margins, n=n, alpha=alpha, seed=seed),
            }
            if keep_margins:
                entry["baselines"][name]["margins"] = margins
        c = calib.get(bucket)
        if c:
            entry["calibration"] = {
                "n_cells": len(c["actual"]),
                "picp": picp(c["actual"], c["mean"], c["sd"], level=level),
                "pinaw": pinaw(c["actual"], c["mean"], c["sd"], level=level),
                "crps": crps_gaussian(c["actual"], c["mean"], c["sd"]),
                "nis": nis(c["actual"], c["mean"], np.square(c["sd"])),
            }
        report[bucket] = entry
    return report


__all__ = ["default_baselines", "forecast_ros_points",
           "forecast_weekly_points", "tier1_report"]
