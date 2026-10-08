"""Weekly incremental GRID pipeline.

Appends one week's play-by-play data to the RAPM accumulators, advances
the Kalman state for every player, re-solves RAPM from accumulated normal
equations, upserts matchup_grades for next week, then calls
compute_valuations.run() to refresh projections/VOR in the database.

Run:
    python -m backend.pipeline.weekly_update --week 5 --season 2025

Designed to be called by run_pipeline.bat after each game week completes.
Idempotent: re-running the same week re-computes but does not double-count
(accumulator state is keyed by last_week and skips if already done).
"""
from __future__ import annotations

import argparse
import sqlite3
from typing import Any

import numpy as np

from backend.db.connection import get_db, init_db
from backend.grid.layers import (
    accumulate,
    build_design,
    fit_from_accumulators,
    init_accumulators,
    load_accumulators,
    save_accumulators,
)
from backend.grid.situations import classify as classify_situations
from backend.grid.statespace import KalmanState, SSParams, detect_changepoints, kalman_step
from backend.pipeline._logging import get_logger
from backend.pipeline.compute_valuations import run as run_valuations
from backend.pipeline.health_check import write_health

logger = get_logger("weekly_update")

# Penalty configuration matching the full-season fit defaults
_LAM = 120.0
_W_MARKET = 40.0
_NFL_SEASON = 18  # regular-season weeks

# Minimum plays required for a situation sub-frame to run its own RAPM solve.
# Situations below this threshold are skipped to avoid rank-deficiency.
MIN_PLAYS = 50


# ---------------------------------------------------------------------------
# Matchup grade helpers
# ---------------------------------------------------------------------------

def _upsert_matchup_grades(
    conn: sqlite3.Connection,
    beta: Any,
    colidx: dict,
    week: int,
    season: int,
    situation: str = "overall",
) -> None:
    """Upsert per-position defensive RAPM grades for next week's matchups.

    Reads team-defence intercepts from the RAPM solve (colidx['t_def']) and
    sign-flips them so that a stronger defence (more negative intercept, because
    defenders enter X with -1) maps to a *higher* rapm_grade (tougher matchup).

        def_strength = -float(beta[col])   # higher → harder to score against

    The same team-level grade is stored for every skill position ('QB','RB','WR','TE')
    tagged with the given situation ('overall', 'red_zone', 'passing_downs', etc.).

    beta:      coefficient vector from fit_from_accumulators (1-D array, indexed by colidx)
    colidx:    column-index dict from build_design; must contain 't_def': {team: col}
    situation: situation tag to store in matchup_grades.situation column.
    """
    team_def: dict[str, int] = colidx.get("t_def", {})
    if not team_def:
        logger.warning("colidx has no 't_def' entries — skipping matchup grades for week=%d", week)
        return

    rows_written = 0
    for team, col in team_def.items():
        def_strength = -float(beta[col])   # sign flip: harder defence → higher grade
        for pos in ("QB", "RB", "WR", "TE"):
            conn.execute(
                """
                INSERT INTO matchup_grades (week, season, def_team, position, situation, rapm_grade)
                VALUES (?, ?, ?, ?, ?, ?)
                ON CONFLICT(week, season, def_team, position, situation) DO UPDATE SET
                    rapm_grade = excluded.rapm_grade,
                    computed_at = datetime('now')
                """,
                (week, season, team, pos, situation, def_strength),
            )
            rows_written += 1
    conn.commit()
    logger.info("Upserted %d matchup_grades rows for week=%d situation=%s", rows_written, week, situation)


# ---------------------------------------------------------------------------
# Kalman trajectory DB write
# ---------------------------------------------------------------------------

def _write_kalman_trajectory(
    conn: sqlite3.Connection,
    ks: "KalmanState",
    week: int,
    season: int,
    pred_var: "np.ndarray | None" = None,
) -> None:
    """Upsert current Kalman state into kalman_trajectory for viz endpoint.

    Writes one row per player with the current mu/sigma.  scheme_fit is
    included only when mu width == 3 (three-component state).

    total  = sum of all state components (talent + form [+ scheme_fit]) — the
             full state, matching the engine's ``total_filt``.  (Previously it
             silently dropped scheme_fit.)
    var_total = the one-step predictive variance S (R included) when ``pred_var``
             is supplied — the honest uncertainty band the UX should surface.
             Falls back to the full filtered quadratic form ``H·Σ·Hᵀ`` (all
             components + cross-covariances) when no predictive is available.
             (Previously it was ``sigma[0,0]+sigma[1,1]`` — dropping scheme_fit,
             the cross-covariances, and the observation noise R.)
    """
    width = ks.mu.shape[1] if ks.mu.ndim == 2 else 1
    rows_written = 0
    for i, pid in enumerate(ks.player_ids):
        mu = ks.mu[i]
        sigma = ks.sigma[i]
        talent = float(mu[0])
        form = float(mu[1])
        scheme_fit = float(mu[2]) if width >= 3 else None
        total = talent + form + (scheme_fit or 0.0)
        if pred_var is not None:
            var_total = float(pred_var[i])
        else:
            # Full filtered quadratic form Hᵀ Σ H over the active components
            # (includes scheme variance and all cross-covariances).
            var_total = float(sigma[:width, :width].sum())

        if scheme_fit is not None:
            conn.execute(
                """
                INSERT INTO kalman_trajectory
                    (player_id, season, week, talent, form, total, var_total, scheme_fit)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(player_id, season, week) DO UPDATE SET
                    talent=excluded.talent, form=excluded.form, total=excluded.total,
                    var_total=excluded.var_total, scheme_fit=excluded.scheme_fit,
                    computed_at=datetime('now')
                """,
                (pid, season, week, talent, form, total, var_total, scheme_fit),
            )
        else:
            conn.execute(
                """
                INSERT INTO kalman_trajectory
                    (player_id, season, week, talent, form, total, var_total)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(player_id, season, week) DO UPDATE SET
                    talent=excluded.talent, form=excluded.form, total=excluded.total,
                    var_total=excluded.var_total, computed_at=datetime('now')
                """,
                (pid, season, week, talent, form, total, var_total),
            )
        rows_written += 1
    conn.commit()
    logger.info("Upserted %d kalman_trajectory rows for week=%d season=%d", rows_written, week, season)


# ---------------------------------------------------------------------------
# Main weekly update
# ---------------------------------------------------------------------------

def run(
    week: int,
    season: int,
    plays_df: Any = None,
    players_df: Any = None,
    market_strength: dict | None = None,
    interventions: set[str] | None = None,
    _conn: sqlite3.Connection | None = None,
) -> dict:
    """Append week's plays to RAPM accumulators, advance Kalman, refresh grades.

    plays_df / players_df: pre-loaded DataFrames (for testing without nflverse).
    When None, attempts to load from nflverse cache (requires data pipeline run).

    Returns dict with keys: week, season, n_players, rapm_solved, kalman_updated.
    """
    logger.info("=== weekly_update START  week=%d season=%d ===", week, season)

    _owns_conn = _conn is None
    conn = _conn if _conn is not None else init_db()

    # Load plays / players if not provided
    if plays_df is None or players_df is None:
        try:
            from backend.grid.nflverse_adapter import load_grid_plays
            from backend.grid.nflverse_loader import load_rosters
            from backend.grid.value import fit_value_model, attach_dv

            logger.info("Loading nflverse GRID plays for week=%d season=%d", week, season)
            # GRID plays contract (drive_points + next-state + participation) so
            # build_design/RAPM actually run on real data.  (Previously load_pbp's
            # stats shape lacked off_players/def_players, so build_design KeyError'd
            # and this except silently no-opped.)
            all_plays = load_grid_plays([season])
            plays_df = all_plays[all_plays["week"] == week].copy()
            players_df = load_rosters([season])
            value_model = fit_value_model(all_plays)
            plays_df = attach_dv(plays_df, value_model)
        except Exception as exc:
            logger.warning("Could not load nflverse data: %s — skipping RAPM update", exc)
            write_health(status="warning", message=f"weekly_update week={week}: no PBP data")
            if _owns_conn:
                conn.close()
            return {"week": week, "season": season, "rapm_solved": False, "kalman_updated": False}

    if len(plays_df) == 0:
        logger.warning("No plays for week=%d — skipping RAPM update", week)
        if _owns_conn:
            conn.close()
        return {"week": week, "season": season, "n_plays": 0, "rapm_solved": False, "kalman_updated": False}

    # Build design matrix for this week's plays
    logger.info("Building design matrix for %d plays", len(plays_df))
    X, y, colidx = build_design(plays_df, players_df)
    n_cols = colidx["nCols"]

    # Canonical player order for the current week (sorted, matches build_design)
    current_player_order = [str(p) for p in sorted(players_df["player_id"].tolist(), key=str)]

    # Load or init accumulators (overall pass, key='all' → legacy file path)
    loaded = load_accumulators(key="all")
    if loaded is not None:
        XtX, Xty, last_week, stored_player_order = loaded
        if last_week >= week:
            logger.info("Accumulators already current through week=%d — skipping", last_week)
            if _owns_conn:
                conn.close()
            return {"week": week, "season": season, "rapm_solved": False, "kalman_updated": False, "skipped": True}
        if XtX.shape[0] != n_cols:
            logger.warning("Accumulator dimension mismatch (%d vs %d) — reinitialising", XtX.shape[0], n_cols)
            XtX, Xty = init_accumulators(n_cols)
        elif stored_player_order and stored_player_order != current_player_order:
            logger.warning(
                "Accumulator player order changed (reorder detected) — reinitialising"
            )
            XtX, Xty = init_accumulators(n_cols)
    else:
        logger.info("No accumulators found — initialising fresh")
        XtX, Xty = init_accumulators(n_cols)

    # Accumulate this week
    XtX, Xty = accumulate(XtX, Xty, X, y)
    save_accumulators(XtX, Xty, week, player_order=current_player_order, key="all")
    logger.info("Accumulators updated through week=%d", week)

    # Re-solve RAPM from accumulated normal equations
    prior_mean = np.zeros(n_cols)
    mask = np.ones(n_cols)
    for t in colidx["teams"]:
        mask[colidx["t_off"][t]] = 0.05
        mask[colidx["t_def"][t]] = 0.05

    if market_strength is not None:
        for t in colidx["teams"]:
            row = np.zeros(n_cols)
            row[colidx["t_off"][t]] = 1.0
            row[colidx["t_def"][t]] = 1.0
            tgt = market_strength.get(t, 0.0)
            XtX += _W_MARKET * np.outer(row, row)
            Xty += _W_MARKET * row * tgt

    beta = fit_from_accumulators(XtX, Xty, _LAM, prior_mean, mask)

    # Build ratings DataFrame for matchup grades + Kalman observations
    inv_pcol = {v: k for k, v in colidx["p_col"].items()}
    n_players = colidx["nP"]
    player_ids = [inv_pcol[i] for i in range(n_players)]
    ratings_arr = beta[:n_players]

    import pandas as pd
    ratings_df = pd.DataFrame({"player_id": player_ids, "rating": ratings_arr})
    if "player_id" in players_df.columns:
        meta_cols = [c for c in ("player_id", "team", "position") if c in players_df.columns]
        ratings_df = ratings_df.merge(players_df[meta_cols], on="player_id", how="left")

    # Advance Kalman state
    ks = KalmanState.load()
    if ks is None:
        ks = KalmanState.init(player_ids)
    elif ks.player_ids != player_ids:
        # Roster changed — rebuild with new player set, preserving known states
        old_idx = {pid: i for i, pid in enumerate(ks.player_ids)}
        new_ks = KalmanState.init(player_ids)
        for new_i, pid in enumerate(player_ids):
            if pid in old_idx:
                old_i = old_idx[pid]
                new_ks.mu[new_i] = ks.mu[old_i]
                new_ks.sigma[new_i] = ks.sigma[old_i]
        ks = new_ks

    obs = np.array([ratings_arr[i] for i in range(n_players)])
    # Snaps: proxy from play counts where each player appeared
    snaps = np.ones(n_players)

    # ---------------------------------------------------------------------------
    # Coaching-change scheme resets
    # Query coaching_changes for this week; map team→player_ids via players_df.
    # OC/HC changes affect skill-position players (QB/RB/WR/TE) on that team.
    # DC changes affect DEF players on that team.
    # If no coaching_changes table yet or no rows, scheme_resets stays empty.
    # ---------------------------------------------------------------------------
    _SKILL_POSITIONS = {"QB", "RB", "WR", "TE"}
    _DEF_POSITIONS = {"DEF"}

    scheme_resets: set[str] = set()
    try:
        coaching_rows = conn.execute(
            """
            SELECT team, role FROM coaching_changes
            WHERE season = ? AND week_effective = ?
            """,
            (season, week),
        ).fetchall()
    except Exception:
        coaching_rows = []

    if coaching_rows and "player_id" in players_df.columns and "team" in players_df.columns:
        team_lookup = players_df.set_index("player_id")
        for row in coaching_rows:
            chg_team = row[0]
            chg_role = row[1]
            if chg_role in ("HC", "OC"):
                target_positions = _SKILL_POSITIONS
            else:  # DC
                target_positions = _DEF_POSITIONS
            for pid, prow in team_lookup.iterrows():
                if prow.get("team") == chg_team and prow.get("position") in target_positions:
                    scheme_resets.add(str(pid))

    if scheme_resets:
        logger.info(
            "Scheme resets for week=%d: %d players across coaching-change teams",
            week, len(scheme_resets),
        )

    # Position-grouped Kalman step: each position gets its own SSParams
    pos_lookup = {}
    if "player_id" in players_df.columns and "position" in players_df.columns:
        pos_lookup = players_df.set_index("player_id")["position"].to_dict()

    positions_by_idx: dict[str, list[int]] = {}
    for i, pid in enumerate(player_ids):
        pos = pos_lookup.get(pid, "UNKNOWN")
        positions_by_idx.setdefault(pos, []).append(i)

    # Per-player one-step predictive variance (R included), scattered back from
    # each position's kalman_step so the trajectory band reflects the honest
    # forecast uncertainty rather than the R-omitted filtered variance.
    pred_var_all = np.full(n_players, np.nan)

    for pos, indices in positions_by_idx.items():
        params = SSParams.from_position(pos)
        idx_arr = np.array(indices)
        sub_state = KalmanState(
            mu=ks.mu[idx_arr],
            sigma=ks.sigma[idx_arr],
            player_ids=[ks.player_ids[i] for i in indices],
        )
        sub_obs = obs[idx_arr]
        sub_snaps = snaps[idx_arr]
        manual_intv = {ks.player_ids[i] for i in indices if ks.player_ids[i] in (interventions or set())}
        sub_resets = {ks.player_ids[i] for i in indices if ks.player_ids[i] in scheme_resets}

        # Auto-detect changepoints using this position's params and sub-slices
        auto_intv = detect_changepoints(sub_state, sub_obs, sub_snaps, params)
        sub_intv = manual_intv | auto_intv
        if auto_intv:
            logger.info(
                "Auto-detected %d changepoint(s) for pos=%s week=%d: %s",
                len(auto_intv), pos, week, sorted(auto_intv),
            )

        updated, sub_pred_var = kalman_step(
            sub_state, sub_obs, sub_snaps, params,
            interventions=sub_intv or None,
            scheme_resets=sub_resets or None,
            return_pred=True,
        )
        ks.mu[idx_arr] = updated.mu
        ks.sigma[idx_arr] = updated.sigma
        pred_var_all[idx_arr] = sub_pred_var
    ks.save()
    logger.info("Kalman state advanced for %d players", n_players)

    # ---------------------------------------------------------------------------
    # Write Kalman trajectory snapshot to DB (for viz endpoint).
    # Guarded with try/except so that a DB schema that pre-dates this table
    # (e.g., a test fixture initialised before the migration) does not break
    # the pipeline run.  The _owns_conn pattern ensures we only call this when
    # a connection is available (conn is always set by this point).
    # ---------------------------------------------------------------------------
    try:
        _write_kalman_trajectory(conn, ks, week, season, pred_var=pred_var_all)
    except sqlite3.OperationalError as _traj_exc:
        logger.warning("kalman_trajectory write skipped: %s", _traj_exc)

    # Upsert matchup grades using team-defence intercepts from the RAPM solve.
    # beta/colidx are already in scope from the solve above.
    _upsert_matchup_grades(conn, beta, colidx, week + 1, season, situation="overall")

    # ---------------------------------------------------------------------------
    # Per-situation keyed accumulator pass (red_zone, passing_downs, rushing_downs, …)
    # interactions=False on this path (documented) — enabling interactions changes
    # nCols and would require their own keyed accumulators; that is a future task.
    # Guarded: if the plays frame lacks situation-classifier columns (down, ydstogo,
    # yardline_100) we skip the pass rather than raising — this keeps the pipeline
    # runnable from partial/test data while real PBP always has the full contract.
    # ---------------------------------------------------------------------------
    _SIT_REQUIRED_COLS = {"down", "ydstogo", "yardline_100"}
    if not _SIT_REQUIRED_COLS.issubset(plays_df.columns):
        logger.info(
            "Situation pass skipped: plays_df missing columns %s",
            _SIT_REQUIRED_COLS - set(plays_df.columns),
        )
        sit_masks = {}
    else:
        sit_masks = classify_situations(plays_df)

    for sit_name, sit_mask in sit_masks.items():
        sub_plays = plays_df.loc[sit_mask]
        if len(sub_plays) < MIN_PLAYS:
            logger.info("skip situation=%s: %d < %d plays", sit_name, len(sub_plays), MIN_PLAYS)
            continue

        Xs, ys, cidx_s = build_design(sub_plays, players_df)
        nc_s = cidx_s["nCols"]

        ld_s = load_accumulators(key=sit_name)
        if ld_s is not None:
            XtX_s, Xty_s, last_week_s, stored_order_s = ld_s
            if last_week_s >= week:
                logger.info(
                    "situation=%s accumulators already current through week=%d — skipping",
                    sit_name, last_week_s,
                )
                continue
            if XtX_s.shape[0] != nc_s:
                logger.warning(
                    "situation=%s accumulator dimension mismatch (%d vs %d) — reinitialising",
                    sit_name, XtX_s.shape[0], nc_s,
                )
                XtX_s, Xty_s = init_accumulators(nc_s)
            elif stored_order_s and stored_order_s != current_player_order:
                logger.warning(
                    "situation=%s accumulator player order changed — reinitialising",
                    sit_name,
                )
                XtX_s, Xty_s = init_accumulators(nc_s)
        else:
            XtX_s, Xty_s = init_accumulators(nc_s)

        XtX_s, Xty_s = accumulate(XtX_s, Xty_s, Xs, ys)
        save_accumulators(XtX_s, Xty_s, week, player_order=current_player_order, key=sit_name)
        logger.info("situation=%s accumulators updated through week=%d", sit_name, week)

        # Build ridge mask for this sub-frame (same structure as overall pass)
        prior_s = np.zeros(nc_s)
        mask_s = np.ones(nc_s)
        for t in cidx_s["teams"]:
            mask_s[cidx_s["t_off"][t]] = 0.05
            mask_s[cidx_s["t_def"][t]] = 0.05

        beta_s = fit_from_accumulators(XtX_s, Xty_s, _LAM, prior_s, mask_s)
        _upsert_matchup_grades(conn, beta_s, cidx_s, week + 1, season, situation=sit_name)

    # Refresh VOR projections
    run_valuations(season=season, _conn=conn)

    if _owns_conn:
        conn.close()

    write_health(status="ok", message=f"weekly_update week={week} season={season} complete")
    logger.info("=== weekly_update DONE ===")
    return {
        "week": week,
        "season": season,
        "n_players": n_players,
        "n_plays": len(plays_df),
        "rapm_solved": True,
        "kalman_updated": True,
    }


def _parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    p = argparse.ArgumentParser(description="Incremental weekly GRID update")
    p.add_argument("--week", type=int, required=True, help="NFL week number (1-18)")
    p.add_argument("--season", type=int, default=2025, help="NFL season year")
    return p.parse_args(argv)


if __name__ == "__main__":
    args = _parse_args()
    run(week=args.week, season=args.season)
