"""Baseline forecasters the GRID engine is scored against (Phase 2a; §9, §11).

The walk-forward backtest reports GRID's **skill vs baselines**, not raw error —
"does GRID beat the references a fantasy manager already has?" (validation plan
§9).  This module supplies those references, each forecasting expected fantasy
points **per game** from a tidy realized-points history so it is horizon-flexible
(weekly = per-game; ROS = per-game × remaining games):

* **persistence** — last observed week (the trivial "they'll do what they just
  did" bar; H1 must clear it by a wide margin).
* **season-to-date mean** — current-season average, *with a spread* so it can be
  a CRPS reference (the real value bar per §9).
* **last-season-actuals** — prior-season per-game average; this is what
  ``compute_valuations`` ships today, so beating it is the **H1 kill criterion**.
* **market** — an external ADP/ESPN projection (report-only).

Every history-derived baseline slices through the :class:`~backend.validation.asof.AsOf`
cutoffs (outcomes strictly before week W), so the baselines are leakage-safe by
construction just like the GRID path.
"""
from __future__ import annotations

from dataclasses import dataclass

import numpy as np
import pandas as pd

from backend.validation.asof import AsOf


@dataclass
class PointForecast:
    """A per-player point forecast, optionally with a per-player Gaussian spread.

    ``mean`` maps ``player_id -> expected points`` (per game).  ``sd`` is set only
    by baselines that emit a spread (``season_to_date_mean``), so the probabilistic
    metrics (CRPS/PICP) have a reference that predicts uncertainty; point-only
    baselines leave it ``None``.
    """

    mean: dict
    sd: dict | None = None


def _history_before_w(history, asof: AsOf, season_col: str, week_col: str):
    """Current-season rows whose outcomes are known before kickoff of week W."""
    return history[(history[season_col] == asof.season)
                   & (history[week_col] <= asof.outcome_cutoff)]


def persistence(
    history: pd.DataFrame,
    asof: AsOf,
    *,
    points_col: str = "points",
    player_col: str = "player_id",
    season_col: str = "season",
    week_col: str = "week",
) -> PointForecast:
    """Forecast = each player's most recent week of points before W.

    Players with no current-season game before W are absent (the backtest aligns
    players across baselines); at W=1 the forecast is therefore empty.
    """
    cur = _history_before_w(history, asof, season_col, week_col)
    mean: dict = {}
    for pid, g in cur.groupby(player_col):
        last_wk = g[week_col].max()
        mean[pid] = float(g.loc[g[week_col] == last_wk, points_col].mean())
    return PointForecast(mean)


def season_to_date_mean(
    history: pd.DataFrame,
    asof: AsOf,
    *,
    points_col: str = "points",
    player_col: str = "player_id",
    season_col: str = "season",
    week_col: str = "week",
) -> PointForecast:
    """Forecast = current-season-to-date mean, with a per-player std spread.

    A player with a single game so far has no sample std, and a zero std would
    make CRPS/PICP degenerate; both cases fall back to the **pooled** spread (the
    cross-player std of the means, floored small), so every player gets a usable,
    non-zero ``sd``.
    """
    cur = _history_before_w(history, asof, season_col, week_col)
    means: dict = {}
    sds: dict = {}
    for pid, g in cur.groupby(player_col):
        v = g[points_col].to_numpy(float)
        means[pid] = float(v.mean())
        sds[pid] = float(v.std(ddof=1)) if len(v) >= 2 else float("nan")
    pooled = (float(np.std(list(means.values()), ddof=1))
              if len(means) >= 2 else float("nan"))
    floor = pooled if np.isfinite(pooled) and pooled > 0 else 1e-6
    for pid, s in sds.items():
        if not np.isfinite(s) or s <= 0:
            sds[pid] = floor
    return PointForecast(means, sds)


def last_season(
    history: pd.DataFrame,
    asof: AsOf,
    *,
    points_col: str = "points",
    player_col: str = "player_id",
    season_col: str = "season",
) -> PointForecast:
    """Forecast = prior-season per-game average (the last-season-actuals baseline).

    This is what ``compute_valuations`` ships today, so it is the H1 kill-criterion
    reference.  Uses the full prior season (all known before the current season).
    """
    prev = history[history[season_col] == asof.season - 1]
    mean = {pid: float(g[points_col].mean()) for pid, g in prev.groupby(player_col)}
    return PointForecast(mean)


def market(
    projection,
    *,
    points_col: str = "projected_points",
    player_col: str = "player_id",
) -> PointForecast:
    """Wrap an external ADP/ESPN projection (report-only) as a PointForecast.

    ``projection`` is a ``{player_id: points}`` mapping or a DataFrame with
    ``player_col``/``points_col``.  It is a pre-season/pre-game projection, not an
    outcome, so no as-of slicing applies.
    """
    if isinstance(projection, pd.DataFrame):
        mean = dict(zip(projection[player_col], projection[points_col].astype(float)))
    else:
        mean = {k: float(v) for k, v in projection.items()}
    return PointForecast(mean)
