# Task 7 Report — Optional WR-CB Interaction Columns

## Status: COMPLETE

## Files changed
- `backend/grid/layers.py` — added `_build_interaction_block()` helper and `interactions=False` / `min_pair_plays=30` params to `build_design`; added heavy-penalty ridge mask for interaction columns in `run_rapm`
- `backend/grid/synth.py` — added `cb_split: bool = False` and `n_cb_per_team: int = 2` fields to `SynthConfig`; re-labels first `n_cb_per_team` DEF starters as position "CB"; updated `_team_index` to include both "DEF" and "CB" in defensive pools
- `tests/grid/test_design_interactions.py` — 5 new tests (all pass)

## Test summary
```
5 passed in 2.73s   (tests/grid/test_design_interactions.py)
285 passed in 21.5s (full suite — was 280, +5 new)
```

## What was done

### layers.py
- `build_design` signature: `(plays, players, interactions=False, min_pair_plays=30)`
- When `interactions=False` (default): returns byte-for-byte identical `X, y, colidx` — zero change to existing code path
- When `interactions=True`: calls `_build_interaction_block` which:
  1. Uses "CB" as proxy if any player has position "CB", otherwise falls back to "DEF" (positional approximation — see honesty note below)
  2. Counts (WR, CB-proxy) co-occurrence across all plays
  3. Prunes pairs with count < `min_pair_plays`
  4. Appends interaction columns after the base block via `sp.hstack`
  5. Records `colidx['interactions'] = {(wr_id, cb_id): col_index}` with indices ≥ base `nCols`
  6. Bumps `colidx['nCols']`
- `run_rapm` ridge mask: heavy penalty (`10.0`) applied to all interaction columns when `"interactions"` key is in `colidx` — this block is unreachable on the default path so the no-interactions ridge mask is unchanged

### synth.py
- `SynthConfig.cb_split = False` (default OFF — all existing synth tests unchanged)
- `SynthConfig.n_cb_per_team = 2`
- When `cb_split=True`: the first `n_cb_per_team` DEF starters per team get position "CB" instead of "DEF"; ability distribution and on-field count unchanged
- `_team_index` updated to include both "DEF" and "CB" in the defensive pool so lineups still work correctly

## Positional-approximation honesty note
The interaction pairing uses **positional proxy only** — every on-field WR is paired with every on-field CB (or DEF if no CB present). This is an approximation because real route-tree / assignment data is unavailable. In production, true WR-CB matchup data (e.g. from Next Gen Stats alignment columns) would replace the positional proxy, and `build_design` is structured to accept that without interface changes.

## Blocking concerns
None.
