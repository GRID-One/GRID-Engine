"""The oracle's real-data box-score ingest vs official nflverse player stats (2023).

What it demonstrates
    Runs the oracle's own ingest (``nflverse_loader._normalize_pbp`` then
    ``data_pipeline._aggregate_stats_from_pbp``) on the 2023 nflverse play-by-play, restricts
    to weeks <= 18, and compares the per-player regular-season sums with the official
    ``player_stats_2023`` (season_type REG), inner-joined on player_id. Also counts offensive
    plays credited with a return/defensive touchdown, sack rows that carry a passer, and
    postseason rows left in the normalized frame.
    Recorded result (cn-issues section 3.7): pass_attempts 19,658 vs 18,315 (+7.3%, sacks
    counted); passing_yards 119,092 vs 128,567 (-7.4%, sack yards netted); passing_tds 814
    vs 754 and receiving_tds 799 vs 754 (return TDs credited); rush attempts 14,178 vs
    14,588 (kneels excluded); fumbles_lost 174 vs 256 (-32%, misattributed); 66 offensive
    plays with a non-offensive TD; 1,459 sack rows with a passer; 1,638 postseason rows.
Evidence for
    KI-NEW-I1 (sacks in attempts/yards), KI-NEW-I2 (return TDs credited), KI-NEW-I3 (fumble
    attribution), KI-NEW-I4 (postseason rows summed), KI-NEW-I5 (two-point conversions never
    populated; kneels). Correction ledger entry 5 in reference/python/PARITY.md; DR-C12
    (proposed default, pending the owner: labels from official weekly stats, REG only).
    Real-data numbers are historical, non-parity evidence (README.md).
Data
    Real 2023 nflverse inputs (``pbp_2023``, ``player_stats_2023``), fetched and
    sha256-verified by ``fetch_realdata.py`` into the gitignored ``data/realdata/``. Not
    synthetic, so no ``--synth`` switch.
Run (from reference/python, after fetch_realdata; about 16 s)
    python3 -m tools.investigations.fetch_realdata
    python3 -m tools.investigations.cmp_stats
Expected output (recorded 2026-10-01 with requirements.lock pins and the pinned inputs)
    season_type counts: {'REG': 47399, 'POST': 2266} max week 22
    normalized weeks: [np.int32(18), np.int32(19), np.int32(20), np.int32(21), np.int32(22)]
    official season_type: {'REG': 5405, 'POST': 248}
    pass_attempts    sum_ours=   19658 sum_off=   18315  n_mismatch=  85 max|d|=65
    completions      sum_ours=   11808 sum_off=   11808  n_mismatch=   0 max|d|=0
    passing_yards    sum_ours=  119092 sum_off=  128567  n_mismatch=  84 max|d|=477
    passing_tds      sum_ours=     814 sum_off=     754  n_mismatch=  33 max|d|=5
    interceptions    sum_ours=     430 sum_off=     430  n_mismatch=   0 max|d|=0
    rush_attempts    sum_ours=   14178 sum_off=   14588  n_mismatch=  62 max|d|=21
    rushing_yards    sum_ours=   61761 sum_off=   61295  n_mismatch=  63 max|d|=25
    rushing_tds      sum_ours=     473 sum_off=     470  n_mismatch=   3 max|d|=1
    targets          sum_ours=   17483 sum_off=   17483  n_mismatch=   0 max|d|=0
    receptions       sum_ours=   11808 sum_off=   11808  n_mismatch=   0 max|d|=0
    receiving_yards  sum_ours=  128564 sum_off=  128562  n_mismatch=  12 max|d|=14
    receiving_tds    sum_ours=     799 sum_off=     754  n_mismatch=  40 max|d|=3
    fumbles_lost ours 174 official 256.0 n_mismatch 47
    plays touchdown=1 but not pass/rush TD (return/defensive TDs on offensive plays): 66
    sacks in pass_attempt==1 rows w/ passer: 1459
    postseason rows in normalized: 1638
                pass_attempts  attempts  ...  passing_tds_o  passing_tds_f
    player_id                            ...
    00-0037077            677       612  ...             26             21
    00-0033106            631       605  ...             33             30
    00-0033077            629       590  ...             37             36
    00-0033873            622       597  ...             29             27
    00-0036264            607       579  ...             32             32

    [5 rows x 6 columns]
Origin
    Consolidation inventory scratch ``scratch-cnissues/cmp_stats.py`` (cn-issues report
    section 3.7). Logic unchanged; the input paths now come from ``data/realdata/`` and the
    path bootstrap and this header were added.
"""
from __future__ import annotations

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2]))
from tools.investigations._common import bootstrap, realdata  # noqa: E402

bootstrap()

import pandas as pd  # noqa: E402

from backend.grid.nflverse_loader import _normalize_pbp  # noqa: E402
from backend.pipeline.data_pipeline import _aggregate_stats_from_pbp  # noqa: E402


def main() -> None:
    raw = pd.read_parquet(realdata("pbp_2023.parquet"))
    print("season_type counts:", raw.season_type.value_counts().to_dict(), "max week", raw.week.max())
    norm = _normalize_pbp(raw)
    print("normalized weeks:", sorted(norm.week.unique())[-5:])
    ours = _aggregate_stats_from_pbp(norm, 2023)
    off = pd.read_parquet(realdata("player_stats_2023.parquet"))
    print("official season_type:", off.season_type.value_counts().to_dict())
    # compare regular season totals
    o = ours[ours.week <= 18].groupby("player_id").sum(numeric_only=True)
    f = off[off.season_type == "REG"].groupby("player_id").sum(numeric_only=True)
    pairs = {"pass_attempts": "attempts", "completions": "completions", "passing_yards": "passing_yards",
             "passing_tds": "passing_tds", "interceptions": "interceptions", "rush_attempts": "carries",
             "rushing_yards": "rushing_yards", "rushing_tds": "rushing_tds", "targets": "targets",
             "receptions": "receptions", "receiving_yards": "receiving_yards", "receiving_tds": "receiving_tds"}
    j = o.join(f, how="inner", lsuffix="_o", rsuffix="_f")
    for a, b in pairs.items():
        ca = a if a in j.columns else a + "_o"
        cb = b if b in j.columns else b + "_f"
        if a == b:
            ca, cb = a + "_o", b + "_f"
        d = j[ca] - j[cb]
        print(f"{a:16s} sum_ours={j[ca].sum():8.0f} sum_off={j[cb].sum():8.0f}  "
              f"n_mismatch={int((d != 0).sum()):4d} max|d|={int(d.abs().max())}")
    # fumbles lost
    fl = f["sack_fumbles_lost"] + f["rushing_fumbles_lost"] + f["receiving_fumbles_lost"]
    d = j["fumbles_lost"] - fl.reindex(j.index).fillna(0)
    print("fumbles_lost ours", j.fumbles_lost.sum(), "official", fl.reindex(j.index).fillna(0).sum(),
          "n_mismatch", int((d != 0).sum()))
    # pick-six / return TD check
    r = raw[(raw.play_type.isin(["pass", "run"])) & raw.down.notna()]
    print("plays touchdown=1 but not pass/rush TD (return/defensive TDs on offensive plays):",
          int(((r.touchdown == 1) & (r.pass_touchdown == 0) & (r.rush_touchdown == 0)).sum()))
    print("sacks in pass_attempt==1 rows w/ passer:",
          int(((r.pass_attempt == 1) & (r.sack == 1) & r.passer_player_id.notna()).sum()))
    print("postseason rows in normalized:", int((norm.week > 18).sum()))
    # QB example
    qb = j.sort_values("pass_attempts", ascending=False).head(5)[
        ["pass_attempts", "attempts", "passing_yards_o", "passing_yards_f", "passing_tds_o", "passing_tds_f"]]
    print(qb)


if __name__ == "__main__":
    main()
