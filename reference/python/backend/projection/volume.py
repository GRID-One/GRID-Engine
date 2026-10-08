"""Volume / role model — the dominant, sticky driver of fantasy points (§4.5).

Fantasy points = volume x efficiency.  Volume (attempts / targets / carries /
receptions) is what most of the points ride on and it is far more predictable
than week-to-week efficiency, yet GRID's RAPM/Kalman measure efficiency only.
So volume gets its own projection here; the GRID talent features (a later
module) supply the efficiency side, and the fitted stat-line model combines them.

The projector is an **empirical-Bayes shrinkage** of a player's own recent
per-game usage toward the position's per-game mean, with the shrinkage weight
``g/(g+k)`` growing with games played ``g`` — a small-sample player regresses to
his position, a full-season player keeps his own rate.  This is a "fitted on
historical seasons" estimator (the shrinkage target + weight are learned from the
data), not the hand-picked constants the orphaned ``fantasy_scoring.py`` used.

A manual **override hook** lets ADP / depth-chart knowledge replace a player's
projected usage — the documented mitigation for offseason role changes (§4.5),
the one place naive prior-usage is weakest.

Leakage-safe: only usage from **before** the forecast point is read (prior
seasons in full, plus the current season through ``W-1``), mirroring
``validation.asof.AsOf``'s information-time cutoff.
"""
from __future__ import annotations

from dataclasses import dataclass

import pandas as pd

from backend.validation.asof import AsOf

# The usage drivers the volume model projects; efficiency (yards/TDs per
# opportunity, completion rate) is the talent model's job, not this one.
VOLUME_COLS = ["pass_attempts", "rush_attempts", "targets", "receptions"]


@dataclass
class VolumeProjection:
    """Per-game projected usage per player, with provenance.

    ``per_game`` maps ``player_id -> {volume_col: projected per-game count}``;
    ``games_prior`` maps ``player_id -> games`` in the recent season the
    projection was built from (the shrinkage weight's denominator input).
    """

    per_game: dict
    games_prior: dict


def project_volume(
    history: pd.DataFrame,
    asof: AsOf,
    *,
    overrides: dict | None = None,
    k_shrink: float = 8.0,
    player_col: str = "player_id",
    pos_col: str = "position",
    season_col: str = "season",
    week_col: str = "week",
) -> VolumeProjection:
    """Project per-game usage as of ``asof``, shrinking recent rate to position mean.

    ``history`` is a weekly ``player_stats``-shaped frame (one row per
    player-week) carrying ``VOLUME_COLS`` + position.  Each player's most recent
    *prior* season supplies his per-game rate and game count ``g``; the
    projection is ``g/(g+k_shrink)`` of that rate plus the rest from the
    position's per-game mean.  ``overrides`` (``{player_id: {col: value}}``)
    replaces projected values for role-changers.
    """
    keep = ((history[season_col] < asof.season)
            | ((history[season_col] == asof.season)
               & (history[week_col] <= asof.outcome_cutoff)))
    hist = history[keep]
    if hist.empty:
        return VolumeProjection({}, {})

    # per (player, season): games played + season-total usage + position
    grp = hist.groupby([player_col, season_col])
    agg = grp.agg(games=(week_col, "nunique"),
                  **{c: (c, "sum") for c in VOLUME_COLS})
    agg[pos_col] = grp[pos_col].first()
    agg = agg.reset_index()
    for c in VOLUME_COLS:
        agg[f"{c}_pg"] = agg[c] / agg["games"]

    # each player's MOST RECENT *prior* season is the usage base (recency).
    # Current-season-to-date rows are kept in the as-of window (leakage boundary)
    # but excluded from the baseline: the documented contract is prior-season, and
    # the alpha's primary use is preseason draft prep.  Mid-season blending of
    # current-to-date usage is a deferred extension.
    prior = agg[agg[season_col] < asof.season]
    if prior.empty:
        return VolumeProjection({}, {})
    recent = prior.loc[prior.groupby(player_col)[season_col].idxmax()].set_index(player_col)

    # position per-game means (the shrinkage target), over those recent rows
    pos_means = {c: recent.groupby(pos_col)[f"{c}_pg"].mean().to_dict() for c in VOLUME_COLS}

    overrides = overrides or {}
    per_game: dict = {}
    games_prior: dict = {}
    for pid, row in recent.iterrows():
        g = float(row["games"])
        position = row[pos_col]
        w = g / (g + k_shrink)          # own-rate weight grows with games
        proj = {}
        for c in VOLUME_COLS:
            pos_mean = pos_means[c].get(position, row[f"{c}_pg"])
            proj[c] = w * row[f"{c}_pg"] + (1.0 - w) * pos_mean
        if pid in overrides:
            unknown = set(overrides[pid]) - set(VOLUME_COLS)
            if unknown:
                raise ValueError(
                    f"unknown volume override columns for {pid}: {sorted(unknown)} "
                    f"(allowed: {VOLUME_COLS})")
            proj.update(overrides[pid])
        per_game[pid] = proj
        games_prior[pid] = int(row["games"])
    return VolumeProjection(per_game, games_prior)
