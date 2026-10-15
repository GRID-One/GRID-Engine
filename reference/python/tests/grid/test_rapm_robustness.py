"""Tests for RAPM robustness: ill-conditioned solve fallback and unknown player ID handling."""
import logging
import numpy as np
import pandas as pd
import pytest

from backend.grid.layers import _solve_ridge_prior, build_design


def test_solve_ridge_prior_lstsq_fallback(caplog):
    """Near-singular XtX should trigger lstsq fallback with a warning."""
    n = 5
    v = np.ones((n, 1))
    XtX = v @ v.T  # rank-1 -> singular without ridge
    Xty = np.ones(n)
    prior_mean = np.zeros(n)
    ridge_mask = np.ones(n)
    with caplog.at_level(logging.WARNING, logger="backend.grid.layers"):
        beta = _solve_ridge_prior(XtX, Xty, lam=1e-15, prior_mean=prior_mean, ridge_mask=ridge_mask)
    assert np.all(np.isfinite(beta))
    assert "ill-conditioned" in caplog.text


def test_solve_ridge_prior_normal_uses_solve(caplog):
    """Well-conditioned matrix should not trigger the warning."""
    n = 5
    XtX = np.eye(n) * 10
    Xty = np.ones(n)
    prior_mean = np.zeros(n)
    ridge_mask = np.ones(n)
    with caplog.at_level(logging.WARNING, logger="backend.grid.layers"):
        beta = _solve_ridge_prior(XtX, Xty, lam=1.0, prior_mean=prior_mean, ridge_mask=ridge_mask)
    assert np.all(np.isfinite(beta))
    assert "ill-conditioned" not in caplog.text


def _make_plays_and_players(include_unknown=False):
    """Build minimal plays/players DataFrames for build_design tests."""
    players = pd.DataFrame({
        "player_id": ["p1", "p2", "p3", "p4"],
        "team": ["A", "A", "B", "B"],
        "position": ["QB", "WR", "QB", "WR"],
        "is_starter": [True, True, True, True],
        "ability": [0.5, 0.3, 0.4, 0.2],
    })
    off_players = [["p1", "p2"], ["p1", "p2"], ["p3", "p4"], ["p3", "p4"]]
    def_players = [["p3", "p4"], ["p3", "p4"], ["p1", "p2"], ["p1", "p2"]]
    if include_unknown:
        off_players[0] = ["p1", "p2", "UNKNOWN_X"]
        def_players[1] = ["p3", "p4", "UNKNOWN_Y"]
    plays = pd.DataFrame({
        "off_players": off_players,
        "def_players": def_players,
        "off_team": ["A", "A", "B", "B"],
        "def_team": ["B", "B", "A", "A"],
        "dv": [0.1, -0.05, 0.2, -0.1],
        "down": [1, 2, 1, 3],
        "ydstogo": [10, 7, 10, 3],
        "yardline_100": [75, 68, 50, 47],
    })
    return plays, players


def test_build_design_unknown_player_skipped(caplog):
    """Plays referencing unknown player IDs should still produce a valid design matrix."""
    plays, players = _make_plays_and_players(include_unknown=True)
    with caplog.at_level(logging.WARNING, logger="backend.grid.layers"):
        X, y, colidx = build_design(plays, players)
    assert X.shape[0] == len(plays)
    assert "dropped" in caplog.text


def test_build_design_all_known_unchanged():
    """When all players are known, design matrix should have expected shape."""
    plays, players = _make_plays_and_players(include_unknown=False)
    X, y, colidx = build_design(plays, players)
    assert X.shape[0] == len(plays)
    n_expected_cols = len(players) + 2 * len(players.team.unique())
    assert X.shape[1] == n_expected_cols
    assert len(y) == len(plays)


def test_build_design_logs_dropped_players(caplog):
    """Verify the warning message includes the count of dropped entries."""
    plays, players = _make_plays_and_players(include_unknown=True)
    with caplog.at_level(logging.WARNING, logger="backend.grid.layers"):
        build_design(plays, players)
    assert "dropped 2 player entries" in caplog.text
