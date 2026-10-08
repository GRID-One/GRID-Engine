"""Tests for the verdict report artifacts (Phase 2c; plan §11)."""
import json

from backend.validation.metrics import BootstrapCI
from backend.validation import report as R


def _ci(point=1.0, lo=0.5, hi=1.5, ex=True):
    return BootstrapCI(point=point, lower=lo, upper=hi, excludes_zero=ex)


def _results(tier2=None):
    """A minimal hand-built run_verdict-shaped results dict."""
    t1 = {"QB": {"n_cells": 10, "mae": 2.5,
                 "baselines": {"last_season": {"n_pairs": 10, "mae_grid": 2.5,
                                               "mae_baseline": 4.0, "skill": 0.375,
                                               "margin": _ci()}},
                 "calibration": {"n_cells": 8, "picp": 0.81, "pinaw": 0.4,
                                 "crps": 1.2, "nis": 1.05}},
          "ALL": {"n_cells": 10, "mae": 2.5,
                  "baselines": {"last_season": {"n_pairs": 10, "mae_grid": 2.5,
                                                "mae_baseline": 4.0, "skill": 0.375,
                                                "margin": _ci()}},
                  "calibration": None}}
    return {"run_label": "test-run", "n_origins": 3,
            "verdict": "SHIP_GRID", "verdict_reason": "margin excludes 0",
            "tier1_ros": t1, "tier1_weekly": t1,
            "h2": {"margins": [1.0, 2.0], "win_rate": 1.0, "overall": _ci(),
                   "starter_out": None, "starter_out_margins": []},
            "tier2": tier2,
            "gates": {"h1_ros_margin_vs_last_season":
                      {"value": 1.0, "gate": 0.2, "passed": True},
                      "kpi_ungated": {"value": 0.5, "gate": None, "passed": None}},
            "registry": {"kpis": {}}}


def test_markdown_renders_verdict_and_sections():
    """The verdict headline leads; skipped Tier-2 stays visible as skipped."""
    md = R.render_markdown(_results(tier2=None))
    assert md.startswith("# Phase 2c verdict")
    assert "SHIP_GRID" in md and "margin excludes 0" in md
    assert "Tier 1 — ROS (H1, kill criterion)" in md
    assert "_skipped (no points-allowed feed supplied)_" in md
    assert "(no gate yet)" in md          # ungated KPI shown as informational


def test_markdown_renders_tier2_when_present():
    t2 = {"n_weeks": 5, "n_cells": 40, "weekly": [0.5] * 5, "predictive": _ci()}
    md = R.render_markdown(_results(tier2=t2))
    assert "weeks: 5, cells: 40" in md
    assert "_skipped" not in md.split("Tier 2")[1].split("##")[0]


def test_write_report_round_trips_json(tmp_path):
    """Artifacts land in out_dir; BootstrapCI serializes to a named dict."""
    json_path, md_path = R.write_report(_results(), out_dir=tmp_path)
    assert json_path.exists() and md_path.exists()
    data = json.loads(json_path.read_text())
    margin = data["tier1_ros"]["QB"]["baselines"]["last_season"]["margin"]
    assert margin == {"point": 1.0, "lower": 0.5, "upper": 1.5,
                      "excludes_zero": True}
    assert data["verdict"] == "SHIP_GRID"
    assert "# Phase 2c verdict" in md_path.read_text()
