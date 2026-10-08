import pytest


def test_grid_package_imports():
    from backend.grid import (
        simulate, make_college, SynthConfig,
        fit_value_model, compute_dv, attach_dv, STATE_COLS,
        build_design, run_rapm, layer1_qb_weekly, fit,
        kalman_two_component, SSParams,
        estimate_equivalency, build_priors, washout_table, PRIOR_SD,
        load_synthetic, REAL_LOADERS,
    )
    assert callable(simulate)
    assert callable(fit_value_model)
    assert callable(run_rapm)
    assert callable(kalman_two_component)
    assert callable(load_synthetic)


def test_grid_synthetic_pipeline():
    from backend.grid import load_synthetic, fit_value_model, attach_dv
    data = load_synthetic()
    assert "plays" in data
    assert "players" in data
    vm = fit_value_model(data["plays"])
    plays = attach_dv(data["plays"], vm)
    assert "dv" in plays.columns
    assert len(plays) > 0
