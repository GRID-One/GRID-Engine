import pandas as pd
import pytest
from unittest.mock import patch, MagicMock
from backend.grid.nflverse_loader import (
    load_pbp, load_rosters, load_raw_pbp, load_participation,
    _normalize_participation, PLAYS_CONTRACT_COLS,
)


def _fake_pbp_df():
    return pd.DataFrame({
        "game_id": ["2023_01_KC_DET"],
        "play_id": [100],
        "posteam": ["KC"],
        "defteam": ["DET"],
        "down": [1],
        "ydstogo": [10],
        "yardline_100": [75],
        "yards_gained": [8],
        "play_type": ["pass"],
        "pass_attempt": [1],
        "rush_attempt": [0],
        "passer_player_id": ["00-0033873"],
        "receiver_player_id": ["00-0036442"],
        "rusher_player_id": [None],
        "air_yards": [5],
        "yards_after_catch": [3],
        "touchdown": [0],
        "td_team": [None],
        "fumble_lost": [0],
        "interception": [0],
        "season": [2023],
        "week": [1],
        "series_result": ["First down"],
    })


def test_plays_contract_columns():
    assert "pass_attempt" in PLAYS_CONTRACT_COLS
    assert "receiver_id" in PLAYS_CONTRACT_COLS
    assert "air_yards" in PLAYS_CONTRACT_COLS


@patch("backend.grid.nflverse_loader._fetch_pbp_year")
def test_load_pbp_returns_dataframe(mock_fetch):
    mock_fetch.return_value = _fake_pbp_df()
    result = load_pbp([2023], cache_dir="/tmp/test_cache")
    assert isinstance(result, pd.DataFrame)
    assert len(result) > 0


@patch("backend.grid.nflverse_loader._fetch_roster_year")
def test_load_rosters_returns_dataframe(mock_fetch):
    mock_fetch.return_value = pd.DataFrame({
        "player_id": ["00-0033873"],
        "player_name": ["Patrick Mahomes"],
        "team": ["KC"],
        "position": ["QB"],
        "status": ["ACT"],
        "season": [2023],
    })
    result = load_rosters([2023], cache_dir="/tmp/test_cache")
    assert isinstance(result, pd.DataFrame)
    assert "player_id" in result.columns
    assert "name" in result.columns


# --------------------------------------------------------------- participation
def _fake_participation_df():
    """A one-row nflverse-shaped participation frame (nflverse_game_id + delimited ids)."""
    return pd.DataFrame({
        "nflverse_game_id": ["2023_01_KC_DET"],
        "play_id": [100],
        "possession_team": ["KC"],
        "offense_formation": ["SHOTGUN"],
        "offense_players": ["00-0033873;00-0036442;00-0099999"],
        "defense_players": ["00-0011111;00-0022222"],
    })


def test_normalize_participation_renames_game_id_and_keeps_delimited():
    """nflverse_game_id -> game_id; off/def player delimited strings kept verbatim."""
    out = _normalize_participation(_fake_participation_df())
    assert list(out.columns) == ["game_id", "play_id", "offense_players", "defense_players"]
    assert out.loc[0, "game_id"] == "2023_01_KC_DET"
    assert out.loc[0, "offense_players"] == "00-0033873;00-0036442;00-0099999"


def test_normalize_participation_falls_back_to_old_game_id():
    """When nflverse_game_id is absent, fall back to old_game_id."""
    raw = pd.DataFrame({"old_game_id": ["2023010100"], "play_id": [1],
                        "offense_players": ["a;b"], "defense_players": ["x;y"]})
    out = _normalize_participation(raw)
    assert out.loc[0, "game_id"] == "2023010100"


def test_normalize_participation_raises_without_keys():
    """Missing game-id/play_id columns raise a clear KeyError."""
    with pytest.raises(KeyError):
        _normalize_participation(pd.DataFrame({"offense_players": ["a;b"]}))


@patch("backend.grid.nflverse_loader._fetch_participation_year")
def test_load_participation_returns_join_shape(mock_fetch, tmp_path):
    """load_participation returns the [game_id, play_id, offense_players, defense_players] join shape."""
    mock_fetch.return_value = _fake_participation_df()
    out = load_participation([2023], cache_dir=str(tmp_path))
    assert list(out.columns) == ["game_id", "play_id", "offense_players", "defense_players"]
    assert len(out) == 1


@patch("backend.grid.nflverse_loader._fetch_roster_year")
def test_load_rosters_maps_real_gsis_schema(mock_fetch, tmp_path):
    """Real nflverse rosters key on gsis_id + full_name — map to player_id/name
    so ids match participation's offense_players/defense_players."""
    mock_fetch.return_value = pd.DataFrame({
        "gsis_id": ["00-0033873"],
        "full_name": ["Patrick Mahomes"],
        "team": ["KC"],
        "position": ["QB"],
        "season": [2023],
    })
    out = load_rosters([2023], cache_dir=str(tmp_path))
    assert "player_id" in out.columns and "name" in out.columns
    assert out.loc[0, "player_id"] == "00-0033873"
    assert out.loc[0, "name"] == "Patrick Mahomes"


@patch("backend.grid.nflverse_loader._fetch_pbp_year")
def test_load_raw_pbp_preserves_drive_columns(mock_fetch, tmp_path):
    """Raw loader keeps fixed_drive/fixed_drive_result that the GRID adapter needs
    and that the stats-normalizing load_pbp drops."""
    raw = _fake_pbp_df()
    raw["fixed_drive"] = [1]
    raw["fixed_drive_result"] = ["Touchdown"]
    mock_fetch.return_value = raw
    out = load_raw_pbp([2023], cache_dir=str(tmp_path))
    assert "fixed_drive" in out.columns and "fixed_drive_result" in out.columns
    assert out.loc[0, "fixed_drive_result"] == "Touchdown"
