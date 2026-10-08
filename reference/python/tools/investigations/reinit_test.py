"""weekly_update roster change: one new signing discards all accumulated RAPM weeks.

What it demonstrates
    ``weekly_update.run`` reinitialises the RAPM accumulators whenever the players frame
    changes size or order. Week 1 runs with 8 players; week 2 adds one player (a ninth row,
    a signing). The XtX entry of a player who was on the field 60 times in each week should
    read 120 after week 2 if evidence accumulated; it reads 60, because week 1 was dropped
    ("dimension mismatch (16 vs 17) -- reinitialising" in the oracle's log).
Evidence for
    KI-NEW-W2 (any roster change silently reinitialises the accumulators). Input to the Rust
    incremental-loop design (stable player index; grow XtX by zero-padding) and to DR-C10
    (proposed default, pending the owner).
Synthetic generator
    None: hand-built plays with numpy default_rng(0) dV noise.
Run (from reference/python, about 2 s)
    python3 -m tools.investigations.reinit_test
Expected output (recorded 2026-10-01 with requirements.lock pins)
    (the oracle's INFO log lines go to stdout first; the key one is the WARNING
    "Accumulator dimension mismatch (16 vs 17) -- reinitialising"), then:
    P0 diag after wk1: 60.0  after wk2 (expect 120 if accumulated): 60.0
Origin
    Consolidation inventory scratch ``scratch-cnissues/reinit_test.py`` (cn-issues report
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

        def players(n):
            return pd.DataFrame({"player_id": [f"P{i}" for i in range(n)], "team": (teams * 3)[:n],
                                 "position": (["QB", "RB", "WR", "TE", "DEF", "DEF", "DEF", "DEF", "WR"])[:n]})

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
            wu.run(week=1, season=2025, plays_df=plays(1), players_df=players(8), _conn=conn)
            xtx1 = np.load(tmp / "acc.npz")["XtX"]
            wu.run(week=2, season=2025, plays_df=plays(2), players_df=players(9), _conn=conn)  # one new signing
            xtx2 = np.load(tmp / "acc.npz")["XtX"]
        finally:
            conn.close()
        print("P0 diag after wk1:", xtx1[0, 0], " after wk2 (expect 120 if accumulated):", xtx2[0, 0])


if __name__ == "__main__":
    main()
