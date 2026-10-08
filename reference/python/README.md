# reference/python: the GRID Python reference oracle

This directory is the working Python GRID engine from `cautious-nevermore`, imported as an
**executable reference oracle** for the Rust engine. Its synthetic-recovery gates and its
golden master are parity targets. Its known defects are parity *anti*-targets: see
`PARITY.md` and `docs/00-meta/known-issues.md`.

The oracle's role has three limits:

- **Dev-time only.** The engine never ships Python; nothing in `crates/` reads this tree.
- **Never authoritative.** It never overrides the consolidated engine spec. Per
  `engine-spec.md` §1.5 (authority order) and §1.7, and ADR-012
  (`docs/02-adr/012-python-reference-oracle.md`), source code here sits at the bottom of the
  authority order. Committed oracle fixtures and goldens rank as tests and fixtures.
  - If the oracle and a spec disagree, that is a decision request, not a reason to copy the
    oracle.
  - An accepted divergence is recorded in `PARITY.md` and decided by a statistical-owner ADR.
- **Read before trusting a number.** On the Rust side it is used through `PARITY.md`, which
  holds the oracle status, the correction ledger, the tolerance classes and the deliberate
  divergences.

## Provenance

| What | Value |
|---|---|
| Source repository | `Seismic-Fate/cautious-nevermore` (https://github.com/Seismic-Fate/cautious-nevermore) |
| Imported commit | `59bce1d` = `59bce1d9f6aac55e823339c044620ddb1d673b7a` (main, 2026-07-18, "Stage 1: reskin Dashboard + shared components (VizAgentBox, DraftGrade) (#96)"); the complete, current engine: no engine file was ever deleted, and no unmerged branch carries an engine change |
| v0 engine upload | `6b0eeee` "initial: existing GRID engine files" (2026-06-21). The v0 engine lived at the repo root, and its `run_demo.py` wrote to `/mnt/user-data/outputs`, a Claude.ai sandbox path |
| Package move | `87e227f` "feat: reorganize GRID engine into backend/grid package" (2026-06-21) |
| First engine audit | PR #53 "fix: GRID engine audit remediation (P0+P1)", merged 2026-06-22. It added the Joseph-form covariance update (C1), the solve-based RTS (C2), the conditioning check with a **deliberate `lstsq` fallback** (C3), the unknown-ID guard in `build_design` (C4), interventions through `kalman_step` (C7), FLEX-aware VOR (W1) and `SSParams.from_position` (W7). This audit is earlier than, and separate from, the 2026-07-13 G/P/V/A/W audit in CN's `docs/06-issues-log.md` |
| Inventory | `docs/06-sessions/2026-10-01-consolidation-inventory/`. The python-closure report is the import manifest's derivation, and the critic report adjudicates between the reports |

The local CN clones are shallow (oldest visible commit 2026-06-21). The `6b0eeee`, `87e227f`
and PR #53 facts above were read from GitHub on 2026-10-01.

**Import rule.** Exactly 105 upstream files, copied from `git archive 59bce1d` with their
relative paths kept. 103 are byte-identical. Two carry documented patches, which are kept as
unified diffs in `patches/`:

| Patch | File | Why |
|---|---|---|
| P1 | `backend/validation/lineup_sim.py` | Inlines the 9-line, stdlib-only `snake_order`. Its body is verbatim from `backend/services/mock_draft.py:38-46` and only the docstring is extended. This cuts the one import edge into the app-only `backend.services` package. |
| P2 | `run_demo.py` | `OUT = os.environ.get("GRID_DEMO_OUT", "/mnt/user-data/outputs")`. The default is unchanged, and the sandbox path is not writable on CI runners. |

`MANIFEST.tsv` records each imported file. Paths in it are relative to this directory. Its
columns are `dest_path`, `source_path`, `source_commit`, `source_sha256`, `dest_sha256` and
`status` (`verbatim`, `patched:P1` or `patched:P2`). Re-check it with:

```bash
python3 -m tools.verify_manifest                                   # dest hashes, verbatim == source, completeness
python3 -m tools.verify_manifest --upstream /path/to/cautious-nevermore   # also re-derives source hashes and re-applies the patches
```

The package name stays `backend.*`. Byte-identity with upstream is what makes the oracle
auditable, and a rename would touch 282 import lines in 77 files.

## License

Pending **DR-A10** (owner ruling). `cautious-nevermore` has no LICENSE file. The intended
ruling is that the owner, as sole substantive author, brings this CN-derived code under
`MIT OR Apache-2.0`, like the rest of this repository. It would be recorded on the pivot PR
in the same form as GRID-Engine PR #1 comment 5357318508. Until that ruling exists, do not
treat this directory as licensed under the repository LICENSE files. Third-party data is
outside any such ruling and is never committed here (DR-A11).

## How to run (Linux only)

```bash
cd reference/python
python3 -m venv .venv && . .venv/bin/activate                 # Python 3.11 (verified: 3.11.15)
pip install -r requirements.txt -c requirements.lock           # engine + pytest
export OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 MKL_NUM_THREADS=1
python3 -m pytest                                              # 446 passed, about 150 s on 4 vCPU
```

- `pytest.ini` pins rootdir here and sets `pythonpath = .` (so there is no root conftest).
  It also loads `tools/pytest_isolation_guard.py`, which fails the run on any network
  access and on any import of an app-only module (`backend.api`, `backend.services`,
  `backend.trades`, `backend.adapters`, `backend.viz_agent`, `fastapi`, `uvicorn`,
  `anthropic`, `posthog`, `httpx`, `websockets`, and others) or of the demo-only
  `matplotlib`. A clean run ends with `isolation guard: 0 violations`.
- Plain `pytest`, and runs from the repository root (`python3 -m pytest reference/python/tests/...`),
  work as well. Both need pytest >= 8.4 (see `requirements.txt`).
- **Pin the threads.** The golden master and determinism tests pin to one thread
  internally with `threadpool_limits(1)`. Without the environment variables, sklearn's
  OpenMP and OpenBLAS oversubscribe a shared runner, and the inventory saw one test file
  slow down more than 10×.

Demo (documentation, not a gate; its numbers are asserted by `tests/grid/test_tier0_recovery.py`):

```bash
pip install -r requirements-demo.txt -c requirements.lock      # adds matplotlib
GRID_DEMO_OUT=/tmp/grid-demo python3 run_demo.py               # about 8 s; writes two PNG files there
```

With threads pinned to 1, the demo prints `overall 0.803`, `team strength 0.664`,
`current-ability (tau+form) corr 0.958`, `talent (tau) 0.674` and
`rookie prior ... corr 0.583`. That matches the Tier-0 gate's observed 0.958 / 0.6745.
`run_demo.py` does not pin threads itself. With default threading on a 4-vCPU Linux box it
prints `0.972` / `0.955` / `0.675` for the three state-space lines, and its text and PNG files
are byte-identical to the inventory's run. The difference is GBM thread-count sensitivity.
The attribution, team and prior lines are identical under both settings.

**Linux only.** The oracle's platform of record is Linux x86_64. Two tests are recorded as
failing on Windows: golden master Layer C (cross-platform GBM variance) and
`tests/grid/test_cache.py::test_ttl_expired` (mtime granularity). The source for this is
CN's `docs/10-next-steps-plan.md` Stage 0 and inventory critic G-6; neither was re-run on
Windows here.

- The oracle CI job therefore runs on `ubuntu-*` only.
- It is outside the frozen `just verify` / `verify.ps1` chain.
- It is never wired into `windows-authoritative` (DR-A3).
- A Windows checkout with `core.autocrlf=true` also breaks every `MANIFEST.tsv` hash.

**Golden master.** `tests/grid/golden/snapshot.npz` (Tier 0.5) is regenerated by
`python3 -m tests.grid.golden_master`, run from this directory. Never run it to make a test
pass. A regeneration is an oracle correction: it needs a correction-ledger entry in
`PARITY.md`, a failing test first, a model-spec note and the approver named there (DR-B1).

## Environment variables

| Variable | Read by | Notes |
|---|---|---|
| `DB_PATH` | `backend/db/connection.py:8` | SQLite path, default `data/db/fantasy.sqlite` (cwd-relative). This is the only variable engine code reads. **Never put `DB_PATH` in the repository-root `.env`:** `connection.py` calls `load_dotenv()` at import, which walks up from `backend/db/` to GRID-Engine's root `.env`, which belongs to the Rust toolchain (ADR-002). That root file's `DATABASE_URL` and `SQLX_OFFLINE` already leak into the Python process. This is harmless today, because no Python code reads them. Set `DB_PATH` in the shell if you need it. |
| `GRID_DEMO_OUT` | `run_demo.py` (patch P2) | Output directory for the two demo PNG files. Default `/mnt/user-data/outputs`. |
| `OMP_NUM_THREADS`, `OPENBLAS_NUM_THREADS`, `MKL_NUM_THREADS` | numpy, scipy, scikit-learn | Set all three to `1` for every oracle run: tests, CI, demo and investigations. The scripts in `tools/investigations/` default them to 1. |

**Runtime writes are cwd-relative.** Run from this directory and they stay under its
gitignored `data/`:

- `data/cache/*.parquet`, `*.npz` and `*.joblib`;
- `data/db/`, `data/logs/pipeline.log` and `data/health.json`;
- `data/snapshots/` and `data/reports/`;
- `data/realdata/`.

A full test run leaves `data/health.json` and `data/logs/pipeline.log`. It also writes
outside the tree: `tests/grid/test_nflverse_loader.py` writes `/tmp/test_cache/`
(KI-NEW-I6), and that is not fixed here because the oracle's tests are verbatim.

## Read this before trusting a number

- **KI-NEW-Y0: legacy synthetic generator.** `backend/grid/synth.py:193` draws every
  play's "defenders" from the **offense's own** DEF players (100% of canonical plays).
  - Every synthetic recovery number, Tier-0 observed value and golden-master array in this
    tree is a **legacy-generator** value. That includes the README numbers above and the
    "observed" comments in the tests.
  - None of them may be frozen as a Rust parity target until the correction ledger in
    `PARITY.md` (entry 1) has run.
  - `tools/investigations/tier0_legacy_vs_fixed.py` and `pytest_defender_fix.py` show the
    impact: Tier-0 still passes 8/8, the golden master fails 4/11, synth calibration fails
    1/6, and focus-QB NIS goes from 4.58 to 8.37.
- **KI-NEW-D1: `backend/db/data/coaching_changes_2025.json`** is unverified and
  illustrative. It contains 2024-cycle changes labelled 2025, Ben Johnson as CHI OC, a
  quarterback as an offensive coordinator, and other implausible rows.
  - It is imported verbatim only because `backend/db/seed_coaching.py` defaults to it.
  - No test or automatic path loads it.
  - It is **never a fixture or provider input**. In Rust, coaching changes become a
    sourced provider contract.
- **Real-data verdicts are historical and non-parity.** The CN real-data H1/H2 verdict,
  `valuations` and `player_stats` rest on a biased box-score ingest (KI-NEW-I1..I5),
  iid-bootstrap CIs (KI-NEW-V1) and current-season participation skew. No real-data number
  from this oracle is a parity target.
- **`weekly_update` is not an oracle.** It has the season-rollover skip (KI-NEW-W1),
  roster-change reinit (KI-NEW-W2), a cumulative-RAPM Kalman observation (KI-NEW-W3) and
  `snaps = 1` (KI-#24). The validated path is batch walk-forward (`validation/backtest.py`,
  `validation/verdict.py`).
- The full defect list, with oracle and Rust impact for each item, is in
  `docs/00-meta/known-issues.md`. Executable evidence is in `tools/investigations/` (see its
  README).

## Module to model-spec map

| Oracle module(s) | Specified by |
|---|---|
| `backend/grid/value.py` (V(s), dV) | `docs/05-model-specs/value-model.md` |
| `backend/grid/layers.py`: `build_design`, `run_rapm`, `_solve_ridge_prior`, accumulators, Layer-3 market rows, `fit()` fixed point; `backend/grid/situations.py`; `run_situation_rapm` | `docs/05-model-specs/rapm-attribution.md` |
| `backend/grid/layers.py`: `layer1_*` (cross-fitted per-play credit) | `docs/05-model-specs/layer1-credit.md` |
| `backend/grid/statespace.py` (Kalman, RTS, changepoints, `KalmanState`) | `docs/05-model-specs/state-space-kalman.md` |
| `backend/grid/priors.py` (equivalency, priors, washout) | `docs/05-model-specs/cross-league-priors.md` |
| `backend/grid/synth.py`, `data_adapters.load_synthetic` | `docs/05-model-specs/synthetic-world.md` |
| `backend/projection/*`, `backend/scoring/{engine,columns,formats}.py` | `docs/05-model-specs/projection-stack.md` |
| `backend/validation/*` (as-of guards, walk-forward backtest, baselines, metrics, Tier 1/2, lineup sim, verdict, thresholds) | `docs/05-model-specs/evaluation-and-leakage.md` |
| `backend/grid/data_adapters.py` (contract docstring), `nflverse_adapter.py` | `docs/03-contracts/plays-contract.md` |
| `backend/grid/nflverse_loader.py`, `backend/pipeline/{data_pipeline,ingest_grid}.py` | `docs/04-providers/nflverse/README.md` and `access-and-license.md` |
| engine outputs (ratings, team strength, grades, Kalman trajectory, valuations) | `docs/03-contracts/engine-output-contract.md` |
| `tests/grid/golden_master.py`, `tests/grid/golden/snapshot.npz`, Tier-0 gates | `docs/03-contracts/parity-fixture-contract.md` and `PARITY.md` |
| `backend/pipeline/{weekly_update,compute_valuations,health_check,_logging}.py`, `backend/grid/cache.py`, `backend/db/*`, `backend/scoring/{vor,format_registry}.py` | No model spec. These are operational or support code kept for import closure. The engine operating model, storage and VOR scope are DR-C14, DR-C15 and DR-C11 |

## What was excluded, and why

131 of the 236 files tracked at `59bce1d` were left out.

| Excluded | Why |
|---|---|
| `backend/api/**`, `backend/viz_agent/**`, `backend/trades/**`, `backend/services/**`, `backend/adapters/**` (29 files) | The fantasy-dashboard app: FastAPI, the LLM chart agent, trade tooling, draft tooling and WebSocket, ESPN/Sleeper league adapters. These consume engine outputs and hold no engine estimator. The pivot is engine-only (ADR-011). |
| `backend/pipeline/sync_leagues.py`, `sync_trade_history.py` | League and trade sync through the app adapters. |
| `backend/fantasy_scoring.py` and `tests/test_fantasy_scoring.py` | An orphaned predecessor of `backend/projection` with hand-picked scaling constants. Docstrings in `projection/model.py`, `volume.py` and `sv_to_points.py` call it superseded, and nothing imports it except its own 3 tests. |
| Tests of all of the above (`tests/{adapters,api,services,trades,viz_agent}/**`, `tests/pipeline/test_sync_*.py`) | 188 tests of excluded modules, plus the 3 orphan tests above: 191 in all. |
| `frontend/**`, `scripts/run_pipeline.bat`, `scripts/setup_scheduler.ps1` | React UI and a Windows scheduler wrapper. The wrapper never ran `ingest_grid` or `weekly_update`. |
| Upstream `requirements.txt`, `.gitignore`, `.gitattributes`, `.env.example`, `CLAUDE.md`, `README.md`, `docs/**`, `sdd/**`, the two demo PNG files | Replaced by engine-only versions here, or carried into the consolidated specs and `docs/07-archive/`. |

Kept for import closure even though they are not engine estimators:

- `backend/db/schema.sql`, verbatim, including the inert app tables that
  `tests/db/test_schema.py` asserts;
- `backend/scoring/{vor,format_registry}.py`;
- `backend/pipeline/{data_pipeline,compute_valuations,weekly_update,health_check,_logging}.py`;
- `backend/db/seed_coaching.py` and its JSON.

This oracle scope is separate from the Rust **port** scope recorded in ADR-011.

## Directory layout

| Path | Contents |
|---|---|
| `backend/`, `tests/`, `run_demo.py` | Imported upstream files (`MANIFEST.tsv`). `tests/projection/` and `tests/validation/` have no `__init__.py` upstream, and that is kept. |
| `patches/` | P1 and P2 as unified diffs against `59bce1d`. |
| `MANIFEST.tsv` | Per-file provenance and hashes. |
| `PARITY.md` | Oracle status, correction ledger (proposed), legacy vs fixed Tier-0, tolerance classes, deliberate Rust divergences, non-gating checks, lifecycle. |
| `requirements.txt`, `requirements-demo.txt`, `requirements.lock` | Engine dependencies, demo extra, and exact verified pins (Python 3.11.15). |
| `pytest.ini`, `tools/pytest_isolation_guard.py` | Test configuration and isolation enforcement. |
| `tools/verify_manifest.py` | Manifest verifier (stdlib only). |
| `tools/investigations/` | Repro scripts for the recorded defects and decisions, and the pinned real-data fetcher. |
