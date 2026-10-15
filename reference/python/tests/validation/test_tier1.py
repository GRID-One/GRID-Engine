"""Tests for the Tier-1 skill + calibration runner (Phase 2c; plan §7, §9).

Planted setup: current-season points = 2*talent + noise, GRID ratings = talent
(a perfect signal through the SV->points map), prior-season points
anti-correlated with talent — so the last-season baseline is deliberately
weak and GRID must beat it with a CI that excludes 0 (the H1 shape).
"""
import numpy as np
import pandas as pd
import pytest

from backend.projection.model import fit_stat_line_model
from backend.projection.sv_to_points import SVToPointsMap
from backend.projection.volume import VolumeProjection
from backend.scoring.columns import SCORING_STAT_COLS
from backend.scoring.formats import STANDARD
from backend.validation.backtest import OriginResult
from backend.validation import tier1 as T


N_PER_POS = 20
WEEKS = range(1, 11)
SEASON, PRIOR = 2024, 2023


def _players():
    """player_id -> (position, talent); talents 0..19 within each position."""
    out = {}
    for pos in ("QB", "RB"):
        for i in range(N_PER_POS):
            out[f"{pos}{i:02d}"] = (pos, float(i))
    return out


def _history(seed=0):
    """Two seasons of realized points: current = 2*talent + N(0,1); prior
    season ANTI-correlated (2*(19 - talent)) so last_season ranks players
    backwards — the deliberately weak baseline."""
    rng = np.random.default_rng(seed)
    rows = []
    for pid, (_pos, t) in _players().items():
        for wk in WEEKS:
            rows.append({"player_id": pid, "season": SEASON, "week": wk,
                         "points": 2.0 * t + rng.normal(0.0, 1.0)})
            rows.append({"player_id": pid, "season": PRIOR, "week": wk,
                         "points": 2.0 * (19.0 - t)})
    return pd.DataFrame(rows)


def _origins():
    """Walk-forward-shaped origins with ratings = planted talent (perfect)."""
    ratings = {pid: t for pid, (_pos, t) in _players().items()}
    return [OriginResult(season=SEASON, week=w, n_train_plays=100,
                         ratings=dict(ratings)) for w in WEEKS]


def _sv_map():
    """Identity-scaled map: forecast = 2 * rating = the true weekly mean."""
    return SVToPointsMap(coeffs={"QB": (0.0, 2.0), "RB": (0.0, 2.0)})


def _positions():
    return {pid: pos for pid, (pos, _t) in _players().items()}


# --------------------------------------------------------------------------- #
# forecast_weekly_points                                                       #
# --------------------------------------------------------------------------- #
def test_forecast_frame_well_formed():
    """One row per (origin, rated player with a mapped position); forecast =
    the per-position affine map applied to the rating."""
    fc = T.forecast_weekly_points(_origins(), _sv_map(), _positions())
    assert list(fc.columns) == ["season", "week", "player_id", "position", "forecast"]
    assert len(fc) == len(list(WEEKS)) * 2 * N_PER_POS
    row = fc[(fc.player_id == "QB07") & (fc.week == 3)].iloc[0]
    assert row.forecast == pytest.approx(2.0 * 7.0)


def test_forecast_omits_unmapped_positions():
    """A position without a fitted SV->points map yields no forecast rows
    (no calibrated conversion -> no projection, not a 0.0 guess)."""
    qb_only = SVToPointsMap(coeffs={"QB": (0.0, 2.0)})
    fc = T.forecast_weekly_points(_origins(), qb_only, _positions())
    assert set(fc.position) == {"QB"}
    unknown_pos = T.forecast_weekly_points(_origins(), _sv_map(), {})
    assert unknown_pos.empty


# --------------------------------------------------------------------------- #
# tier1_report — structure                                                     #
# --------------------------------------------------------------------------- #
def test_report_tables_well_formed():
    """Per-position keys plus ALL; each entry carries n_cells/mae/baselines
    with paired counts and a BootstrapCI margin."""
    fc = T.forecast_weekly_points(_origins(), _sv_map(), _positions())
    rep = T.tier1_report(fc, _history(), n=500)
    assert set(rep) == {"QB", "RB", "ALL"}
    for entry in rep.values():
        assert entry["n_cells"] > 0
        assert np.isfinite(entry["mae"])
        assert set(entry["baselines"]) == {"persistence", "season_to_date_mean",
                                           "last_season"}
        for b in entry["baselines"].values():
            assert b["n_pairs"] > 0
            assert hasattr(b["margin"], "excludes_zero")
    assert rep["ALL"]["n_cells"] == rep["QB"]["n_cells"] + rep["RB"]["n_cells"]


# --------------------------------------------------------------------------- #
# tier1_report — the H1 shape                                                  #
# --------------------------------------------------------------------------- #
def test_skill_positive_vs_weak_baseline_weekly():
    """GRID (planted-perfect) beats the anti-correlated last-season baseline:
    skill > 0 and the paired-margin CI excludes 0, per position and overall."""
    fc = T.forecast_weekly_points(_origins(), _sv_map(), _positions())
    rep = T.tier1_report(fc, _history(), horizon="weekly", n=500)
    for bucket in ("QB", "RB", "ALL"):
        ls = rep[bucket]["baselines"]["last_season"]
        assert ls["skill"] > 0
        assert ls["margin"].point > 0
        assert ls["margin"].excludes_zero is True


def test_skill_positive_vs_weak_baseline_ros():
    """The H1 target proper: rest-of-season per-game skill vs last_season."""
    fc = T.forecast_weekly_points(_origins(), _sv_map(), _positions())
    rep = T.tier1_report(fc, _history(), horizon="ros", n=500)
    ls = rep["ALL"]["baselines"]["last_season"]
    assert ls["skill"] > 0
    assert ls["margin"].excludes_zero is True
    # ROS realized = per-game mean of remaining weeks, so a true-mean forecast
    # has a smaller MAE than against single noisy weeks
    assert rep["ALL"]["mae"] < T.tier1_report(fc, _history(), n=500)["ALL"]["mae"]


def test_report_rejects_unknown_horizon():
    """Anything but 'weekly'/'ros' is an explicit error."""
    fc = T.forecast_weekly_points(_origins(), _sv_map(), _positions())
    with pytest.raises(ValueError):
        T.tier1_report(fc, _history(), horizon="season", n=100)


# --------------------------------------------------------------------------- #
# tier1_report — calibration                                                   #
# --------------------------------------------------------------------------- #
def test_calibration_honest_on_planted_noise():
    """Forecast = true mean, noise sd = 1: the expanding past-errors spread
    should make the Gaussian scores consistent (NIS ~ 1, PICP ~ level)."""
    fc = T.forecast_weekly_points(_origins(), _sv_map(), _positions())
    rep = T.tier1_report(fc, _history(), level=0.80, min_sd_history=20, n=500)
    cal = rep["ALL"]["calibration"]
    assert cal is not None and cal["n_cells"] > 100
    assert cal["nis"] == pytest.approx(1.0, abs=0.4)
    assert cal["picp"] == pytest.approx(0.80, abs=0.10)
    assert np.isfinite(cal["crps"]) and cal["crps"] > 0


def test_calibration_none_without_enough_history():
    """A min_sd_history no position can reach reports calibration: None
    instead of a spread trained on too little (or current-origin) data."""
    fc = T.forecast_weekly_points(_origins(), _sv_map(), _positions())
    rep = T.tier1_report(fc, _history(), min_sd_history=10_000, n=100)
    assert rep["ALL"]["calibration"] is None


# --------------------------------------------------------------------------- #
# forecast_ros_points — the volume x efficiency ROS projection                 #
# --------------------------------------------------------------------------- #
def _blank_line() -> dict:
    return {c: 0.0 for c in SCORING_STAT_COLS}


def _ros_history() -> pd.DataFrame:
    """One prior-season stat-line row per player: QBs driven by pass_attempts,
    RBs by rush_attempts, with efficiency rates rising in planted talent."""
    rows = []
    for pid, (pos, t) in _players().items():
        line = _blank_line()
        line.update(player_id=pid, season=PRIOR, position=pos,
                    rapm_rating=t, smoothed_talent=0.0, prior_mean=0.0)
        if pos == "QB":
            pa = 200.0 + 5.0 * t
            line.update(pass_attempts=pa, completions=pa * 0.6,
                        passing_yards=pa * (7.0 + 0.1 * t),
                        passing_tds=pa * 0.05, interceptions=pa * 0.02)
        else:  # RB
            ra = 100.0 + 5.0 * t
            line.update(rush_attempts=ra, rushing_yards=ra * (4.0 + 0.1 * t),
                        rushing_tds=ra * 0.03)
        rows.append(line)
    return pd.DataFrame(rows)


def _ros_models() -> dict:
    """Per-position fitted stat-line models (deterministic ridge, no seed)."""
    return {pos: fit_stat_line_model(g)
            for pos, g in _ros_history().groupby("position")}


def _volume(pid: str, pos: str, t: float) -> dict:
    line = _blank_line()
    if pos == "QB":
        line.update(pass_attempts=30.0 + 0.5 * t)
    else:
        line.update(rush_attempts=10.0 + 0.5 * t)
    return line


def _volume_by_origin(as_dict: bool = False) -> dict:
    """Per-game usage per player, same at every origin (frozen-usage stand-in)."""
    per_game = {pid: _volume(pid, pos, t)
                for pid, (pos, t) in _players().items()}
    out = {}
    for o in _origins():
        pg = {pid: dict(v) for pid, v in per_game.items()}
        out[(o.season, o.week)] = pg if as_dict else VolumeProjection(pg, {})
    return out


def test_ros_forecast_well_formed_and_uses_the_model():
    """One row per (origin, rated player with a model + projected volume); the
    forecast is exactly the stat-line model's scored projection — proving ROS
    is driven by volume x efficiency, not the weekly SV->points map."""
    fc = T.forecast_ros_points(_origins(), _ros_models(), _volume_by_origin(),
                               _positions(), STANDARD)
    assert list(fc.columns) == ["season", "week", "player_id", "position", "forecast"]
    assert len(fc) == len(list(WEEKS)) * 2 * N_PER_POS
    o = _origins()[0]
    per_game = _volume_by_origin()[(o.season, o.week)].per_game
    expected = _ros_models()["RB"].project_fantasy_points(
        per_game["RB07"], {"rapm_rating": 7.0}, STANDARD)
    row = fc[(fc.player_id == "RB07") & (fc.week == o.week)].iloc[0]
    assert row.forecast == pytest.approx(expected)
    # a real projection, not a degenerate 0.0
    assert expected > 0.0


def test_ros_forecast_omits_no_model_position_and_missing_volume():
    """No fitted model for a position, or no projected volume for a player this
    origin, means no forecast row (mirrors the weekly no-conversion contract)."""
    rb_only = {"RB": _ros_models()["RB"]}
    fc = T.forecast_ros_points(_origins(), rb_only, _volume_by_origin(),
                               _positions(), STANDARD)
    assert set(fc.position) == {"RB"}          # QB has no model -> omitted

    vbo = _volume_by_origin()
    for key in vbo:
        vbo[key].per_game.pop("RB05", None)    # no volume for RB05 anywhere
    fc2 = T.forecast_ros_points(_origins(), rb_only, vbo, _positions(), STANDARD)
    assert "RB05" not in set(fc2.player_id)

    # unknown position (empty positions map) -> nothing to forecast
    assert T.forecast_ros_points(_origins(), _ros_models(),
                                 _volume_by_origin(), {}, STANDARD).empty


def test_ros_forecast_accepts_projection_or_raw_dict():
    """volume_by_origin may hold a VolumeProjection or a raw {pid: {col: v}}
    dict — both resolve to the same forecast frame."""
    fc_proj = T.forecast_ros_points(_origins(), _ros_models(),
                                    _volume_by_origin(as_dict=False),
                                    _positions(), STANDARD)
    fc_dict = T.forecast_ros_points(_origins(), _ros_models(),
                                    _volume_by_origin(as_dict=True),
                                    _positions(), STANDARD)
    pd.testing.assert_frame_equal(fc_proj, fc_dict)


def test_ros_forecast_scales_with_volume():
    """Holding talent fixed, more projected volume -> more projected points:
    the defining property of a volume x efficiency projection."""
    model = _ros_models()["RB"]
    low = model.project_fantasy_points(
        {"rush_attempts": 5.0}, {"rapm_rating": 7.0}, STANDARD)
    high = model.project_fantasy_points(
        {"rush_attempts": 20.0}, {"rapm_rating": 7.0}, STANDARD)
    assert high > low > 0.0


def _smoothed_driven_rb_model():
    """A rate model whose rushing efficiency rises purely in smoothed_talent
    (rapm_rating held flat), so a forecast that ignores smoothed is detectable."""
    rows = []
    for i, st in enumerate((-1.0, 0.0, 1.0, 2.0)):
        line = _blank_line()
        line.update(player_id=f"RB{i}", season=PRIOR, position="RB",
                    rapm_rating=0.0, smoothed_talent=st, prior_mean=0.0,
                    rush_attempts=100.0, rushing_yards=100.0 * (4.0 + st),
                    rushing_tds=100.0 * 0.03)
        rows.append(line)
    return {"RB": fit_stat_line_model(pd.DataFrame(rows))}


def test_ros_forecast_consumes_smoothed_talent():
    """forecast_ros_points must feed `smoothed` into the model's talent row.
    Regression guard for the fit<->forecast feature-space contract: a forecast
    that dropped smoothed_talent (projecting at 0.0 while the model was trained
    on real smoothed talent) would ship biased ROS numbers with a green suite.
    With the feature driving the fitted rate, supplying `smoothed` must move the
    forecast — and in the right direction."""
    models = _smoothed_driven_rb_model()
    origins = [OriginResult(season=2023, week=5, n_train_plays=1,
                            ratings={"RBx": 0.0})]
    positions, vbo = {"RBx": "RB"}, {(2023, 5): {"RBx": {"rush_attempts": 10.0}}}

    base = T.forecast_ros_points(origins, models, vbo, positions, STANDARD)
    hi = T.forecast_ros_points(origins, models, vbo, positions, STANDARD,
                               smoothed={"RBx": 2.0})
    assert base.iloc[0].forecast != pytest.approx(hi.iloc[0].forecast)
    assert hi.iloc[0].forecast > base.iloc[0].forecast     # more talent -> more pts
    # a player absent from `smoothed` falls back to 0.0 (== the no-smoothed base)
    other = T.forecast_ros_points(origins, models, vbo, positions, STANDARD,
                                  smoothed={"someone_else": 9.0})
    assert other.iloc[0].forecast == pytest.approx(base.iloc[0].forecast)


def test_ros_forecast_empty_origins_is_empty():
    """No origins -> a well-formed empty frame, not an error."""
    fc = T.forecast_ros_points([], _ros_models(), _volume_by_origin(),
                               _positions(), STANDARD)
    assert list(fc.columns) == ["season", "week", "player_id", "position", "forecast"]
    assert fc.empty


def test_ros_forecast_skips_origin_without_volume():
    """An origin absent from volume_by_origin is skipped whole (vol is None) —
    no rows for it, not a 0.0-filled forecast."""
    vbo = _volume_by_origin()
    dropped = list(vbo)[0]                  # one origin has no volume entry
    del vbo[dropped]
    fc = T.forecast_ros_points(_origins(), _ros_models(), vbo,
                               _positions(), STANDARD)
    got = {(int(s), int(w))
           for s, w in fc[["season", "week"]].itertuples(index=False)}
    assert dropped not in got
    assert len(got) == len(list(WEEKS)) - 1


def test_ros_forecast_keeps_present_but_empty_model_as_zero():
    """A present-but-empty StatLineModel (no fitted rates — a position whose
    RATE_STATS volume is always 0, e.g. DB/P) is KEPT with a 0.0 forecast, not
    omitted: the deliberately position-agnostic contract, distinct from the
    "no model object -> omit" case."""
    zero_hist = pd.DataFrame([{**{c: 0.0 for c in SCORING_STAT_COLS},
                               "rapm_rating": 1.0, "smoothed_talent": 0.0,
                               "prior_mean": 0.0}])
    empty = fit_stat_line_model(zero_hist)
    assert empty.rates == {}                       # nothing fit (zero volume)
    origins = [OriginResult(season=SEASON, week=1, n_train_plays=10,
                            ratings={"DB1": 1.0})]
    vbo = {(SEASON, 1): {"DB1": {c: 0.0 for c in SCORING_STAT_COLS}}}
    fc = T.forecast_ros_points(origins, {"DB": empty}, vbo, {"DB1": "DB"}, STANDARD)
    assert list(fc.player_id) == ["DB1"]           # kept, not dropped
    assert fc.iloc[0].forecast == 0.0


def test_ros_forecast_games_scales_linearly():
    """games multiplies the per-game forecast linearly (reserved multi-week use)."""
    one = T.forecast_ros_points(_origins(), _ros_models(), _volume_by_origin(),
                                _positions(), STANDARD)
    three = T.forecast_ros_points(_origins(), _ros_models(), _volume_by_origin(),
                                  _positions(), STANDARD, games=3)
    m = one.merge(three, on=["season", "week", "player_id", "position"],
                  suffixes=("_1", "_3"))
    assert len(m) == len(one) > 0
    assert np.allclose(m["forecast_3"].to_numpy(), 3.0 * m["forecast_1"].to_numpy())
