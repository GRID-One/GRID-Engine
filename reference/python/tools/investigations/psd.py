"""Stress test: are the RTS-smoothed state covariances positive semi-definite?

What it demonstrates
    The RTS smoother update ``Ps[w] = Pf + C (Ps[w+1] - Pp[w+1]) C^T`` in
    ``kalman_two_component`` is not PSD-preserving by construction. This script runs 3,000
    random filters (3-19 weeks, snaps from 1 to 4000, about 30% missed weeks, 0-2
    interventions, r_scale 1e-4 or 0.4) and reports the smallest eigenvalue of any smoothed
    covariance, the largest asymmetry, and how many smoothed talent variances are negative.
    Result: no violation was reproduced, so the defect stays theoretical.
Evidence for
    KI-G4 (smoothed covariance not guaranteed PSD; "open in theory, not reproduced"). Input
    to DR-B6 (proposed default, pending the owner: Rust symmetrizes and fails typed).
Synthetic generator
    None: random series from numpy's default_rng(0).
Run (from reference/python, about 6 s)
    python3 -m tools.investigations.psd
Expected output (recorded 2026-10-01 with requirements.lock pins)
    min eigenvalue over smoothed covs: 0.000e+00  max asymmetry 2.082e-17  negative var_tau_smooth count 0
    (about 6 s with threads=1)
Origin
    Consolidation inventory scratch ``scratch-cnissues/psd.py`` (cn-issues report section
    3.3). Logic unchanged; only the path bootstrap and this header were added.
"""
from __future__ import annotations

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2]))
from tools.investigations._common import bootstrap  # noqa: E402

bootstrap()

import numpy as np  # noqa: E402

from backend.grid.statespace import SSParams, kalman_two_component  # noqa: E402


def main() -> None:
    rng = np.random.default_rng(0)
    worst = 0
    asym = 0
    negvar = 0
    for _ in range(3000):
        W = rng.integers(3, 20)
        y = rng.normal(0, 0.1, W)
        snaps = rng.choice([1, 5, 40, 400, 4000], W).astype(float)
        played = rng.random(W) > 0.3
        intv = set(rng.choice(W, rng.integers(0, 3), replace=False).tolist())
        out = kalman_two_component(y, snaps, played, interventions=intv,
                                   params=SSParams(r_scale=rng.choice([1e-4, 0.4])))
        for P in out["sigma_smooth"]:
            ev = np.linalg.eigvalsh((P + P.T) / 2)
            worst = min(worst, ev.min())
            asym = max(asym, np.abs(P - P.T).max())
        negvar += int((out["var_tau_smooth"] < 0).sum())
    print("min eigenvalue over smoothed covs: %.3e  max asymmetry %.3e  negative var_tau_smooth count %d"
          % (worst, asym, negvar))


if __name__ == "__main__":
    main()
