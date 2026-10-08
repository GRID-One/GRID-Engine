"""Matchup-grade sign on the canonical synth, legacy vs defender-fixed generator.

What it demonstrates
    On the canonical ``load_synthetic()`` league (single-threaded), fits ``run_rapm`` without
    a market and correlates ``beta_def`` with planted DEF quality, and ``beta_def`` and the
    stored grade ``-beta_def`` ("higher = tougher" per the oracle) with realized points
    allowed. Recorded (reconcile-spec-first C2):

        synth    corr(beta_def, planted DEF quality)   corr(-beta_def, points allowed)
        legacy   -0.22 (meaningless: KI-NEW-Y0)          +0.41
        fixed    +0.53                                   +0.59

    On the fixed generator a stronger defence has a larger ``beta_def``, and the stored
    grade rises with points allowed: it is an "easiness" score, the inverse of its label.
Evidence for
    KI-NEW-A2 (agrees with ``defsign_planted.py``); the legacy row is also evidence for
    KI-NEW-Y0. Input to DR-B5 (proposed default, pending the owner: grade = +E_def).
Synthetic generator
    Both; the decisive row is ``--synth fixed`` (defenders from def_team, in memory; see
    _common.py). ``--synth legacy`` (default) reproduces the misleading -0.22.
Run (from reference/python, about 4 s each)
    python3 -m tools.investigations.def_sign --synth fixed
    python3 -m tools.investigations.def_sign --synth legacy
Expected output (recorded 2026-10-01 with requirements.lock pins)
    --synth fixed:
        [def_sign] backend imported from ./backend | synth generator: defender-fixed (in memory) | requested --synth fixed
        corr(beta_def, planted def quality) = 0.529
        corr(beta_def, realized points allowed) = -0.585
        corr(-beta_def [stored rapm_grade, 'higher=tougher'], points allowed) = 0.585
    --synth legacy:
        [def_sign] backend imported from ./backend | synth generator: legacy (oracle as imported, KI-NEW-Y0) | requested --synth legacy
        corr(beta_def, planted def quality) = -0.223
        corr(beta_def, realized points allowed) = -0.414
        corr(-beta_def [stored rapm_grade, 'higher=tougher'], points allowed) = 0.414
Origin
    Consolidation inventory scratch ``scratch-specfirst/def_sign.py`` run in
    ``scratch-specfirst/cn`` and ``scratch-specfirst/cnfix`` (reconcile-spec-first report
    section 4, C2). Logic unchanged; the path bootstrap, ``--synth`` switch (replacing the
    patched copy), banner and this header were added.
"""
from __future__ import annotations

import argparse
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2]))
from tools.investigations._common import add_synth_arg, banner, bootstrap, select_synth  # noqa: E402

bootstrap()

import numpy as np  # noqa: E402
from threadpoolctl import threadpool_limits  # noqa: E402

from backend.grid import attach_dv, fit_value_model, load_synthetic  # noqa: E402
from backend.grid.layers import run_rapm  # noqa: E402


def main(argv=None) -> None:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    add_synth_arg(ap)
    args = ap.parse_args(argv)
    banner("def_sign", select_synth(args.synth))
    with threadpool_limits(1):
        b = load_synthetic()
        plays, players, gt = b["plays"], b["players"], b["gt"]
        plays = attach_dv(plays, fit_value_model(plays))
        r, tdf, beta, ci = run_rapm(plays, players)
        teams = ci["teams"]
        def_q = {int(t): players[(players.team == t) & (players.position == "DEF") & players.is_starter].ability.mean()
                 for t in teams}
        bd = np.array([beta[ci["t_def"][t]] for t in teams])
        dq = np.array([def_q[int(t)] for t in teams])
        # realized points allowed per game by each defense
        pa = plays.groupby("def_team").points.sum()
        pa = np.array([pa[t] for t in teams])
        print("corr(beta_def, planted def quality) = %.3f" % np.corrcoef(bd, dq)[0, 1])
        print("corr(beta_def, realized points allowed) = %.3f" % np.corrcoef(bd, pa)[0, 1])
        print("corr(-beta_def [stored rapm_grade, 'higher=tougher'], points allowed) = %.3f"
              % np.corrcoef(-bd, pa)[0, 1])


if __name__ == "__main__":
    main()
