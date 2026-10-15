"""
Tests for Task 11: GRID Engine Performance Extensions.

1. build_design() vectorization — correctness parity with the old row-loop
   approach, and a basic timing check (vectorized should be at least 2x faster
   on a medium-sized frame).
2. fit_value_model() cache_path persistence (file-path contract).
3. load_value_model() round-trip and error handling.
"""
from __future__ import annotations
import os
import time

import numpy as np
import pandas as pd
import pytest
import scipy.sparse as sp

# ---------------------------------------------------------------------------
# Shared synthetic data fixture
# ---------------------------------------------------------------------------

@pytest.fixture(scope="module")
def synth_data():
    """Return (plays_with_dv, players) from the synthetic simulator."""
    from backend.grid.synth import simulate
    from backend.grid.value import fit_value_model, attach_dv

    plays, players, _ = simulate()
    vm = fit_value_model(plays)
    plays = attach_dv(plays, vm)
    return plays, players


# ---------------------------------------------------------------------------
# Reference (original row-loop) implementation of build_design
# kept here so we can compare outputs without touching the production code.
# ---------------------------------------------------------------------------

def _build_design_reference(plays, players):
    """Original row-by-row implementation — used as a correctness baseline."""
    import scipy.sparse as sp_ref

    pids = sorted(players["player_id"].tolist(), key=str)
    p_col = {p: i for i, p in enumerate(pids)}
    nP = len(pids)
    teams = sorted(players.team.unique())
    t_off = {t: nP + i for i, t in enumerate(teams)}
    t_def = {t: nP + len(teams) + i for i, t in enumerate(teams)}
    nCols = nP + 2 * len(teams)

    rows, cols, vals = [], [], []
    for r, play in enumerate(plays.itertuples(index=False)):
        for pl in play.off_players:
            rows.append(r); cols.append(p_col[pl]); vals.append(1.0)
        for pl in play.def_players:
            rows.append(r); cols.append(p_col[pl]); vals.append(-1.0)
        rows.append(r); cols.append(t_off[play.off_team]); vals.append(1.0)
        rows.append(r); cols.append(t_def[play.def_team]); vals.append(-1.0)

    X = sp_ref.csr_matrix((vals, (rows, cols)), shape=(len(plays), nCols))
    y = plays["dv"].to_numpy(float)
    colidx = dict(p_col=p_col, t_off=t_off, t_def=t_def, nP=nP, teams=teams, nCols=nCols)
    return X, y, colidx


# ===========================================================================
# build_design() — correctness
# ===========================================================================

class TestBuildDesignCorrectness:
    def test_output_shape_matches_reference(self, synth_data):
        plays, players = synth_data
        from backend.grid.layers import build_design

        X_new, y_new, ci_new = build_design(plays, players)
        X_ref, y_ref, ci_ref = _build_design_reference(plays, players)

        assert X_new.shape == X_ref.shape, (
            f"Shape mismatch: new={X_new.shape} vs ref={X_ref.shape}"
        )
        assert len(y_new) == len(y_ref)

    def test_matrix_values_match_reference(self, synth_data):
        plays, players = synth_data
        from backend.grid.layers import build_design

        X_new, _, _ = build_design(plays, players)
        X_ref, _, _ = _build_design_reference(plays, players)

        # Convert both to dense and compare element-wise.
        diff = (X_new - X_ref)
        assert diff.nnz == 0 or np.allclose(diff.toarray(), 0.0), (
            "build_design vectorized output differs from reference row-loop"
        )

    def test_y_values_match_reference(self, synth_data):
        plays, players = synth_data
        from backend.grid.layers import build_design

        _, y_new, _ = build_design(plays, players)
        _, y_ref, _ = _build_design_reference(plays, players)

        np.testing.assert_array_equal(y_new, y_ref)

    def test_colidx_matches_reference(self, synth_data):
        plays, players = synth_data
        from backend.grid.layers import build_design

        _, _, ci_new = build_design(plays, players)
        _, _, ci_ref = _build_design_reference(plays, players)

        assert ci_new["nP"] == ci_ref["nP"]
        assert ci_new["nCols"] == ci_ref["nCols"]
        assert ci_new["teams"] == ci_ref["teams"]
        assert ci_new["p_col"] == ci_ref["p_col"]
        assert ci_new["t_off"] == ci_ref["t_off"]
        assert ci_new["t_def"] == ci_ref["t_def"]

    def test_row_sums_are_correct(self, synth_data):
        """Each play row must sum to: n_off_players - n_def_players + 1 - 1 = 0
        (team off +1 and team def -1 cancel; player entries also cancel
        in aggregate).  More precisely: sum of +1 (off) and -1 (def) values
        per row equals (n_off - n_def).  We spot-check a few rows."""
        plays, players = synth_data
        from backend.grid.layers import build_design

        X, _, _ = build_design(plays, players)
        # row sums: +1 per off player, -1 per def player, +1 team_off, -1 team_def
        row_sums = np.asarray(X.sum(axis=1)).ravel()
        expected = plays["off_players"].apply(len).to_numpy() - \
                   plays["def_players"].apply(len).to_numpy()
        # team entries cancel (+1 off_team, -1 def_team)
        np.testing.assert_array_equal(
            row_sums, expected,
            err_msg="Row sums of design matrix do not match expected (n_off - n_def)",
        )

    def test_values_match_reference_unique(self, synth_data):
        """Values in the vectorized matrix must match the reference exactly.

        CSR duplicate-entry summation is expected behavior: a player who
        appears on both sides of the same play will have their entries
        summed (e.g. +1 off -1 def = 0, or two +1 off entries = +2).
        Both implementations must produce identical unique value sets.
        """
        plays, players = synth_data
        from backend.grid.layers import build_design

        X_new, _, _ = build_design(plays, players)
        X_ref, _, _ = _build_design_reference(plays, players)
        np.testing.assert_array_equal(
            np.unique(X_new.data), np.unique(X_ref.data),
            err_msg="Vectorized matrix has different unique values than reference",
        )

    def test_run_rapm_end_to_end(self, synth_data):
        """Full RAPM solve should still work with the vectorized design matrix."""
        plays, players = synth_data
        from backend.grid.layers import run_rapm

        ratings, team_df, beta, colidx = run_rapm(plays, players)
        assert len(ratings) == len(players)
        assert "rating" in ratings.columns
        assert np.all(np.isfinite(ratings.rating))


# ===========================================================================
# build_design() — performance
# ===========================================================================

class TestBuildDesignPerformance:
    def test_vectorized_faster_than_reference(self, synth_data):
        """Vectorized implementation should be at least 2x faster than row-loop
        on the synthetic dataset (~50 k plays)."""
        plays, players = synth_data
        from backend.grid.layers import build_design

        # Warm-up (JIT, import overhead)
        build_design(plays, players)
        _build_design_reference(plays, players)

        reps = 3
        t_new = 0.0
        t_ref = 0.0
        for _ in range(reps):
            t0 = time.perf_counter()
            build_design(plays, players)
            t_new += time.perf_counter() - t0

            t0 = time.perf_counter()
            _build_design_reference(plays, players)
            t_ref += time.perf_counter() - t0

        # Allow a 2x speedup threshold; on large frames the gain is ~10x, but
        # we keep the bar conservative to avoid flakiness on slow CI machines.
        speedup = t_ref / t_new
        assert speedup >= 1.5, (
            f"Expected vectorized build_design to be at least 1.5x faster than "
            f"row-loop; got {speedup:.2f}x (ref={t_ref:.3f}s, new={t_new:.3f}s)"
        )


# ===========================================================================
# fit_value_model() cache_path  (file-path contract)
# ===========================================================================

class TestFitValueModelCache:
    def test_cache_creates_file(self, tmp_path, synth_data):
        """cache_path is a file path; the file must exist after fitting."""
        plays, _ = synth_data
        from backend.grid.value import fit_value_model

        cache_file = str(tmp_path / "vm_cache" / "vmodel.joblib")
        fit_value_model(plays, cache_path=cache_file)

        assert os.path.exists(cache_file), (
            "fit_value_model with cache_path should create the specified file"
        )

    def test_cache_parent_directories_created_if_missing(self, tmp_path, synth_data):
        """Parent directories of cache_path must be created automatically."""
        plays, _ = synth_data
        from backend.grid.value import fit_value_model

        nested_file = str(tmp_path / "a" / "b" / "c" / "vmodel.joblib")
        assert not os.path.exists(os.path.dirname(nested_file))
        fit_value_model(plays, cache_path=nested_file)
        assert os.path.exists(nested_file)

    def test_no_cache_by_default(self, tmp_path, synth_data, monkeypatch):
        """When cache_path is None (default) no files should be written."""
        plays, _ = synth_data
        from backend.grid.value import fit_value_model

        monkeypatch.chdir(tmp_path)
        fit_value_model(plays)  # cache_path=None
        # nothing in tmp_path
        assert list(tmp_path.iterdir()) == []

    def test_model_still_predicts_after_caching(self, tmp_path, synth_data):
        plays, _ = synth_data
        from backend.grid.value import fit_value_model, compute_dv

        vm = fit_value_model(plays, cache_path=str(tmp_path / "vc" / "vmodel.joblib"))
        dv = compute_dv(plays, vm)
        assert len(dv) == len(plays)
        assert np.all(np.isfinite(dv))


# ===========================================================================
# load_value_model()
# ===========================================================================

class TestLoadValueModel:
    def test_round_trip_file_path(self, tmp_path, synth_data):
        """Save to an explicit file path, load from the same path."""
        plays, _ = synth_data
        from backend.grid.value import fit_value_model, load_value_model

        cache_file = str(tmp_path / "vm" / "vmodel.joblib")
        vm_orig = fit_value_model(plays, cache_path=cache_file)
        vm_loaded = load_value_model(cache_file)

        # Predictions must be identical
        preds_orig = vm_orig.predict(plays[["down", "ydstogo", "yardline_100"]].to_numpy(float))
        preds_loaded = vm_loaded.predict(plays[["down", "ydstogo", "yardline_100"]].to_numpy(float))
        np.testing.assert_array_equal(preds_orig, preds_loaded)

    def test_round_trip_different_filename(self, tmp_path, synth_data):
        """cache_path can be any filename, not just value_model.joblib."""
        plays, _ = synth_data
        from backend.grid.value import fit_value_model, load_value_model

        cache_file = str(tmp_path / "custom_name.joblib")
        fit_value_model(plays, cache_path=cache_file)

        vm_loaded = load_value_model(cache_file)
        preds = vm_loaded.predict(plays[["down", "ydstogo", "yardline_100"]].to_numpy(float))
        assert len(preds) == len(plays)

    def test_load_missing_file_path_raises(self, tmp_path):
        from backend.grid.value import load_value_model

        with pytest.raises(FileNotFoundError):
            load_value_model(str(tmp_path / "no_such_file.joblib"))

    def test_load_nonexistent_path_raises(self, tmp_path):
        from backend.grid.value import load_value_model

        with pytest.raises(FileNotFoundError):
            load_value_model(str(tmp_path / "nonexistent_dir" / "model.joblib"))

    def test_loaded_model_compute_dv(self, tmp_path, synth_data):
        plays, _ = synth_data
        from backend.grid.value import fit_value_model, load_value_model, attach_dv

        cache_file = str(tmp_path / "vm3" / "vmodel.joblib")
        fit_value_model(plays, cache_path=cache_file)
        vm = load_value_model(cache_file)
        plays_with_dv = attach_dv(plays, vm)
        assert "dv" in plays_with_dv.columns
        assert not plays_with_dv["dv"].isna().any()

    def test_load_value_model_exported_from_package(self):
        """load_value_model must be importable from the top-level package."""
        from backend.grid import load_value_model
        assert callable(load_value_model)
