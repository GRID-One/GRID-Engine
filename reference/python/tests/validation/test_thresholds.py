"""Tests for the calibrate-then-gate threshold registry (roadmap §6.5 #8)."""
import json

import numpy as np
import pytest

from backend.validation import thresholds as TH


def test_promotion_math():
    """gate = baseline mean + z * SE(baseline) on a hand-computable sample."""
    samples = [1.0, 2.0, 3.0, 4.0]           # mean 2.5, sd ~1.29099, n 4
    reg = {"kpis": {}}
    entry = TH.promote(reg, "kpi_a", samples, z=1.959964, run_label="run-1")
    se = np.std(samples, ddof=1) / 2.0
    assert entry["baseline_mean"] == pytest.approx(2.5)
    assert entry["baseline_se"] == pytest.approx(se)
    assert entry["gate"] == pytest.approx(2.5 + 1.959964 * se)
    assert entry["n"] == 4
    assert entry["run_label"] == "run-1"


def test_freeze_once_semantics():
    """A second promotion is a no-op; force=True deliberately recalibrates."""
    reg = {"kpis": {}}
    first = TH.promote(reg, "kpi_a", [1.0, 2.0, 3.0], run_label="run-1")
    second = TH.promote(reg, "kpi_a", [100.0, 200.0, 300.0], run_label="run-2")
    assert second == first                       # frozen: no quiet re-tuning
    assert reg["kpis"]["kpi_a"]["gate"] == first["gate"]

    forced = TH.promote(reg, "kpi_a", [100.0, 200.0, 300.0],
                        run_label="run-2", force=True)
    assert forced["gate"] != first["gate"]       # explicit recalibration only
    assert reg["kpis"]["kpi_a"]["run_label"] == "run-2"


def test_promote_returns_a_copy_of_the_frozen_entry():
    """Mutating the returned entry must not corrupt the frozen registry state
    (the freeze-once guarantee lives in the registry, not the return value)."""
    reg = {"kpis": {}}
    entry = TH.promote(reg, "kpi_a", [1.0, 2.0, 3.0])
    entry["gate"] = -999.0
    assert reg["kpis"]["kpi_a"]["gate"] != -999.0
    again = TH.promote(reg, "kpi_a", [50.0, 60.0])     # freeze-once path
    again["gate"] = -999.0
    assert reg["kpis"]["kpi_a"]["gate"] != -999.0


def test_promotion_rejects_degenerate_baseline():
    """A single sample has no SE — refuse to promote a fake noise floor."""
    with pytest.raises(ValueError):
        TH.promote({"kpis": {}}, "kpi_a", [1.0])


def test_gate_check_directions():
    """value > gate passes; below fails; an unpromoted KPI has no verdict."""
    reg = {"kpis": {}}
    TH.promote(reg, "kpi_a", [0.0, 0.0, 0.0, 0.0])   # gate = 0.0
    assert TH.gate_check(reg, "kpi_a", 0.5) == (True, 0.0)
    assert TH.gate_check(reg, "kpi_a", -0.5) == (False, 0.0)
    assert TH.gate_check(reg, "kpi_never_promoted", 0.5) == (None, None)


def test_registry_round_trip(tmp_path):
    """save/load preserves entries; a missing file yields an empty registry."""
    p = tmp_path / "reg.json"
    reg = TH.load_registry(p)
    assert reg == {"kpis": {}}                   # missing file -> fresh
    TH.promote(reg, "kpi_a", [1.0, 2.0], run_label="run-1")
    TH.save_registry(reg, p)
    loaded = TH.load_registry(p)
    assert loaded == reg
    assert json.loads(p.read_text())["kpis"]["kpi_a"]["run_label"] == "run-1"


def test_committed_registry_is_valid_and_empty():
    """The version-controlled provisional_thresholds.json exists, parses, and
    ships with no frozen gates (the first real verdict run fills it)."""
    assert TH.REGISTRY_PATH.exists()
    reg = TH.load_registry()
    assert reg["kpis"] == {}
