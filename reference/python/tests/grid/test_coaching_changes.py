"""Tests for Task 10: coaching_changes seed table + scheme-reset wiring.

Named tests (per spec):
  - test_coaching_changes_schema_idempotent
  - test_scheme_reset_zeroes_scheme_only
  - test_scheme_reset_independent_of_intervention
  - test_weekly_update_applies_coaching_reset
"""
from __future__ import annotations
import os
import sqlite3
import numpy as np
import pandas as pd
import pytest

from backend.db.connection import init_db
from backend.grid.statespace import KalmanState, SSParams, kalman_step, N_STATE


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _make_db(tmp_path, suffix=""):
    """Create an isolated DB for tests, bypassing the module-level DB_PATH constant."""
    import sqlite3 as _sqlite3
    db_path = tmp_path / f"test{suffix}.db"
    conn = _sqlite3.connect(str(db_path), check_same_thread=False)
    conn.row_factory = _sqlite3.Row
    conn.execute("PRAGMA foreign_keys=ON")
    init_db(conn)
    return conn


def _table_exists(conn: sqlite3.Connection, name: str) -> bool:
    row = conn.execute(
        "SELECT name FROM sqlite_master WHERE type='table' AND name=?", (name,)
    ).fetchone()
    return row is not None


def _make_two_player_state(
    mu_p1=(0.5, 0.3, 0.2),
    mu_p2=(0.1, 0.05, 0.08),
) -> KalmanState:
    """Two-player KalmanState with known mu values for scheme-reset testing."""
    state = KalmanState.init(["p1", "p2"])
    state.mu[0] = mu_p1
    state.mu[1] = mu_p2
    # Set non-trivial sigma so cross-cov zeroing is detectable
    state.sigma[0] = np.array([
        [0.05, 0.01, 0.02],
        [0.01, 0.02, 0.01],
        [0.02, 0.01, 0.04],
    ])
    state.sigma[1] = np.array([
        [0.05, 0.01, 0.02],
        [0.01, 0.02, 0.01],
        [0.02, 0.01, 0.04],
    ])
    return state


# ---------------------------------------------------------------------------
# test_coaching_changes_schema_idempotent
# ---------------------------------------------------------------------------

def test_coaching_changes_schema_idempotent(tmp_path):
    """coaching_changes table is created by init_db and calling init_db again is safe."""
    conn = _make_db(tmp_path, "a")
    assert _table_exists(conn, "coaching_changes"), "coaching_changes table must exist after init_db"

    # Idempotent: calling init_db again must not error
    init_db(conn)
    assert _table_exists(conn, "coaching_changes"), "table must still exist after second init_db"

    # Schema smoke-check: insert a valid row
    conn.execute(
        """
        INSERT INTO coaching_changes (season, week_effective, team, role, old_name, new_name)
        VALUES (2025, 5, 'KC', 'HC', 'Old Coach', 'New Coach')
        """
    )
    conn.commit()

    # UNIQUE constraint: re-inserting same (season, week_effective, team, role) must fail
    with pytest.raises(sqlite3.IntegrityError):
        conn.execute(
            """
            INSERT INTO coaching_changes (season, week_effective, team, role, old_name, new_name)
            VALUES (2025, 5, 'KC', 'HC', 'Another Old', 'Another New')
            """
        )

    # role CHECK constraint: invalid role must fail
    with pytest.raises(sqlite3.IntegrityError):
        conn.execute(
            """
            INSERT INTO coaching_changes (season, week_effective, team, role, old_name, new_name)
            VALUES (2025, 6, 'KC', 'ST', 'Old', 'New')
            """
        )

    conn.close()


# ---------------------------------------------------------------------------
# test_scheme_reset_zeroes_scheme_only
# ---------------------------------------------------------------------------

def test_scheme_reset_zeroes_scheme_only():
    """Scheme reset on p1: p1 scheme_fit → 0 and sigma set; talent+form of p1 unchanged.
    ALL of p2 completely unchanged (no cross-player contamination).

    p1 has NO observation this step (NaN) so the Kalman update does not push
    scheme_fit away from 0; the post-step value is exactly the prior reset value.
    p2 HAS an observation, proving resets don't bleed across players.
    """
    state = _make_two_player_state()
    params = SSParams()

    # p1 has no observation; p2 does — isolates the reset effect cleanly
    obs = np.array([np.nan, 0.1])
    snaps = np.array([50.0, 50.0])

    # Run WITHOUT scheme_resets (baseline — p1 predict-only, p2 updated)
    baseline = kalman_step(state, obs, snaps=snaps, params=params)

    # Run WITH scheme_resets={p1}
    result = kalman_step(state, obs, snaps=snaps, params=params, scheme_resets={"p1"})

    # --- p1 scheme_fit mean must be zeroed (no update to move it) ---
    assert result.mu[0, 2] == pytest.approx(0.0, abs=1e-10), (
        f"p1 scheme_fit mean must be 0.0 after reset, got {result.mu[0, 2]}"
    )

    # --- p1 talent and form means must match baseline (predict-only, no observation) ---
    np.testing.assert_allclose(
        result.mu[0, :2], baseline.mu[0, :2], atol=1e-10,
        err_msg="p1 talent/form means must be identical to baseline after scheme reset"
    )

    # --- p1 sigma[2,2] must equal scheme_reset_var (no update inflating it further) ---
    assert result.sigma[0, 2, 2] == pytest.approx(params.scheme_reset_var, rel=1e-6), (
        f"p1 sigma[2,2] must equal scheme_reset_var={params.scheme_reset_var}"
    )

    # --- p1 scheme cross-covariances must be zero (zeroed in reset, no update) ---
    assert result.sigma[0, 2, 0] == pytest.approx(0.0, abs=1e-10), "p1 sigma[2,0] must be 0"
    assert result.sigma[0, 2, 1] == pytest.approx(0.0, abs=1e-10), "p1 sigma[2,1] must be 0"
    assert result.sigma[0, 0, 2] == pytest.approx(0.0, abs=1e-10), "p1 sigma[0,2] must be 0"
    assert result.sigma[0, 1, 2] == pytest.approx(0.0, abs=1e-10), "p1 sigma[1,2] must be 0"

    # --- ALL of p2 must be identical to baseline (no cross-player contamination) ---
    np.testing.assert_array_equal(
        result.mu[1], baseline.mu[1],
        err_msg="p2 mu must be completely unchanged by p1 scheme reset"
    )
    np.testing.assert_array_equal(
        result.sigma[1], baseline.sigma[1],
        err_msg="p2 sigma must be completely unchanged by p1 scheme reset"
    )


# ---------------------------------------------------------------------------
# test_scheme_reset_independent_of_intervention
# ---------------------------------------------------------------------------

def test_scheme_reset_independent_of_intervention():
    """scheme_resets and interventions are separate: combined effect is additive, not conflated.

    Uses NaN for p1 observation so results are analytically clean (predict-only for p1).
    """
    state = _make_two_player_state()
    params = SSParams()
    # p1: no observation (NaN) keeps results clean; p2: observed
    obs = np.array([np.nan, 0.1])
    snaps = np.array([50.0, 50.0])

    # p1 with only intervention
    res_intv_only = kalman_step(
        state, obs, snaps=snaps, params=params,
        interventions={"p1"}, scheme_resets=None,
    )

    # p1 with only scheme_reset
    res_reset_only = kalman_step(
        state, obs, snaps=snaps, params=params,
        interventions=None, scheme_resets={"p1"},
    )

    # p1 with both
    res_both = kalman_step(
        state, obs, snaps=snaps, params=params,
        interventions={"p1"}, scheme_resets={"p1"},
    )

    # --- scheme reset zeroes mu[i,2] (predict-only so stays at 0) ---
    assert res_reset_only.mu[0, 2] == pytest.approx(0.0, abs=1e-10), (
        "scheme-only reset: p1 scheme_fit mu must be 0 (no obs)"
    )
    assert res_both.mu[0, 2] == pytest.approx(0.0, abs=1e-10), (
        "both flags: p1 scheme_fit mu must be 0 (no obs)"
    )

    # --- intervention-only does NOT explicitly zero scheme_fit mean ---
    # (intervention spikes variance but scheme_fit mean follows the AR transition)
    # The predicted scheme_fit = phi_scheme * mu[0,2] = 0.985 * 0.2 ≈ 0.197 (non-zero)
    assert abs(res_intv_only.mu[0, 2]) > 1e-6, (
        "intervention-only must NOT zero the scheme_fit mean"
    )

    # --- scheme reset cross-covariances are zero (no obs → no update to inject them) ---
    assert res_reset_only.sigma[0, 2, 0] == pytest.approx(0.0, abs=1e-10), (
        "scheme-reset-only: sigma[p1,2,0] must be 0"
    )
    assert res_both.sigma[0, 2, 0] == pytest.approx(0.0, abs=1e-10), (
        "both flags: sigma[p1,2,0] must be 0"
    )

    # --- intervention-only does NOT zero the cross-covariances ---
    # The predict step propagates the non-zero off-diagonal from the state,
    # so sigma[p1,2,0] will be non-zero under intervention-only.
    # (state has sigma[0]=[2,0]=0.02 going in; F @ sigma @ F.T preserves the off-diagonal)
    assert abs(res_intv_only.sigma[0, 2, 0]) > 1e-6, (
        "intervention-only: sigma[p1,2,0] must NOT be zeroed"
    )

    # --- p2 is always untouched ---
    baseline = kalman_step(state, obs, snaps=snaps, params=params)
    np.testing.assert_array_equal(res_both.mu[1], baseline.mu[1])
    np.testing.assert_array_equal(res_both.sigma[1], baseline.sigma[1])


# ---------------------------------------------------------------------------
# test_weekly_update_applies_coaching_reset
# ---------------------------------------------------------------------------

def _make_plays_with_teams(n=40, seed=42):
    """Make plays with KC and BUF teams."""
    rng = np.random.default_rng(seed)
    teams = ["KC", "BUF"]
    off_teams = [teams[i % 2] for i in range(n)]
    def_teams = [teams[(i + 1) % 2] for i in range(n)]

    # 6 players: P0-P2 on KC, P3-P5 on BUF
    kc_pool = ["P0", "P1", "P2"]
    buf_pool = ["P3", "P4", "P5"]

    off_players = [
        list(rng.choice(kc_pool if off_teams[i] == "KC" else buf_pool, size=2, replace=False))
        for i in range(n)
    ]
    def_players = [
        list(rng.choice(buf_pool if off_teams[i] == "KC" else kc_pool, size=2, replace=False))
        for i in range(n)
    ]
    return pd.DataFrame({
        "off_team": off_teams,
        "def_team": def_teams,
        "off_players": off_players,
        "def_players": def_players,
        "dv": rng.normal(scale=0.5, size=n),
        "week": [5] * n,
    })


def _make_players_with_kc():
    """Make players: P0=KC/QB, P1=KC/RB, P2=KC/WR, P3=BUF/QB, P4=BUF/RB, P5=BUF/WR."""
    return pd.DataFrame({
        "player_id": ["P0", "P1", "P2", "P3", "P4", "P5"],
        "team":      ["KC",  "KC",  "KC",  "BUF", "BUF", "BUF"],
        "position":  ["QB",  "RB",  "WR",  "QB",  "RB",  "WR"],
        "is_starter": [True] * 6,
        "ability": np.zeros(6),
    })


def test_weekly_update_applies_coaching_reset(tmp_path, monkeypatch):
    """Seed an HC change for team KC at week 5; after run(), team KC's skill-position
    players should have scheme_fit (mu[:,2]) reset to 0 in the saved KalmanState.

    We verify P0 (KC/QB) specifically. BUF players must NOT be reset.
    """
    import backend.grid.layers as layers_mod
    import backend.grid.statespace as ss_mod
    from backend.db.seed_coaching import seed_coaching_changes

    kal_path = tmp_path / "kal.npz"
    monkeypatch.setattr(layers_mod, "_ACCUM_PATH", tmp_path / "acc.npz")
    monkeypatch.setattr(ss_mod, "_KALMAN_PATH", kal_path)
    monkeypatch.setattr("backend.pipeline.weekly_update.run_valuations",
                        lambda **kwargs: None)
    monkeypatch.setattr("backend.pipeline.weekly_update.write_health",
                        lambda **kwargs: None)

    import sqlite3 as _sqlite3
    conn = _sqlite3.connect(str(tmp_path / "coach_test.db"), check_same_thread=False)
    conn.row_factory = _sqlite3.Row
    conn.execute("PRAGMA foreign_keys=ON")
    init_db(conn)

    # Seed an HC change for KC at week 5
    seed_coaching_changes(conn=conn, rows=[
        {"season": 2025, "week_effective": 5, "team": "KC", "role": "HC",
         "old_name": "Old Coach", "new_name": "New Coach"},
    ])

    plays_df = _make_plays_with_teams(n=40, seed=42)
    players_df = _make_players_with_kc()

    # Pre-seed a KalmanState with non-zero scheme_fit for all players
    player_ids_sorted = sorted(players_df["player_id"].tolist())
    ks_prior = KalmanState.init(player_ids_sorted)
    ks_prior.mu[:, 2] = 0.5   # give everyone a non-zero scheme_fit going in
    ks_prior.save()

    from backend.pipeline.weekly_update import run
    result = run(
        week=5, season=2025,
        plays_df=plays_df, players_df=players_df,
        _conn=conn,
    )
    conn.close()

    # Load the saved Kalman state
    ks = KalmanState.load()
    assert ks is not None, "KalmanState must be saved after run()"

    pid_to_idx = {pid: i for i, pid in enumerate(ks.player_ids)}

    # KC skill-position players (P0/QB, P1/RB, P2/WR) must have scheme_fit near 0.
    # Prior was reset to 0.0 before the Kalman update.  Even after a Kalman update
    # the posterior scheme_fit is K[2]*(obs - prior_total), which is very small
    # because scheme_reset_var (0.04) << r_scale (0.35-0.40), so K[2] ≈ 0.1.
    # Threshold: |scheme_fit| < 0.05 captures the reset effect robustly.
    kc_skill_pids = ["P0", "P1", "P2"]
    buf_pids = ["P3", "P4", "P5"]

    for pid in kc_skill_pids:
        if pid not in pid_to_idx:
            continue
        i = pid_to_idx[pid]
        assert abs(ks.mu[i, 2]) < 0.05, (
            f"KC player {pid} scheme_fit must be near 0 after HC coaching change reset; "
            f"got {ks.mu[i, 2]:.4f} (prior was reset to 0, should be small post-update)"
        )

    # BUF players must NOT be reset: prior was 0.5 → after predict 0.5*phi_scheme ≈ 0.49
    # → posterior is close to that, definitely not near 0.
    for pid in buf_pids:
        if pid not in pid_to_idx:
            continue
        i = pid_to_idx[pid]
        assert abs(ks.mu[i, 2]) > 0.1, (
            f"BUF player {pid} scheme_fit must NOT be zeroed (no coaching change); "
            f"got {ks.mu[i, 2]:.4f} (prior was 0.5, no reset should keep it large)"
        )
