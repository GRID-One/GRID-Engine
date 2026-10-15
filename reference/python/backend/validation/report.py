"""Phase-2c verdict report artifacts (roadmap §5 Phase 2c; validation plan §11).

Renders the assembled verdict results (Tier-1 weekly/ROS tables, the H2
lineup-margin simulation, the Tier-2 matchup check, the gate registry state,
and the H1 kill-criterion verdict) to ``data/reports/phase2c_verdict.json``
(machine-readable, BootstrapCI tuples expanded to dicts) and a human-readable
``phase2c_verdict.md`` — mirroring the pipeline's artifact conventions
(``health_check.write_health`` / ``ingest_grid``): reports are reproducible
output and stay gitignored; only the thresholds registry is committed.
"""
from __future__ import annotations

import json
from pathlib import Path

from backend.validation.metrics import BootstrapCI

DEFAULT_REPORT_DIR = "data/reports"


def _jsonable(obj):
    """Recursively convert results to JSON-serializable form.

    ``BootstrapCI`` named tuples become dicts (a bare tuple would silently
    drop the field names the report consumers key on); dict/list/tuple
    containers recurse; everything else passes through.
    """
    if isinstance(obj, BootstrapCI):
        return dict(obj._asdict())
    if isinstance(obj, dict):
        return {k: _jsonable(v) for k, v in obj.items()}
    if isinstance(obj, (list, tuple)):
        return [_jsonable(v) for v in obj]
    return obj


def _fmt_ci(ci) -> str:
    """One-cell rendering of a BootstrapCI: point [lo, hi] and the gate mark."""
    if ci is None:
        return "-"
    mark = "PASS" if (ci.point > 0 and ci.excludes_zero) else "no"
    return f"{ci.point:+.3f} [{ci.lower:+.3f}, {ci.upper:+.3f}] {mark}"


def _tier1_section(title: str, rep: dict) -> list[str]:
    """Markdown lines for one tier1_report table (per-position + ALL)."""
    lines = [f"## {title}", "",
             "| pos | n | MAE | vs persistence | vs season-mean | vs last-season |",
             "|-----|---|-----|----------------|----------------|----------------|"]
    for pos in sorted(rep, key=lambda p: (p == "ALL", p)):   # ALL last
        e = rep[pos]
        cells = [pos, str(e["n_cells"]), f"{e['mae']:.3f}"]
        for name in ("persistence", "season_to_date_mean", "last_season"):
            b = e["baselines"].get(name)
            cells.append(_fmt_ci(b["margin"]) if b else "-")
        lines.append("| " + " | ".join(cells) + " |")
    lines.append("")
    cal_rows = [(pos, e["calibration"]) for pos, e in rep.items() if e["calibration"]]
    if cal_rows:
        lines += ["| pos | n | PICP | PINAW | CRPS | NIS |",
                  "|-----|---|------|-------|------|-----|"]
        for pos, c in sorted(cal_rows, key=lambda r: (r[0] == "ALL", r[0])):
            lines.append(f"| {pos} | {c['n_cells']} | {c['picp']:.3f} | "
                         f"{c['pinaw']:.3f} | {c['crps']:.3f} | {c['nis']:.3f} |")
        lines.append("")
    return lines


def render_markdown(results: dict) -> str:
    """Render the verdict results dict to the human-readable report body.

    ``results`` is ``verdict.run_verdict``'s output.  The headline verdict
    (SHIP_GRID vs BASELINE_FALLBACK, from the H1 kill criterion) leads; the
    supporting tables follow.  Sections whose inputs were unavailable (e.g.
    Tier-2 without a points-allowed feed) render as "skipped" rather than
    disappearing silently.
    """
    lines = ["# Phase 2c verdict", "",
             f"**H1 verdict: {results['verdict']}** — {results['verdict_reason']}", ""]
    if results.get("run_label"):
        lines += [f"Run: `{results['run_label']}`", ""]

    lines += _tier1_section("Tier 1 — ROS (H1, kill criterion)", results["tier1_ros"])
    lines += _tier1_section("Tier 1 — weekly", results["tier1_weekly"])

    dq = results.get("ros_data_quality")
    if dq and dq.get("degraded_columns"):
        cols = ", ".join(dq["degraded_columns"])
        lines += ["## ROS data quality", "",
                  f"⚠️ {len(dq['degraded_columns'])} scoring column(s) absent or "
                  f"all-null in `player_stats` — the ROS stat-line model scored "
                  f"these as 0.0: {cols}", ""]

    h2 = results.get("h2")
    if h2 is not None:
        lines += ["## H2 — weekly lineup margin (GRID vs last-season)", "",
                  f"- cells: {len(h2['margins'])}, win rate: {h2['win_rate']:.3f}",
                  f"- overall margin: {_fmt_ci(h2['overall'])}",
                  f"- starter-OUT subset ({len(h2['starter_out_margins'])} cells): "
                  f"{_fmt_ci(h2['starter_out'])}", ""]
    else:
        lines += ["## H2 — weekly lineup margin", "", "_skipped (no availability data)_", ""]

    t2 = results.get("tier2")
    if t2 is not None:
        lines += ["## Tier 2 — matchup-grade predictive power", "",
                  f"- weeks: {t2['n_weeks']}, cells: {t2['n_cells']}",
                  f"- predictive: {_fmt_ci(t2['predictive'])}", ""]
    else:
        lines += ["## Tier 2 — matchup-grade predictive power", "",
                  "_skipped (no points-allowed feed supplied)_", ""]

    gates = results.get("gates", {})
    if gates:
        lines += ["## Gates (calibrate-then-gate registry)", "",
                  "| KPI | value | gate | passed |", "|-----|-------|------|--------|"]
        for name, g in sorted(gates.items()):
            passed = {True: "yes", False: "NO", None: "(no gate yet)"}[g["passed"]]
            gate_s = "-" if g["gate"] is None else f"{g['gate']:+.4f}"
            lines.append(f"| {name} | {g['value']:+.4f} | {gate_s} | {passed} |")
        lines.append("")
    return "\n".join(lines)


def write_report(results: dict, out_dir: str = DEFAULT_REPORT_DIR) -> tuple[Path, Path]:
    """Write ``phase2c_verdict.{json,md}`` under ``out_dir``; returns the paths."""
    out = Path(out_dir)
    out.mkdir(parents=True, exist_ok=True)
    json_path = out / "phase2c_verdict.json"
    md_path = out / "phase2c_verdict.md"
    json_path.write_text(
        json.dumps(_jsonable(results), indent=2, sort_keys=True,
                   allow_nan=False) + "\n",     # strict JSON: fail fast on NaN/Inf
        encoding="utf-8")
    md_path.write_text(render_markdown(results), encoding="utf-8")
    return json_path, md_path


__all__ = ["DEFAULT_REPORT_DIR", "render_markdown", "write_report"]
