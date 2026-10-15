"""Tier-2 KPI on a "total defensive effect" grade (legacy-synth evidence).

What it demonstrates
    Follow-up to ``defgrade_test.py``: instead of the intercept-only grade, scores a total
    defensive effect per team and origin, ``+beta_def`` plus the mean per-play sum of the
    defenders' as-of-origin ratings, with ``tier2_report`` against yards allowed, and
    correlates the final total and intercept-only grades with planted DEF ability. Seeds 7,
    11, 23; 16 teams x 16 weeks; walk-forward warm-up 4 weeks.
    Result recorded by the inventory: adding on-field defender ratings did not produce a
    validated signal on this generator either.
Evidence for
    KI-NEW-V2; context for DR-B5 (proposed default, pending the owner: grade = +E_def, the
    gauge-invariant total defensive effect). **Read with KI-NEW-Y0:** on the legacy
    generator the defenders on a play belong to the offense, so "defender ratings" here are
    the offense team's own DEF players; re-measure on the fixed generator before drawing a
    conclusion about the grade definition.
Synthetic generator
    Evidence was recorded on ``--synth legacy`` (the default). ``--synth fixed`` re-runs it on
    the defender-fixed generator (in memory; see _common.py).
Run (from reference/python, about 14 s)
    python3 -m tools.investigations.defgrade_test2 [--synth legacy|fixed]
Expected output, --synth legacy (recorded 2026-10-01 with requirements.lock pins)
    [defgrade_test2] backend imported from ./backend | synth generator: legacy (oracle as imported, KI-NEW-Y0) | requested --synth legacy
    seed 7: total-effect grade tier2 point=-0.070 CI=[-0.169,+0.034] corr(total, truth)=+0.049 corr(+b_def, truth)=-0.188
    seed 11: total-effect grade tier2 point=-0.011 CI=[-0.210,+0.158] corr(total, truth)=-0.284 corr(+b_def, truth)=-0.083
    seed 23: total-effect grade tier2 point=+0.116 CI=[-0.002,+0.231] corr(total, truth)=-0.379 corr(+b_def, truth)=+0.040
Origin
    Consolidation inventory scratch ``scratch-cnissues/defgrade_test2.py`` (cn-issues report
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
    banner("defgrade_test2", select_synth(args.synth))
    for seed in (7, 11, 23):
        plays, players, gt = simulate(SynthConfig(seed=seed, n_teams=16, weeks=16, yards_noise_sd=3.2,
                                                  drives_per_team_per_game=12))
        plays["season"] = 2023
        origins = walk_forward(plays, players, warmup_weeks=4)
        # total defensive effect = +b_def + mean per-play sum of defender ratings (as of origin)
        rows = []
        for o in origins:
            prior = plays[plays.week < o.week]
            for t, g in o.def_grades.items():
                dp = prior[prior.def_team == t].def_players
                s = np.mean([sum(o.ratings.get(p, 0.0) for p in tup) for tup in dp]) if len(dp) else 0.0
                rows.append({"season": o.season, "week": o.week, "def_team": t, "rapm_grade": -g + s, "int_only": -g})
        grades = pd.DataFrame(rows)
        allowed = plays.groupby(["season", "week", "def_team"]).yards.sum().rename("points_allowed").reset_index()
        rep = tier2_report(grades[["season", "week", "def_team", "rapm_grade"]], allowed, n=2000)
        dtrue = players[(players.position == "DEF") & players.is_starter].groupby("team").ability.mean()
        last = grades[grades.week == grades.week.max()].set_index("def_team")
        print(f"seed {seed}: total-effect grade tier2 point={rep['predictive'].point:+.3f} "
              f"CI=[{rep['predictive'].lower:+.3f},{rep['predictive'].upper:+.3f}] "
              f"corr(total, truth)={np.corrcoef(last.loc[dtrue.index, 'rapm_grade'], dtrue.values)[0, 1]:+.3f} "
              f"corr(+b_def, truth)={np.corrcoef(last.loc[dtrue.index, 'int_only'], dtrue.values)[0, 1]:+.3f}")


if __name__ == "__main__":
    main()
