"""
Data adapters -- THE SWAP POINT between synthetic and real data.

The rest of GRID consumes a small, fixed contract:

  plays  : one row per play with columns
           [play_id, drive_id, week, off_team, def_team,
            down, ydstogo, yardline_100,            # state s
            yards, points, terminal, terminal_value,# outcome
            n_down, n_ydstogo, n_yardline_100,      # next state s'
            drive_points,                           # label to fit V(s)
            off_players (tuple), def_players (tuple)]# participation
  players: [player_id, team, position, is_starter, (ability if synthetic)]
  market : {team -> closing-line-implied strength}
  college: [player_id, position, feeder_sv, feeder_snaps, is_rookie]

`load_synthetic()` returns this contract with planted ground truth and is the
default the demo runs on. The real loaders below are stubs with the correct
endpoints sketched; fill in participation + odds + a college SV pass and the
identical downstream pipeline runs on real football. They are intentionally not
executed here (large downloads; participation/odds/CFBD need their own sources
and keys), and the nflverse path should be re-confirmed against current docs.
"""
from __future__ import annotations
import pandas as pd

from backend.grid.synth import simulate, make_college, SynthConfig


# --------------------------------------------------------------- synthetic (default)
def load_synthetic(cfg: SynthConfig | None = None):
    cfg = cfg or SynthConfig(yards_noise_sd=3.2, drives_per_team_per_game=12)
    plays, players, gt = simulate(cfg)
    college = make_college(players, gt, cfg)
    return dict(plays=plays, players=players, gt=gt, college=college, cfg=cfg)


# --------------------------------------------------------------- real loaders (stubs)
def load_nflfastr_pbp(years):
    """Download nflfastR play-by-play for the given seasons and normalize to the
    GRID `plays` contract. Requires deriving s' (next state) within each drive
    and labeling drive_points (next-score on the possession)."""
    raise NotImplementedError(
        "Wire up: pd.read_parquet(NFLVERSE_PBP.format(year=y)) for y in years, "
        "then map columns -> GRID contract (down, ydstogo, yardline_100, ...), "
        "build next-state per drive, and attach drive_points.")


def load_participation(years):
    """On-field participation (off_players / def_players per play). nflverse
    participation data exists for a limited span and is the linchpin of Layer 2
    RAPM -- without it, plus-minus is not identified. Merge onto pbp by play id.

    Real loader: nflverse/FTN ``pbp_participation_{year}.parquet`` (free,
    published once after the postseason). Returns [game_id, play_id,
    offense_players, defense_players]."""
    from backend.grid.nflverse_loader import load_participation as _load_part  # noqa: PLC0415
    return _load_part(years)


def load_odds(years):
    """Closing spread/total per game -> team-strength prior for Layer 3. Needs a
    separate odds dataset (not in nflverse). Return {team -> implied strength}
    per season, used ONLY as a team-level anchor, never per play."""
    raise NotImplementedError("Provide a closing-line odds source + mapping.")


def load_cfbd(years, api_key=None):
    """CollegeFootballData play-by-play for the feeder (NCAA) SV pass. Run the
    reduced Layer-1/Layer-2 machinery within college (with within-NCAA opponent
    adjustment) to produce feeder_sv per player. USFL/XFL/UFL get their own
    league-strength slope; each feeder league is its own regime."""
    raise NotImplementedError("Query CFBD API (needs key); compute feeder SV.")


REAL_LOADERS = dict(pbp=load_nflfastr_pbp, participation=load_participation,
                    odds=load_odds, cfbd=load_cfbd)
