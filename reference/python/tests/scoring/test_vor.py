import pytest
from backend.scoring.vor import calculate_vor, assign_tiers


def _sample_players():
    return [
        {"player_id": "qb1", "name": "QB A", "position": "QB", "projected_points": 320},
        {"player_id": "qb2", "name": "QB B", "position": "QB", "projected_points": 290},
        {"player_id": "qb3", "name": "QB C", "position": "QB", "projected_points": 260},
        {"player_id": "rb1", "name": "RB A", "position": "RB", "projected_points": 280},
        {"player_id": "rb2", "name": "RB B", "position": "RB", "projected_points": 250},
        {"player_id": "rb3", "name": "RB C", "position": "RB", "projected_points": 200},
        {"player_id": "rb4", "name": "RB D", "position": "RB", "projected_points": 180},
        {"player_id": "rb5", "name": "RB E", "position": "RB", "projected_points": 150},
    ]


# --- Tests specified in the task brief ---

def test_vor_ranks_by_value_over_replacement():
    roster_slots = {"QB": 1, "RB": 2}
    result = calculate_vor(_sample_players(), roster_slots, num_teams=2)
    by_id = {p["player_id"]: p for p in result}
    assert by_id["qb1"]["vor"] > by_id["qb3"]["vor"]
    assert by_id["rb1"]["vor"] > by_id["rb3"]["vor"]


def test_replacement_level_correct():
    roster_slots = {"QB": 1, "RB": 2}
    result = calculate_vor(_sample_players(), roster_slots, num_teams=2)
    by_id = {p["player_id"]: p for p in result}
    # QB replacement = QB at position (1 * 2 teams + 1) = QB3 (260)
    # QB1 VOR = 320 - 260 = 60
    assert by_id["qb1"]["vor"] == pytest.approx(60.0)


def test_tiers_group_similar_players():
    players = [
        {"player_id": "a", "vor": 50},
        {"player_id": "b", "vor": 48},
        {"player_id": "c", "vor": 30},
        {"player_id": "d", "vor": 28},
        {"player_id": "e", "vor": 5},
    ]
    result = assign_tiers(players, max_gap_pct=0.15)
    by_id = {p["player_id"]: p for p in result}
    assert by_id["a"]["tier"] == by_id["b"]["tier"]
    assert by_id["c"]["tier"] == by_id["d"]["tier"]
    assert by_id["a"]["tier"] < by_id["c"]["tier"]


def test_all_players_get_rank():
    roster_slots = {"QB": 1, "RB": 2}
    result = calculate_vor(_sample_players(), roster_slots, num_teams=2)
    ranks = [p["rank"] for p in result]
    assert ranks == list(range(1, len(result) + 1))


# --- Additional edge-case tests ---

def test_rb_replacement_level_correct():
    """RB replacement = RB at index 2*2=4 (0-indexed) = RB5 (150).
    RB1 VOR = 280 - 150 = 130."""
    roster_slots = {"QB": 1, "RB": 2}
    result = calculate_vor(_sample_players(), roster_slots, num_teams=2)
    by_id = {p["player_id"]: p for p in result}
    assert by_id["rb1"]["vor"] == pytest.approx(130.0)


def test_players_not_in_roster_slots_get_zero_vor():
    """Players at positions not in roster_slots get vor=0.0."""
    players = [
        {"player_id": "k1", "name": "Kicker A", "position": "K", "projected_points": 100},
        {"player_id": "qb1", "name": "QB A", "position": "QB", "projected_points": 320},
    ]
    roster_slots = {"QB": 1}
    result = calculate_vor(players, roster_slots, num_teams=2)
    by_id = {p["player_id"]: p for p in result}
    assert by_id["k1"]["vor"] == pytest.approx(0.0)


def test_empty_player_list():
    result = calculate_vor([], {"QB": 1}, num_teams=12)
    assert result == []


def test_assign_tiers_empty():
    assert assign_tiers([]) == []


def test_assign_tiers_single_player():
    result = assign_tiers([{"player_id": "a", "vor": 100}])
    assert result[0]["tier"] == 1


def test_assign_tiers_all_same_tier():
    """If gaps are all below max_gap_pct, all players share tier 1."""
    players = [
        {"player_id": "a", "vor": 50},
        {"player_id": "b", "vor": 49},
        {"player_id": "c", "vor": 48},
    ]
    result = assign_tiers(players, max_gap_pct=0.10)
    tiers = {p["player_id"]: p["tier"] for p in result}
    assert tiers["a"] == tiers["b"] == tiers["c"] == 1


def test_assign_tiers_each_own_tier():
    """Large gaps force every player into their own tier."""
    players = [
        {"player_id": "a", "vor": 100},
        {"player_id": "b", "vor": 50},
        {"player_id": "c", "vor": 10},
    ]
    result = assign_tiers(players, max_gap_pct=0.10)
    tiers = {p["player_id"]: p["tier"] for p in result}
    assert tiers["a"] == 1
    assert tiers["b"] == 2
    assert tiers["c"] == 3


def test_rank_is_1_indexed_and_sequential():
    players = [
        {"player_id": "qb1", "name": "QB A", "position": "QB", "projected_points": 320},
        {"player_id": "qb2", "name": "QB B", "position": "QB", "projected_points": 290},
    ]
    result = calculate_vor(players, {"QB": 1}, num_teams=1)
    ranks = sorted(p["rank"] for p in result)
    assert ranks == [1, 2]


def test_replacement_player_has_zero_vor():
    """The player exactly at the replacement boundary has VOR=0."""
    roster_slots = {"QB": 1}
    num_teams = 2
    # repl_idx = 1*2 = 2 -> QB3 is replacement player
    result = calculate_vor(_sample_players(), roster_slots, num_teams=num_teams)
    by_id = {p["player_id"]: p for p in result}
    assert by_id["qb3"]["vor"] == pytest.approx(0.0)


def test_below_replacement_has_negative_vor():
    """Players worse than the replacement level get negative VOR."""
    players = [
        {"player_id": "qb1", "name": "QB A", "position": "QB", "projected_points": 320},
        {"player_id": "qb2", "name": "QB B", "position": "QB", "projected_points": 290},
        {"player_id": "qb3", "name": "QB C", "position": "QB", "projected_points": 260},
        {"player_id": "qb4", "name": "QB D", "position": "QB", "projected_points": 220},
    ]
    # repl_idx = 1*2 = 2 -> QB3 (260) is replacement; QB4 = 220 - 260 = -40
    result = calculate_vor(players, {"QB": 1}, num_teams=2)
    by_id = {p["player_id"]: p for p in result}
    assert by_id["qb4"]["vor"] == pytest.approx(-40.0)


def test_original_player_dicts_not_mutated():
    """calculate_vor should not mutate the input player dicts."""
    original = _sample_players()
    originals_copy = [dict(p) for p in original]
    calculate_vor(original, {"QB": 1, "RB": 2}, num_teams=2)
    for orig, copy in zip(original, originals_copy):
        assert orig == copy


# ─── FLEX-aware VOR tests ────────────────────────────────────────────────────

def _flex_players():
    return [
        {"player_id": "qb1", "name": "QB A", "position": "QB", "projected_points": 320},
        {"player_id": "rb1", "name": "RB A", "position": "RB", "projected_points": 280},
        {"player_id": "rb2", "name": "RB B", "position": "RB", "projected_points": 250},
        {"player_id": "rb3", "name": "RB C", "position": "RB", "projected_points": 200},
        {"player_id": "rb4", "name": "RB D", "position": "RB", "projected_points": 180},
        {"player_id": "wr1", "name": "WR A", "position": "WR", "projected_points": 270},
        {"player_id": "wr2", "name": "WR B", "position": "WR", "projected_points": 230},
        {"player_id": "wr3", "name": "WR C", "position": "WR", "projected_points": 190},
        {"player_id": "wr4", "name": "WR D", "position": "WR", "projected_points": 160},
        {"player_id": "te1", "name": "TE A", "position": "TE", "projected_points": 210},
        {"player_id": "te2", "name": "TE B", "position": "TE", "projected_points": 170},
        {"player_id": "te3", "name": "TE C", "position": "TE", "projected_points": 130},
    ]


def test_vor_with_flex_slot():
    """FLEX slot should raise replacement level for eligible positions."""
    players = _flex_players()
    no_flex = calculate_vor(players, {"QB": 1, "RB": 2, "WR": 2, "TE": 1}, num_teams=1)
    with_flex = calculate_vor(players, {"QB": 1, "RB": 2, "WR": 2, "TE": 1, "FLEX": 1}, num_teams=1)
    no_flex_rb1 = next(p for p in no_flex if p["player_id"] == "rb1")
    with_flex_rb1 = next(p for p in with_flex if p["player_id"] == "rb1")
    assert with_flex_rb1["vor"] <= no_flex_rb1["vor"], \
        "FLEX should raise replacement level, lowering top-player VOR"


def test_vor_flex_replacement_level_correct():
    """Hand-computed verification of FLEX replacement level."""
    players = _flex_players()
    # RB: 2 slots, WR: 2 slots, TE: 1 slot, FLEX: 1 slot, num_teams=1
    # Total FLEX demand = (2+2+1+1)*1 = 6 positions across FLEX-eligible
    # Combined pool sorted: RB1(280), WR1(270), RB2(250), WR2(230), TE1(210), RB3(200), WR3(190), ...
    # FLEX repl at index 6 = WR3(190)
    result = calculate_vor(players, {"QB": 1, "RB": 2, "WR": 2, "TE": 1, "FLEX": 1}, num_teams=1)
    by_id = {p["player_id"]: p for p in result}
    # RB replacement (position-only: idx 2 = rb3 = 200) vs FLEX repl (190)
    # max(200, 190) = 200, so RB replacement stays at 200
    assert by_id["rb1"]["vor"] == pytest.approx(280 - 200)
    # TE replacement (position-only: idx 1 = te2 = 170) vs FLEX repl (190)
    # max(170, 190) = 190, so TE replacement RAISED to 190
    assert by_id["te1"]["vor"] == pytest.approx(210 - 190)


def test_vor_no_flex_slot_unchanged():
    """Without FLEX in roster_slots, VOR should match pre-FLEX behavior."""
    players = _flex_players()
    result = calculate_vor(players, {"QB": 1, "RB": 2, "WR": 2, "TE": 1}, num_teams=1)
    by_id = {p["player_id"]: p for p in result}
    # RB replacement at idx 2 = rb3 (200)
    assert by_id["rb1"]["vor"] == pytest.approx(280 - 200)
    # TE replacement at idx 1 = te2 (170)
    assert by_id["te1"]["vor"] == pytest.approx(210 - 170)


def test_vor_flex_only_affects_eligible_positions():
    """QB should be unaffected by FLEX slots."""
    players = _flex_players()
    no_flex = calculate_vor(players, {"QB": 1, "RB": 2, "WR": 2, "TE": 1}, num_teams=1)
    with_flex = calculate_vor(players, {"QB": 1, "RB": 2, "WR": 2, "TE": 1, "FLEX": 1}, num_teams=1)
    qb_no = next(p for p in no_flex if p["player_id"] == "qb1")
    qb_with = next(p for p in with_flex if p["player_id"] == "qb1")
    assert qb_no["vor"] == qb_with["vor"]


def test_vor_flex_zero_slots_unchanged():
    """FLEX: 0 should have no effect."""
    players = _flex_players()
    r0 = calculate_vor(players, {"QB": 1, "RB": 2, "WR": 2, "TE": 1, "FLEX": 0}, num_teams=1)
    r_none = calculate_vor(players, {"QB": 1, "RB": 2, "WR": 2, "TE": 1}, num_teams=1)
    for p0, pn in zip(r0, r_none):
        assert p0["vor"] == pn["vor"]


def test_vor_flex_only_position_gets_flex_replacement():
    """A FLEX-eligible position with no dedicated slot is ranked against the
    FLEX replacement level, not zeroed out (it competes for the FLEX spot)."""
    players = _flex_players()
    # TE has NO dedicated slot; FLEX demand = (2+2+0+1)*1 = 5
    # flex pool sorted: rb1 280, wr1 270, rb2 250, wr2 230, te1 210, rb3 200, ...
    # FLEX repl at index 5 = rb3 (200)
    result = calculate_vor(players, {"QB": 1, "RB": 2, "WR": 2, "FLEX": 1}, num_teams=1)
    by_id = {p["player_id"]: p for p in result}
    assert by_id["te1"]["vor"] == pytest.approx(210 - 200)   # above FLEX repl
    assert by_id["te2"]["vor"] == pytest.approx(170 - 200)   # below it
    # Without FLEX the unlisted position still gets the documented 0.0
    no_flex = calculate_vor(players, {"QB": 1, "RB": 2, "WR": 2}, num_teams=1)
    te1 = next(p for p in no_flex if p["player_id"] == "te1")
    assert te1["vor"] == pytest.approx(0.0)
