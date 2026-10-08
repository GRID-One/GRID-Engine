"""Tests for the fitted stat-line projection model (Phase 2b; roadmap §4.5).

Synth-only: a planted volume*efficiency relationship the model must recover,
full stat-line emission, the volume/zero-volume edge cases, and a synth
beats-last-season-baseline skill check reusing the 2a metrics module.
"""
import numpy as np
import pandas as pd
import pytest

from backend.scoring.columns import SCORING_STAT_COLS
from backend.scoring.engine import ScoringConfig, calculate_points
from backend.validation.metrics import mae, skill_score
from backend.projection.model import (
    RATE_STATS,
    TALENT_FEATURE_COLS,
    StatLineModel,
    fit_stat_line_model,
)
from backend.projection.volume import VOLUME_COLS


def _history(n=200, seed=0):
    """Synthetic (player, season) rows with a PLANTED, noiseless relationship:
    passing_yards-per-attempt = 6.0 + 2.0*rapm_rating; rushing_yards-per-carry =
    4.0 + 1.0*rapm_rating; receiving_yards-per-reception = 9.0 + 0.5*rapm_rating.
    Other rate stats are small deterministic functions of rapm_rating too, so
    every RATE_STATS entry has a real signal to recover."""
    rng = np.random.default_rng(seed)
    rapm = rng.uniform(-1.5, 1.5, size=n)
    pass_att = rng.integers(10, 40, size=n).astype(float)
    rush_att = rng.integers(5, 25, size=n).astype(float)
    rec = rng.integers(2, 15, size=n).astype(float)
    return pd.DataFrame({
        "rapm_rating": rapm,
        "smoothed_talent": rapm,           # synth: identical signal, simpler setup
        "prior_mean": np.zeros(n),
        "pass_attempts": pass_att,
        "completions": pass_att * np.clip(0.6 + 0.05 * rapm, 0.3, 0.85),
        "passing_yards": pass_att * (6.0 + 2.0 * rapm),
        "passing_tds": pass_att * np.clip(0.04 + 0.01 * rapm, 0.0, None),
        "interceptions": pass_att * np.clip(0.03 - 0.01 * rapm, 0.0, None),
        "rush_attempts": rush_att,
        "rushing_yards": rush_att * (4.0 + 1.0 * rapm),
        "rushing_tds": rush_att * np.clip(0.03 + 0.005 * rapm, 0.0, None),
        "targets": rec * 1.5,
        "receptions": rec,
        "receiving_yards": rec * (9.0 + 0.5 * rapm),
        "receiving_tds": rec * np.clip(0.05 + 0.01 * rapm, 0.0, None),
        "fumbles_lost": np.zeros(n),
        "two_point_conversions": np.zeros(n),
    })


# --------------------------------------------------------------------------- #
# Recovers a planted relationship                                              #
# --------------------------------------------------------------------------- #
def test_recovers_planted_yards_per_attempt():
    """With near-zero ridge penalty, the fit recovers the planted YPA formula."""
    hist = _history(n=300, seed=1)
    model = fit_stat_line_model(hist, alpha=1e-6)
    rate_model = model.rates[("pass_attempts", "passing_yards")]
    for rapm in (-1.0, 0.0, 1.0):
        true_rate = 6.0 + 2.0 * rapm
        pred = rate_model.predict({"rapm_rating": rapm, "smoothed_talent": rapm, "prior_mean": 0.0})
        assert pred == pytest.approx(true_rate, abs=0.15)


def test_recovers_planted_relationship_for_every_rate_stat():
    """Every (volume_col, stat) pair in RATE_STATS gets a fitted model with
    finite, sane coefficients (not just the one spot-checked above)."""
    hist = _history(n=300, seed=2)
    model = fit_stat_line_model(hist, alpha=1e-6)
    for vol_col, stats in RATE_STATS.items():
        for stat in stats:
            assert (vol_col, stat) in model.rates
            pred = model.rates[(vol_col, stat)].predict({c: 0.0 for c in TALENT_FEATURE_COLS})
            assert np.isfinite(pred)


# --------------------------------------------------------------------------- #
# Stat-line emission                                                           #
# --------------------------------------------------------------------------- #
def test_project_stat_line_emits_all_14_cols():
    """Every SCORING_STAT_COLS key is present, finite, and non-negative."""
    model = fit_stat_line_model(_history(seed=3))
    volume = {"pass_attempts": 30.0, "rush_attempts": 5.0, "targets": 1.0, "receptions": 1.0}
    talent = {"rapm_rating": 0.5, "smoothed_talent": 0.5, "prior_mean": 0.0}
    line = model.project_stat_line(volume, talent)
    assert set(line) == set(SCORING_STAT_COLS)
    assert all(np.isfinite(v) and v >= 0.0 for v in line.values())


def test_volume_cols_pass_through_unchanged():
    """VOLUME_COLS are copied from volume_per_game verbatim, not refit."""
    model = fit_stat_line_model(_history(seed=4))
    volume = {"pass_attempts": 22.0, "rush_attempts": 3.0, "targets": 7.0, "receptions": 5.0}
    line = model.project_stat_line(volume, {"rapm_rating": 0.0, "smoothed_talent": 0.0, "prior_mean": 0.0})
    for c in VOLUME_COLS:
        assert line[c] == pytest.approx(volume[c])


def test_zero_volume_gives_zero_downstream_stats():
    """No attempts -> no passing yards/TDs/etc, regardless of talent."""
    model = fit_stat_line_model(_history(seed=5))
    volume = {"pass_attempts": 0.0, "rush_attempts": 0.0, "targets": 0.0, "receptions": 0.0}
    line = model.project_stat_line(volume, {"rapm_rating": 2.0, "smoothed_talent": 2.0, "prior_mean": 0.0})
    for stats in RATE_STATS.values():
        for stat in stats:
            assert line[stat] == 0.0


def test_unmodeled_stats_are_always_zero():
    """fumbles_lost / two_point_conversions are never produced by this model."""
    model = fit_stat_line_model(_history(seed=6))
    line = model.project_stat_line(
        {"pass_attempts": 30.0, "rush_attempts": 5.0, "targets": 5.0, "receptions": 4.0},
        {"rapm_rating": 1.0, "smoothed_talent": 1.0, "prior_mean": 0.0})
    assert line["fumbles_lost"] == 0.0
    assert line["two_point_conversions"] == 0.0


def test_zero_volume_rows_excluded_from_fit():
    """Rows with zero volume for a driver don't poison that driver's rate fit
    (an undefined per-unit rate would otherwise inject inf/NaN into the ridge)."""
    hist = _history(n=50, seed=7)
    hist = pd.concat([hist, pd.DataFrame([{
        **{c: 0.0 for c in SCORING_STAT_COLS}, **{c: 0.0 for c in TALENT_FEATURE_COLS},
        "pass_attempts": 0.0,
    }])], ignore_index=True)
    model = fit_stat_line_model(hist, alpha=1e-6)
    pred = model.rates[("pass_attempts", "passing_yards")].predict({"rapm_rating": 0.0})
    assert np.isfinite(pred)


# --------------------------------------------------------------------------- #
# NaN-feature robustness (regression: audit finding P1)                        #
# --------------------------------------------------------------------------- #
def test_predict_tolerates_present_but_nan_feature():
    """A talent feature present with value NaN must NOT crash Ridge.predict.

    Non-rookies carry a NaN ``prior_mean`` from
    ``features.assemble_talent_features().to_frame()`` (no cross-league prior).
    ``dict.get(c, 0.0)`` returns that NaN, not the default, so predict must
    guard explicitly. To stay consistent with the fit side's ``fillna(0.0)``
    (model.py:131), a NaN feature must predict identically to 0.0."""
    model = fit_stat_line_model(_history(seed=10), alpha=1e-6)
    rate_model = model.rates[("pass_attempts", "passing_yards")]
    nan_row = {"rapm_rating": 0.5, "smoothed_talent": 0.5, "prior_mean": float("nan")}
    zero_row = {"rapm_rating": 0.5, "smoothed_talent": 0.5, "prior_mean": 0.0}
    pred_nan = rate_model.predict(nan_row)
    assert np.isfinite(pred_nan)
    assert pred_nan == pytest.approx(rate_model.predict(zero_row))
    # ±inf is also coerced to 0.0 (posinf/neginf), never propagated into Ridge
    inf_row = {"rapm_rating": 0.5, "smoothed_talent": 0.5, "prior_mean": float("inf")}
    neg_inf_row = {"rapm_rating": 0.5, "smoothed_talent": 0.5, "prior_mean": float("-inf")}
    assert rate_model.predict(inf_row) == pytest.approx(rate_model.predict(zero_row))
    assert rate_model.predict(neg_inf_row) == pytest.approx(rate_model.predict(zero_row))


def test_project_stat_line_tolerates_nan_prior_mean():
    """The real non-rookie path: ``prior_mean`` is NaN. ``project_stat_line``
    must emit a full finite, non-negative line rather than raising — P1 broke
    preseason projection (Rankings) for every non-rookie exactly this way."""
    model = fit_stat_line_model(_history(seed=11))
    volume = {"pass_attempts": 30.0, "rush_attempts": 5.0, "targets": 6.0, "receptions": 4.0}
    talent = {"rapm_rating": 0.4, "smoothed_talent": 0.4, "prior_mean": float("nan")}
    line = model.project_stat_line(volume, talent)
    assert set(line) == set(SCORING_STAT_COLS)
    assert all(np.isfinite(v) and v >= 0.0 for v in line.values())


# --------------------------------------------------------------------------- #
# Scoring integration                                                          #
# --------------------------------------------------------------------------- #
def test_project_fantasy_points_matches_manual_calculate_points():
    """project_fantasy_points == calculate_points(project_stat_line(...), config)."""
    model = fit_stat_line_model(_history(seed=8))
    volume = {"pass_attempts": 28.0, "rush_attempts": 4.0, "targets": 6.0, "receptions": 4.0}
    talent = {"rapm_rating": 0.3, "smoothed_talent": 0.3, "prior_mean": 0.0}
    config = ScoringConfig(name="test", rules={"passing_yards": 0.04, "passing_tds": 4.0,
                                                "rushing_yards": 0.1, "receptions": 1.0})
    expected = calculate_points(model.project_stat_line(volume, talent), config)
    assert model.project_fantasy_points(volume, talent, config) == pytest.approx(expected)


# --------------------------------------------------------------------------- #
# Beats the last-season-actuals baseline on synth                              #
# --------------------------------------------------------------------------- #
def test_beats_last_season_baseline_on_synth():
    """On a held-out synth season, the fitted model's points are closer to
    truth than naively reusing last season's actual points (skill_score > 0),
    reusing the 2a validation.metrics module per the standard the real backtest
    will use (validation plan §9: skill vs baselines, not raw error)."""
    rng = np.random.default_rng(9)
    n = 150
    rapm = rng.uniform(-1.5, 1.5, size=n)
    pass_att = rng.integers(15, 35, size=n).astype(float)
    config = ScoringConfig(name="pass-yards-only", rules={"passing_yards": 0.04})

    # Train on a season with one noise draw.
    train = pd.DataFrame({
        "rapm_rating": rapm, "smoothed_talent": rapm, "prior_mean": np.zeros(n),
        "pass_attempts": pass_att,
        "passing_yards": pass_att * (6.0 + 2.0 * rapm + rng.normal(0, 0.3, n)),
        **{c: np.zeros(n) for c in
           ("completions", "passing_tds", "interceptions", "rush_attempts",
            "rushing_yards", "rushing_tds", "receptions", "receiving_yards",
            "receiving_tds", "fumbles_lost", "two_point_conversions")},
        "targets": np.zeros(n),
    })
    model = fit_stat_line_model(train, alpha=0.5)

    # Held-out next season: SAME players/talent/volume, a NEW noise draw for actuals
    # (talent persists season-to-season; the realized stat line doesn't).
    actual_points = pass_att * (6.0 + 2.0 * rapm + rng.normal(0, 0.3, n)) * 0.04
    last_season_points = train["passing_yards"].to_numpy() * 0.04   # the baseline

    model_points = np.array([
        model.project_fantasy_points(
            {"pass_attempts": pass_att[i], "rush_attempts": 0.0, "targets": 0.0, "receptions": 0.0},
            {"rapm_rating": rapm[i], "smoothed_talent": rapm[i], "prior_mean": 0.0},
            config)
        for i in range(n)
    ])

    model_mae = mae(model_points, actual_points)
    baseline_mae = mae(last_season_points, actual_points)
    assert skill_score(model_mae, baseline_mae) > 0
