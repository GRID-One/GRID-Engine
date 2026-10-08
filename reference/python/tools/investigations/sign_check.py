"""Market-row sign, matchup-grade sign and gauge-invariant team effects (canonical synth).

What it demonstrates
    On the canonical ``load_synthetic()`` league (single-threaded), re-solves Layer 2 with
    four market variants (none; shipped ``[+1,+1]`` anchoring off - def; the queued
    ``[+1,-1]`` anchoring off - def; ``[+1,+1]`` anchoring off + def) and prints, for each:
    corr of the intercept-only team ratings with both planted conventions, corr of
    ``+/-gamma_def`` with planted DEF quality, and player recovery. It then compares those
    raw intercepts with *gauge-invariant* team effects: E_off = gamma_off + exposure-weighted
    on-field offensive ratings, E_def = gamma_def + exposure-weighted on-field defensive
    ratings. Recorded finding (reconcile-code-first C12): raw-intercept team correlations
    0.46-0.74 vs 0.82-0.92 for the gauge-invariant aggregates; the market rows change little.
Evidence for
    KI-G1 / KI-V1, KI-NEW-A1, KI-NEW-A2; input to DR-B5 (proposed default, pending the
    owner: report net strength from gauge-invariant aggregates; grade = +E_def).
    **Read with KI-NEW-Y0:** recorded on the legacy generator, where a play's defenders are
    the offense's own DEF players, so every DEF / team-convention number here must be
    re-measured on the fixed generator (critic X-2) before it informs a decision.
Synthetic generator
    Evidence was recorded on ``--synth legacy`` (the default). ``--synth fixed`` re-runs it on
    the defender-fixed generator (in memory; see _common.py).
Run (from reference/python, about 5 s)
    python3 -m tools.investigations.sign_check [--synth legacy|fixed]
Expected output (recorded 2026-10-01 with requirements.lock pins)
    --synth legacy (the recorded evidence):
        [sign_check] backend imported from ./backend | synth generator: legacy (oracle as imported, KI-NEW-Y0) | requested --synth legacy
        no market                                  corr(g_off-g_def, off-def)=+0.693 corr(g_off+g_def, off+def)=+0.471 corr(g_def, def_s)=-0.223 corr(-g_def, def_s)=+0.223 player corr=0.797
        CURRENT [+1,+1] -> (off-def)+noise         corr(g_off-g_def, off-def)=+0.682 corr(g_off+g_def, off+def)=+0.510 corr(g_def, def_s)=-0.185 corr(-g_def, def_s)=+0.185 player corr=0.805
        AUDIT-FIX [+1,-1] -> (off-def)+noise       corr(g_off-g_def, off-def)=+0.739 corr(g_off+g_def, off+def)=+0.459 corr(g_def, def_s)=-0.287 corr(-g_def, def_s)=+0.287 player corr=0.805
        PROPOSED [+1,+1] -> (off+def)+noise        corr(g_off-g_def, off-def)=+0.678 corr(g_off+g_def, off+def)=+0.547 corr(g_def, def_s)=-0.180 corr(-g_def, def_s)=+0.180 player corr=0.805
        --- gauge-invariant team effects (intercept + on-field-weighted player ratings) ---
        no market                    corr(E_off,true_off)=+0.857 corr(E_def,true_def)=+0.609 corr(E_off+E_def,true_off+true_def)=+0.844 corr(E_off-E_def, true_off-true_def)=+0.854
        CURRENT                      corr(E_off,true_off)=+0.875 corr(E_def,true_def)=+0.677 corr(E_off+E_def,true_off+true_def)=+0.824 corr(E_off-E_def, true_off-true_def)=+0.901
        AUDIT-FIX                    corr(E_off,true_off)=+0.886 corr(E_def,true_def)=+0.681 corr(E_off+E_def,true_off+true_def)=+0.829 corr(E_off-E_def, true_off-true_def)=+0.920
        PROPOSED(off+def,[+1,+1])    corr(E_off,true_off)=+0.878 corr(E_def,true_def)=+0.692 corr(E_off+E_def,true_off+true_def)=+0.830 corr(E_off-E_def, true_off-true_def)=+0.902
    --synth fixed (for comparison; not part of the inventory record):
        [sign_check] backend imported from ./backend | synth generator: defender-fixed (in memory) | requested --synth fixed
        no market                                  corr(g_off-g_def, off-def)=+0.684 corr(g_off+g_def, off+def)=+0.431 corr(g_def, def_s)=+0.529 corr(-g_def, def_s)=-0.529 player corr=0.822
        CURRENT [+1,+1] -> (off-def)+noise         corr(g_off-g_def, off-def)=+0.689 corr(g_off+g_def, off+def)=+0.509 corr(g_def, def_s)=+0.512 corr(-g_def, def_s)=-0.512 player corr=0.828
        AUDIT-FIX [+1,-1] -> (off-def)+noise       corr(g_off-g_def, off-def)=+0.736 corr(g_off+g_def, off+def)=+0.452 corr(g_def, def_s)=+0.170 corr(-g_def, def_s)=-0.170 player corr=0.833
        PROPOSED [+1,+1] -> (off+def)+noise        corr(g_off-g_def, off-def)=+0.688 corr(g_off+g_def, off+def)=+0.542 corr(g_def, def_s)=+0.540 corr(-g_def, def_s)=-0.540 player corr=0.828
        --- gauge-invariant team effects (intercept + on-field-weighted player ratings) ---
        no market                    corr(E_off,true_off)=+0.963 corr(E_def,true_def)=+0.943 corr(E_off+E_def,true_off+true_def)=+0.959 corr(E_off-E_def, true_off-true_def)=+0.961
        CURRENT                      corr(E_off,true_off)=+0.962 corr(E_def,true_def)=+0.945 corr(E_off+E_def,true_off+true_def)=+0.958 corr(E_off-E_def, true_off-true_def)=+0.961
        AUDIT-FIX                    corr(E_off,true_off)=+0.965 corr(E_def,true_def)=+0.940 corr(E_off+E_def,true_off+true_def)=+0.959 corr(E_off-E_def, true_off-true_def)=+0.962
        PROPOSED(off+def,[+1,+1])    corr(E_off,true_off)=+0.962 corr(E_def,true_def)=+0.945 corr(E_off+E_def,true_off+true_def)=+0.958 corr(E_off-E_def, true_off-true_def)=+0.961
Origin
    Consolidation inventory scratch ``scratch-reconcile-code-first/sign_check.py``
    (reconcile-code-first report section 6, C12/C13). Logic unchanged (one unused local,
    ``pos``, dropped); the path bootstrap, ``--synth`` switch, banner and this header were added.
"""
from __future__ import annotations

import argparse
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2]))
from tools.investigations._common import add_synth_arg, banner, bootstrap, select_synth  # noqa: E402

bootstrap()

from collections import Counter  # noqa: E402

import numpy as np  # noqa: E402
from threadpoolctl import threadpool_limits  # noqa: E402

from backend.grid import attach_dv, fit_value_model, load_synthetic  # noqa: E402
from backend.grid.layers import _solve_ridge_prior, build_design  # noqa: E402


def main(argv=None) -> None:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    add_synth_arg(ap)
    args = ap.parse_args(argv)
    banner("sign_check", select_synth(args.synth))
    with threadpool_limits(1):
        b = load_synthetic()
        plays, players, gt = b["plays"], b["players"], b["gt"]
        plays = attach_dv(plays, fit_value_model(plays))
    X, y, ci = build_design(plays, players)
    teams = ci["teams"]
    nC = ci["nCols"]
    XtX0 = (X.T @ X).toarray()
    Xty0 = X.T @ y
    mask = np.ones(nC)
    for t in teams:
        mask[ci["t_off"][t]] = 0.05
        mask[ci["t_def"][t]] = 0.05
    ab = gt["ability"]
    P = players
    off_s = {t: P[(P.team == t) & P.position.isin(["QB", "RB", "WR", "TE"]) & P.is_starter].ability.mean() for t in teams}
    def_s = {t: P[(P.team == t) & (P.position == "DEF") & P.is_starter].ability.mean() for t in teams}
    planted_minus = np.array([off_s[t] - def_s[t] for t in teams])   # synth _team_strength (current)
    planted_plus = np.array([off_s[t] + def_s[t] for t in teams])    # off + def (good D positive)
    rng = np.random.default_rng(1)
    noise = rng.normal(0, 0.01, len(teams))

    def solve(sign_def, target):
        XtX, Xty = XtX0.copy(), Xty0.copy()
        if target is not None:
            for i, t in enumerate(teams):
                row = np.zeros(nC)
                row[ci["t_off"][t]] = 1.0
                row[ci["t_def"][t]] = sign_def
                XtX += 40 * np.outer(row, row)
                Xty += 40 * row * target[i]
        return _solve_ridge_prior(XtX, Xty, 120.0, np.zeros(nC), mask)

    def rep(name, beta):
        go = np.array([beta[ci["t_off"][t]] for t in teams])
        gd = np.array([beta[ci["t_def"][t]] for t in teams])
        pr = beta[:ci["nP"]]
        order = [k for k, _ in sorted(ci["p_col"].items(), key=lambda kv: kv[1])]
        abil = np.array([ab[k] for k in order])
        print(f"{name:42s} corr(g_off-g_def, off-def)={np.corrcoef(go - gd, planted_minus)[0, 1]:+.3f} "
              f"corr(g_off+g_def, off+def)={np.corrcoef(go + gd, planted_plus)[0, 1]:+.3f} "
              f"corr(g_def, def_s)={np.corrcoef(gd, [def_s[t] for t in teams])[0, 1]:+.3f} "
              f"corr(-g_def, def_s)={np.corrcoef(-gd, [def_s[t] for t in teams])[0, 1]:+.3f} "
              f"player corr={np.corrcoef(pr, abil)[0, 1]:.3f}")

    rep("no market", solve(1.0, None))
    rep("CURRENT [+1,+1] -> (off-def)+noise", solve(1.0, planted_minus + noise))
    rep("AUDIT-FIX [+1,-1] -> (off-def)+noise", solve(-1.0, planted_minus + noise))
    rep("PROPOSED [+1,+1] -> (off+def)+noise", solve(1.0, planted_plus + noise))
    print("--- gauge-invariant team effects (intercept + on-field-weighted player ratings) ---")
    offc = {t: Counter() for t in teams}
    defc = {t: Counter() for t in teams}
    for o, d, op, dp in zip(plays.off_team, plays.def_team, plays.off_players, plays.def_players):
        offc[o].update(op)
        defc[d].update(dp)

    def gi(beta):
        pc = ci["p_col"]
        Eo = np.array([beta[ci["t_off"][t]] + sum(beta[pc[p]] * n for p, n in offc[t].items()) / sum(offc[t].values()) * 5 for t in teams])
        Ed = np.array([beta[ci["t_def"][t]] + sum(beta[pc[p]] * n for p, n in defc[t].items()) / sum(defc[t].values()) * 7 for t in teams])
        to = np.array([sum(ab[p] * n for p, n in offc[t].items()) / sum(offc[t].values()) * 5 for t in teams])
        td = np.array([sum(ab[p] * n for p, n in defc[t].items()) / sum(defc[t].values()) * 7 for t in teams])
        return Eo, Ed, to, td

    for name, beta in [("no market", solve(1.0, None)), ("CURRENT", solve(1.0, planted_minus + noise)),
                       ("AUDIT-FIX", solve(-1.0, planted_minus + noise)),
                       ("PROPOSED(off+def,[+1,+1])", solve(1.0, planted_plus + noise))]:
        Eo, Ed, to, td = gi(beta)
        print(f"{name:28s} corr(E_off,true_off)={np.corrcoef(Eo, to)[0, 1]:+.3f} "
              f"corr(E_def,true_def)={np.corrcoef(Ed, td)[0, 1]:+.3f} "
              f"corr(E_off+E_def,true_off+true_def)={np.corrcoef(Eo + Ed, to + td)[0, 1]:+.3f} "
              f"corr(E_off-E_def, true_off-true_def)={np.corrcoef(Eo - Ed, to - td)[0, 1]:+.3f}")


if __name__ == "__main__":
    main()
