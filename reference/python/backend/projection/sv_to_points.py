"""SV -> fantasy-points map (roadmap Phase 2b §4.5; validation plan §7).

The weekly Layer-1 credit (`layers.layer1_all_players`, #80) is in **dV / SV
currency** — expected-points-above-context per snap — not fantasy points. H2
(the weekly lineup-margin headline) needs a *weekly fantasy-point* projection
for RB/WR/TE, so this module fits a **per-position affine map**

    weekly_fantasy_points ≈ a_pos + b_pos * weekly_credit

on historical ``(weekly credit, realized weekly fantasy points)`` pairs. Affine
(not just a scalar) because a position's baseline scoring floor (``a_pos``) and
its points-per-unit-credit slope (``b_pos``) both differ — a WR's PPR floor and
a QB's passing-heavy scoring don't share a conversion constant, which is exactly
why the orphaned `fantasy_scoring.py`'s single hand-picked constant per stat
was uncalibrated.

Fit is per position and **leakage-safe**: only ``(credit, points)`` pairs from
before the forecast point train the map (via `validation.asof.AsOf`), matching
the weekly one-step-ahead calibration target (validation plan §7). This is the
weekly counterpart to the season-long stat-line model (`model.py`): that one
drives the ROS/draft projection, this one the weekly injury-replacement lever.

The real-data fit + weekly calibration verdict are Phase 2c; this module builds
and synth-validates the map only.
"""
from __future__ import annotations

from dataclasses import dataclass

import numpy as np
import pandas as pd

from backend.validation.asof import AsOf


@dataclass
class SVToPointsMap:
    """Per-position affine credit->points maps: ``{position: (intercept, slope)}``."""

    coeffs: dict

    def predict(self, position: str, weekly_credit: float) -> float:
        """Weekly fantasy points for a per-snap ``weekly_credit`` at ``position``.

        Positions without a fitted map (too little history) fall back to 0.0 —
        "no calibrated conversion" is surfaced as no projection, not a guess.
        """
        if position not in self.coeffs:
            return 0.0
        a, b = self.coeffs[position]
        return float(a + b * weekly_credit)


def fit_sv_to_points(
    history: pd.DataFrame,
    asof: AsOf,
    *,
    credit_col: str = "credit",
    points_col: str = "fantasy_points",
    pos_col: str = "position",
    week_col: str = "week",
    min_rows: int = 10,
) -> SVToPointsMap:
    """Fit the per-position affine SV->points map on pre-cutoff weekly pairs.

    ``history`` is one row per (player, season, week) carrying the weekly Layer-1
    ``credit``, the realized weekly ``fantasy_points``, and ``position``.  Only
    rows strictly before the ``asof`` cutoff (prior seasons in full + current
    season through ``W-1``) train the map — the cutoff comes from the canonical
    :meth:`AsOf.slice_plays` so the leakage contract stays defined in one place.
    A position with fewer than ``min_rows`` training rows, or with no spread in
    credit (a degenerate slope), is omitted (``predict`` returns 0.0 for it).
    """
    hist = asof.slice_plays(history, week_col=week_col)

    coeffs: dict = {}
    for pos, grp in hist.groupby(pos_col):
        if len(grp) < min_rows:
            continue
        x = grp[credit_col].to_numpy(float)
        y = grp[points_col].to_numpy(float)
        if np.ptp(x) == 0:            # no credit spread -> slope undefined; skip
            continue
        slope, intercept = np.polyfit(x, y, 1)
        coeffs[pos] = (float(intercept), float(slope))
    return SVToPointsMap(coeffs)
