"""Tests for priors expansion: age adjustment, draft capital, league factors."""
import numpy as np
import pandas as pd
import pytest

from backend.grid.priors import (
    LEAGUE_FACTORS,
    _age_adjustment,
    _draft_capital_adjustment,
    build_priors,
    estimate_equivalency,
)


# ─── age adjustment ───────────────────────────────────────────────────────────

def test_age_under_24_positive():
    assert _age_adjustment(21) == pytest.approx(0.05)
    assert _age_adjustment(23) == pytest.approx(0.05)


def test_age_prime_zero():
    for age in range(24, 31):
        assert _age_adjustment(age) == 0.0


def test_age_over_30_negative():
    assert _age_adjustment(31) == pytest.approx(-0.05)
    assert _age_adjustment(35) == pytest.approx(-0.05)


# ─── draft capital adjustment ────────────────────────────────────────────────

def test_draft_round_1_highest():
    r1 = _draft_capital_adjustment(1)
    r2 = _draft_capital_adjustment(2)
    r4 = _draft_capital_adjustment(4)
    assert r1 > r2 > r4


def test_draft_round_2_3():
    assert _draft_capital_adjustment(2) == pytest.approx(0.03)
    assert _draft_capital_adjustment(3) == pytest.approx(0.03)


def test_undrafted_negative():
    assert _draft_capital_adjustment(0) == pytest.approx(-0.03)


def test_draft_round_none_zero():
    assert _draft_capital_adjustment(None) == 0.0


# ─── league factors ──────────────────────────────────────────────────────────

def test_fcs_factor_less_than_fbs():
    assert LEAGUE_FACTORS["FCS"] < LEAGUE_FACTORS["FBS"]


def test_ufl_weakest_factor():
    assert LEAGUE_FACTORS["UFL"] <= min(
        v for k, v in LEAGUE_FACTORS.items() if k != "unknown"
    )


# ─── build_priors integration ────────────────────────────────────────────────

def _make_college_df(n=10, seed=42):
    rng = np.random.default_rng(seed)
    return pd.DataFrame({
        "player_id": [f"P{i}" for i in range(n)],
        "position": (["QB", "RB", "WR", "TE"] * 3)[:n],
        "is_rookie": [True] * n,
        "feeder_sv": rng.normal(0, 0.2, n),
        "age": [21, 22, 23, 24, 25, 26, 27, 28, 31, 33][:n],
        "draft_round": [1, 2, 3, 4, 5, 6, 0, None, 1, 2][:n],
    })


def test_build_priors_with_age_col():
    df = _make_college_df()
    equiv = dict(slope=0.5, intercept=0.0, oos_r2=0.5, factor_check=0.35,
                 n_shared=50, league_type="FBS", league_factor=0.35)
    priors_no_age = build_priors(df, equiv)
    priors_age = build_priors(df, equiv, age_col="age")
    # Young players (age < 24) should get a higher prior_mean
    young_mask = df.age < 24
    delta = (priors_age.loc[young_mask, "prior_mean"].values
             - priors_no_age.loc[young_mask, "prior_mean"].values)
    assert np.all(delta == pytest.approx(0.05))


def test_build_priors_with_draft_round_col():
    df = _make_college_df()
    equiv = dict(slope=0.5, intercept=0.0, oos_r2=0.5, factor_check=0.35,
                 n_shared=50, league_type="FBS", league_factor=0.35)
    priors_no_dr = build_priors(df, equiv)
    priors_dr = build_priors(df, equiv, draft_round_col="draft_round")
    # Round-1 picks should exceed baseline
    r1_mask = df.draft_round == 1
    delta = (priors_dr.loc[r1_mask, "prior_mean"].values
             - priors_no_dr.loc[r1_mask, "prior_mean"].values)
    assert np.all(delta == pytest.approx(0.08))


def test_build_priors_missing_col_ignored():
    df = _make_college_df()
    equiv = dict(slope=0.5, intercept=0.0, oos_r2=0.5, factor_check=0.35,
                 n_shared=50, league_type="FBS", league_factor=0.35)
    # Passing a non-existent column should not raise — just skip
    priors = build_priors(df, equiv, age_col="nonexistent_col")
    assert "prior_mean" in priors.columns


# ─── estimate_equivalency: real-data unblock (no planted `ability`) ───────────

def _shared_college_and_ratings(n=60, seed=7, with_ability=False, slope=0.4, intercept=0.1):
    """Non-rookie feeder players with a planted feeder_sv -> NFL rating line."""
    rng = np.random.default_rng(seed)
    feeder_sv = rng.normal(0, 0.3, n)
    rating = intercept + slope * feeder_sv + rng.normal(0, 0.02, n)
    college = pd.DataFrame({
        "player_id": [f"P{i}" for i in range(n)],
        "position": (["QB", "RB", "WR", "TE"] * n)[:n],
        "is_rookie": [False] * n,
        "feeder_sv": feeder_sv,
    })
    cols = {"player_id": college.player_id, "rating": rating}
    if with_ability:
        cols["ability"] = feeder_sv / max(slope, 1e-9)   # planted true ability
    return college, pd.DataFrame(cols)


def test_estimate_equivalency_real_data_no_ability():
    """The real-data unblock: with NO planted `ability` column (RAPM/Kalman
    ratings as the target), the regression still recovers slope/intercept and
    factor_check is None (its only use is the synth sanity check)."""
    college, ratings = _shared_college_and_ratings(with_ability=False,
                                                   slope=0.4, intercept=0.1)
    equiv = estimate_equivalency(college, ratings, league_type="FBS")
    assert equiv["slope"] == pytest.approx(0.4, abs=0.05)
    assert equiv["intercept"] == pytest.approx(0.1, abs=0.05)
    assert equiv["factor_check"] is None
    assert equiv["league_factor"] == LEAGUE_FACTORS["FBS"]
    assert np.isfinite(equiv["oos_r2"])


def test_estimate_equivalency_synth_path_unchanged():
    """With `ability` present (synth), factor_check is still computed — the
    change is backward compatible."""
    college, ratings = _shared_college_and_ratings(with_ability=True)
    equiv = estimate_equivalency(college, ratings)
    assert equiv["factor_check"] is not None
    assert np.isfinite(equiv["factor_check"])


def test_estimate_equivalency_feeds_build_priors_on_real_data():
    """End-to-end real-data path: equivalency (no ability) -> build_priors gives
    rookies an informative prior_mean monotonic in feeder_sv."""
    college, ratings = _shared_college_and_ratings(with_ability=False,
                                                   slope=0.4, intercept=0.1)
    equiv = estimate_equivalency(college, ratings)
    rng = np.random.default_rng(1)
    rookies = pd.DataFrame({
        "player_id": [f"R{i}" for i in range(12)],
        "position": (["QB", "RB", "WR", "TE"] * 3),
        "is_rookie": [True] * 12,
        "feeder_sv": np.linspace(-0.4, 0.4, 12),
    })
    priors = build_priors(rookies, equiv)
    # positive equivalency slope -> prior_mean increases with feeder_sv
    assert priors["prior_mean"].is_monotonic_increasing
    assert (priors["prior_var"] > 0).all()
