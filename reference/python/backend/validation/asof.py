"""As-of accessor + injectable cache namespace — the leakage choke point.

Roadmap Phase 2a; validation plan §8 ("leakage guards — three axes, not one").
Every backtest stage gets its inputs through an :class:`AsOf` so the harness is
**leakage-safe by construction** instead of by spot-checking. The three leakage
axes §8 names, and how this module closes each:

* **Temporal** (future rows reach a stage) — :meth:`AsOf.slice_plays` keeps only
  outcomes through ``week-1``; :class:`TripwireFrame` raises if a future row is
  ever read downstream, localizing the leak to its use site.
* **Scope** (a global fit spanning time leaks into a local prediction) — the
  as-of pool / market / intervention slices give each stage only the rows it may
  see, so a global RAPM/VOR/market fit can't smuggle the future in.
* **State** (persistent caches are a global singleton) — :class:`CacheNamespace`
  confines the engine's accumulator + Kalman writes to an injected scratch dir,
  so a fold never reads another fold's cache and a backtest never overwrites the
  user's real ``data/cache/`` state.

Information-time model (§8): the cutoff is "available **before kickoff** of W."
*Outcomes* (drive results, realized points) are known only through ``W-1``;
*pre-game* signals (Vegas lines, injury/depth reports, a known regime change for
W) are available *for* W. So outcome slices use ``week ≤ W-1`` and pre-game
slices use ``week ≤ W``.
"""
from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import pandas as pd

# Production cache locations the backtest must never touch (mirror the engine
# module defaults in layers.py / statespace.py).
_PRODUCTION_CACHE_DIR = Path("data/cache")


class LeakageError(AssertionError):
    """Raised when future-dated data reaches a stage that must not see it."""


def _asof_keep_mask(frame, season, cutoff, week_col="week", season_col="season"):
    """Boolean keep-mask for rows known as of (season, cutoff-week).

    With a ``season`` column present the cutoff is the *tuple* (season, week):
    keep all prior seasons in full, plus the current season through ``cutoff``.
    This is what makes a multi-season frame safe — a week-only filter would admit
    future-season rows with low week numbers and drop prior-season rows with high
    week numbers that were already known.  Without a season column it falls back
    to week-only (single-season frames).
    """
    if season_col in frame.columns:
        return ((frame[season_col] < season)
                | ((frame[season_col] == season) & (frame[week_col] <= cutoff)))
    return frame[week_col] <= cutoff


# --------------------------------------------------------------------------- #
# The as-of accessor                                                          #
# --------------------------------------------------------------------------- #
@dataclass(frozen=True)
class AsOf:
    """Information state as of the kickoff of (``season``, ``week``).

    ``week`` is the 1-indexed week being forecast.  ``mode`` is ``"backtest"``
    (the offline walk-forward) or ``"production"`` (the live forward path); it is
    carried for callers/guards to branch on, not enforced here.
    """

    season: int
    week: int
    mode: str = "backtest"

    @property
    def outcome_cutoff(self) -> int:
        """Last week whose *outcomes* are known before kickoff of ``week`` (= W-1)."""
        return self.week - 1

    @property
    def pregame_cutoff(self) -> int:
        """Last week whose *pre-game* signals are available for ``week`` (= W)."""
        return self.week

    # -- slices -------------------------------------------------------------- #
    def slice_plays(self, plays: pd.DataFrame, week_col: str = "week") -> pd.DataFrame:
        """Outcome rows usable to forecast W: ``(season, week) ≤ (season, W-1)``.

        Season-aware (see :func:`_asof_keep_mask`): on a multi-season frame this
        keeps prior seasons in full and the current season only through ``W-1``.
        """
        return plays[_asof_keep_mask(plays, self.season, self.outcome_cutoff, week_col)].copy()

    def slice_pool(
        self,
        plays: pd.DataFrame,
        participation_cols: tuple[str, ...] = ("off_players", "def_players"),
        week_col: str = "week",
    ) -> set:
        """The as-of player universe: everyone who has appeared through ``W-1``.

        Built from the participation columns of the sliced plays so the design
        matrix / Kalman roster never reveal a later call-up (§8 player-universe
        leakage).  Flattens list/tuple participation cells.
        """
        sub = self.slice_plays(plays, week_col=week_col)
        pool: set = set()
        for col in participation_cols:
            if col not in sub.columns:
                continue
            for cell in sub[col]:
                if isinstance(cell, (list, tuple, set)):
                    pool.update(cell)
                elif cell is not None and not (isinstance(cell, float) and pd.isna(cell)):
                    pool.add(cell)
        return pool

    def slice_market(self, market, week_col: str = "week"):
        """Market strength as published *before* W (a pre-game signal).

        A plain dict (the synth/data-adapter ``{team: strength}`` prior-season
        anchor) passes through unchanged.  A per-week table is sliced to
        ``week ≤ W`` — the line for W is published before kickoff, so it is
        usable, but later weeks are not.
        """
        if isinstance(market, pd.DataFrame):
            return market[_asof_keep_mask(market, self.season, self.pregame_cutoff, week_col)].copy()
        return market

    def slice_interventions(self, interventions, week_col: str = "week"):
        """Regime changes *known as of* W (a pre-game signal): ``week ≤ W``.

        Prevents intervention foreknowledge (§8) — handing a week-5 forecast the
        full-season set would reveal a week-9 injury.  Accepts a DataFrame (sliced
        by ``week_col``) or an iterable of records each carrying ``week_col``.
        """
        if isinstance(interventions, pd.DataFrame):
            return interventions[
                _asof_keep_mask(interventions, self.season, self.pregame_cutoff, week_col)].copy()
        kept = []
        for iv in interventions or []:
            wk = iv.get(week_col) if isinstance(iv, dict) else getattr(iv, week_col, None)
            if wk is None:
                raise LeakageError(
                    f"intervention missing required {week_col!r} — undated regime "
                    f"changes would be visible to every fold")
            if wk <= self.pregame_cutoff:
                kept.append(iv)
        return kept

    # -- guards -------------------------------------------------------------- #
    def assert_watermark(self, last_week: int) -> None:
        """Assert an accumulator's high-water week never exceeds ``W-1`` (§8).

        ``last_week`` is the engine accumulator's ``last_week`` field after a
        fold; if it has absorbed week ≥ W, future outcomes leaked into the fit.
        """
        if last_week > self.outcome_cutoff:
            raise LeakageError(
                f"watermark {last_week} > {self.outcome_cutoff} "
                f"(future outcomes leaked into the week-{self.week} fit)")

    def tripwire(self, frame: pd.DataFrame, week_col: str = "week",
                 source: str = "plays") -> "TripwireFrame":
        """Wrap an outcome frame so any future-row read raises (localizes leaks)."""
        return TripwireFrame(frame, self.outcome_cutoff, season=self.season,
                             week_col=week_col, source=source)


# --------------------------------------------------------------------------- #
# Injectable cache namespace (state-axis isolation)                            #
# --------------------------------------------------------------------------- #
@dataclass
class CacheNamespace:
    """A scratch cache root for one fold/backtest — confines engine state writes.

    Pass :attr:`accum_base_path` as ``base_path`` to
    ``layers.save/load_accumulators`` and :attr:`kalman_path` as ``path`` to
    ``KalmanState.save/load``.  Because every persistent write goes here, a fold
    never contaminates another fold and the backtest never overwrites the user's
    real ``data/cache/`` (the production-overwrite hazard in §8).
    """

    root: Path

    def __post_init__(self) -> None:
        self.root = Path(self.root)
        self.assert_isolated()       # fail before any filesystem side effect
        self.root.mkdir(parents=True, exist_ok=True)

    @property
    def accum_base_path(self) -> Path:
        """File path handed to the RAPM accumulator save/load (``base_path``)."""
        return self.root / "rapm_accumulators.npz"

    @property
    def kalman_path(self) -> Path:
        """File path handed to ``KalmanState.save/load``."""
        return self.root / "kalman_state.npz"

    def is_isolated(self) -> bool:
        """True iff this namespace is not the production ``data/cache/`` dir."""
        root = self.root.resolve()
        prod = _PRODUCTION_CACHE_DIR.resolve()
        return root != prod and prod not in root.parents and root not in prod.parents

    def assert_isolated(self) -> None:
        """Raise unless the namespace is confined away from production cache."""
        if not self.is_isolated():
            raise LeakageError(
                f"cache namespace {self.root} overlaps production "
                f"{_PRODUCTION_CACHE_DIR} — a backtest would clobber real state")


# --------------------------------------------------------------------------- #
# Tripwire proxy (temporal-axis localization)                                 #
# --------------------------------------------------------------------------- #
class TripwireFrame:
    """A thin DataFrame proxy that raises if it ever exposes a future row.

    The frame should already be sliced to ``week ≤ cutoff``; the proxy re-checks
    that invariant at construction and on every read, so a mis-slice surfaces as
    a :class:`LeakageError` at the *use* site (stack-trace localization, §8)
    rather than as silently-wrong numbers downstream.
    """

    def __init__(self, df: pd.DataFrame, cutoff: int, season: int | None = None,
                 week_col: str = "week", source: str = "plays") -> None:
        object.__setattr__(self, "_df", df)
        object.__setattr__(self, "_cutoff", cutoff)
        object.__setattr__(self, "_season", season)
        object.__setattr__(self, "_week_col", week_col)
        object.__setattr__(self, "_source", source)
        self._check()

    def _check(self) -> None:
        df, wc = self._df, self._week_col
        if wc in df.columns:
            # season-aware when both a season and a season column are present,
            # else week-only — mirrors AsOf's slicing so the proxy can't be
            # fooled by a future-season row with a low week number.
            if self._season is not None:
                keep = _asof_keep_mask(df, self._season, self._cutoff, wc)
            else:
                keep = df[wc] <= self._cutoff
            n_future = int((~keep).sum())
            if n_future:
                raise LeakageError(
                    f"{self._source}: {n_future} row(s) past the as-of cutoff "
                    f"(season≤{self._season}, {wc}≤{self._cutoff}) — future leak")

    @property
    def frame(self) -> pd.DataFrame:
        """The underlying frame, re-validated."""
        self._check()
        return self._df

    def __getattr__(self, name):
        # only reached for names not set on the proxy itself
        self._check()
        return getattr(self._df, name)

    def __getitem__(self, key):
        self._check()
        return self._df[key]

    def __len__(self) -> int:
        self._check()
        return len(self._df)
