"""Tier 0.5 — synth calibration (roadmap Phase 0; validation plan §6, §7).

The honest one-step-ahead predictive distribution N(mean, S) the Kalman now
surfaces (var_total_pred, R included) must be *calibrated*: standardized
innovations z = (y - mean)/sqrt(S) should behave like N(0,1).  This freezes the
filter-consistency gates on synth — a deterministic, no-data-dependency check
that catches a filter that has drifted into over/under-confidence.

Scope: the weekly Layer-1 signal exists for QB only today, so calibration is
gated for QB (pooled across ALL synthetic QBs — n≈310 player-weeks — not the
single focus QB, whose n=12 + injury intervention make it far too noisy to
calibrate against).  RB/WR/TE/DEF await their per-position weekly Layer-1 credit
(Phase 2b) before they can be calibrated the same way.

The QB r_scale was tuned to 0.55 against these gates (was 0.35, overconfident).
Statistic choice (validation plan §7): weekly fantasy/credit is TD-lumpy and
heavy-tailed, so the POOLED mean NIS is pulled up by a few tail weeks while the
MEDIAN per-QB NIS measures central calibration robustly — both are gated, the
median tightly.  Gaussian PIT tail defects on weekly data are reality, not a bug
(ROS aggregates would be the Gaussian-clean target; that is Tier 1).
"""
import numpy as np
import pytest
from threadpoolctl import threadpool_limits
from sklearn.model_selection import GroupKFold
from sklearn.ensemble import HistGradientBoostingRegressor

from backend.grid import (
    load_synthetic, fit_value_model, attach_dv, fit,
    kalman_two_component, SSParams,
)
from backend.grid.layers import _onfield_rating_sums
from backend.grid.value import STATE_COLS


def _normal_cdf(z):
    """Standard-normal CDF (PIT transform) without scipy."""
    from math import erf, sqrt
    return 0.5 * (1.0 + np.vectorize(lambda x: erf(x / sqrt(2.0)))(z))


@pytest.fixture(scope="module")
def qb_calibration():
    """Pool one-step-ahead standardized innovations z across ALL synthetic QBs,
    using the shipped QB Kalman params (SSParams.from_position('QB')).

    The Layer-1 context model is QB-agnostic, so its out-of-fold residuals are
    computed ONCE (not per QB) and then aggregated to each QB's weekly credit —
    keeping the gate fast.  Cross-fitting groups by WEEK (GroupKFold) so every
    QB-week is fully held out of the fold that scores it — a play-level KFold
    would leak same-week plays across train/test and make these gates score
    partly-seen samples (stricter here than production layer1_qb_weekly's
    play-level CV; the calibration measure must be genuinely held-out).
    Single-threaded for reproducibility (the GBM diverges ~1e-2 multi-threaded;
    see the golden master)."""
    with threadpool_limits(limits=1):
        b = load_synthetic()
        plays, players, gt = b["plays"], b["players"], b["gt"]
        plays = attach_dv(plays, fit_value_model(plays))
        rng = np.random.default_rng(1)
        market = {t: v + rng.normal(0, 0.01) for t, v in gt["team_strength"].items()}
        res = fit(plays, players, market, gt["focus_qb"], n_iter=3, verbose=False)
        ratings_lookup = dict(zip(res["ratings"].player_id, res["ratings"].rating))

        # Out-of-fold context residuals (QB-agnostic) — computed once.
        off = plays.copy()
        _, def_sum = _onfield_rating_sums(off, ratings_lookup)
        feats = np.column_stack([off[STATE_COLS].to_numpy(float), def_sum])
        y = off["dv"].to_numpy(float)
        groups = off["week"].to_numpy()   # hold out whole weeks -> QB-weeks fully out of fold
        oof = np.zeros(len(off))
        for tr, te in GroupKFold(n_splits=5).split(feats, groups=groups):
            g = HistGradientBoostingRegressor(max_depth=3, learning_rate=0.1,
                                              max_iter=200, min_samples_leaf=150,
                                              random_state=0)
            g.fit(feats[tr], y[tr])
            oof[te] = g.predict(feats[te])
        off = off.assign(_resid=y - oof)
        W = int(off.week.max())

        params = SSParams.from_position("QB")
        z_all, per_qb_nis = [], []
        for qb in players[players.position == "QB"].player_id:
            mask = off.off_players.apply(lambda t: qb in t)
            if mask.sum() < 20:
                continue
            wk = off[mask].groupby("week").agg(c=("_resid", "mean"), s=("_resid", "size"))
            yv = np.full(W, np.nan)
            sn = np.zeros(W)
            pl = np.zeros(W, bool)
            for w, r in wk.iterrows():
                yv[w - 1] = r.c
                sn[w - 1] = r.s
                pl[w - 1] = True
            ss = kalman_two_component(yv, sn, pl, params=params)
            m = pl.copy()
            m[0] = False   # drop week 1: pure-prior forecast, not a real one-step prediction
            z = (yv[m] - ss["total_pred"][m]) / np.sqrt(ss["var_total_pred"][m])
            z_all.append(z)
            per_qb_nis.append(float(np.mean(z ** 2)))

    z = np.concatenate(z_all)
    return dict(z=z, per_qb_nis=np.array(per_qb_nis),
                pit=_normal_cdf(z), n_weeks=len(z), n_qb=len(per_qb_nis))


def test_sample_is_large_enough(qb_calibration):
    """Guard the calibration sample so the gates below are meaningful."""
    assert qb_calibration["n_qb"] >= 20
    assert qb_calibration["n_weeks"] >= 200


def test_median_per_qb_nis_calibrated(qb_calibration):
    """Robust central filter consistency: the median per-QB NIS mean(z^2) sits in
    the ideal [0.8, 1.25].  This is the primary calibration gate (tail-robust)."""
    med = float(np.median(qb_calibration["per_qb_nis"]))
    assert 0.8 <= med <= 1.25, f"median per-QB NIS={med:.3f} outside [0.8,1.25]"


def test_pooled_nis_bounded(qb_calibration):
    """Pooled NIS mean(z^2) over all QB-weeks. Heavy weekly tails pull it above 1,
    so the band is a touch wider than the median's; observed ~1.21."""
    nis = float(np.mean(qb_calibration["z"] ** 2))
    assert 0.8 <= nis <= 1.4, f"pooled NIS={nis:.3f} outside [0.8,1.4]"


def test_pit_unbiased(qb_calibration):
    """PIT mean ~0.5 — no systematic over/under-prediction bias."""
    assert abs(float(qb_calibration["pit"].mean()) - 0.5) < 0.07


def test_pit_dispersion_near_uniform(qb_calibration):
    """PIT std near a uniform's 1/sqrt(12)=0.289 — not grossly over-dispersed
    (overconfident, std>>0.289) nor under-dispersed (std<<0.289)."""
    sd = float(qb_calibration["pit"].std())
    assert 0.24 <= sd <= 0.34, f"PIT std={sd:.3f} far from uniform 0.289"


def test_coverage_80(qb_calibration):
    """80% predictive interval covers ~80% of outcomes (PICP@80 within ±10pp);
    observed ~0.78."""
    picp80 = float(np.mean(np.abs(qb_calibration["z"]) < 1.2816))
    assert 0.70 <= picp80 <= 0.90, f"PICP@80={picp80:.3f} outside [0.70,0.90]"
