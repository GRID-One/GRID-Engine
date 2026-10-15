"""Tier-0 recovery values on the legacy vs the defender-fixed synthetic generator.

What it demonstrates
    Computes every value the oracle's Tier-0 gates assert on, using the gate's own fixture
    function (``tests/grid/test_tier0_recovery.py::recovery``: canonical ``load_synthetic()``,
    market seed 1, ``fit(n_iter=3)``, focus-QB Kalman with an intervention at week 9, priors),
    once per generator, each in its own single-threaded subprocess, and prints them side by
    side with the share of plays whose defenders come from the offense / defence team.
    This is the critic G-1 impact table: fixing the defenders keeps Tier-0 at 8/8 but moves
    every number, and focus-QB NIS nearly doubles towards its <= 10 guard.
Evidence for
    KI-NEW-Y0 (synthetic generator draws defenders from the offense's own team); the
    "legacy vs defender-fixed Tier-0" table in reference/python/PARITY.md; input to DR-B3
    (proposed default, pending the owner: recovery floors re-set on the fixed generator)
    and DR-B4. For the oracle's other gates
    on the fixed generator (golden master 4 fail, synth calibration 1 fail), run pytest with
    ``-p tools.investigations.pytest_defender_fix`` (see README.md in this folder).
Synthetic generator
    Both: ``legacy`` = the oracle as imported; ``fixed`` = defenders from def_team, in memory
    only (see _common.py). Planted ``team_strength`` keeps the legacy off - def definition in
    both, so the fixed column's team correlation is against a semantically wrong target.
Run (from reference/python, about 17 s)
    python3 -m tools.investigations.tier0_legacy_vs_fixed
    python3 -m tools.investigations.tier0_legacy_vs_fixed --synth fixed --json   # one variant
Expected output (recorded 2026-10-01 with requirements.lock pins)
    Tier-0 value (canonical synth, fit n_iter=3, market seed 1)    legacy     fixed
    defenders from off_team (share of plays)                 1.000     0.000
    defenders from def_team (share of plays)                 0.000     1.000
    pooled attribution corr (gate >= 0.77)                  0.8025    0.8276
    QB attribution corr (gate >= 0.83)                      0.8690    0.8845
    RB attribution corr (gate >= 0.70)                      0.7433    0.7711
    WR attribution corr (gate >= 0.76)                      0.7973    0.8599
    TE attribution corr (gate >= 0.73)                      0.7713    0.7784
    DEF attribution corr (gate >= 0.73)                     0.7653    0.7827
    team corr vs legacy off-def target (gate >= 0.60)       0.6643    0.6596
    Kalman total_smooth corr (gate >= 0.92)                 0.9580    0.9760
    Kalman tau_smooth corr (gate >= 0.60)                   0.6745    0.6538
    focus-QB predictive NIS (gate <= 10)                     4.581     8.371
    equivalency slope (gate in [0.9, 1.8])                  1.3159    1.2119
    feeder->NFL OOS R^2 (gate >= 0.05)                      0.1489    0.2134
    rookie prior corr (gate >= 0.50)                        0.5829    0.5829
Origin
    Consolidation inventory scratch ``scratch-critic/t0.py`` run in ``scratch-critic/cn`` and
    ``scratch-critic/cnfix`` (critic report G-1). Rewritten: the fixture is called through
    ``__wrapped__`` instead of re-executing the test module's source, both variants run from
    the one imported oracle, and the output is a single table.
"""
from __future__ import annotations

import argparse
import json
import pathlib
import subprocess
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2]))
from tools.investigations._common import ROOT, add_synth_arg, bootstrap, defender_shares, select_synth  # noqa: E402

bootstrap()

ROWS = [  # (label, key, digits)
    ("defenders from off_team (share of plays)", "def_from_off", 3),
    ("defenders from def_team (share of plays)", "def_from_def", 3),
    ("pooled attribution corr (gate >= 0.77)", "overall", 4),
    ("QB attribution corr (gate >= 0.83)", "QB", 4),
    ("RB attribution corr (gate >= 0.70)", "RB", 4),
    ("WR attribution corr (gate >= 0.76)", "WR", 4),
    ("TE attribution corr (gate >= 0.73)", "TE", 4),
    ("DEF attribution corr (gate >= 0.73)", "DEF", 4),
    ("team corr vs legacy off-def target (gate >= 0.60)", "team_corr", 4),
    ("Kalman total_smooth corr (gate >= 0.92)", "total_smooth_corr", 4),
    ("Kalman tau_smooth corr (gate >= 0.60)", "tau_smooth_corr", 4),
    ("focus-QB predictive NIS (gate <= 10)", "nis", 3),
    ("equivalency slope (gate in [0.9, 1.8])", "equiv_slope", 4),
    ("feeder->NFL OOS R^2 (gate >= 0.05)", "oos_r2", 4),
    ("rookie prior corr (gate >= 0.50)", "rookie_corr", 4),
]


def measure() -> dict:
    from threadpoolctl import threadpool_limits

    from backend.grid import load_synthetic
    import tests.grid.test_tier0_recovery as T

    fixture = T.recovery
    fn = getattr(fixture, "__wrapped__", None)
    if fn is None and hasattr(fixture, "_get_wrapped_function"):
        fn = fixture._get_wrapped_function()
    if fn is None:
        raise SystemExit("cannot reach the undecorated Tier-0 fixture with this pytest version")
    with threadpool_limits(1):
        b = load_synthetic()
        from_off, from_def = defender_shares(b["plays"], b["players"])
        r = fn()
    out = {"def_from_off": from_off, "def_from_def": from_def}
    for k in ("overall", "team_corr", "total_smooth_corr", "tau_smooth_corr", "nis",
              "equiv_slope", "oos_r2", "rookie_corr"):
        out[k] = float(r[k])
    out.update({k: float(v) for k, v in r["perpos"].items()})
    return out


def main(argv=None) -> None:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    add_synth_arg(ap)
    ap.add_argument("--json", action="store_true", help="measure the one --synth variant and print JSON")
    args = ap.parse_args(argv)
    if args.json:
        select_synth(args.synth)
        print(json.dumps(measure()))
        return
    res = {}
    for variant in ("legacy", "fixed"):
        proc = subprocess.run([sys.executable, "-m", "tools.investigations.tier0_legacy_vs_fixed",
                               "--synth", variant, "--json"], cwd=ROOT, capture_output=True, text=True)
        if proc.returncode != 0:
            sys.stderr.write(proc.stderr)
            raise SystemExit(f"{variant} run failed (exit {proc.returncode})")
        res[variant] = json.loads(proc.stdout.strip().splitlines()[-1])
    print(f"{'Tier-0 value (canonical synth, fit n_iter=3, market seed 1)':52s} {'legacy':>9s} {'fixed':>9s}")
    for label, key, digits in ROWS:
        print(f"{label:52s} {res['legacy'][key]:9.{digits}f} {res['fixed'][key]:9.{digits}f}")


if __name__ == "__main__":
    main()
