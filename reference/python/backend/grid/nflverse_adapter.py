"""nflverse -> GRID `plays`-contract adapter (roadmap Phase 1; design §4.4, §10).

`nflverse_loader._normalize_pbp` emits a *stats-aggregation* shape (yards, td,
attempts ...).  The GRID engine consumes a different, richer contract
(`data_adapters.py`): per-play drive segmentation, next-state s', a next-score
``drive_points`` label, terminal flags, and on-field participation — logic that
until now existed ONLY in `synth.py`.  This module is the real-data counterpart:
it turns raw nflverse play-by-play into the same contract `simulate()` produces,
so `fit_value_model`, `compute_dv`, and (with participation) `build_design`/
`run_rapm` run unchanged on real football.

Scope of this module today: the state / drive / next-score transformation —
the synth-only long pole.  Participation (`off_players`/`def_players`) is wired
as an OPTIONAL join here (plumbing + filtering), populated empty when absent; the
real nflverse participation *loader* (the FTN feed) lands next and fills it for
RAPM.

Semantics are matched to `synth.simulate()` so downstream behaviour is identical:
  * drive_points: the drive's eventual points, broadcast to EVERY play in the
    drive — Touchdown -> 7, Field goal -> 3, everything else -> 0 (the V(s)
    target).  (Safety / defensive TD are scored 0 for the offense in this
    scaffold, matching synth's 7/3/0 vocabulary; refine later if needed.)
  * terminal: the LAST modelled (pass/run) play of each drive.
  * terminal_value: drive_points on the terminal play, NaN elsewhere (compute_dv
    only reads it for terminal rows).
  * points: drive_points on the terminal play, 0.0 elsewhere.
  * n_down/n_ydstogo/n_yardline_100: the next modelled play's state within the
    same drive; (-1, -1, -1) on terminal plays.
"""
from __future__ import annotations

import numpy as np
import pandas as pd

# fixed_drive_result -> points the offense's drive yielded (V(s) target vocab).
_DRIVE_POINTS = {"Touchdown": 7.0, "Field goal": 3.0}

# Offensive skill positions GRID treats as estimands (OL is a team-level
# nuisance absorbed by the offense intercept, never an estimand — see layers.py).
_OFFENSE_SKILL = {"QB", "RB", "WR", "TE", "FB"}


def load_grid_plays(years: "list[int]", cache_dir: str = "data/cache") -> pd.DataFrame:
    """Assemble the real-data GRID `plays` contract end-to-end for ``years``.

    Fetches raw nflverse pbp + per-play participation + rosters (all cached), then
    runs :func:`build_plays_contract` with a roster position map so off_players is
    filtered to offensive skill positions.  The result is ready for
    ``fit_value_model`` / ``attach_dv`` / ``run_rapm`` — i.e. RAPM on real NFL
    data.  (Participation is the free FTN feed: real for completed seasons,
    published once after the postseason.)
    """
    from backend.grid.nflverse_loader import (  # noqa: PLC0415
        load_raw_pbp, load_participation, load_rosters,
    )
    raw_pbp = load_raw_pbp(years, cache_dir)
    participation = load_participation(years, cache_dir)
    roster = load_rosters(years, cache_dir)
    roster_positions = dict(zip(roster["player_id"].astype(str), roster["position"], strict=True))
    return build_plays_contract(raw_pbp, participation=participation,
                                roster_positions=roster_positions)


def build_plays_contract(
    raw_pbp: pd.DataFrame,
    participation: pd.DataFrame | None = None,
    roster_positions: dict | None = None,
) -> pd.DataFrame:
    """Transform raw nflverse play-by-play into the GRID ``plays`` contract.

    Parameters
    ----------
    raw_pbp:
        Raw nflverse PBP with at least: game_id, play_id, week, posteam, defteam,
        down, ydstogo, yardline_100, yards_gained, play_type, fixed_drive,
        fixed_drive_result.
    participation:
        Optional per-play participation with [game_id, play_id, offense_players,
        defense_players] where the player columns are ``;``-separated id strings
        (the nflverse/FTN shape).  When omitted, off_players/def_players are
        empty tuples (the value/dV path needs no participation; RAPM does).
    roster_positions:
        Optional {player_id: position} map used to keep only offensive skill
        players in off_players (defense is kept as listed).  When omitted, all
        listed players are kept.

    Returns
    -------
    DataFrame with exactly the GRID plays-contract columns.
    """
    df = raw_pbp[raw_pbp["play_type"].isin(["pass", "run"])].copy()
    df = df[df["down"].notna()].copy()
    if df.empty:
        return _empty_contract()

    # Stable within-drive order: chronological by play_id inside each drive.
    df["_drive_no"] = df["fixed_drive"].astype("int64")
    df = df.sort_values(["game_id", "_drive_no", "play_id"]).reset_index(drop=True)
    drive_id = df["game_id"].astype(str) + "_" + df["_drive_no"].astype(str)

    down = df["down"].astype(int).to_numpy()
    ydstogo = df["ydstogo"].astype(int).to_numpy()
    yardline = df["yardline_100"].astype(float).to_numpy()

    # drive_points: map the drive result, broadcast to every play in the drive.
    drive_points = df["fixed_drive_result"].map(_DRIVE_POINTS).fillna(0.0).to_numpy(float)

    # next-state within each drive (shift up by one); last play of a drive is
    # terminal and has no next state.
    grp = df.groupby(drive_id, sort=False)
    n_down = grp["down"].shift(-1).to_numpy()
    n_ydstogo = grp["ydstogo"].shift(-1).to_numpy()
    n_yardline = grp["yardline_100"].shift(-1).to_numpy()
    terminal = np.isnan(n_down)

    n_down = np.where(terminal, -1, n_down).astype(int)
    n_ydstogo = np.where(terminal, -1, n_ydstogo).astype(int)
    n_yardline = np.where(terminal, -1.0, n_yardline)

    terminal_value = np.where(terminal, drive_points, np.nan)
    points = np.where(terminal, drive_points, 0.0)

    off_players, def_players = _participation_tuples(
        df, participation, roster_positions
    )

    out = pd.DataFrame({
        "play_id": df["play_id"].to_numpy(),
        "drive_id": drive_id.to_numpy(),
        "week": df["week"].astype(int).to_numpy(),
        "off_team": df["posteam"].to_numpy(),
        "def_team": df["defteam"].to_numpy(),
        "down": down,
        "ydstogo": ydstogo,
        "yardline_100": yardline,
        "yards": df["yards_gained"].astype(float).to_numpy(),
        "points": points,
        "terminal": terminal,
        "terminal_value": terminal_value,
        "n_down": n_down,
        "n_ydstogo": n_ydstogo,
        "n_yardline_100": n_yardline,
        "drive_points": drive_points,
        "off_players": off_players,
        "def_players": def_players,
    })
    return out


def _empty_contract() -> pd.DataFrame:
    """Return a zero-row DataFrame with the full GRID plays-contract columns."""
    cols =["play_id", "drive_id", "week", "off_team", "def_team", "down",
            "ydstogo", "yardline_100", "yards", "points", "terminal",
            "terminal_value", "n_down", "n_ydstogo", "n_yardline_100",
            "drive_points", "off_players", "def_players"]
    return pd.DataFrame({c: [] for c in cols})


def _split_ids(cell) -> list[str]:
    """Split a ``;``-separated participation cell into a list of player ids.

    Returns [] for missing values — including pandas nullable scalars (``pd.NA``),
    ``None`` and ``NaN`` — so off_players/def_players never contain a bogus
    ``"<NA>"`` placeholder id from real participation data.
    """
    if cell is None:
        return []
    if isinstance(cell, (list, tuple, np.ndarray)):
        return [str(x) for x in cell]
    if pd.isna(cell):
        return []
    return [p for p in str(cell).split(";") if p]


def _participation_tuples(df, participation, roster_positions):
    """Return (off_players, def_players) lists aligned to df rows.

    Empty tuples everywhere when participation is absent.  When present, join on
    (game_id, play_id), split the id strings, and keep only offensive skill
    players in off_players (defense kept as listed) — RAPM models skill players
    as estimands and absorbs OL into the offense intercept.
    """
    n = len(df)
    if participation is None:
        empty = [()] * n
        return empty, list(empty)

    # Normalize join keys on BOTH sides so the lookup can't silently miss: pbp
    # play_id is often float (276.0) while participation play_id is int (276), and
    # game_id must be a plain str. Mismatched dtypes would drop all participation.
    part = participation.copy()
    part["game_id"] = part["game_id"].astype(str)
    part["play_id"] = part["play_id"].astype("int64")
    part = part.set_index(["game_id", "play_id"])
    keys = list(zip(df["game_id"].astype(str),
                    df["play_id"].astype("int64"), strict=True))

    def _keep_off(ids):
        """Keep only offensive skill-position ids (drop OL etc.) when a roster
        position map is available; otherwise keep all listed offense ids."""
        if roster_positions is None:
            return tuple(ids)
        return tuple(p for p in ids if roster_positions.get(p) in _OFFENSE_SKILL)

    off, deff = [], []
    for k in keys:
        if k in part.index:
            row = part.loc[k]
            off.append(_keep_off(_split_ids(row.get("offense_players"))))
            deff.append(tuple(_split_ids(row.get("defense_players"))))
        else:
            off.append(())
            deff.append(())
    return off, deff
