"""Batch vs incremental Kalman filter divergence after an intervention.

What it demonstrates
    ``kalman_two_component`` (the batch filter used by backtests and goldens) and
    ``kalman_step`` (the incremental filter used by ``weekly_update``) agree week by week from
    the same x0/P0 until an intervention, then diverge: the batch filter keeps inflating the
    observation variance for ``post_event_games`` after the event (it counts
    ``games_since_event``), while ``kalman_step`` carries no event counter and inflates only
    in the intervention week. Same y, snaps and SSParams; intervention at index 5 (week 6).
Evidence for
    KI-NEW-S1 (two filter cores disagree after interventions); input to DR-C10 (proposed default,
    pending the owner: one filter core with a persisted games_since_event).
Synthetic generator
    None: a fixed 12-week series drawn from numpy's default_rng(0).
Run (from reference/python, about 2 s)
    python3 -m tools.investigations.kparity
Expected output (recorded 2026-10-01 with requirements.lock pins)
    max |batch - incremental| total_filt by week: [0.       0.       0.       0.       0.       0.       0.009342 0.003882
     0.003657 0.00262  0.001606 0.000942]
    i.e. identical through week 6, then non-zero from week 7 (the inventory quoted the first
    three non-zero values as 0.0093, 0.0039, 0.0037).
Origin
    Consolidation inventory scratch ``scratch-cnissues/kparity.py`` (cn-issues report
    section 3.3). Logic unchanged; only the path bootstrap and this header were added.
"""
from __future__ import annotations

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[2]))
from tools.investigations._common import bootstrap  # noqa: E402

bootstrap()

import numpy as np  # noqa: E402

from backend.grid.statespace import KalmanState, SSParams, kalman_step, kalman_two_component  # noqa: E402


def main() -> None:
    rng = np.random.default_rng(0)
    W = 12
    y = 0.1 + rng.normal(0, 0.05, W)
    snaps = np.full(W, 40.0)
    played = np.ones(W, bool)
    p = SSParams()
    out = kalman_two_component(y, snaps, played, interventions={5}, params=p, x0=[0, 0, 0],
                               P0=np.diag([0.05, 0.02, 0.01]))
    ks = KalmanState.init(["a"])
    filt = []
    for w in range(W):
        ks = kalman_step(ks, np.array([y[w]]), np.array([snaps[w]]), p,
                         interventions={"a"} if w == 5 else None)
        filt.append(ks.mu[0].sum())
    d = np.abs(np.array(filt) - out["total_filt"])
    print("max |batch - incremental| total_filt by week:", np.round(d, 6))


if __name__ == "__main__":
    main()
