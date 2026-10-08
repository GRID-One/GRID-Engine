"""Preseason draft-time projection assembly (roadmap Phase 2b §4.5).

The alpha's primary use is draft prep, where there is no current-season data.
This assembles the **preseason stat-line projection** that `compute_valuations`
sources the `valuations` table from, combining the Phase-2b pieces built in the
prior PRs into one per-player season stat line:

    volume (volume.project_volume)  x  efficiency (model.StatLineModel fed the
    GRID talent features: prior-season RAPM + Kalman end-of-prior-season smoothed
    talent + rookie cross-league prior)  ->  a per-game stat line, scaled to a
    season by an expected games count, then scored per format downstream.

Pure assembly over already-built inputs — no estimation here. A player needs
both a projected volume and a fitted per-position model to be projected; players
without prior-season volume (e.g. rookies with no usage history) are absent
unless given a volume override (the documented §4.5 role-changer/rookie
weakness — the ADP/depth-chart override hook is their path in).

The real-data fit of the underlying models + the H1 verdict are Phase 2c; this
module wires the assembly.
"""
from __future__ import annotations

import pandas as pd

from backend.projection.model import StatLineModel
from backend.projection.volume import VolumeProjection
from backend.scoring.columns import SCORING_STAT_COLS

DEFAULT_GAMES = 17   # a full modern NFL regular season


def project_preseason(
    volume: VolumeProjection,
    talent_frame: pd.DataFrame,
    models: dict,
    positions: dict,
    *,
    games: int = DEFAULT_GAMES,
    player_col: str = "player_id",
) -> dict:
    """Assemble ``{player_id: season stat line}`` over the 14 `SCORING_STAT_COLS`.

    ``volume`` is `volume.project_volume`'s per-game usage; ``talent_frame`` is
    `features.TalentFeatures.to_frame()` (one row per player); ``models`` maps
    position -> fitted `StatLineModel`; ``positions`` maps player_id -> position.
    Each player's per-game stat line (`StatLineModel.project_stat_line`) is scaled
    by ``games`` to a season total (VOR only needs a consistent scale across
    players).  Players lacking a projected volume or a model for their position
    are omitted.
    """
    talent_rows = talent_frame.set_index(player_col).to_dict("index")
    out: dict = {}
    for pid, vol_per_game in volume.per_game.items():
        model = models.get(positions.get(pid))
        if model is None:
            continue
        per_game_line = model.project_stat_line(vol_per_game, talent_rows.get(pid, {}))
        out[pid] = {c: per_game_line[c] * games for c in SCORING_STAT_COLS}
    return out
