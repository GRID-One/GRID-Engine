"""The Phase-2c verdict runner (roadmap §5 Phase 2c; validation plan §12 6-8).

Stitches the whole trust layer together: walk-forward GRID ratings ->
fantasy forecasts -> Tier-1 skill/calibration tables (weekly + ROS), the H2
lineup-margin simulation, the Tier-2 matchup check, gate promotion into the
calibrate-then-gate registry, and the **H1 kill-criterion verdict**:

    SHIP_GRID          - ROS margin vs last-season-actuals > 0, CI excludes 0
    BASELINE_FALLBACK  - it isn't (compute_valuations keeps the baseline)
    INCONCLUSIVE       - no paired last-season cells to judge on

The verdict is *rendered*, not acted on — flipping ``compute_valuations`` to
GRID-powered by default is a deliberate follow-up gated on the measured H1
(the user's call), never an automatic side effect of a validation run.

:func:`run_verdict` is the pure, synth-testable driver (frames in, results
dict out — no disk, no network).  ``python -m backend.validation.verdict`` is
the **dev/nightly real run** (§7.5 B — NOT CI): it needs the frozen snapshot
(``python -m backend.pipeline.ingest_grid --years 2019 ... 2024``, one-time
network ingest into the gitignored ``data/snapshots/``) and a populated
SQLite DB (``python -m backend.pipeline.data_pipeline`` for the realized
weekly stat lines), then writes ``data/reports/phase2c_verdict.{json,md}``
and fills the committed thresholds registry on the first run.

Gate promotion uses each KPI's **sign-flip null**: the observed per-unit
samples with random signs (seeded) — the distribution the paired margin
would have if GRID carried no signal — so the frozen gate is that KPI's own
measured noise floor (``mean + 1.96*SE``), per roadmap §6.5 #8.
"""
from __future__ import annotations

import argparse
import json
import warnings
from pathlib import Path

import numpy as np
import pandas as pd

from backend.grid.layers import layer1_all_players
from backend.grid.value import attach_dv, fit_value_model
from backend.projection.features import assemble_smoothed_talent
from backend.projection.model import fit_stat_line_model
from backend.projection.sv_to_points import fit_sv_to_points
from backend.projection.volume import project_volume
from backend.scoring.columns import SCORING_STAT_COLS
from backend.scoring.formats import STANDARD
from backend.validation import baselines as B
from backend.validation.asof import AsOf
from backend.validation.backtest import walk_forward
from backend.validation.lineup_sim import draft_rosters, simulate_weekly_margin
from backend.validation.report import write_report
from backend.validation.thresholds import (
    gate_check,
    load_registry,
    promote,
    save_registry,
)
from backend.validation.tier1 import (
    forecast_ros_points,
    forecast_weekly_points,
    tier1_report,
)
from backend.validation.tier2 import tier2_report

# Skill-position slots for the synthetic H2 rosters: the projection universe
# has no kickers/defences, so the app's DEFAULT_ROSTER_SLOTS (K/DEF) doesn't fit.
H2_ROSTER_SLOTS = {"QB": 1, "RB": 2, "WR": 2, "TE": 1, "FLEX": 1}


def _pre_first_origin(history: pd.DataFrame, asof: AsOf) -> pd.DataFrame:
    """History rows whose outcomes are known before the first forecast origin."""
    prior = history[history["season"] < asof.season]
    cur = history[(history["season"] == asof.season)
                  & (history["week"] <= asof.outcome_cutoff)]
    return pd.concat([prior, cur], ignore_index=True)


def _fit_points_map(first_origin, history: pd.DataFrame, positions: dict):
    """Per-position affine rating->points map, fit strictly pre-first-origin.

    The first origin's RAPM ratings are built from the warm-up weeks only, and
    the realized points they are paired with are sliced to before that origin
    (prior seasons + warm-up) by ``fit_sv_to_points``'s AsOf cutoff — so the
    map is frozen on pre-forecast information and reused at every origin,
    mirroring the frozen-V(s) discipline of the backtest driver.
    """
    asof = AsOf(season=first_origin.season, week=first_origin.week)
    rows = [{"season": int(r.season), "week": int(r.week),
             "position": positions[r.player_id],
             "credit": float(first_origin.ratings[r.player_id]),
             "fantasy_points": float(r.points)}
            for r in history.itertuples(index=False)
            if r.player_id in first_origin.ratings and r.player_id in positions]
    return fit_sv_to_points(pd.DataFrame(rows), asof)


def _degraded_stat_columns(stats: pd.DataFrame) -> list:
    """`SCORING_STAT_COLS` that are absent or entirely null in ``stats``.

    The ROS stat-line model scores such a column as a constant 0.0 (an absent
    column is filled with 0.0; an all-null column sums to 0.0 under skipna), so
    a partial DB schema silently zeroes those stats.  Surfacing them — in the
    fit-time warning and in the verdict report — keeps that degradation visible
    rather than shipping a plausible-looking but partially-zero ROS table.
    """
    return [c for c in SCORING_STAT_COLS
            if c not in stats.columns or stats[c].isna().all()]


def _ros_fit_frame(first_origin, stats: pd.DataFrame,
                   positions: dict) -> pd.DataFrame:
    """Pre-first-origin player-season rows the ROS models are fit on.

    Prior seasons + warm-up (frozen, leakage-safe via ``_pre_first_origin``),
    restricted to rated & positioned players.  Shared by :func:`_fit_ros_models`
    (the fit + its warning) and ``run_verdict``'s report-facing data-quality
    check so both read the **identical** frame — a column degraded only within
    this window can't warn at fit time yet read clean in the report.
    """
    pre = _pre_first_origin(stats, AsOf(season=first_origin.season,
                                        week=first_origin.week))
    return pre[pre["player_id"].isin(first_origin.ratings)
               & pre["player_id"].isin(positions)]


def _smoothed_talent_pre_first_origin(
    plays: pd.DataFrame, first_origin, positions: dict,
    *, n_splits: int = 5, seed: int = 0, min_plays: int = 20,
) -> dict:
    """Per-player end-of-pre-period RTS-smoothed talent, frozen pre-first-origin.

    The ROS stat-line model is designed to regress each per-unit rate on GRID's
    full talent vector (``model.TALENT_FEATURE_COLS`` = ``rapm_rating``,
    ``smoothed_talent``, ``prior_mean``), but only ``rapm_rating`` was ever
    supplied — the other two entered as the constant 0.0 the predict side
    defaults to, so two thirds of GRID's efficiency signal was dead weight in
    the projection the H1 verdict rests on.  This turns ``smoothed_talent`` on.

    It is the §4.5 "best retrospective talent" (the preseason cold-start
    signal): the RTS-smoothed end-of-period Kalman talent, run over each
    player's **weekly Layer-1 credit** series (`layers.layer1_all_players`, the
    per-position weekly signal Phase 2b built) up to — and strictly before — the
    first forecast origin.  Mirroring the frozen ``sv_map``/V(s)/rate-model
    discipline, it is computed **once** on the pre-first-origin window and reused
    at every origin: ``rapm_rating`` is the fast, season-to-date talent that
    updates per origin, ``smoothed_talent`` the slow retrospective prior-period
    talent, so freezing the latter is the design, not a shortcut.

    Leakage safety: every input is strictly before the first origin — the plays
    (``_pre_first_origin`` cutoff), the V(s) refit on them, and
    ``first_origin.ratings`` as the opponent-adjustment lookup (built from the
    warm-up only).  Layer-1 groups weekly credit by week, so a pre-window that
    ever spanned seasons would collide week numbers; the credit is therefore
    computed **per season** and each player's observations concatenated in
    chronological ``(season, week)`` order before smoothing (a no-op in the
    single-season pre-window today, robust if the warm-up ever crosses a
    season boundary).

    Returns ``{player_id: smoothed_talent}`` for players with >= ``min_plays``
    on-field offensive snaps in a season of the window (``min_plays`` is applied
    per season inside `layer1_all_players`); absent players fall back to the 0.0
    the fit/predict feature space already supplies (RAPM's ridge-toward-zero
    "no evidence = average" prior).
    """
    fo_s, fo_w = int(first_origin.season), int(first_origin.week)
    # Route the pre-origin slice through the AsOf leakage choke point (outcomes
    # known only through W-1) rather than hand-rolling the cutoff.
    pre = AsOf(season=fo_s, week=fo_w).slice_plays(plays)
    if pre.empty:
        return {}
    pre = attach_dv(pre, fit_value_model(pre))
    ratings_lookup = {pid: float(r) for pid, r in first_origin.ratings.items()}
    player_ids = [pid for pid in first_origin.ratings if pid in positions]

    # Weekly Layer-1 credit, computed per season (layer1 groups by week only, so
    # a season-spanning single call would collide week numbers across seasons).
    per_player: dict = {}   # pid -> {(season, week): (credit, snaps)}
    for season, grp in pre.groupby("season"):
        weekly = layer1_all_players(grp, ratings_lookup, player_ids,
                                    n_splits=n_splits, seed=seed,
                                    min_plays=min_plays)
        for pid, df in weekly.items():
            slots = per_player.setdefault(pid, {})
            for r in df.itertuples(index=False):
                slots[(int(season), int(r.week))] = (float(r.credit),
                                                     float(r.snaps))

    smoothed, _var = assemble_smoothed_talent(
        _weekly_obs_on_timeline(per_player, _window_timeline(pre)), positions)
    return smoothed


def _window_timeline(pre: pd.DataFrame) -> list:
    """The full chronological ``[(season, week), ...]`` timeline of the window.

    Every distinct game-week in ``pre``, sorted — the reference axis the RTS
    smoother walks in order, so the sort is load-bearing (a scrambled timeline
    misplaces observations in time).  Season-aware, so equal week numbers in
    different seasons never collide.
    """
    return sorted({(int(s), int(w))
                   for s, w in zip(pre["season"], pre["week"])})


def _weekly_obs_on_timeline(per_player: dict, timeline: list) -> dict:
    """Align each player's ``{(season, week): (credit, snaps)}`` onto the full
    chronological ``timeline`` of the window, as the Kalman observation triples.

    A week a player did NOT appear enters the smoother as a predict-only step
    (``played=False``), so absences grow talent uncertainty instead of being
    compressed out — the missing-week semantics the state-space layer relies on
    ("missing weeks predict without updating so uncertainty grows during
    absences").  The absent week's ``y`` is ``NaN`` (not 0.0): the Kalman's
    talent init is ``nanmean(y[:3])``, which reads ``y`` directly *without*
    consulting ``played``, so a structural zero there would bias the whole
    trajectory toward 0 — ``NaN`` is the value the init skips (the missing-week
    contract implemented in `statespace.kalman_two_component`'s ``nanmean`` init
    and documented on `features.assemble_smoothed_talent`).  Pure (no estimation)
    and deterministic so the ordering/masking is unit-testable without the
    Layer-1 GBM.
    """
    idx = {slot: i for i, slot in enumerate(timeline)}
    n = len(timeline)
    obs: dict = {}
    for pid, slots in per_player.items():
        y = np.full(n, np.nan)           # absent weeks: NaN, ignored by the init
        snaps = np.zeros(n)
        played = np.zeros(n, dtype=bool)
        for slot, (credit, snap) in slots.items():
            i = idx[slot]
            y[i], snaps[i], played[i] = credit, snap, True
        obs[pid] = {"y": y, "snaps": snaps, "played": played}
    return obs


def _fit_ros_models(first_origin, stats: pd.DataFrame, positions: dict,
                    smoothed: dict | None = None) -> dict:
    """Per-position stat-line models for the ROS forecast, frozen pre-first-origin.

    The design assigns ROS/draft projection to the season stat-line model
    (:mod:`backend.projection.model`), not the weekly SV->points affine map.
    Mirroring the frozen ``sv_map``/V(s) discipline, the rate models are fit
    once on player-season totals known strictly before the first forecast
    origin (prior seasons + warm-up), then reused at every origin.

    Two of the three talent features (``model.TALENT_FEATURE_COLS``) come from
    the walk-forward and this frozen window: ``rapm_rating`` is the first
    origin's RAPM rating (the per-player as-of talent), and ``smoothed_talent``
    is the frozen end-of-pre-period RTS-smoothed talent
    (:func:`_smoothed_talent_pre_first_origin`) when ``smoothed`` is supplied,
    else 0.0 (the back-compatible default that keeps the pre-``smoothed`` feature
    space intact).  ``prior_mean`` stays 0.0 until cross-league priors are wired
    onto real data (``priors.py`` is synth-only today).  The **same** feature
    values are supplied here and in :func:`~backend.validation.tier1.forecast_ros_points`,
    so fit and forecast share one feature space.  Missing stat columns are
    filled with 0.0 so a partial DB schema degrades gracefully (that driver's
    rate models just don't fit).
    """
    pre = _ros_fit_frame(first_origin, stats, positions)
    if pre.empty:
        return {}
    present = [c for c in SCORING_STAT_COLS if c in pre.columns]
    degraded = _degraded_stat_columns(pre)
    if degraded:
        warnings.warn(
            f"_fit_ros_models: {len(degraded)} scoring column(s) absent or "
            f"all-null in player_stats {degraded}; their rate models default "
            f"to 0.0 (partial-schema stat lines are silently zero for these)",
            stacklevel=2)
    agg = pre.groupby(["player_id", "season"])[present].sum().reset_index()
    for c in SCORING_STAT_COLS:
        if c not in agg.columns:
            agg[c] = 0.0
    agg["position"] = agg["player_id"].map(positions)
    agg["rapm_rating"] = agg["player_id"].map(
        {pid: float(r) for pid, r in first_origin.ratings.items()})
    agg["smoothed_talent"] = (agg["player_id"].map(smoothed).fillna(0.0)
                              if smoothed else 0.0)
    agg["prior_mean"] = 0.0
    return {pos: fit_stat_line_model(grp)
            for pos, grp in agg.groupby("position")}


def _h2(origins, forecasts, history, positions, roster_slots, num_teams,
        num_rounds, out, n, alpha, seed):
    """The H2 lineup-margin simulation over the walk-forward origin weeks.

    Rosters are VOR-drafted from pre-first-origin per-game means (shared,
    method-neutral draft info); GRID's per-week projection comes from the
    walk-forward forecasts, the reference from the last-season baseline at the
    same as-of cutoff; realized points decide.  Week keys are ``(season,
    week)`` tuples so multi-season snapshots don't collide.
    """
    first = AsOf(season=origins[0].season, week=origins[0].week)
    pre = _pre_first_origin(history, first)
    pool = [{"player_id": pid, "position": positions[pid],
             "projected_points": float(g["points"].mean())}
            for pid, g in pre.groupby("player_id") if pid in positions]
    rosters = draft_rosters(pool, roster_slots=roster_slots,
                            num_teams=num_teams, num_rounds=num_rounds)

    proj_a: dict = {}
    for r in forecasts.itertuples(index=False):
        proj_a.setdefault((int(r.season), int(r.week)), {})[r.player_id] = float(r.forecast)
    proj_b: dict = {}
    actual: dict = {}
    for o in origins:
        key = (o.season, o.week)
        proj_b[key] = B.last_season(history, AsOf(season=o.season, week=o.week)).mean
        wk = history[(history["season"] == o.season) & (history["week"] == o.week)]
        actual[key] = {pid: float(g["points"].mean())
                       for pid, g in wk.groupby("player_id")}

    res = simulate_weekly_margin(rosters, proj_a, proj_b, actual, roster_slots,
                                 out=out, n=n, alpha=alpha, seed=seed)
    return {"margins": res.margins, "win_rate": res.win_rate,
            "overall": res.overall, "starter_out": res.starter_out,
            "starter_out_margins": res.starter_out_margins}


def _tier2(origins, allowed, n, alpha, seed):
    """Tier-2 matchup check over the walk-forward defensive grades."""
    rows = [{"season": o.season, "week": o.week, "def_team": t, "rapm_grade": g}
            for o in origins for t, g in o.def_grades.items()]
    if not rows:
        return None
    return tier2_report(pd.DataFrame(rows), allowed, n=n, alpha=alpha, seed=seed)


def _sign_flip_null(samples, seed: int):
    """A KPI's no-signal control: the observed samples with random signs."""
    rng = np.random.default_rng(seed)
    s = np.asarray(samples, dtype=float)
    return s * rng.choice([-1.0, 1.0], size=s.size)


def run_verdict(
    plays: pd.DataFrame,
    players: pd.DataFrame,
    history: pd.DataFrame,
    *,
    stats_history: pd.DataFrame | None = None,
    allowed: pd.DataFrame | None = None,
    out: dict | None = None,
    positions: dict | None = None,
    roster_slots: dict | None = None,
    num_teams: int = 8,
    num_rounds: int = 8,
    warmup_weeks: int = 4,
    origin_stride: int = 1,
    market: dict | None = None,
    registry: dict | None = None,
    run_label: str | None = None,
    n: int = 10000,
    alpha: float = 0.05,
    seed: int = 0,
) -> dict:
    """Run the full Phase-2c verdict on in-memory frames; return the results dict.

    ``plays``/``players`` feed :func:`~backend.validation.backtest.walk_forward`
    (the GRID plays contract + fixed player frame); ``history`` is the tidy
    realized weekly fantasy-points frame (``[player_id, season, week,
    points]``) every baseline and target reads.  ``stats_history`` is the raw
    weekly ``player_stats``-shaped frame (the box-score columns behind
    ``history``); when supplied, the ROS (H1) forecast is scored by the season
    volume x efficiency stat-line model (the design's ROS projection) instead
    of the weekly SV->points map — ``None`` keeps the pure synth driver on the
    weekly forecast, unchanged.  ``allowed`` (``[season, week,
    def_team, points_allowed]``) enables Tier 2 — without a points-allowed
    feed that section is skipped, visibly, in the report.  ``out`` optionally
    maps ``(season, week) -> set(player_id)`` unavailable that week (the H2
    starter-OUT subset needs it).  ``registry`` is the calibrate-then-gate
    dict (freeze-once promotion happens in place; the caller persists it);
    ``None`` uses a throwaway so the pure driver never touches disk.

    Returns ``{run_label, n_origins, verdict, verdict_reason, tier1_ros,
    tier1_weekly, h2, tier2, gates, ros_data_quality, registry}`` — ready for
    :func:`backend.validation.report.write_report`.  ``ros_data_quality`` is
    ``{"degraded_columns": [...]}`` when ``stats_history`` was supplied (the
    ``SCORING_STAT_COLS`` absent/all-null in it, scored as 0.0), else ``None``.
    """
    # Dedupe the player universe once, at the boundary: run_verdict is the pure
    # driver and can't assume a caller's players frame is deduped.  A duplicate
    # player_id must perturb nothing downstream — not the RAPM design
    # (`walk_forward`/`build_design`) nor the volume merge — so dedupe here
    # rather than defend each consumer piecemeal.
    players = players.drop_duplicates("player_id").reset_index(drop=True)
    if positions is None:
        positions = dict(zip(players["player_id"], players["position"],
                             strict=True))
    if roster_slots is None:
        roster_slots = dict(H2_ROSTER_SLOTS)
    if registry is None:
        registry = {"kpis": {}}

    origins = walk_forward(plays, players, warmup_weeks=warmup_weeks,
                           origin_stride=origin_stride, market=market)
    if not origins:
        raise ValueError(
            "walk_forward produced no origins — provide more history or lower "
            "warmup_weeks/origin_stride")
    sv_map = _fit_points_map(origins[0], history, positions)
    forecasts = forecast_weekly_points(origins, sv_map, positions)

    # ROS (H1) is scored by the season volume x efficiency stat-line model when
    # a weekly stats frame is supplied (the design's ROS/draft projection);
    # without one, the pure synth driver falls back to the weekly SV->points
    # forecast so existing callers are unaffected.
    if stats_history is not None:
        # Frozen pre-first-origin GRID talent: the smoothed end-of-pre-period
        # Kalman talent (the §4.5 retrospective signal), supplied to BOTH the
        # fit and the forecast so they share one feature space.
        smoothed = _smoothed_talent_pre_first_origin(plays, origins[0],
                                                     positions, seed=seed)
        ros_models = _fit_ros_models(origins[0], stats_history, positions,
                                     smoothed=smoothed)
        # players is already deduped on player_id at entry, so this merge can't
        # fan out and inflate the volume aggregates.
        usage = stats_history.merge(players[["player_id", "position"]],
                                    on="player_id", how="inner")
        volume_by_origin = {
            (o.season, o.week): project_volume(usage,
                                               AsOf(season=o.season, week=o.week))
            for o in origins}
        ros_forecasts = forecast_ros_points(origins, ros_models,
                                            volume_by_origin, positions, STANDARD,
                                            smoothed=smoothed)
        # data-quality read from the SAME pre-first-origin frame the fit used,
        # so the report can't read clean while the fit-time warning fired.
        ros_pre = _ros_fit_frame(origins[0], stats_history, positions)
        ros_data_quality = {"degraded_columns": (
            [] if ros_pre.empty else _degraded_stat_columns(ros_pre))}
    else:
        ros_forecasts = forecasts
        ros_data_quality = None

    tier1_weekly = tier1_report(forecasts, history, horizon="weekly",
                                n=n, alpha=alpha, seed=seed)
    tier1_ros = tier1_report(ros_forecasts, history, horizon="ros",
                             keep_margins=True, n=n, alpha=alpha, seed=seed)
    h2 = _h2(origins, forecasts, history, positions, roster_slots,
             num_teams, num_rounds, out, n, alpha, seed)
    tier2 = (_tier2(origins, allowed, n, alpha, seed)
             if allowed is not None else None)

    # ---- the H1 kill criterion -------------------------------------------- #
    ls = tier1_ros.get("ALL", {}).get("baselines", {}).get("last_season")
    if ls is None:
        verdict, reason = "INCONCLUSIVE", "no paired last-season ROS cells"
    elif ls["margin"].point > 0 and ls["margin"].excludes_zero:
        verdict = "SHIP_GRID"
        reason = (f"ROS margin vs last-season {ls['margin'].point:+.3f} "
                  f"[{ls['margin'].lower:+.3f}, {ls['margin'].upper:+.3f}] excludes 0")
    else:
        verdict = "BASELINE_FALLBACK"
        reason = (f"ROS margin vs last-season {ls['margin'].point:+.3f} "
                  f"[{ls['margin'].lower:+.3f}, {ls['margin'].upper:+.3f}] "
                  f"does not clear the kill criterion")

    # ---- calibrate-then-gate: promote noise floors, check values ---------- #
    kpis: dict = {}
    if ls is not None:
        kpis["h1_ros_margin_vs_last_season"] = (ls["margin"].point, ls["margins"])
    if h2["margins"]:
        kpis["h2_weekly_lineup_margin"] = (h2["overall"].point, h2["margins"])
    if tier2 is not None:
        kpis["tier2_matchup_predictive"] = (tier2["predictive"].point, tier2["weekly"])

    gates: dict = {}
    for i, (name, (value, samples)) in enumerate(sorted(kpis.items())):
        if len(samples) >= 2:
            promote(registry, name, _sign_flip_null(samples, seed + i),
                    run_label=run_label)
        passed, gate = gate_check(registry, name, float(value))
        gates[name] = {"value": float(value), "gate": gate, "passed": passed}

    return {"run_label": run_label, "n_origins": len(origins),
            "verdict": verdict, "verdict_reason": reason,
            "tier1_ros": tier1_ros, "tier1_weekly": tier1_weekly,
            "h2": h2, "tier2": tier2, "gates": gates,
            "ros_data_quality": ros_data_quality, "registry": registry}


# --------------------------------------------------------------------------- #
# The dev/nightly real run (NOT CI)                                            #
# --------------------------------------------------------------------------- #
def _load_snapshot(snapshot_dir: str) -> tuple[pd.DataFrame, list[int]]:
    """Concatenate the frozen per-season snapshot parquets, tagging seasons.

    Returns ``(plays, years)`` — the manifest's season list also keys the
    roster load for the RAPM player universe.
    """
    snap = Path(snapshot_dir)
    manifest = json.loads((snap / "manifest.json").read_text())
    frames = []
    for year in manifest["years"]:
        df = pd.read_parquet(snap / f"grid_plays_{year}.parquet")
        if "season" not in df.columns:
            df["season"] = int(year)
        frames.append(df)
    if not frames:
        raise FileNotFoundError(
            f"no seasons in {snap / 'manifest.json'} — run "
            f"`python -m backend.pipeline.ingest_grid` first")
    return pd.concat(frames, ignore_index=True), [int(y) for y in manifest["years"]]


def _load_players_frame(years: list[int]) -> pd.DataFrame:
    """The RAPM player universe: the full nflverse roster, all positions.

    Mirrors the production forward path (``weekly_update`` feeds
    ``load_rosters`` output to ``build_design``): the snapshot's
    ``off_players``/``def_players`` are roster GSIS ids and are mostly
    defenders/linemen, so the universe must NOT be the fantasy-skill subset —
    a skill-only frame makes ``build_design`` drop every participant as
    unknown.  Multi-season rosters are deduped to each player's most recent
    row (one column per player; latest team for the intercepts).  Reads the
    parquet cache the ingest already populated — no new network.
    """
    from backend.grid.nflverse_loader import load_rosters

    rosters = load_rosters(years)
    rosters = rosters[rosters["team"].notna()]
    # stable sort: same-season ties (mid-season team changes) keep the
    # loader's row order, so "latest team" is deterministic
    rosters = (rosters.sort_values("season", kind="stable")
               .drop_duplicates("player_id", keep="last"))
    return rosters[["player_id", "position", "team"]].reset_index(drop=True)


def _load_stats() -> pd.DataFrame:
    """Raw weekly ``player_stats`` frame from SQLite (GSIS player ids).

    The box-score columns behind ``_load_history``; the ROS forecast fits the
    season stat-line model (volume x efficiency) and projects usage on them,
    so the raw frame is loaded once and shared with ``run_verdict``.
    """
    from backend.db.connection import get_db

    conn = get_db()
    try:
        return pd.read_sql_query("SELECT * FROM player_stats", conn)
    finally:
        conn.close()


def _load_history(stats: pd.DataFrame | None = None) -> pd.DataFrame:
    """Realized weekly fantasy-points history from SQLite (GSIS player ids).

    Weekly points are scored with the STANDARD format — the H1/H2 currency;
    format sensitivity is a report-level follow-up, not a verdict input.
    ``stats`` may be passed to reuse an already-loaded ``player_stats`` frame
    (the ROS path loads it for the stat-line model); ``None`` loads it here.
    """
    from backend.scoring.engine import calculate_points

    if stats is None:
        stats = _load_stats()
    stat_cols = [c for c in stats.columns
                 if c not in ("player_id", "season", "week")]
    history = stats[["player_id", "season", "week"]].copy()
    history["points"] = [calculate_points(row, STANDARD)
                         for row in stats[stat_cols].to_dict("records")]
    return history


def main(argv: list[str] | None = None) -> None:
    """CLI: run the real verdict on the frozen snapshot + DB, write artifacts."""
    parser = argparse.ArgumentParser(description="Phase-2c verdict run (dev/nightly)")
    parser.add_argument("--snapshot-dir", default="data/snapshots")
    parser.add_argument("--origin-stride", type=int, default=1,
                        help="forecast every k-th origin (fast dev iteration)")
    parser.add_argument("--run-label", default=None,
                        help="label frozen into newly promoted gates")
    args = parser.parse_args(argv)

    plays, years = _load_snapshot(args.snapshot_dir)
    players = _load_players_frame(years)
    stats = _load_stats()
    history = _load_history(stats)
    registry = load_registry()
    results = run_verdict(plays, players, history,
                          stats_history=stats,
                          origin_stride=args.origin_stride,
                          registry=registry, run_label=args.run_label)
    save_registry(registry)
    json_path, md_path = write_report(results)
    print(f"{results['verdict']}: {results['verdict_reason']}")
    print(f"report: {md_path} (+ {json_path})")


if __name__ == "__main__":
    main()
