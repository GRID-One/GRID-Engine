"""Tier 0 — recovery gates (roadmap Phase 0; validation plan §4, §9).

Formalizes the run_demo.py recovery correlations into asserted, seeded tests:
at each layer the estimator must recover the PLANTED ground truth from synth.py.
These are hard regression gates against *math* drift — they run on the small
synth fixture, are deterministic, and need no real data.

Thresholds are CALIBRATED BELOW THE OBSERVED BASELINE (calibrate-then-gate,
validation plan §9), not set to the plan's aspirational targets: the synth
baseline is e.g. overall attribution ~0.80 (plan's 0.85 was aspirational), team
strength ~0.66 (not 0.90), prior OOS R^2 ~0.15 (weak by design — wide priors).
Each gate sits a margin below the measured value so it passes comfortably today
and fails only on a real recovery regression.  Observed values are recorded in
comments next to each assert; tighten as the synth model evolves.
"""
import numpy as np
import pandas as pd
import pytest

from backend.grid import (
    load_synthetic, fit_value_model, attach_dv, fit,
    kalman_two_component, estimate_equivalency, build_priors, SSParams,
)


@pytest.fixture(scope="module")
def recovery():
    """Run the full synth recovery stack once (seeded) and expose every metric
    the Tier 0 gates assert on."""
    b = load_synthetic()
    plays, players, gt, college = b["plays"], b["players"], b["gt"], b["college"]
    plays = attach_dv(plays, fit_value_model(plays))
    rng = np.random.default_rng(1)
    market = {t: v + rng.normal(0, 0.01) for t, v in gt["team_strength"].items()}
    res = fit(plays, players, market, gt["focus_qb"], n_iter=3, verbose=False)
    ratings, team_df, qb_weekly = res["ratings"], res["team_df"], res["qb_weekly"]

    # --- attribution ---
    overall = np.corrcoef(ratings.rating, ratings.ability)[0, 1]
    perpos = {}
    for pos in ("QB", "RB", "WR", "TE", "DEF"):
        s = ratings[ratings.position == pos]
        perpos[pos] = np.corrcoef(s.rating, s.ability)[0, 1]
    tm = team_df.merge(
        pd.DataFrame({"team": list(gt["team_strength"]),
                      "true": list(gt["team_strength"].values())}),
        on="team",
    )
    team_corr = np.corrcoef(tm.team_rating, tm.true)[0, 1]

    # --- state-space (focus QB through an injury at weeks 8-9) ---
    W = gt["focus_tau"].shape[0]
    y = np.full(W, np.nan); snaps = np.zeros(W); played = np.zeros(W, bool)
    wk = qb_weekly.set_index("week")
    for w in range(1, W + 1):
        if w in wk.index:
            y[w - 1] = wk.loc[w, "qb_credit"]
            snaps[w - 1] = wk.loc[w, "snaps"]
            played[w - 1] = True
    ss = kalman_two_component(y, snaps, played, interventions={9}, params=SSParams())
    truth = gt["focus_tau"]; m = np.isfinite(truth) & played
    total_smooth_corr = np.corrcoef(ss["total_smooth"][m], truth[m])[0, 1]
    tau_smooth_corr = np.corrcoef(ss["tau_smooth"][m], truth[m])[0, 1]
    nis = float(np.mean(((y[m] - ss["total_pred"][m]) / np.sqrt(ss["var_total_pred"][m])) ** 2))

    # --- cross-league prior ---
    equiv = estimate_equivalency(college, ratings)
    priors = build_priors(college, equiv)
    rk = priors[priors.is_rookie].merge(ratings[["player_id", "ability"]], on="player_id")
    rookie_corr = np.corrcoef(rk.prior_mean, rk.ability)[0, 1]

    return dict(
        overall=overall, perpos=perpos, team_corr=team_corr,
        ss=ss, total_smooth_corr=total_smooth_corr, tau_smooth_corr=tau_smooth_corr,
        nis=nis, league_factor=gt["league_factor"],
        equiv_slope=equiv["slope"],           # the mapping build_priors actually uses
        factor_check=equiv["factor_check"],   # synth-only sanity number (NOT used downstream)
        oos_r2=equiv["oos_r2"],
        rookie_corr=rookie_corr,
    )


# --------------------------------------------------------------- attribution
def test_attribution_pooled(recovery):
    """Pooled corr(rating, planted ability) clears the calibrated floor."""
    assert recovery["overall"] >= 0.77   # observed 0.8025


def test_attribution_per_position(recovery):
    """Per-position attribution recovery clears each calibrated floor."""
    floors = {"QB": 0.83, "RB": 0.70, "WR": 0.76, "TE": 0.73, "DEF": 0.73}
    # observed: QB 0.869 RB 0.743 WR 0.797 TE 0.771 DEF 0.765
    for pos, floor in floors.items():
        assert recovery["perpos"][pos] >= floor, f"{pos} recovery regressed: {recovery['perpos'][pos]:.3f}"


def test_team_strength_recovery(recovery):
    """Market-reconciled team-strength recovery clears the calibrated floor."""
    assert recovery["team_corr"] >= 0.60   # observed 0.6643 (market-reconciled)


# --------------------------------------------------------------- state-space
def test_kalman_trajectory_recovery(recovery):
    """Smoothed current-ability and talent trajectories recover planted truth."""
    assert recovery["total_smooth_corr"] >= 0.92   # observed 0.958 (current ability)
    assert recovery["tau_smooth_corr"] >= 0.60     # observed 0.6745 (smoothed talent)


def test_injury_detection_widens_variance(recovery):
    """Returning from the planted week 8-9 injury (intervention at index 9)
    must widen the filtered variance vs a healthy mid-season week."""
    var = recovery["ss"]["var_total_filt"]
    healthy_wk3, return_wk10 = var[2], var[9]   # observed 0.0023 -> 0.0061
    assert return_wk10 > healthy_wk3, (
        f"injury return did not widen variance: {return_wk10:.4f} <= {healthy_wk3:.4f}"
    )


def test_filter_nis_is_bounded(recovery):
    """NIS mean(z^2) on the focus QB's predictive innovations is finite and
    bounded.  NOTE: observed ~4.6 (n=12) — ABOVE the plan's ideal [0.8, 1.25],
    i.e. the filter is currently overconfident on the dV-currency QB signal.
    Tightening to ~1 is filter-calibration work (tune r_scale/d), tracked for
    the synth-calibration tier; here we only guard against blow-up / NaN."""
    assert np.isfinite(recovery["nis"])
    # One-sided: only catches blow-up / NaN. No lower bound — if r_scale/d
    # calibration later pulls NIS toward the ideal (<1), this must not fail.
    assert recovery["nis"] <= 10.0   # observed 4.58


# --------------------------------------------------------------- cross-league prior
def test_prior_equivalency_mapping(recovery):
    """Gate the feeder->NFL mapping that build_priors actually uses
    (equiv['slope'] = prior_mean = intercept + slope*feeder_sv), NOT the
    synth-only factor_check sanity number.  OOS R^2 is weak BY DESIGN (weak
    signal -> wide priors), so it only has to clear a low floor."""
    assert 0.9 <= recovery["equiv_slope"] <= 1.8   # observed 1.3159 (planted factor 0.62)
    assert recovery["oos_r2"] >= 0.05              # observed 0.149 (weak signal)


def test_rookie_prior_recovery(recovery):
    """Rookie prior_mean (slope-mapped feeder SV) recovers planted ability."""
    assert recovery["rookie_corr"] >= 0.50   # observed 0.5829
