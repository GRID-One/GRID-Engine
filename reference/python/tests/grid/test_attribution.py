"""Tests for extended attribution: position-specific lambda and multi-QB Layer 1."""
from unittest.mock import patch

import numpy as np
import pandas as pd
import pytest

from backend.grid.layers import layer1_all_qbs, layer1_all_players, layer1_qb_weekly, run_rapm


def _make_synth_data(n_plays=200, seed=5):
    """Minimal synthetic plays + players for attribution tests."""
    from backend.grid.synth import simulate, SynthConfig
    from backend.grid.value import fit_value_model, attach_dv

    cfg = SynthConfig()
    plays, players, gt = simulate(cfg)
    vm = fit_value_model(plays)
    plays = attach_dv(plays, vm)
    return plays, players, gt


def test_run_rapm_tolerates_players_without_synth_columns():
    """run_rapm must work on REAL rosters, which (unlike synth players) carry no
    is_starter/ability columns — its output assembly used to hard-require them."""
    plays, players, _ = _make_synth_data()
    real_like = players[["player_id", "team", "position"]].copy()  # drop is_starter/ability
    ratings, _, _, _ = run_rapm(plays, real_like)
    assert "rating" in ratings.columns
    assert len(ratings) == len(real_like)
    assert not ratings.rating.isna().any()
    assert "ability" not in ratings.columns   # gracefully absent, not crashing


# ─── position-specific lambda tests ──────────────────────────────────────────

def test_lambda_by_pos_accepted():
    """run_rapm with lambda_by_pos should not raise and return valid ratings."""
    plays, players, gt = _make_synth_data()
    lbp = {"QB": 0.5, "RB": 1.5, "WR": 1.0, "TE": 1.0}
    ratings, team_df, beta, colidx = run_rapm(plays, players, lambda_by_pos=lbp)
    assert "rating" in ratings.columns
    assert len(ratings) == len(players)
    assert not ratings.rating.isna().any()


def test_lambda_by_pos_affects_ratings():
    """Different lambda_by_pos configs should yield different per-position ratings."""
    plays, players, gt = _make_synth_data()
    r1, *_ = run_rapm(plays, players)
    r2, *_ = run_rapm(plays, players, lambda_by_pos={"QB": 0.1, "RB": 10.0})

    # QB ratings should differ most between configs
    qb1 = r1[r1.position == "QB"].rating.values
    qb2 = r2[r2.position == "QB"].rating.values
    rb1 = r1[r1.position == "RB"].rating.values
    rb2 = r2[r2.position == "RB"].rating.values

    qb_change = np.abs(qb1 - qb2).mean()
    rb_change = np.abs(rb1 - rb2).mean()
    # High QB-lambda change vs high RB-lambda change — one should differ more
    assert qb_change != rb_change or qb_change > 0


def test_lambda_by_pos_partial_ok():
    """Providing lambda for only some positions leaves others at default 1.0."""
    plays, players, gt = _make_synth_data()
    r1, *_ = run_rapm(plays, players)
    r2, *_ = run_rapm(plays, players, lambda_by_pos={"QB": 2.0})
    # Only QB ratings should shift meaningfully
    qb1 = r1[r1.position == "QB"].rating.values
    qb2 = r2[r2.position == "QB"].rating.values
    assert not np.allclose(qb1, qb2, atol=1e-6), "QB ratings should change with lambda"


# ─── multi-QB Layer 1 tests ───────────────────────────────────────────────────

def test_layer1_all_qbs_returns_dict():
    plays, players, gt = _make_synth_data()
    # Use at least two QB IDs from the synth roster
    qb_ids = players[players.position == "QB"].player_id.tolist()
    if len(qb_ids) < 2:
        pytest.skip("Synth data has fewer than 2 QBs")

    from backend.grid.layers import run_rapm, layer1_qb_weekly
    ratings, *_ = run_rapm(plays, players)
    rlook = ratings.set_index("player_id").rating.to_dict()

    result = layer1_all_qbs(plays, rlook, qb_ids, n_splits=3)
    assert isinstance(result, dict)
    assert len(result) >= 1  # at least one QB has enough snaps


def test_layer1_all_qbs_skips_sparse_qbs():
    """QBs with fewer than 20 plays should be omitted from results."""
    plays, players, gt = _make_synth_data()
    qb_ids = players[players.position == "QB"].player_id.tolist()
    if not qb_ids:
        pytest.skip("No QBs in synth data")

    from backend.grid.layers import run_rapm
    ratings, *_ = run_rapm(plays, players)
    rlook = ratings.set_index("player_id").rating.to_dict()

    # Add a fake QB who never appears
    fake_qb = "FAKE_QB_999"
    result = layer1_all_qbs(plays, rlook, qb_ids + [fake_qb], n_splits=3)
    assert fake_qb not in result


# ─── any-position Layer 1 (Phase 2b: layer1_all_players) ─────────────────────

def test_layer1_all_players_matches_layer1_qb_weekly_for_a_qb():
    """layer1_all_players is a behavior-preserving generalization: for a QB it
    reproduces layer1_qb_weekly's exact weekly credit/snaps (same seed/splits),
    just under the renamed 'credit' column."""
    plays, players, gt = _make_synth_data()
    qb_ids = players[players.position == "QB"].player_id.tolist()
    if not qb_ids:
        pytest.skip("No QBs in synth data")
    focus_qb = qb_ids[0]

    ratings, *_ = run_rapm(plays, players)
    rlook = ratings.set_index("player_id").rating.to_dict()

    direct = layer1_qb_weekly(plays, rlook, focus_qb, n_splits=3, seed=1)
    generalized = layer1_all_players(plays, rlook, [focus_qb], n_splits=3, seed=1)

    assert focus_qb in generalized
    g = generalized[focus_qb].rename(columns={"credit": "qb_credit"})
    pd.testing.assert_frame_equal(g, direct)


def test_layer1_all_players_generalizes_to_rb_wr():
    """Non-QB positions get finite weekly credit too (the Phase 2b gap this closes)."""
    plays, players, gt = _make_synth_data()
    non_qb_ids = players[players.position.isin(["RB", "WR"])].player_id.tolist()
    if not non_qb_ids:
        pytest.skip("No RB/WR in synth data")

    ratings, *_ = run_rapm(plays, players)
    rlook = ratings.set_index("player_id").rating.to_dict()

    result = layer1_all_players(plays, rlook, non_qb_ids, n_splits=3)
    assert len(result) >= 1
    for wk in result.values():
        assert {"week", "credit", "snaps"} <= set(wk.columns)
        assert np.isfinite(wk["credit"]).all()
        assert (wk["snaps"] > 0).all()


def test_layer1_all_players_skips_sparse_players():
    """A player with fewer than min_plays on-field snaps is omitted."""
    plays, players, gt = _make_synth_data()
    any_ids = players.player_id.tolist()[:3]
    ratings, *_ = run_rapm(plays, players)
    rlook = ratings.set_index("player_id").rating.to_dict()

    fake_player = "FAKE_PLAYER_999"
    result = layer1_all_players(plays, rlook, any_ids + [fake_player], n_splits=3)
    assert fake_player not in result


def test_layer1_all_players_fits_the_context_model_exactly_once():
    """The expensive GBM cross-fit runs n_splits times TOTAL, independent of how
    many players are requested -- the whole point of the generalization (calling
    layer1_qb_weekly per player would instead cost n_splits * len(player_ids))."""
    plays, players, gt = _make_synth_data()
    player_ids = players.player_id.tolist()[:8]
    ratings, *_ = run_rapm(plays, players)
    rlook = ratings.set_index("player_id").rating.to_dict()

    with patch("backend.grid.layers.HistGradientBoostingRegressor") as mock_gbm:
        mock_gbm.return_value.fit.return_value = None
        mock_gbm.return_value.predict.return_value = np.zeros(1)
        # predict must return an array matching the fold's held-out size; use a
        # side_effect keyed off input shape instead of a fixed-size stub.
        mock_gbm.return_value.predict.side_effect = lambda X: np.zeros(len(X))
        layer1_all_players(plays, rlook, player_ids, n_splits=3)

    assert mock_gbm.call_count == 3   # == n_splits, NOT n_splits * len(player_ids)
