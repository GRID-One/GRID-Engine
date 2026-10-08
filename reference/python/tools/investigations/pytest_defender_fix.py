"""pytest plugin: run the oracle's own tests against the defender-fixed synthetic generator.

What it demonstrates
    Loading this plugin swaps the oracle's ``backend.grid.synth.simulate`` for the
    defender-fixed generator **in the test process only** (``_common.apply_defender_fix``;
    ``backend/grid/synth.py`` on disk is never modified), so the unchanged gates can be
    re-run on the corrected world. Recorded by the consolidation critic (G-1), threads = 1:

        tests/grid/test_tier0_recovery.py      8 passed
        tests/grid/test_determinism.py         2 passed
        tests/grid/test_golden_master.py       4 failed: test_layerB_top_player_ranking,
                                               test_layerB_team_ranking_order,
                                               test_layerC_player_and_team_ratings,
                                               test_layerC_qb_weekly_and_kalman
        tests/grid/test_calibration_synth.py   1 failed: test_pooled_nis_bounded
                                               (pooled NIS 1.829, band [0.8, 1.4])

    i.e. the golden master and the QB r_scale calibration were frozen on the buggy world.
Evidence for
    KI-NEW-Y0; the correction ledger entry 1 in reference/python/PARITY.md (goldens
    affected); DR-B1 / DR-B4 (proposed defaults, pending the owner).
Synthetic generator
    Defender-fixed (that is the point of the plugin). Without it the same command runs the
    legacy generator and everything passes.
Run (from reference/python, about 22 s; add -q -rf for the summary below)
    python3 -m pytest -p tools.investigations.pytest_defender_fix \\
        tests/grid/test_tier0_recovery.py tests/grid/test_determinism.py \\
        tests/grid/test_golden_master.py tests/grid/test_calibration_synth.py
Expected output (recorded 2026-10-01 with requirements.lock pins)
    ...............FF..FF..F...                                              [100%]
    (failure details; the golden ones list the largest movers, e.g.
     "qb_credit drifted beyond rtol=1e-05/atol=1e-06; largest movers: 4: golden=0.57512
     live=0.25859 (delta 0.317)", and "AssertionError: pooled NIS=1.829 outside [0.8,1.4]")
    isolation guard: 0 violations
    FAILED tests/grid/test_golden_master.py::test_layerB_top_player_ranking
    FAILED tests/grid/test_golden_master.py::test_layerB_team_ranking_order
    FAILED tests/grid/test_golden_master.py::test_layerC_player_and_team_ratings
    FAILED tests/grid/test_golden_master.py::test_layerC_qb_weekly_and_kalman
    FAILED tests/grid/test_calibration_synth.py::test_pooled_nis_bounded
    5 failed, 22 passed (about 19 s; exit status 1)
    The non-zero exit status is the expected result: it is the evidence.
Origin
    Replaces the inventory's patched copy ``scratch-critic/cnfix`` (critic report G-1) with
    an in-memory patch of the one imported oracle.
"""
from __future__ import annotations

from tools.investigations._common import apply_defender_fix

apply_defender_fix()


def pytest_report_header(config):
    return ("synthetic generator: DEFENDER-FIXED in memory (tools/investigations/pytest_defender_fix.py); "
            "results are investigation evidence, not the oracle's gate")
