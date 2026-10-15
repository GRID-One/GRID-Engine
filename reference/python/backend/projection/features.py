"""GRID talent-feature accessor (roadmap Phase 2b §4.5).

Assembles the **efficiency** side of the projection — volume is `volume.py`'s
job, this is GRID's. Three features per player, each already estimated
elsewhere in the engine; this module does pure assembly (union the player IDs,
look up each input, fill sane defaults), **no new estimation**:

* **Prior-season RAPM rating** — `layers.run_rapm`'s ``ratings_df``, read as-is.
* **End-of-season RTS-smoothed talent** ("best retrospective talent", the
  preseason cold-start signal per §4.5) — genuinely runs the RTS smoother
  (`statespace.kalman_two_component`) over each player's full-season weekly
  observation series.  This is **not** the same number as the DB's
  `kalman_trajectory.talent` column, which is the *filtered* (online, causal)
  state from `kalman_step` — conflating filtered and smoothed state is exactly
  the kind of correct-looking-wrong-semantic bug the engine's golden master
  exists to catch (see `validation plan §6`), so this module never aliases one
  for the other.  Real-data wiring of the weekly observation arrays (currently
  caller-supplied, e.g. from synth or a future weekly Layer-1 series) lands
  with the preseason-path PR.
* **Cross-league prior** (rookies only) — `priors.build_priors()`'s
  ``prior_mean``/``prior_var``.  ``None`` until the real-data wiring PR lands
  (`priors.estimate_equivalency` is synth-only today); this module already
  accepts it so that PR is additive, not a rewrite here.
"""
from __future__ import annotations

from dataclasses import dataclass

import numpy as np
import pandas as pd

from backend.grid.statespace import SSParams, kalman_two_component


@dataclass
class TalentFeatures:
    """Per-player talent features, each ``{player_id: value}``.

    ``smoothed_var`` is the RTS-smoothed variance at season end (a confidence
    signal for the projection model); ``prior_mean``/``prior_var`` are empty
    dicts until a cross-league prior is supplied.
    """

    rapm_rating: dict
    smoothed_talent: dict
    smoothed_var: dict
    prior_mean: dict
    prior_var: dict

    def to_frame(self, player_ids=None) -> pd.DataFrame:
        """Tidy per-player feature frame, one row per player.

        Missing ``rapm_rating``/``smoothed_talent`` default to 0.0 (RAPM's own
        ridge-toward-zero prior — "no evidence" already means "average").
        Missing ``prior_mean``/``prior_var`` are ``NaN`` ("no cross-league prior
        for this player", typically a non-rookie) rather than 0.0, so a
        downstream model can distinguish "no prior" from "prior of zero".
        """
        ids = (player_ids if player_ids is not None else
               sorted(set(self.rapm_rating) | set(self.smoothed_talent)
                      | set(self.prior_mean), key=str))
        return pd.DataFrame([{
            "player_id": pid,
            "rapm_rating": self.rapm_rating.get(pid, 0.0),
            "smoothed_talent": self.smoothed_talent.get(pid, 0.0),
            "smoothed_var": self.smoothed_var.get(pid, float("nan")),
            "prior_mean": self.prior_mean.get(pid, float("nan")),
            "prior_var": self.prior_var.get(pid, float("nan")),
        } for pid in ids])


def assemble_rapm_features(
    ratings_df: pd.DataFrame, player_col: str = "player_id", rating_col: str = "rating",
) -> dict:
    """``{player_id: rating}`` from a `layers.run_rapm` ``ratings_df``."""
    return dict(zip(ratings_df[player_col], ratings_df[rating_col].astype(float)))


def assemble_smoothed_talent(weekly_obs: dict, positions: dict | None = None) -> tuple[dict, dict]:
    """Run the RTS smoother per player over a full season of weekly observations.

    ``weekly_obs`` is ``{player_id: {"y": array, "snaps": array, "played": array,
    "interventions": set?}}`` — the same ``(y, snaps, played)`` shape
    `kalman_two_component` already takes, one entry per player. ``positions``
    (``{player_id: position}``) selects position-specific `SSParams`; a player
    absent from it gets the generic default.

    Missing-week input contract: a week the player did not appear must be
    ``played=False`` **and** ``y=NaN`` (not 0.0).  `kalman_two_component` seeds
    its talent init from ``nanmean(y[:3])``, reading ``y`` *without* consulting
    ``played``, so a structural 0.0 in the first weeks would bias the smoothed
    estimate toward zero; ``NaN`` is the value that init (and the ``played``-
    guarded update) skips.

    Returns ``({player_id: tau_smooth at the final week}, {player_id:
    var_tau_smooth at the final week})`` — the end-of-season "best retrospective
    talent" estimate and its uncertainty.
    """
    positions = positions or {}
    talent: dict = {}
    var: dict = {}
    for pid, obs in weekly_obs.items():
        params = SSParams.from_position(positions[pid]) if pid in positions else None
        out = kalman_two_component(
            np.asarray(obs["y"], dtype=float),
            np.asarray(obs["snaps"], dtype=float),
            np.asarray(obs["played"], dtype=bool),
            interventions=obs.get("interventions"),
            params=params,
        )
        talent[pid] = float(out["tau_smooth"][-1])
        var[pid] = float(out["var_tau_smooth"][-1])
    return talent, var


def build_priors_lookup(priors_df: pd.DataFrame | None, player_col: str = "player_id") -> tuple[dict, dict]:
    """``({player_id: prior_mean}, {player_id: prior_var})`` from `priors.build_priors()`.

    ``None`` or empty (the synth-only-today case, before the real-data cross-
    league wiring PR lands) returns two empty dicts.
    """
    if priors_df is None or len(priors_df) == 0:
        return {}, {}
    mean = dict(zip(priors_df[player_col], priors_df["prior_mean"].astype(float)))
    var = dict(zip(priors_df[player_col], priors_df["prior_var"].astype(float)))
    return mean, var


def assemble_talent_features(
    ratings_df: pd.DataFrame,
    weekly_obs: dict,
    *,
    positions: dict | None = None,
    priors_df: pd.DataFrame | None = None,
) -> TalentFeatures:
    """Assemble the GRID talent features the stat-line model consumes.

    Pure assembly over already-estimated inputs (`run_rapm`'s ``ratings_df``,
    per-player weekly observations for the RTS smoother, and an optional
    cross-league ``priors_df`) — no new estimation happens here.
    """
    rapm_rating = assemble_rapm_features(ratings_df)
    smoothed_talent, smoothed_var = assemble_smoothed_talent(weekly_obs, positions)
    prior_mean, prior_var = build_priors_lookup(priors_df)
    return TalentFeatures(rapm_rating, smoothed_talent, smoothed_var, prior_mean, prior_var)
