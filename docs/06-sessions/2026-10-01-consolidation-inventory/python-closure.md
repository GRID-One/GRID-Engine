# Python reference-engine closure: what to import into `GRID-Engine/reference/python/`

Source: `/home/user/cautious-nevermore` @ `59bce1d` (main; identical to `origin/main` and to the
checked-out `claude/grid-engine-consolidation-e7kmh9`). All unmerged remote branches were checked
for engine-path changes. `origin/claude/project-sync-updates-db6ceb` carries the P1 `RateModel.predict`
NaN guard and the A3 `resolve_or_create_format` hardening, but both are already on main (squash-merged
as #94). The other branches touch no engine path. **main@59bce1d is the complete, current engine.** No
engine `.py`/`.npz`/`.json`/`.sql` file was ever deleted in history, so nothing needs recovering from git.

Scratch assembly (proved runnable): `/tmp/claude-0/-home-user/693e74a1-f8af-5256-86e9-2299b8697223/scratchpad/inventory/scratch-python/reference/python/`
(105 upstream files + 5 new support files). Patches are in `scratch-python/lineup_sim.patch` and
`scratch-python/run_demo.patch`.

---

## 0. Bottom line

* **Import set:** 105 upstream files. 103 are byte-identical (sha256 verified against `git show 59bce1d:<path>`),
  1 has a **required** patch (`backend/validation/lineup_sim.py`: inline `snake_order`, which cuts the only
  edge into app-only `backend.services`), and 1 has an **optional** one-line patch (`run_demo.py`: make the
  hard-coded `/mnt/user-data/outputs` overridable via `GRID_DEMO_OUT`).
* **New files:** `requirements.txt` (engine-only), `requirements-demo.txt` (adds matplotlib for `run_demo.py`),
  `requirements.lock` (exact verified versions), `pytest.ini` (pins rootdir and `pythonpath = .`), `.gitignore`.
* **Package name stays `backend.*`.** There's no strong reason to rename (section 2).
* **Proof:** In a **clean venv built from the engine `requirements.txt` alone** (no fastapi/httpx/pydantic/anthropic/posthog/websockets; matplotlib was added later only for the demo), with network sockets and app-stack/app-package imports, including matplotlib, hard-blocked by a guard plugin that recorded 0 attempts, `cd reference/python && python -m pytest` gives **446 passed, 0 failed, 0 skipped, 0 warnings, in about 144-149 s**. That is the identical 446 node-ID set the original repo runs for these directories (baseline in-scope: 446 passed, 141 s; baseline full suite: 637 passed, 147 s). `run_demo.py` runs in 9.5 s, and its printed output and recovery PNG are byte-identical to the unpatched upstream run.
* **GRID-Engine integration blockers that the import PR must handle** (section 7). `typos` reports 152
  false positives on the verbatim tree, so add `reference/python/` to `_typos.toml` `extend-exclude`.
  `check-traceability.sh` needs a WP/ADR that names `reference/python/` as a directory. The secret scan
  passes. There is also a dotenv walk-up quirk to document.

---

## 1. Import graph (complete, AST-derived, includes function-local imports)

Method: `ast.walk` over every candidate module, recording every `from backend…` / `import backend…` with
its enclosing function (script: `scratch-python/imports.py`). The third-party scan is in
`scratch-python/thirdparty.py`.

### 1.1 Intra-engine edges (all resolve inside the import set)

| Module | Imports (module-level unless noted) |
|---|---|
| `backend/grid/__init__.py` | grid.synth, grid.value, grid.layers, grid.statespace, grid.priors, grid.data_adapters |
| `backend/grid/data_adapters.py` | grid.synth; **local** `load_participation()` L57 → grid.nflverse_loader |
| `backend/grid/layers.py` | grid.value (L38); **local** `run_situation_rapm()` L467 → grid.situations |
| `backend/grid/nflverse_adapter.py` | **local** `load_grid_plays()` L53 → grid.nflverse_loader |
| `backend/grid/nflverse_loader.py` | grid.cache |
| `backend/projection/features.py` | grid.statespace |
| `backend/projection/model.py` | projection.volume, scoring.columns, scoring.engine |
| `backend/projection/preseason.py` | projection.model, projection.volume, scoring.columns |
| `backend/projection/sv_to_points.py`, `volume.py` | validation.asof |
| `backend/validation/__init__.py` | validation.sniff |
| `backend/validation/backtest.py` | grid.layers, grid.value, validation.asof; **local** `_log_subset()` L212 → pipeline._logging |
| `backend/validation/baselines.py` | validation.asof |
| `backend/validation/lineup_sim.py` | scoring.vor, **services.mock_draft (L32), cut by PATCH P1**, validation.metrics |
| `backend/validation/report.py`, `tier2.py` | validation.metrics |
| `backend/validation/tier1.py` | projection.sv_to_points, validation.{baselines,asof,metrics} |
| `backend/validation/verdict.py` | grid.layers, grid.value, projection.{features,model,sv_to_points,volume}, scoring.{columns,formats}, validation.{baselines,asof,backtest,lineup_sim,report,thresholds,tier1,tier2}; **local** `_load_players_frame()` L521 → grid.nflverse_loader; **local** `_load_stats()` L539 → db.connection; **local** `_load_history()` L556 → scoring.engine |
| `backend/scoring/formats.py` | scoring.engine |
| `backend/scoring/format_registry.py` | scoring.engine, scoring.formats |
| `backend/pipeline/compute_valuations.py` | db.connection, scoring.{engine,formats,vor,format_registry}, pipeline.{_logging,health_check}, validation.sniff |
| `backend/pipeline/data_pipeline.py` | db.connection, grid.nflverse_loader, pipeline.{_logging,health_check}, scoring.columns |
| `backend/pipeline/ingest_grid.py` | pipeline._logging, grid.nflverse_adapter |
| `backend/pipeline/weekly_update.py` | db.connection, grid.layers, grid.situations, grid.statespace, pipeline.{_logging,compute_valuations,health_check}; **local** `run()` L201-203 → grid.nflverse_adapter, grid.nflverse_loader, grid.value |
| `backend/db/seed_coaching.py` | **local** L37 → db.connection |
| `run_demo.py` | grid (public API) |

Script-mode `__main__` blocks in `grid/layers.py:614-615`, `grid/priors.py:175-177`, `grid/statespace.py:421-424`
and `grid/value.py:114` use bare `from synth import …` / `from value import …`. They only work with
`cwd=backend/grid` (`python layers.py`). They are inert under package import and need no action.

### 1.2 Every edge into app-only code, and its cut

| # | Edge (file:line) | Used for | Cut |
|---|---|---|---|
| E1 | `backend/validation/lineup_sim.py:32` `from backend.services.mock_draft import snake_order` | `draft_rosters()` L61: pick order for the deterministic VOR-greedy snake draft in the H2 lineup simulation. `verdict.py:51` imports `draft_rosters` from it. | **PATCH P1**: inline the 9-line, stdlib-only `snake_order` into `lineup_sim.py` (the AST body was verified identical to `mock_draft.py:38-46`). This drops the whole `backend/services/` package. A stub `backend/services/mock_draft.py` was rejected: it would be an upstream path with 235 lines deleted, which is a bigger and more misleading diff than +16/-2. |
| E2 | `backend/validation/verdict.py:539` (local) `from backend.db.connection import get_db` | `_load_stats()` reads `player_stats` (filled by `data_pipeline`) for the real-data verdict | **Not app-only. Keep** `backend/db/` (engine pipeline storage). No cut. |
| E3 | `backend/pipeline/sync_leagues.py:24-25` → `backend.adapters.espn/sleeper` | league sync | **Drop** `sync_leagues.py` (app-only) and `tests/pipeline/test_sync_leagues.py`. |
| E4 | `backend/pipeline/sync_trade_history.py:216,222` (local, in `run()` try) → `backend.adapters.espn/sleeper` | trade-history sync | **Drop** `sync_trade_history.py` and `tests/pipeline/test_sync_trade_history.py`. |
| E5 | `backend/pipeline/compute_valuations.py:20` → `backend.scoring.format_registry` | `ensure_preset_formats(conn)` (writes the STANDARD/HALF/FULL rows into `scoring_formats`), `DEFAULT_ROSTER_SLOTS` | **Not app-only. Keep `format_registry.py` verbatim.** `resolve_or_create_format()` (L36-83) is only called by `sync_leagues.py` and stays as dead code in the reference. Its tests (`TestResolveOrCreateFormat`, part of the dropped `test_sync_leagues.py`) leave with it. |
| E6 | `sync_leagues.py:29` → `format_registry.resolve_or_create_format` | app league formats | Covered by E3. |

There are no other edges. With `isolation_guard.py`, all 46 backend modules of the import set import in a
fresh interpreter, no app-stack module (fastapi/uvicorn/httpx/pydantic/anthropic/posthog/websockets/
starlette/matplotlib/requests) is loaded, and no `backend.{api,services,adapters,trades,viz_agent}` import
is attempted.

The reverse direction (app consuming the engine) is also clean. `api/routes/stats.py:245` reads
`KalmanState`/`_KALMAN_PATH`, `api/routes/draft.py:10` reads `DEFAULT_ROSTER_SLOTS`, and the adapters use
`ScoringConfig`. `backend/trades/trade_model.py` (RSV: projected FP × injury discount + matchup-grade schedule
adjustment + VOR scarcity + Kalman trajectory) *consumes* engine outputs and holds no engine estimator.
It is classified app-only per scope (see Decisions).

### 1.3 Decisions on the "decide" items

| Item | Decision | Why |
|---|---|---|
| `scoring/vor.py` | **Import** | Hard dependency of `compute_valuations` (VOR/tiers into `valuations`) and `validation/lineup_sim` (H2 draft). It is pure, with no deps. |
| `scoring/format_registry.py` | **Import verbatim** | `compute_valuations` needs `ensure_preset_formats` and `DEFAULT_ROSTER_SLOTS`. Splitting out `resolve_or_create_format` would be a needless patch. |
| `pipeline/data_pipeline.py` | **Import** | It is the only writer of `players` and `player_stats`, which `compute_valuations._load_players_with_stats` and `verdict._load_stats` read. That makes it the engine's real-data box-score ingest. |
| `pipeline/health_check.py` | **Import** | Imported at module level by `data_pipeline`, `compute_valuations` and `weekly_update`. It is stdlib-only (60 lines). Cutting it would mean patching 3 files. |
| `pipeline/_logging.py` | **Import** | Imported by 4 engine modules (plus a local import at `validation/backtest.py:212`). It is stdlib-only. |
| `db/schema.sql` | **Import verbatim** (including app tables) | `tests/db/test_schema.py:17-22` `EXPECTED_TABLES` asserts the app tables (`leagues`, `rosters`, `draft_history`, `saved_views`, `draft_sessions`, `draft_queue`, `trade_history`), and `test_leagues_columns`/`test_trade_history_columns` assert their columns. Trimming would mean patching both files. The app DDL is inert `CREATE TABLE IF NOT EXISTS`. Engine-used tables: `players`, `player_stats`, `scoring_formats`, `projections`, `valuations`, `matchup_grades`, `coaching_changes`, `kalman_trajectory` (`situation_grades` exists but no engine module writes it). App-only tables: `leagues`, `rosters`, `draft_history`, `saved_views`, `draft_sessions`, `draft_queue`, `trade_history`, `viz_query_cache`. If a later WP wants an engine-only schema, it patches `schema.sql` and `test_schema.py` together. |
| `db/seed_coaching.py` + `db/data/coaching_changes_2025.json` | **Import** | Seeds `coaching_changes`. `weekly_update.run()` L321-344 turns those rows into Kalman `scheme_resets` (the scheme_fit component). `tests/grid/test_coaching_changes.py:287` uses it. |
| `backend/fantasy_scoring.py` + `tests/test_fantasy_scoring.py` | **Exclude** | It is an orphaned predecessor: hand-picked `GRID_SCALING` constants, explicitly superseded according to the docstrings of `projection/model.py:3`, `projection/volume.py:14` and `projection/sv_to_points.py:14` ("the orphaned `fantasy_scoring.py`"). Nothing imports it except its own 3 tests. |
| `run_demo.py` | **Import + optional PATCH P2** + `requirements-demo.txt` | It needs `matplotlib`, which upstream `requirements.txt` never declared, and it does `os.makedirs("/mnt/user-data/outputs")` at import time (a Claude.ai-sandbox path that is not writable on a GitHub runner). Its correlations are already asserted by `tests/grid/test_tier0_recovery.py`, so the demo is documentation, not a gate. |

---

## 2. Package name: keep `backend.*`

Keep it. Byte-identity with upstream is what makes the oracle auditable: `diff -r` against
`cautious-nevermore@59bce1d` shows exactly two patched files. Renaming would touch all 282 `from backend.` /
`import backend` lines across 77 files.

The name is ugly in an engine repo, but nothing collides. GRID-Engine's root `tests/guards/` has no
`__init__.py`, so it is at most a namespace portion, and a regular package (`reference/python/tests/__init__.py`)
always wins over namespace portions. The Rust crates never see the Python tree. The only real cost is
cosmetic, so it is not worth a 300-line diff. (If a rename is ever wanted, do it as one mechanical commit
*after* the verbatim import, so the import commit stays a pure copy.)

---

## 3. Running the closure: test results

All runs used Python 3.11.15 on linux x86_64 (4 vCPU), with numpy 2.4.6, pandas 3.0.6, scipy 1.17.1,
scikit-learn 1.9.1 and pyarrow 25.0.1. The clean venv and the system site-packages have the same versions
for these, so the comparisons are like-for-like. The baseline is a pristine `git archive 59bce1d` copy
(`scratch-python/baseline/`), so the read-only repo was never executed in place.

| Run | Env | Tests | Result | Wall |
|---|---|---|---|---|
| **Scratch `reference/python`**, `python -m pytest`, isolation guard on | clean venv (requirements.txt; demo extra present but guard-blocked), `OMP/OPENBLAS/MKL_NUM_THREADS=1` | 446 | **446 passed**, 0 warnings, 0 guard hits | 144.3 s (ran concurrently with the next row) |
| Scratch, same, default threading, run alone | clean venv | 446 | **446 passed** | 149.2 s |
| Baseline, same 446 in-scope node IDs | system python (full app stack installed), single-thread | 446 | **446 passed** | 141.0 s |
| Baseline, full upstream suite | system python, single-thread | 637 | **637 passed**, 2 warnings (both FastAPI `on_event` deprecations in excluded `backend/api/main.py:13`) | 146.8 s |

* **Node-ID parity:** the `--collect-only` ID lists of the scratch tree and of the baseline restricted to the
  in-scope paths are identical (446 = 446, empty `diff`). Per directory: `tests/db` 12, `tests/grid` 169,
  `tests/pipeline` 57 (81 minus 19 `test_sync_leagues` minus 5 `test_sync_trade_history`), `tests/projection` 44,
  `tests/scoring` 27, `tests/validation` 137. Excluded: 191 tests (adapters 13, api 64, services 13, trades 25,
  viz_agent 49, sync_* 24, fantasy_scoring 3).
* **Self-containment proof**, in three layers. (1) All 46 backend modules import in a fresh interpreter with zero
  app-stack modules loaded. (2) The clean venv physically lacks the app stack. It was built with `pip install -r requirements.txt`, and its `pip freeze` is `requirements.lock`. matplotlib (from `requirements-demo.txt`) was installed into it before the final run for the demo, but the guard blocks it and no test attempted it. (3) `scratch-python/plugins/isolation_guard.py` raised on any import of
  `backend.{api,services,adapters,trades,viz_agent,fantasy_scoring,pipeline.sync_*}`, any of
  fastapi/uvicorn/httpx/pydantic/starlette/anthropic/posthog/websockets/matplotlib/requests, and any
  non-loopback `connect`/`getaddrinfo`. It recorded **0 hits** (`scratch-python/guard_hits_venv.txt` is empty).
* **Slowest tests** (single-thread): `test_attribution.py::test_layer1_all_qbs_{skips_sparse_qbs,returns_dict}`
  at about 16 s each (each re-simulates the full synth league and refits the GBM). Next are
  `test_verdict.py::test_run_verdict_dedupes_players_before_volume_merge` at 9.8 s,
  `test_determinism.py::test_pipeline_is_deterministic_in_process` at 9.1 s, and the setups of
  `test_verdict`/`test_calibration_synth`/`test_tier0_recovery`/`test_golden_master` at 4-7 s each.
* **Threading hazard (important for CI):** my first attempt ran the baseline and scratch suites concurrently with
  default threading (sklearn OpenMP plus OpenBLAS each spawn per-core threads, about 12 threads per process,
  load average about 8 on 4 cores). The OpenMP spin-wait oversubscription made `test_attribution` take more than 15
  min for 4 tests, so I killed both runs (logs kept as `*.multithread_killed.log`). Pinning
  `OMP_NUM_THREADS=OPENBLAS_NUM_THREADS=MKL_NUM_THREADS=1` brought the same 10 attribution tests down to 45 s.
  Alone, default threading takes 149 s versus 144 s single-threaded, so threads buy nothing. **Recommend that
  the CI job set those three variables to 1.** That matches the single-thread freeze the golden master and
  determinism gates already enforce internally with `threadpool_limits(1)`.
* **Failures:** none, in any final run.

---

## 4. Network / offline behaviour

* No in-scope test needs the network. Every nflverse fetch is mocked. `tests/grid/test_nflverse_loader.py`
  patches `_fetch_pbp_year`/`_fetch_roster_year`/`_fetch_participation_year`.
  `tests/grid/test_nflverse_adapter.py:176-178` patches `load_raw_pbp`/`load_participation`/`load_rosters`.
  `tests/pipeline/test_data_pipeline.py:278-306` patches `load_rosters`/`load_pbp`.
  `tests/pipeline/test_weekly_update.py:113-114` and `tests/validation/test_verdict.py:138` monkeypatch the
  loaders. `tests/pipeline/test_ingest_grid.py` injects a `loader=`. The scratch run had `socket.connect`
  and `getaddrinfo` blocked for every non-loopback address (`scratch-python/plugins/isolation_guard.py`),
  and the guard recorded **zero** network attempts.
* The real network entry points (CLI only, never in tests) are `nflverse_loader._fetch_*_year` →
  `pd.read_parquet("https://github.com/nflverse/nflverse-data/releases/download/…")`. Callers:
  `python -m backend.pipeline.data_pipeline`, `python -m backend.pipeline.ingest_grid`, and
  `weekly_update.run()` when called without `plays_df`/`players_df`. Offline, `weekly_update.run()` catches
  the error (L215-220), writes a warning to `data/health.json` and returns `rapm_solved=False`.
  `data_pipeline.run()` logs and continues per year. `ingest_grid` raises.
* Cache semantics: `ParquetCache` TTL is 168 h (`nflverse_loader._load_cached_years`). Once
  `data/cache/nflverse_*` exists and is fresh, everything is offline. `verdict.main()` reads only
  `data/snapshots/` plus the cache plus SQLite.
* Test-hygiene notes (pre-existing, not blockers): `test_nflverse_loader.py:47,62` write to the absolute
  path `/tmp/test_cache` instead of `tmp_path`. Importing any pipeline module creates `./data/logs/pipeline.log`
  relative to the **cwd** (`_logging.py:8,14`), so a test run from `reference/python` creates
  `reference/python/data/` (gitignored, see section 5). Measured in the scratch run: the suite leaves exactly `./data/health.json` (from `write_health()` at its default path, e.g. via `compute_valuations.run`) and `./data/logs/pipeline.log` in the cwd, plus the `/tmp/test_cache/nflverse_{pbp,roster}_2023.parquet` files mentioned above. No `data/db/`, `data/cache/` or `data/snapshots/` was left behind: the SQLite, accumulator and Kalman tests redirect `DB_PATH`/`_ACCUM_PATH`/`_KALMAN_PATH` (or pass connections and paths) into `tmp_path`.

---

## 5. Generated-data conventions to carry over

Upstream `.gitignore` (cautious-nevermore) ignores `data/cache/*.parquet`, `data/cache/*.npz`,
`data/db/*.sqlite`, `data/logs/`, `data/health.json`, `data/snapshots/`, `__pycache__/`, `*.pyc`,
`.pytest_cache/`, `.env`, `.venv/`, `*.egg-info/`. Upstream tracks nothing under `data/`
(`git ls-files data` is empty), so ignoring the whole of `/data/` is stricter and equivalent. It also
covers two paths upstream missed: `data/reports/` (written by `validation/report.py:18,129` /
`verdict.main`) and the `data/cache/*.joblib` V(s) cache.

All runtime paths are **cwd-relative**:

| Path | Writer |
|---|---|
| `data/cache/*.parquet` | `ParquetCache` |
| `data/cache/rapm_accumulators*.npz` | `layers._ACCUM_PATH` |
| `data/cache/kalman_state.npz` | `statespace._KALMAN_PATH` |
| `data/db/fantasy.sqlite` | `connection.DB_PATH`, env `DB_PATH` |
| `data/logs/pipeline.log` | `_logging` |
| `data/health.json` | `health_check` |
| `data/snapshots/` | `ingest_grid` |
| `data/reports/` | `report` |

So the effective location depends on where the command runs. From `reference/python/`, the new
`reference/python/.gitignore` (`/data/`) covers them. From the GRID-Engine root, GRID-Engine's existing
`/data/` rule covers them. Both were verified.

`reference/python/.gitignore` (scratch copy, recommended verbatim):
```
/data/
*.sqlite
*.sqlite-wal
*.sqlite-shm
__pycache__/
*.pyc
.pytest_cache/
.venv/
*.egg-info/
.env
!tests/grid/golden/snapshot.npz
```
The golden `tests/grid/golden/snapshot.npz` (38,764 B, zip/npz, store method) **must be tracked**. No rule
above matches it, and the `!` line protects it if someone later adds a global `*.npz`. GRID-Engine
`.gitattributes` has no `* text=auto`, so git will not touch its bytes. Optionally add
`reference/python/**/*.npz binary` for explicitness. All 105 files are LF with no CRLF, so the `*.sh eol=lf`
policy is unaffected.

---

## 6. Final manifest

### 6.1 Files to import (105). Source → destination, mode, reason

Mode was verified by sha256 against `git show 59bce1d:<path>`. Regenerate the files with
`git -C /home/user/cautious-nevermore archive 59bce1d <paths> | tar -x -C reference/python`, then apply P1
(and optionally P2).

| # | Source (cautious-nevermore @ 59bce1d) | Destination | Mode | Reason |
|---|---|---|---|---|
| 1 | `backend/__init__.py` | `reference/python/backend/__init__.py` | verbatim | package root (empty) |
| 2 | `backend/db/__init__.py` | `reference/python/backend/db/__init__.py` | verbatim | package marker |
| 3 | `backend/db/connection.py` | `reference/python/backend/db/connection.py` | verbatim | get_db/init_db + matchup_grades migration guards |
| 4 | `backend/db/data/coaching_changes_2025.json` | `reference/python/backend/db/data/coaching_changes_2025.json` | verbatim | default seed data for seed_coaching |
| 5 | `backend/db/schema.sql` | `reference/python/backend/db/schema.sql` | verbatim | SQLite schema (verbatim; app tables inert, asserted by test_schema) |
| 6 | `backend/db/seed_coaching.py` | `reference/python/backend/db/seed_coaching.py` | verbatim | coaching_changes seed -> weekly_update scheme_fit resets |
| 7 | `backend/grid/__init__.py` | `reference/python/backend/grid/__init__.py` | verbatim | GRID public API re-exports |
| 8 | `backend/grid/cache.py` | `reference/python/backend/grid/cache.py` | verbatim | ParquetCache (TTL) |
| 9 | `backend/grid/data_adapters.py` | `reference/python/backend/grid/data_adapters.py` | verbatim | contract swap point (synthetic + stubs) |
| 10 | `backend/grid/layers.py` | `reference/python/backend/grid/layers.py` | verbatim | RAPM L2 / market L3 / L1 credit / accumulators |
| 11 | `backend/grid/nflverse_adapter.py` | `reference/python/backend/grid/nflverse_adapter.py` | verbatim | real pbp -> GRID plays contract |
| 12 | `backend/grid/nflverse_loader.py` | `reference/python/backend/grid/nflverse_loader.py` | verbatim | nflverse pbp/roster/participation fetch + normalize |
| 13 | `backend/grid/priors.py` | `reference/python/backend/grid/priors.py` | verbatim | cross-league priors + washout |
| 14 | `backend/grid/situations.py` | `reference/python/backend/grid/situations.py` | verbatim | situation masks |
| 15 | `backend/grid/statespace.py` | `reference/python/backend/grid/statespace.py` | verbatim | Kalman talent/form/scheme_fit, RTS, changepoints |
| 16 | `backend/grid/synth.py` | `reference/python/backend/grid/synth.py` | verbatim | planted-ground-truth generator (oracle input) |
| 17 | `backend/grid/value.py` | `reference/python/backend/grid/value.py` | verbatim | V(s) expected-points model, dV |
| 18 | `backend/pipeline/__init__.py` | `reference/python/backend/pipeline/__init__.py` | verbatim | package marker |
| 19 | `backend/pipeline/_logging.py` | `reference/python/backend/pipeline/_logging.py` | verbatim | logger used by 4 engine modules (stdlib only) |
| 20 | `backend/pipeline/compute_valuations.py` | `reference/python/backend/pipeline/compute_valuations.py` | verbatim | valuations/VOR table (called by weekly_update) |
| 21 | `backend/pipeline/data_pipeline.py` | `reference/python/backend/pipeline/data_pipeline.py` | verbatim | nflverse -> players/player_stats (read by compute_valuations + verdict) |
| 22 | `backend/pipeline/health_check.py` | `reference/python/backend/pipeline/health_check.py` | verbatim | write_health used by 3 engine modules (stdlib only) |
| 23 | `backend/pipeline/ingest_grid.py` | `reference/python/backend/pipeline/ingest_grid.py` | verbatim | frozen multi-season plays-contract snapshots (verdict input) |
| 24 | `backend/pipeline/weekly_update.py` | `reference/python/backend/pipeline/weekly_update.py` | verbatim | incremental RAPM accumulators + Kalman advance + grades |
| 25 | `backend/projection/__init__.py` | `reference/python/backend/projection/__init__.py` | verbatim | stat-line projection (volume x efficiency), SV->points, preseason |
| 26 | `backend/projection/features.py` | `reference/python/backend/projection/features.py` | verbatim | stat-line projection (volume x efficiency), SV->points, preseason |
| 27 | `backend/projection/model.py` | `reference/python/backend/projection/model.py` | verbatim | stat-line projection (volume x efficiency), SV->points, preseason |
| 28 | `backend/projection/preseason.py` | `reference/python/backend/projection/preseason.py` | verbatim | stat-line projection (volume x efficiency), SV->points, preseason |
| 29 | `backend/projection/sv_to_points.py` | `reference/python/backend/projection/sv_to_points.py` | verbatim | stat-line projection (volume x efficiency), SV->points, preseason |
| 30 | `backend/projection/volume.py` | `reference/python/backend/projection/volume.py` | verbatim | stat-line projection (volume x efficiency), SV->points, preseason |
| 31 | `backend/scoring/__init__.py` | `reference/python/backend/scoring/__init__.py` | verbatim | stat line -> fantasy points (engine scoring) |
| 32 | `backend/scoring/columns.py` | `reference/python/backend/scoring/columns.py` | verbatim | stat line -> fantasy points (engine scoring) |
| 33 | `backend/scoring/engine.py` | `reference/python/backend/scoring/engine.py` | verbatim | stat line -> fantasy points (engine scoring) |
| 34 | `backend/scoring/format_registry.py` | `reference/python/backend/scoring/format_registry.py` | verbatim | ensure_preset_formats/DEFAULT_ROSTER_SLOTS used by compute_valuations (resolve_or_create_format is app-only dead code here) |
| 35 | `backend/scoring/formats.py` | `reference/python/backend/scoring/formats.py` | verbatim | stat line -> fantasy points (engine scoring) |
| 36 | `backend/scoring/vor.py` | `reference/python/backend/scoring/vor.py` | verbatim | VOR + tiers: used by compute_valuations and lineup_sim |
| 37 | `backend/validation/__init__.py` | `reference/python/backend/validation/__init__.py` | verbatim | as-of leakage guards, walk-forward backtest, tiers, verdict |
| 38 | `backend/validation/asof.py` | `reference/python/backend/validation/asof.py` | verbatim | as-of leakage guards, walk-forward backtest, tiers, verdict |
| 39 | `backend/validation/backtest.py` | `reference/python/backend/validation/backtest.py` | verbatim | as-of leakage guards, walk-forward backtest, tiers, verdict |
| 40 | `backend/validation/baselines.py` | `reference/python/backend/validation/baselines.py` | verbatim | as-of leakage guards, walk-forward backtest, tiers, verdict |
| 41 | `backend/validation/lineup_sim.py` | `reference/python/backend/validation/lineup_sim.py` | **patched** | H2 lineup sim; PATCH P1 cuts backend.services edge |
| 42 | `backend/validation/metrics.py` | `reference/python/backend/validation/metrics.py` | verbatim | as-of leakage guards, walk-forward backtest, tiers, verdict |
| 43 | `backend/validation/provisional_thresholds.json` | `reference/python/backend/validation/provisional_thresholds.json` | verbatim | gate registry read by thresholds.py (REGISTRY_PATH) |
| 44 | `backend/validation/report.py` | `reference/python/backend/validation/report.py` | verbatim | as-of leakage guards, walk-forward backtest, tiers, verdict |
| 45 | `backend/validation/sniff.py` | `reference/python/backend/validation/sniff.py` | verbatim | as-of leakage guards, walk-forward backtest, tiers, verdict |
| 46 | `backend/validation/thresholds.py` | `reference/python/backend/validation/thresholds.py` | verbatim | as-of leakage guards, walk-forward backtest, tiers, verdict |
| 47 | `backend/validation/tier1.py` | `reference/python/backend/validation/tier1.py` | verbatim | as-of leakage guards, walk-forward backtest, tiers, verdict |
| 48 | `backend/validation/tier2.py` | `reference/python/backend/validation/tier2.py` | verbatim | as-of leakage guards, walk-forward backtest, tiers, verdict |
| 49 | `backend/validation/verdict.py` | `reference/python/backend/validation/verdict.py` | verbatim | as-of leakage guards, walk-forward backtest, tiers, verdict |
| 50 | `run_demo.py` | `reference/python/run_demo.py` | **patched** | end-to-end recovery demo; PATCH P2 (optional) env-overridable OUT |
| 51 | `tests/__init__.py` | `reference/python/tests/__init__.py` | verbatim | makes `tests.grid.golden_master` importable |
| 52 | `tests/db/__init__.py` | `reference/python/tests/db/__init__.py` | verbatim | schema + migration-guard tests |
| 53 | `tests/db/test_schema.py` | `reference/python/tests/db/test_schema.py` | verbatim | schema + migration-guard tests |
| 54 | `tests/grid/__init__.py` | `reference/python/tests/grid/__init__.py` | verbatim | engine tests |
| 55 | `tests/grid/golden/snapshot.npz` | `reference/python/tests/grid/golden/snapshot.npz` | verbatim | Tier 0.5 golden master (binary, 38,764 B) |
| 56 | `tests/grid/golden_master.py` | `reference/python/tests/grid/golden_master.py` | verbatim | golden snapshot generator/loader |
| 57 | `tests/grid/test_attribution.py` | `reference/python/tests/grid/test_attribution.py` | verbatim | engine tests |
| 58 | `tests/grid/test_cache.py` | `reference/python/tests/grid/test_cache.py` | verbatim | engine tests |
| 59 | `tests/grid/test_calibration_synth.py` | `reference/python/tests/grid/test_calibration_synth.py` | verbatim | engine tests |
| 60 | `tests/grid/test_changepoint.py` | `reference/python/tests/grid/test_changepoint.py` | verbatim | engine tests |
| 61 | `tests/grid/test_coaching_changes.py` | `reference/python/tests/grid/test_coaching_changes.py` | verbatim | engine tests |
| 62 | `tests/grid/test_design_interactions.py` | `reference/python/tests/grid/test_design_interactions.py` | verbatim | engine tests |
| 63 | `tests/grid/test_determinism.py` | `reference/python/tests/grid/test_determinism.py` | verbatim | engine tests |
| 64 | `tests/grid/test_golden_master.py` | `reference/python/tests/grid/test_golden_master.py` | verbatim | engine tests |
| 65 | `tests/grid/test_grid_import.py` | `reference/python/tests/grid/test_grid_import.py` | verbatim | engine tests |
| 66 | `tests/grid/test_incremental.py` | `reference/python/tests/grid/test_incremental.py` | verbatim | engine tests |
| 67 | `tests/grid/test_kalman_numerical.py` | `reference/python/tests/grid/test_kalman_numerical.py` | verbatim | engine tests |
| 68 | `tests/grid/test_layers.py` | `reference/python/tests/grid/test_layers.py` | verbatim | engine tests |
| 69 | `tests/grid/test_layers_situations.py` | `reference/python/tests/grid/test_layers_situations.py` | verbatim | engine tests |
| 70 | `tests/grid/test_nflverse_adapter.py` | `reference/python/tests/grid/test_nflverse_adapter.py` | verbatim | engine tests |
| 71 | `tests/grid/test_nflverse_loader.py` | `reference/python/tests/grid/test_nflverse_loader.py` | verbatim | engine tests |
| 72 | `tests/grid/test_performance.py` | `reference/python/tests/grid/test_performance.py` | verbatim | engine tests |
| 73 | `tests/grid/test_phase0_prereqs.py` | `reference/python/tests/grid/test_phase0_prereqs.py` | verbatim | engine tests |
| 74 | `tests/grid/test_priors.py` | `reference/python/tests/grid/test_priors.py` | verbatim | engine tests |
| 75 | `tests/grid/test_rapm_robustness.py` | `reference/python/tests/grid/test_rapm_robustness.py` | verbatim | engine tests |
| 76 | `tests/grid/test_situations.py` | `reference/python/tests/grid/test_situations.py` | verbatim | engine tests |
| 77 | `tests/grid/test_tier0_recovery.py` | `reference/python/tests/grid/test_tier0_recovery.py` | verbatim | engine tests |
| 78 | `tests/pipeline/__init__.py` | `reference/python/tests/pipeline/__init__.py` | verbatim | engine pipeline tests |
| 79 | `tests/pipeline/test_compute_valuations.py` | `reference/python/tests/pipeline/test_compute_valuations.py` | verbatim | engine pipeline tests |
| 80 | `tests/pipeline/test_data_pipeline.py` | `reference/python/tests/pipeline/test_data_pipeline.py` | verbatim | engine pipeline tests |
| 81 | `tests/pipeline/test_ingest_grid.py` | `reference/python/tests/pipeline/test_ingest_grid.py` | verbatim | engine pipeline tests |
| 82 | `tests/pipeline/test_kalman_trajectory_write.py` | `reference/python/tests/pipeline/test_kalman_trajectory_write.py` | verbatim | engine pipeline tests |
| 83 | `tests/pipeline/test_pipeline.py` | `reference/python/tests/pipeline/test_pipeline.py` | verbatim | engine pipeline tests |
| 84 | `tests/pipeline/test_weekly_situations.py` | `reference/python/tests/pipeline/test_weekly_situations.py` | verbatim | engine pipeline tests |
| 85 | `tests/pipeline/test_weekly_update.py` | `reference/python/tests/pipeline/test_weekly_update.py` | verbatim | engine pipeline tests |
| 86 | `tests/projection/test_features.py` | `reference/python/tests/projection/test_features.py` | verbatim | projection tests (no __init__.py upstream; kept as-is) |
| 87 | `tests/projection/test_model.py` | `reference/python/tests/projection/test_model.py` | verbatim | projection tests (no __init__.py upstream; kept as-is) |
| 88 | `tests/projection/test_preseason.py` | `reference/python/tests/projection/test_preseason.py` | verbatim | projection tests (no __init__.py upstream; kept as-is) |
| 89 | `tests/projection/test_sv_to_points.py` | `reference/python/tests/projection/test_sv_to_points.py` | verbatim | projection tests (no __init__.py upstream; kept as-is) |
| 90 | `tests/projection/test_volume.py` | `reference/python/tests/projection/test_volume.py` | verbatim | projection tests (no __init__.py upstream; kept as-is) |
| 91 | `tests/scoring/__init__.py` | `reference/python/tests/scoring/__init__.py` | verbatim | scoring tests |
| 92 | `tests/scoring/test_engine.py` | `reference/python/tests/scoring/test_engine.py` | verbatim | scoring tests |
| 93 | `tests/scoring/test_vor.py` | `reference/python/tests/scoring/test_vor.py` | verbatim | scoring tests |
| 94 | `tests/validation/test_asof.py` | `reference/python/tests/validation/test_asof.py` | verbatim | validation tests (no __init__.py upstream; kept as-is) |
| 95 | `tests/validation/test_backtest.py` | `reference/python/tests/validation/test_backtest.py` | verbatim | validation tests (no __init__.py upstream; kept as-is) |
| 96 | `tests/validation/test_baselines.py` | `reference/python/tests/validation/test_baselines.py` | verbatim | validation tests (no __init__.py upstream; kept as-is) |
| 97 | `tests/validation/test_leakage_guards.py` | `reference/python/tests/validation/test_leakage_guards.py` | verbatim | validation tests (no __init__.py upstream; kept as-is) |
| 98 | `tests/validation/test_lineup_sim.py` | `reference/python/tests/validation/test_lineup_sim.py` | verbatim | validation tests (no __init__.py upstream; kept as-is) |
| 99 | `tests/validation/test_metrics.py` | `reference/python/tests/validation/test_metrics.py` | verbatim | validation tests (no __init__.py upstream; kept as-is) |
| 100 | `tests/validation/test_report.py` | `reference/python/tests/validation/test_report.py` | verbatim | validation tests (no __init__.py upstream; kept as-is) |
| 101 | `tests/validation/test_sniff.py` | `reference/python/tests/validation/test_sniff.py` | verbatim | validation tests (no __init__.py upstream; kept as-is) |
| 102 | `tests/validation/test_thresholds.py` | `reference/python/tests/validation/test_thresholds.py` | verbatim | validation tests (no __init__.py upstream; kept as-is) |
| 103 | `tests/validation/test_tier1.py` | `reference/python/tests/validation/test_tier1.py` | verbatim | validation tests (no __init__.py upstream; kept as-is) |
| 104 | `tests/validation/test_tier2.py` | `reference/python/tests/validation/test_tier2.py` | verbatim | validation tests (no __init__.py upstream; kept as-is) |
| 105 | `tests/validation/test_verdict.py` | `reference/python/tests/validation/test_verdict.py` | verbatim | validation tests (no __init__.py upstream; kept as-is) |

### 6.2 New files (not from upstream)

| Destination | Content / purpose |
|---|---|
| `reference/python/requirements.txt` | `numpy>=1.24`, `pandas>=2.0`, `scipy>=1.11`, `scikit-learn>=1.3`, `pyarrow>=14.0`, `joblib>=1.3`, `python-dotenv>=1.0` (needed by `db/connection.py:4`), `threadpoolctl>=3.1` (imported directly by `tests/grid/golden_master.py:20`, `test_calibration_synth.py:24`, `tests/validation/test_{backtest,leakage_guards,verdict}.py`), `pytest>=7.0`. **Removed** from upstream: fastapi, uvicorn, httpx, pydantic, websockets, anthropic, posthog (none is imported by the closure). |
| `reference/python/requirements-demo.txt` | `-r requirements.txt` + `matplotlib>=3.7` (only `run_demo.py:16-18`). |
| `reference/python/requirements.lock` | Exact `pip freeze` of the verified clean venv (Python 3.11.15, x86_64): numpy 2.4.6, pandas 3.0.6, scipy 1.17.1, scikit-learn 1.9.1, pyarrow 25.0.1, joblib 1.6.0, threadpoolctl 3.7.0, python-dotenv 1.2.3, pytest 9.1.1 (+ transitive). Use it as a constraints file (`-c`). The golden master asserts at `rtol 1e-5 / atol 1e-6` (`test_golden_master.py` layer C) on sklearn GBM + RAPM output, so an unpinned sklearn/numpy bump can drift it. An oracle should pin. |
| `reference/python/pytest.ini` | `[pytest]` / `testpaths = tests` / `pythonpath = .`. This pins rootdir to `reference/python` (verified from `reference/python/` and from the repo root via `python -m pytest reference/python/tests/...`), so no root conftest is needed. Plain `pytest` works as well as `python -m pytest`. |
| `reference/python/.gitignore` | As in section 5. |
| *(recommended, not in scratch)* `reference/python/README.md` | Provenance (`cautious-nevermore@59bce1d`, patch list), how to run (`pip install -r requirements.txt -c requirements.lock && python -m pytest`), the env var `DB_PATH`, the cwd-relative `data/` layout, the golden regeneration command (`python -m tests.grid.golden_master` from `reference/python/`), and a pointer to the parity-target tests (section 8). |

### 6.3 Files deliberately excluded (131 of 236 tracked)

| Excluded | Count | Reason |
|---|---|---|
| `backend/api/**` (`__init__`, `deps`, `main`, `middleware`, `posthog_client`, `routes/{__init__,draft,leagues,players,rankings,season,stats,viz_agent}`) | 13 | FastAPI app (fastapi/posthog). |
| `backend/viz_agent/**` (`__init__`, `agent`, `cache`, `schema`) | 4 | LLM chart agent (anthropic). |
| `backend/trades/**` (`__init__`, `trade_analyzer`, `trade_finder`, `trade_model`) | 4 | Trade tooling: an app decision layer over engine outputs. |
| `backend/services/**` (`__init__`, `draft_grade`, `live_draft`, `mock_draft`) | 4 | Draft tooling and WebSocket manager. The only engine need (`snake_order`) is inlined by P1. |
| `backend/adapters/**` (`__init__`, `espn`, `league`, `sleeper`) | 4 | ESPN/Sleeper league adapters (cookie auth). |
| `backend/pipeline/sync_leagues.py`, `sync_trade_history.py` | 2 | League/trade sync (edges E3/E4). |
| `backend/fantasy_scoring.py` | 1 | Orphaned, superseded by `backend/projection` (section 1.3). |
| `tests/adapters/**` (5), `tests/api/**` (6), `tests/services/**` (3), `tests/trades/**` (4), `tests/viz_agent/**` (5) | 23 | Tests of excluded modules (13+64+13+25+49 = 164 tests). |
| `tests/pipeline/test_sync_leagues.py` (19 tests incl. `TestResolveOrCreateFormat`), `tests/pipeline/test_sync_trade_history.py` (5) | 2 | Tests of excluded modules. |
| `tests/test_fantasy_scoring.py` | 1 | Tests the orphan (3 tests). |
| `frontend/**` | 51 | React UI. |
| `scripts/run_pipeline.bat`, `scripts/setup_scheduler.ps1` | 2 | Windows Task Scheduler wrapper. It also runs `sync_leagues` and is CRLF (upstream `.gitattributes`). The engine CLIs are `python -m backend.pipeline.{data_pipeline,ingest_grid,weekly_update,compute_valuations}` and `python -m backend.validation.verdict`. |
| `requirements.txt`, `.gitignore`, `.gitattributes` | 3 | Replaced by engine-only versions. `.gitattributes` only set CRLF for `*.bat`/`*.ps1`, which are not imported. |
| `.env.example` | 1 | Almost entirely app variables (ESPN cookies, league IDs, ANTHROPIC/POSTHOG, VIZ_AGENT_*). The only engine variable is `DB_PATH` (default `data/db/fantasy.sqlite`). Document it in the README instead. |
| `grid_recovery.png`, `grid_focus_qb.png` | 2 | `run_demo.py` outputs (regenerable). If the spec docs want them as figures, put them under `docs/`, not the oracle. |
| `CLAUDE.md`, `README.md` | 2 | App-wide guidance. Rewrite engine-scoped. The upstream CLAUDE.md sections "synthetic-data contract" and "GRID engine architecture" are good source text for the README and specs. |
| `docs/01..10-*.md`, `sdd/task-8-report.md`, `.superpowers/sdd/task-14-report.md` | 12 | Spec/doc material, owned by the spec-consolidation workstream, not by `reference/python`. |

### 6.4 Patches (unified diffs against cautious-nevermore@59bce1d)

**P1 (required): `backend/validation/lineup_sim.py`.** Cuts edge E1. The inlined function body is
AST-identical to `backend/services/mock_draft.py:38-46`.
```diff
--- a/backend/validation/lineup_sim.py
+++ b/backend/validation/lineup_sim.py
@@ -7,7 +7,7 @@
 
 * **Synthetic rosters** — a deterministic VOR-greedy snake draft
   (:func:`draft_rosters`, reusing :func:`~backend.scoring.vor.calculate_vor` and
-  :func:`~backend.services.mock_draft.snake_order`) builds realistic
+  :func:`snake_order`) builds realistic
   starters-plus-bench rosters from a *shared* projection, so roster construction
   never favors either compared method.
 * **One slot-filling rule** — :func:`set_lineup` is the single greedy
@@ -29,12 +29,27 @@
 from dataclasses import dataclass, field
 
 from backend.scoring.vor import calculate_vor
-from backend.services.mock_draft import snake_order
 from backend.validation.metrics import BootstrapCI, bootstrap_ci
 
 FLEX_ELIGIBLE = ("RB", "WR", "TE")
 
 
+def snake_order(num_teams: int, num_rounds: int) -> list[int]:
+    """Return list of team_slot (1-based) for every pick in a snake draft.
+
+    Verbatim copy of the app-side ``backend.services.mock_draft.snake_order``
+    (cautious-nevermore @ 59bce1d), inlined so the engine reference does not
+    import the app-only ``backend.services`` package.
+    """
+    order = []
+    for r in range(num_rounds):
+        teams = list(range(1, num_teams + 1))
+        if r % 2 == 1:
+            teams = list(reversed(teams))
+        order.extend(teams)
+    return order
+
+
 # --------------------------------------------------------------------------- #
 # Roster construction                                                          #
 # --------------------------------------------------------------------------- #
```

**P2 (optional, recommended): `run_demo.py`.** Makes the output directory overridable. The default is
unchanged, so behaviour is identical when the variable is unset.
```diff
--- a/run_demo.py
+++ b/run_demo.py
@@ -23,7 +23,7 @@
 )
 from backend.grid import SSParams, PRIOR_SD
 
-OUT = "/mnt/user-data/outputs"
+OUT = os.environ.get("GRID_DEMO_OUT", "/mnt/user-data/outputs")
 os.makedirs(OUT, exist_ok=True)
 POS_COLORS = {"QB": "#1f77b4", "RB": "#ff7f0e", "WR": "#2ca02c",
               "TE": "#9467bd", "DEF": "#d62728"}
```
If P2 is declined, import `run_demo.py` verbatim and accept that it can only run where
`/mnt/user-data/outputs` is creatable. Either way it needs `requirements-demo.txt`.

---

## 7. GRID-Engine integration checks (run against the scratch tree)

| Guard | Result | Action for the import PR |
|---|---|---|
| `typos` (pinned 1.49.0 via the PyPI `typos` wheel, run with GRID-Engine `_typos.toml`) | **FAILS: 152 findings, all false positives.** 137 are `vor`/`VOR` (Value Over Replacement, including the filenames `vor.py`/`test_vor.py`). The rest: `ot_*` (6, `test_weekly_situations.py:98-104`), `fo` (4, `verdict.py:166,169`), `yhat` (2, `test_verdict.py:336,338`), `pn` (2, `test_vor.py:238-239`), `mis` (1, `asof.py:220` "mis-slice"). | Add `"reference/python/",` to `_typos.toml [files] extend-exclude`. Justification for alpha-spec 12.7: a verbatim upstream oracle where byte-identity outranks spelling. Separately, `VOR = "VOR"` / `vor = "vor"` in `[default.extend-words]` is justified for the Rust port and specs (it is a domain term). **Verified:** with exactly `"reference/python/",` added to `extend-exclude`, `typos` over a GRID-Engine-shaped tree exits 0. Today's GRID-Engine tree (3823478) has 0 findings, so all 152 come from the import. |
| `scripts/check-secrets.sh` (full-tree mode) | **OK**: "no secret patterns found … 100 whole file(s) read". The other 9 listed files were deliberate skips: the binary `snapshot.npz` and the 8 empty `__init__.py`. | None. `.env.example` (which has `ANTHROPIC_API_KEY=`) is excluded anyway. |
| `scripts/check-traceability.sh` | Would FAIL unless covered. It is per-path, and the 110 new paths must be named, or sit under a directory named as `reference/python/` (ending in a slash), in a WP or ADR cited in the commit range or PR body. | The import WP/ADR must literally contain `reference/python/` in its Scope. `requirements.lock` is classed trivial (`*.lock`). Nested `.gitignore` files are *not* trivial (the pattern only matches the root file), but the directory prefix covers them. |
| `check-migrations.sh` | Unaffected. It only diffs `migrations/`, and `backend/db/schema.sql` is not under it. | None. |
| `.gitattributes` | Unaffected. No CRLF in the import set, and no `text=auto`. | Optional `reference/python/**/*.npz binary`. |
| **dotenv walk-up** | `backend/db/connection.py:4-6` calls `load_dotenv()` at import time, and `find_dotenv()` walks *up* from `backend/db/`. Inside GRID-Engine it finds the **root `.env`**. Verified: `DATABASE_URL=sqlite:target/grid-dev.db` and `SQLX_OFFLINE=true` were loaded into the Python process. That is harmless today because the root `.env` has no `DB_PATH`. | Document it in the README: never put `DB_PATH` in the root `.env`. No patch is needed. |
| CI (`.github/workflows/alpha-ci.yml`) | Not exercised here. | Add a separate Python job (`setup-python 3.11`, `pip install -r reference/python/requirements.txt -c reference/python/requirements.lock`, `cd reference/python && python -m pytest`, with env `OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 MKL_NUM_THREADS=1`). It takes about 2.5 min on 4 vCPU (section 3). If the owner instead wants it inside `just verify`, that changes the frozen verify contract (ADR-001 D5, owner approval required), and `scripts/check-verify-parity.sh` then requires the same step in the `justfile`, `scripts/verify.sh` *and* `scripts/verify.ps1`. A separate CI job avoids touching D5. |

---

## 8. Parity targets inside the import (for the Rust ports)

Gates are calibrated *below* observed values (calibrate-then-gate). Observed values are noted in-line in
the tests.

| Gate file | Count | What it pins |
|---|---|---|
| `tests/grid/test_tier0_recovery.py` | 8 tests | Pooled attribution ≥0.77 (obs 0.8025). Per-position floors QB 0.83, RB 0.70, WR 0.76, TE 0.73, DEF 0.73 (obs .869/.743/.797/.771/.765). Team strength ≥0.60 (obs .6643). Kalman total_smooth corr ≥0.92 (obs .958) and tau_smooth ≥0.60 (obs .6745). Injury-return variance > healthy. NIS ≤10 (obs 4.58). Equivalency slope in [0.9, 1.8] (obs 1.3159). OOS R² ≥0.05 (obs .149). Rookie prior corr ≥0.50 (obs .5829). |
| `tests/grid/test_golden_master.py` + `golden_master.py` + `golden/snapshot.npz` | 11 tests | Tier 0.5. Shapes (288 players, 12 teams, 14 weeks), semantic invariants, and numeric `assert_allclose rtol 1e-5 atol 1e-6`. Single-thread (`threadpool_limits(1)`), with `CANONICAL_SYNTH` (n_teams=12, weeks=14, seed=7, yards_noise_sd=3.2, drives_per_team_per_game=12, …), `MARKET_SEED=1`, `N_ITER=3`, `FOCUS_INTERVENTION=9`. |
| `tests/grid/test_determinism.py` | 2 | In-process determinism. |
| `tests/grid/test_calibration_synth.py` | 6 | Synth calibration (validation plan §6-7). |
| `tests/validation/test_leakage_guards.py` | 7 | Three-axis leakage guards (§8). |
| `tests/validation/test_tier1.py` / `test_tier2.py` / `test_lineup_sim.py` / `test_verdict.py` | 17 / 7 / 13 / 23 | Phase-2c skill + calibration, matchup-grade check, H2 lineup margin, end-to-end synth verdict. |

### 8.1 `run_demo.py` recovery correlations (scratch copy, P2 applied, `GRID_DEMO_OUT` set to scratch; 9.5 s)

```
data: 16,825 plays | 288 players | 12 teams | 14 weeks
[2] ATTRIBUTION  overall 0.803 | QB 0.869 (n=24) RB 0.743 (n=36) WR 0.797 (n=60) TE 0.771 (n=24) DEF 0.765 (n=144)
    team strength (market-reconciled): 0.664
[3] STATE-SPACE focus QB: raw weekly 0.972 | current-ability (tau+form) 0.955 | talent (tau) 0.675
    current-ability variance healthy(wk3)=0.0023 return(wk10)=0.0061
[4] PRIOR: planted league factor 0.620 | recovered factor check 0.678 | feeder->NFL OOS R^2 0.149 | rookie prior corr 0.583 (n=105)
    washout games_to_<50%: QB 1, RB 4, WR 4, TE 6, DEF 4  (prior_sd QB .16 RB .07 WR .08 TE .06 DEF .07)
```
The unpatched upstream `run_demo.py` (run from the pristine baseline copy with only the OUT literal swapped in
memory) produced **byte-identical printed output and an identical `grid_recovery.png`**. These values match the
"observed" comments in `test_tier0_recovery.py` (0.8025 / .869 / .743 / .797 / .771 / .765 / .6643 / .6745 / .149 / .5829).
The upstream environment cannot run the demo as-is: matplotlib is neither installed nor declared. Hence
`requirements-demo.txt`.

---

## 9. Reproduction commands

```bash
# assemble
R=GRID-Engine/reference/python; mkdir -p $R
git -C cautious-nevermore archive 59bce1d $(cat manifest_files.txt) | tar -x -C $R   # list = section 6.1
(cd $R && patch -p1 < lineup_sim.patch && patch -p1 < run_demo.patch)               # P1 (+P2)
cp {requirements.txt,requirements-demo.txt,requirements.lock,pytest.ini,.gitignore} $R/
# verify
python3.11 -m venv .venv && .venv/bin/pip install -r $R/requirements.txt -c $R/requirements.lock
cd $R && ../../.venv/bin/python -m pytest
```
The file list is `scratch-python/manifest_files.txt` and the hash table is `scratch-python/identity.tsv`.
