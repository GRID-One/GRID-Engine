"""Tests for the backtest metrics (Phase 2a; validation plan §7, §9).

Closed-form / known-value checks — no data dependency, deterministic.
"""
import numpy as np
import pytest

from backend.validation import metrics as M


# --------------------------------------------------------------------------- #
# Normal helpers                                                               #
# --------------------------------------------------------------------------- #
def test_normal_cdf_known_points():
    """Φ(0)=0.5; Φ(±1.96)≈0.025/0.975 (the 95% interval edges)."""
    assert M.normal_cdf(0.0) == pytest.approx(0.5, abs=1e-9)
    lo, hi = M.normal_cdf(np.array([-1.959964, 1.959964]))
    assert lo == pytest.approx(0.025, abs=1e-4)
    assert hi == pytest.approx(0.975, abs=1e-4)


def test_ppf_is_cdf_inverse():
    """normal_ppf round-trips normal_cdf across the central + tail regions."""
    for p in (0.001, 0.02425, 0.1, 0.5, 0.9, 0.97575, 0.999):
        assert M.normal_cdf(M.normal_ppf(p)) == pytest.approx(p, abs=1e-6)


def test_ppf_rejects_out_of_range():
    """p outside (0,1) has no finite quantile."""
    with pytest.raises(ValueError):
        M.normal_ppf(0.0)


# --------------------------------------------------------------------------- #
# Accuracy                                                                     #
# --------------------------------------------------------------------------- #
def test_accuracy_basics():
    """MAE/RMSE/bias on a hand-computable example."""
    pred = [1.0, 2.0, 3.0]
    actual = [1.0, 0.0, 6.0]            # errors: 0, +2, -3
    assert M.mae(pred, actual) == pytest.approx((0 + 2 + 3) / 3)
    assert M.rmse(pred, actual) == pytest.approx(np.sqrt((0 + 4 + 9) / 3))
    assert M.bias(pred, actual) == pytest.approx((0 + 2 - 3) / 3)


def test_skill_score_sign_and_guard():
    """Skill >0 when model beats ref, <0 when worse, 0 when ref has no signal."""
    assert M.skill_score(0.5, 1.0) == pytest.approx(0.5)   # half the error -> +0.5
    assert M.skill_score(2.0, 1.0) == pytest.approx(-1.0)  # twice the error -> negative
    assert M.skill_score(1.0, 0.0) == 0.0                  # nothing to beat


# --------------------------------------------------------------------------- #
# Uncertainty (bootstrap)                                                       #
# --------------------------------------------------------------------------- #
def test_bootstrap_ci_covers_known_mean():
    """A 95% percentile CI brackets the true mean of a large calibrated sample."""
    rng = np.random.default_rng(0)
    samples = rng.normal(5.0, 2.0, size=2000)
    ci = M.bootstrap_ci(samples, n=2000, alpha=0.05, seed=1)
    assert ci.lower < 5.0 < ci.upper
    assert ci.point == pytest.approx(float(np.mean(samples)))


def test_bootstrap_ci_excludes_zero_for_clearly_positive():
    """A sample far from 0 (tight spread) yields a CI strictly above 0."""
    rng = np.random.default_rng(2)
    samples = rng.normal(3.0, 0.5, size=500)
    ci = M.bootstrap_ci(samples, n=2000, seed=3)
    assert ci.lower > 0.0
    assert ci.excludes_zero is True


def test_bootstrap_ci_includes_zero_for_centered_sample():
    """A sample centered on 0 straddles it -> excludes_zero is False."""
    rng = np.random.default_rng(4)
    samples = rng.normal(0.0, 1.0, size=500)
    ci = M.bootstrap_ci(samples, n=2000, seed=5)
    assert ci.lower < 0.0 < ci.upper
    assert ci.excludes_zero is False


def test_bootstrap_ci_deterministic_under_seed():
    """Same seed -> identical interval (reproducible synth gate)."""
    samples = [1.0, -2.0, 3.0, 0.5, -1.0, 4.0]
    a = M.bootstrap_ci(samples, n=1000, seed=7)
    b = M.bootstrap_ci(samples, n=1000, seed=7)
    assert a == b


def test_bootstrap_ci_custom_statistic():
    """A non-default statistic (median) is resampled and bracketed."""
    rng = np.random.default_rng(8)
    samples = rng.normal(10.0, 1.0, size=1000)
    ci = M.bootstrap_ci(samples, n=1000, seed=9, statistic=np.median)
    assert ci.lower < 10.0 < ci.upper


def test_bootstrap_ci_rejects_empty_and_bad_alpha():
    """Empty samples and out-of-range alpha are rejected."""
    with pytest.raises(ValueError):
        M.bootstrap_ci([], n=100)
    with pytest.raises(ValueError):
        M.bootstrap_ci([1.0, 2.0], alpha=0.0)


# --------------------------------------------------------------------------- #
# Probabilistic calibration                                                    #
# --------------------------------------------------------------------------- #
def test_nis_unit_for_standard_innovations():
    """Innovations drawn at exactly ±1 sd give mean(z²)=1 (perfect consistency)."""
    actual = np.array([1.0, -1.0, 1.0, -1.0])
    mean = np.zeros(4)
    var = np.ones(4)
    assert M.nis(actual, mean, var) == pytest.approx(1.0)


def test_nis_flags_overconfidence():
    """Variance too small (actual spread wider than claimed) pushes NIS above 1."""
    actual = np.array([2.0, -2.0, 2.0, -2.0])
    assert M.nis(actual, np.zeros(4), np.ones(4)) == pytest.approx(4.0)


def test_pit_uniform_for_calibrated_normal():
    """A large in-distribution sample yields a near-uniform PIT (mean .5, std≈.289)."""
    rng = np.random.default_rng(0)
    actual = rng.normal(3.0, 2.0, size=20000)
    u = M.pit(actual, np.full_like(actual, 3.0), np.full_like(actual, 2.0))
    assert u.mean() == pytest.approx(0.5, abs=0.02)
    assert u.std() == pytest.approx(1 / np.sqrt(12), abs=0.02)


def test_crps_collapses_to_mae_at_zero_spread():
    """As sd→0 a Gaussian CRPS → |actual−mean| (the deterministic limit)."""
    actual = np.array([1.0, 4.0])
    mean = np.array([0.0, 0.0])
    sd = np.array([1e-6, 1e-6])
    assert M.crps_gaussian(actual, mean, sd) == pytest.approx((1.0 + 4.0) / 2, abs=1e-3)


def test_crps_skill_prefers_sharper_correct_forecast():
    """A correct, sharp forecast has lower CRPS than a diffuse one -> positive skill."""
    rng = np.random.default_rng(1)
    actual = rng.normal(0.0, 1.0, size=5000)
    sharp = M.crps_gaussian(actual, np.zeros_like(actual), np.ones_like(actual))
    diffuse = M.crps_gaussian(actual, np.zeros_like(actual), np.full_like(actual, 4.0))
    assert M.skill_score(sharp, diffuse) > 0


def test_picp_matches_nominal_level():
    """An 80% interval covers ~80% of a calibrated sample."""
    rng = np.random.default_rng(2)
    actual = rng.normal(0.0, 1.0, size=20000)
    cov = M.picp(actual, np.zeros_like(actual), np.ones_like(actual), level=0.80)
    assert cov == pytest.approx(0.80, abs=0.02)


def test_pinaw_sharper_is_smaller():
    """Tighter sd -> smaller normalized interval width."""
    actual = np.linspace(-3, 3, 100)
    mean = np.zeros_like(actual)
    wide = M.pinaw(actual, mean, np.full_like(actual, 2.0))
    narrow = M.pinaw(actual, mean, np.full_like(actual, 0.5))
    assert narrow < wide


def test_pinball_zero_for_perfect_quantile():
    """A quantile forecast equal to the actuals has zero pinball loss."""
    actual = np.array([1.0, 2.0, 3.0])
    assert M.pinball_loss(actual, actual, q=0.2) == pytest.approx(0.0)


def test_pinball_asymmetry():
    """Low-q under-prediction is penalized less than over-prediction (the floor lever)."""
    actual = np.array([10.0, 10.0])
    under = M.pinball_loss(actual, np.array([8.0, 8.0]), q=0.2)   # pred below actual
    over = M.pinball_loss(actual, np.array([12.0, 12.0]), q=0.2)  # pred above actual
    assert under < over


# --------------------------------------------------------------------------- #
# Ranking                                                                      #
# --------------------------------------------------------------------------- #
def test_spearman_monotone_and_reversed():
    """Monotone agreement -> +1; exact reversal -> −1."""
    x = np.arange(10.0)
    assert M.spearman(x, 2 * x + 1) == pytest.approx(1.0)
    assert M.spearman(x, -x) == pytest.approx(-1.0)


def test_spearman_tie_aware():
    """Tied values get average ranks, matching the standard Spearman definition."""
    # actual has a tie at the top two; a pred that orders them perfectly aside
    # from the tie should still be near +1 (ties contribute no rank disagreement).
    pred = np.array([4.0, 3.0, 2.0, 1.0])
    actual = np.array([9.0, 9.0, 5.0, 1.0])   # tie between the first two
    # average-rank Spearman is well-defined and high here; ordinal ranking would
    # arbitrarily break the tie and could understate it.
    assert M.spearman(pred, actual) == pytest.approx(0.9486833, abs=1e-6)


def test_top_n_hit_rate():
    """Predicted top-2 overlapping the true top-2 in one of two slots -> 0.5."""
    pred = [9, 8, 1, 0]      # predicted order: idx 0,1 on top
    actual = [9, 1, 8, 0]    # true top-2: idx 0,2
    assert M.top_n_hit_rate(pred, actual, n=2) == pytest.approx(0.5)


def test_ndcg_perfect_and_degraded():
    """Perfect ranking -> NDCG 1.0; a worse ranking scores strictly lower."""
    actual = np.array([3.0, 2.0, 1.0, 0.0])
    assert M.ndcg_at_k(actual, actual, k=4) == pytest.approx(1.0)
    worse = M.ndcg_at_k(np.array([0.0, 1.0, 2.0, 3.0]), actual, k=4)
    assert worse < 1.0
