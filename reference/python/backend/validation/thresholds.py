"""Calibrate-then-gate threshold registry (roadmap §6.5 #8; Phase 2c).

Directional KPIs (Tier-1 skill margins, the H2 lineup margin, Tier-2 matchup
predictive power) can't have their pass/fail thresholds picked in advance —
any number chosen before seeing real data is a guess to be argued with.  The
mechanism instead is **calibrate then gate**:

1. The **first full verdict run** measures each KPI's *baseline noise floor*
   and promotes a gate = ``baseline_mean + z * SE(baseline)`` (z = 1.96, the
   95% one-sided-ish noise bound) into the version-controlled registry file
   ``provisional_thresholds.json``.
2. The entry is then **frozen**: later runs gate against it and a re-promotion
   attempt is a no-op — no quiet re-tuning of the bar to whatever the latest
   run produced.  Recalibrating is a deliberate, reviewable act
   (``force=True`` plus the diff of the committed JSON).

The registry file lives next to this module and IS committed (unlike every
other validation artifact) because the gates are part of the spec once
frozen, not reproducible output.
"""
from __future__ import annotations

import json
from pathlib import Path

import numpy as np

REGISTRY_PATH = Path(__file__).parent / "provisional_thresholds.json"

_Z = 1.959964    # matches the 95% interval convention in metrics.py


def load_registry(path: Path | str | None = None) -> dict:
    """Load the registry (``{"kpis": {name: entry}}``) from ``path``.

    Defaults to the committed :data:`REGISTRY_PATH`.  A missing file yields a
    fresh empty registry so the first verdict run can promote into it.
    """
    p = Path(path) if path is not None else REGISTRY_PATH
    if not p.exists():
        return {"kpis": {}}
    with open(p, encoding="utf-8") as fh:
        reg = json.load(fh)
    reg.setdefault("kpis", {})
    return reg


def save_registry(registry: dict, path: Path | str | None = None) -> None:
    """Write the registry back as stable, diff-friendly JSON (sorted keys)."""
    p = Path(path) if path is not None else REGISTRY_PATH
    with open(p, "w", encoding="utf-8") as fh:
        json.dump(registry, fh, indent=2, sort_keys=True)
        fh.write("\n")


def promote(
    registry: dict,
    name: str,
    baseline_samples,
    *,
    z: float = _Z,
    run_label: str | None = None,
    force: bool = False,
) -> dict:
    """Promote a KPI's measured noise floor into a frozen gate.

    ``baseline_samples`` are the per-unit values of the *reference* the KPI
    must clear (e.g. the per-week margins of a no-skill/shuffled control, or
    the baseline's own resampled statistic): the gate becomes
    ``mean + z * std/sqrt(n)`` — the level a signal-free run would only reach
    by luck.  ``run_label`` records which run froze it (pass an explicit
    label; no implicit timestamps, so promotion is reproducible).

    **Freeze-once:** if ``name`` already has an entry, the call is a no-op
    returning the existing entry unless ``force=True`` (a deliberate
    recalibration that should ship as a reviewed diff of the JSON).  Returns
    a **copy** of the (existing or new) entry — mutating the return value
    must not reach the frozen registry state; the registry dict itself is
    mutated in place.
    """
    kpis = registry.setdefault("kpis", {})
    if name in kpis and not force:
        return dict(kpis[name])

    samples = np.asarray(baseline_samples, dtype=float)
    if samples.size < 2:
        raise ValueError(
            f"promote({name!r}) needs >= 2 baseline samples to estimate a "
            f"noise floor; got {samples.size}")
    mean = float(samples.mean())
    se = float(samples.std(ddof=1) / np.sqrt(samples.size))
    entry = {
        "gate": mean + z * se,
        "baseline_mean": mean,
        "baseline_se": se,
        "n": int(samples.size),
        "z": z,
        "run_label": run_label,
    }
    kpis[name] = entry
    return dict(entry)


def gate_check(registry: dict, name: str, value: float) -> tuple[bool | None, float | None]:
    """Check a measured KPI value against its frozen gate.

    Returns ``(passed, gate)``: ``passed`` is ``value > gate``, or ``None``
    when the KPI has never been promoted (no gate exists yet — the verdict
    report shows the value as informational, not pass/fail).
    """
    entry = registry.get("kpis", {}).get(name)
    if entry is None:
        return None, None
    gate = float(entry["gate"])
    return value > gate, gate


__all__ = ["REGISTRY_PATH", "gate_check", "load_registry", "promote", "save_registry"]
