"""The real 2023 plays contract built by the oracle's nflverse adapter.

What it demonstrates
    Builds the GRID plays contract from real 2023 inputs with the oracle's own
    ``nflverse_loader._normalize_participation`` / ``_normalize_roster`` and
    ``nflverse_adapter.build_plays_contract``, then reports: roster duplicates and null
    teams, play count and **postseason rows (weeks > 18) left in the contract**, on-field
    offensive/defensive participant counts, participants missing from the roster, the
    ``fixed_drive_result`` vocabulary (e.g. "Opp touchdown" and "Safety", which the adapter
    maps to 0 drive points), and plays with no QB among ``off_players``.
Evidence for
    KI-NEW-V0a (no season_type filter: 1,638 postseason plays enter RAPM, V(s) and the
    verdict), KI-NEW-V0b (drive_points vocabulary: 502 "Opp touchdown" and 62 "Safety" plays
    map to 0), and the participation shape behind DR-C1 (proposed default, pending the
    owner). Real-data numbers are historical, non-parity evidence (README.md).
Data
    Real 2023 nflverse inputs (``pbp_2023``, ``part_2023``, ``roster_2023``), fetched and
    sha256-verified by ``fetch_realdata.py`` into the gitignored ``data/realdata/``.
Run (from reference/python, after fetch_realdata; about 13 s)
    python3 -m tools.investigations.real_contract
Expected output (recorded 2026-10-01 with requirements.lock pins and the pinned inputs)
    roster dup player_ids: 0 null team: 0
    plays 35474 weeks>18: 1638 secs 10.3   (secs = wall time; varies)
    off_players size dist: {1: 2, 2: 7, 3: 22, 4: 55, 5: 913, 6: 34475}
    def_players size dist: {9: 1, 10: 21, 11: 35451, 13: 1}
    participants not in roster: 0
    drive_result vocab: {'Punt': 14862, 'Touchdown': 13188, 'Field goal': 10392, 'Turnover': 3690, 'Turnover on downs': 3263, 'End of half': 2135, 'Missed field goal': 1569, 'Opp touchdown': 502, 'Safety': 62}
    plays w/o any QB in off_players: 25
Origin
    Consolidation inventory scratch ``scratch-cnissues/real_contract.py`` (cn-issues report
    section 3.6). Logic unchanged; the input paths now come from ``data/realdata/`` and the
    path bootstrap and this header were added.
"""
from __future__ import annotations

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2]))
from tools.investigations._common import bootstrap, realdata  # noqa: E402

bootstrap()

import time  # noqa: E402

import pandas as pd  # noqa: E402

from backend.grid.nflverse_adapter import build_plays_contract  # noqa: E402
from backend.grid.nflverse_loader import _normalize_participation, _normalize_roster  # noqa: E402


def main() -> None:
    t = time.time()
    raw = pd.read_parquet(realdata("pbp_2023.parquet"))
    part = _normalize_participation(pd.read_parquet(realdata("part_2023.parquet")))
    ros = _normalize_roster(pd.read_parquet(realdata("roster_2023.parquet")))
    print("roster dup player_ids:", int(ros.player_id.duplicated().sum()), "null team:", int(ros.team.isna().sum()))
    rp = dict(zip(ros["player_id"].astype(str), ros["position"], strict=True))
    pc = build_plays_contract(raw, participation=part, roster_positions=rp)
    print("plays", len(pc), "weeks>18:", int((pc.week > 18).sum()), "secs", round(time.time() - t, 1))
    off_n = pc.off_players.apply(len)
    def_n = pc.def_players.apply(len)
    print("off_players size dist:", off_n.value_counts().sort_index().to_dict())
    print("def_players size dist:", def_n.value_counts().sort_index().head(15).to_dict())
    allp = set(p for t in pc.off_players for p in t) | set(p for t in pc.def_players for p in t)
    print("participants not in roster:", len(allp - set(ros.player_id.astype(str))))
    print("drive_result vocab:", raw.fixed_drive_result.value_counts().to_dict())
    # off_players filtered to skill: how many plays have no QB?
    qbs = set(ros[ros.position == "QB"].player_id)
    print("plays w/o any QB in off_players:", int(pc.off_players.apply(lambda t: not any(p in qbs for p in t)).sum()))


if __name__ == "__main__":
    main()
