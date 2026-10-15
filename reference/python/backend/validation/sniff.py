"""Known-player rank-band sniff test (roadmap Phase 1 / cross-cutting #9).

Replaces the eyeball "do the elites rank near the top?" check with an automatable
soft gate: given a ranking (the VOR/`calculate_vor` output or any [id, rank]
list) and a small hand-curated set of consensus-elite players, assert that
enough of them land in the top band and none fall outside a sane ceiling.  It
catches GROSS projection/scoring/ranking breakage (a units bug, a sign flip, an
empty join) without asserting a precise order.

The checker is generic and data-source-agnostic; the curated ELITE registry that
makes it meaningful on real data is filled once the GRID projection ships
(Phase 2b) — until then callers pass their own elite set (e.g. planted-truth top
players on synth).
"""
from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass, field

import pandas as pd


@dataclass
class SniffResult:
    """Outcome of a rank-band check: pass/fail + human-readable failures."""
    passed: bool
    failures: list[str] = field(default_factory=list)
    found: int = 0
    in_top_k: int = 0
    missing: list = field(default_factory=list)


def _rank_lookup(rankings, id_key: str, rank_key: str) -> dict:
    """Build {player_id: rank} from dict rows, 2-item [id, rank] rows, or a DataFrame."""
    if isinstance(rankings, pd.DataFrame):
        return dict(zip(rankings[id_key].astype(str), rankings[rank_key], strict=True))
    lookup = {}
    for row in rankings:
        if isinstance(row, Mapping):
            player_id, rank = row[id_key], row[rank_key]
        else:
            player_id, rank = row     # a 2-item [id, rank] sequence
        lookup[str(player_id)] = rank
    return lookup


def check_rank_bands(
    rankings,
    elites,
    *,
    top_k: int = 5,
    min_in_top_k: int = 3,
    max_rank: int | None = 24,
    id_key: str = "player_id",
    rank_key: str = "rank",
) -> SniffResult:
    """Assert consensus-elite players occupy a sane rank band.

    Two checks (the roadmap #9 shape — "≥3 of {set} in top-5; none outside top-24"):
      * at least ``min_in_top_k`` of the elite set rank within ``top_k``;
      * no elite present ranks worse than ``max_rank`` (skipped if max_rank None).
    Elites absent from the ranking are reported (and count against the top-k bar).

    ``rankings`` may be the list[dict] from ``calculate_vor`` or a DataFrame; ranks
    are 1-indexed (rank 1 = best).  Returns a :class:`SniffResult` (never raises on
    a band miss — the caller decides whether to gate).
    """
    ranks = _rank_lookup(rankings, id_key, rank_key)
    elites = [str(e) for e in elites]

    present = [e for e in elites if e in ranks]
    missing = [e for e in elites if e not in ranks]
    in_top = [e for e in present if ranks[e] <= top_k]
    outside = [e for e in present if max_rank is not None and ranks[e] > max_rank]

    failures: list[str] = []
    if len(in_top) < min_in_top_k:
        failures.append(
            f"only {len(in_top)}/{len(elites)} elite players in top {top_k} "
            f"(need {min_in_top_k}); ranks={{{', '.join(f'{e}:{ranks[e]}' for e in present)}}}")
    if outside:
        failures.append(
            f"elite players ranked worse than {max_rank}: "
            + ", ".join(f"{e}:{ranks[e]}" for e in outside))
    if missing:
        failures.append(f"elite players absent from ranking: {missing}")

    return SniffResult(passed=not failures, failures=failures,
                       found=len(present), in_top_k=len(in_top), missing=missing)
