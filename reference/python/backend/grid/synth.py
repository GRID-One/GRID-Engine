"""
Synthetic NFL data generator with PLANTED GROUND TRUTH.

The whole point of this module: produce play-by-play that is generated from
known, hidden player abilities and team strengths, so the estimators in the
rest of the package can be validated by checking whether they recover the
planted truth. Real data gives you no ground truth; synthetic data does.

Mirrors the *shape* of nflfastR play-by-play (one row per play, game state,
on-field participation, outcome) so the real-data adapters can drop in.

Design choices that make recovery testable:
  * A player's marginal effect flows through YARDS GAINED, which is linear in
    the net on-field ability gap. Better players -> more yards -> better field
    position / more points -> higher dV. RAPM then recovers a *scaled* version
    of ability, so validation is by correlation (scale-invariant), not by
    matching units exactly.
  * Substitutions (starters ~80% of snaps, backups rotate) create the lineup
    variation that adjusted plus-minus needs to separate teammates.
  * One designated QB ("focus" QB) has a time-varying weekly ability:
    a rising trajectory, an injury that costs two games, a diminished return,
    and a recovery -- so the state-space (Kalman) layer has a real latent
    trajectory + intervention to recover.
"""
from __future__ import annotations
from dataclasses import dataclass, field
import numpy as np
import pandas as pd

# ---- on-field personnel (offense skill positions only; OL is a team-level
# nuisance, never an estimand, consistent with the design) ----
OFF_ONFIELD = {"QB": 1, "RB": 1, "WR": 2, "TE": 1}
DEF_ONFIELD = 7
# roster depth per team (extra bodies -> rotation -> lineup variation)
ROSTER = {"QB": 2, "RB": 3, "WR": 5, "TE": 2, "DEF": 12}
# planted ability scale (interpretable-ish as EPA/play marginal contribution)
ABILITY_SD = {"QB": 0.090, "RB": 0.030, "WR": 0.028, "TE": 0.022, "DEF": 0.022}
STARTER_BONUS = {"QB": 0.10, "RB": 0.03, "WR": 0.03, "TE": 0.02, "DEF": 0.02}


@dataclass
class SynthConfig:
    n_teams: int = 12
    weeks: int = 14
    seed: int = 7
    yards_ability_gain: float = 26.0   # yards sensitivity to net ability gap
    yards_noise_sd: float = 4.2
    drives_per_team_per_game: int = 10
    max_plays_per_drive: int = 12
    fg_range_yardline: int = 35        # attempt FG inside this (yardline_100)
    league_factor: float = 0.62        # feeder->NFL true equivalency slope
    college_noise_sd: float = 0.030
    cb_split: bool = False             # Task 7: split some DEF slots into CB role
    n_cb_per_team: int = 2             # how many DEF players become CBs (cb_split=True)


def _make_players(cfg: SynthConfig, rng) -> pd.DataFrame:
    rows = []
    pid = 0
    for team in range(cfg.n_teams):
        for pos, n in ROSTER.items():
            for k in range(n):
                is_starter = k < (OFF_ONFIELD.get(pos, 0) or (DEF_ONFIELD if pos == "DEF" else 0))
                # DEF "starter" defined loosely: first 7 are the base unit
                if pos == "DEF":
                    is_starter = k < DEF_ONFIELD
                ability = rng.normal(0.0, ABILITY_SD[pos])
                if is_starter:
                    ability += STARTER_BONUS[pos]
                else:
                    ability -= 0.5 * STARTER_BONUS[pos]  # backups a touch worse
                # cb_split knob (Task 7): re-label the first n_cb_per_team DEF
                # starters as "CB" so WR-vs-CB interaction tests are generatable.
                # The ability distribution and on-field count are unchanged;
                # only the position label changes.  Default OFF preserves all
                # existing synth-based tests byte-for-byte.
                actual_pos = pos
                if cfg.cb_split and pos == "DEF" and k < cfg.n_cb_per_team:
                    actual_pos = "CB"
                rows.append(dict(player_id=pid, team=team, position=actual_pos,
                                 is_starter=bool(is_starter), ability=float(ability)))
                pid += 1
    return pd.DataFrame(rows)


def _focus_qb_trajectory(weeks: int):
    """Weekly TRUE ability for the focus QB: ramp, injury (NaN = did not play),
    diminished return, recovery. Realistic amplitude (a healthy->injured QB is a
    large swing) so the latent trajectory sits above the one-game noise floor.
    NaN marks games missed."""
    tau = np.full(weeks, np.nan)
    base, slope, peak = 0.05, 0.025, 0.05 + 0.025 * 7
    for w in range(weeks):
        wk = w + 1
        if wk <= 7:
            tau[w] = base + slope * wk                 # steady rise to ~0.225
        elif wk in (8, 9):
            tau[w] = np.nan                            # injured, missed games
        elif wk == 10:
            tau[w] = 0.06                              # returns badly diminished
        else:
            frac = min(1.0, (wk - 10) / 3.0)           # recover toward peak
            tau[w] = 0.06 * (1 - frac) + peak * frac
    return tau


def _pick_onfield(team_players: dict, rng):
    """Return on-field offensive and defensive player_ids with rotation."""
    off = []
    for pos, n in OFF_ONFIELD.items():
        pool = team_players[("off", pos)]
        starters = pool["starters"]
        backups = pool["backups"]
        chosen = []
        for slot in range(n):
            # starter plays ~80% of the time at that slot
            if rng.random() < 0.80 or not backups:
                chosen.append(starters[slot % len(starters)])
            else:
                chosen.append(backups[rng.integers(len(backups))])
        off.extend(chosen)
    # defense: 7 of the 12, base unit usually in, some rotation
    dpool = team_players[("def", "DEF")]["all"]
    if rng.random() < 0.7:
        deff = list(dpool[:DEF_ONFIELD])
    else:
        deff = list(rng.choice(dpool, size=DEF_ONFIELD, replace=False))
    return off, deff


def _team_index(players: pd.DataFrame):
    idx = {}
    for team in players.team.unique():
        tp = players[players.team == team]
        d = {}
        for pos in OFF_ONFIELD:
            pos_p = tp[tp.position == pos].sort_values("is_starter", ascending=False)
            n_start = OFF_ONFIELD[pos]
            d[("off", pos)] = dict(
                starters=list(pos_p.player_id.values[:n_start]),
                backups=list(pos_p.player_id.values[n_start:]),
            )
        # Include both DEF and CB players in the defensive pool so that
        # cb_split=True configs still populate defensive lineups correctly.
        defp = tp[tp.position.isin(["DEF", "CB"])].sort_values("is_starter", ascending=False)
        d[("def", "DEF")] = dict(all=list(defp.player_id.values))
        idx[team] = d
    return idx


def _round_robin(n_teams: int, weeks: int, rng):
    teams = list(range(n_teams))
    sched = []
    for w in range(1, weeks + 1):
        perm = teams[:]
        rng.shuffle(perm)
        for i in range(0, n_teams - 1, 2):
            sched.append((w, perm[i], perm[i + 1]))
    return sched


def simulate(cfg: SynthConfig | None = None):
    """Return (plays_df, players_df, ground_truth_dict)."""
    cfg = cfg or SynthConfig()
    rng = np.random.default_rng(cfg.seed)

    players = _make_players(cfg, rng)
    ability = players.set_index("player_id").ability.to_dict()

    # focus QB = the starting QB of team 0
    focus_qb = int(players[(players.team == 0) & (players.position == "QB")]
                   .sort_values("is_starter", ascending=False).player_id.values[0])
    focus_tau = _focus_qb_trajectory(cfg.weeks)

    tidx = _team_index(players)
    sched = _round_robin(cfg.n_teams, cfg.weeks, rng)

    rows = []
    play_id = 0
    drive_id = 0
    for (week, a, b) in sched:
        for off_team, def_team in ((a, b), (b, a)):
            # if focus QB's team is on offense and he's injured this week, his
            # team plays its backup (focus QB contributes nothing those weeks)
            focus_injured = (off_team == 0 and np.isnan(focus_tau[week - 1]))
            for _ in range(cfg.drives_per_team_per_game):
                drive_id += 1
                yardline = int(np.clip(rng.normal(75, 8), 60, 95))  # yards to goal
                down, togo = 1, 10
                drive_plays = []
                drive_points = 0.0
                for _p in range(cfg.max_plays_per_drive):
                    off_pl, def_pl = _pick_onfield(tidx[off_team], rng)
                    # focus QB weekly ability override
                    off_abil = 0.0
                    for pl in off_pl:
                        if pl == focus_qb and off_team == 0:
                            if focus_injured:
                                # backup is on field instead; replace QB slot
                                continue
                            off_abil += focus_tau[week - 1]
                        else:
                            off_abil += ability[pl]
                    if focus_injured:
                        # add a backup QB ability in place of focus QB
                        backup_qb = tidx[0][("off", "QB")]["backups"]
                        off_abil += ability[backup_qb[0]] if backup_qb else -0.05
                        off_pl = [p for p in off_pl if p != focus_qb]
                        if backup_qb:
                            off_pl = [backup_qb[0]] + off_pl
                    def_abil = sum(ability[pl] for pl in def_pl)
                    net = off_abil - def_abil

                    yards = cfg.yards_ability_gain * net + rng.normal(4.0, cfg.yards_noise_sd)
                    yards = float(np.clip(round(yards), -8, 60))

                    s = dict(down=down, ydstogo=togo, yardline_100=yardline)
                    new_yardline = yardline - yards
                    terminal = False
                    term_val = np.nan
                    pts = 0.0
                    if new_yardline <= 0:                    # touchdown
                        terminal, term_val, pts = True, 7.0, 7.0
                        drive_points = 7.0
                    else:
                        gained_first = yards >= togo
                        if gained_first:
                            ndown, ntogo = 1, 10
                        else:
                            ndown, ntogo = down + 1, int(max(1, togo - yards))
                        if ndown > 4:                        # turnover on downs
                            if new_yardline <= cfg.fg_range_yardline and rng.random() < 0.82:
                                terminal, term_val, pts = True, 3.0, 3.0   # FG
                                drive_points = 3.0
                            else:
                                terminal, term_val, pts = True, 0.0, 0.0   # downs
                                drive_points = 0.0
                        else:
                            nyardline = int(max(1, new_yardline))

                    rec = dict(
                        play_id=play_id, drive_id=drive_id, week=week,
                        off_team=off_team, def_team=def_team,
                        down=s["down"], ydstogo=s["ydstogo"], yardline_100=s["yardline_100"],
                        yards=yards, points=pts, terminal=terminal, terminal_value=term_val,
                        off_players=tuple(int(x) for x in off_pl),
                        def_players=tuple(int(x) for x in def_pl),
                    )
                    drive_plays.append(rec)
                    play_id += 1

                    if terminal:
                        break
                    down, togo, yardline = ndown, ntogo, nyardline

                # drive truncated at max plays without resolving -> stalls (0 pts)
                if drive_plays and not drive_plays[-1]["terminal"]:
                    drive_plays[-1]["terminal"] = True
                    drive_plays[-1]["terminal_value"] = 0.0
                    drive_points = drive_points or 0.0

                # attach drive_points and next-state to each play in the drive
                for i, rec in enumerate(drive_plays):
                    rec["drive_points"] = drive_points
                    if rec["terminal"]:
                        rec["n_down"] = -1; rec["n_ydstogo"] = -1; rec["n_yardline_100"] = -1
                    else:
                        nxt = drive_plays[i + 1]
                        rec["n_down"] = nxt["down"]; rec["n_ydstogo"] = nxt["ydstogo"]
                        rec["n_yardline_100"] = nxt["yardline_100"]
                    rows.append(rec)

    plays = pd.DataFrame(rows)
    gt = dict(
        ability=ability,
        team_strength=_team_strength(players, ability),
        focus_qb=focus_qb,
        focus_tau=focus_tau,
        league_factor=cfg.league_factor,
        players=players,
    )
    return plays, players, gt


def _team_strength(players: pd.DataFrame, ability: dict) -> dict:
    """A team's planted strength = mean starter offense ability minus mean
    starter defense ability (a clean scalar to validate market reconciliation)."""
    out = {}
    for team in players.team.unique():
        tp = players[players.team == team]
        off = tp[(tp.position.isin(OFF_ONFIELD)) & (tp.is_starter)].ability.mean()
        deff = tp[(tp.position == "DEF") & (tp.is_starter)].ability.mean()
        out[int(team)] = float(off - deff)
    return out


def make_college(players: pd.DataFrame, gt: dict, cfg: SynthConfig | None = None,
                 frac_rookies: float = 0.4):
    """Generate a feeder-league (college) season for a subset of players.

    Feeder SV = league_factor * true_ability + noise. 'Shared players' are the
    ones with both a feeder season and NFL snaps; regressing NFL on feeder
    recovers league_factor (with range restriction in reality). Returns a frame
    [player_id, position, feeder_sv, feeder_snaps, is_rookie]. Rookies have NO
    NFL history -> their prior must come entirely from the feeder mapping.
    """
    cfg = cfg or SynthConfig()
    rng = np.random.default_rng(cfg.seed + 99)
    rows = []
    for pid, abil in gt["ability"].items():
        pos = players.loc[players.player_id == pid, "position"].values[0]
        feeder = cfg.league_factor * abil + rng.normal(0, cfg.college_noise_sd)
        snaps = int(rng.integers(250, 900))
        is_rookie = rng.random() < frac_rookies
        rows.append(dict(player_id=int(pid), position=pos, feeder_sv=float(feeder),
                         feeder_snaps=snaps, is_rookie=bool(is_rookie)))
    return pd.DataFrame(rows)


if __name__ == "__main__":
    plays, players, gt = simulate()
    print("plays:", len(plays), "| players:", len(players),
          "| teams:", players.team.nunique(), "| weeks:", plays.week.nunique())
    print("scoring drives: TD rate %.3f  FG rate %.3f  avg drive pts %.3f" % (
        (plays.terminal_value == 7).mean() / (plays.terminal.mean()),
        (plays.terminal_value == 3).mean() / (plays.terminal.mean()),
        plays.groupby("drive_id").drive_points.first().mean()))
    print("focus QB:", gt["focus_qb"], "tau:", np.round(gt["focus_tau"], 3))
