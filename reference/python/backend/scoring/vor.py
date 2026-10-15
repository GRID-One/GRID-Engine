from __future__ import annotations


def calculate_vor(
    players: list[dict],
    roster_slots: dict[str, int],
    num_teams: int,
) -> list[dict]:
    """Calculate Value Over Replacement for each player.

    Replacement level for a position is the projected points of the player
    at rank (slots * num_teams + 1) — the first player who would go
    undrafted. VOR = player projected_points - replacement level.

    Players at positions not listed in roster_slots receive vor=0.0 — except
    FLEX-eligible positions (RB/WR/TE) when FLEX slots exist, whose replacement
    level is the FLEX level (they compete for the FLEX spot).

    Returns a new list of player dicts with added fields:
      - vor (float)
      - tier (int, assigned by assign_tiers)
      - rank (int, overall rank by VOR descending, 1-indexed)
    """
    # Group players by position
    by_position: dict[str, list[dict]] = {}
    for p in players:
        pos = p["position"]
        by_position.setdefault(pos, []).append(p)

    # Compute replacement level per position
    replacement_level: dict[str, float] = {}
    for pos, slots in roster_slots.items():
        pos_players = sorted(
            by_position.get(pos, []),
            key=lambda x: x["projected_points"],
            reverse=True,
        )
        repl_idx = slots * num_teams
        if repl_idx < len(pos_players):
            replacement_level[pos] = pos_players[repl_idx]["projected_points"]
        elif pos_players:
            # Fewer players than roster spots — last player is replacement level
            replacement_level[pos] = pos_players[-1]["projected_points"]
        else:
            replacement_level[pos] = 0.0

    # FLEX pass: raise replacement level for FLEX-eligible positions
    flex_slots = roster_slots.get("FLEX", 0)
    if flex_slots > 0:
        flex_eligible = {"RB", "WR", "TE"}
        flex_pool = []
        for pos in flex_eligible:
            flex_pool.extend(by_position.get(pos, []))
        flex_pool.sort(key=lambda x: x["projected_points"], reverse=True)
        total_flex_demand = sum(roster_slots.get(p, 0) for p in flex_eligible) + flex_slots
        flex_repl_idx = total_flex_demand * num_teams
        if flex_repl_idx < len(flex_pool):
            flex_repl = flex_pool[flex_repl_idx]["projected_points"]
        elif flex_pool:
            flex_repl = flex_pool[-1]["projected_points"]
        else:
            flex_repl = 0.0
        for pos in flex_eligible:
            if pos in replacement_level:
                replacement_level[pos] = max(replacement_level[pos], flex_repl)
            elif by_position.get(pos):
                # FLEX-only position (no dedicated slot in roster_slots): its
                # players compete solely for FLEX, so the FLEX level IS their
                # replacement level — without this they'd all get vor=0.0 and
                # never rank for the FLEX spot they can legitimately fill.
                replacement_level[pos] = flex_repl

    # Build result list with VOR
    result: list[dict] = []
    for p in players:
        pos = p["position"]
        repl = replacement_level.get(pos, None)
        if repl is None:
            vor = 0.0
        else:
            vor = p["projected_points"] - repl
        result.append({**p, "vor": vor})

    # Sort by VOR descending and assign overall rank
    result.sort(key=lambda x: x["vor"], reverse=True)
    for i, p in enumerate(result):
        p["rank"] = i + 1

    return assign_tiers(result)


def assign_tiers(players: list[dict], max_gap_pct: float = 0.10) -> list[dict]:
    """Group players into tiers based on VOR value clusters.

    A new tier begins when the gap between consecutive players' VOR values
    exceeds max_gap_pct of the higher player's VOR (using absolute value
    to handle negative VOR gracefully).

    Players are returned sorted by VOR descending with a 'tier' field added.
    Tier numbers start at 1 (best players).
    """
    if not players:
        return players

    sorted_players = sorted(players, key=lambda x: x.get("vor", 0), reverse=True)
    tier = 1
    sorted_players[0]["tier"] = tier

    for i in range(1, len(sorted_players)):
        prev_vor = sorted_players[i - 1].get("vor", 0)
        curr_vor = sorted_players[i].get("vor", 0)
        denom = abs(prev_vor) if prev_vor != 0 else 1.0
        gap = (prev_vor - curr_vor) / denom

        if gap > max_gap_pct:
            tier += 1
        sorted_players[i]["tier"] = tier

    return sorted_players
