---
adr-id: 012
status: Proposed
date: 2026-10-07
deciders: Product/Architecture owner (DR-A2, DR-A12); Statistical owner (DR-A2 oracle placement, accepted divergences); Data/Licensing owner (DR-A10, DR-A11); Security/Release owner (the CI job and its setup)
supersedes:
superseded-by:
---

# ADR-012 — Python reference oracle

## Status

**Proposed** 2026-10-07 under work package P0-01, as the companion to ADR-011. It is not accepted.

- **What it adopts.** DR-A2 (oracle placement), DR-A11 and DR-A12, subject to owner ratification at the
  merge of the P0-01 pull request.
- **DR-A10 is not adopted here.** It needs the Data/Licensing owner's ruling, recorded on the pull
  request, before merge (§8).
- **What it does not decide.** Every B, C and D decision it mentions remains proposed or open. The
  correction-ledger policy (DR-B1) in particular is **not part of this ADR**.

## Context

- ADR-011 D2 makes Rust the target and keeps the working Python GRID engine as an executable reference.
  The owner chose that option on 2026-10-01.
- **The reference.** It is the cautious-nevermore engine at `59bce1d`
  (`59bce1d9f6aac55e823339c044620ddb1d673b7a`): `backend.*`. It is validated against planted synthetic
  truth by recovery gates (Tier 0), a golden master (Tier 0.5), calibration and determinism tests.
- **Why committed fixtures.** Rust cannot reproduce numpy's PCG64 streams bit for bit. Numeric parity
  must therefore run on committed fixtures, not on a live interpreter
  (`docs/06-sessions/2026-10-01-consolidation-inventory/final-build-spec.md` §5).
- **The oracle has known defects.** The most consequential is in the synthetic generator, which draws
  every play's defenders from the offense's own team (KI-NEW-Y0, `backend/grid/synth.py:193`). Every
  team-strength, Layer-3, DEF-recovery and matchup-grade number in the oracle was measured against that
  generator.
- **The oracle is Linux-only.** Golden-master Layer C and `tests/grid/test_cache.py::test_ttl_expired`
  are recorded as failing on Windows (critic G-6).
- **There is no licence.** cautious-nevermore has no LICENSE file. GRID-Engine is `MIT OR Apache-2.0`
  (ADR-001 D4).
- **Some data cannot be committed.** nflverse participation is CC-BY-SA 4.0 (checked 2026-10-01; see
  `docs/04-providers/nflverse/access-and-license.md`), so real-data-derived fixtures cannot simply be
  committed into this tree.

## Decision

### 1. Oracle scope and port scope are separate lists (critic X-5)

| List | Definition |
|---|---|
| **Oracle scope** | The 105 files of `reference/python/MANIFEST.tsv`: `grid`, `projection`, `validation`, `scoring`, the engine parts of `pipeline`, `db` support, and their tests. This set was chosen because it is the only one proven **import-closed**, and it passes **446** tests in a clean environment with application imports blocked. The other 131 of the 236 files tracked at `59bce1d` are the application and are excluded (`reference/python/README.md`, "What was excluded"). |
| **Port scope** | What Rust implements: a strict subset, mapped module by module in `engine-spec.md` §8.1 and `docs/03-contracts/parity-fixture-contract.md`. |

These modules are in the oracle scope and **not** in the port scope:

- the scoring format registry;
- VOR and tier outputs (VOR survives only inside the evaluation lineup simulation, proposed — DR-C11);
- `compute_valuations`;
- the box-score `data_pipeline` (labels are re-derived, `engine-spec.md` §4.7);
- `health_check`;
- the TTL `ParquetCache`;
- the application database schema.

`weekly_update` is imported but is **not** a parity source (`engine-spec.md` §8.6.6). Being in the oracle
scope never puts a module in the port scope.

### 2. Authority placement (DR-A2)

- **Committed fixtures and goldens** rank at authority level 5, with tests and fixtures. That covers
  fixtures and goldens exported from the oracle.
- **The source under `reference/python/`** ranks at level 6, with code.
- **The oracle never overrides** the engine spec, an accepted ADR, a contract or a model spec.
  - A disagreement is a decision request.
  - An accepted divergence needs an ADR approved by the Statistical owner and an entry in
    `reference/python/PARITY.md`.
  - An oracle golden that contradicts a model spec is regenerated through the correction ledger. It is
    never hand-edited, and it is never obeyed over the model spec.
- **Prohibited** (`engine-spec.md` §1.7, Appendix D):
  - changing oracle semantics, tolerances, fixtures or goldens to make a Rust parity test pass;
  - regenerating them without a reviewed semantic explanation;
  - porting a known defect "for parity";
  - reading oracle output at engine run time.

### 3. Verbatim import and its integrity

- **Source.** The files come from `git archive 59bce1d`, with relative paths kept: 105 files, of which
  **102 are byte-identical** (103 at import; correction-ledger entry L0 then regenerated the golden
  snapshot, §5).
- **Two import patches.** Neither changes engine behaviour. Both are kept as unified diffs in
  `reference/python/patches/`, next to the binary git patch of ledger entry L0:
  - **P1** inlines the 9-line, stdlib-only `snake_order` into `backend/validation/lineup_sim.py`. This
    cuts the only import edge into the app-only `backend.services`.
  - **P2** makes the output directory of `run_demo.py` overridable through `GRID_DEMO_OUT`. The default
    is unchanged.
- **The manifest.** `reference/python/MANIFEST.tsv` records `dest_path`, `source_path`,
  `source_commit`, `source_sha256`, `dest_sha256` and `status` for each file. `tools/verify_manifest.py`
  is stdlib-only. It checks the destination hashes, that each verbatim file equals its source, and
  completeness. With `--upstream`, it also re-derives the source hashes and re-applies the patches.
- **The package keeps the name `backend.*`.** Byte identity with upstream is what makes the oracle
  auditable, and a rename would touch 282 import lines in 77 files.
- **The isolation guard.** `tools/pytest_isolation_guard.py`, loaded by `pytest.ini` on every run, fails
  the session on network access or on any import of an app-only module or of the demo-only
  `matplotlib`. A clean run ends `isolation guard: 0 violations`.
- **The platform pin.** `tools/pytest_platform_pin.py`, also loaded by `pytest.ini`, sets
  `OPENBLAS_CORETYPE=Haswell` before NumPy loads, refuses any other explicit kernel and any interpreter
  other than CPython 3.11, and checks through threadpoolctl that OpenBLAS runs the `Haswell` kernel. It
  pins the platform the golden master is frozen on (KI-NEW-Z78; DR-D31; ledger entry L0).
- **Engine-only files added around the import:**
  - `README.md`, `PARITY.md` and `MANIFEST.tsv`;
  - `pytest.ini`;
  - `requirements.txt`, `requirements.lock` and `requirements-demo.txt`;
  - `.gitignore`;
  - `tools/`, which includes the inventory's investigation scripts under `tools/investigations/`.
- **Line endings.** A Windows `core.autocrlf=true` checkout would rewrite the verbatim bytes and break
  every `MANIFEST.tsv` hash. The protection is a `.gitattributes` rule for the verbatim tree, which
  extends ADR-004's narrow policy. P0-01 tracks it until it lands.

### 4. The `reference-oracle` CI job runs outside the frozen verify chain

The job `reference-oracle` in `.github/workflows/alpha-ci.yml` runs on `ubuntu-latest`, with a 30-minute
timeout, `OMP_NUM_THREADS`, `OPENBLAS_NUM_THREADS` and `MKL_NUM_THREADS` all set to `1`, and
`OPENBLAS_CORETYPE=Haswell` (DR-D31, ratified 2026-10-08; ledger entry L0). Its steps:

1. select the newest tool-cache CPython 3.11 on the runner image
   (`/opt/hostedtoolcache/Python/3.11.*/x64/bin/python3`), failing the job if there is none, and record
   the interpreter, the machine and the CPU model;
2. run `tools/verify_manifest.py` with that interpreter before anything is installed;
3. create a venv from it and run `pip install -r requirements.txt -c requirements.lock`, then `pip freeze`;
4. record the OpenBLAS kernel in use, then run `python3 -m pytest` (446 tests, expected all passing; the
   platform-pin plugin fails the session unless the interpreter is CPython 3.11 and OpenBLAS runs the
   `Haswell` kernel).

**Why it is outside the chain.**

- Adding an oracle step to `just verify` or `verify.ps1` is a D5 amendment, which needs its own ADR.
- The verify chain's merge-authoritative run is Windows (ADR-011 D8, DR-A3). There, Layer C would have to
  be skipped, so the gate would be weaker than this job.
- A separate job needs no amendment. It is never wired into `verify.ps1` or `windows-authoritative`, and
  no `cargo` target depends on it.

**Why the interpreter is the runner image's tool-cache CPython 3.11.**

- The workflow file is a security boundary: no third-party action is used beyond `actions/checkout`.
- `actions/setup-python`, or pinning the runner image, needs the Security/Release owner's approval. The
  tool-cache interpreter ships with the image, so selecting it adds no action.
- The lock was verified on CPython 3.11.15, and the image's default `python3` is 3.12, which moves the
  golden master: PR #4's first run failed the two Layer C golden tests (KI-NEW-Z78). The job therefore
  selects 3.11 explicitly and fails loudly if the image stops shipping it, rather than testing another
  interpreter.
- 3.11 removes the interpreter difference but not the kernel difference: the imported Layer C golden
  reproduced at rtol 1e-5 only with OpenBLAS AVX-512 kernels, so the AMD runners failed it. The owner
  decided that through **DR-D31** (ratified 2026-10-08, option 1; critic X-7): every oracle run pins
  `OPENBLAS_CORETYPE=Haswell`, a kernel every x86-64 AVX2 CPU runs, and correction-ledger entry L0
  regenerated the golden once under CPython 3.11 + Haswell. The job is therefore expected green on
  ordinary `ubuntu-latest` runners, with no runner change. A tolerance is never loosened, and a test is
  never skipped, instead.

**Why threads = 1.** The determinism-sensitive tests pin themselves. Without the environment variables,
OpenMP and OpenBLAS oversubscription slowed one test file by more than 10×.

**Still open.**

- Whether this job is a required merge check is **DR-D30**.
- How its golden master is made reproducible on CI hardware was **DR-D31** (KI-NEW-Z78). It is no longer
  open: the owner ratified option 1 on 2026-10-08, and P0-01 implements it as ledger entry L0.
- The fixture-regeneration and hash-drift step arrives with the first parity package (proposed —
  DR-B2).

### 5. Legacy status; the correction-ledger policy is not decided here

- **Status.** The oracle has the status `legacy-59bce1d + L0`: the import plus ledger entry L0. `PARITY.md`
  proposes the tag `oracle-legacy-59bce1d` for the import commit. Creating the tag is an owner action.
- **Freeze rule.** While KI-NEW-Y0 stands, no legacy output is frozen as a Rust parity target for any
  quantity that a ledger correction would change.
- **The ledger policy is DR-B1**, proposed and awaiting the Statistical owner. Under it, each correction
  is made in Python as its own approved commit, with a failing test first, regenerated goldens with a
  model-spec note, and an entry in `PARITY.md`. The proposed order is:
  1. the synthetic defenders;
  2. the team-strength estimand and the Layer-3 rows;
  3. the matchup-grade sign;
  4. causal Kalman initialization;
  5. ingest bias.
- **Only entry L0 is applied.** L0 is a platform-portability entry, applied in P0-01 with the owner's
  written ratification of DR-D31 option 1 (2026-10-08) as its approval: `tests/grid/golden/snapshot.npz` regenerated once
  under CPython 3.11 and `OPENBLAS_CORETYPE=Haswell`, with only Layer C arrays changed and no generator,
  estimator, test or tolerance touched (`MANIFEST.tsv` status `patched:L0`;
  `patches/L0-golden-snapshot-haswell-regeneration.patch`). It sits ahead of the DR-B1 ledger and does
  not pre-empt it. Entries 1 to 5 are proposed and not applied; if DR-B1 is ratified, they are a separate
  work package.

### 6. Parity regime

- **The contract** is `docs/03-contracts/parity-fixture-contract.md` (draft):
  - fixtures exported single-threaded;
  - a sha256 manifest;
  - stage-isolated injection downstream of every booster;
  - synthetic-only data.

  Rust tests read the committed files and never invoke Python.
- **Live oracle and fixtures together** is DR-B2 (proposed).
- **The tolerance classes** A, A′, B, C and D are DR-B3 (proposed). Further items are raised and open:
  - **DR-D27**, a V(s) parity envelope in place of Class C;
  - **DR-D26**, seed-ensemble statistics for Class D recovery gates;
  - **DR-D28**, a draw tape that proves a Rust generator implements the oracle model;
  - **DR-D29**, the on-disk fixture format;
  - a Layer-1 criterion proposed in `docs/05-model-specs/layer1-credit.md`.
- **No Rust parity test exists in P0-01.** The first parity package (P1-01) is not Ready until DR-B1,
  DR-B2 and DR-B3 are ratified (`engine-spec.md` §8.16.1).

### 7. Typed-failure divergences (DR-B6, proposed)

Where the oracle degrades silently, the proposed engine behaviour is a typed failure. The oracle stays
unchanged, and parity is defined on the healthy path only. The paths are:

- the `lstsq` fallback on an ill-conditioned solve;
- re-initialization on corrupt state;
- skip-on-failure;
- refitting V(s) on every run;
- non-atomic saves.

They are listed in `PARITY.md` section (e).

The `lstsq` fallback was a **deliberate** earlier decision, cautious-nevermore PR #53 audit item C3. The
ADR that adopts the typed failure for ill-conditioning MUST cite PR #53 C3 as the decision it reverses.
This ADR records that requirement. It does not adopt the policy.

### 8. Licence (DR-A10): owner action required before merge

cautious-nevermore has no LICENSE file. Its visible history has these authors:

- two human author identities, both the owner's;
- three agent-authored commits;
- one `posthog[bot]` commit, which touched only `backend/api/*`, `.env.example` and `requirements.txt`.
  None of those paths is imported, and `reference/python/requirements.txt` is a new engine-only file.

The recommended default:

- **The ruling.** The Data/Licensing owner records a ruling **on the P0-01 pull request** bringing
  `reference/python/` under `MIT OR Apache-2.0`. It takes the same form as GRID-Engine PR #1 comment
  5357318508, the ruling behind ADR-001 D4.
- **Before the ruling.** The licence status is "pending", and `reference/python/README.md` says not to
  treat the tree as licensed.
- **This is merge-blocking.** Before acceptance, this section is amended to cite the ruling comment.
- **Third-party data** is outside any such ruling.

### 9. Fixture and data policy (DR-A11)

- **Parity fixtures are synthetic-only.**
- **No real third-party data under `reference/python/`.** `reference/python/.gitignore` ignores `/data/`,
  `*.parquet`, `*.sqlite` and `*.joblib`, and the root `.gitignore` ignores `/reference/python/data/`.
- **Investigation inputs are fetched, never committed.** Real inputs used by investigations are
  downloaded by `reference/python/tools/investigations/fetch_realdata.py`, which pins each URL and
  sha256, into the ignored `data/realdata/`.
- **Real-data fixtures, if ever ruled admissible**, go only under `fixtures/third-party/<provider>/`,
  with a LICENSE or NOTICE carrying the attribution and the CC-BY-SA notice.
- **`backend/db/data/coaching_changes_2025.json` (KI-NEW-D1) is unverified.** It is never a fixture or
  a provider input.
- **Oracle real-data results are historical and non-parity**
  (`docs/07-archive/cautious-nevermore/real-data-results.md`).

### 10. Lifecycle (DR-A12)

- **Frozen until.** The oracle stays a frozen CI oracle until every ported component has parity evidence
  **and** Phase 2 live evidence exists.
- **How it changes.** Only through approved ledger entries. A `requirements.lock` bump counts as an
  oracle change, because it can move the golden master.
- **Retirement.** Its future is reviewed in the optional package **P2-09**. Retirement never deletes the
  legacy tag, the committed fixtures, `MANIFEST.tsv` or the ledger.

## Consequences

**Easier.**

- **Behaviour, not prose.** Recovery gates and the golden master can be re-run. Fixtures can be
  regenerated and their hashes proved.
- **Defects can be shown.** Known defects can be demonstrated by executable investigations
  (`tools/investigations/`), not only described.
- **A port has a definite reference.** Each Rust port has one at a pinned commit, with a manifest
  proving the bytes.

**Harder.**

- **A second language to keep running.** A 105-file Python tree lives in a Rust repository, and its CI
  job adds an install plus about 150 s of tests.
- **Python or BLAS-kernel drift on the runner** can move the golden master. That escalates to the owner;
  it is not absorbed. It has happened once: KI-NEW-Z78, settled by DR-D31 (ratified 2026-10-08) and
  fixed by ledger entry L0, which pins the kernel and the interpreter. The sensitivity is controlled,
  not removed: under another kernel or Python 3.12 the Layer C tests would fail again.
- **Defects must not leak into Rust.** Every known oracle defect has to be tracked so that it is never
  ported, and every legacy number has to carry its generator label.
- **Most parity work waits.** DR-B1 to DR-B3 and DR-D26 to DR-D29 are open, so the parity packages
  (P1-01, P1-06, P1-12) are not Ready.
- **The merge waits on a licence ruling**, outside the implementer's control.
- **`.env` leaks into the oracle.** `backend/db/connection.py` calls `load_dotenv()`, which walks up to
  the repository-root `.env`. That file's `DATABASE_URL` and `SQLX_OFFLINE` therefore enter the Python
  process. This is harmless today, and documented in `reference/python/README.md`.

**Inherited.** Every package that declares oracle parity targets inherits:

- the authority placement;
- the synthetic-only fixture rule;
- the Linux-only oracle job;
- the readiness gate on DR-B1 to DR-B3.

## Compliance

| Rule | Enforced or detected by |
|---|---|
| Verbatim import | `tools/verify_manifest.py` in the `reference-oracle` job, before install. It fails on any unmanifested change |
| Isolation (no app imports, no network) | `tools/pytest_isolation_guard.py`, loaded by `pytest.ini` on every run |
| Oracle suite green | `python3 -m pytest` in the `reference-oracle` job (446 tests), on the tool-cache CPython 3.11 with `OPENBLAS_CORETYPE=Haswell`; the job fails if that interpreter is absent. Any failure is a regression |
| Pinned numerical platform | `tools/pytest_platform_pin.py`, loaded by `pytest.ini` on every run: it refuses any interpreter other than CPython 3.11 and any OpenBLAS kernel other than `Haswell` (DR-D31; ledger entry L0) |
| Outside the frozen chain | `scripts/check-verify-parity.sh` (15 steps, with no oracle step) and review of any change to `justfile`, `verify.sh` or `verify.ps1` |
| No Python in the engine | Review: `grep -rn -i 'python\|pyo3' crates/ Cargo.toml` empty. No guard asserts it yet (ADR-011 Compliance) |
| No real data committed | Both `.gitignore` files; `scripts/check-secrets.sh` and `scripts/check-traceability.sh` see every untracked file; review |
| Spelling over the oracle | `typos`, with justified allowlist entries in `_typos.toml`. `reference/python/` is never excluded |
| Licence (§8) | An owner comment on the pull request, cited here before acceptance. No script can check it |

## Alternatives considered

- **Import with `git subtree`, keeping cautious-nevermore's full history.** It would preserve authorship
  and blame. Rejected for three reasons:
  - it imports the whole repository, application included, which contradicts ADR-011 D1;
  - the local clones are shallow (oldest visible commit 2026-06-21), so the history would be incomplete
    anyway;
  - a manifest of sha256 hashes against a pinned commit proves the bytes more directly than history
    does.

  Provenance is recorded in `reference/python/README.md` and `docs/07-archive/cautious-nevermore/HISTORY.md`.
- **Run the live Python oracle inside the verify chain.** Rejected:
  - it is a D5 amendment;
  - on the merge-authoritative Windows run it would have to skip Layer C, so it would be weaker than a
    Linux job;
  - it would make every `cargo` gate depend on a Python environment, against `engine-spec.md` §1.1
    item 8;
  - Rust cannot reproduce numpy streams in any case, so the Rust contract has to be committed fixtures.
- **Commit fixtures only, with no oracle in the repository.** Rejected:
  - fixtures could never be regenerated or re-proved;
  - the correction ledger (DR-B1, if ratified) would have nothing to correct;
  - known defects could not be demonstrated;
  - a fixture whose generator is gone becomes unreviewable. This is the "Rust only, Python folded into
    documentation" option the owner declined.
- **Rename the package or restructure the oracle tree.** Rejected: it breaks byte identity with upstream,
  touches 282 import lines, and makes `MANIFEST.tsv` unverifiable against `59bce1d`.
