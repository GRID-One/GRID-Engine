"""End-to-end synth test of the Phase-2c verdict runner (plan §12 6-8).

One shared run on the synthetic engine contract: current-season points are
driven by the planted abilities the walk-forward RAPM recovers, while the
prior season is anti-correlated — so the last-season baseline is wrong and the
H1 verdict must come out SHIP_GRID.  The real-data run is the documented
dev/nightly step, not CI.
"""
import json

import numpy as np
import pandas as pd
import pytest
from threadpoolctl import threadpool_limits

from backend.grid import load_synthetic
from backend.scoring.columns import SCORING_STAT_COLS
from backend.validation import verdict as V
from backend.validation.backtest import OriginResult, walk_forward
from backend.validation.report import write_report

SEASON, PRIOR = 2023, 2022


@pytest.fixture(scope="module")
def synth_bundle():
    """Plays/players + planted realized-points history + points-allowed feed."""
    b = load_synthetic()
    plays, players = b["plays"].copy(), b["players"]
    plays["season"] = SEASON
    skill = players[players["position"] != "DEF"]

    rng = np.random.default_rng(0)
    rows = []
    for r in skill.itertuples(index=False):
        for wk in sorted(plays["week"].unique()):
            rows.append({"player_id": r.player_id, "season": SEASON,
                         "week": int(wk),
                         "points": 8.0 + 30.0 * r.ability + rng.normal(0, 0.5)})
            rows.append({"player_id": r.player_id, "season": PRIOR,
                         "week": int(wk),
                         "points": 8.0 - 30.0 * r.ability})   # anti-correlated
    history = pd.DataFrame(rows)

    # Points allowed built noiselessly from the SAME def grades the verdict's
    # own walk-forward produces (deterministic): tougher grade -> fewer points.
    with threadpool_limits(limits=1):
        origins = walk_forward(plays, players, warmup_weeks=4)
    allowed = pd.DataFrame(
        [{"season": o.season, "week": o.week, "def_team": t,
          "points_allowed": 20.0 - 30.0 * g}
         for o in origins for t, g in o.def_grades.items()])
    return plays, players, history, allowed


@pytest.fixture(scope="module")
def verdict_run(synth_bundle):
    """One shared full run (walk-forward is the expensive part)."""
    plays, players, history, allowed = synth_bundle
    registry = {"kpis": {}}
    with threadpool_limits(limits=1):
        results = V.run_verdict(plays, players, history, allowed=allowed,
                                registry=registry, run_label="synth-1", n=500)
    return results, registry


def test_h1_verdict_is_ship_grid(verdict_run):
    """GRID (recovers planted ability) beats the anti-correlated last-season
    baseline on ROS skill -> the kill criterion passes."""
    results, _ = verdict_run
    assert results["verdict"] == "SHIP_GRID"
    ls = results["tier1_ros"]["ALL"]["baselines"]["last_season"]
    assert ls["margin"].point > 0 and ls["margin"].excludes_zero is True
    assert len(ls["margins"]) == ls["n_pairs"]        # keep_margins wired


def test_h2_margin_positive(verdict_run):
    """GRID-set lineups outscore last-season-set lineups on realized points."""
    results, _ = verdict_run
    h2 = results["h2"]
    assert h2["overall"].point > 0
    assert h2["win_rate"] > 0.5
    assert h2["starter_out"] is None                  # no availability data fed


def test_tier2_predictive_on_deterministic_allowed(verdict_run):
    """points_allowed built monotonically from the grades -> predictive ~ 1."""
    results, _ = verdict_run
    t2 = results["tier2"]
    assert t2 is not None and t2["n_weeks"] == results["n_origins"]
    assert t2["predictive"].point > 0.9
    assert t2["predictive"].excludes_zero is True


def test_gates_promoted_and_checked(verdict_run):
    """All three headline KPIs get a frozen noise-floor gate on the first run,
    and the (strong, planted) H1 value clears its own null floor."""
    results, registry = verdict_run
    assert set(results["gates"]) == {"h1_ros_margin_vs_last_season",
                                     "h2_weekly_lineup_margin",
                                     "tier2_matchup_predictive"}
    for g in results["gates"].values():
        assert g["gate"] is not None
    assert results["gates"]["h1_ros_margin_vs_last_season"]["passed"] is True
    assert all(e["run_label"] == "synth-1" for e in registry["kpis"].values())


def test_registry_frozen_across_runs(synth_bundle, verdict_run):
    """A second run against the same registry re-uses the frozen gates
    (no re-promotion) and skips Tier 2 visibly when no allowed feed exists."""
    plays, players, history, _ = synth_bundle
    _, registry = verdict_run
    before = json.loads(json.dumps(registry))         # deep snapshot
    with threadpool_limits(limits=1):
        results2 = V.run_verdict(plays, players, history, allowed=None,
                                 registry=registry, run_label="synth-2",
                                 origin_stride=5, seed=7, n=200)
    assert results2["tier2"] is None
    assert "tier2_matchup_predictive" not in results2["gates"]
    # frozen: same gates, original run_label — synth-2 promoted nothing anew
    assert registry["kpis"]["h1_ros_margin_vs_last_season"] == \
        before["kpis"]["h1_ros_margin_vs_last_season"]
    assert registry["kpis"]["h2_weekly_lineup_margin"] == \
        before["kpis"]["h2_weekly_lineup_margin"]


def test_load_players_frame_is_full_roster_universe(monkeypatch):
    """The RAPM players frame is the FULL roster (all positions, deduped to the
    latest season) — regression for the real run dropping every participant
    because the universe was the skill-position DB subset."""
    roster = pd.DataFrame([
        {"player_id": "00-01", "name": "A", "team": "KC", "position": "QB", "season": 2023},
        {"player_id": "00-01", "name": "A", "team": "SF", "position": "QB", "season": 2024},
        {"player_id": "00-02", "name": "B", "team": "SF", "position": "CB", "season": 2024},
        {"player_id": "00-03", "name": "C", "team": None, "position": "DT", "season": 2024},
    ])
    import backend.grid.nflverse_loader as NL
    monkeypatch.setattr(NL, "load_rosters", lambda years: roster)
    frame = V._load_players_frame([2023, 2024])
    by_id = dict(zip(frame["player_id"], frame["team"], strict=True))
    assert by_id == {"00-01": "SF", "00-02": "SF"}   # deduped to latest; NaN-team dropped
    assert "CB" in set(frame["position"])            # defenders stay in the universe


def test_load_history_scores_weekly_stat_lines(monkeypatch):
    """_load_history scores each weekly stat line with STANDARD (keys are the
    stat columns only — player_id/season/week never reach the scorer)."""
    import sqlite3
    from pathlib import Path

    conn = sqlite3.connect(":memory:")
    schema = Path("backend/db/schema.sql").read_text()
    conn.executescript(schema)
    conn.execute("INSERT INTO players (player_id, name, position) VALUES ('00-01','A','QB')")
    conn.execute(
        "INSERT INTO player_stats (player_id, season, week, passing_yards, passing_tds) "
        "VALUES ('00-01', 2024, 3, 250, 2)")
    conn.commit()
    import backend.db.connection as DBC
    monkeypatch.setattr(DBC, "get_db", lambda: conn)

    history = V._load_history()
    assert list(history.columns) == ["player_id", "season", "week", "points"]
    row = history.iloc[0]
    assert (row.player_id, row.season, row.week) == ("00-01", 2024, 3)
    # STANDARD: 250 pass yds * 0.04 + 2 pass TD * 4 = 18.0
    assert row.points == pytest.approx(18.0)


def test_load_snapshot_tags_seasons_and_returns_years(tmp_path):
    """Per-season parquets concatenate with a season column; years come back
    for the roster load."""
    manifest = {"years": [2023, 2024]}
    (tmp_path / "manifest.json").write_text(json.dumps(manifest))
    for year in manifest["years"]:
        pd.DataFrame({"week": [1, 2]}).to_parquet(tmp_path / f"grid_plays_{year}.parquet")
    plays, years = V._load_snapshot(str(tmp_path))
    assert years == [2023, 2024]
    assert sorted(plays["season"].unique()) == [2023, 2024]
    assert len(plays) == 4


def test_report_artifacts_from_real_results(verdict_run, tmp_path):
    """The full results dict round-trips through the artifact writer."""
    results, _ = verdict_run
    json_path, md_path = write_report(results, out_dir=tmp_path)
    data = json.loads(json_path.read_text())
    assert data["verdict"] == "SHIP_GRID"
    md = md_path.read_text()
    assert "SHIP_GRID" in md and "Tier 2" in md and "Gates" in md


# --------------------------------------------------------------------------- #
# ROS stat-line model wiring: leakage + run_verdict integration                #
# --------------------------------------------------------------------------- #
def _blank(**over):
    line = {c: 0.0 for c in SCORING_STAT_COLS}
    line.update(over)
    return line


def _rb_stats(poison=False):
    """Two RBs, clean weeks 1-4 of 2022+2023 (all <= a week-5 origin's W-1
    cutoff); ``poison`` adds a 2023 wk-17 row with absurd efficiency that must
    be excluded from a fit frozen as of that origin."""
    rows = []
    for pid, ypc in [("RB1", 4.0), ("RB2", 5.0)]:
        for season in (2022, 2023):
            for wk in (1, 2, 3, 4):
                rows.append(_blank(player_id=pid, season=season, week=wk,
                                   rush_attempts=10.0,
                                   rushing_yards=10.0 * ypc, rushing_tds=0.5))
    if poison:
        rows.append(_blank(player_id="RB1", season=2023, week=17,
                           rush_attempts=10.0, rushing_yards=1.0e7))
    return pd.DataFrame(rows)


def test_fit_ros_models_freezes_on_pre_first_origin():
    """A poisoned stat row past the first origin's W-1 cutoff must not reach the
    fitted rate models — the ROS fit is frozen on pre-first-origin data (the
    leakage-critical path that run_verdict exercises with a real stats frame)."""
    first = OriginResult(season=2023, week=5, n_train_plays=100,
                         ratings={"RB1": 1.0, "RB2": 2.0})
    positions = {"RB1": "RB", "RB2": "RB"}
    vol, talent = {"rush_attempts": 10.0}, {"rapm_rating": 1.0}
    clean = V._fit_ros_models(first, _rb_stats(False), positions)
    poisoned = V._fit_ros_models(first, _rb_stats(True), positions)
    yc = clean["RB"].project_stat_line(vol, talent)["rushing_yards"]
    yp = poisoned["RB"].project_stat_line(vol, talent)["rushing_yards"]
    assert yc > 0.0                    # a real fitted rate, not degenerate
    assert yc == pytest.approx(yp)     # the wk-17 poison had no effect


def _synth_stats(players, weeks):
    """player_stats-shaped volume/efficiency lines for the synth skill players,
    driven per position so every one gets a non-empty rate model + volume."""
    rows = []
    for r in players[players["position"] != "DEF"].itertuples(index=False):
        pos = r.position
        for season in (PRIOR, SEASON):
            for wk in weeks:
                line = _blank(player_id=r.player_id, season=season, week=int(wk))
                if pos == "QB":
                    line.update(pass_attempts=30.0,
                                passing_yards=30.0 * (7.0 + r.ability),
                                passing_tds=30.0 * 0.05)
                elif pos == "RB":
                    line.update(rush_attempts=12.0,
                                rushing_yards=12.0 * (4.0 + r.ability),
                                rushing_tds=12.0 * 0.03)
                else:
                    line.update(receptions=5.0,
                                receiving_yards=5.0 * (10.0 + r.ability),
                                receiving_tds=5.0 * 0.05)
                rows.append(line)
    return pd.DataFrame(rows)


def test_run_verdict_scores_ros_with_stat_line_model(synth_bundle):
    """stats_history routes ROS through the volume x efficiency stat-line model
    (not the weekly SV->points map): the full loader->model->volume->forecast
    wiring produces a populated ROS table and a verdict."""
    plays, players, history, _ = synth_bundle
    weeks = sorted(plays["week"].unique())
    stats = _synth_stats(players, weeks)
    registry = {"kpis": {}}
    with threadpool_limits(limits=1):
        results = V.run_verdict(plays, players, history, stats_history=stats,
                                registry=registry, run_label="synth-ros",
                                origin_stride=5, n=200)
    ros = results["tier1_ros"]["ALL"]
    assert ros["n_cells"] > 0
    ls = ros["baselines"]["last_season"]
    assert ls["n_pairs"] > 0 and np.isfinite(ls["margin"].point)
    assert {"QB", "RB", "WR", "TE"} & set(results["tier1_ros"])   # skill rows scored


def test_run_verdict_wires_smoothed_to_both_fit_and_forecast(synth_bundle, monkeypatch):
    """run_verdict must pass the SAME computed smoothed-talent dict to BOTH the
    ROS fit and the ROS forecast — the feature-space contract at the wiring
    level. Dropping smoothed= from either call silently reintroduces train/serve
    skew (model trained on real talent, projected at 0.0, or vice-versa), which
    the finer-grained tests don't catch one layer up."""
    plays, players, history, _ = synth_bundle
    stats = _synth_stats(players, sorted(plays["week"].unique()))

    captured: dict = {}
    real_fit, real_fc = V._fit_ros_models, V.forecast_ros_points

    def spy_fit(*a, **k):
        captured["fit"] = k.get("smoothed")
        return real_fit(*a, **k)

    def spy_fc(*a, **k):
        captured["forecast"] = k.get("smoothed")
        return real_fc(*a, **k)

    monkeypatch.setattr(V, "_fit_ros_models", spy_fit)
    monkeypatch.setattr(V, "forecast_ros_points", spy_fc)
    with threadpool_limits(limits=1):
        V.run_verdict(plays, players, history, stats_history=stats,
                      registry={"kpis": {}}, run_label="wire",
                      origin_stride=5, n=100)
    assert captured["fit"] and len(captured["fit"]) > 0     # feature computed + wired to fit
    assert captured["forecast"] is captured["fit"]          # the SAME dict to both sinks


def test_run_verdict_dedupes_players_before_volume_merge(synth_bundle):
    """A duplicate player_id in the players frame must not fan out the
    stats_history merge and inflate projected volume (the drop_duplicates
    guard): ROS scoring is identical with and without the duplicate row."""
    plays, players, history, _ = synth_bundle
    stats = _synth_stats(players, sorted(plays["week"].unique()))
    dup_players = pd.concat([players, players.iloc[[0]]], ignore_index=True)
    with threadpool_limits(limits=1):
        base = V.run_verdict(plays, players, history, stats_history=stats,
                             registry={"kpis": {}}, run_label="dedup-base",
                             origin_stride=5, n=100)
        dup = V.run_verdict(plays, dup_players, history, stats_history=stats,
                            registry={"kpis": {}}, run_label="dedup-dup",
                            origin_stride=5, n=100)
    b, d = base["tier1_ros"]["ALL"], dup["tier1_ros"]["ALL"]
    assert d["n_cells"] == b["n_cells"]
    assert d["mae"] == pytest.approx(b["mae"])


def test_fit_ros_models_excludes_players_absent_from_ratings():
    """_fit_ros_models fits only on players present in BOTH ratings and
    positions; a stats row for an unrated player is dropped from the fit."""
    first = OriginResult(season=2023, week=5, n_train_plays=100,
                         ratings={"RB1": 1.0})            # RB2 not rated
    positions = {"RB1": "RB", "RB2": "RB"}
    models = V._fit_ros_models(first, _rb_stats(False), positions)
    assert set(models) == {"RB"}
    yhat = models["RB"].project_stat_line(
        {"rush_attempts": 10.0}, {"rapm_rating": 1.0})["rushing_yards"]
    assert yhat == pytest.approx(40.0)               # RB1 rate 4.0 * 10 attempts


def test_smoothed_talent_is_frozen_pre_first_origin_and_live(synth_bundle):
    """The smoothed-talent feature is a real, non-constant signal AND frozen on
    the pre-first-origin window: appending every later week to the input plays
    leaves it bit-identical (it reads only weeks strictly before the origin), so
    it is leakage-safe by construction."""
    plays, players, _, _ = synth_bundle
    positions = dict(zip(players["player_id"], players["position"], strict=True))
    with threadpool_limits(limits=1):
        first = walk_forward(plays, players, warmup_weeks=4)[0]
        full = V._smoothed_talent_pre_first_origin(plays, first, positions)
        # truncating the input to weeks before the origin must not change it
        pre_only = plays[plays["week"] < first.week]
        trunc = V._smoothed_talent_pre_first_origin(pre_only, first, positions)
    assert len(full) > 0
    assert np.std(list(full.values())) > 0                 # a live signal, not all-0
    assert set(full) == set(trunc)
    assert max(abs(full[k] - trunc[k]) for k in full) == 0.0   # frozen == leakage-safe


def test_window_timeline_is_chronological_across_seasons():
    """The window timeline is sorted (season, week) — a load-bearing sort the RTS
    smoother walks in order; equal week numbers in different seasons must not
    collide or reorder. Built from an intentionally out-of-order frame."""
    pre = pd.DataFrame({
        "season": [2023, 2022, 2023, 2022, 2023],
        "week":   [2,     17,   1,    16,   3],
    })
    assert V._window_timeline(pre) == [(2022, 16), (2022, 17),
                                       (2023, 1), (2023, 2), (2023, 3)]


def test_weekly_obs_on_timeline_orders_and_masks_absences():
    """The pure timeline aligner (a) places each player's observations at the
    right chronological (season, week) slot — across seasons, without colliding
    equal week numbers — and (b) marks weeks a player did not appear as
    predict-only (played=False, y=NaN), rather than compressing gaps out."""
    # window spans two seasons; week 1 recurs in each — must NOT collide
    timeline = [(2022, 1), (2022, 2), (2023, 1)]
    per_player = {
        # appears in both seasons' week 1 and 2022 wk2 — a full row
        "A": {(2022, 1): (0.1, 5.0), (2022, 2): (0.2, 6.0), (2023, 1): (0.3, 7.0)},
        # appears only in the two week-1 slots — the 2022 wk2 gap is an absence
        "B": {(2022, 1): (-0.5, 4.0), (2023, 1): (0.5, 8.0)},
    }
    obs = V._weekly_obs_on_timeline(per_player, timeline)

    # A: every slot played, credit in chronological order, no NaN
    assert list(obs["A"]["played"]) == [True, True, True]
    assert list(obs["A"]["y"]) == [0.1, 0.2, 0.3]
    assert list(obs["A"]["snaps"]) == [5.0, 6.0, 7.0]
    # B: the middle slot (2022 wk2) is a modeled absence — played=False and
    # y=NaN (so the Kalman init's nanmean skips it, not a structural zero); the
    # two week-1 observations land in different seasons' slots (no collision)
    assert list(obs["B"]["played"]) == [True, False, True]
    assert obs["B"]["y"][0] == -0.5 and obs["B"]["y"][2] == 0.5
    assert np.isnan(obs["B"]["y"][1])
    assert list(obs["B"]["snaps"]) == [4.0, 0.0, 8.0]


def test_smoothed_talent_enters_ros_fit_and_forecast():
    """Wiring smoothed_talent changes the fitted ROS rate models: with the
    feature dead (the old 0.0 default) it carries no coefficient; supplied as a
    signal that varies with the training rate, the fitted model — and thus the
    projection at a given smoothed_talent — differs.  Guards against the feature
    being silently ignored again."""
    first = OriginResult(season=2023, week=5, n_train_plays=100,
                         ratings={"RB1": 1.0, "RB2": 2.0})
    positions = {"RB1": "RB", "RB2": "RB"}
    stats = _rb_stats(False)                          # RB1 ypc 4.0, RB2 ypc 5.0
    smoothed = {"RB1": -1.0, "RB2": 1.0}              # co-varies with the rate
    dead = V._fit_ros_models(first, stats, positions)                 # -> 0.0
    live = V._fit_ros_models(first, stats, positions, smoothed=smoothed)
    vol, talent = {"rush_attempts": 10.0}, {"rapm_rating": 1.0,
                                             "smoothed_talent": 1.0}
    y_dead = dead["RB"].project_stat_line(vol, talent)["rushing_yards"]
    y_live = live["RB"].project_stat_line(vol, talent)["rushing_yards"]
    assert y_dead != pytest.approx(y_live)


def test_load_history_reuses_passed_stats_without_requery(monkeypatch):
    """_load_history(stats) scores a preloaded frame without touching the DB —
    the point of the _load_stats/_load_history split that main() relies on."""
    import backend.db.connection as DBC

    def _no_db():
        raise AssertionError("_load_history must not re-query when stats passed")
    monkeypatch.setattr(DBC, "get_db", _no_db)
    stats = pd.DataFrame([_blank(player_id="00-01", season=2024, week=3,
                                 passing_yards=250.0, passing_tds=2.0)])
    history = V._load_history(stats)
    assert list(history.columns) == ["player_id", "season", "week", "points"]
    assert history.iloc[0].points == pytest.approx(18.0)   # 250*0.04 + 2*4


def test_degraded_stat_columns_flags_absent_and_all_null():
    """_degraded_stat_columns flags a scoring column that is absent OR present
    but entirely null (both silently score 0.0)."""
    df = pd.DataFrame([_blank(player_id="p", season=2024, week=1)])
    assert V._degraded_stat_columns(df) == []              # full schema, clean
    assert "targets" in V._degraded_stat_columns(df.drop(columns=["targets"]))
    df_nan = df.copy()
    df_nan["rushing_tds"] = np.nan
    assert "rushing_tds" in V._degraded_stat_columns(df_nan)   # present but all-null


def test_fit_ros_models_warns_on_degraded_columns():
    """A complete schema fits silently; a present-but-all-null column raises the
    partial-schema warning (not just an absent column)."""
    import warnings as W
    first = OriginResult(season=2023, week=5, n_train_plays=100,
                         ratings={"RB1": 1.0, "RB2": 2.0})
    positions = {"RB1": "RB", "RB2": "RB"}
    with W.catch_warnings(record=True) as rec:
        W.simplefilter("always")
        V._fit_ros_models(first, _rb_stats(False), positions)
    assert not [w for w in rec if "scoring column" in str(w.message)]
    nan_stats = _rb_stats(False)
    nan_stats["receiving_yards"] = np.nan
    with pytest.warns(UserWarning, match="absent or all-null"):
        V._fit_ros_models(first, nan_stats, positions)


def test_report_surfaces_ros_data_quality(verdict_run, tmp_path):
    """ros_data_quality with degraded columns is written to both artifacts: a
    Markdown section and the JSON dict."""
    results, _ = verdict_run
    results = {**results,
               "ros_data_quality": {"degraded_columns": ["passing_yards", "targets"]}}
    json_path, md_path = write_report(results, out_dir=tmp_path)
    md = md_path.read_text()
    assert "ROS data quality" in md and "passing_yards" in md
    data = json.loads(json_path.read_text())
    assert data["ros_data_quality"]["degraded_columns"] == ["passing_yards", "targets"]


def test_ros_data_quality_reads_fit_window_not_full_history():
    """The report's degraded-column check reads the pre-first-origin fit window
    (via _ros_fit_frame), matching what _fit_ros_models actually fit on — a
    column all-null only in that window but populated later must still surface
    (regression: run_verdict previously read the full, unfiltered stats_history
    and could report clean while the fit-time warning fired)."""
    first = OriginResult(season=2023, week=5, n_train_plays=100,
                         ratings={"RB1": 1.0})
    positions = {"RB1": "RB"}
    stats = _rb_stats(False)
    stats["receiving_yards"] = np.nan                     # null in the fit window
    later = _blank(player_id="RB1", season=2023, week=17, receiving_yards=50.0)
    stats = pd.concat([stats, pd.DataFrame([later])], ignore_index=True)
    # full-history view would see the wk-17 value and read clean...
    assert "receiving_yards" not in V._degraded_stat_columns(stats)
    # ...but the fit-window view (what the report now uses) flags it.
    pre = V._ros_fit_frame(first, stats, positions)
    assert "receiving_yards" in V._degraded_stat_columns(pre)
