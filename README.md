# GRID Engine

GRID Engine is a statistical player-value and projection engine for NFL fantasy football. It is
meant to produce week-by-week stat-line distributions for QB, RB, WR and TE, then turn them into
fantasy points for Standard, Half-PPR, PPR and custom scoring.

GRID (Game-state Relative Individual Decomposition) is the engine's play-level estimation stack. It
values each play against a situational-value surface V(s), attributes that value to the players on
the field with regularized adjusted plus-minus, reconciles team strength to the betting market,
tracks each player's talent, form and scheme fit with a state-space filter, and translates
feeder-league evidence into NFL priors. Every estimator is validated first on whether it recovers
planted ground truth in a synthetic world.

This repository builds **the engine only**:

- **Rust is the target.** The engine is a set of Rust library crates over SQLite, plus a headless
  CLI that is a thin adapter over the library API. It has no user interface, installer or FFI layer
  (ADR-011).
- **Python is the reference oracle.** `reference/python/` holds the working Python GRID engine
  imported from cautious-nevermore. It produces parity fixtures and synthetic-recovery evidence in
  development and CI. It is never part of an engine build, binary or release artifact (ADR-012).

## Status

**Pre-implementation.** Permitted wording for anything this engine produces today: *Experimental
projections* (`engine-spec.md` §3.2, §3.3).

- **Rust engine: stubs.** The workspace has 11 crates. Only `persistence` has code and tests: the
  SQLx scaffold with one infrastructure migration. No engine computation exists yet.
- **Python oracle: runs.** At import it passed its own suite on Linux: 446 tests, CPython 3.11.15,
  threads pinned to 1, isolation guard clean (`reference/python/PARITY.md` §(a)). Its golden master
  is frozen on a pinned numerical platform, CPython 3.11 with `OPENBLAS_CORETYPE=Haswell`, which any
  x86-64 AVX2 CPU runs: correction-ledger entry L0 regenerated it there (KI-NEW-Z78; DR-D31, ratified by the owner on 2026-10-08), and `reference/python/tools/pytest_platform_pin.py` enforces it. It
  passes 446 tests under the pin.
- **Known defects are registered, not fixed.** The imported oracle has known defects, all recorded in
  `docs/00-meta/known-issues.md`. The most consequential is in the synthetic generator, which draws
  every play's defenders from the offense's own team (KI-NEW-Y0). Corrections are proposed through a
  correction ledger (`reference/python/PARITY.md` §(b)); none of the semantic corrections has been
  applied. Only entry L0, the platform pin above, is applied, and it fixes no model defect.
- **GRID has not cleared the Phase-1 gate on historical evidence.** The historical cautious-nevermore
  real-data results did not show GRID beating the stronger transparent baselines, and they rest on
  biased labels and too-narrow intervals. They are non-citable for this engine
  (`engine-spec.md` §3.3; `docs/07-archive/cautious-nevermore/real-data-results.md`).
- **Owner decisions are open.** Most modeling and parity decisions are proposed defaults awaiting the
  owner (`docs/00-meta/decision-register.md`). A work package is not Ready while a decision it
  depends on is unratified.

The active work package is P0-01, the engine-only consolidation
(`docs/01-work-packages/p0-01-engine-consolidation.md`).

## Quick start

Rust workspace (Linux smoke; `scripts/verify.ps1 -Scope Full` on Windows is merge-authoritative):

```bash
just bootstrap    # once per fresh environment: pinned dev tools + local SQLite dev database
just verify       # Linux smoke: the frozen verify chain
powershell -ExecutionPolicy Bypass -File scripts/verify.ps1 -Scope Full   # merge-authoritative (DR-A3)
```

The committed `.env` supplies `DATABASE_URL=sqlite:target/grid-dev.db` and `SQLX_OFFLINE=true`
(ADR-002).

Python reference oracle (Linux only, CPython 3.11 + `OPENBLAS_CORETYPE=Haswell`; KI-NEW-Z78, DR-D31):

```bash
cd reference/python
python3 -m venv .venv && . .venv/bin/activate   # python3 must be CPython 3.11 (the lock and the golden)
pip install -r requirements.txt -c requirements.lock
export OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 MKL_NUM_THREADS=1
python3 tools/verify_manifest.py    # imported files match MANIFEST.tsv
python3 -m pytest                   # the oracle's own suite, with the isolation guard and platform pin
```

Under pytest the platform pin sets `OPENBLAS_CORETYPE=Haswell` itself and refuses any other kernel or
interpreter. For `run_demo.py`, the golden generator and `tools/investigations/`, export
`OPENBLAS_CORETYPE=Haswell` and use CPython 3.11 yourself. Never loosen a tolerance to absorb a platform
difference.

The oracle job (`reference-oracle` in `.github/workflows/alpha-ci.yml`) runs the same steps on the
runner image's tool-cache CPython 3.11 with `OPENBLAS_CORETYPE=Haswell`, and fails if that interpreter
is absent. It is expected green (446 passed) on ordinary `ubuntu-latest` runners. It sits outside the
frozen verify chain and is not merge-authoritative (`engine-spec.md` §8.19).

## Repository map

| Path | Contents |
|---|---|
| `engine-spec.md` | The engine specification: authority level 1. Mirrored byte-identically at `docs/00-meta/specs/engine-spec.md` |
| `CLAUDE.md`, `docs/CLAUDE.md` | Durable rules for the implementing agent and for humans: authority, commands, boundaries |
| `crates/` | Rust workspace: `domain`, `persistence`, `ingestion`, `identity`, `features`, `models`, `simulation`, `scoring`, `evaluation`, `governance`, `application` |
| `migrations/` | Append-only SQLx migrations |
| `.sqlx/` | Generated SQLx offline query cache (never hand-edited) |
| `reference/python/` | Python reference oracle: `backend/`, `tests/`, `MANIFEST.tsv`, `PARITY.md`, `patches/`, `tools/` |
| `scripts/`, `tests/guards/` | Verify scripts, repository guards and the guard behaviour suite |
| `toolchains/` | Pinned developer tools |
| `.ai/evidence/<WP-ID>/` | Committed evidence manifests for completed work packages |
| `docs/` | The documentation vault (below) |

Not yet present: `fixtures/` (parity and provider fixtures), `crates/pipeline/`, `crates/grid-cli/`
and `crates/synth/`. Each has an owner work package (`engine-spec.md` §8.14).

## Documentation map

| Path | Contents |
|---|---|
| `docs/00-meta/` | Authority index, decision register, known issues, lessons learned, dashboard, daily log; spec mirror and superseded specs under `specs/` |
| `docs/01-work-packages/` | One file per work package, plus the index |
| `docs/02-adr/` | Architecture Decision Records |
| `docs/03-contracts/` | Plays contract, engine output contract, parity-fixture contract |
| `docs/04-providers/` | Provider contracts (nflverse, CFBD) |
| `docs/05-model-specs/` | Model specifications, one per GRID or projection component (index in its README) |
| `docs/06-sessions/` | Session logs and review reports (records) |
| `docs/07-archive/` | Non-authoritative history, including the archived cautious-nevermore documents |
| `docs/99-templates/` | ADR, model-spec, provider-contract, session-log and work-package templates |

**Order of authority** (`engine-spec.md` §1.5; full map in `docs/00-meta/authority-index.md`):

1. `engine-spec.md` and its byte-identical mirror at `docs/00-meta/specs/engine-spec.md`
2. accepted ADRs in `docs/02-adr/`
3. contracts, model specifications and provider manifests (`docs/03-contracts/`,
   `docs/05-model-specs/`, `docs/04-providers/`)
4. the approved work package in `docs/01-work-packages/`
5. tests and fixtures, including committed reference-oracle fixtures and golden files
6. existing source code and conventions, including the `reference/python/` source

The superseded specifications, `docs/07-archive/` and `docs/06-sessions/` are history and carry no
authority.

## Where the application went

This repository used to plan a Flutter Windows application, and cautious-nevermore held a
fantasy-football dashboard (FastAPI, React, league adapters, draft and trade tooling) built around
the Python engine. Both are out of scope here (ADR-011). The dashboard application stays in
`Seismic-Fate/cautious-nevermore`. Consumers such as applications, dashboards and notebooks live
outside this repository and integrate only through the versioned engine output contract
(`docs/03-contracts/engine-output-contract.md`) and the engine's exports and reports.

## Licence

The repository is licensed under `MIT OR Apache-2.0` (`LICENSE-MIT`, `LICENSE-APACHE`).

`reference/python/` is the exception for now. cautious-nevermore carried no licence file, and the
licence ruling for the imported oracle is pending (DR-A10). Until that ruling is recorded, do not
treat `reference/python/` as licensed under the repository's licence files
(`reference/python/README.md`). Real third-party data is never committed (DR-A11).

## Contributing

Work is done in bounded work packages, by humans or by Claude Code as the implementing agent.

- **Work packages.** Each change belongs to an approved work package in `docs/01-work-packages/`, on
  its own branch `wp/P<phase>-NN-description`. Commit messages cite the work package or ADR, which
  the traceability guard checks.
- **Decisions.** A conflict, or a missing decision that changes behaviour, stops the work at a clean
  boundary and becomes a decision request (`docs/00-meta/decision-register.md`). Architecture
  decisions are recorded as ADRs in `docs/02-adr/`.
- **Evidence.** A completion claim needs the verification command, its exit status, a test summary
  and an evidence manifest generated with `just evidence <WP-ID>`.
- **Review and merge.** A fresh-context reviewer who is not the implementer reviews every pull
  request. The repository owner merges. The agent never merges.

Start with `CLAUDE.md`, then `docs/00-meta/authority-index.md`.
