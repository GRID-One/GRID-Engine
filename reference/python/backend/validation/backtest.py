"""Rolling-origin walk-forward driver (roadmap Phase 2a; validation plan §11, §11.5).

At each historical origin (season, week W) the driver produces GRID's forecast
using only information available *before kickoff of W*, so it can later be scored
against what actually happened (Tier 1, Phase 2c).  It is leakage-safe by
construction — every training row comes from before W — and **fast by design**,
the two performance requirements §7.5/§11.5 make non-negotiable:

* **Frozen pre-period V(s).** The situational-value GBM is fit **once** on a
  reserved warm-up pre-period and reused at every origin.  Refitting V(s) per
  origin would mean ~hundreds of GBM fits (hours); V is slow-moving, so freezing
  it is both defensible and the difference between minutes and hours.
* **Incremental RAPM accumulators.** Each week's plays are added to running
  normal-equation accumulators (``accumulate``) exactly once; every origin solves
  from the current sums (``fit_from_accumulators``) instead of rebuilding the
  design over all prior weeks.  Because ``build_design`` derives its columns from
  the fixed ``players`` frame (not the per-week plays), the column layout is
  stable across weeks and the per-week contributions are simply additive — which
  is also why the incremental solve equals a from-scratch fit (the two-path
  equivalence the leakage suite asserts).

Accumulators are held in memory for the duration of one walk, so a backtest never
reads or writes the production ``data/cache/`` at all (the state-axis isolation
§8 wants — see :class:`~backend.validation.asof.CacheNamespace` for the on-disk
case the two-path guard exercises against ``weekly_update``).
"""
from __future__ import annotations

from dataclasses import dataclass, field

import numpy as np
import pandas as pd

from backend.grid.layers import accumulate, build_design, fit_from_accumulators, init_accumulators
from backend.grid.value import attach_dv, fit_value_model
from backend.validation.asof import AsOf

# Defaults mirror run_rapm so the backtest's RAPM matches the production solve.
_DEFAULT_LAM = 120.0
_DEFAULT_W_MARKET = 40.0
_TEAM_RIDGE = 0.05   # lightly penalize team intercepts (matches weekly_update)


@dataclass
class OriginResult:
    """One walk-forward origin: GRID's as-of-W player ratings + provenance.

    ``def_grades`` carries the sign-flipped team-defence intercepts from the
    same solve (``-beta[t_def]``, matching the production convention in
    ``weekly_update._upsert_matchup_grades``: higher = tougher matchup) so the
    Tier-2 matchup check can score the walk-forward output directly.
    """

    season: int
    week: int
    n_train_plays: int
    ratings: dict = field(default_factory=dict)     # player_id -> RAPM rating
    def_grades: dict = field(default_factory=dict)  # def_team -> matchup grade


def enumerate_origins(
    plays: pd.DataFrame,
    *,
    warmup_weeks: int = 4,
    season_col: str = "season",
    week_col: str = "week",
) -> list[tuple[int, int]]:
    """The (season, week) slots to forecast: every slot after the warm-up pre-period.

    The earliest ``warmup_weeks`` slots are reserved to fit V(s) and seed the RAPM
    accumulators, so the first forecast has real training history behind it.
    """
    slots = _ordered_slots(plays, season_col, week_col)
    return slots[warmup_weeks:]


def _ordered_slots(plays, season_col, week_col) -> list[tuple[int, int]]:
    """All distinct (season, week) slots in chronological order."""
    pairs = plays[[season_col, week_col]].drop_duplicates()
    pairs = pairs.sort_values([season_col, week_col])
    return [(int(s), int(w)) for s, w in pairs.itertuples(index=False)]


def solve_rapm(XtX, Xty, colidx, *, lam=_DEFAULT_LAM, w_market=_DEFAULT_W_MARKET,
               market=None) -> np.ndarray:
    """Solve RAPM from accumulated normal equations (matches the production solve).

    Players are ridged toward 0 at strength ``lam``; team intercepts are lightly
    penalized; an optional ``market`` (a static ``{team: strength}`` mapping)
    anchors each *listed* team's (off+def) intercept toward its strength with
    weight ``w_market``.  Teams absent from ``market`` are left to the normal team
    ridge — they are **not** anchored toward zero.
    """
    n_cols = colidx["nCols"]
    prior_mean = np.zeros(n_cols)
    mask = np.ones(n_cols)
    for t in colidx["teams"]:
        mask[colidx["t_off"][t]] = _TEAM_RIDGE
        mask[colidx["t_def"][t]] = _TEAM_RIDGE

    if market:
        XtX = XtX.copy()
        Xty = Xty.copy()
        for t in colidx["teams"]:
            if t not in market:
                continue   # only anchor teams with an explicit line; no false zero prior
            row = np.zeros(n_cols)
            row[colidx["t_off"][t]] = 1.0
            row[colidx["t_def"][t]] = 1.0
            tgt = market[t]
            XtX += w_market * np.outer(row, row)
            Xty += w_market * row * tgt

    return fit_from_accumulators(XtX, Xty, lam, prior_mean, mask)


def walk_forward(
    plays: pd.DataFrame,
    players: pd.DataFrame,
    *,
    warmup_weeks: int = 4,
    lam: float = _DEFAULT_LAM,
    w_market: float = _DEFAULT_W_MARKET,
    market=None,
    origin_stride: int = 1,
    season_col: str = "season",
    week_col: str = "week",
) -> list[OriginResult]:
    """Run the leakage-safe, incremental rolling-origin walk-forward.

    Freezes V(s) on the warm-up pre-period, then advances week by week: at each
    origin it solves RAPM from the accumulators (which hold only weeks strictly
    before the origin) and records the as-of player ratings, *then* folds the
    origin week into the accumulators for the next origin.

    ``market`` is a **static** ``{team: strength}`` anchor (the prior-season /
    closing-line-implied strength from the ``data_adapters`` contract), applied
    unchanged at every origin — it is not a per-week table, so it needs no as-of
    slicing.  A future per-week market-line feed would instead be sliced per
    origin via :meth:`AsOf.slice_market` before the solve; that is out of scope
    here.

    ``origin_stride`` > 1 forecasts every k-th origin only (fast dev iteration);
    the number of skipped origins is logged, never silently dropped.  Returns one
    :class:`OriginResult` per forecast origin, in chronological order.
    """
    if warmup_weeks <= 0:
        raise ValueError(f"warmup_weeks must be > 0 (a pre-period is needed to "
                         f"freeze V(s)); got {warmup_weeks}")
    if origin_stride <= 0:
        raise ValueError(f"origin_stride must be > 0; got {origin_stride}")
    slots = _ordered_slots(plays, season_col, week_col)
    if len(slots) <= warmup_weeks:
        raise ValueError(
            f"need more than warmup_weeks={warmup_weeks} distinct (season, week) "
            f"slots to have any forecast origin; got {len(slots)}")

    origin_slots = slots[warmup_weeks:]
    if origin_stride > 1:
        kept = origin_slots[::origin_stride]
        _log_subset(len(origin_slots), len(kept), origin_stride)
        origin_slots = kept
    origin_set = set(origin_slots)

    # --- freeze V(s) once on the warm-up pre-period ------------------------- #
    pre_slots = set(slots[:warmup_weeks])
    pre_mask = _slot_mask(plays, pre_slots, season_col, week_col)
    vmodel = fit_value_model(plays[pre_mask])

    # --- advance week by week, accumulating incrementally ------------------- #
    acc_XtX = acc_Xty = None
    colidx = None
    n_train_plays = 0
    last_slot: tuple[int, int] | None = None
    results: list[OriginResult] = []

    for (s, w) in slots:
        if (s, w) in origin_set and acc_XtX is not None:
            # defense-in-depth: accumulators must hold only weeks before this one
            if last_slot is not None and last_slot[0] == s:
                AsOf(season=s, week=w).assert_watermark(last_slot[1])
            beta = solve_rapm(acc_XtX, acc_Xty, colidx,
                              lam=lam, w_market=w_market, market=market)
            ratings = {pid: float(beta[i]) for pid, i in colidx["p_col"].items()}
            def_grades = {t: -float(beta[c])
                          for t, c in colidx.get("t_def", {}).items()}
            results.append(OriginResult(season=s, week=w,
                                        n_train_plays=n_train_plays,
                                        ratings=ratings, def_grades=def_grades))

        wk = plays[_slot_mask(plays, {(s, w)}, season_col, week_col)].copy()
        wk = attach_dv(wk, vmodel)
        X, y, cidx = build_design(wk, players)
        if colidx is None:
            colidx = cidx
            acc_XtX, acc_Xty = init_accumulators(cidx["nCols"])
        acc_XtX, acc_Xty = accumulate(acc_XtX, acc_Xty, X, y)
        n_train_plays += len(wk)
        last_slot = (s, w)

    return results


def _slot_mask(plays, slots: set, season_col: str, week_col: str) -> pd.Series:
    """Boolean mask selecting rows whose (season, week) is in ``slots``."""
    keys = list(zip(plays[season_col], plays[week_col], strict=True))
    return pd.Series([k in slots for k in keys], index=plays.index)


def _log_subset(total: int, kept: int, stride: int) -> None:
    """Announce subset sampling so dropped origins are never silent (§11.5)."""
    from backend.pipeline._logging import get_logger
    get_logger(__name__).info(
        "walk_forward subset mode: stride=%d -> %d/%d origins (%d skipped)",
        stride, kept, total, total - kept)
