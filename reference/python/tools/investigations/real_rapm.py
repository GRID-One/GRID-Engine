"""Real 2023 single-season RAPM scale vs the synthetic scale, and QB identifiability.

What it demonstrates
    Builds the real 2023 regular-season plays contract (weeks <= 18) with the oracle's
    adapter, fits the oracle's V(s) and ``run_rapm`` on it, and reports the rating SD by
    position for players who took the field; repeats the fit on a canonical-shaped synthetic
    league for comparison; then lists the top and bottom starting QBs (> 300 snaps) and the
    correlation of a starter QB's rating with his own team-offense intercept.
    Recorded result (cn-issues NEW-A3 / NEW-P4): real rating SD about 0.04-0.046 at every
    position (QB 0.042) against synthetic QB 0.259 and 0.07-0.09 elsewhere; top QBs by rating
    include Browning, Flacco and O'Connell, with Hurts and Young near the bottom;
    corr(starter QB rating, own t_off) = 0.13. Every hand-set scale constant (PRIOR_SD, the
    age/draft steps, KalmanState.init P0, SSParams q/r) is synth-calibrated.
Evidence for
    KI-NEW-A3 (QB column nearly collinear with the team-offense intercept on real data),
    KI-NEW-P4 (scale constants calibrated on synthetic units); context for DR-C1 and DR-B4
    (proposed defaults, pending the owner). Real-data numbers are historical, non-parity.
Data and synthetic generator
    Real 2023 nflverse inputs (``pbp_2023``, ``part_2023``, ``roster_2023``) from
    ``fetch_realdata.py``. The synthetic comparison row was recorded on ``--synth legacy``
    (default); ``--synth fixed`` re-runs it on the defender-fixed generator (in memory).
Run (from reference/python, after fetch_realdata; about 38 s)
    python3 -m tools.investigations.real_rapm [--synth legacy|fixed]
Expected output, --synth legacy (recorded 2026-10-01 with requirements.lock pins and the pinned inputs)
    [real_rapm] backend imported from ./backend | synth generator: legacy (oracle as imported, KI-NEW-Y0) | requested --synth legacy
    real plays 33836 dv sd 1.135 players on field 1602 secs 34.4   (secs = wall time; varies)
    real RAPM rating SD by position: {'DB': 0.0411, 'DL': 0.0445, 'LB': 0.0405, 'QB': 0.0422, 'RB': 0.0441, 'TE': 0.0462, 'WR': 0.0454}
    synth dv sd 1.155 synth RAPM rating SD by pos: {'DEF': 0.0794, 'QB': 0.2593, 'RB': 0.0917, 'TE': 0.0711, 'WR': 0.0811}
    team_rating SD real 0.1038
    real offensive snaps/season median (skill players w/ >100): 429
                name team   rating  snaps
    Matthew Stafford   LA 0.086745    946
       Jake Browning  CIN 0.085339    448
          Joe Flacco  CLE 0.079077    329
     Aidan O'Connell   LV 0.055850    618
         C.J. Stroud  HOU 0.053270    918
     Patrick Mahomes   KC 0.051105    976
         Zach Wilson  NYJ 0.049808    649
      Deshaun Watson  CLE 0.043982    364
               name team    rating  snaps
        Taysom Hill   NO -0.022842    392
        Jalen Hurts  PHI -0.031017   1057
    Jimmy Garoppolo   LV -0.040633    304
        Bryce Young  CAR -0.047519   1012
         Sam Howell  WAS -0.114035   1013
    corr(QB rating, own team off intercept) among starters: 0.132
    team off intercept SD 0.0718
Origin
    Consolidation inventory scratch ``scratch-cnissues/real_rapm.py`` (cn-issues report
    sections 0.6, 3.1 and 3.4). Logic unchanged (one unused import, ``SSParams``, dropped);
    the input paths now come from ``data/realdata/``, and the path bootstrap, ``--synth``
    switch, banner and this header were added.
"""
from __future__ import annotations

import argparse
import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2]))
from tools.investigations._common import add_synth_arg, banner, bootstrap, realdata, select_synth  # noqa: E402

bootstrap()

import time  # noqa: E402

import numpy as np  # noqa: E402
import pandas as pd  # noqa: E402

from backend.grid.layers import run_rapm  # noqa: E402
from backend.grid.nflverse_adapter import build_plays_contract  # noqa: E402
from backend.grid.nflverse_loader import _normalize_participation, _normalize_roster  # noqa: E402
from backend.grid.synth import SynthConfig, simulate  # noqa: E402
from backend.grid.value import attach_dv, fit_value_model  # noqa: E402


def main(argv=None) -> None:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    add_synth_arg(ap)
    args = ap.parse_args(argv)
    banner("real_rapm", select_synth(args.synth))
    t = time.time()
    raw = pd.read_parquet(realdata("pbp_2023.parquet"))
    part = _normalize_participation(pd.read_parquet(realdata("part_2023.parquet")))
    ros = _normalize_roster(pd.read_parquet(realdata("roster_2023.parquet")))
    rp = dict(zip(ros["player_id"].astype(str), ros["position"], strict=True))
    pc = build_plays_contract(raw, participation=part, roster_positions=rp)
    pc = pc[pc.week <= 18]
    pc = attach_dv(pc, fit_value_model(pc))
    players = ros.drop_duplicates("player_id")[["player_id", "team", "position"]]
    ratings, team_df, beta, ci = run_rapm(pc, players)
    on = set(p for t in pc.off_players for p in t) | set(p for t in pc.def_players for p in t)
    r = ratings[ratings.player_id.isin(on)]
    print("real plays", len(pc), "dv sd %.3f" % pc.dv.std(), "players on field", len(r), "secs", round(time.time() - t, 1))
    print("real RAPM rating SD by position:", r.groupby("position").rating.std().round(4).to_dict())
    p2, pl2, gt = simulate(SynthConfig(yards_noise_sd=3.2, drives_per_team_per_game=12))
    p2 = attach_dv(p2, fit_value_model(p2))
    rs, *_ = run_rapm(p2, pl2)
    print("synth dv sd %.3f" % p2.dv.std(), "synth RAPM rating SD by pos:", rs.groupby("position").rating.std().round(4).to_dict())
    print("team_rating SD real %.4f" % team_df.team_rating.std())
    # per-player per-week snap counts on real data
    snaps = pd.Series([p for t in pc.off_players for p in t]).value_counts()
    print("real offensive snaps/season median (skill players w/ >100):", int(snaps[snaps > 100].median()))
    names = ros.drop_duplicates("player_id").set_index("player_id")
    qb = r[r.position == "QB"].copy()
    qb["snaps"] = qb.player_id.map(snaps).fillna(0)
    qb = qb[qb.snaps > 300].sort_values("rating", ascending=False)
    qb["name"] = qb.player_id.map(names["name"])
    print(qb[["name", "team", "rating", "snaps"]].head(8).to_string(index=False))
    print(qb[["name", "team", "rating", "snaps"]].tail(5).to_string(index=False))
    toff = {t: beta[ci["t_off"][t]] for t in ci["teams"]}
    qb["team_off_int"] = qb.team.map(toff)
    print("corr(QB rating, own team off intercept) among starters: %.3f" % np.corrcoef(qb.rating, qb.team_off_int)[0, 1])
    print("team off intercept SD %.4f" % np.std(list(toff.values())))


if __name__ == "__main__":
    main()
