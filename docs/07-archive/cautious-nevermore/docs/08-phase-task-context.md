# GRID Fantasy — Phase & Task Context for Agents

> Reference for AI coding agents working on this codebase. Read this before starting any task. Contains conventions, gotchas, key file locations, and the synthetic-data contract.

## Before You Start

1. Read `CLAUDE.md` — the codebase guide with commands, architecture summary, and conventions.
2. Check `docs/05-current-state.md` — what's built, what's broken, what's next.
3. Check `docs/06-issues-log.md` — known bugs and issues (avoid reintroducing fixed ones).
4. This file covers agent-specific context: conventions, traps, key paths.

## Commands

```bash
# Backend tests (run from repo root, as module so backend.* imports resolve)
python -m pytest                              # all tests
python -m pytest tests/grid/test_layers.py    # single file
python -m pytest -k "rapm and not robustness"  # by keyword

# API server
uvicorn backend.api.main:app --reload          # :8000

# Data pipeline
python -m backend.pipeline.data_pipeline --years 2022 2023 2024
python -m backend.pipeline.ingest_grid --years 2022 2023

# Demo
python run_demo.py                             # end-to-end GRID recovery on synth

# Frontend
cd frontend && npm run dev                     # :5173
cd frontend && npm run build                   # tsc -b && vite build
cd frontend && npm run lint                    # eslint
```

**IMPORTANT:** `pytest` is NOT in `requirements.txt`. Install it separately: `pip install pytest`.

**IMPORTANT:** Tests are run as a module from the repo root (`python -m pytest`) so that `backend.*` imports resolve. There is no installed package, no `pytest.ini`, and no `conftest.py` at the root.

## Key Conventions

- `from __future__ import annotations` at the top of every backend module
- Absolute `backend.*` imports throughout
- Tests mirror the package layout under `tests/` and build small synthetic DataFrames inline
- Config and secrets via `.env` (see `.env.example`): league IDs, ESPN cookies, `DB_PATH`, `ANTHROPIC_API_KEY`
- Never commit real cookies/keys
- Generated data (`data/cache/*.parquet`, `data/cache/*.npz`, `*.sqlite`, logs) is gitignored
- Comments in the engine are load-bearing — they state the statistical intent. Keep that intent intact when editing.
- Pipeline `run(..., _conn=None)` does not close a passed conn
- Idempotent `ON CONFLICT`/`INSERT OR IGNORE` upserts
- Pydantic v2 models
- No SQL injection — all queries use parameterized `?` placeholders

## The Synthetic-Data Contract (Most Important)

GRID is validated against **planted ground truth**, not just unit assertions. `backend/grid/synth.py` generates nflfastR-shaped play-by-play from known hidden player abilities and team strengths; the estimators are checked on whether they *recover* the planted truth.

`backend/grid/data_adapters.py` is **the swap point**: every downstream stage consumes one fixed contract (`plays`, `players`, `market`, `college`). `load_synthetic()` returns the contract with ground truth. Real loaders fill the same contract from nflverse data. **When changing any engine stage, preserve this contract.**

Plays contract columns: `game_id, play_id, drive_id, week, season, off_team, def_team, down, ydstogo, yardline_100, yards, points, terminal, terminal_value, n_down, n_ydstogo, n_yardline_100, drive_points, off_players, def_players`

## Key File Locations

### GRID Engine
| File | Purpose |
|------|---------|
| `backend/grid/layers.py` | RAPM attribution (the workhorse, ~631 lines) |
| `backend/grid/value.py` | Situational value V(s) |
| `backend/grid/statespace.py` | Kalman filter + RTS smoother |
| `backend/grid/priors.py` | Cross-league priors |
| `backend/grid/synth.py` | Synthetic data generator |
| `backend/grid/data_adapters.py` | The swap point (synth ↔ real) |
| `backend/grid/nflverse_adapter.py` | Real nflverse → GRID plays contract |
| `backend/grid/cache.py` | Parquet cache with TTL |
| `backend/grid/situations.py` | Situation masks (red_zone, passing_downs, etc.) |

### Projection
| File | Purpose |
|------|---------|
| `backend/projection/volume.py` | Volume/role model (empirical-Bayes shrinkage) |
| `backend/projection/model.py` | Fitted stat-line model (ridge regression) |
| `backend/projection/sv_to_points.py` | SV→fantasy-points map (weekly lever) |
| `backend/projection/features.py` | Talent feature accessor (RAPM + smoothed + prior) |
| `backend/projection/preseason.py` | Preseason assembly (volume × efficiency → season line) |

### Validation
| File | Purpose |
|------|---------|
| `backend/validation/asof.py` | AsOf accessor + CacheNamespace + TripwireFrame |
| `backend/validation/backtest.py` | Walk-forward driver with incremental accumulators |
| `backend/validation/baselines.py` | Persistence, season-mean, last-season, market |
| `backend/validation/metrics.py` | MAE/RMSE/CRPS/PIT/PICP/NIS/Spearman/NDCG |
| `backend/validation/tier1.py` | ROS + weekly forecast + calibration report |
| `backend/validation/verdict.py` | End-to-end verdict runner |
| `backend/validation/sniff.py` | Known-player rank-band check |

### Scoring + Pipeline
| File | Purpose |
|------|---------|
| `backend/scoring/engine.py` | `calculate_points(stat_line, config)` |
| `backend/scoring/formats.py` | Standard, Half-PPR, Full PPR presets |
| `backend/scoring/vor.py` | VOR + tiers |
| `backend/pipeline/data_pipeline.py` | nflverse → SQLite ingest |
| `backend/pipeline/compute_valuations.py` | `valuations` table producer |
| `backend/pipeline/weekly_update.py` | Incremental RAPM + Kalman |
| `backend/pipeline/ingest_grid.py` | Multi-season GRID plays snapshot |

### API + Frontend
| File | Purpose |
|------|---------|
| `backend/api/main.py` | FastAPI app |
| `backend/api/routes/` | All route modules |
| `frontend/src/hooks/useApi.ts` | All TanStack Query hooks |
| `frontend/src/types/index.ts` | All TypeScript interfaces |
| `frontend/src/charts/ChartRenderer.tsx` | Renders ChartSpec (scatter/bar/line/radar) |

## Known Traps

### Market reconciliation sign convention
The design matrix uses `+1` for offense, `-1` for defense. Team rating = `beta[t_off] - beta[t_def]`. The market pseudo-observation MUST use `[+1, -1]`, not `[+1, +1]`. This bug was found in both `layers.py:404-405` and `backtest.py:101-102` — if you touch either, verify the signs.

### NaN in dict.get()
`dict.get(key, default)` returns the default only when the key is ABSENT. If the key is present with value `NaN`, it returns `NaN`. This crashes `Ridge.predict` via `StandardScaler.transform`. When consuming `to_frame()` output (which intentionally fills NaN for missing priors), guard for NaN explicitly.

### max(0.0, NaN) returns 0.0
In Python, `max(0.0, float('nan'))` returns `0.0`. Never use `max()` with potentially-NaN values. Use `np.nanmax` or guard explicitly.

### `fantasy_scoring.py` is orphaned
`backend/fantasy_scoring.py` is the original sketch with hand-picked `GRID_SCALING` constants. It's replaced by `backend/projection/model.py`'s fitted ridge regression. It still has passing tests but is not wired into the product. Don't add new code to it.

### `var_total` missing covariance
`var_total = sigma[0,0] + sigma[1,1]` is WRONG for the sum of correlated random variables. The correct formula is `H @ sigma @ H^T` where `H=[1,1,1]` for the 3-component state. This bug appeared in 3 files. If you see `sigma[0,0] + sigma[1,1]`, fix it.

### Tests that only assert > 0
Several tests assert `var > 0` instead of asserting the expected value. These tests are too weak — they pass for any positive number, including wrong ones. When writing tests for numerical correctness, assert the expected VALUE, not just a property of it.

### Windows path issues
`search_files` may fail on Windows drive paths (e.g., `C:\...` or `/c/...`). Use `terminal` with `find` as a fallback for file discovery. MSYS-style paths (`/c/Users/...`) work alongside native paths in terminal commands.

### Thread determinism
The value-model GBM diverges ~1e-2 multi-threaded. Determinism tests require `OMP_NUM_THREADS=1` and `threadpool_limits(limits=1)`. Always set single-threaded when running tests that assert exact values.

## Phase Build History (PR log)

| PR | Phase | Title |
|----|-------|-------|
| #91 | 2c | Wire smoothed_talent into the ROS talent features |
| #90 | 2c | Score ROS (H1) with the volume x efficiency model |
| #89 | 2c | Fix verdict runner player universe |
| #88 | 2c | Add verdict runner + report artifacts |
| #87 | 2c | Add Tier-2 matchup check + calibrate-then-gate registry |
| #86 | 2c | Add Tier-1 skill + calibration runner |
| #85 | 2c | Add H2 lineup simulation machinery |
| #84 | 2c | Add bootstrap_ci helper for directional CIs |
| #83 | 2b | Add preseason path + wire projection into compute_valuations |
| #82 | 2b | Wire cross-league priors into real data |
| #81 | 2b | Add SV→fantasy-points map |
| #80 | 2b | Add per-position weekly Layer-1 credit |
| #79 | 2b | Add fitted stat-line projection model |
| #78 | 2b | Add GRID talent-feature accessor |
| #77 | 2b | Add volume/role model |
| #76 | 2a | Add leakage-guard suite |
| #75 | 2a | Add rolling-origin walk-forward driver |
| #74 | 2a | Add baseline forecasters |
| #73 | 2a | Add as-of accessor + cache namespace + tripwire |
| #72 | 2a | Add backtest metrics module |
| #71 | 1 | Add known-player rank-band sniff gate |
| #70 | 1 | Multi-season GRID ingest + frozen snapshot |
| #69 | 1 | Wire weekly_update to load_grid_plays + real-data-tolerant RAPM |
| #68 | 1 | Real participation loader + load_grid_plays |
| #67 | 1 | nflverse→GRID plays-contract adapter |
| #66 | 0 | Synth calibration gates + QB filter calibration |
| #65 | 0 | Three-layer golden master |
| #64 | 0 | In-process determinism gate + Tier 0 recovery gates |
| #63 | 0 | Engine prereqs: predictive (mean, S), trajectory band, injectable cache paths |