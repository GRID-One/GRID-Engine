"""Phase 0 engine prerequisites (roadmap §Phase 0 / validation plan §7, §8).

Covers three additive engine changes that the validation suite depends on:

  1. kalman_two_component surfaces the one-step-ahead predictive (mean, S) — the
     honest forecast distribution calibration must score against (S includes R,
     unlike the post-update filtered variance).
  2. kalman_step(return_pred=True) surfaces the per-player predictive variance so
     the trajectory write path can persist the honest band.
  3. The engine cache paths (_KALMAN_PATH, _ACCUM_PATH) are injectable so backtest
     / fold code never reads or writes the production data/cache/ namespace.
"""
import numpy as np
import pytest

from backend.grid.statespace import (
    SSParams, KalmanState, kalman_two_component, kalman_step, N_STATE,
)
from backend.grid import layers as layers_mod


# --------------------------------------------------------------------------- 1
def _benign_series(W=20, seed=3):
    """Build a benign (always-played, constant-snaps) weekly series for filter tests."""
    rng = np.random.default_rng(seed)
    y = rng.normal(0.4, 0.15, W)
    snaps = np.full(W, 40.0)
    played = np.ones(W, dtype=bool)
    return y, snaps, played


def test_predictive_keys_present_and_shaped():
    """total_pred/var_total_pred are returned, length-W, finite, and positive."""
    y, snaps, played = _benign_series(W=15)
    out = kalman_two_component(y, snaps, played)
    assert "total_pred" in out and "var_total_pred" in out
    assert out["total_pred"].shape == (15,)
    assert out["var_total_pred"].shape == (15,)
    assert np.all(np.isfinite(out["total_pred"]))
    assert np.all(out["var_total_pred"] > 0)


def test_predictive_variance_wider_than_filtered():
    """S = HPpHᵀ + R is the pre-update variance plus observation noise; it must be
    strictly larger than the post-update FILTERED variance every played week —
    that gap is exactly the overconfidence the old code shipped."""
    y, snaps, played = _benign_series(W=20)
    out = kalman_two_component(y, snaps, played)
    assert np.all(out["var_total_pred"] > out["var_total_filt"])


# Frozen baseline of the LEGACY filtered/smoothed keys for the benign series
# (seed=11, W=18, interventions={6}), captured BEFORE the predictive (mean, S)
# was surfaced.  Asserting against these proves the change is purely additive —
# a regression in any legacy key fails here rather than passing silently.
_LEGACY_BASELINE = {
    "total_filt": [
        0.42061101595931666, 0.523892031174438, 0.553320279951515,
        0.45723133021852447, 0.4195940410885371, 0.38496891573430225,
        0.4593898331893394, 0.4276498168683128, 0.47366462389575165,
        0.3121606263541181, 0.45375121192500917, 0.42284078216873655,
        0.4555668574228764, 0.4225093088762103, 0.38957503127921805,
        0.424168610027123, 0.46533284581425965, 0.4237286112966822,
    ],
    "var_total_filt": [
        0.008769348995874246, 0.005517578387602277, 0.004650151179406237,
        0.004208869058411627, 0.003943851159761817, 0.0037845353154556467,
        0.014557370417246451, 0.009129922077460113, 0.005476299221190972,
        0.004581330069744502, 0.004291866877461487, 0.0041987561801524395,
        0.0041769877328639675, 0.004178623543285493, 0.004184423437954295,
        0.004187415868323557, 0.004185869120514861, 0.004180078732946369,
    ],
    "total_smooth": [
        0.46407580319856806, 0.4816589429484314, 0.4644518046591827,
        0.4163841769310605, 0.3971556200523966, 0.38757408636790786,
        0.43063249125618724, 0.42241425142144984, 0.42052816195219933,
        0.3850812422209551, 0.4426673264815444, 0.43020503010934985,
        0.43620684607824894, 0.417841558205567, 0.4134234170051259,
        0.4349533938624515, 0.443895568829139, 0.4237286112966822,
    ],
    "tau_smooth": [
        0.4617247579988954, 0.4708018122006664, 0.45881894734673095,
        0.4311060405712975, 0.41740812093239205, 0.4095763790159394,
        0.41272166627658907, 0.4091132959932847, 0.4080790434461195,
        0.3892836536074327, 0.42280271123646473, 0.4170573204265324,
        0.42082728596364777, 0.4101855932453114, 0.4079508030063186,
        0.421268833286718, 0.426924468713446, 0.41509675410271246,
    ],
    "var_tau_smooth": [
        0.011371143742671182, 0.010746511365615996, 0.01048204215402265,
        0.010446063507186005, 0.010625600701528591, 0.011107563606935131,
        0.014757399275491268, 0.01575917832211361, 0.016582207467874404,
        0.0172401200120592, 0.017702995246119662, 0.017988657696428073,
        0.01813764405159316, 0.018200904458756977, 0.01823907750233344,
        0.01833645126720706, 0.018642217211020922, 0.019479009818072702,
    ],
}


def test_predictive_does_not_change_the_filter():
    """Surfacing the predictive is purely additive: every pre-existing
    filtered/smoothed output is unchanged vs a frozen baseline (so a regression
    in the legacy keys is caught), and the new predictive is self-consistent."""
    y, snaps, played = _benign_series(W=18, seed=11)
    intv = {6}
    out = kalman_two_component(y, snaps, played, interventions=intv)

    # Legacy keys must match the frozen pre-change baseline exactly.
    for key, expected in _LEGACY_BASELINE.items():
        np.testing.assert_allclose(
            out[key], expected, rtol=1e-12, atol=0,
            err_msg=f"legacy key {key!r} drifted — the predictive change is not additive",
        )

    # And the surfaced predictive is internally consistent: standardised
    # innovation z = (y - mean) / sqrt(S) is finite and order-1 on benign data.
    z = (y - out["total_pred"]) / np.sqrt(out["var_total_pred"])
    assert np.all(np.isfinite(z))
    assert np.mean(z ** 2) < 5.0  # loose sanity bound, not a calibration gate


def test_predictive_defined_on_missing_weeks():
    """Missing weeks (did-not-play) still get a finite predictive — snaps floor
    supplies R — so the forecast distribution is defined every week."""
    W = 12
    y = np.full(W, 0.3)
    snaps = np.full(W, 30.0)
    played = np.ones(W, dtype=bool)
    played[5:8] = False
    snaps[5:8] = 0.0
    out = kalman_two_component(y, snaps, played)
    assert np.all(np.isfinite(out["var_total_pred"]))
    assert np.all(out["var_total_pred"] > 0)


# --------------------------------------------------------------------------- 2
def test_kalman_step_return_pred_tuple():
    """return_pred=True yields (state, pred_var) with a finite, positive variance
    for every player — including the one with a NaN observation."""
    state = KalmanState.init(["p1", "p2", "p3"])
    obs = np.array([0.2, np.nan, -0.1])
    snaps = np.array([20.0, 0.0, 35.0])
    updated, pred_var = kalman_step(state, obs, snaps, return_pred=True)
    assert isinstance(updated, KalmanState)
    assert pred_var.shape == (3,)
    # Predictive variance is defined for EVERY player, including the NaN-obs one.
    assert np.all(np.isfinite(pred_var))
    assert np.all(pred_var > 0)


def test_kalman_step_default_return_unchanged():
    """Default (return_pred=False) still returns a bare KalmanState — backward
    compatible with all existing callers."""
    state = KalmanState.init(["p1", "p2"])
    obs = np.array([0.1, 0.2])
    out = kalman_step(state, obs)
    assert isinstance(out, KalmanState)
    assert not isinstance(out, tuple)


# --------------------------------------------------------------------------- 3
def test_kalman_state_path_injectable_and_isolates_default(tmp_path, monkeypatch):
    """Saving to an explicit path round-trips and never touches the module
    default — proving backtest mode can confine writes to a scratch namespace."""
    import backend.grid.statespace as ss
    sentinel = tmp_path / "default_must_not_be_written.npz"
    monkeypatch.setattr(ss, "_KALMAN_PATH", sentinel)

    ks = KalmanState.init(["a", "b"])
    ks.mu[0, 0] = 0.7
    target = tmp_path / "fold" / "kal.npz"
    ks.save(target)

    assert target.exists()
    assert not sentinel.exists(), "explicit path must not write the module default"

    loaded = KalmanState.load(target)
    assert loaded is not None
    assert loaded.player_ids == ["a", "b"]
    np.testing.assert_allclose(loaded.mu, ks.mu)
    # And the default loader still reads nothing when the default is absent.
    assert KalmanState.load() is None


def test_accumulator_path_injectable_and_isolates_default(tmp_path, monkeypatch):
    """save/load_accumulators round-trip through an explicit base_path and never
    write the module default."""
    monkeypatch.setattr(layers_mod, "_ACCUM_PATH", tmp_path / "default_acc.npz")

    XtX, Xty = layers_mod.init_accumulators(3)
    XtX[0, 0] = 2.0
    Xty[1] = 1.5
    base = tmp_path / "fold" / "acc.npz"
    layers_mod.save_accumulators(XtX, Xty, week=4, player_order=["a", "b", "c"], base_path=base)

    assert base.exists()
    assert not (tmp_path / "default_acc.npz").exists(), "explicit base_path must not write the default"

    loaded = layers_mod.load_accumulators(base_path=base)
    assert loaded is not None
    rXtX, rXty, last_week, order = loaded
    np.testing.assert_allclose(rXtX, XtX)
    np.testing.assert_allclose(rXty, Xty)
    assert last_week == 4
    assert order == ["a", "b", "c"]


def test_accum_path_default_naming_unchanged(monkeypatch, tmp_path):
    """The 'all' key resolves to the base path and situation keys produce the
    documented sibling name — default behaviour byte-identical to before."""
    base = tmp_path / "rapm_accumulators.npz"
    monkeypatch.setattr(layers_mod, "_ACCUM_PATH", base)
    assert layers_mod._accum_path("all") == base
    assert layers_mod._accum_path("red_zone") == base.with_name("rapm_accumulators_red_zone.npz")
    # explicit base_path overrides the module default
    other = tmp_path / "scratch" / "acc.npz"
    assert layers_mod._accum_path("all", base_path=other) == other
    assert layers_mod._accum_path("two_minute", base_path=other) == other.with_name("acc_two_minute.npz")
