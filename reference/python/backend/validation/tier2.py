"""Tier-2 matchup-grade predictive check (roadmap Phase 2c; validation plan §9).

Tier 1 scores the *player* forecast; Tier 2 asks whether the engine's
**defensive matchup grades** carry real predictive signal: does a tougher
graded defence (higher ``rapm_grade``, the sign-flipped team-defence intercept
``weekly_update._upsert_matchup_grades`` stores as-of W-1) actually allow
fewer fantasy points in week W?

The KPI is **predictive power = mean over weeks of -spearman(grade, points
allowed)** — positive when tougher grades line up with fewer points allowed.
Correlating within each week (then aggregating with a bootstrap CI over the
weekly values) keeps league-wide week effects — weather slates, scoring
environment — from confounding a pooled correlation.  The KPI is directional
("> 0, CI excludes 0" like every §9 gate) and is the quantity the
calibrate-then-gate registry (:mod:`backend.validation.thresholds`) freezes a
noise floor for after the first real run.

Synth-gated in CI; the real numbers come from the Phase-2c verdict run on the
frozen snapshot (grades from the walk-forward RAPM solve, points allowed
aggregated from realized play-by-play).
"""
from __future__ import annotations

import pandas as pd

from backend.validation.metrics import bootstrap_ci, spearman

_JOIN = ["season", "week", "def_team"]


def tier2_report(
    grades: pd.DataFrame,
    allowed: pd.DataFrame,
    *,
    min_teams: int = 6,
    n: int = 10000,
    alpha: float = 0.05,
    seed: int = 0,
) -> dict:
    """Score matchup grades against realized points allowed, week by week.

    ``grades`` is ``[season, week, def_team, rapm_grade]`` (one row per
    defence per week — the caller picks a position/situation slice before
    passing it in); ``allowed`` is ``[season, week, def_team,
    points_allowed]``, the realized fantasy points scored against that
    defence that week.  Rows are inner-joined on ``(season, week, def_team)``
    — a graded defence with no realized game that week (bye) simply drops
    out.  Weeks with fewer than ``min_teams`` joined defences are skipped
    (a two-team correlation is noise, not signal).

    Both inputs must be unique on the join key — ``matchup_grades`` stores
    the same team grade once per *position*, so an unsliced frame would
    silently multiply through the merge; duplicates are rejected rather than
    deduped so that caller mistake stays loud.

    Returns ``{n_weeks, n_cells, weekly, predictive}`` where ``weekly`` is
    the per-week ``-spearman(grade, points_allowed)`` list (chronological)
    and ``predictive`` is its :func:`~backend.validation.metrics.bootstrap_ci`
    — the Tier-2 gate is ``point > 0`` with ``excludes_zero``.  Raises
    ``ValueError`` when no week qualifies, rather than reporting a KPI from
    nothing.
    """
    if min_teams < 2:
        raise ValueError(f"min_teams must be >= 2 (a <2-team spearman is NaN), "
                         f"got {min_teams}")
    for label, frame in (("grades", grades), ("allowed", allowed)):
        dups = frame.duplicated(subset=_JOIN)
        if dups.any():
            raise ValueError(
                f"{label} has {int(dups.sum())} duplicate (season, week, "
                f"def_team) rows — slice to one row per defence per week "
                f"(e.g. a single position/situation) before scoring")
    merged = grades.merge(allowed, on=_JOIN, how="inner")
    weekly: list[float] = []
    n_cells = 0
    for (_s, _w), grp in merged.sort_values(_JOIN).groupby(["season", "week"]):
        if len(grp) < min_teams:
            continue
        weekly.append(-spearman(grp["rapm_grade"], grp["points_allowed"]))
        n_cells += len(grp)
    if not weekly:
        raise ValueError(
            f"no (season, week) slot has >= min_teams={min_teams} joined "
            f"defences — nothing to score")
    return {
        "n_weeks": len(weekly),
        "n_cells": n_cells,
        "weekly": weekly,
        "predictive": bootstrap_ci(weekly, n=n, alpha=alpha, seed=seed),
    }


__all__ = ["tier2_report"]
