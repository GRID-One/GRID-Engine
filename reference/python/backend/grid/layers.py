"""
Attribution engine.

Layer 2 (the workhorse): regularized adjusted plus-minus over participation.
Every play is an observation; each player is a column (+1 if on the field for
the offense, -1 if on defense). Team-offense / team-defense intercepts absorb
line + scheme + baseline so individual skill players are NOT credited with the
offensive line's work -- this is how we keep OL as a nuisance rather than an
estimand. Ridge with a prior MEAN shrinks toward replacement / a Layer-1 seed.

Layer 3 (market reconciliation): extra pseudo-observations tie each team's
intercept gap to a market-implied team strength (here, synthetic "closing
line"). Anchors the otherwise free team-level constant and is a falsifiable
hook (does bottom-up player value reconstruct the market?).

Layer 1 (event credit): a cross-fitted, opponent-adjusted residual. Its job is
to supply the *weekly* signal the state-space layer consumes. The context model
g(state, opponent-defense-rating) is position-agnostic, so it is fit ONCE per
plays frame and shared across every player's weekly aggregation
(`_cross_fitted_context_residual`); `layer1_qb_weekly` backs onto it for a
single QB, `layer1_all_players` generalizes it to any set of players (roadmap
Phase 2b) -- tractable at RB/WR/TE scale, unlike refitting per player. Layer 2
still carries positions/weeks without a Layer-1 credit computed for them.

Fixed point: Layer 2 -> defender ratings -> Layer 1 opponent adjustment ->
(optional) Layer 1 QB credit re-seeds the QB's Layer-2 prior. RAPM already does
most opponent/teammate adjustment jointly, so the loop is light here.
"""
from __future__ import annotations
import logging
from pathlib import Path
import numpy as np
import pandas as pd
import scipy.sparse as sp
from sklearn.model_selection import KFold
from sklearn.ensemble import HistGradientBoostingRegressor

from backend.grid.value import STATE_COLS

logger = logging.getLogger(__name__)

_ACCUM_PATH = Path("data/cache/rapm_accumulators.npz")


def _accum_path(key: str = "all", base_path: "Path | str | None" = None) -> Path:
    """Return the accumulator file path for the given situation key.

    key='all' resolves to ``base_path`` (default module ``_ACCUM_PATH``) so the
    overall pass is byte-for-byte identical to before.  Any other key produces a
    sibling file named ``{base_stem}_{key}.npz``.

    base_path is injectable so backtest/fold code can confine all accumulator
    writes to a scratch namespace and never touch the production ``data/cache/``.
    """
    base = Path(base_path) if base_path is not None else _ACCUM_PATH
    if key == "all":
        return base
    return base.with_name(f"{base.stem}_{key}.npz")


# --------------------------------------------------------- incremental RAPM
def init_accumulators(n_cols: int) -> tuple[np.ndarray, np.ndarray]:
    """Return zeroed (XtX, Xty) for a fresh accumulation."""
    return np.zeros((n_cols, n_cols)), np.zeros(n_cols)


def accumulate(
    XtX: np.ndarray,
    Xty: np.ndarray,
    X: sp.spmatrix,
    y: np.ndarray,
) -> tuple[np.ndarray, np.ndarray]:
    """Add one week's plays to running XtX/Xty sums."""
    week_XtX = (X.T @ X).toarray()
    week_Xty = np.asarray(X.T @ y).ravel()
    return XtX + week_XtX, Xty + week_Xty


def fit_from_accumulators(
    XtX: np.ndarray,
    Xty: np.ndarray,
    lam: float,
    prior_mean: np.ndarray,
    ridge_mask: np.ndarray,
) -> np.ndarray:
    """Solve RAPM from accumulated normal equations."""
    return _solve_ridge_prior(XtX, Xty, lam, prior_mean, ridge_mask)


def save_accumulators(
    XtX: np.ndarray,
    Xty: np.ndarray,
    week: int,
    player_order: list[str] | None = None,
    key: str = "all",
    base_path: "Path | str | None" = None,
) -> None:
    """Persist accumulators to disk.

    player_order: the sorted pid list used to build XtX/Xty columns.  Stored
    as a string array so load_accumulators can detect REORDER (same player
    count, different canonical order) and trigger reinitialisation.

    key: situation key.  'all' (default) uses the legacy _ACCUM_PATH so the
    overall pass is unchanged.  Any other key writes a sibling file.
    base_path: injectable cache root (default module _ACCUM_PATH); backtest/fold
    code passes a scratch path so writes never reach production data/cache/.
    """
    path = _accum_path(key, base_path)
    path.parent.mkdir(parents=True, exist_ok=True)
    kwargs: dict = dict(XtX=XtX, Xty=Xty, week=np.array([week]))
    if player_order is not None:
        # Store as a unicode string array (not object dtype) so the file loads
        # with pickle disabled — no arbitrary deserialization from an injectable
        # cache path.
        kwargs["player_order"] = np.array(player_order, dtype=str)
    np.savez(str(path), **kwargs)


def load_accumulators(
    key: str = "all",
    base_path: "Path | str | None" = None,
) -> tuple[np.ndarray, np.ndarray, int, list[str]] | None:
    """Load saved accumulators.

    Returns (XtX, Xty, last_week, player_order) or None.
    player_order is a list[str] of the pids in the order they were stored;
    callers should reinit if this differs from the current canonical sorted pids.

    key: situation key.  'all' (default) uses the legacy _ACCUM_PATH so the
    overall pass is unchanged.  Any other key reads a sibling file.
    base_path: injectable cache root (default module _ACCUM_PATH); backtest/fold
    code passes a scratch path so reads come from an isolated namespace.
    """
    path = _accum_path(key, base_path)
    if not path.exists():
        return None
    # allow_pickle stays False: all arrays (XtX/Xty/week floats, player_order as
    # a unicode string array) round-trip without object deserialization.
    data = np.load(str(path))
    player_order: list[str] = [str(p) for p in data["player_order"]] if "player_order" in data else []
    return data["XtX"], data["Xty"], int(data["week"][0]), player_order


# ---------------------------------------------------------------- design matrix
def _build_interaction_block(
    plays: pd.DataFrame,
    players: pd.DataFrame,
    base_nCols: int,
    min_pair_plays: int,
) -> tuple[sp.csr_matrix, dict]:
    """Build the optional WR-vs-CB interaction columns block.

    Positional approximation: pair each on-field WR with each on-field
    CB-proxy (CB if present in players, otherwise DEF as fallback).
    Prune pairs seen fewer than min_pair_plays times.

    Returns (X_int, pair_col) where:
      X_int  — sparse matrix shape (nPlays, n_pairs) with +1 entries
      pair_col — dict {(wr_id, cb_id): column_index} with indices starting
                 at base_nCols so they can be hstacked after the base block.
    """
    nPlays = len(plays)

    # Determine which position acts as the CB proxy.
    # If any player has position "CB", use CB; otherwise fall back to DEF.
    all_positions = set(players["position"].tolist()) if "position" in players.columns else set()
    cb_proxy_pos = "CB" if "CB" in all_positions else "DEF"

    wr_ids = set(players.loc[players["position"] == "WR", "player_id"].tolist()) \
        if "position" in players.columns else set()
    cb_ids = set(players.loc[players["position"] == cb_proxy_pos, "player_id"].tolist()) \
        if "position" in players.columns else set()

    if not wr_ids or not cb_ids:
        return sp.csr_matrix((nPlays, 0)), {}

    # Count co-occurrences: for each play, cross each on-field WR x each on-field CB-proxy
    pair_counts: dict = {}
    for row_off, row_def in zip(plays["off_players"], plays["def_players"]):
        play_wrs = [p for p in row_off if p in wr_ids]
        play_cbs = [p for p in row_def if p in cb_ids]
        for wr in play_wrs:
            for cb in play_cbs:
                pair_counts[(wr, cb)] = pair_counts.get((wr, cb), 0) + 1

    # Prune low-support pairs
    kept_pairs = sorted(
        (pair for pair, cnt in pair_counts.items() if cnt >= min_pair_plays),
        key=lambda x: (str(x[0]), str(x[1])),
    )
    if not kept_pairs:
        return sp.csr_matrix((nPlays, 0)), {}

    # Assign column indices starting right after the base block
    pair_col = {pair: base_nCols + i for i, pair in enumerate(kept_pairs)}
    n_pairs = len(kept_pairs)

    # Build sparse triplets for the interaction block
    int_rows: list = []
    int_cols: list = []
    for play_idx, (row_off, row_def) in enumerate(
        zip(plays["off_players"], plays["def_players"])
    ):
        play_wrs = [p for p in row_off if p in wr_ids]
        play_cbs = [p for p in row_def if p in cb_ids]
        for wr in play_wrs:
            for cb in play_cbs:
                pair = (wr, cb)
                if pair in pair_col:
                    # Column relative to the start of the interaction block
                    int_rows.append(play_idx)
                    int_cols.append(pair_col[pair] - base_nCols)

    if not int_rows:
        return sp.csr_matrix((nPlays, n_pairs)), pair_col

    X_int = sp.csr_matrix(
        (np.ones(len(int_rows), dtype=float),
         (np.array(int_rows, dtype=np.intp), np.array(int_cols, dtype=np.intp))),
        shape=(nPlays, n_pairs),
    )
    return X_int, pair_col


def build_design(
    plays: pd.DataFrame,
    players: pd.DataFrame,
    interactions: bool = False,
    min_pair_plays: int = 30,
):
    """Return (X, y, colidx). X columns: [players..., team_off..., team_def...].

    Vectorized implementation: instead of iterating row-by-row with
    itertuples, we explode the player-list columns into a flat index array
    and use numpy concatenation to build all (row, col, val) triplets in
    bulk before constructing the sparse matrix.  For typical season-sized
    play frames this is ~10x faster than the row-loop equivalent.

    interactions : bool, default False
        When False (the default), returns EXACTLY the same X, y, colidx as
        before this parameter existed — byte-for-byte identical.
        When True, appends WR-vs-CB interaction columns AFTER the base block
        [players, team_off, team_def].  Existing column indices are stable.
        colidx gains 'interactions' = {(wr_id, cb_id): col_index} and nCols
        is bumped by the number of surviving pairs.

    min_pair_plays : int, default 30
        Only used when interactions=True.  Pairs seen fewer than this many
        times are pruned (too little data for a reliable estimate).
    """
    pids = sorted(players["player_id"].tolist(), key=str)
    p_col = {p: i for i, p in enumerate(pids)}
    nP = len(pids)
    teams = sorted(players.team.unique())
    t_off = {t: nP + i for i, t in enumerate(teams)}
    t_def = {t: nP + len(teams) + i for i, t in enumerate(teams)}
    nCols = nP + 2 * len(teams)
    nPlays = len(plays)

    # --- vectorised player entries -------------------------------------------
    # plays["off_players"] / plays["def_players"] are iterables of player_ids.
    # We build a list-of-lists then explode into flat arrays:
    #   off_row_arr[k] = play index for the k-th (off, player) entry
    #   off_col_arr[k] = column index for that player
    off_lengths = plays["off_players"].apply(len).to_numpy(int)
    def_lengths = plays["def_players"].apply(len).to_numpy(int)

    play_idx = np.arange(nPlays, dtype=np.intp)

    # Repeat play indices by the per-row player counts
    off_row_arr = np.repeat(play_idx, off_lengths)
    def_row_arr = np.repeat(play_idx, def_lengths)

    # Flatten the player id lists, then map to column indices.
    # Fast path: all player IDs are known (common case). Slow path filters unknowns.
    known_pids = set(p_col)
    all_off_pids = [pid for plist in plays["off_players"] for pid in plist]
    all_def_pids = [pid for plist in plays["def_players"] for pid in plist]
    has_unknowns = any(pid not in known_pids for pid in all_off_pids) or \
                   any(pid not in known_pids for pid in all_def_pids)

    if not has_unknowns:
        off_col_arr = np.array([p_col[pid] for pid in all_off_pids], dtype=np.intp)
        def_col_arr = np.array([p_col[pid] for pid in all_def_pids], dtype=np.intp)
    else:
        _off_pairs = [(r, p_col[pid])
                      for r, plist in enumerate(plays["off_players"])
                      for pid in plist if pid in known_pids]
        _def_pairs = [(r, p_col[pid])
                      for r, plist in enumerate(plays["def_players"])
                      for pid in plist if pid in known_pids]
        _dropped = len(all_off_pids) + len(all_def_pids) - len(_off_pairs) - len(_def_pairs)
        logger.warning("build_design: dropped %d player entries with unknown IDs", _dropped)
        if _off_pairs:
            off_row_arr, off_col_arr = (np.array(x, dtype=np.intp) for x in zip(*_off_pairs))
        else:
            off_row_arr = np.array([], dtype=np.intp)
            off_col_arr = np.array([], dtype=np.intp)
        if _def_pairs:
            def_row_arr, def_col_arr = (np.array(x, dtype=np.intp) for x in zip(*_def_pairs))
        else:
            def_row_arr = np.array([], dtype=np.intp)
            def_col_arr = np.array([], dtype=np.intp)

    # --- vectorised team entries ---------------------------------------------
    # Use dict lookups so string team codes ("KC", "NE") work in addition to
    # integer codes.  to_numpy(np.intp) would fail on string team columns.
    toff_col_arr = np.array(
        [t_off[t] for t in plays["off_team"]], dtype=np.intp
    )
    tdef_col_arr = np.array(
        [t_def[t] for t in plays["def_team"]], dtype=np.intp
    )

    # --- assemble triplets ---------------------------------------------------
    row_arr = np.concatenate([off_row_arr, def_row_arr, play_idx, play_idx])
    col_arr = np.concatenate([off_col_arr, def_col_arr, toff_col_arr, tdef_col_arr])
    val_arr = np.concatenate([
        np.ones(len(off_row_arr), dtype=float),
        np.full(len(def_row_arr), -1.0),
        np.ones(nPlays, dtype=float),
        np.full(nPlays, -1.0),
    ])

    X = sp.csr_matrix((val_arr, (row_arr, col_arr)), shape=(nPlays, nCols))
    y = plays["dv"].to_numpy(float)
    colidx = dict(p_col=p_col, t_off=t_off, t_def=t_def, nP=nP, teams=teams, nCols=nCols)

    # Default path: return unchanged (byte-for-byte identical to pre-task behaviour)
    if not interactions:
        return X, y, colidx

    # Interaction path: append WR-vs-CB columns after the base block
    X_int, pair_col = _build_interaction_block(plays, players, nCols, min_pair_plays)
    n_pairs = X_int.shape[1]
    if n_pairs > 0:
        X = sp.hstack([X, X_int], format="csr")
        colidx["interactions"] = pair_col
        colidx["nCols"] = nCols + n_pairs
    else:
        colidx["interactions"] = {}

    return X, y, colidx


def _solve_ridge_prior(XtX, Xty, lam, prior_mean, ridge_mask):
    """Closed-form ridge with a prior mean: min ||y-Xb||^2 + lam||b-m||^2.
    ridge_mask scales the penalty per column (we penalize players, lightly
    penalize team intercepts)."""
    n = XtX.shape[0]
    L = lam * np.diag(ridge_mask)
    A = XtX + L
    b = Xty + L @ prior_mean
    cond = np.linalg.cond(A)
    if cond > 1e10:
        logger.warning("RAPM normal equations ill-conditioned (cond=%.2e), using lstsq fallback", cond)
        return np.linalg.lstsq(A, b, rcond=None)[0]
    return np.linalg.solve(A, b)


def run_rapm(plays, players, market_strength=None, lam=120.0, w_market=40.0,
             prior_mean_players=None, lambda_by_pos=None):
    """Solve Layer 2 (+ Layer 3 market rows). Returns (ratings_df, team_df, beta, colidx).

    lambda_by_pos: optional {position: multiplier} dict for position-specific ridge.
    e.g. {"QB": 0.8, "RB": 1.2} scales the ridge penalty per position.
    """
    X, y, colidx = build_design(plays, players)
    nCols, teams = colidx["nCols"], colidx["teams"]

    # prior mean vector
    m = np.zeros(nCols)
    if prior_mean_players is not None:
        for pid, val in prior_mean_players.items():
            m[colidx["p_col"][pid]] = val

    # penalty mask: penalize players fully (with optional per-position scaling),
    # team intercepts lightly
    mask = np.ones(nCols)
    if lambda_by_pos is not None:
        pos_lookup = players.set_index("player_id")["position"].to_dict()
        for pid, col in colidx["p_col"].items():
            pos = pos_lookup.get(pid)
            if pos and pos in lambda_by_pos:
                mask[col] = lambda_by_pos[pos]
    for t in teams:
        mask[colidx["t_off"][t]] = 0.05
        mask[colidx["t_def"][t]] = 0.05
    # Interaction columns (if present): penalize heavily so they don't dominate
    # the main player estimates.  This block is only reached when interactions
    # were passed to build_design, so the no-interactions ridge mask is unchanged.
    if "interactions" in colidx:
        for col in colidx["interactions"].values():
            mask[col] = 10.0

    XtX = (X.T @ X).toarray()
    Xty = X.T @ y

    # Layer 3: market pseudo-observations  (gamma_off[t] - gamma_def[t]) ~ market
    if market_strength is not None:
        for t in teams:
            row = np.zeros(nCols)
            row[colidx["t_off"][t]] = 1.0
            row[colidx["t_def"][t]] = 1.0   # both intercepts move the team level
            tgt = market_strength[t]
            XtX += w_market * np.outer(row, row)
            Xty += w_market * row * tgt

    beta = _solve_ridge_prior(XtX, Xty, lam, m, mask)

    # assemble player ratings
    inv_pcol = {v: k for k, v in colidx["p_col"].items()}
    # Merge whatever player meta is available.  Synthetic players carry
    # is_starter/ability (ground truth); real rosters do not — select only the
    # columns present so run_rapm works on real data, not just synth.
    meta_cols = [c for c in ("player_id", "team", "position", "is_starter", "ability")
                 if c in players.columns]
    ratings = pd.DataFrame({
        "player_id": [inv_pcol[i] for i in range(colidx["nP"])],
        "rating": beta[:colidx["nP"]],
    }).merge(players[meta_cols], on="player_id", how="left")

    team_df = pd.DataFrame({
        "team": teams,
        "team_rating": [beta[colidx["t_off"][t]] - beta[colidx["t_def"][t]] for t in teams],
    })
    return ratings, team_df, beta, colidx


# ------------------------------------------------- per-situation RAPM
def run_situation_rapm(plays, players, situations=None, min_plays=200, **kw):
    """Solve a separate RAPM for each situational slice of plays.

    Thin orchestration over ``run_rapm``: for every boolean mask in
    *situations* (defaulting to ``backend.grid.situations.classify(plays)``),
    slice the plays frame and call ``run_rapm`` on the sub-frame unchanged.
    ``build_design`` rebuilds a self-consistent colidx on each sub-frame, so
    players absent from a situation get a ridge-prior 0 — expected behaviour
    that callers should label "no data" rather than a real zero grade.

    Parameters
    ----------
    plays:
        Plays DataFrame.  **Must** contain ``off_players`` and ``def_players``
        columns.  If they are absent a ``ValueError`` is raised immediately
        (the real nflverse loader does not yet emit them).
    players:
        Players DataFrame passed through to ``run_rapm`` unchanged.
    situations:
        Optional ``dict[str, np.ndarray[bool]]`` of pre-computed masks.
        When ``None`` (default), ``backend.grid.situations.classify(plays)``
        is called to produce the standard red_zone / passing_downs /
        rushing_downs (and optionally two_minute) masks.
    min_plays:
        Minimum number of plays required for a situation to be solved.
        Situations with fewer rows are logged at INFO level and skipped.
    **kw:
        Forwarded verbatim to ``run_rapm`` (e.g. ``lam``, ``market_strength``).

    Returns
    -------
    dict[str, pd.DataFrame]
        Mapping of situation name -> player ratings DataFrame for every
        situation that met the ``min_plays`` threshold.
    """
    from backend.grid import situations as S

    if "off_players" not in plays.columns or "def_players" not in plays.columns:
        raise ValueError(
            "situation RAPM requires participation columns (off_players/def_players); "
            "real nflverse path does not yet emit them (load_participation is a stub)"
        )

    masks = situations if situations is not None else S.classify(plays)
    out: dict = {}
    for name, m in masks.items():
        sub = plays.loc[m]
        if len(sub) < min_plays:
            logger.info("skip %s: %d < %d plays", name, len(sub), min_plays)
            continue
        ratings, _team_df, _beta, _colidx = run_rapm(sub, players, **kw)
        out[name] = ratings
    return out


# ------------------------------------------------- Layer 1: cross-fitted credit
def _onfield_rating_sums(plays, ratings_lookup):
    """Per-play sum of current ratings for on-field offense and defense."""
    off_sum = np.zeros(len(plays)); def_sum = np.zeros(len(plays))
    for r, play in enumerate(plays.itertuples(index=False)):
        off_sum[r] = sum(ratings_lookup.get(p, 0.0) for p in play.off_players)
        def_sum[r] = sum(ratings_lookup.get(p, 0.0) for p in play.def_players)
    return off_sum, def_sum


def _cross_fitted_context_residual(plays, ratings_lookup, n_splits=5, seed=0):
    """Out-of-fold residual dv - g(state, opp_def_rating), aligned to plays' rows.

    g is position-agnostic (conditions on situational state + aggregate
    on-field opponent-defense rating only, never who is on offense), so this is
    computed ONCE per plays frame and shared across every player's weekly
    aggregation -- refitting per player would multiply an already-nontrivial
    5-fold GBM cross-fit by the player count: tolerable at QB scale (~32
    teams), prohibitive at RB/WR/TE scale (hundreds of players).
    """
    _, def_sum = _onfield_rating_sums(plays, ratings_lookup)
    feats = np.column_stack([plays[STATE_COLS].to_numpy(float), def_sum])
    y = plays["dv"].to_numpy(float)

    oof = np.zeros(len(plays))
    kf = KFold(n_splits=n_splits, shuffle=True, random_state=seed)
    for tr, te in kf.split(feats):
        g = HistGradientBoostingRegressor(max_depth=3, learning_rate=0.1,
                                          max_iter=200, min_samples_leaf=150,
                                          random_state=seed)
        g.fit(feats[tr], y[tr])
        oof[te] = g.predict(feats[te])
    return y - oof


def layer1_qb_weekly(plays, ratings_lookup, focus_qb, n_splits=5, seed=0):
    """Cross-fitted, opponent-adjusted weekly credit for the focus QB.

    Context model g(state, opp_def_rating) is trained out-of-fold on ALL
    offensive plays; the residual dv - g_oof is the value above what state +
    opponent strength explain. The focus QB's weekly credit = mean residual on
    his on-field offensive snaps; snaps -> measurement precision for the Kalman.
    """
    off = plays.copy()
    off = off.assign(_resid=_cross_fitted_context_residual(off, ratings_lookup, n_splits, seed))

    mask = off.off_players.apply(lambda t: focus_qb in t)
    qb = off[mask]
    wk = qb.groupby("week").agg(qb_credit=("_resid", "mean"),
                                snaps=("_resid", "size")).reset_index()
    return wk


# -------------------------------------------- Layer 1: multi-QB attribution
def layer1_all_qbs(plays, ratings_lookup, qb_ids, n_splits=5, seed=0):
    """Run layer1_qb_weekly for every QB in qb_ids.

    Returns a dict {qb_id: weekly_credit_df} with per-QB weekly credit DataFrames.
    QBs with fewer than 20 plays are skipped (insufficient data for cross-fitting).
    """
    results = {}
    for qb in qb_ids:
        mask = plays.off_players.apply(lambda t: qb in t)
        if mask.sum() < 20:
            continue
        try:
            wk = layer1_qb_weekly(plays, ratings_lookup, qb, n_splits=n_splits, seed=seed)
            results[qb] = wk
        except Exception:
            pass
    return results


# -------------------------------------- Layer 1: any-position attribution (Phase 2b)
def layer1_all_players(plays, ratings_lookup, player_ids, n_splits=5, seed=0, min_plays=20):
    """Cross-fitted, opponent-adjusted weekly credit for every player in player_ids.

    Generalizes layer1_qb_weekly beyond QB (roadmap Phase 2b §4.5): the
    expensive context-model cross-fit (`_cross_fitted_context_residual`) runs
    ONCE over all offensive plays, then every player's weekly credit is a cheap
    mask + groupby on the shared residual -- tractable regardless of position
    or player count, unlike calling layer1_qb_weekly (which refits per call) in
    a loop. Players with fewer than min_plays on-field snaps are skipped
    (insufficient data for a meaningful weekly mean).

    Returns {player_id: weekly_credit_df} with columns [week, credit, snaps]
    ("credit" rather than "qb_credit" -- this spans every position).
    """
    off = plays.copy()
    off = off.assign(_resid=_cross_fitted_context_residual(off, ratings_lookup, n_splits, seed))

    results = {}
    for pid in player_ids:
        mask = off.off_players.apply(lambda t, pid=pid: pid in t)
        if mask.sum() < min_plays:
            continue
        sub = off[mask]
        wk = sub.groupby("week").agg(credit=("_resid", "mean"),
                                     snaps=("_resid", "size")).reset_index()
        results[pid] = wk
    return results


# ----------------------------------------------------------------- fixed point
def fit(plays, players, market_strength, focus_qb, n_iter=3,
        lam=120.0, w_market=40.0, verbose=True, lambda_by_pos=None):
    """Run the coordinate-ascent loop and return everything downstream needs."""
    prior = None
    ratings = None
    for it in range(n_iter):
        ratings, team_df, beta, colidx = run_rapm(
            plays, players, market_strength=market_strength,
            lam=lam, w_market=w_market, prior_mean_players=prior,
            lambda_by_pos=lambda_by_pos)
        rlook = ratings.set_index("player_id").rating.to_dict()
        # Layer 1 QB credit using updated defender ratings (opponent adjustment)
        qb_weekly = layer1_qb_weekly(plays, rlook, focus_qb)
        # re-seed QB prior with his season Layer-1 credit (light coupling)
        prior = {focus_qb: float(qb_weekly.qb_credit.mean())}
        if verbose:
            corr = np.corrcoef(ratings.rating, ratings.ability)[0, 1]
            print(f"  iter {it}: corr(rating, true ability) = {corr:.3f}")
    return dict(ratings=ratings, team_df=team_df, qb_weekly=qb_weekly,
                beta=beta, colidx=colidx)


if __name__ == "__main__":
    from synth import simulate
    from value import fit_value_model, attach_dv
    plays, players, gt = simulate()
    plays = attach_dv(plays, fit_value_model(plays))
    # synthetic market = planted team strength + noise (a "closing line")
    rng = np.random.default_rng(1)
    market = {t: v + rng.normal(0, 0.01) for t, v in gt["team_strength"].items()}
    res = fit(plays, players, market, gt["focus_qb"], n_iter=3)
    r = res["ratings"]
    for pos in ["QB", "RB", "WR", "TE", "DEF"]:
        sub = r[r.position == pos]
        print("  %-4s corr(rating,truth)=%.3f n=%d" %
              (pos, np.corrcoef(sub.rating, sub.ability)[0, 1], len(sub)))
    tm = res["team_df"].merge(
        pd.DataFrame({"team": list(gt["team_strength"]),
                      "true": list(gt["team_strength"].values())}), on="team")
    print("  team rating vs truth corr=%.3f" %
          np.corrcoef(tm.team_rating, tm.true)[0, 1])
