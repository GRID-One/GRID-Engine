"""Tests for the SV->fantasy-points map (Phase 2b; roadmap §4.5, validation §7).

Synthetic weekly (credit, points) pairs with a planted per-position affine
relationship; checks recovery, per-position independence, leakage safety, and
the degenerate/sparse fallbacks.
"""
import numpy as np
import pandas as pd
import pytest

from backend.validation.asof import AsOf
from backend.projection.sv_to_points import SVToPointsMap, fit_sv_to_points


def _weekly(pos, season, week, credit, points):
    return {"position": pos, "season": season, "week": week,
            "credit": credit, "fantasy_points": points}


def _history(rows):
    return pd.DataFrame(rows)


def _planted(pos, a, b, season, n=40, seed=0):
    """n weekly rows where fantasy_points = a + b*credit (noiseless)."""
    rng = np.random.default_rng(seed)
    credit_vals = rng.uniform(-0.5, 0.5, size=n)
    return [_weekly(pos, season, (i % 17) + 1, c, a + b * c) for i, c in enumerate(credit_vals)]


# --------------------------------------------------------------------------- #
# Recovery                                                                     #
# --------------------------------------------------------------------------- #
def test_recovers_planted_affine_map():
    """Fit recovers the planted intercept + slope for a position."""
    hist = _history(_planted("RB", a=8.0, b=25.0, season=2022))
    m = fit_sv_to_points(hist, AsOf(2023, 1))
    a, b = m.coeffs["RB"]
    assert a == pytest.approx(8.0, abs=1e-6)
    assert b == pytest.approx(25.0, abs=1e-6)
    # and predict() applies it
    assert m.predict("RB", 0.2) == pytest.approx(8.0 + 25.0 * 0.2)


def test_per_position_maps_are_independent():
    """Each position gets its own (intercept, slope) — a WR floor/slope differs
    from a QB's and they don't cross-contaminate."""
    hist = _history(_planted("WR", 6.0, 20.0, 2022, seed=1)
                    + _planted("QB", 14.0, 40.0, 2022, seed=2))
    m = fit_sv_to_points(hist, AsOf(2023, 1))
    assert m.coeffs["WR"][0] == pytest.approx(6.0, abs=1e-6)
    assert m.coeffs["WR"][1] == pytest.approx(20.0, abs=1e-6)
    assert m.coeffs["QB"][0] == pytest.approx(14.0, abs=1e-6)
    assert m.coeffs["QB"][1] == pytest.approx(40.0, abs=1e-6)


# --------------------------------------------------------------------------- #
# Leakage safety                                                               #
# --------------------------------------------------------------------------- #
def test_leakage_future_season_excluded():
    """Forecasting 2023 only trains on <=2022 pairs, not 2023's own outcomes."""
    hist = _history(_planted("RB", 8.0, 25.0, 2022, seed=3)
                    + _planted("RB", 999.0, 999.0, 2023, seed=4))  # future: different map
    m = fit_sv_to_points(hist, AsOf(2023, 1))
    assert m.coeffs["RB"][0] == pytest.approx(8.0, abs=1e-6)   # 2022 map, not 2023


def test_midseason_uses_current_weeks_before_w():
    """Mid-season, current-season weeks < W are usable training pairs."""
    rows = [_weekly("RB", 2023, w, c, 5.0 + 10.0 * c)
            for w, c in enumerate(np.linspace(-0.4, 0.4, 12), start=1)]
    rows.append(_weekly("RB", 2023, 13, 5.0, 9999.0))   # week 13 = future outlier
    m = fit_sv_to_points(_history(rows), AsOf(2023, 13), min_rows=5)   # forecast W=13
    # the week-13 outlier is excluded, so the clean 5 + 10*credit map survives
    assert m.coeffs["RB"][1] == pytest.approx(10.0, abs=1e-6)


# --------------------------------------------------------------------------- #
# Fallbacks                                                                    #
# --------------------------------------------------------------------------- #
def test_sparse_position_omitted():
    """A position below min_rows is not fit; predict() falls back to 0.0."""
    hist = _history([_weekly("TE", 2022, w, 0.1 * w, 4.0) for w in range(1, 5)])  # 4 rows
    m = fit_sv_to_points(hist, AsOf(2023, 1), min_rows=10)
    assert "TE" not in m.coeffs
    assert m.predict("TE", 0.3) == 0.0


def test_zero_credit_spread_omitted():
    """A position whose credit never varies has an undefined slope -> omitted."""
    hist = _history([_weekly("RB", 2022, w, 0.0, 5.0 + w) for w in range(1, 20)])
    m = fit_sv_to_points(hist, AsOf(2023, 1))
    assert "RB" not in m.coeffs


def test_predict_unknown_position_returns_zero():
    """A position with no fitted map returns 0.0 (no guessed conversion)."""
    m = SVToPointsMap(coeffs={"RB": (8.0, 25.0)})
    assert m.predict("K", 0.5) == 0.0
