"""Tests for the nflverse -> GRID `plays`-contract adapter (Phase 1).

Network-free: all on small hand-built nflverse-shaped DataFrames, mirroring the
tests/grid/test_nflverse_loader.py convention.  Verifies the synth-matched
contract semantics (drive segmentation, next-state, drive_points/terminal
labeling, participation join) and that the output is consumable by the GRID
value path (fit_value_model + compute_dv).
"""
import numpy as np
import pandas as pd
import pytest
from unittest.mock import patch

from backend.grid.nflverse_adapter import build_plays_contract, load_grid_plays
from backend.grid.value import fit_value_model, attach_dv, STATE_COLS

_RAW_COLS = ["game_id", "play_id", "week", "posteam", "defteam", "down",
             "ydstogo", "yardline_100", "yards_gained", "play_type",
             "fixed_drive", "fixed_drive_result"]


def _raw(rows: list[dict]) -> pd.DataFrame:
    """Build a raw-nflverse-shaped PBP frame from partial row dicts (defaults
    filled in), so each test only states the fields it cares about."""
    base = dict(game_id="G1", week=1, posteam="AAA", defteam="BBB",
                down=1, ydstogo=10, yardline_100=75, yards_gained=4.0,
                play_type="pass", fixed_drive=1, fixed_drive_result="Punt")
    return pd.DataFrame([{**base, **r} for r in rows])[_RAW_COLS]


# --- a canonical 2-drive game used by several tests ---------------------------
def _two_drive_game() -> pd.DataFrame:
    return _raw([
        # drive 1 — Touchdown, 3 plays
        dict(play_id=10, fixed_drive=1, fixed_drive_result="Touchdown", down=1, ydstogo=10, yardline_100=80),
        dict(play_id=11, fixed_drive=1, fixed_drive_result="Touchdown", down=2, ydstogo=6,  yardline_100=76),
        dict(play_id=12, fixed_drive=1, fixed_drive_result="Touchdown", down=1, ydstogo=10, yardline_100=70),
        # drive 2 — Punt, 2 plays
        dict(play_id=20, fixed_drive=2, fixed_drive_result="Punt", down=1, ydstogo=10, yardline_100=75),
        dict(play_id=21, fixed_drive=2, fixed_drive_result="Punt", down=2, ydstogo=10, yardline_100=75),
    ])


def test_drive_id_construction():
    """drive_id is game_id + '_' + fixed_drive, one per (game, drive)."""
    out = build_plays_contract(_two_drive_game())
    assert set(out["drive_id"]) == {"G1_1", "G1_2"}
    assert (out[out.play_id.isin([10, 11, 12])]["drive_id"] == "G1_1").all()


def test_drive_points_mapping_and_broadcast():
    """fixed_drive_result maps to TD 7 / FG 3 / else 0, broadcast to every play."""
    raw = _raw([
        dict(play_id=1, fixed_drive=1, fixed_drive_result="Touchdown"),
        dict(play_id=2, fixed_drive=1, fixed_drive_result="Touchdown"),
        dict(play_id=3, fixed_drive=2, fixed_drive_result="Field goal"),
        dict(play_id=4, fixed_drive=3, fixed_drive_result="Punt"),
        dict(play_id=5, fixed_drive=4, fixed_drive_result="Turnover"),
        dict(play_id=6, fixed_drive=5, fixed_drive_result="Missed field goal"),
        dict(play_id=7, fixed_drive=6, fixed_drive_result="End of half"),
    ])
    out = build_plays_contract(raw).set_index("play_id")
    assert out.loc[1, "drive_points"] == 7.0 and out.loc[2, "drive_points"] == 7.0  # broadcast
    assert out.loc[3, "drive_points"] == 3.0
    for pid in (4, 5, 6, 7):
        assert out.loc[pid, "drive_points"] == 0.0


def test_next_state_and_terminal_within_drive():
    """Next-state is the next play's state; last play of a drive is terminal (n_*=-1)."""
    out = build_plays_contract(_two_drive_game()).set_index("play_id")
    # mid-drive next-state == the next play's actual state
    assert (out.loc[10, ["n_down", "n_ydstogo", "n_yardline_100"]].tolist() == [2, 6, 76.0])
    assert (out.loc[11, ["n_down", "n_ydstogo", "n_yardline_100"]].tolist() == [1, 10, 70.0])
    # last play of each drive is terminal with n_* == -1
    assert out.loc[12, "terminal"] and out.loc[21, "terminal"]
    assert out.loc[12, ["n_down", "n_ydstogo", "n_yardline_100"]].tolist() == [-1, -1, -1.0]
    # non-terminal plays are not terminal
    assert not out.loc[10, "terminal"] and not out.loc[20, "terminal"]


def test_terminal_value_and_points():
    """terminal_value/points = drive_points on the terminal play, NaN/0 elsewhere."""
    out = build_plays_contract(_two_drive_game()).set_index("play_id")
    # terminal_value = drive_points on terminal, NaN elsewhere
    assert out.loc[12, "terminal_value"] == 7.0          # TD drive terminal
    assert out.loc[21, "terminal_value"] == 0.0          # Punt drive terminal
    assert np.isnan(out.loc[10, "terminal_value"])       # non-terminal
    # points = drive_points on terminal, 0 elsewhere
    assert out.loc[12, "points"] == 7.0 and out.loc[10, "points"] == 0.0
    assert out.loc[21, "points"] == 0.0


def test_no_cross_drive_bleed():
    """The last play of drive 1 must terminate, not borrow drive 2's state."""
    out = build_plays_contract(_two_drive_game()).set_index("play_id")
    assert out.loc[12, "terminal"]
    assert out.loc[12, "n_yardline_100"] == -1.0   # NOT 75 (drive 2's first play)


def test_non_modelled_rows_dropped():
    """Non-pass/run plays and null-down rows are filtered out."""
    raw = _raw([
        dict(play_id=1, fixed_drive=1, fixed_drive_result="Punt", play_type="pass"),
        dict(play_id=2, fixed_drive=1, fixed_drive_result="Punt", play_type="punt"),    # dropped
        dict(play_id=3, fixed_drive=1, fixed_drive_result="Punt", play_type="run"),
        dict(play_id=4, fixed_drive=1, fixed_drive_result="Punt", play_type="run", down=np.nan),  # dropped
    ])
    out = build_plays_contract(raw)
    assert set(out["play_id"]) == {1, 3}


def test_participation_join_and_skill_filter():
    """Participation joins on (game_id, play_id); off_players keeps only skill
    positions, def_players is kept as listed, and unmatched plays get ()."""
    raw = _two_drive_game()
    participation = pd.DataFrame([
        # play 10: skill A,B + OL C on offense; defenders X,Y
        dict(game_id="G1", play_id=10, offense_players="A;B;C", defense_players="X;Y"),
        dict(game_id="G1", play_id=11, offense_players="A;B", defense_players="X;Y;Z"),
        # plays 12, 20, 21 have no participation row
    ])
    roster = {"A": "QB", "B": "WR", "C": "T", "X": "CB", "Y": "LB", "Z": "DE"}
    out = build_plays_contract(raw, participation=participation, roster_positions=roster).set_index("play_id")
    assert out.loc[10, "off_players"] == ("A", "B")        # OL "C" filtered out
    assert out.loc[10, "def_players"] == ("X", "Y")        # defense kept as listed
    assert out.loc[11, "def_players"] == ("X", "Y", "Z")
    assert out.loc[12, "off_players"] == () and out.loc[12, "def_players"] == ()  # no row


def test_participation_missing_cells_never_yield_placeholder_ids():
    """A pd.NA / NaN / None participation cell yields () — never a bogus '<NA>' id."""
    raw = _two_drive_game()
    participation = pd.DataFrame([
        dict(game_id="G1", play_id=10, offense_players=pd.NA, defense_players=np.nan),
        dict(game_id="G1", play_id=11, offense_players=None, defense_players="X;Y"),
    ])
    out = build_plays_contract(raw, participation=participation).set_index("play_id")
    assert out.loc[10, "off_players"] == () and out.loc[10, "def_players"] == ()
    assert out.loc[11, "off_players"] == () and out.loc[11, "def_players"] == ("X", "Y")
    # no placeholder id leaked anywhere
    for col in ("off_players", "def_players"):
        assert all("<NA>" not in t and "nan" not in t for t in out[col])


def test_participation_absent_gives_empty_tuples():
    """With no participation frame, off_players/def_players are all empty tuples."""
    out = build_plays_contract(_two_drive_game())
    assert (out["off_players"].apply(lambda t: t == ())).all()
    assert (out["def_players"].apply(lambda t: t == ())).all()


def test_empty_input_returns_empty_contract():
    """When all rows are filtered out, return a zero-row frame with the full schema."""
    raw = _raw([dict(play_id=1, play_type="punt")])   # everything filtered out
    out = build_plays_contract(raw)
    assert len(out) == 0
    for col in ("drive_id", "drive_points", "terminal", "n_down", "off_players"):
        assert col in out.columns


def test_load_grid_plays_assembles_contract_with_participation():
    """load_grid_plays wires raw pbp + participation + rosters into the contract,
    with off_players skill-filtered and a play_id dtype mismatch (float pbp vs int
    participation) that must still join."""
    raw = _two_drive_game()
    raw["play_id"] = raw["play_id"].astype(float)   # pbp play_id often float
    participation = pd.DataFrame([
        dict(game_id="G1", play_id=10, offense_players="QB1;WR1;LT1", defense_players="CB1;LB1"),
        dict(game_id="G1", play_id=20, offense_players="QB1;RB1;LG1", defense_players="DE1"),
    ])  # play_id int here, float in pbp -> exercises the dtype-normalization join
    roster = pd.DataFrame({
        "player_id": ["QB1", "WR1", "LT1", "RB1", "LG1", "CB1", "LB1", "DE1"],
        "position": ["QB", "WR", "T", "RB", "G", "CB", "LB", "DE"],
    })
    with patch("backend.grid.nflverse_loader.load_raw_pbp", return_value=raw), \
         patch("backend.grid.nflverse_loader.load_participation", return_value=participation), \
         patch("backend.grid.nflverse_loader.load_rosters", return_value=roster):
        out = load_grid_plays([2023], cache_dir="unused").set_index("play_id")

    # skill filter: OL (LT1/LG1) dropped from offense; defense kept as listed
    assert out.loc[10, "off_players"] == ("QB1", "WR1")
    assert out.loc[10, "def_players"] == ("CB1", "LB1")
    assert out.loc[20, "off_players"] == ("QB1", "RB1")
    # plays without a participation row -> empty tuples
    assert out.loc[12, "off_players"] == ()
    # full contract shape present
    for col in ("drive_id", "drive_points", "terminal", "n_down", "off_players"):
        assert col in out.columns


def test_output_feeds_grid_value_path():
    """Integration: the contract flows through fit_value_model + compute_dv with
    finite dV, and the fitted V(s) increases as the offense nears the goal line."""
    rng = np.random.default_rng(0)
    rows = []
    pid = 0
    for drv in range(400):
        start = int(rng.integers(20, 95))           # yards to goal at drive start
        p_td = max(0.02, (100 - start) / 130.0)     # closer start -> more likely TD
        r = rng.random()
        result = "Touchdown" if r < p_td else ("Field goal" if r < p_td + 0.15 else "Punt")
        yl = start
        nplays = int(rng.integers(2, 6))
        for k in range(nplays):
            rows.append(dict(game_id="G1", play_id=pid, fixed_drive=drv,
                             fixed_drive_result=result, down=min(4, 1 + k),
                             ydstogo=10, yardline_100=max(1, yl), play_type="pass"))
            pid += 1
            yl = max(1, yl - int(rng.integers(3, 12)))
    out = build_plays_contract(_raw(rows))
    out = attach_dv(out, fit_value_model(out))
    assert np.isfinite(out["dv"]).all()
    vm = fit_value_model(out)
    grid = pd.DataFrame({"down": 1, "ydstogo": 10, "yardline_100": [90, 60, 30, 10]})
    v = vm.predict(grid[STATE_COLS].to_numpy(float))
    # monotonic the whole way toward the goal line, not just endpoints
    assert np.all(np.diff(v) > 0), f"V should rise monotonically toward the goal line, got {v}"
