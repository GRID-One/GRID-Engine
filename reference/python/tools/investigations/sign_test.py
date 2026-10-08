"""Layer-3 market-row convention vs recovery, three seeds (legacy-synth evidence).

What it demonstrates
    Re-solves the Layer-2 RAPM ridge (lambda 120, team-intercept mask 0.05) with the Layer-3
    market pseudo-rows written three ways (no market; the shipped ``[+1 on t_off, +1 on
    t_def]``; the issue log's queued ``[+1, -1]``), anchoring the planted
    ``gt["team_strength"]`` (+ N(0, 0.01) noise), and reports team and player recovery plus
    how ``beta_def`` relates to the planted defence quality. Seeds 7, 11, 23 with the
    default ``SynthConfig`` (not the canonical one).
Evidence for
    KI-G1 / KI-V1 (market row sign) and KI-NEW-A1 (team-strength convention). Input to DR-B5.
    **Read with KI-NEW-Y0:** on the legacy generator the "defenders" belong to the offense,
    and the planted ``team_strength`` (off - def) is only coherent with that bug. The
    consolidation critic (X-2, X-19) therefore treats every ``[+1,-1]`` improvement measured
    here as moot: it aligns the code with the buggy generator. Kept as the record of what was
    measured, not as support for ``[+1,-1]``.
Synthetic generator
    Evidence was recorded on ``--synth legacy`` (the default). ``--synth fixed`` re-runs it on
    the defender-fixed generator (in memory; see _common.py), where the planted off - def
    target is semantically wrong (critic G-1), so read those numbers with care.
Run (from reference/python, about 6 s)
    python3 -m tools.investigations.sign_test [--synth legacy|fixed]
Expected output, --synth legacy (recorded 2026-10-01 with requirements.lock pins)
    [sign_test] backend imported from ./backend | synth generator: legacy (oracle as imported, KI-NEW-Y0) | requested --synth legacy
    seed 7
                            no_market  [+1,+1]  [+1,-1]
    corr_team_rating_truth      0.834    0.804    0.860
    corr_player                 0.801    0.808    0.809
    corr_bdef_true_def          0.169    0.184    0.120
    corr_neg_bdef_true_def     -0.169   -0.184   -0.120
    mean_boff                  -0.032   -0.024   -0.006
    mean_bdef                   0.032    0.038    0.006
    seed 11
                            no_market  [+1,+1]  [+1,-1]
    corr_team_rating_truth      0.743    0.671    0.818
    corr_player                 0.799    0.802    0.801
    corr_bdef_true_def          0.051    0.078    0.009
    corr_neg_bdef_true_def     -0.051   -0.078   -0.009
    mean_boff                  -0.031   -0.021   -0.004
    mean_bdef                   0.031    0.040    0.004
    seed 23
                            no_market  [+1,+1]  [+1,-1]
    corr_team_rating_truth      0.760    0.795    0.835
    corr_player                 0.799    0.803    0.801
    corr_bdef_true_def          0.333    0.327    0.347
    corr_neg_bdef_true_def     -0.333   -0.327   -0.347
    mean_boff                  -0.050   -0.039   -0.009
    mean_bdef                   0.050    0.060    0.009
Origin
    Consolidation inventory scratch ``scratch-cnissues/sign_test.py`` (cn-issues report
    section 3.1, G1). Logic unchanged; the path bootstrap, ``--synth`` switch, banner and this
    header were added.
"""
from __future__ import annotations

import argparse
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2]))
from tools.investigations._common import add_synth_arg, banner, bootstrap, select_synth  # noqa: E402

bootstrap()

import numpy as np  # noqa: E402
import pandas as pd  # noqa: E402

import backend.grid.layers as L  # noqa: E402
from backend.grid.synth import OFF_ONFIELD, SynthConfig, simulate  # noqa: E402
from backend.grid.value import attach_dv, fit_value_model  # noqa: E402


def run(plays, players, market, sign, lam=120.0, w_market=40.0):
    X, y, ci = L.build_design(plays, players)
    n = ci["nCols"]
    teams = ci["teams"]
    mask = np.ones(n)
    for t in teams:
        mask[ci["t_off"][t]] = .05
        mask[ci["t_def"][t]] = .05
    XtX = (X.T @ X).toarray()
    Xty = X.T @ y
    if market is not None:
        for t in teams:
            r = np.zeros(n)
            r[ci["t_off"][t]] = 1.
            r[ci["t_def"][t]] = sign
            XtX += w_market * np.outer(r, r)
            Xty += w_market * r * market[t]
    b = L._solve_ridge_prior(XtX, Xty, lam, np.zeros(n), mask)
    return b, ci


def main(argv=None) -> None:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    add_synth_arg(ap)
    args = ap.parse_args(argv)
    banner("sign_test", select_synth(args.synth))
    for seed in (7, 11, 23):
        plays, players, gt = simulate(SynthConfig(seed=seed))
        plays = attach_dv(plays, fit_value_model(plays))
        rng = np.random.default_rng(1)
        ts = gt["team_strength"]   # synth convention: mean_off - mean_def
        market = {t: v + rng.normal(0, 0.01) for t, v in ts.items()}
        # true per-team def ability (higher = better defense)
        tp = players
        def_true = tp[(tp.position == "DEF") & tp.is_starter].groupby("team").ability.mean()
        off_true = tp[(tp.position.isin(list(OFF_ONFIELD))) & tp.is_starter].groupby("team").ability.mean()  # noqa: F841 (unused, as in the original)
        out = {}
        for label, mk, sign in (("no_market", None, 1.0), ("[+1,+1]", market, 1.0), ("[+1,-1]", market, -1.0)):
            b, ci = run(plays, players, mk, sign)
            teams = ci["teams"]
            boff = np.array([b[ci["t_off"][t]] for t in teams])
            bdef = np.array([b[ci["t_def"][t]] for t in teams])
            team_rating = boff - bdef
            tr = np.array([ts[t] for t in teams])
            pr = b[:ci["nP"]]
            inv = {v: k for k, v in ci["p_col"].items()}
            ab = np.array([gt["ability"][inv[i]] for i in range(ci["nP"])])
            out[label] = dict(
                corr_team_rating_truth=np.corrcoef(team_rating, tr)[0, 1],
                corr_player=np.corrcoef(pr, ab)[0, 1],
                corr_bdef_true_def=np.corrcoef(bdef, def_true.loc[teams].values)[0, 1],
                corr_neg_bdef_true_def=np.corrcoef(-bdef, def_true.loc[teams].values)[0, 1],
                mean_boff=boff.mean(), mean_bdef=bdef.mean())
        print("seed", seed)
        print(pd.DataFrame(out).round(3).to_string())


if __name__ == "__main__":
    main()
