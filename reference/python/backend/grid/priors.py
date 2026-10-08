"""
Cross-league prior layer (NCAA / USFL / XFL / UFL -> NFL).

Two jobs:
  1. Estimate the feeder->NFL equivalency from SHARED players (players observed
     in both a feeder league and the NFL), by regressing NFL rating on feeder
     SV. This is the same shared-unit identification as the opponent/teammate
     fixed point -- shared units link otherwise-incomparable pools. In reality
     this is range-restricted (you only see the mapping for players good enough
     to reach the NFL); here we report out-of-sample predictive fit honestly.
  2. Translate a prospect's feeder SV into an NFL-scale prior (mean + variance)
     for the state-space layer. The variance is POSITION-SPECIFIC: wide where
     college predicts the NFL poorly (QB), tight where it translates cleanly.
     A wide prior -> high early Kalman gain -> the prior washes out fast.

Each feeder league (USFL, XFL, UFL, and each NCAA tier) would get its own
slope/intercept; here we use one feeder league with a planted factor.
"""
from __future__ import annotations
import numpy as np
import pandas as pd
from sklearn.linear_model import LinearRegression

# position-specific prior SD on the NFL rating scale (translation fidelity).
# QB widest (college is a weak predictor of NFL QB play); specialists tightest.
PRIOR_SD = {"QB": 0.16, "RB": 0.07, "WR": 0.08, "TE": 0.06, "DEF": 0.07}

# League equivalency factors — how much of a feeder league's value translates to NFL.
# FBS is the strongest predictor; UFL/USFL the weakest.
LEAGUE_FACTORS = {
    "FBS": 0.35,
    "FCS": 0.20,
    "UFL": 0.15,
    "USFL": 0.15,
    "XFL": 0.15,
    "CFL": 0.18,
    "unknown": 0.25,  # fallback
}


def _age_adjustment(age: int) -> float:
    """Return a prior-mean boost/penalty based on player age.

    Under 24: ascending trajectory (+0.05).
    Over 30:  declining trajectory (-0.05).
    24-30:    prime years, no adjustment.
    """
    if age < 24:
        return 0.05
    if age > 30:
        return -0.05
    return 0.0


def _draft_capital_adjustment(draft_round: int | None) -> float:
    """Return a prior-mean boost based on NFL draft selection round.

    Round 1 picks have highest scouting confidence; undrafted get a small discount.
    """
    if draft_round is None:
        return 0.0
    if draft_round == 0:  # 0 encodes undrafted free agent
        return -0.03
    if draft_round == 1:
        return 0.08
    if draft_round <= 3:
        return 0.03
    return 0.0


def estimate_equivalency(
    college_df: pd.DataFrame,
    nfl_ratings: pd.DataFrame,
    league_type: str = "FBS",
):
    """Regress NFL rating on feeder SV over SHARED, non-rookie players.

    Returns dict(slope, intercept, oos_r2, factor_check, league_type). `factor_check` is a
    synthetic-only sanity number: the slope of feeder_sv on TRUE ability, which
    should land near the planted league_factor (confirms the feeder encodes
    ability at the right discount). oos_r2 is honest held-out predictive fit.

    ``nfl_ratings`` supplies the ``rating`` column the regression targets. On
    **real data** pass any real NFL rating there — the RAPM season rating
    (`run_rapm`) or, for the preseason cold-start, the Kalman end-of-season
    smoothed talent — and simply omit the planted ``ability`` column: this is the
    real-data unblock (the function was synth-only precisely because it hard-
    required ``ability``, which exists only in synth).  When ``ability`` is
    absent ``factor_check`` is ``None`` (its only use is the synth sanity check);
    everything else — slope, intercept, oos_r2 — is computed identically.

    league_type: one of 'FBS', 'FCS', 'UFL', 'USFL', 'XFL', 'CFL', 'unknown'.
    The equivalency factor for that league is stored in the returned dict so
    build_priors() can scale the slope accordingly.
    """
    has_ability = "ability" in nfl_ratings.columns
    merge_cols = ["player_id", "rating"] + (["ability"] if has_ability else [])
    shared = college_df[~college_df.is_rookie].merge(
        nfl_ratings[merge_cols], on="player_id")
    x = shared.feeder_sv.to_numpy().reshape(-1, 1)
    y = shared.rating.to_numpy()

    # out-of-sample fit via a simple split
    rng = np.random.default_rng(3)
    idx = rng.permutation(len(shared))
    cut = int(0.7 * len(shared))
    tr, te = idx[:cut], idx[cut:]
    lr = LinearRegression().fit(x[tr], y[tr])
    ss_res = np.sum((y[te] - lr.predict(x[te])) ** 2)
    ss_tot = np.sum((y[te] - y[te].mean()) ** 2)
    oos_r2 = 1 - ss_res / ss_tot

    full = LinearRegression().fit(x, y)
    # synth-only sanity number; on real data (no planted ability) it's undefined.
    factor_check = (float(np.polyfit(shared.ability, shared.feeder_sv, 1)[0])
                    if has_ability else None)
    league_equiv = LEAGUE_FACTORS.get(league_type, LEAGUE_FACTORS["unknown"])
    return dict(
        slope=float(full.coef_[0]),
        intercept=float(full.intercept_),
        oos_r2=float(oos_r2),
        factor_check=factor_check,
        n_shared=len(shared),
        league_type=league_type,
        league_factor=league_equiv,
    )


def build_priors(
    college_df: pd.DataFrame,
    equiv: dict,
    age_col: str | None = None,
    draft_round_col: str | None = None,
) -> pd.DataFrame:
    """Translate feeder SV -> NFL-scale prior (mean, var) for every prospect.

    age_col: column name in college_df with player age (int). When provided,
    _age_adjustment() is applied to prior_mean.
    draft_round_col: column name with draft round (int, 0=undrafted). When
    provided, _draft_capital_adjustment() is applied to prior_mean.
    """
    df = college_df.copy()
    df["prior_mean"] = equiv["intercept"] + equiv["slope"] * df["feeder_sv"]
    if age_col is not None and age_col in df.columns:
        df["prior_mean"] += df[age_col].apply(_age_adjustment)
    if draft_round_col is not None and draft_round_col in df.columns:
        df["prior_mean"] += df[draft_round_col].apply(_draft_capital_adjustment)
    df["prior_sd"] = df["position"].map(PRIOR_SD).fillna(0.10)
    df["prior_var"] = df["prior_sd"] ** 2
    return df[["player_id", "position", "is_rookie", "feeder_sv",
               "prior_mean", "prior_var", "prior_sd"]]


def washout_table(prior_sd_by_pos: dict, obs_var_per_game: float,
                  games=(1, 2, 4, 6, 8, 12)) -> pd.DataFrame:
    """How fast does the feeder prior wash out? Prior weight after k games under
    a conjugate-normal update: w = (1/P0) / (1/P0 + k/R). Answers 'how much does
    college matter, and for how long' -- per position."""
    rows = []
    for pos, sd in prior_sd_by_pos.items():
        P0 = sd ** 2
        row = {"position": pos, "prior_sd": sd}
        half = None
        for k in games:
            w = (1 / P0) / (1 / P0 + k / obs_var_per_game)
            row[f"g{k}"] = round(w, 2)
            if half is None and w < 0.5:
                half = k
        row["games_to_<50%"] = half
        rows.append(row)
    return pd.DataFrame(rows)


if __name__ == "__main__":
    from synth import simulate, make_college, SynthConfig
    from value import fit_value_model, attach_dv
    from layers import fit

    cfg = SynthConfig(yards_noise_sd=3.2, drives_per_team_per_game=12)
    plays, players, gt = simulate(cfg)
    plays = attach_dv(plays, fit_value_model(plays))
    rng = np.random.default_rng(1)
    market = {t: v + rng.normal(0, 0.01) for t, v in gt["team_strength"].items()}
    res = fit(plays, players, market, gt["focus_qb"], n_iter=2, verbose=False)

    college = make_college(players, gt, cfg)
    equiv = estimate_equivalency(college, res["ratings"])
    print("equivalency: slope=%.3f intercept=%.3f  oos_R2=%.3f  n_shared=%d"
          % (equiv["slope"], equiv["intercept"], equiv["oos_r2"], equiv["n_shared"]))
    print("planted league_factor=%.3f  recovered factor_check=%.3f"
          % (gt["league_factor"], equiv["factor_check"]))

    priors = build_priors(college, equiv)
    rk = priors[priors.is_rookie].merge(res["ratings"][["player_id", "ability"]], on="player_id")
    print("rookie prior_mean vs TRUE ability corr=%.3f (prior is informative)"
          % np.corrcoef(rk.prior_mean, rk.ability)[0, 1])
    print(washout_table(PRIOR_SD, obs_var_per_game=0.40 / 90).to_string(index=False))
