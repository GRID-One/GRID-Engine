"""Tier 0.5 — golden master (roadmap Phase 0; validation plan §6).

Freezes the engine's output on a fixed synth seed to catch ANY drift — and,
crucially, MISLABELING (correct-looking numbers under the wrong semantic, the
motivating `matchup_grades` bug).  Three layers, most -> least robust:

  Layer A — schema & semantic invariants (exact, anchored to PLANTED TRUTH).
    Column presence, dtypes, shapes, and sign/relationship checks tied to the
    synth ground truth ("rating correlates positively with planted ability",
    "talent is the most persistent Kalman component", "the injury-return week
    dips and widens variance").  This layer CANNOT be gamed by regenerating the
    golden file, because it is anchored to truth, not to the last run.

  Layer B — ordering & membership invariants (exact ints/sets vs golden).
    Top-N player ranking, team ranking order, injury-dip location, played-week
    set.  Robust to float jitter, sensitive to logic changes.

  Layer C — numeric values (assert_allclose rtol 1e-5 / atol 1e-6 vs golden).
    Ratings, team ratings, weekly QB credit, focus-QB Kalman states.  Tolerances
    absorb cross-platform float jitter (golden generated on Linux/CI); failures
    print the largest-moving rows so they are actionable, not silenced.

Regenerate the golden file with ``python -m tests.grid.golden_master`` after an
intended engine change; the golden diff in the PR shows which players moved.
Anti-gaming backstop: Layer A + the Tier 0 recovery gates are truth-anchored and
must ALSO pass, so a regenerate cannot turn a real regression green.
"""
import numpy as np
import pytest

from tests.grid.golden_master import compute_snapshot, load_golden


@pytest.fixture(scope="module")
def live():
    """The engine's current output on the canonical synth seed."""
    return compute_snapshot()


@pytest.fixture(scope="module")
def golden():
    """The committed frozen snapshot."""
    return load_golden()


def _ac1(x):
    """Lag-1 autocorrelation of a 1-D series (persistence proxy)."""
    x = x - x.mean()
    return float(np.sum(x[1:] * x[:-1]) / np.sum(x * x))


# ============================================================ Layer A — schema
def test_layerA_schema_shapes_dtypes(live):
    """Engine output has the expected arrays, shapes, and dtypes."""
    assert live["rating"].shape == (288,)
    assert live["ability"].shape == (288,)
    assert live["team_rating"].shape == (12,)
    assert live["planted_team"].shape == (12,)
    assert live["qb_credit"].shape == live["qb_week"].shape
    for k in ("k_total_filt", "k_total_smooth", "k_tau_smooth",
              "k_var_total_filt", "k_total_pred", "k_var_total_pred"):
        assert live[k].shape == (14,), f"{k} wrong shape"
    assert live["rating"].dtype == np.float64
    assert live["qb_week"].dtype == np.int64
    assert live["player_id"].dtype.kind == "U"   # unicode strings, not object


# ============================================================ Layer A — semantics (truth-anchored)
def test_layerA_rating_recovers_ability_sign(live):
    """Estimated rating correlates POSITIVELY with planted ability, pooled and
    per position — the core attribution semantic (sign, not magnitude)."""
    assert np.corrcoef(live["rating"], live["ability"])[0, 1] > 0
    for pos in ("QB", "RB", "WR", "TE", "DEF"):
        mask = live["position"] == pos
        assert np.corrcoef(live["rating"][mask], live["ability"][mask])[0, 1] > 0, \
            f"{pos} rating anti-correlates with planted ability — mislabel/sign bug"


def test_layerA_team_rating_sign_and_top(live):
    """Team rating tracks planted strength; the strongest planted team lands in
    the estimated top 2 (market-reconciled)."""
    assert np.corrcoef(live["team_rating"], live["planted_team"])[0, 1] > 0
    est_order = live["team"][np.argsort(-live["team_rating"])]
    strongest_planted = live["team"][np.argmax(live["planted_team"])]
    assert strongest_planted in set(est_order[:2])


def test_layerA_talent_is_most_persistent_component(live):
    """Smoothed talent (tau) is MORE persistent than the total (tau+form+scheme),
    which carries the transient form — the labelling invariant for the Kalman
    decomposition (catches a talent/form swap)."""
    assert _ac1(live["k_tau_smooth"]) > _ac1(live["k_total_smooth"])


def test_layerA_injury_dip_and_variance(live):
    """The planted injury (missed weeks 8-9, return at index 9) dips the filtered
    current ability below the pre-injury peak and widens its variance."""
    filt, var = live["k_total_filt"], live["k_var_total_filt"]
    assert filt[9] < filt[6], "injury-return week did not dip below pre-injury peak"
    assert filt[9] < filt[10], "current ability did not recover after the return"
    assert var[9] > var[2], "injury-return week did not widen variance"
    # the focus QB missed weeks 8 and 9 (1-based) -> not played
    played_weeks = set((np.where(live["focus_played"])[0] + 1).tolist())
    assert 8 not in played_weeks and 9 not in played_weeks


# ============================================================ Layer B — ordering / membership
def test_layerB_top_player_ranking(live, golden):
    """Top-10 player_ids by rating (ordered) match the frozen golden ranking."""
    live_top = list(live["player_id"][np.argsort(-live["rating"])][:10])
    gold_top = list(golden["player_id"][np.argsort(-golden["rating"])][:10])
    assert live_top == gold_top


def test_layerB_team_ranking_order(live, golden):
    """Team ranking order by team_rating matches the frozen golden order."""
    live_order = list(live["team"][np.argsort(-live["team_rating"])])
    gold_order = list(golden["team"][np.argsort(-golden["team_rating"])])
    assert live_order == gold_order


def test_layerB_injury_dip_location_and_played_set(live, golden):
    """The steepest single-week drop in filtered ability lands at the same week
    as in golden (the injury return), and the played-week set is unchanged."""
    live_drop = int(np.argmin(np.diff(live["k_total_filt"])))
    gold_drop = int(np.argmin(np.diff(golden["k_total_filt"])))
    assert live_drop == gold_drop == 8   # drop INTO index 9 (week 10 return)
    assert np.array_equal(live["focus_played"], golden["focus_played"])


# ============================================================ Layer C — numeric freeze
def _assert_close_reporting(name, live, gold, ids=None, rtol=1e-5, atol=1e-6):
    """assert_allclose, but on failure surface the largest-moving rows."""
    live = np.asarray(live)
    gold = np.asarray(gold)
    if live.shape != gold.shape:
        pytest.fail(f"{name}: shape changed {gold.shape} -> {live.shape} "
                    "(regenerate golden if intended)")
    if not np.allclose(live, gold, rtol=rtol, atol=atol):
        diff = np.abs(live - gold)
        worst = np.argsort(-diff.ravel())[:5]
        lines = []
        for w in worst:
            label = ids[w] if ids is not None else w
            lines.append(f"    {label}: golden={gold.ravel()[w]:.6g} "
                         f"live={live.ravel()[w]:.6g} Δ={diff.ravel()[w]:.3g}")
        pytest.fail(f"{name} drifted beyond rtol={rtol}/atol={atol}; "
                    f"largest movers:\n" + "\n".join(lines) +
                    "\n(regenerate the golden file if this change is intended)")


def test_layerC_ordering_matches_golden(live, golden):
    """Canonical id/order arrays are byte-identical — the numeric arrays below
    are only comparable row-for-row if these align."""
    assert np.array_equal(live["player_id"], golden["player_id"])
    assert np.array_equal(live["position"], golden["position"])
    assert np.array_equal(live["team"], golden["team"])
    assert np.array_equal(live["qb_week"], golden["qb_week"])


def test_layerC_player_and_team_ratings(live, golden):
    """Player ratings, planted ability, and team ratings match golden numerics."""
    _assert_close_reporting("rating", live["rating"], golden["rating"], live["player_id"])
    _assert_close_reporting("ability", live["ability"], golden["ability"], live["player_id"])
    _assert_close_reporting("team_rating", live["team_rating"], golden["team_rating"], live["team"])


def test_layerC_qb_weekly_and_kalman(live, golden):
    """Weekly QB credit and the full focus-QB Kalman trajectory match golden."""
    _assert_close_reporting("qb_credit", live["qb_credit"], golden["qb_credit"])
    for k in ("k_total_filt", "k_total_smooth", "k_tau_smooth",
              "k_var_total_filt", "k_total_pred", "k_var_total_pred"):
        _assert_close_reporting(k, live[k], golden[k])
