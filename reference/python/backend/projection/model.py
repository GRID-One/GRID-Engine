"""Fitted stat-line projection model (roadmap Phase 2b §4.5).

Replaces the orphaned `fantasy_scoring.py`'s hand-picked ``GRID_SCALING``
constants with coefficients **fit** on historical seasons. Structure (§4.5):

    projected_stat_line(player) = g(volume features, GRID talent features)
    projected_points(format) = scoring.engine.calculate_points(stat_line, format)

— project a stat line, then score it through any format (one projection, many
scoring rule sets). The volume side (`volume.py`'s ``VOLUME_COLS``:
``pass_attempts``, ``rush_attempts``, ``targets``, ``receptions``) is already
projected upstream and copied straight through here unchanged. This module
owns the **efficiency** side: each downstream box-score stat the volume model
doesn't already supply (yards, TDs, completions, interceptions) is produced as
``volume_driver * fitted_rate(talent_features)`` — a small standardized-feature
ridge regression of the historical per-unit rate (e.g. yards-per-attempt) on
the GRID talent
features (`features.assemble_talent_features`'s ``rapm_rating``,
``smoothed_talent``, ``prior_mean``). This is exactly §4.5's "GRID enters as a
*feature*, not as the projection itself" — efficiency, not volume.

``fumbles_lost`` and ``two_point_conversions`` are low-magnitude, rarely-driven
stats left at 0.0 (unmodeled) — the old orphaned module never touched them
either.

Fit is **per position** (rate magnitude/variance differs enough across
positions that pooling would blur the signal) — callers fit one
`StatLineModel` per position. The real-data fit (Phase 2c) trains through
`validation.backtest`'s leakage-safe walk-forward harness; this module builds
and synth-validates the fitting + scoring machinery only — no real
coefficients are frozen here.
"""
from __future__ import annotations

from dataclasses import dataclass, field

import numpy as np
import pandas as pd
from sklearn.linear_model import Ridge
from sklearn.preprocessing import StandardScaler

from backend.projection.volume import VOLUME_COLS
from backend.scoring.columns import SCORING_STAT_COLS
from backend.scoring.engine import ScoringConfig, calculate_points

# The GRID talent features (features.TalentFeatures.to_frame() columns) the
# rate models regress on.
TALENT_FEATURE_COLS = ["rapm_rating", "smoothed_talent", "prior_mean"]

# {volume_driver: [downstream stats produced as volume_driver * fitted_rate]}.
# Every SCORING_STAT_COLS entry is accounted for: VOLUME_COLS are copied
# through directly (the volume model's job); these are the efficiency-derived
# remainder; fumbles_lost/two_point_conversions are deliberately unmodeled.
RATE_STATS = {
    "pass_attempts": ["completions", "passing_yards", "passing_tds", "interceptions"],
    "rush_attempts": ["rushing_yards", "rushing_tds"],
    "receptions": ["receiving_yards", "receiving_tds"],
}


@dataclass
class RateModel:
    """A fitted ridge regression: per-unit rate ~ talent features, for one stat.

    ``scaler`` standardizes features before the ridge fit/predict, so the L2
    penalty (``alpha``) shrinks every feature on a comparable scale rather than
    favoring whichever talent feature happens to have larger raw magnitude.
    """

    ridge: Ridge
    scaler: StandardScaler
    feature_cols: list

    def predict(self, talent_row: dict) -> float:
        # ``dict.get(c, 0.0)`` returns the default only for ABSENT keys; a
        # present-but-NaN feature (non-rookies carry a NaN ``prior_mean`` from
        # features.to_frame()) slips through and crashes Ridge.predict via
        # StandardScaler ("Input X contains NaN"). Fill NaN/inf with 0.0 to
        # match the fit side's ``fillna(0.0)`` (fit_stat_line_model below) so
        # predict stays train-consistent instead of raising. (audit finding P1)
        x = np.array([[talent_row.get(c, 0.0) for c in self.feature_cols]], dtype=float)
        x = np.nan_to_num(x, nan=0.0, posinf=0.0, neginf=0.0)
        return float(self.ridge.predict(self.scaler.transform(x))[0])


@dataclass
class StatLineModel:
    """Fitted rate models for one position, ready to project + score."""

    rates: dict = field(default_factory=dict)   # {(volume_col, stat): RateModel}

    def project_stat_line(self, volume_per_game: dict, talent_row: dict) -> dict:
        """Per-game stat line over all 14 `SCORING_STAT_COLS` for one player.

        Volume columns are copied through from ``volume_per_game`` unchanged;
        every other stat is ``volume * fitted_rate(talent_row)`` (0.0 when no
        rate model was fit for that pair — e.g. zero training volume).
        """
        line = {c: 0.0 for c in SCORING_STAT_COLS}
        for vol_col in VOLUME_COLS:
            line[vol_col] = max(0.0, float(volume_per_game.get(vol_col, 0.0)))
        for vol_col, stats in RATE_STATS.items():
            vol = line[vol_col]
            for stat in stats:
                rate_model = self.rates.get((vol_col, stat))
                rate = rate_model.predict(talent_row) if rate_model is not None else 0.0
                line[stat] = max(0.0, vol * rate)
        return line

    def project_fantasy_points(
        self, volume_per_game: dict, talent_row: dict, config: ScoringConfig,
    ) -> float:
        """`project_stat_line` then score it through ``config`` — format-flexible."""
        return calculate_points(self.project_stat_line(volume_per_game, talent_row), config)


def fit_stat_line_model(
    history: pd.DataFrame,
    *,
    feature_cols: list = TALENT_FEATURE_COLS,
    alpha: float = 1.0,
) -> StatLineModel:
    """Fit per-(volume_driver, stat) ridge rate models on a tidy season history.

    ``history`` is one row per (player, season) carrying `RATE_STATS`' volume
    drivers + downstream stats (season totals — the rate ``stat/volume`` is
    invariant to per-game vs. season aggregation, since both numerator and
    denominator scale together) plus ``feature_cols`` (that season's talent
    features). Rows with zero volume for a driver are excluded from *that
    driver's* fits only — a zero-volume row has an undefined per-unit rate, not
    a zero rate, and would otherwise poison the regression with inf/NaN.
    """
    rates: dict = {}
    for vol_col, stats in RATE_STATS.items():
        sub = history[history[vol_col] > 0]
        if sub.empty:
            continue
        X = sub[feature_cols].fillna(0.0).to_numpy(float)
        scaler = StandardScaler().fit(X)          # shared scaler across this driver's stats
        X_scaled = scaler.transform(X)
        for stat in stats:
            y = (sub[stat] / sub[vol_col]).to_numpy(float)
            ridge = Ridge(alpha=alpha)
            ridge.fit(X_scaled, y)
            rates[(vol_col, stat)] = RateModel(ridge, scaler, list(feature_cols))
    return StatLineModel(rates)
