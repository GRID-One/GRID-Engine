"""Tests for the preseason projection assembly (Phase 2b; roadmap §4.5)."""
import numpy as np
import pandas as pd
import pytest

from backend.scoring.columns import SCORING_STAT_COLS
from backend.projection.model import fit_stat_line_model
from backend.projection.preseason import project_preseason
from backend.projection.volume import VolumeProjection


def _model():
    """A stat-line model fit on a tiny planted history (QB passing efficiency)."""
    n = 60
    rng = np.random.default_rng(0)
    rapm = rng.uniform(-1, 1, n)
    pass_att = rng.integers(20, 40, n).astype(float)
    hist = pd.DataFrame({
        "rapm_rating": rapm, "smoothed_talent": rapm, "prior_mean": np.zeros(n),
        "pass_attempts": pass_att,
        "passing_yards": pass_att * (7.0 + 1.0 * rapm),
        "completions": pass_att * 0.65, "passing_tds": pass_att * 0.05,
        "interceptions": pass_att * 0.02,
        **{c: np.zeros(n) for c in
           ("rush_attempts", "rushing_yards", "rushing_tds", "targets",
            "receptions", "receiving_yards", "receiving_tds",
            "fumbles_lost", "two_point_conversions")},
    })
    return fit_stat_line_model(hist, alpha=1e-6)


def test_project_preseason_scales_per_game_to_season():
    """Season stat line = per-game line * games, over all 14 SCORING_STAT_COLS."""
    model = _model()
    volume = VolumeProjection(per_game={"QB_A": {"pass_attempts": 30.0, "rush_attempts": 0.0,
                                                 "targets": 0.0, "receptions": 0.0}},
                              games_prior={"QB_A": 16})
    talent = pd.DataFrame([{"player_id": "QB_A", "rapm_rating": 0.5,
                            "smoothed_talent": 0.5, "prior_mean": 0.0}])
    season = project_preseason(volume, talent, {"QB": model}, {"QB_A": "QB"}, games=17)

    assert set(season["QB_A"]) == set(SCORING_STAT_COLS)
    # season pass_attempts = 30 per game * 17
    assert season["QB_A"]["pass_attempts"] == pytest.approx(30.0 * 17)
    # passing_yards scales with the season attempts and the fitted YPA
    assert season["QB_A"]["passing_yards"] > season["QB_A"]["pass_attempts"]


def test_players_without_a_model_are_omitted():
    """A player whose position has no fitted model is skipped, not errored."""
    model = _model()
    volume = VolumeProjection(
        per_game={"QB_A": {"pass_attempts": 30.0, "rush_attempts": 0.0, "targets": 0.0, "receptions": 0.0},
                  "K_A": {"pass_attempts": 0.0, "rush_attempts": 0.0, "targets": 0.0, "receptions": 0.0}},
        games_prior={"QB_A": 16, "K_A": 16})
    talent = pd.DataFrame([
        {"player_id": "QB_A", "rapm_rating": 0.0, "smoothed_talent": 0.0, "prior_mean": 0.0},
        {"player_id": "K_A", "rapm_rating": 0.0, "smoothed_talent": 0.0, "prior_mean": 0.0}])
    season = project_preseason(volume, talent, {"QB": model}, {"QB_A": "QB", "K_A": "K"})
    assert "QB_A" in season and "K_A" not in season   # no K model -> omitted


def test_player_without_volume_is_absent():
    """A player absent from the volume projection (e.g. a rookie with no prior
    usage) is not projected — the documented preseason rookie/role-changer gap."""
    model = _model()
    volume = VolumeProjection(per_game={}, games_prior={})   # nobody has volume
    talent = pd.DataFrame([{"player_id": "QB_A", "rapm_rating": 0.5,
                            "smoothed_talent": 0.5, "prior_mean": 0.0}])
    season = project_preseason(volume, talent, {"QB": model}, {"QB_A": "QB"})
    assert season == {}


def test_missing_talent_row_defaults_gracefully():
    """A player in volume but not in talent_frame still projects (talent defaults)."""
    model = _model()
    volume = VolumeProjection(per_game={"QB_A": {"pass_attempts": 25.0, "rush_attempts": 0.0,
                                                 "targets": 0.0, "receptions": 0.0}},
                              games_prior={"QB_A": 16})
    talent = pd.DataFrame(columns=["player_id", "rapm_rating", "smoothed_talent", "prior_mean"])
    season = project_preseason(volume, talent, {"QB": model}, {"QB_A": "QB"})
    assert "QB_A" in season
    assert np.isfinite(season["QB_A"]["passing_yards"])
