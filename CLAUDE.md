# CLAUDE.md — GRID Engine

Durable project rules. Detail lives in versioned docs, not here.
Full map: `docs/00-meta/authority-index.md`. Module rules: `docs/CLAUDE.md`.
Oracle operating rules: `reference/python/README.md`.

## Authority order (engine-spec.md §1.5)

1. the specification, `engine-spec.md`, and its byte-identical mirror at
   `docs/00-meta/specs/engine-spec.md`
2. accepted Architecture Decision Records in `docs/02-adr/`
3. versioned contracts and schemas, model specifications, and provider manifests in
   `docs/03-contracts/`, `docs/05-model-specs/`, and `docs/04-providers/`
4. the approved work-package file in `docs/01-work-packages/`
5. tests and fixtures that implement the approved contracts, including committed reference-oracle
   fixtures and golden files
6. existing source code, comments, and local conventions, including the `reference/python/` source

Existing code is not authoritative merely because it already exists. Tests are not authoritative if
they contradict a higher-level approved requirement. On a conflict or a missing decision that
changes behavior: **stop the work package at a clean boundary and raise a decision request.** Do
not pick silently.

The oracle never overrides the spec. A disagreement between oracle behaviour and a higher authority
is a decision request; oracle behaviour is never a reason to weaken a rule. The superseded specs
(`docs/00-meta/specs/superseded/`), `docs/07-archive/` and `docs/06-sessions/` are history only.
Registers in `docs/00-meta/` change requirements only once carried into the spec, an accepted ADR
or a model spec. This order is DR-A2: adopted by P0-01, in force once the owner's merge comment
ratifies it (`docs/02-adr/README.md`).

## Commands

```bash
just bootstrap    # once per fresh environment: dev tools + local SQLite dev database
just verify       # Linux smoke
powershell -ExecutionPolicy Bypass -File scripts/verify.ps1 -Scope Full   # merge-authoritative (DR-A3)
just evidence <WP-ID>   # evidence manifest for a work package
```

Reference oracle (Linux only, CPython 3.11 + `OPENBLAS_CORETYPE=Haswell`, outside the verify chain):

```bash
cd reference/python
python3 -m venv .venv && . .venv/bin/activate   # python3 must be CPython 3.11
pip install -r requirements.txt -c requirements.lock
export OMP_NUM_THREADS=1 OPENBLAS_NUM_THREADS=1 MKL_NUM_THREADS=1
python3 tools/verify_manifest.py && python3 -m pytest   # pytest pins OPENBLAS_CORETYPE=Haswell itself
```

**Verification recipes are a frozen contract.** Make the repository satisfy them. Never edit a
recipe, weaken a lint, or skip a check to make a failure disappear — if a recipe cannot pass,
stop and report. A recipe changes only by ADR (ADR-001 D5). `scripts/check-verify-parity.sh`
keeps `justfile`, `scripts/verify.sh` and `scripts/verify.ps1` in lockstep. Never wire an oracle
step into the chain without its own ADR.

Requires `DATABASE_URL=sqlite:target/grid-dev.db` and `SQLX_OFFLINE=true`, both supplied by the
committed `.env`; see `docs/02-adr/002-sqlx-offline-cache.md`. Offline is the default so a fresh
clone compiles against the committed `.sqlx` cache with no database. `cargo sqlx prepare`
overrides it when you need to regenerate the cache. `scripts/check-env-contract.sh` checks that
`DATABASE_URL` is well-formed and that `SQLX_OFFLINE` agrees with whether `.sqlx/` exists — it
does not, and cannot, assert which value you should be using.

## Engine non-negotiables (engine-spec.md §1.1)

- **Engine only.** Rust library API, a headless CLI that is a thin adapter over it, and versioned
  file and database outputs. No UI, Flutter, FFI layer or installer in this repository.
- **Rust owns all authoritative state.** In-memory state is a cache rebuildable from SQLite and
  registered immutable artifacts.
- SQLite is the durable source of truth, via SQLx with compile-time-checked queries. Migrations
  are append-only.
- Tokio for async I/O; Rayon or `spawn_blocking` for CPU-heavy math. Never block the runtime.
  Numerical crates stay synchronous.
- **No Python in any engine crate, binary, build script, runtime or release artifact** — no
  embedding, spawning, linking or pyo3. Python exists only as the dev-time oracle in
  `reference/python/`. Its outputs reach Rust only as committed, content-hashed fixtures and
  goldens. Removing `reference/python/` must not break `cargo build` or `cargo test`.
- Once-daily external fetch per source. No continuous polling.
- Deterministic versioning: Data → Feature → Model → Prediction.
- No coding-model dependency at run time.

## Generated files — never hand-edit

| Artifact | Regenerate with |
|---|---|
| `.sqlx/` | `cargo sqlx prepare` |
| `Cargo.lock` | `cargo generate-lockfile` |
| oracle-derived fixtures and goldens (`fixtures/parity/`) | the oracle exporter; a regeneration needs a reviewed semantic explanation and a correction-ledger entry |

Change the source definition and re-run the generator. Golden files are never regenerated
without a reviewed semantic explanation.

## Dependencies and migrations

- No new production dependency without written justification and owner approval. Prefer what
  is already in `[workspace.dependencies]`.
- Verify crate names and APIs against pinned docs or source. Never from memory.
- Major upgrades are their own work package, never folded into feature work.
- `rust-toolchain.toml` is not touched inside a feature PR.
- **Migrations are append-only.** Never rewrite, squash, or delete one to make a build pass.
  Enforced by `scripts/check-migrations.sh`.

## Reference oracle rules (engine-spec.md §1.7, ADR-012)

- Never edit `reference/python/backend/` or `reference/python/tests/` except through an approved
  correction-ledger entry (`reference/python/PARITY.md` §(b)), approved by the statistical owner.
- Never change oracle semantics, tolerances, fixtures or goldens, and never regenerate them, to
  make a Rust test pass.
- Never port a known oracle defect (`docs/00-meta/known-issues.md`) "for parity". Cite the `KI-…`
  entry and implement the spec.
- Linux only, BLAS/OpenMP threads pinned to 1, and the pinned numerical platform: CPython 3.11 with
  `OPENBLAS_CORETYPE=Haswell` (KI-NEW-Z78; DR-D31, correction-ledger entry L0). The golden is frozen on
  it, and `reference/python/tools/pytest_platform_pin.py` enforces it for pytest; export it yourself
  for the demo, the golden generator and investigations. Never loosen a tolerance or disable the pin
  to absorb a platform difference. Real third-party data is never committed; parity fixtures are
  synthetic-only.
- `python3 tools/verify_manifest.py` must pass: the imported files match `MANIFEST.tsv`.
- Legacy-oracle numbers and historical real-data results are never evidence of engine accuracy.

## Decisions

Proposed defaults in sections B, C and D of `docs/00-meta/decision-register.md` stay **proposed
until the named owner ratifies them**. Never present one as settled. A work package is not Ready
while a decision it depends on is unratified. The register is authoritative for status.

## Prohibited (engine-spec.md Appendix D)

Claiming a command passed when it was not run against the final commit; inventing a provider
field, crate, package, SQLx behavior or model formula; hand-editing generated code or artifacts;
rewriting migrations (item 4); weakening a test, metric, leakage rule, lock rule, warning policy,
parity tolerance, recovery floor or security control; auto-accepting golden changes; using
post-lock information, competitor projections, or target outcomes as features; converting
malformed data to a healthy default; adding an unapproved dependency or external host; exposing
credentials or private data; merging, signing, publishing, promoting, or deploying on the agent's
own authority; leaving a critical path stubbed while marking a package done; changing
`reference/python/` or regenerating oracle fixtures to make a Rust parity test pass; porting a
known oracle defect without a recorded decision; presenting a proposed decision as settled, or
treating a package as Ready while a decision it depends on is unratified; citing legacy-generator
values or historical cautious-nevermore real-data results as parity or accuracy evidence.

## Branch, commit, PR

- One work package per branch: `wp/P<phase>-NN-description`.
- Atomic commits. The human reviews the diff before each commit. Every commit message cites its
  work package or ADR (`P1-06`, `ADR-012`, …) so `scripts/check-traceability.sh` can trace it.
- PR body follows Appendix C and covers every §12.8 item.
- **The agent never merges.** A fresh-context reviewer who is not the implementer is required.

## Before claiming done

Evidence, not assertion. A completion claim needs the verification command, its exit status,
a test summary, and `.ai/evidence/<WP-ID>/manifest.json`. Generate it with
`just evidence <WP-ID>` — never hand-write it. No unexplained `TODO`, skipped test, or
placeholder may remain in scope.
