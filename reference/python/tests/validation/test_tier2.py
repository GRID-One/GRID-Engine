"""Tests for the Tier-2 matchup-grade predictive check (Phase 2c; plan §9).

Planted setup: defence strengths drive both the grades and (negatively) the
realized points allowed, so a real signal must show predictive > 0 with the
CI excluding 0 — and a shuffled control must not.
"""
import numpy as np
import pandas as pd
import pytest

from backend.validation.tier2 import tier2_report

TEAMS = [f"T{i}" for i in range(10)]
WEEKS = range(1, 9)


def _frames(signal=True, seed=0):
    """Grades = planted strength; points allowed = 20 - 3*strength + noise.

    With ``signal=False`` the grades are shuffled independently each week —
    the no-signal control the CI must NOT call predictive.
    """
    rng = np.random.default_rng(seed)
    strength = {t: float(i) / 3.0 for i, t in enumerate(TEAMS)}
    g_rows, a_rows = [], []
    for wk in WEEKS:
        graded = list(TEAMS)
        if not signal:
            graded = list(rng.permutation(TEAMS))
        for team, src in zip(TEAMS, graded, strict=True):
            g_rows.append({"season": 2024, "week": wk, "def_team": team,
                           "rapm_grade": strength[src] + rng.normal(0, 0.05)})
            a_rows.append({"season": 2024, "week": wk, "def_team": team,
                           "points_allowed": 20.0 - 3.0 * strength[team]
                                             + rng.normal(0, 1.0)})
    return pd.DataFrame(g_rows), pd.DataFrame(a_rows)


def test_planted_signal_is_predictive():
    """True-strength grades: predictive > 0 and the CI excludes 0."""
    grades, allowed = _frames(signal=True)
    rep = tier2_report(grades, allowed, n=500)
    assert rep["n_weeks"] == len(list(WEEKS))
    assert rep["n_cells"] == len(TEAMS) * len(list(WEEKS))
    assert rep["predictive"].point > 0
    assert rep["predictive"].excludes_zero is True


def test_shuffled_grades_are_not_predictive():
    """Shuffled (signal-free) grades: the CI straddles 0."""
    grades, allowed = _frames(signal=False, seed=3)
    rep = tier2_report(grades, allowed, n=500)
    assert rep["predictive"].excludes_zero is False


def test_unmatched_defences_drop_out():
    """A graded defence with no realized game that week (bye) is excluded by
    the inner join, not scored as zero points allowed."""
    grades, allowed = _frames(signal=True)
    allowed = allowed[~((allowed.week == 1) & (allowed.def_team == "T0"))]
    rep = tier2_report(grades, allowed, n=100)
    assert rep["n_cells"] == len(TEAMS) * len(list(WEEKS)) - 1


def test_thin_weeks_are_skipped():
    """Weeks with fewer than min_teams joined defences don't contribute a
    correlation (a two-team Spearman is noise)."""
    grades, allowed = _frames(signal=True)
    # week 1 keeps only 3 defences -> below the min_teams=6 default
    keep = (allowed.week != 1) | (allowed.def_team.isin(["T0", "T1", "T2"]))
    rep = tier2_report(grades, allowed[keep], n=100)
    assert rep["n_weeks"] == len(list(WEEKS)) - 1


def test_no_scorable_week_raises():
    """If nothing qualifies the report refuses to fabricate a KPI."""
    grades, allowed = _frames(signal=True)
    with pytest.raises(ValueError):
        tier2_report(grades, allowed, min_teams=len(TEAMS) + 1, n=100)


def test_min_teams_below_two_rejected():
    """min_teams < 2 would let a 1-team week produce a NaN spearman —
    rejected up front instead of silently corrupting the CI."""
    grades, allowed = _frames(signal=True)
    with pytest.raises(ValueError):
        tier2_report(grades, allowed, min_teams=1, n=100)


def test_duplicate_join_keys_rejected():
    """Duplicate (season, week, def_team) rows (e.g. an unsliced
    matchup_grades frame with one row per position) are rejected loudly —
    they would multiply through the merge and inflate the KPI."""
    grades, allowed = _frames(signal=True)
    dup = pd.concat([grades, grades.iloc[[0]]], ignore_index=True)
    with pytest.raises(ValueError, match="duplicate"):
        tier2_report(dup, allowed, n=100)
    dup_a = pd.concat([allowed, allowed.iloc[[0]]], ignore_index=True)
    with pytest.raises(ValueError, match="duplicate"):
        tier2_report(grades, dup_a, n=100)
