"""Matchup-grade sign on a planted-defence league: the shipped grade is inverted.

What it demonstrates
    The oracle stores the defensive matchup grade as ``-beta[t_def]`` and documents it as
    "higher = tougher" (``weekly_update.py:62-66,84``; ``backtest.py:48-51,185-186``). Because
    defenders enter the RAPM design with -1, a *stronger* defence fits a *larger*
    ``beta[t_def]``, so the shipped grade ranks the ELITE defence easiest. This script plants
    team-level defence effects (ELITE -0.5, three AVG 0, WEAK +0.5 dV per play) on 20,000
    plays with fixed 4-man offences and 7-man defences, fits ``run_rapm``, and prints
    ``beta_def``, the shipped grade and the defenders' summed ratings per team.
Evidence for
    KI-NEW-A2 (matchup-grade sign inverted; CONFIRMED). Input to DR-B5 (proposed default,
    pending the owner: grade = +E_def, higher = tougher). Note the oracle's own unit tests
    (tests/pipeline/test_weekly_update.py:286-370) plant the wrong sign by hand, so they pass.
Synthetic generator
    None of the oracle's: its own tiny generator with defenders drawn from the defending team,
    so this evidence is valid regardless of KI-NEW-Y0 (critic X-3).
Run (from reference/python, about 3 s)
    python3 -m tools.investigations.defsign_planted
Expected output (recorded 2026-10-01 with requirements.lock pins)
    ELITE  b_def=+0.409  shipped grade (-b_def)=-0.409  sum def-player ratings=+0.143
    AVG1   b_def=-0.008  shipped grade (-b_def)=+0.008  sum def-player ratings=-0.003
    AVG2   b_def=-0.012  shipped grade (-b_def)=+0.012  sum def-player ratings=-0.004
    AVG3   b_def=+0.005  shipped grade (-b_def)=-0.005  sum def-player ratings=+0.002
    WEAK   b_def=-0.382  shipped grade (-b_def)=+0.382  sum def-player ratings=-0.134
Origin
    Consolidation inventory scratch ``scratch-cnissues/defsign_planted.py`` (cn-issues report
    section 3.1, NEW-A2). Logic unchanged; only the path bootstrap and this header were added.
"""
from __future__ import annotations

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2]))
from tools.investigations._common import bootstrap  # noqa: E402

bootstrap()

import numpy as np  # noqa: E402
import pandas as pd  # noqa: E402

import backend.grid.layers as L  # noqa: E402


def main() -> None:
    rng = np.random.default_rng(0)
    teams = ["ELITE", "AVG1", "AVG2", "AVG3", "WEAK"]
    # effect on the offense's dV when facing this defense
    def_effect = {"ELITE": -0.5, "AVG1": 0, "AVG2": 0, "AVG3": 0, "WEAK": +0.5}
    players = []
    roster = {}
    for t in teams:
        roster[t] = {"off": [f"O{t}{i}" for i in range(4)], "def": [f"D{t}{i}" for i in range(7)]}
        for p in roster[t]["off"]:
            players.append(dict(player_id=p, team=t, position="WR"))
        for p in roster[t]["def"]:
            players.append(dict(player_id=p, team=t, position="DEF"))
    players = pd.DataFrame(players)
    rows = []
    for _ in range(20000):
        o, d = rng.choice(teams, 2, replace=False)
        rows.append(dict(off_team=o, def_team=d, off_players=tuple(roster[o]["off"]),
                         def_players=tuple(roster[d]["def"]), dv=def_effect[d] + rng.normal(0, 1.0)))
    plays = pd.DataFrame(rows)
    ratings, team_df, beta, ci = L.run_rapm(plays, players)
    for t in teams:
        print(f"{t:6s} b_def={beta[ci['t_def'][t]]:+.3f}  shipped grade (-b_def)={-beta[ci['t_def'][t]]:+.3f}  "
              f"sum def-player ratings={ratings[ratings.team.eq(t) & ratings.position.eq('DEF')].rating.sum():+.3f}")


if __name__ == "__main__":
    main()
