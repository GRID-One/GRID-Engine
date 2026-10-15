"""_write_kalman_trajectory variance/total fix (roadmap §Phase 0; validation §7).

The shipped trajectory band previously persisted total = talent+form (dropping
scheme_fit) and var_total = sigma[0,0]+sigma[1,1] (dropping scheme variance, the
cross-covariances, AND the observation noise R).  The fix:
  * total includes every active state component (matches the engine's total_filt);
  * var_total is the one-step predictive S (R included) when supplied, else the
    full filtered quadratic form Hᵀ Σ H.
"""
import sqlite3

import numpy as np
import pytest

from backend.grid.statespace import KalmanState
from backend.pipeline.weekly_update import _write_kalman_trajectory


def _conn_with_table() -> sqlite3.Connection:
    """In-memory SQLite connection with the kalman_trajectory table created."""
    conn = sqlite3.connect(":memory:")
    conn.execute(
        """
        CREATE TABLE kalman_trajectory (
            player_id TEXT NOT NULL, season INTEGER NOT NULL, week INTEGER NOT NULL,
            talent REAL NOT NULL, form REAL NOT NULL, total REAL NOT NULL,
            var_total REAL NOT NULL, scheme_fit REAL,
            computed_at TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (player_id, season, week)
        )
        """
    )
    return conn


def _make_state() -> KalmanState:
    """A 2-player width-3 KalmanState with non-trivial cross-covariances so the
    full quadratic form differs from the plain diagonal sum."""
    ks = KalmanState.init(["p1", "p2"])
    ks.mu[0] = [0.30, 0.10, 0.05]          # talent, form, scheme_fit
    ks.mu[1] = [0.20, -0.05, 0.02]
    # non-trivial cross-covariances so the quadratic form != diagonal sum
    ks.sigma[0] = np.array([[0.04, 0.01, 0.00],
                            [0.01, 0.02, 0.00],
                            [0.00, 0.00, 0.01]])
    return ks


def test_total_includes_scheme_fit():
    """total persists talent+form+scheme_fit, not the old talent+form."""
    conn = _conn_with_table()
    ks = _make_state()
    _write_kalman_trajectory(conn, ks, week=3, season=2025)
    row = conn.execute(
        "SELECT talent, form, scheme_fit, total FROM kalman_trajectory WHERE player_id='p1'"
    ).fetchone()
    talent, form, scheme_fit, total = row
    assert scheme_fit == 0.05
    # total must equal talent+form+scheme_fit, not the old talent+form
    assert total == pytest.approx(0.45)
    assert total != pytest.approx(0.40)


def test_var_total_uses_full_quadratic_form_when_no_pred():
    """Without pred_var, var_total is the full Hᵀ Σ H (incl. cross-cov), not the
    old sigma[0,0]+sigma[1,1]."""
    conn = _conn_with_table()
    ks = _make_state()
    _write_kalman_trajectory(conn, ks, week=3, season=2025)
    var_total = conn.execute(
        "SELECT var_total FROM kalman_trajectory WHERE player_id='p1'"
    ).fetchone()[0]
    # Full Hᵀ Σ H over the 3 components = sum of all 9 entries (incl. cross-cov),
    # NOT the old sigma[0,0]+sigma[1,1] = 0.06.
    expected = float(ks.sigma[0].sum())  # 0.04+0.01+0.01+0.02 + 0.01 = 0.09
    assert var_total == pytest.approx(expected)
    assert var_total != pytest.approx(0.06)


def test_var_total_uses_predictive_when_supplied():
    """When pred_var is supplied, var_total is the per-player predictive S."""
    conn = _conn_with_table()
    ks = _make_state()
    pred_var = np.array([0.123, 0.456])
    _write_kalman_trajectory(conn, ks, week=3, season=2025, pred_var=pred_var)
    rows = {
        r[0]: r[1]
        for r in conn.execute("SELECT player_id, var_total FROM kalman_trajectory")
    }
    assert rows["p1"] == pytest.approx(0.123)
    assert rows["p2"] == pytest.approx(0.456)
