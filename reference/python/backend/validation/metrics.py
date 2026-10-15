"""Backtest metrics (roadmap Phase 2a; validation plan §7, §9, §11).

Pure, dependency-light (numpy only) scoring functions shared by every later
walk-forward backtest and calibration report.  Three families:

* **Accuracy** — MAE / RMSE / bias, and a relative **skill** score vs a baseline.
* **Probabilistic calibration** — the honest one-step predictive N(mean, S) the
  Kalman now surfaces is scored with **NIS** (filter consistency), **PIT**
  (uniformity), **CRPS** (closed form for a Gaussian — no sampling), **PICP +
  PINAW** (coverage *paired* with sharpness — never coverage alone, per §7), and
  **pinball** loss at a quantile (decision-tail calibration).
* **Ranking** — Spearman, top-N hit rate, NDCG@k (what people draft/start).

These consolidate the inline helpers in ``tests/grid/test_calibration_synth.py``
(``_normal_cdf`` + the ``mean(z**2)`` NIS) into one reusable module.  Everything
is closed-form and deterministic so the synth calibration gates stay in the CI
budget (§7.5 A).
"""
from __future__ import annotations

from math import erf, sqrt
from typing import Callable, NamedTuple

import numpy as np

_SQRT2 = sqrt(2.0)
_INV_SQRT_PI = 1.0 / sqrt(np.pi)
_SQRT_2PI = sqrt(2.0 * np.pi)


# --------------------------------------------------------------------------- #
# Normal helpers (no scipy)                                                    #
# --------------------------------------------------------------------------- #
def normal_cdf(z):
    """Standard-normal CDF Φ(z), vectorized over an array (PIT transform)."""
    z = np.asarray(z, dtype=float)
    return 0.5 * (1.0 + np.vectorize(lambda x: erf(x / _SQRT2))(z))


def _normal_pdf(z):
    """Standard-normal PDF φ(z)."""
    z = np.asarray(z, dtype=float)
    return np.exp(-0.5 * z * z) / _SQRT_2PI


# Acklam's rational approximation to the inverse normal CDF (|err| < 1.15e-9).
_A = (-3.969683028665376e+01, 2.209460984245205e+02, -2.759285104469687e+02,
      1.383577518672690e+02, -3.066479806614716e+01, 2.506628277459239e+00)
_B = (-5.447609879822406e+01, 1.615858368580409e+02, -1.556989798598866e+02,
      6.680131188771972e+01, -1.328068155288572e+01)
_C = (-7.784894002430293e-03, -3.223964580411365e-01, -2.400758277161838e+00,
      -2.549732539343734e+00, 4.374664141464968e+00, 2.938163982698783e+00)
_D = (7.784695709041462e-03, 3.224671290700398e-01, 2.445134137142996e+00,
      3.754408661907416e+00)


def _ppf_scalar(p: float) -> float:
    """Inverse standard-normal CDF for a single probability in (0, 1)."""
    if not 0.0 < p < 1.0:
        raise ValueError(f"normal_ppf requires 0<p<1, got {p}")
    plow, phigh = 0.02425, 1 - 0.02425
    if p < plow:                                  # lower tail
        q = sqrt(-2.0 * np.log(p))
        return (((((_C[0] * q + _C[1]) * q + _C[2]) * q + _C[3]) * q + _C[4]) * q + _C[5]) / \
               ((((_D[0] * q + _D[1]) * q + _D[2]) * q + _D[3]) * q + 1.0)
    if p > phigh:                                 # upper tail
        q = sqrt(-2.0 * np.log(1.0 - p))
        return -(((((_C[0] * q + _C[1]) * q + _C[2]) * q + _C[3]) * q + _C[4]) * q + _C[5]) / \
               ((((_D[0] * q + _D[1]) * q + _D[2]) * q + _D[3]) * q + 1.0)
    q = p - 0.5                                   # central region
    r = q * q
    return (((((_A[0] * r + _A[1]) * r + _A[2]) * r + _A[3]) * r + _A[4]) * r + _A[5]) * q / \
           (((((_B[0] * r + _B[1]) * r + _B[2]) * r + _B[3]) * r + _B[4]) * r + 1.0)


def normal_ppf(p):
    """Inverse standard-normal CDF Φ⁻¹(p) (scalar or array), Acklam approximation."""
    if np.isscalar(p):
        return _ppf_scalar(float(p))
    return np.array([_ppf_scalar(float(x)) for x in np.asarray(p, dtype=float)])


# --------------------------------------------------------------------------- #
# Accuracy                                                                     #
# --------------------------------------------------------------------------- #
def mae(pred, actual) -> float:
    """Mean absolute error."""
    pred, actual = np.asarray(pred, float), np.asarray(actual, float)
    return float(np.mean(np.abs(pred - actual)))


def rmse(pred, actual) -> float:
    """Root mean squared error."""
    pred, actual = np.asarray(pred, float), np.asarray(actual, float)
    return float(np.sqrt(np.mean((pred - actual) ** 2)))


def bias(pred, actual) -> float:
    """Mean signed error (pred − actual): positive = systematic over-prediction."""
    pred, actual = np.asarray(pred, float), np.asarray(actual, float)
    return float(np.mean(pred - actual))


def skill_score(model_score: float, ref_score: float) -> float:
    """Relative skill ``1 − model/ref`` (>0 = model beats the reference).

    Works for any *lower-is-better* score (MSE, MAE, CRPS): pass the model's
    score and the baseline's.  Returns ``0.0`` when the reference score is 0
    (no signal to beat).
    """
    if ref_score == 0:
        return 0.0
    return 1.0 - model_score / ref_score


# --------------------------------------------------------------------------- #
# Uncertainty (bootstrap)                                                      #
# --------------------------------------------------------------------------- #
class BootstrapCI(NamedTuple):
    """A percentile bootstrap confidence interval and its ``excludes 0`` verdict.

    ``point`` is the statistic on the observed sample; ``lower``/``upper`` are the
    percentile-CI bounds; ``excludes_zero`` is the H1/H2 gate — every headline is
    stated as "> 0, CI excludes 0", so the whole interval must sit on one side of
    zero for a directional claim to survive.
    """

    point: float
    lower: float
    upper: float
    excludes_zero: bool


def bootstrap_ci(
    samples,
    *,
    n: int = 10000,
    alpha: float = 0.05,
    seed: int = 0,
    statistic: Callable | None = None,
) -> BootstrapCI:
    """Percentile bootstrap CI for a sample statistic (default: the mean).

    Resamples ``samples`` with replacement ``n`` times, recomputes ``statistic``
    on each resample, and returns the central ``1−alpha`` percentile interval
    plus whether it **excludes 0** (the directional-KPI gate used for H1/H2).
    Deterministic given ``seed`` (``numpy.random.default_rng``) so the synth gate
    is reproducible.  A per-sample paired margin (GRID − baseline) is the intended
    input: its CI excluding 0 is exactly "GRID beats baseline, significantly".

    Args:
        samples: 1-D array of per-unit observations (e.g. per-week point margins).
        n: number of bootstrap resamples.
        alpha: two-sided miscoverage; ``alpha=0.05`` → a 95% CI.
        seed: RNG seed for reproducibility.
        statistic: callable applied to each resample; defaults to :func:`numpy.mean`.
    """
    samples = np.asarray(samples, dtype=float)
    if samples.size == 0:
        raise ValueError("bootstrap_ci requires at least one sample")
    if not 0.0 < alpha < 1.0:
        raise ValueError(f"bootstrap_ci requires 0<alpha<1, got {alpha}")
    if n < 1:
        raise ValueError(f"bootstrap_ci requires n>=1, got {n}")

    stat = statistic if statistic is not None else np.mean
    point = float(stat(samples))

    rng = np.random.default_rng(seed)
    idx = rng.integers(0, samples.size, size=(n, samples.size))
    resampled = samples[idx]
    if statistic is None:
        boot = resampled.mean(axis=1)
    else:
        boot = np.array([float(statistic(row)) for row in resampled])

    lower = float(np.quantile(boot, alpha / 2.0))
    upper = float(np.quantile(boot, 1.0 - alpha / 2.0))
    excludes_zero = lower > 0.0 or upper < 0.0
    return BootstrapCI(point=point, lower=lower, upper=upper, excludes_zero=excludes_zero)


# --------------------------------------------------------------------------- #
# Probabilistic calibration                                                    #
# --------------------------------------------------------------------------- #
def standardized_innovations(actual, mean, sd):
    """z = (actual − mean) / sd — the standardized one-step predictive errors."""
    actual, mean, sd = np.asarray(actual, float), np.asarray(mean, float), np.asarray(sd, float)
    return (actual - mean) / sd


def nis(actual, mean, var) -> float:
    """Normalized innovation squared, ``mean(z²)`` with z=(actual−mean)/√var.

    A consistent filter has NIS ≈ 1: >1 overconfident (variance too small), <1
    underconfident.  ``var`` is the predictive variance S (R included), not the
    R-omitted filtered variance.
    """
    actual, mean, var = np.asarray(actual, float), np.asarray(mean, float), np.asarray(var, float)
    z = (actual - mean) / np.sqrt(var)
    return float(np.mean(z ** 2))


def pit(actual, mean, sd):
    """Probability integral transform u = Φ((actual−mean)/sd); ~Uniform(0,1) if calibrated."""
    return normal_cdf(standardized_innovations(actual, mean, sd))


def crps_gaussian(actual, mean, sd) -> float:
    """Mean CRPS of a Gaussian forecast N(mean, sd²) vs realized ``actual``.

    Closed form (Gneiting & Raftery 2007), no sampling:
    ``CRPS = sd·[ z(2Φ(z)−1) + 2φ(z) − 1/√π ]``, z=(actual−mean)/sd.
    Lower is better; feed to :func:`skill_score` for CRPS skill vs a baseline
    that emits a spread.
    """
    actual, mean, sd = np.asarray(actual, float), np.asarray(mean, float), np.asarray(sd, float)
    z = (actual - mean) / sd
    crps = sd * (z * (2.0 * normal_cdf(z) - 1.0) + 2.0 * _normal_pdf(z) - _INV_SQRT_PI)
    return float(np.mean(crps))


def picp(actual, mean, sd, level: float = 0.80) -> float:
    """Prediction-interval coverage probability at ``level`` for a Gaussian forecast.

    Fraction of ``actual`` falling within the central ``level`` interval
    [mean ± z·sd].  Report *paired* with :func:`pinaw` — coverage alone is
    gameable by widening the marginal (§7).
    """
    actual, mean, sd = np.asarray(actual, float), np.asarray(mean, float), np.asarray(sd, float)
    half = normal_ppf((1.0 + level) / 2.0) * sd
    inside = (actual >= mean - half) & (actual <= mean + half)
    return float(np.mean(inside))


def pinaw(actual, mean, sd, level: float = 0.80) -> float:
    """Prediction-interval normalized average width (sharpness) at ``level``.

    Mean interval width ÷ range of ``actual``; the denominator normalizes across
    targets so PINAW is comparable.  Lower = sharper at equal coverage.  ``mean``
    is unused (the width depends only on ``sd``) but kept in the signature so this
    mirrors :func:`picp` exactly — the two are reported as a pair, so a caller
    copying the ``picp(actual, mean, sd)`` call shape gets the right argument
    order here too.
    """
    sd, actual = np.asarray(sd, float), np.asarray(actual, float)
    width = 2.0 * normal_ppf((1.0 + level) / 2.0) * sd
    rng = float(np.max(actual) - np.min(actual))
    if rng == 0:
        return 0.0
    return float(np.mean(width) / rng)


def pinball_loss(actual, quantile_pred, q: float) -> float:
    """Mean pinball (quantile) loss for the ``q``-quantile forecast.

    ``q∈(0,1)``; ties calibration to the decision tail — injury-replacement cares
    about the ceiling (high q), bench-depth about the floor (low q).
    """
    actual, quantile_pred = np.asarray(actual, float), np.asarray(quantile_pred, float)
    diff = actual - quantile_pred
    return float(np.mean(np.maximum(q * diff, (q - 1.0) * diff)))


# --------------------------------------------------------------------------- #
# Ranking                                                                      #
# --------------------------------------------------------------------------- #
def _avg_rank(a):
    """Tie-aware ranks: tied values share their average rank (standard Spearman)."""
    order = np.argsort(a, kind="mergesort")
    sorted_a = a[order]
    ranks = np.empty(len(a), dtype=float)
    i = 0
    while i < len(a):
        j = i
        while j + 1 < len(a) and sorted_a[j + 1] == sorted_a[i]:
            j += 1
        ranks[order[i:j + 1]] = (i + j) / 2.0
        i = j + 1
    return ranks


def spearman(pred, actual) -> float:
    """Spearman rank correlation between predicted and realized values.

    Tie-aware: tied values get their average rank, so the standard definition
    holds on ranking targets with ties (e.g. multiple players at 0 points).
    """
    pred, actual = np.asarray(pred, float), np.asarray(actual, float)
    if len(pred) < 2:
        return float("nan")
    rp = _avg_rank(pred)
    ra = _avg_rank(actual)
    rp -= rp.mean()
    ra -= ra.mean()
    denom = np.sqrt(np.sum(rp ** 2) * np.sum(ra ** 2))
    if denom == 0:
        return 0.0
    return float(np.sum(rp * ra) / denom)


def top_n_hit_rate(pred, actual, n: int) -> float:
    """Fraction of the true top-``n`` (by ``actual``) recovered in the predicted top-``n``."""
    pred, actual = np.asarray(pred, float), np.asarray(actual, float)
    n = min(n, len(pred))
    if n == 0:
        return 0.0
    pred_top = set(np.argsort(pred)[::-1][:n].tolist())
    true_top = set(np.argsort(actual)[::-1][:n].tolist())
    return len(pred_top & true_top) / n


def ndcg_at_k(pred, actual, k: int) -> float:
    """Normalized discounted cumulative gain at ``k`` (relevance = realized value).

    Ranks items by ``pred``, sums realized ``actual`` discounted by log-rank, and
    normalizes by the ideal ordering.  Rewards putting genuinely high-value
    players near the top.  Assumes non-negative relevance.
    """
    pred, actual = np.asarray(pred, float), np.asarray(actual, float)
    k = min(k, len(pred))
    if k == 0:
        return 0.0
    order = np.argsort(pred)[::-1][:k]
    discounts = 1.0 / np.log2(np.arange(2, k + 2))
    dcg = float(np.sum(actual[order] * discounts))
    ideal = np.sort(actual)[::-1][:k]
    idcg = float(np.sum(ideal * discounts))
    if idcg == 0:
        return 0.0
    return dcg / idcg
