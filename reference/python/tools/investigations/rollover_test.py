"""weekly_update season rollover: week 1 of a new season is skipped.

What it demonstrates
    ``weekly_update.run`` keeps the RAPM accumulators keyed by week only and skips any week
    ``<=`` the last processed week. After 2024 week 18 is solved, 2025 week 1 returns
    ``skipped: True`` and is never processed until the accumulator cache is deleted.
    Runs the real ``weekly_update.run`` on a tiny hand-built league (4 teams, 8 players, 60
    plays a week), with the accumulator, Kalman state and SQLite DB redirected to a temporary
    directory and valuations / health writes stubbed out.
Evidence for
    KI-NEW-W1 (season rollover skips forever; no season key). Input to DR-C10 (proposed default,
    pending the owner: state keyed by (season, week)) and to the Rust incremental-loop design.
Synthetic generator
    None: hand-built plays with numpy default_rng(0) dV noise.
Run (from reference/python, about 2 s)
    python3 -m tools.investigations.rollover_test
Expected output (recorded 2026-10-01 with requirements.lock pins)
    (the oracle's INFO log lines go to stdout first; the key one is
    "Accumulators already current through week=18 -- skipping"), then:
    2024 wk18: {'week': 18, 'season': 2024, 'n_players': 8, 'n_plays': 60, 'rapm_solved': True, 'kalman_updated': True}
    2025 wk1: {'week': 1, 'season': 2025, 'rapm_solved': False, 'kalman_updated': False, 'skipped': True}
Origin
    Consolidation inventory scratch ``scratch-cnissues/rollover_test.py`` (cn-issues report
    section 3.2). Logic unchanged; the temporary directory and DB connection are now cleaned
    up, and the path bootstrap and this header were added.
"""
from __future__ import annotations

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2]))
from tools.investigations._common import bootstrap  # noqa: E402

bootstrap()

import sqlite3  # noqa: E402
import tempfile  # noqa: E402

import numpy as np  # noqa: E402
import pandas as pd  # noqa: E402

import backend.grid.layers as layers_mod  # noqa: E402
import backend.grid.statespace as ss_mod  # noqa: E402
import backend.pipeline.weekly_update as wu  # noqa: E402
from backend.db.connection import init_db  # noqa: E402


def main() -> None:
    with tempfile.TemporaryDirectory() as d:
        tmp = pathlib.Path(d)
        layers_mod._ACCUM_PATH = tmp / "acc.npz"
        ss_mod._KALMAN_PATH = tmp / "kal.npz"
        wu.run_valuations = lambda **kw: None
        wu.write_health = lambda **kw: None
        rng = np.random.default_rng(0)
        teams = ["KC", "BUF", "PHI", "DAL"]
        players = pd.DataFrame({"player_id": [f"P{i}" for i in range(8)], "team": teams * 2,
                                "position": ["QB", "RB", "WR", "TE", "DEF", "DEF", "DEF", "DEF"]})

        def plays(week):
            n = 60
            return pd.DataFrame({"off_team": [teams[i % 2] for i in range(n)],
                                 "def_team": [teams[2 + i % 2] for i in range(n)],
                                 "off_players": [("P0", "P1", "P2") for _ in range(n)],
                                 "def_players": [("P4", "P5") for _ in range(n)],
                                 "dv": rng.normal(size=n), "week": [week] * n})

        conn = sqlite3.connect(str(tmp / "db.sqlite"))
        conn.row_factory = sqlite3.Row
        try:
            init_db(conn)
            r1 = wu.run(week=18, season=2024, plays_df=plays(18), players_df=players, _conn=conn)
            r2 = wu.run(week=1, season=2025, plays_df=plays(1), players_df=players, _conn=conn)
        finally:
            conn.close()
        print("2024 wk18:", r1)
        print("2025 wk1:", r2)


if __name__ == "__main__":
    main()
