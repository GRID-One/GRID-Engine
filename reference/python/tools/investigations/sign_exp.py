"""Team-strength estimand: which planted convention do the intercepts and margins track?

What it demonstrates
    On the canonical ``load_synthetic()`` league (single-threaded), fits ``run_rapm`` with no
    market and with the market anchored on ``gt["team_strength"]`` (off - def), and reports
    how the intercept combinations ``gamma_off - gamma_def`` and ``gamma_off + gamma_def``
    track the planted off - def and off + def ("net") conventions; then correlates each
    team's realized point margin with both conventions.
    Recorded on the legacy generator (reconcile-spec-first C1): corr(gamma_off - gamma_def,
    off - def) 0.693; corr(gamma_off + gamma_def, off + def) 0.471; realized margin vs
    off + def 0.427 and vs off - def 0.719 -- i.e. on the legacy world the *margin* follows
    off - def, which only makes sense because defenders come from the offense.
Evidence for
    KI-NEW-Y0 (via the margin check) and KI-NEW-A1 / KI-G1; input to DR-B4 and DR-B5
    (proposed defaults, pending the owner).
Synthetic generator
    ``--synth legacy`` (default, the oracle as imported) and ``--synth fixed`` (defenders from
    def_team, in memory; see _common.py). The inventory ran this on both; on the fixed
    generator use ``sign_exp2.py``, which anchors the market on net strength.
Run (from reference/python, about 5 s)
    python3 -m tools.investigations.sign_exp [--synth legacy|fixed]
Expected output (recorded 2026-10-01 with requirements.lock pins)
    --synth legacy (the recorded evidence):
        [sign_exp] backend imported from ./backend | synth generator: legacy (oracle as imported, KI-NEW-Y0) | requested --synth legacy
        no market
          corr(g_off - g_def, planted off-def) = 0.693
          corr(g_off + g_def, planted off+def) = 0.471
          corr(g_off - g_def, planted off+def) = 0.482
        market=gt(off-def)
          corr(g_off - g_def, planted off-def) = 0.692
          corr(g_off + g_def, planted off+def) = 0.528
          corr(g_off - g_def, planted off+def) = 0.525
        corr(realized point margin, planted off+def) = 0.427
        corr(realized point margin, planted off-def) = 0.719
    --synth fixed:
        [sign_exp] backend imported from ./backend | synth generator: defender-fixed (in memory) | requested --synth fixed
        no market
          corr(g_off - g_def, planted off-def) = 0.684
          corr(g_off + g_def, planted off+def) = 0.431
          corr(g_off - g_def, planted off+def) = 0.394
        market=gt(off-def)
          corr(g_off - g_def, planted off-def) = 0.687
          corr(g_off + g_def, planted off+def) = 0.536
          corr(g_off - g_def, planted off+def) = 0.414
        corr(realized point margin, planted off+def) = 0.855
        corr(realized point margin, planted off-def) = 0.754
Origin
    Consolidation inventory scratch ``scratch-specfirst/sign_exp.py`` (reconcile-spec-first
    report section 4, C1). Logic unchanged (unused locals ``ab``, ``rr`` and ``tot_off``
    dropped; they never reached the output); the path bootstrap, ``--synth`` switch, banner
    and this header were added.
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
    banner("sign_exp", select_synth(args.synth))
    with threadpool_limits(1):
        b = load_synthetic()
        plays, players, gt = b["plays"], b["players"], b["gt"]
        plays = attach_dv(plays, fit_value_model(plays))
        # planted components
        off_q, def_q = {}, {}
        for t in players.team.unique():
            tp = players[players.team == t]
            off_q[int(t)] = tp[(tp.position.isin(["QB", "RB", "WR", "TE"])) & tp.is_starter].ability.mean()
            def_q[int(t)] = tp[(tp.position == "DEF") & tp.is_starter].ability.mean()
        planted_offminusdef = {t: off_q[t] - def_q[t] for t in off_q}   # == gt team_strength
        planted_net = {t: off_q[t] + def_q[t] for t in off_q}           # off quality + def quality
        for label, mkt in [("no market", None), ("market=gt(off-def)", gt["team_strength"])]:
            r, tdf, beta, ci = run_rapm(plays, players, market_strength=mkt)
            teams = ci["teams"]
            go = np.array([beta[ci["t_off"][t]] for t in teams])
            gd = np.array([beta[ci["t_def"][t]] for t in teams])
            p_omd = np.array([planted_offminusdef[int(t)] for t in teams])
            p_net = np.array([planted_net[int(t)] for t in teams])
            print(label)
            print("  corr(g_off - g_def, planted off-def) = %.3f" % np.corrcoef(go - gd, p_omd)[0, 1])
            print("  corr(g_off + g_def, planted off+def) = %.3f" % np.corrcoef(go + gd, p_net)[0, 1])
            print("  corr(g_off - g_def, planted off+def) = %.3f" % np.corrcoef(go - gd, p_net)[0, 1])
        # realized per-team point margin (net strength proxy)
        pts_for = plays.groupby("off_team").points.sum()
        pts_ag = plays.groupby("def_team").points.sum()
        margin = (pts_for - pts_ag)
        m = np.array([margin[t] for t in sorted(margin.index)])
        print("corr(realized point margin, planted off+def) = %.3f"
              % np.corrcoef(m, [planted_net[int(t)] for t in sorted(margin.index)])[0, 1])
        print("corr(realized point margin, planted off-def) = %.3f"
              % np.corrcoef(m, [planted_offminusdef[int(t)] for t in sorted(margin.index)])[0, 1])


if __name__ == "__main__":
    main()
