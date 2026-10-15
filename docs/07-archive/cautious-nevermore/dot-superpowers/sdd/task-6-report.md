# Task 6 Report — Per-situation keyed incremental accumulators

## Status: COMPLETE

## Commit
To be filled after commit (see below).

## Test summary
280 passed, 0 failed (276 pre-existing + 4 new in `tests/pipeline/test_weekly_situations.py`)

## What was done

### `backend/grid/layers.py`
- Added `_accum_path(key='all') -> Path` helper: returns legacy `_ACCUM_PATH` for `key='all'`, else `rapm_accumulators_{key}.npz` sibling.
- Added `key: str = 'all'` param to both `save_accumulators` and `load_accumulators`; both delegate path resolution to `_accum_path`. Legacy callers with no `key` arg are unaffected.

### `backend/pipeline/weekly_update.py`
- Added `MIN_PLAYS = 50` module constant (min plays for a situation sub-frame solve).
- Added `from backend.grid.situations import classify as classify_situations` import.
- Updated `_upsert_matchup_grades` signature: added `situation: str = 'overall'` param; it is now forwarded into the INSERT/ON CONFLICT SQL instead of hardcoded `'overall'`.
- Updated the overall-pass `load_accumulators()` / `save_accumulators()` call sites to explicitly pass `key='all'`.
- Added situation loop after the overall solve: for each situation from `classify_situations(plays_df)`, applies identical dim+order reinit guard keyed per-situation, accumulates, saves with `key=sit_name`, fits beta, upserts grades tagged with `situation=sit_name`.
- Guarded the situation pass: if plays frame lacks `down`/`ydstogo`/`yardline_100`, the pass is skipped (backward-compat with test data that omits these columns).

### `tests/pipeline/test_weekly_situations.py` (new, 4 tests)
1. `test_situation_accumulator_files_created_per_situation` — verifies `_accum_path('all')` == legacy path AND per-situation npz files are created.
2. `test_situation_dim_mismatch_reinitialises` — pre-plants a 2×2 situation file, confirms run completes without error.
3. `test_rerun_week_skips_all_situation_accumulators` — pre-plants both overall and situation accumulators at week=5, confirms second run returns `skipped=True` and sentinel situation acculator is untouched.
4. `test_situation_grades_written_with_situation_tag` — verifies `matchup_grades` contains both `'overall'` and at least one named situation tag.

## Blocking concerns
None.
