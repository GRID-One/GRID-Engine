"""Tests for the H2 lineup simulation (Phase 2c; validation plan §9, §11).

Small inline rosters/projections; checks the §8 constraint (one slot-filling
rule, only the projection source differs), the margin's sign on planted
better/identical projections, and the starter-OUT subset.
"""
import pytest

from backend.validation import lineup_sim as L


# --------------------------------------------------------------------------- #
# Fixtures                                                                     #
# --------------------------------------------------------------------------- #
SLOTS = {"QB": 1, "RB": 2, "WR": 1, "FLEX": 1}


def _pool(n_per_pos=12):
    """A deep pool: projected_points decreasing within each position.

    Depth matters for the fills-starting-slots guarantee: FLEX can consume up
    to one extra RB/WR/TE per team, so per-position supply must cover the
    worst case ``num_teams * (dedicated + FLEX)`` (= 12 RBs at 4 teams here).
    """
    players = []
    for pos in ("QB", "RB", "WR", "TE"):
        for i in range(n_per_pos):
            players.append({
                "player_id": f"{pos}{i}",
                "position": pos,
                "projected_points": 200.0 - 10.0 * i + {"QB": 100, "RB": 50, "WR": 40, "TE": 0}[pos],
            })
    return players


def _roster():
    """One hand-built roster: starters + bench at each position."""
    return [
        {"player_id": "QB_A", "position": "QB"},
        {"player_id": "QB_B", "position": "QB"},
        {"player_id": "RB_A", "position": "RB"},
        {"player_id": "RB_B", "position": "RB"},
        {"player_id": "RB_C", "position": "RB"},
        {"player_id": "WR_A", "position": "WR"},
        {"player_id": "WR_B", "position": "WR"},
    ]


# --------------------------------------------------------------------------- #
# draft_rosters                                                                #
# --------------------------------------------------------------------------- #
def test_draft_partitions_pool():
    """Rosters are disjoint, each num_rounds deep — no player drafted twice."""
    rosters = L.draft_rosters(_pool(), roster_slots=SLOTS, num_teams=4, num_rounds=6)
    assert len(rosters) == 4
    ids = [p["player_id"] for r in rosters for p in r]
    assert len(ids) == len(set(ids)) == 4 * 6


def test_draft_fills_starting_slots():
    """With a deep pool every team covers its dedicated starting slots."""
    rosters = L.draft_rosters(_pool(), roster_slots=SLOTS, num_teams=4, num_rounds=6)
    for roster in rosters:
        by_pos: dict = {}
        for p in roster:
            by_pos[p["position"]] = by_pos.get(p["position"], 0) + 1
        for pos, need in SLOTS.items():
            if pos == "FLEX":
                continue
            assert by_pos.get(pos, 0) >= need


def test_flex_only_te_is_draftable_and_starts_in_flex():
    """A high-projection TE has no dedicated slot in SLOTS, but ranks via the
    FLEX replacement level — it must be drafted and started in the FLEX
    (regression: flex-only positions used to get vor=0.0 and never rank)."""
    pool = [
        {"player_id": "TE_ELITE", "position": "TE", "projected_points": 260.0},
        {"player_id": "TE_LOW", "position": "TE", "projected_points": 100.0},
        {"player_id": "QB0", "position": "QB", "projected_points": 300.0},
        {"player_id": "QB1", "position": "QB", "projected_points": 290.0},
        {"player_id": "RB0", "position": "RB", "projected_points": 250.0},
        {"player_id": "RB1", "position": "RB", "projected_points": 240.0},
        {"player_id": "RB2", "position": "RB", "projected_points": 230.0},
        {"player_id": "RB3", "position": "RB", "projected_points": 220.0},
        {"player_id": "WR0", "position": "WR", "projected_points": 210.0},
        {"player_id": "WR1", "position": "WR", "projected_points": 200.0},
    ]
    rosters = L.draft_rosters(pool, roster_slots=SLOTS, num_teams=2, num_rounds=5)
    drafted = {p["player_id"] for r in rosters for p in r}
    assert "TE_ELITE" in drafted                 # ranked, not zeroed out
    team = next(r for r in rosters
                if any(p["player_id"] == "TE_ELITE" for p in r))
    proj = {p["player_id"]: p["projected_points"] for p in pool}
    starters = L.set_lineup(team, proj, SLOTS)
    assert "TE_ELITE" in starters                # started, via the FLEX


def test_draft_is_deterministic():
    """Same pool → identical rosters (no mock-draft randomness)."""
    a = L.draft_rosters(_pool(), roster_slots=SLOTS, num_teams=4, num_rounds=6)
    b = L.draft_rosters(_pool(), roster_slots=SLOTS, num_teams=4, num_rounds=6)
    assert [[p["player_id"] for p in r] for r in a] == \
           [[p["player_id"] for p in r] for r in b]


# --------------------------------------------------------------------------- #
# set_lineup                                                                   #
# --------------------------------------------------------------------------- #
def test_lineup_starts_highest_projected_per_slot():
    """Each dedicated slot gets the best projected player; FLEX the next-best eligible."""
    proj = {"QB_A": 20, "QB_B": 15, "RB_A": 18, "RB_B": 12, "RB_C": 10,
            "WR_A": 14, "WR_B": 9}
    starters = L.set_lineup(_roster(), proj, SLOTS)
    # QB_A over QB_B; RB_A+RB_B in the two RB slots; WR_A; RB_C (10) beats WR_B (9) for FLEX
    assert set(starters) == {"QB_A", "RB_A", "RB_B", "WR_A", "RB_C"}


def test_lineup_excludes_out_players():
    """An OUT starter is replaced by the next-best projected at the position."""
    proj = {"QB_A": 20, "QB_B": 15, "RB_A": 18, "RB_B": 12, "RB_C": 10,
            "WR_A": 14, "WR_B": 9}
    starters = L.set_lineup(_roster(), proj, SLOTS, out={"RB_A"})
    assert "RB_A" not in starters
    assert {"RB_B", "RB_C"} <= set(starters)   # both remaining RBs now start
    assert "WR_B" in starters                  # WR_B inherits the FLEX


def test_lineup_missing_projection_scores_zero_but_stays_eligible():
    """A player absent from the projection is benched behind any positive number,
    yet still fills an otherwise-empty slot."""
    proj = {"QB_B": 1.0}                        # QB_A unprojected
    starters = L.set_lineup(_roster(), proj, SLOTS)
    assert "QB_B" in starters and "QB_A" not in starters
    # RB/WR/FLEX slots still filled even though nobody there is projected
    assert len(starters) == 5


def test_lineup_deterministic_tie_break():
    """Equal projections break ties by player_id — same inputs, same lineup."""
    proj = {p["player_id"]: 10.0 for p in _roster()}
    a = L.set_lineup(_roster(), proj, SLOTS)
    b = L.set_lineup(_roster(), proj, SLOTS)
    assert a == b
    assert "QB_A" in a                          # id-ordered winner of the QB tie


# --------------------------------------------------------------------------- #
# simulate_weekly_margin                                                       #
# --------------------------------------------------------------------------- #
def _weeks(mapping, weeks=(1, 2, 3)):
    """Broadcast one {player: points} dict to every week."""
    return {w: dict(mapping) for w in weeks}


def test_identical_projections_zero_margin():
    """proj_a == proj_b → every margin exactly 0; ties give win_rate 0.5."""
    proj = _weeks({"QB_A": 20, "RB_A": 18, "RB_B": 12, "RB_C": 10, "WR_A": 14})
    actual = _weeks({"QB_A": 11, "QB_B": 25, "RB_A": 3, "RB_B": 9, "RB_C": 7,
                     "WR_A": 8, "WR_B": 30})
    res = L.simulate_weekly_margin([_roster()], proj, proj, actual, SLOTS, n=500)
    assert res.margins == [0.0, 0.0, 0.0]
    assert res.overall.point == 0.0
    assert res.overall.excludes_zero is False
    assert res.win_rate == pytest.approx(0.5)


def test_better_projection_wins_and_ci_excludes_zero():
    """An oracle (projects the realized points) beats a wrong reference:
    every margin > 0, bootstrap CI strictly above 0, win_rate 1."""
    actual = _weeks({"QB_A": 5, "QB_B": 25, "RB_A": 20, "RB_B": 15, "RB_C": 2,
                     "WR_A": 10, "WR_B": 12})
    oracle = actual                              # method A sees the truth
    wrong = _weeks({"QB_A": 30, "QB_B": 1, "RB_A": 1, "RB_B": 2, "RB_C": 25,
                    "WR_A": 3, "WR_B": 1})       # method B inverts it
    res = L.simulate_weekly_margin([_roster()], oracle, wrong, actual, SLOTS, n=500)
    assert all(m > 0 for m in res.margins)
    assert res.overall.point > 0
    assert res.overall.excludes_zero is True
    assert res.win_rate == pytest.approx(1.0)


def test_margin_antisymmetric_in_methods():
    """Swapping which method is A vs B negates every margin — the slot-filling
    rule is shared, so nothing about 'being method A' confers an edge (§8)."""
    actual = _weeks({"QB_A": 5, "QB_B": 25, "RB_A": 20, "RB_B": 15, "RB_C": 2,
                     "WR_A": 10, "WR_B": 12})
    p1 = actual
    p2 = _weeks({"QB_A": 30, "QB_B": 1, "RB_A": 1, "RB_B": 2, "RB_C": 25,
                 "WR_A": 3, "WR_B": 1})
    fwd = L.simulate_weekly_margin([_roster()], p1, p2, actual, SLOTS, n=100)
    rev = L.simulate_weekly_margin([_roster()], p2, p1, actual, SLOTS, n=100)
    assert fwd.margins == [-m for m in rev.margins]


def test_starter_out_subset_flags_only_affected_cells():
    """Weeks where the reference's would-be starter is OUT land in the subset;
    untouched weeks (or bench-only absences) do not."""
    proj = _weeks({"QB_A": 20, "QB_B": 15, "RB_A": 18, "RB_B": 12, "RB_C": 10,
                   "WR_A": 14, "WR_B": 9})
    actual = _weeks({p["player_id"]: 10.0 for p in _roster()})
    # Week 1: reference starter RB_A is OUT. Week 2: bench WR_B OUT (starters
    # untouched). Week 3: nobody OUT.
    out = {1: {"RB_A"}, 2: {"WR_B"}}
    res = L.simulate_weekly_margin([_roster()], proj, proj, actual, SLOTS,
                                   out=out, n=100)
    assert len(res.starter_out_margins) == 1
    assert res.starter_out is not None
    assert res.starter_out.point == 0.0          # identical projections


def test_no_out_weeks_leaves_subset_empty():
    """With no availability data the starter-OUT CI is None, overall still runs."""
    proj = _weeks({"QB_A": 20})
    actual = _weeks({"QB_A": 10})
    res = L.simulate_weekly_margin([_roster()], proj, proj, actual, SLOTS, n=100)
    assert res.starter_out is None
    assert res.starter_out_margins == []
