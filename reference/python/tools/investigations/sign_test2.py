"""Team-strength conventions vs market-row sign, five seeds (legacy-synth evidence).

What it demonstrates
    For seeds 7, 11, 23, 31, 42 of the canonical-shaped ``SynthConfig(yards_noise_sd=3.2,
    drives_per_team_per_game=12)``, compares team recovery under two planted conventions:
    B = off - def (the oracle's ``_team_strength``) and A = off + def ("net strength", what a
    point spread prices), each with and without a market anchor and with the row written
    ``[+1,+1]`` or ``[+1,-1]``. Prints per-seed correlations and the five-seed means.
Evidence for
    KI-G1 / KI-V1 and KI-NEW-A1; input to DR-B5 (proposed default, pending the owner: net
    strength, keep ``[+1,+1]``, reject ``[+1,-1]``). The inventory quoted five-seed means of
    0.580 (shipped) vs 0.644 (``[+1,-1]``) for convention B, and 0.536 (option A, anchored)
    vs 0.403 (no market). **Read with KI-NEW-Y0:** all of this is the legacy generator,
    where defenders come from the offense; the critic (X-2, X-19) rules the ``[+1,-1]``
    numbers moot. Recorded as history, not as a parity target.
Synthetic generator
    Evidence was recorded on ``--synth legacy`` (the default). ``--synth fixed`` re-runs it on
    the defender-fixed generator (in memory; see _common.py).
Run (from reference/python, about 11 s)
    python3 -m tools.investigations.sign_test2 [--synth legacy|fixed]
Expected output, --synth legacy (recorded 2026-10-01 with requirements.lock pins)
    [sign_test2] backend imported from ./backend | synth generator: legacy (oracle as imported, KI-NEW-Y0) | requested --synth legacy
          B_cur_[+1,+1]_off-def  B_fix_[+1,-1]_off-def  A_[+1,+1]_off+def  nomarket_off+def  nomarket_off-def
    seed
    7                     0.682                  0.739              0.547             0.471             0.693
    11                    0.401                  0.593              0.456             0.256             0.424
    23                    0.686                  0.697              0.821             0.764             0.600
    31                    0.659                  0.638              0.555             0.356             0.505
    42                    0.471                  0.555              0.301             0.167             0.442
    B_cur_[+1,+1]_off-def    0.580
    B_fix_[+1,-1]_off-def    0.644
    A_[+1,+1]_off+def        0.536
    nomarket_off+def         0.403
    nomarket_off-def         0.533
Origin
    Consolidation inventory scratch ``scratch-cnissues/sign_test2.py`` (cn-issues report
    section 3.1, G1 and NEW-A1). Logic unchanged; the path bootstrap, ``--synth`` switch,
    banner and this header were added.
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


def solve(plays, players, market, sign):
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
            XtX += 40 * np.outer(r, r)
            Xty += 40 * r * market[t]
    return L._solve_ridge_prior(XtX, Xty, 120.0, np.zeros(n), mask), ci


def main(argv=None) -> None:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    add_synth_arg(ap)
    args = ap.parse_args(argv)
    banner("sign_test2", select_synth(args.synth))
    res = []
    for seed in (7, 11, 23, 31, 42):
        plays, players, gt = simulate(SynthConfig(seed=seed, yards_noise_sd=3.2, drives_per_team_per_game=12))
        plays = attach_dv(plays, fit_value_model(plays))
        tp = players
        off = tp[tp.position.isin(list(OFF_ONFIELD)) & tp.is_starter].groupby("team").ability.mean()
        de = tp[(tp.position == "DEF") & tp.is_starter].groupby("team").ability.mean()
        rng = np.random.default_rng(1)
        noise = {t: rng.normal(0, 0.01) for t in off.index}
        truthB = {t: off[t] - de[t] for t in off.index}   # current synth convention
        truthA = {t: off[t] + de[t] for t in off.index}   # net strength convention
        row = {"seed": seed}
        for lab, truth, sign, tr in (("B_cur_[+1,+1]_off-def", truthB, 1., -1),
                                     ("B_fix_[+1,-1]_off-def", truthB, -1., -1),
                                     ("A_[+1,+1]_off+def", truthA, 1., 1),
                                     ("nomarket_off+def", truthA, None, 1),
                                     ("nomarket_off-def", truthB, None, -1)):
            mk = None if sign is None else {t: truth[t] + noise[t] for t in truth}
            b, ci = solve(plays, players, mk, sign if sign else 1.)
            rating = np.array([b[ci["t_off"][t]] + tr * b[ci["t_def"][t]] for t in ci["teams"]])
            row[lab] = np.corrcoef(rating, [truth[t] for t in ci["teams"]])[0, 1]
        res.append(row)
    df = pd.DataFrame(res).set_index("seed")
    print(df.round(3).to_string())
    print(df.mean().round(3).to_string())


if __name__ == "__main__":
    main()
