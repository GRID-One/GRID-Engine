"""Tier-2 KPI on engine-produced defensive grades, both signs (legacy-synth evidence).

What it demonstrates
    The Tier-2 matchup-grade check (``backend/validation/tier2.py``) had only ever been run
    on hand-planted grades. This script produces grades the way the engine does (walk-forward
    origins from ``backend/validation/backtest.walk_forward``, warm-up 4 weeks, intercept-only
    ``def_grades``), scores them with ``tier2_report`` against yards allowed, as shipped
    (``-beta_def``) and flipped (``+beta_def``), and correlates the final grade with the
    planted mean DEF-starter ability. Seeds 7, 11, 23; 16 teams x 16 weeks.
    Result recorded by the inventory: the KPI is about 0 for either sign, with CIs that
    include 0, so an intercept-only grade carries no validated signal on this generator.
Evidence for
    KI-NEW-V2 (Tier-2 never exercised on engine-produced grades). Context for KI-NEW-A2 and
    DR-B5. **Read with KI-NEW-Y0:** on the legacy generator a team's "defence" is its own
    DEF players defending its own offence, so the planted-truth correlation here is not a
    valid sign test (use ``defsign_planted.py`` for that).
Synthetic generator
    Evidence was recorded on ``--synth legacy`` (the default). ``--synth fixed`` re-runs it on
    the defender-fixed generator (in memory; see _common.py).
Run (from reference/python, about 13 s)
    python3 -m tools.investigations.defgrade_test [--synth legacy|fixed]
Expected output, --synth legacy (recorded 2026-10-01 with requirements.lock pins)
    [defgrade_test] backend imported from ./backend | synth generator: legacy (oracle as imported, KI-NEW-Y0) | requested --synth legacy
    seed 7: tier2 predictive (as shipped, -b_def) point=-0.007 CI=[-0.146,+0.111]  | flipped (+b_def) point=+0.007 | corr(shipped grade, true DEF ability)=+0.188
    seed 11: tier2 predictive (as shipped, -b_def) point=+0.039 CI=[-0.090,+0.182]  | flipped (+b_def) point=-0.039 | corr(shipped grade, true DEF ability)=+0.083
    seed 23: tier2 predictive (as shipped, -b_def) point=-0.076 CI=[-0.195,+0.044]  | flipped (+b_def) point=+0.076 | corr(shipped grade, true DEF ability)=-0.040
Origin
    Consolidation inventory scratch ``scratch-cnissues/defgrade_test.py`` (cn-issues report
    section 3.9, NEW-V2). Logic unchanged; the path bootstrap, ``--synth`` switch, banner and
    this header were added.
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

from backend.grid.synth import SynthConfig, simulate  # noqa: E402
from backend.validation.backtest import walk_forward  # noqa: E402
from backend.validation.tier2 import tier2_report  # noqa: E402


def main(argv=None) -> None:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    add_synth_arg(ap)
    args = ap.parse_args(argv)
    banner("defgrade_test", select_synth(args.synth))
    for seed in (7, 11, 23):
        plays, players, gt = simulate(SynthConfig(seed=seed, n_teams=16, weeks=16, yards_noise_sd=3.2,
                                                  drives_per_team_per_game=12))
        plays["season"] = 2023
        origins = walk_forward(plays, players, warmup_weeks=4)
        grades = pd.DataFrame([{"season": o.season, "week": o.week, "def_team": t, "rapm_grade": g}
                               for o in origins for t, g in o.def_grades.items()])
        allowed = plays.groupby(["season", "week", "def_team"]).yards.sum().rename("points_allowed").reset_index()
        rep = tier2_report(grades, allowed, min_teams=6, n=2000)
        # also flipped
        g2 = grades.assign(rapm_grade=-grades.rapm_grade)
        rep2 = tier2_report(g2, allowed, min_teams=6, n=2000)
        # truth: mean starter DEF ability per team vs final grade
        dtrue = players[(players.position == "DEF") & players.is_starter].groupby("team").ability.mean()
        last = origins[-1].def_grades
        c = np.corrcoef([last[t] for t in dtrue.index], dtrue.values)[0, 1]
        print(f"seed {seed}: tier2 predictive (as shipped, -b_def) point={rep['predictive'].point:+.3f} "
              f"CI=[{rep['predictive'].lower:+.3f},{rep['predictive'].upper:+.3f}]  "
              f"| flipped (+b_def) point={rep2['predictive'].point:+.3f} | "
              f"corr(shipped grade, true DEF ability)={c:+.3f}")


if __name__ == "__main__":
    main()
