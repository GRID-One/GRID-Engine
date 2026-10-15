# Task 1 Report — Canonical `player_id` ordering + accumulator order fingerprint

## Status: DONE

## What changed

### `backend/grid/layers.py`
- **`build_design`**: Changed `pids = list(players.player_id.values)` to
  `pids = sorted(players["player_id"].tolist(), key=str)`.
  Uses `key=str` rather than `.astype(str).tolist()` (the brief suggested the latter, but
  that coerces integer IDs to string objects, breaking the merge in `run_rapm` which joins
  `ratings["player_id"]` (now str) against `players["player_id"]` (int64) — causing dtype
  mismatch in pandas). With `key=str` the sort is stable and canonical, but original types
  are preserved.
- **`save_accumulators`**: Added optional `player_order: list[str] | None = None` parameter.
  When provided, stored as `np.array(player_order, dtype=object)` in the npz under key
  `"player_order"`. Backward-compatible: old callers that pass no `player_order` still work
  (the key is simply absent from the npz).
- **`load_accumulators`**: Return type extended from `(XtX, Xty, week)` 3-tuple to
  `(XtX, Xty, week, player_order)` 4-tuple. If `"player_order"` key is absent in the npz
  (old files), returns `[]` for that field. Uses `allow_pickle=True` for object-array load.

### `backend/pipeline/weekly_update.py`
- Computes `current_player_order = [str(p) for p in sorted(players_df["player_id"].tolist(), key=str)]`
  (str-serialized for stable comparison regardless of original dtype).
- Unpacks `load_accumulators()` as 4-tuple: `XtX, Xty, last_week, stored_player_order = loaded`.
- Adds reorder reinit guard: if `stored_player_order` is non-empty and differs from
  `current_player_order`, logs a warning and reinits accumulators.
- `save_accumulators(...)` call now passes `player_order=current_player_order`.

### `tests/grid/test_layers.py` (new file)
Three new tests per the brief (verbatim names):
1. `test_build_design_player_columns_sorted` — verifies column index of a known pid matches sorted position.
2. `test_build_design_order_stable_across_row_shuffle` — shuffling `players_df` rows yields identical `colidx["p_col"]`.
3. `test_accumulator_reinit_on_player_reorder` — stores then reloads with a different (unsorted) order; verifies
   4-tuple return, that `player_order` is stored faithfully, and that a reorder is detectable vs canonical sorted order.

### Existing tests updated (3 files)
- **`tests/grid/test_incremental.py`** line 90: `loaded_XtX, loaded_Xty, week = result` → `..., _player_order = result`
- **`tests/pipeline/test_weekly_update.py`** line 114: `_, _, last_week = loaded` → `_, _, last_week, _player_order = loaded`
- **`tests/grid/test_performance.py`** line 44 (reference impl `_build_design_reference`):
  `pids = list(players.player_id.values)` → `pids = sorted(players["player_id"].tolist(), key=str)`
  (the reference is used for vectorized-vs-row-loop parity; it must use the same sort as production).

## `run_rapm` unaffected
Verified: `run_rapm` calls `build_design` internally and derives `inv_pcol` from its own `colidx["p_col"]`,
so the player → column mapping it uses is always consistent with the matrix it built. No changes needed there.

## Full-suite test output
```
260 passed, 1 warning in 22.13s
```
(257 baseline + 3 new tests)

## Concerns
None. The `key=str` sort deviation from the brief's `.astype(str)` is intentional and correct: preserving
original ID types prevents a regression in `run_rapm`'s merge while still achieving stable canonical ordering.
The behavior is identical for string IDs (all real data) and correct for integer IDs (synthetic test data).
