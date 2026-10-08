"""Tests for the multi-season GRID ingest + frozen snapshot (Phase 1).

Network-free: the GRID plays loader is injected with a small in-memory contract,
so the test exercises snapshot/manifest writing + stats without any download.
"""
import json

import numpy as np
import pandas as pd
import pytest

from backend.pipeline.ingest_grid import ingest, _season_stats


def _fake_contract(n=6):
    """A tiny GRID plays-contract frame with populated participation."""
    return pd.DataFrame({
        "play_id": range(n),
        "drive_id": ["G_1"] * (n // 2) + ["G_2"] * (n - n // 2),
        "week": [1] * n,
        "off_players": [("QB1", "WR1")] * n,
        "def_players": [("CB1", "LB1")] * n,
        "drive_points": [7.0] * (n // 2) + [0.0] * (n - n // 2),
    })


def _fake_loader(years, cache_dir="data/cache"):
    """Stand-in for load_grid_plays — one contract per requested season."""
    return _fake_contract()


def test_season_stats_counts_and_coverage():
    """Full participation -> off/def coverage 1.0 and correct play/drive counts."""
    plays = _fake_contract(n=6)
    s = _season_stats(plays)
    assert s["n_plays"] == 6
    assert s["n_drives"] == 2
    assert s["off_coverage"] == 1.0 and s["def_coverage"] == 1.0


def test_season_stats_partial_participation():
    """A play with empty off_players drops off_coverage below 1.0."""
    plays = _fake_contract(n=4)
    # one of four plays missing offense participation -> 0.75 coverage
    plays["off_players"] = [(), ("QB1", "WR1"), ("QB1", "WR1"), ("QB1", "WR1")]
    s = _season_stats(plays)
    assert s["off_coverage"] == 0.75


def test_ingest_writes_snapshot_and_manifest(tmp_path):
    """ingest writes one parquet per season + a manifest matching the return value."""
    years = [2022, 2023]
    manifest = ingest(years=years, snapshot_dir=str(tmp_path),
                      cache_dir=str(tmp_path / "cache"), loader=_fake_loader)
    # per-season parquet snapshots exist
    for y in years:
        assert (tmp_path / f"grid_plays_{y}.parquet").exists()
    # manifest written + matches return value
    disk = json.loads((tmp_path / "manifest.json").read_text())
    assert disk == manifest
    assert disk["years"] == years
    assert set(disk["per_season"]) == {"2022", "2023"}
    assert disk["per_season"]["2022"]["n_plays"] == 6
    assert disk["totals"]["n_plays"] == 12   # 6 per season x 2
    assert disk["per_season"]["2023"]["off_coverage"] == 1.0


def test_ingest_is_idempotent(tmp_path):
    """Re-running overwrites in place and yields an identical manifest."""
    kw = dict(years=[2023], snapshot_dir=str(tmp_path),
              cache_dir=str(tmp_path / "cache"), loader=_fake_loader)
    m1 = ingest(**kw)
    m2 = ingest(**kw)
    assert m1 == m2


def test_ingest_snapshot_roundtrips(tmp_path):
    """The frozen snapshot is a faithful copy of the assembled contract."""
    ingest(years=[2023], snapshot_dir=str(tmp_path),
           cache_dir=str(tmp_path / "cache"), loader=_fake_loader)
    back = pd.read_parquet(tmp_path / "grid_plays_2023.parquet")
    expected = _fake_contract()
    # parquet round-trips list-likes as numpy arrays; coerce back to tuples and
    # compare the FULL payload (values + types), not just shape.
    for col in ("off_players", "def_players"):
        back[col] = back[col].apply(tuple)
    pd.testing.assert_frame_equal(back, expected)


def test_ingest_empty_years_is_noop(tmp_path):
    """An explicit empty years list ingests nothing (not the defaults)."""
    manifest = ingest(years=[], snapshot_dir=str(tmp_path),
                      cache_dir=str(tmp_path / "cache"), loader=_fake_loader)
    assert manifest["years"] == []
    assert manifest["per_season"] == {}
    assert manifest["totals"]["n_plays"] == 0
    assert not list(tmp_path.glob("grid_plays_*.parquet"))


def test_ingest_rejects_duplicate_years(tmp_path):
    """Duplicate seasons would corrupt per_season/totals — rejected up front."""
    with pytest.raises(ValueError, match="duplicate"):
        ingest(years=[2023, 2023], snapshot_dir=str(tmp_path),
               cache_dir=str(tmp_path / "cache"), loader=_fake_loader)
