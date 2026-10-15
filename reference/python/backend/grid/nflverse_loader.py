from __future__ import annotations
import pandas as pd
from backend.grid.cache import ParquetCache

NFLVERSE_PBP_URL = (
    "https://github.com/nflverse/nflverse-data/releases/"
    "download/pbp/play_by_play_{year}.parquet"
)
NFLVERSE_ROSTER_URL = (
    "https://github.com/nflverse/nflverse-data/releases/"
    "download/rosters/roster_{year}.parquet"
)
NFLVERSE_PARTICIPATION_URL = (
    "https://github.com/nflverse/nflverse-data/releases/"
    "download/pbp_participation/pbp_participation_{year}.parquet"
)

PLAYS_CONTRACT_COLS = [
    "game_id", "play_id", "week", "off_team", "def_team",
    "down", "ydstogo", "yardline_100", "yards_gained",
    "pass_attempt", "rush_attempt",
    "passer_id", "receiver_id", "rusher_id",
    "air_yards", "yards_after_catch",
    "td_type", "fumble", "interception",
    "td_player_id", "complete_pass",
]


def _fetch_pbp_year(year: int) -> pd.DataFrame:
    """Download the raw nflverse play-by-play parquet for one season."""
    url = NFLVERSE_PBP_URL.format(year=year)
    return pd.read_parquet(url)


def _fetch_roster_year(year: int) -> pd.DataFrame:
    """Download the raw nflverse season roster parquet for one season."""
    url = NFLVERSE_ROSTER_URL.format(year=year)
    return pd.read_parquet(url)


def _fetch_participation_year(year: int) -> pd.DataFrame:
    """Download the raw nflverse/FTN participation parquet for one season."""
    url = NFLVERSE_PARTICIPATION_URL.format(year=year)
    return pd.read_parquet(url)


def _normalize_pbp(raw: pd.DataFrame) -> pd.DataFrame:
    """Normalize raw pbp to the stats-aggregation shape (PLAYS_CONTRACT_COLS).

    NOTE: this drops drive columns (fixed_drive/fixed_drive_result); the GRID
    plays-contract path uses load_raw_pbp + the adapter instead.
    """
    df = raw[raw["play_type"].isin(["pass", "run"])].copy()
    df = df[df["down"].notna()].copy()

    out = pd.DataFrame()
    out["game_id"] = df["game_id"]
    out["play_id"] = df["play_id"]
    out["week"] = df["week"]
    out["off_team"] = df["posteam"]
    out["def_team"] = df["defteam"]
    out["down"] = df["down"].astype(int)
    out["ydstogo"] = df["ydstogo"].astype(int)
    out["yardline_100"] = df["yardline_100"]
    out["yards_gained"] = df["yards_gained"]
    out["pass_attempt"] = df["pass_attempt"].fillna(0).astype(int)
    out["rush_attempt"] = df["rush_attempt"].fillna(0).astype(int)
    out["passer_id"] = df["passer_player_id"]
    out["receiver_id"] = df["receiver_player_id"]
    out["rusher_id"] = df["rusher_player_id"]
    out["air_yards"] = df["air_yards"]
    out["yards_after_catch"] = df["yards_after_catch"]
    out["td_type"] = df.apply(
        lambda r: "pass" if r.get("touchdown") == 1 and r.get("pass_attempt") == 1
        else ("rush" if r.get("touchdown") == 1 and r.get("rush_attempt") == 1 else None),
        axis=1,
    )
    out["fumble"] = df["fumble_lost"].fillna(0).astype(int)
    out["interception"] = df["interception"].fillna(0).astype(int)
    out["td_player_id"] = df.get("td_player_id", pd.Series(dtype="object", index=df.index))
    out["complete_pass"] = df["complete_pass"].fillna(0).astype(int) if "complete_pass" in df.columns else 0
    return out.reset_index(drop=True)


def _normalize_participation(raw: pd.DataFrame) -> pd.DataFrame:
    """Normalize nflverse/FTN participation to the join shape the GRID adapter
    expects: [game_id, play_id, offense_players, defense_players].

    The participation file keys games by ``nflverse_game_id`` (same format as
    pbp ``game_id``); fall back to ``game_id``/``old_game_id`` defensively across
    feed versions.  ``offense_players``/``defense_players`` are ``;``-delimited
    GSIS id strings — passed through verbatim for the adapter to split.
    """
    df = raw.copy()
    game_col = next((c for c in ("nflverse_game_id", "game_id", "old_game_id")
                     if c in df.columns), None)
    if game_col is None or "play_id" not in df.columns:
        raise KeyError(
            "participation data missing a game id (nflverse_game_id/game_id/"
            "old_game_id) or play_id column")
    out = pd.DataFrame({
        "game_id": df[game_col].astype(str),
        "play_id": df["play_id"],
    })
    for col in ("offense_players", "defense_players"):
        out[col] = df[col] if col in df.columns else None
    return out.reset_index(drop=True)


def _normalize_roster(raw: pd.DataFrame) -> pd.DataFrame:
    # Real nflverse rosters key players by `gsis_id` (the GSIS id that matches
    # participation's offense_players/defense_players) and name them `full_name`.
    # Map those to the canonical player_id/name, while still accepting the
    # already-canonical columns (used by tests / other feeds).
    rename = {}
    if "player_id" not in raw.columns and "gsis_id" in raw.columns:
        rename["gsis_id"] = "player_id"
    if "name" not in raw.columns:
        if "player_name" in raw.columns:
            rename["player_name"] = "name"
        elif "full_name" in raw.columns:
            rename["full_name"] = "name"
    df = raw.rename(columns=rename)
    # Select only columns that exist in the raw data
    desired_cols = ["player_id", "name", "team", "position", "season", "status"]
    optional_cols = ["week"]
    cols = [c for c in desired_cols + optional_cols if c in df.columns]
    return df[cols].copy()


def _load_cached_years(
    years: list[int],
    key_prefix: str,
    fetch_fn,
    normalize_fn,
    cache_dir: str = "data/cache",
    ttl_hours: int = 168,
) -> pd.DataFrame:
    """Load + concat ``years``, fetching+normalizing+caching any not already
    cached fresh (per-year parquet under ``{key_prefix}{year}``)."""
    cache = ParquetCache(cache_dir)
    frames = []
    for year in years:
        path = cache.get(f"{key_prefix}{year}", ttl_hours)
        if path is not None:
            frames.append(pd.read_parquet(path))
        else:
            df = normalize_fn(fetch_fn(year))
            cache.put(f"{key_prefix}{year}", df)
            frames.append(df)
    return pd.concat(frames, ignore_index=True) if frames else pd.DataFrame()


def load_pbp(years: list[int], cache_dir: str = "data/cache") -> pd.DataFrame:
    """Load normalized (stats-aggregation shape) nflverse pbp for ``years``."""
    return _load_cached_years(years, "nflverse_pbp_", _fetch_pbp_year, _normalize_pbp, cache_dir)


def load_raw_pbp(years: list[int], cache_dir: str = "data/cache") -> pd.DataFrame:
    """Cache + return RAW nflverse pbp (identity-normalized).

    Unlike load_pbp (which emits the stats-aggregation shape and drops drive
    columns), this preserves fixed_drive/fixed_drive_result/posteam/... that the
    GRID plays-contract adapter needs.  Cached under a distinct key so it never
    collides with the normalized pbp cache.
    """
    return _load_cached_years(years, "nflverse_raw_pbp_", _fetch_pbp_year, lambda df: df, cache_dir)


def load_rosters(years: list[int], cache_dir: str = "data/cache") -> pd.DataFrame:
    """Load normalized nflverse rosters (player_id/name/team/position) for ``years``."""
    return _load_cached_years(years, "nflverse_roster_", _fetch_roster_year, _normalize_roster, cache_dir)


def load_participation(years: list[int], cache_dir: str = "data/cache") -> pd.DataFrame:
    """Load nflverse/FTN per-play participation, normalized to
    [game_id, play_id, offense_players, defense_players].  Free, but published
    once after the postseason (not in-season) — see the data-sourcing notes."""
    return _load_cached_years(
        years, "nflverse_participation_", _fetch_participation_year,
        _normalize_participation, cache_dir,
    )
