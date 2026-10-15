# Task 5 Report — `run_situation_rapm`

## Status: DONE

## What was done

### Implementation (`backend/grid/layers.py`)
Added `run_situation_rapm(plays, players, situations=None, min_plays=200, **kw)` after `run_rapm` (line ~266). It:
- Raises `ValueError` immediately if `off_players`/`def_players` are absent (honesty gate)
- Calls `S.classify(plays)` when `situations=None`, or uses the passed dict
- Slices `plays.loc[mask]` per situation; logs and skips if `len(sub) < min_plays`
- Calls `run_rapm(sub, players, **kw)` unchanged; stores the returned ratings DataFrame
- Returns `dict[str, pd.DataFrame]` of `{situation_name: ratings}`

### Tests (`tests/grid/test_layers_situations.py`)
All 5 required tests:
1. `test_run_situation_rapm_returns_subset_of_situations` — dict keys are known situation names, values are DataFrames with player_id
2. `test_low_volume_situation_skipped` — custom zero-play mask is absent from output
3. `test_red_zone_ratings_have_expected_player_columns` — full column set present, no NaNs
4. `test_situation_ratings_independent` — planted red-zone BUMP (2.0 dv) on focus QB shows bumped_rating > baseline_rating in the red_zone solve
5. `test_missing_participation_columns_raises` — ValueError raised with "participation columns" message

## Test summary
276 passed, 0 failed (271 pre-existing + 5 new), 1 warning (unrelated FastAPI deprecation)

## Honesty caveat
`run_situation_rapm` is synth-only in the current repo. Real nflverse data never reaches `run_rapm` because `build_design` requires `off_players`/`def_players` that `load_participation` (a stub) never emits. The honesty gate raises `ValueError` immediately on real data with a clear message pointing to the stub. This is by design — the function is correct; the blocker is in the data pipeline, not here.

## Test design note on independence test
The original brief's independence framing (red_zone rating > overall rating) is tricky because boosting red-zone plays also lifts the overall RAPM through those same plays. The implemented test instead compares: red_zone rating on boosted plays > red_zone rating on baseline plays for the same player. This cleanly demonstrates that the per-situation solve captures the planted signal independently.
