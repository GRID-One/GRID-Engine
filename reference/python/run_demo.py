"""
End-to-end GRID demo on synthetic data with PLANTED ground truth.

Runs the full stack and reports whether each estimator recovers the truth:
  1. value      -> fit V(s), value plays as dV
  2. layers     -> fixed-point RAPM (+ market); recover player & team value
  3. statespace -> recover the focus QB's weekly talent/form trajectory + injury
  4. priors     -> recover the feeder->NFL equivalency; prior washout speed

Saves two plots to /mnt/user-data/outputs/.
"""
from __future__ import annotations
import os
import numpy as np
import pandas as pd
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

from backend.grid import (
    load_synthetic, fit_value_model, attach_dv, fit,
    kalman_two_component, estimate_equivalency, build_priors, washout_table,
)
from backend.grid import SSParams, PRIOR_SD

OUT = os.environ.get("GRID_DEMO_OUT", "/mnt/user-data/outputs")
os.makedirs(OUT, exist_ok=True)
POS_COLORS = {"QB": "#1f77b4", "RB": "#ff7f0e", "WR": "#2ca02c",
              "TE": "#9467bd", "DEF": "#d62728"}


def main():
    print("=" * 64)
    print("GRID  --  end-to-end recovery on synthetic ground truth")
    print("=" * 64)

    bundle = load_synthetic()
    plays, players, gt, college = bundle["plays"], bundle["players"], bundle["gt"], bundle["college"]
    print(f"data: {len(plays):,} plays | {len(players)} players | "
          f"{players.team.nunique()} teams | {plays.week.nunique()} weeks")

    # ---- 1-2. value + attribution (fixed point) ----
    plays = attach_dv(plays, fit_value_model(plays))
    rng = np.random.default_rng(1)
    market = {t: v + rng.normal(0, 0.01) for t, v in gt["team_strength"].items()}
    res = fit(plays, players, market, gt["focus_qb"], n_iter=3, verbose=False)
    ratings, team_df, qb_weekly = res["ratings"], res["team_df"], res["qb_weekly"]

    overall = np.corrcoef(ratings.rating, ratings.ability)[0, 1]
    print("\n[2] ATTRIBUTION  corr(estimated rating, planted ability)")
    print(f"    overall: {overall:.3f}")
    perpos = {}
    for pos in ["QB", "RB", "WR", "TE", "DEF"]:
        s = ratings[ratings.position == pos]
        perpos[pos] = np.corrcoef(s.rating, s.ability)[0, 1]
        print(f"    {pos:<4} {perpos[pos]:.3f}  (n={len(s)})")
    tm = team_df.merge(pd.DataFrame({"team": list(gt["team_strength"]),
                                     "true": list(gt["team_strength"].values())}), on="team")
    team_corr = np.corrcoef(tm.team_rating, tm.true)[0, 1]
    print(f"    team strength (market-reconciled): {team_corr:.3f}")

    # ---- 3. state-space on the focus QB ----
    W = gt["focus_tau"].shape[0]
    y = np.full(W, np.nan); snaps = np.zeros(W); played = np.zeros(W, bool)
    wk = qb_weekly.set_index("week")
    for w in range(1, W + 1):
        if w in wk.index:
            y[w - 1] = wk.loc[w, "qb_credit"]; snaps[w - 1] = wk.loc[w, "snaps"]; played[w - 1] = True
    ss = kalman_two_component(y, snaps, played, interventions={9}, params=SSParams())
    truth = gt["focus_tau"]; m = np.isfinite(truth) & played
    raw_corr = np.corrcoef(y[m], truth[m])[0, 1]
    cur_corr = np.corrcoef(ss["total_smooth"][m], truth[m])[0, 1]
    tau_corr = np.corrcoef(ss["tau_smooth"][m], truth[m])[0, 1]
    print("\n[3] STATE-SPACE  focus QB (rise -> injury wk8-9 -> diminished return -> recovery)")
    print(f"    raw weekly signal corr            : {raw_corr:.3f}")
    print(f"    current-ability (tau+form) corr   : {cur_corr:.3f}")
    print(f"    talent (tau) corr                 : {tau_corr:.3f}  (smooth arc, ignores transient)")
    print(f"    current-ability variance healthy(wk3)={ss['var_total_filt'][2]:.4f} "
          f"return(wk10)={ss['var_total_filt'][9]:.4f}  (intervention widens it)")

    # ---- 4. cross-league prior ----
    equiv = estimate_equivalency(college, ratings)
    priors = build_priors(college, equiv)
    rk = priors[priors.is_rookie].merge(ratings[["player_id", "ability"]], on="player_id")
    rookie_corr = np.corrcoef(rk.prior_mean, rk.ability)[0, 1]
    print("\n[4] CROSS-LEAGUE PRIOR  (NCAA/USFL/XFL/UFL -> NFL)")
    print(f"    planted league factor             : {gt['league_factor']:.3f}")
    print(f"    recovered factor check            : {equiv['factor_check']:.3f}")
    print(f"    feeder->NFL out-of-sample R^2     : {equiv['oos_r2']:.3f}  (weak signal -> wide priors)")
    print(f"    rookie prior vs true ability corr : {rookie_corr:.3f}  (n={len(rk)})")
    wt = washout_table(PRIOR_SD, obs_var_per_game=0.12 ** 2)
    print("    prior washout (prior weight by game; wide QB prior overwritten fastest):")
    print(wt.to_string(index=False).replace("\n", "\n    "))

    # ---- plots ----
    _plot_recovery(ratings, overall)
    _plot_qb(truth, y, played, ss, cur_corr)
    print(f"\nsaved plots -> {OUT}/grid_recovery.png , {OUT}/grid_focus_qb.png")
    return dict(overall=overall, perpos=perpos, team_corr=team_corr,
                raw_corr=raw_corr, cur_corr=cur_corr, tau_corr=tau_corr,
                equiv=equiv, rookie_corr=rookie_corr)


def _plot_recovery(ratings, overall):
    fig, ax = plt.subplots(figsize=(6.4, 6.0))
    for pos, c in POS_COLORS.items():
        s = ratings[ratings.position == pos]
        ax.scatter(s.ability, s.rating, s=22, alpha=0.7, color=c, label=pos, edgecolor="none")
    lo = min(ratings.ability.min(), ratings.rating.min())
    hi = max(ratings.ability.max(), ratings.rating.max())
    ax.plot([lo, hi], [lo, hi], color="0.6", lw=1, ls="--", zorder=0)
    ax.set_xlabel("planted true ability (EPA/play units)")
    ax.set_ylabel("GRID estimated rating")
    ax.set_title(f"Player value recovery — overall r = {overall:.2f}\n"
                 "(every player generated from hidden ability; estimated blind)")
    ax.legend(title="position", frameon=False, loc="upper left")
    ax.grid(alpha=0.25)
    fig.tight_layout(); fig.savefig(f"{OUT}/grid_recovery.png", dpi=150); plt.close(fig)


def _plot_qb(truth, y, played, ss, cur_corr):
    W = len(truth); weeks = np.arange(1, W + 1)
    # map estimate (arbitrary rating scale) to planted-ability units via least
    # squares on played weeks, for a readable single-axis overlay; the reported
    # correlation is scale-invariant and does not depend on this mapping.
    m = np.isfinite(truth) & played
    A = np.vstack([ss["total_smooth"][m], np.ones(m.sum())]).T
    a, b = np.linalg.lstsq(A, truth[m], rcond=None)[0]
    cur = a * ss["total_filt"] + b
    tau = a * ss["tau_smooth"] + b
    band = np.abs(a) * np.sqrt(ss["var_total_filt"])

    fig, ax = plt.subplots(figsize=(8.2, 4.8))
    ax.axvspan(7.5, 9.5, color="0.85", label="injured (missed)")
    ax.plot(weeks, truth, "o-", color="black", lw=2, ms=6, label="planted true ability", zorder=5)
    ax.plot(weeks, cur, "-", color="#1f77b4", lw=2, label="GRID current ability (filtered, τ+form)")
    ax.fill_between(weeks, cur - band, cur + band, color="#1f77b4", alpha=0.18)
    ax.plot(weeks, tau, "--", color="#2ca02c", lw=1.8, label="GRID talent (smoothed τ)")
    ax.set_xlabel("week"); ax.set_ylabel("ability (planted units)")
    ax.set_title(f"Focus QB: recovering a latent trajectory through an injury "
                 f"(current-ability r = {cur_corr:.2f})")
    ax.legend(frameon=False, fontsize=8, loc="lower right")
    ax.grid(alpha=0.25); ax.set_xticks(weeks)
    fig.tight_layout(); fig.savefig(f"{OUT}/grid_focus_qb.png", dpi=150); plt.close(fig)


if __name__ == "__main__":
    main()
