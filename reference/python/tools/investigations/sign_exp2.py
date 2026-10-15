"""Net-strength anchor and player recovery, legacy vs defender-fixed generator.

What it demonstrates
    Prints the share of plays whose defenders belong to ``def_team`` (0.0 on the legacy
    generator: KI-NEW-Y0), then on the canonical ``load_synthetic()`` league
    (single-threaded) fits ``run_rapm`` with no market and with a market anchored on planted
    **net** strength (off + def quality, + N(0, 0.01) noise) using the shipped ``[+1,+1]``
    rows, and reports: corr(gamma_off + gamma_def, planted net), corr(gamma_off - gamma_def,
    planted net) (the oracle's current ``team_rating``), per-position player recovery, and
    the realized point margin against both planted conventions.
    Recorded by the inventory (reconcile-spec-first C1) on the **fixed** generator: margin vs
    net 0.855 (vs off - def 0.754); gamma_off + gamma_def vs net 0.431 without a market and
    0.542 with the net-anchored ``[+1,+1]`` market, against 0.412 for the current
    ``team_rating``; player recovery QB 0.887, RB 0.767, WR 0.851, TE 0.775, DEF 0.782.
Evidence for
    KI-NEW-Y0 (defender share line), KI-NEW-A1 / KI-G1; input to DR-B4 and DR-B5 (proposed
    defaults, pending the owner: fix the generator, plant net strength, keep ``[+1,+1]``).
Synthetic generator
    Both. The decisive numbers are ``--synth fixed`` (defenders from def_team, in memory;
    see _common.py); ``--synth legacy`` (default) shows the bug itself.
Run (from reference/python, about 5 s each)
    python3 -m tools.investigations.sign_exp2 --synth fixed
    python3 -m tools.investigations.sign_exp2 --synth legacy
Expected output (recorded 2026-10-01 with requirements.lock pins)
    --synth fixed:
        [sign_exp2] backend imported from ./backend | synth generator: defender-fixed (in memory) | requested --synth fixed
        def_players from def_team: 1.0
        no market
          corr(g_off + g_def, planted net)    = 0.431
          corr(g_off - g_def, planted net)    = 0.394  (current team_rating)
           QB corr=0.887   RB corr=0.767   WR corr=0.851   TE corr=0.775   DEF corr=0.782
        market=net(off+def)+noise
          corr(g_off + g_def, planted net)    = 0.542
          corr(g_off - g_def, planted net)    = 0.412  (current team_rating)
           QB corr=0.898   RB corr=0.770   WR corr=0.862   TE corr=0.790   DEF corr=0.779
        corr(realized margin, planted net) = 0.855 ; vs off-def = 0.754
    --synth legacy:
        [sign_exp2] backend imported from ./backend | synth generator: legacy (oracle as imported, KI-NEW-Y0) | requested --synth legacy
        def_players from def_team: 0.0
        no market
          corr(g_off + g_def, planted net)    = 0.471
          corr(g_off - g_def, planted net)    = 0.482  (current team_rating)
           QB corr=0.875   RB corr=0.738   WR corr=0.787   TE corr=0.750   DEF corr=0.764
        market=net(off+def)+noise
          corr(g_off + g_def, planted net)    = 0.547
          corr(g_off - g_def, planted net)    = 0.539  (current team_rating)
           QB corr=0.883   RB corr=0.741   WR corr=0.797   TE corr=0.776   DEF corr=0.770
        corr(realized margin, planted net) = 0.427 ; vs off-def = 0.719
Origin
    Consolidation inventory scratch ``scratch-specfirst/sign_exp2.py`` run in
    ``scratch-specfirst/cn`` (legacy) and ``scratch-specfirst/cnfix`` (fixed copy of
    synth.py), reconcile-spec-first report section 4, C1. Logic unchanged (one unused local,
    ``a_omd``, dropped); the path bootstrap, ``--synth`` switch (replacing the patched copy), banner and this header were
    added.
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
    banner("sign_exp2", select_synth(args.synth))
    with threadpool_limits(1):
        b = load_synthetic()
        plays, players, gt = b["plays"], b["players"], b["gt"]
        pl = players.set_index("player_id").team.to_dict()
        print("def_players from def_team:",
              np.mean([all(pl[x] == d for x in dp) for d, dp in zip(plays.def_team, plays.def_players)]))
        plays = attach_dv(plays, fit_value_model(plays))
        off_q, def_q = {}, {}
        for t in players.team.unique():
            tp = players[players.team == t]
            off_q[int(t)] = tp[(tp.position.isin(["QB", "RB", "WR", "TE"])) & tp.is_starter].ability.mean()
            def_q[int(t)] = tp[(tp.position == "DEF") & tp.is_starter].ability.mean()
        p_omd = {t: off_q[t] - def_q[t] for t in off_q}
        p_net = {t: off_q[t] + def_q[t] for t in off_q}
        rng = np.random.default_rng(1)
        for label, mkt in [("no market", None),
                           ("market=net(off+def)+noise", {t: v + rng.normal(0, 0.01) for t, v in p_net.items()})]:
            r, tdf, beta, ci = run_rapm(plays, players, market_strength=mkt)
            teams = ci["teams"]
            go = np.array([beta[ci["t_off"][t]] for t in teams])
            gd = np.array([beta[ci["t_def"][t]] for t in teams])
            a_net = np.array([p_net[int(t)] for t in teams])
            print(label)
            print("  corr(g_off + g_def, planted net)    = %.3f" % np.corrcoef(go + gd, a_net)[0, 1])
            print("  corr(g_off - g_def, planted net)    = %.3f  (current team_rating)" % np.corrcoef(go - gd, a_net)[0, 1])
            for pos in ["QB", "RB", "WR", "TE", "DEF"]:
                s = r[r.position == pos]
                print("   %s corr=%.3f" % (pos, np.corrcoef(s.rating, s.ability)[0, 1]), end="")
            print()
        pts_for = plays.groupby("off_team").points.sum()
        pts_ag = plays.groupby("def_team").points.sum()
        m = (pts_for - pts_ag).sort_index()
        print("corr(realized margin, planted net) = %.3f ; vs off-def = %.3f" % (
            np.corrcoef(m.values, [p_net[int(t)] for t in m.index])[0, 1],
            np.corrcoef(m.values, [p_omd[int(t)] for t in m.index])[0, 1]))


if __name__ == "__main__":
    main()
