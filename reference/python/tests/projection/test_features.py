"""Tests for the GRID talent-feature accessor (Phase 2b; roadmap §4.5).

Pure-assembly checks: each piece is verified against a direct call to the
underlying engine function it wraps, so these tests would catch a filtered-vs-
smoothed mixup or a silent default change.
"""
import numpy as np
import pandas as pd
import pytest

from backend.grid.statespace import SSParams, kalman_two_component
from backend.projection.features import (
    TalentFeatures,
    assemble_rapm_features,
    assemble_smoothed_talent,
    assemble_talent_features,
    build_priors_lookup,
)


def _obs(y, snaps=None, played=None):
    n = len(y)
    return {
        "y": np.array(y, dtype=float),
        "snaps": np.array(snaps if snaps is not None else [10.0] * n),
        "played": np.array(played if played is not None else [True] * n),
    }


# --------------------------------------------------------------------------- #
# assemble_rapm_features                                                      #
# --------------------------------------------------------------------------- #
def test_assemble_rapm_features_from_ratings_df():
    """A run_rapm-shaped ratings_df becomes a {player_id: rating} dict."""
    df = pd.DataFrame({"player_id": ["A", "B"], "rating": [1.5, -0.5]})
    assert assemble_rapm_features(df) == {"A": 1.5, "B": -0.5}


# --------------------------------------------------------------------------- #
# assemble_smoothed_talent                                                     #
# --------------------------------------------------------------------------- #
def test_smoothed_talent_matches_direct_kalman_call():
    """The wrapper's tau_smooth/var_tau_smooth match calling kalman_two_component
    directly with the same inputs — proves it's pure pass-through, no re-derivation."""
    y = [0.5, 0.6, 0.4, 0.55, 0.45]
    obs = _obs(y)
    talent, var = assemble_smoothed_talent({"A": obs})
    direct = kalman_two_component(obs["y"], obs["snaps"], obs["played"])
    assert talent["A"] == pytest.approx(float(direct["tau_smooth"][-1]))
    assert var["A"] == pytest.approx(float(direct["var_tau_smooth"][-1]))


def test_smoothed_talent_uses_position_params():
    """A position hint selects SSParams.from_position, changing the smoothed
    result relative to the no-position default (different d_steady/r_scale)."""
    y = [0.5, 0.6, 0.4, 0.55, 0.45]
    obs = _obs(y)
    talent_default, _ = assemble_smoothed_talent({"A": obs})
    talent_qb, _ = assemble_smoothed_talent({"A": obs}, positions={"A": "QB"})
    direct_qb = kalman_two_component(obs["y"], obs["snaps"], obs["played"],
                                     params=SSParams.from_position("QB"))
    assert talent_qb["A"] == pytest.approx(float(direct_qb["tau_smooth"][-1]))
    assert talent_qb["A"] != pytest.approx(talent_default["A"])


def test_smoothed_talent_handles_missing_weeks():
    """A played=False week doesn't crash; the result stays finite."""
    obs = _obs([0.5, 0.6, np.nan, 0.55], played=[True, True, False, True])
    talent, var = assemble_smoothed_talent({"A": obs})
    assert np.isfinite(talent["A"]) and np.isfinite(var["A"])


def test_smoothed_talent_multiple_players_independent():
    """Each player's smoother runs over only their own series."""
    talent, _ = assemble_smoothed_talent({
        "A": _obs([1.0, 1.0, 1.0, 1.0]),
        "B": _obs([-1.0, -1.0, -1.0, -1.0]),
    })
    assert talent["A"] > 0 and talent["B"] < 0


# --------------------------------------------------------------------------- #
# build_priors_lookup                                                          #
# --------------------------------------------------------------------------- #
def test_build_priors_lookup_none_returns_empty():
    """No priors_df (today's real-data default) -> two empty dicts."""
    assert build_priors_lookup(None) == ({}, {})


def test_build_priors_lookup_empty_frame_returns_empty():
    """An empty (but schema-correct) priors_df also returns empty dicts."""
    empty = pd.DataFrame(columns=["player_id", "prior_mean", "prior_var"])
    assert build_priors_lookup(empty) == ({}, {})


def test_build_priors_lookup_extracts_mean_and_var():
    """A populated priors_df (e.g. priors.build_priors() output) extracts both columns."""
    df = pd.DataFrame({"player_id": ["R1"], "prior_mean": [0.3], "prior_var": [0.2]})
    mean, var = build_priors_lookup(df)
    assert mean == {"R1": 0.3} and var == {"R1": 0.2}


# --------------------------------------------------------------------------- #
# assemble_talent_features + to_frame                                          #
# --------------------------------------------------------------------------- #
def test_assemble_talent_features_combines_all_three():
    """End-to-end: ratings_df + weekly_obs + priors_df -> a populated TalentFeatures."""
    ratings_df = pd.DataFrame({"player_id": ["A"], "rating": [0.8]})
    weekly_obs = {"A": _obs([0.5, 0.6, 0.4, 0.55])}
    priors_df = pd.DataFrame({"player_id": ["R1"], "prior_mean": [0.1], "prior_var": [0.5]})
    feats = assemble_talent_features(ratings_df, weekly_obs, priors_df=priors_df)
    assert feats.rapm_rating == {"A": 0.8}
    assert "A" in feats.smoothed_talent and "A" in feats.smoothed_var
    assert feats.prior_mean == {"R1": 0.1} and feats.prior_var == {"R1": 0.5}


def test_to_frame_fills_defaults_for_partial_coverage():
    """A rookie only in priors_df gets 0.0 rapm/smoothed but a real prior; a
    veteran only in ratings_df/weekly_obs gets NaN prior (no cross-league signal)."""
    ratings_df = pd.DataFrame({"player_id": ["VET"], "rating": [1.2]})
    weekly_obs = {"VET": _obs([0.5, 0.6, 0.4])}
    priors_df = pd.DataFrame({"player_id": ["ROOK"], "prior_mean": [0.2], "prior_var": [0.6]})
    feats = assemble_talent_features(ratings_df, weekly_obs, priors_df=priors_df)
    frame = feats.to_frame().set_index("player_id")

    assert frame.loc["ROOK", "rapm_rating"] == 0.0
    assert frame.loc["ROOK", "smoothed_talent"] == 0.0
    assert frame.loc["ROOK", "prior_mean"] == 0.2

    assert frame.loc["VET", "rapm_rating"] == 1.2
    assert np.isnan(frame.loc["VET", "prior_mean"])
    assert np.isnan(frame.loc["VET", "prior_var"])


def test_to_frame_player_ids_union_is_sorted():
    """Default player_ids is the sorted union across all three inputs."""
    feats = TalentFeatures(
        rapm_rating={"B": 1.0}, smoothed_talent={"C": 0.5}, smoothed_var={},
        prior_mean={"A": 0.1}, prior_var={"A": 0.2})
    frame = feats.to_frame()
    assert list(frame["player_id"]) == ["A", "B", "C"]


def test_to_frame_explicit_player_ids_overrides_union():
    """Passing player_ids restricts/orders the output rows explicitly."""
    feats = TalentFeatures(
        rapm_rating={"A": 1.0, "B": 2.0}, smoothed_talent={}, smoothed_var={},
        prior_mean={}, prior_var={})
    frame = feats.to_frame(player_ids=["B"])
    assert list(frame["player_id"]) == ["B"]
    assert frame.iloc[0]["rapm_rating"] == 2.0
