"""Multi-season GRID `plays`-contract ingest + frozen snapshot (roadmap Phase 1).

Materializes the real-data GRID plays contract (via
``nflverse_adapter.load_grid_plays``) for a set of seasons and freezes it to a
reproducible Parquet snapshot, plus a manifest of per-season row counts and
participation coverage (the roadmap's "frozen snapshot" + baseline-measurement
done-when).  The snapshot is what the Phase-2a walk-forward backtest reads, so it
must be reproducible and offline-rebuildable: ``load_grid_plays`` keeps the raw
nflverse parquet cached, so re-running rebuilds the snapshot without re-download.

CLI:
    python -m backend.pipeline.ingest_grid --years 2019 2020 2021 2022 2023 2024
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

from backend.pipeline._logging import get_logger
from backend.grid.nflverse_adapter import load_grid_plays

logger = get_logger(__name__)

DEFAULT_YEARS = [2019, 2020, 2021, 2022, 2023, 2024]
SNAPSHOT_DIR = "data/snapshots"


def _season_stats(plays) -> dict:
    """Row counts + participation coverage for one season's plays contract."""
    n = len(plays)
    off_cov = float(plays["off_players"].apply(len).gt(0).mean()) if n else 0.0
    def_cov = float(plays["def_players"].apply(len).gt(0).mean()) if n else 0.0
    return {
        "n_plays": int(n),
        "n_drives": int(plays["drive_id"].nunique()) if n else 0,
        "off_coverage": round(off_cov, 4),
        "def_coverage": round(def_cov, 4),
    }


def ingest(
    years: list[int] | None = None,
    snapshot_dir: str = SNAPSHOT_DIR,
    cache_dir: str = "data/cache",
    loader=load_grid_plays,
) -> dict:
    """Freeze a GRID plays snapshot for each season + write a manifest.

    Writes ``{snapshot_dir}/grid_plays_{year}.parquet`` per season and a
    ``manifest.json`` of per-season stats.  Idempotent (overwrites in place).
    ``loader`` is injectable so tests can run without network.  Returns the
    manifest dict.
    """
    if years is None:
        years = DEFAULT_YEARS            # None -> defaults; [] stays empty (no-op)
    if len(set(years)) != len(years):
        raise ValueError(f"duplicate seasons in years={years!r}")
    out_dir = Path(snapshot_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    per_season: dict[str, dict] = {}
    for year in years:
        logger.info("Ingesting GRID plays for season %d", year)
        plays = loader([year], cache_dir=cache_dir)
        path = out_dir / f"grid_plays_{year}.parquet"
        plays.to_parquet(str(path), index=False)
        stats = _season_stats(plays)
        per_season[str(year)] = stats
        logger.info(
            "season %d: %d plays, %d drives, participation off=%.3f def=%.3f -> %s",
            year, stats["n_plays"], stats["n_drives"],
            stats["off_coverage"], stats["def_coverage"], path,
        )

    totals = {
        "n_plays": sum(s["n_plays"] for s in per_season.values()),
        "n_drives": sum(s["n_drives"] for s in per_season.values()),
    }
    manifest = {"years": list(years), "per_season": per_season, "totals": totals}
    (out_dir / "manifest.json").write_text(json.dumps(manifest, indent=2))
    logger.info("=== ingest_grid DONE  %d seasons, %d plays ===",
                len(years), totals["n_plays"])
    return manifest


def _parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    """Parse the ingest CLI arguments."""
    parser = argparse.ArgumentParser(
        description="Ingest the GRID plays contract for seasons and freeze a snapshot")
    parser.add_argument("--years", nargs="+", type=int, default=DEFAULT_YEARS,
                        help="NFL seasons to ingest (default: 2019-2024)")
    parser.add_argument("--snapshot-dir", default=SNAPSHOT_DIR,
                        help="Directory for the frozen snapshot (default: data/snapshots)")
    parser.add_argument("--cache-dir", default="data/cache",
                        help="Directory for the parquet cache (default: data/cache)")
    return parser.parse_args(argv)


if __name__ == "__main__":
    args = _parse_args()
    ingest(years=args.years, snapshot_dir=args.snapshot_dir, cache_dir=args.cache_dir)
