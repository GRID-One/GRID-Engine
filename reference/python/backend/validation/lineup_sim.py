"""H2 lineup simulation (roadmap Phase 2c; validation plan §9, §11).

The **H2 headline** (injury-replacement / weekly): a GRID-set weekly lineup
beats a last-season-set lineup on **realized weekly point margin** — overall
and on the starter-OUT subset — with a bootstrap CI that excludes 0.  This
module supplies the machinery:

* **Synthetic rosters** — a deterministic VOR-greedy snake draft
  (:func:`draft_rosters`, reusing :func:`~backend.scoring.vor.calculate_vor` and
  :func:`snake_order`) builds realistic
  starters-plus-bench rosters from a *shared* projection, so roster construction
  never favors either compared method.
* **One slot-filling rule** — :func:`set_lineup` is the single greedy
  start/sit rule used for *both* methods; only the projection fed to it may
  differ (the §8 leakage constraint: any difference in realized margin must come
  from the projections, never from the lineup mechanics).
* **The margin** — :func:`simulate_weekly_margin` scores
  ``sum(actual(method-A lineup)) - sum(actual(method-B lineup))`` per roster x week
  and bootstraps it (:func:`~backend.validation.metrics.bootstrap_ci`), overall
  and on the **starter-OUT** subset (cells where a player the *reference*
  method would have started is unavailable — the injury-replacement scenario).

Pure and deterministic (the draft breaks ties by rank, the lineup by player_id,
the bootstrap by seed) so the synth H2 gates stay in the CI budget (§7.5 A).
The real H2 numbers come from the Phase-2c verdict run, not CI.
"""
from __future__ import annotations

from dataclasses import dataclass, field

from backend.scoring.vor import calculate_vor
from backend.validation.metrics import BootstrapCI, bootstrap_ci

FLEX_ELIGIBLE = ("RB", "WR", "TE")


def snake_order(num_teams: int, num_rounds: int) -> list[int]:
    """Return list of team_slot (1-based) for every pick in a snake draft.

    Reference-oracle patch P1: the function body is a verbatim copy of the
    app-side ``backend.services.mock_draft.snake_order`` (cautious-nevermore
    @ 59bce1d, mock_draft.py:38-46; only this docstring is extended). It is
    inlined so the engine reference never imports the app-only
    ``backend.services`` package. See reference/python/MANIFEST.tsv.
    """
    order = []
    for r in range(num_rounds):
        teams = list(range(1, num_teams + 1))
        if r % 2 == 1:
            teams = list(reversed(teams))
        order.extend(teams)
    return order


# --------------------------------------------------------------------------- #
# Roster construction                                                          #
# --------------------------------------------------------------------------- #
def draft_rosters(
    players: list[dict],
    *,
    roster_slots: dict[str, int],
    num_teams: int,
    num_rounds: int,
) -> list[list[dict]]:
    """Deterministic VOR-greedy snake draft → ``num_teams`` rosters.

    ``players`` need ``player_id``, ``position`` and ``projected_points`` (the
    *shared* draft-time projection — deliberately not either compared method's
    weekly numbers, so the rosters are method-neutral).  Each pick takes the
    highest-VOR available player who fills a remaining starting need (dedicated
    slot first, then FLEX for :data:`FLEX_ELIGIBLE` positions); once a team's
    starting slots are filled, remaining rounds take best-available (bench).
    No randomness — unlike the mock-draft AI this is a validation harness, and
    the same pool must always yield the same rosters.
    """
    pool = calculate_vor(players, roster_slots, num_teams)
    pool = sorted(pool, key=lambda p: p["rank"])
    order = snake_order(num_teams, num_rounds)

    rosters: list[list[dict]] = [[] for _ in range(num_teams)]
    needs = [
        {pos: n for pos, n in roster_slots.items() if pos != "FLEX"}
        for _ in range(num_teams)
    ]
    flex_needs = [roster_slots.get("FLEX", 0) for _ in range(num_teams)]
    taken: set = set()

    for team_slot in order:
        t = team_slot - 1
        pick = None
        if any(n > 0 for n in needs[t].values()) or flex_needs[t] > 0:
            for p in pool:
                if p["player_id"] in taken:
                    continue
                pos = p["position"]
                if needs[t].get(pos, 0) > 0:
                    pick = p
                    needs[t][pos] -= 1
                    break
                if flex_needs[t] > 0 and pos in FLEX_ELIGIBLE:
                    pick = p
                    flex_needs[t] -= 1
                    break
        if pick is None:  # bench round, or nothing fills a need: best available
            pick = next((p for p in pool if p["player_id"] not in taken), None)
        if pick is None:  # pool exhausted
            continue
        taken.add(pick["player_id"])
        rosters[t].append(pick)

    return rosters


# --------------------------------------------------------------------------- #
# Slot filling — the ONE rule both methods share (§8)                          #
# --------------------------------------------------------------------------- #
def set_lineup(
    roster: list[dict],
    projections: dict,
    roster_slots: dict[str, int],
    *,
    out: frozenset | set = frozenset(),
) -> list[str]:
    """Greedy start/sit: fill each slot with the highest-*projected* available player.

    This is the **single** lineup rule for every compared method — callers vary
    only ``projections`` (``{player_id: projected points}``), never the
    mechanics, so H2's margin isolates projection quality (§8).  Players in
    ``out`` are unavailable that week.  Iterates the roster in descending
    projection (ties broken by ``player_id`` so the lineup is deterministic),
    taking a player's dedicated slot first, else an open FLEX slot for
    :data:`FLEX_ELIGIBLE` positions.  A rostered player with no projection
    counts as 0.0 — benched behind anyone with a positive number, but still
    eligible so a slot is never left empty while a body exists.  Returns the
    starting ``player_id`` list.
    """
    ranked = sorted(
        (p for p in roster if p["player_id"] not in out),
        key=lambda p: (-float(projections.get(p["player_id"], 0.0)), p["player_id"]),
    )
    need = {pos: n for pos, n in roster_slots.items() if pos != "FLEX"}
    flex_left = roster_slots.get("FLEX", 0)
    starters: list[str] = []
    for p in ranked:
        pos = p["position"]
        if need.get(pos, 0) > 0:
            need[pos] -= 1
            starters.append(p["player_id"])
        elif flex_left > 0 and pos in FLEX_ELIGIBLE:
            flex_left -= 1
            starters.append(p["player_id"])
    return starters


# --------------------------------------------------------------------------- #
# The H2 margin                                                                #
# --------------------------------------------------------------------------- #
@dataclass
class LineupSimResult:
    """Realized weekly point margins of method A over method B, with CIs.

    ``margins`` is one realized ``sum(actual(A lineup)) - sum(actual(B lineup))``
    per roster x week cell; ``overall`` is its bootstrap CI (the H2 gate is
    ``point > 0`` with ``excludes_zero``).  ``win_rate`` is the fraction of
    cells A wins (ties count ½ — identical lineups must land at 0.5, not 0).
    ``starter_out`` restricts to cells where a player the reference method
    would have started (were everyone available) is OUT — the
    injury-replacement subset; ``None`` when no cell qualifies.
    """

    margins: list[float]
    overall: BootstrapCI
    win_rate: float
    starter_out: BootstrapCI | None = None
    starter_out_margins: list[float] = field(default_factory=list)


def simulate_weekly_margin(
    rosters: list[list[dict]],
    proj_a: dict,
    proj_b: dict,
    actual: dict,
    roster_slots: dict[str, int],
    *,
    out: dict | None = None,
    n: int = 10000,
    alpha: float = 0.05,
    seed: int = 0,
) -> LineupSimResult:
    """Score method A vs method B on realized weekly lineup points.

    For every roster and every week in ``actual``, both methods set a lineup
    from the same roster with the same rule (:func:`set_lineup`) and the same
    availability — only the projection differs — then the week's **realized**
    points decide the margin.  ``proj_a``/``proj_b``/``actual`` map
    ``week -> {player_id: points}``; ``out`` optionally maps
    ``week -> set(player_id)`` unavailable that week.  A player with no
    ``actual`` entry scores 0.0 (an OUT starter who was wrongly left in costs
    exactly their absence).  Margins are bootstrapped via
    :func:`~backend.validation.metrics.bootstrap_ci` (deterministic under
    ``seed``), overall and on the starter-OUT subset.
    """
    out = out or {}
    margins: list[float] = []
    starter_out_margins: list[float] = []
    wins = 0.0

    for roster in rosters:
        for week in sorted(actual):
            out_w = out.get(week, set())
            pa, pb = proj_a.get(week, {}), proj_b.get(week, {})
            lineup_a = set_lineup(roster, pa, roster_slots, out=out_w)
            lineup_b = set_lineup(roster, pb, roster_slots, out=out_w)
            realized = actual[week]
            margin = (sum(realized.get(pid, 0.0) for pid in lineup_a)
                      - sum(realized.get(pid, 0.0) for pid in lineup_b))
            margins.append(margin)
            wins += 1.0 if margin > 0 else (0.5 if margin == 0 else 0.0)

            # Injury-replacement subset: the reference method's would-be
            # starter (with everyone available) is OUT this week.
            if out_w:
                would_start_b = set_lineup(roster, pb, roster_slots)
                if any(pid in out_w for pid in would_start_b):
                    starter_out_margins.append(margin)

    overall = bootstrap_ci(margins, n=n, alpha=alpha, seed=seed)
    starter_out = (bootstrap_ci(starter_out_margins, n=n, alpha=alpha, seed=seed)
                   if starter_out_margins else None)
    return LineupSimResult(
        margins=margins,
        overall=overall,
        win_rate=wins / len(margins) if margins else float("nan"),
        starter_out=starter_out,
        starter_out_margins=starter_out_margins,
    )
