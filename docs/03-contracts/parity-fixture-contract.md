---
contract: grid.parity-fixture
contract-version: 1 (Draft)
status: Draft            # Draft | Approved | Superseded
authority-level: 3       # engine-spec.md §1.5; the fixtures themselves are level 5 (tests and fixtures)
semantics-owner: Statistical owner (tolerances, correction levels); Data/Licensing owner (synthetic-only rule); Security/Release owner (CI jobs)
work-packages: P1-01 (harness skeleton and fixture reader), P1-06 / P1-12 (first fixture sets), P1-05 (synthetic world fixtures)
---

# Contract — Parity fixtures (how Rust ports prove parity with `reference/python/`)

The Python engine imported under `reference/python/` (ADR-012) is an **executable reference
oracle**. Its synthetic-recovery gates and golden masters are the parity targets for the Rust
ports (engine-spec §1.7, §7.12, §7.13). This contract defines:

- how oracle outputs become committed, hash-pinned fixtures;
- how a Rust test consumes them;
- which tolerance applies to which stage;
- which oracle version a fixture represents;
- how fixtures are regenerated and checked in CI.

**Status.** No fixture exists yet. The first port work package that needs one (P1-06 or P1-12)
creates the exporter and the first cases under this contract. Most rules below implement owner
decisions that are still *proposed*:

| Topic | Decision |
|---|---|
| Correction levels | DR-B1 |
| Committed fixtures and oracle job | DR-B2 |
| Tolerance classes | DR-B3, including the proposed C-L1 amendment |
| V(s) parity criterion C-V | DR-D27 |
| Synthetic world | DR-B4 |
| Typed-failure divergences | DR-B6 |
| Synthetic-only fixtures | DR-A11 |
| Fixture format | DR-D29 |
| Oracle golden platform (interpreter and OpenBLAS kernel) | DR-D31 (ratified 2026-10-08, option 1; ledger entry L0) |

See [decision-register.md](../00-meta/decision-register.md). Until ratification these rules are
the default the first port WP proposes to the owners. They are not settled law. The exception is the
oracle platform pin, which DR-D31 settled.

## 1. What parity is, and what it is not

1. **The oracle never overrides the spec** *(DR-A2, adopted subject to ratification)*. The
   committed fixtures sit at authority level 5 (tests and fixtures). Oracle *source* sits at
   level 6 (code). When the oracle and a model spec disagree, that is a decision request. An
   accepted divergence is a statistical-owner ADR plus an entry in `reference/python/PARITY.md`.
2. **Parity is stage-isolated.** Each Rust stage is fed the *oracle's own upstream outputs*, so
   a difference in one stage cannot cascade into the next (§5). End-to-end comparison is by
   recovery statistics (Class D) only.
3. **Parity targets the healthy path.** The oracle's failure-path behaviour is not a parity
   target:
   - the `lstsq` fallback when `cond > 1e10` (`reference/python/backend/grid/layers.py:355-358`);
   - re-initializing on a corrupt state file;
   - skip-on-failure in `weekly_update`;
   - non-atomic saves.

   Rust raises typed errors there *(proposed — DR-B6)*. The ADR making ill-conditioning a typed
   failure cites cautious-nevermore PR #53 audit item C3, which added the fallback deliberately.
   Each divergence is listed in `PARITY.md`.
4. **The oracle is never edited to make a Rust test pass** (engine-spec Appendix D item 13
   and §6.8 rule 3, which carries superseded alpha-spec §6.6 rule 3). Approved corrections are
   made first in Python under the correction ledger (§7) and only then exported.

## 2. Fixture format (DR-D29)

**Recommendation:**

- **one deterministic JSON manifest** per case;
- **raw little-endian binary arrays**, one file per array;
- **UTF-8 JSON arrays** for strings;
- an **offsets + values** pair for ragged lists.

| Option | New Rust dependencies | Exact float round-trip | NaN | Strings, ragged lists | Byte-stable across library versions | Verdict |
|---|---|---|---|---|---|---|
| **Manifest JSON + raw LE binary** (recommended) | **none** to read. `std` decodes the arrays (`f64::from_le_bytes`); the manifest uses `serde_json`, already in `[workspace.dependencies]` | yes (bit-exact) | yes | strings as a JSON array; ragged lists as offsets + values | yes. NumPy `tofile` writes only data bytes | ✓ |
| `.npz` read via an npy reader | yes: an ndarray crate, an npy reader and a zip reader | yes | yes | npy unicode (`<U…`) and object dtypes have no portable Rust reader guarantee (the existing golden `snapshot.npz` stores `player_id`, `position` and `team` as `<U21`/`<U3`); ragged participation needs an offsets encoding anyway | mostly | ✗ |
| Parquet | yes: an arrow/parquet stack (large) | yes | yes | native | **no**: the writer embeds library-version metadata, so the sha256 changes on any `pyarrow` bump | ✗ |
| Plain JSON for everything | none | serde_json's default float parsing is best-effort; exact parsing needs a feature change | **no**. Strict JSON has no NaN, yet `terminal_value` is NaN on every non-terminal row | native | yes | ✗ |

Any new Rust dependency, including a dev-dependency used only by tests, needs written
justification and owner approval (superseded alpha-spec §14.2; root `CLAUDE.md`). The
recommended format needs none. The parity harness uses `serde_json` as the *first use of an
already-declared workspace dependency*, the same status as `crates/persistence`'s use of
`sqlx`. Verifying sha256 *inside* Rust would need a hashing crate as a new direct dependency.
This contract therefore puts hash verification in a shell guard (§9). An in-Rust check is an
owner-approval item.

### 2.1 Encoding rules

| Kind | File | Encoding |
|---|---|---|
| float array | `<name>.f64le` | IEEE-754 binary64, little-endian, C order, no header. The shape lives in the manifest. NaN is kept bit-for-bit as written by NumPy |
| integer array | `<name>.i64le` or `<name>.i32le` | two's complement, little-endian. A narrower width only if the manifest declares it |
| boolean array | `<name>.u8` | one byte per element, values 0 or 1 only |
| nullable non-float array | `<name>.<dtype>` + `<name>.valid.u8` | validity mask (1 = present). Never a magic value, **except** the plays-contract v0 sentinels (−1 in `n_*`), which are part of the contract and are kept verbatim |
| string array | `<name>.json` | JSON array of strings: UTF-8, ASCII-escaped, LF, trailing newline |
| ragged list (for example `off_players`) | `<name>.offsets.i64le` + `<name>.values.<dtype>` | `offsets` has length n+1, starts at 0 and is non-decreasing. Row *i* is `values[offsets[i]..offsets[i+1]]`. **Order and duplicates are preserved**: legacy synthetic participation is a multiset ([plays-contract.md](plays-contract.md) D-8) |
| scalar compared at Class A–B | `<name>.f64le` with shape `[]` or `[1]` | binary, never a JSON number |
| parameters (inputs written as decimal literals, e.g. `lam = 120.0`) | manifest `params` | JSON numbers. Never NaN or infinity |

Binary files are stored byte-exact by git. **Fixture directories MUST be marked `-text`** in
`.gitattributes`. GitHub's Windows runners default to `core.autocrlf=true`, which would rewrite
the JSON files on checkout and break their sha256. The current `.gitattributes` has no such
entry; it must be added together with the first fixture (infra owner).

## 3. Layout

```text
fixtures/parity/
  INDEX.tsv                         component, case, manifest sha256 (one row per case)
  <component>/<case>/
    manifest.json
    <input and output files listed in the manifest>
```

- **`<component>`** is the model-spec name that owns the stage:
  - `value-model`, `rapm-attribution`, `layer1-credit`, `state-space-kalman`;
  - `cross-league-priors`, `synthetic-world`, `projection-stack`, `evaluation-and-leakage`;
  - `scoring`.
- **`<case>`** is kebab-case and ends in the correction level: `-legacy` or `-corrected`
  (§7). Examples: `canonical-legacy`, `golden-focus-qb-legacy`, `design-canonical-corrected`.
- **Large shared inputs** (the canonical synthetic world) are stored **once**, under
  `synthetic-world/<case>/`. Other cases reference them through `inputs_from` with the
  referenced manifest's sha256. They are never copied. At 8-byte widths the canonical legacy
  world is about 4 MB: 16,825 plays × 15 eight-byte columns, plus participation offsets and
  values.
- A directory holds **exactly** the files its manifest lists, plus `manifest.json`.

## 4. `manifest.json`

The manifest is deterministic, so that regeneration is byte-identical:

- sorted keys, 2-space indent, LF, trailing newline, ASCII-escaped;
- **no wall-clock timestamps, no absolute paths, no host names or user names**.

Run provenance (date, runner, operator) goes into the PR evidence, not the manifest.

```json
{
  "case": "golden-focus-qb-legacy",
  "component": "state-space-kalman",
  "contracts": { "parity_fixture": "1", "plays": "grid.plays/0" },
  "description": "Focus-QB Kalman filter + RTS on the oracle's weekly Layer-1 credit (golden master inputs).",
  "environment": {
    "blas": { "architecture": "Haswell", "internal_api": "openblas", "openblas_coretype": "Haswell",
              "version": "<OpenBLAS version reported by threadpoolctl>" },
    "packages": { "joblib": "1.6.0", "numpy": "2.4.6", "pandas": "3.0.6", "pyarrow": "25.0.1",
                  "scikit-learn": "1.9.1", "scipy": "1.17.1", "threadpoolctl": "3.7.0" },
    "platform": "linux-x86_64",
    "python": "3.11.15",
    "python_implementation": "CPython",
    "threads": { "MKL_NUM_THREADS": "1", "OMP_NUM_THREADS": "1", "OPENBLAS_NUM_THREADS": "1",
                 "threadpool_limits": 1 }
  },
  "generator": { "entry": "<function>", "script": "reference/python/tools/parity/<exporter>.py",
                 "script_sha256": "<sha256>" },
  "inputs": [
    { "dtype": "f64le", "file": "y.f64le", "name": "y", "sha256": "<sha256>", "shape": [14] }
  ],
  "inputs_from": [],
  "manifest_version": 1,
  "oracle": { "corrections_applied": [], "import_patches": ["P1", "P2"], "ledger_independent": false,
              "level": "legacy", "manifest_tsv_sha256": "<sha256 of reference/python/MANIFEST.tsv>",
              "source": "cautious-nevermore", "source_commit": "59bce1d" },
  "outputs": [
    { "class": "A", "dtype": "f64le", "file": "total_filt.f64le", "name": "total_filt",
      "sha256": "<sha256>", "shape": [14] }
  ],
  "params": { "interventions": [9], "ssparams": "SSParams()" },
  "provenance": { "data": "synthetic", "profile": "synthetic-legacy", "synth_config": "CANONICAL_SYNTH" },
  "seeds": { "college": 106, "gbm": 0, "kfold": 0, "market": 1, "synth": 7 }
}
```

**Required keys:**

- `manifest_version`, `component`, `case`, `description`;
- `contracts`, `generator`, `oracle`, `environment`, `seeds`, `provenance`;
- `inputs`, `outputs`. Every output carries its tolerance `class`.

**Values:**

- `environment.packages` is read from the running interpreter. The exporter MUST refuse to run
  if any version differs from `reference/python/requirements.lock`.
- `environment.python` and `environment.python_implementation` record the interpreter the case
  was exported under (`sys.version_info`, `platform.python_implementation()`).
- `environment.blas` records the BLAS library and the **OpenBLAS kernel (core type)** in use:
  `internal_api`, `version` and `architecture` as `threadpoolctl.threadpool_info()` reports them
  after NumPy, SciPy and scikit-learn are imported (before that it reports nothing), and
  `openblas_coretype`, the value of `OPENBLAS_CORETYPE` in the environment or `null` when unset.
  Gradient-boosted oracle outputs move with the kernel and the interpreter (KI-NEW-Z78), so a case
  is only expected to reproduce under its recorded interpreter and kernel. Every oracle run pins
  CPython 3.11 and `OPENBLAS_CORETYPE=Haswell` (DR-D31, ratified 2026-10-08; correction-ledger entry
  L0, which regenerated the golden master under that pin). The exporter MUST refuse to run unless
  `openblas_coretype` and `architecture` are both `Haswell`, as `tools/pytest_platform_pin.py`
  already does for the test suite.
- The seeds of the canonical world are the oracle's own:
  - synthetic `seed=7`; college `seed+99 = 106`;
  - market `default_rng(1)`;
  - Layer-1 `KFold(random_state=0)` and GBM `random_state=0`;
  - the equivalency split `default_rng(3)`.

  Each is recorded where it applies.
- `provenance.data` MUST be `synthetic` (§8).

The oracle's own reproducibility records (`reference/python/MANIFEST.tsv`, `PARITY.md`,
`requirements.lock`) are referenced by hash. They are not duplicated.

## 5. Stage-isolated injection

Rust cannot reproduce sklearn's `HistGradientBoostingRegressor` or NumPy's PCG64 normal streams
bit for bit (final-build-spec.md inventory §5). Every stage downstream of a booster or an RNG
therefore takes the oracle's realized intermediate as an **injected input**.

| Component / stage | Injected from the oracle | Rust computes | Compared outputs | Class |
|---|---|---|---|---|
| `synthetic-world` canonical world | — (exported frames are the input) | loads v0 frames and upgrades them (`from_legacy_v0`) | round-trip of every column and id order | exact |
| `value-model` dV | per-row V(s) (`value.py:92`) and V(s′) for continuing rows (`value.py:99-102`) | `compute_dv` | `dv` | A |
| `value-model` V(s) estimator | the plays frame; the oracle V on a declared state grid, with per-cell support counts; the oracle's seed envelope (V on the grid and `dv` for seeds 1–10) | Rust V(s) (C-7 estimator) | V on grid cells with support ≥ `min_samples_leaf` (120 in the oracle, `value.py:53`); `dv` on all rows | C-V (proposed — DR-D27; `value-model.md` §10.3), not Class C (§6; KI-NEW-Z74) |
| `rapm-attribution` design | plays + players + `dv` | `build_design` | COO/CSR triplets, player and team index maps, y | exact (values in {−1, +1, +2}, [plays-contract.md](plays-contract.md) D-8) |
| `rapm-attribution` normal equations | design + `dv` | XᵀX, Xᵀy, incremental accumulation | XᵀX, Xᵀy; incremental == batch | A |
| `rapm-attribution` solve | XᵀX, Xᵀy, `lam=120`, mask (players 1.0 or `lambda_by_pos`; team intercepts 0.05; interactions 10.0), prior mean, market rows (`w_market=40`) | dense or CG ridge | β, ratings, team ratings | A′ (dense) / B (CG) |
| `rapm-attribution` situations | plays | situation masks | boolean masks | exact |
| `layer1-credit` context model | play features `x_i`, `dv` and the fold map (`layers.py:497-519`) | the Rust booster's out-of-fold residual | the residual, weekly credit, per-position season credit | C-L1 (proposed amendment to DR-B3; `layer1-credit.md` §10.3), not Class C (§6; KI-NEW-Z74) |
| `layer1-credit` weekly credit | plays + `dv` + the out-of-fold residual `_resid` (`layers.py:497-519`); fold ids for audit | per-player weekly mean residual and snaps | `credit`/`qb_credit`, `snaps`, week set | A (credit), exact (snaps, weeks) |
| `layer1-credit` fixed point `fit(n_iter=3)` | `_resid` **for each iteration** (the defender-rating feature changes per iteration) and V(s)/dV | the RAPM ↔ Layer-1 loop | ratings, team ratings, `qb_weekly` | A′/B + A |
| `state-space-kalman` | `y`, `snaps`, `played`, interventions, scheme resets, `SSParams`, and the **effective** `x0` and `P0` | filter, RTS, predictive variance | `total_filt`, `total_smooth`, `tau_smooth`, `var_total_filt`, `total_pred`, `var_total_pred`, NIS terms | A |
| `cross-league-priors` | college frame, ratings, the equivalency **permutation indices** (`priors.py:104-105`) | equivalency, priors, washout | slope, intercept, `oos_r2`, prior mean and variance, washout table | A′ (least squares), A |
| `evaluation-and-leakage` metrics | inputs; **bootstrap resample indices** | closed-form metrics; CIs on injected resamples | metric values, CIs | A |
| `scoring` | stat lines, built-in profiles | affine scoring with offset 0 | points | A |

Rules:

- **The effective `x0`/`P0` is exported explicitly.** The legacy filter initializes talent with
  `nanmean(y[:3])`, a look-ahead (KI-#15, `statespace.py:183-188`). Rust MUST NOT implement that
  default. It consumes the value the oracle actually used.
- The Tier-0.5 golden arrays (`reference/python/tests/grid/golden/snapshot.npz`, 17 arrays) stay
  a **Python-vs-Python** golden on Linux at rtol 1e-5 / atol 1e-6. The exporter re-encodes the
  relevant arrays into this format, as `rapm-attribution/golden-*` and
  `state-space-kalman/golden-*` cases with injected V(s) and residuals. Rust never reads the
  `.npz`.
- **Order is carried, not inferred.** Player columns are ordered by the string form of the id
  (`layers.py:252`). Team columns use *native* order, which is numeric for synthetic integer
  teams (`layers.py:255`). Fixtures carry the explicit index maps. Rust compares by key, and
  adopts the fixture's column order where Class A/A′ summation order matters.

## 6. Tolerance classes (proposed — DR-B3)

The values are those of critic.md §3.2 (decision B-3). Some norms are not stated there; those
choices are part of the same proposal.

| Class | Applies to | Criterion (Rust `r` vs oracle `p`, identical inputs) |
|---|---|---|
| **A** | element-wise closed forms: Kalman, RTS, fixed-lag, affine and scoring, credit means, metrics, dV subtraction, XᵀX/Xᵀy | `max_i \|r_i − p_i\| ≤ 1e-12` |
| **A′** | dense linear solves (ridge with prior mean, least squares) | `‖r − p‖∞ / ‖p‖∞ ≤ 1e-9` |
| **B** | iterative sparse CG RAPM | `‖β_r − β_p‖₂ / ‖β_p‖₂ ≤ 10 × cg_tol` **and** the solver diagnostics report `converged = true`. `cg_tol` is recorded in the case |
| **C** | booster stages: V(s) and the Layer-1 context model. The unratified DR-B3 default, measured unattainable (KI-NEW-Z74); see the stage-specific C-V and C-L1 below | `corr(dV_r, dV_p) ≥ 0.999` **and** `\|V_r − V_p\| ≤ 0.10` EP on state-grid cells with support ≥ `min_samples_leaf` |
| **D** | end-to-end recovery on the synthetic world | the recovery floors of engine-spec §7.13, **re-set on the corrected (defender-fixed) synthetic world**, calibrated below observed values the way the oracle's gates are. The floors live in engine-spec §7.13 and `reference/python/PARITY.md`, never in fixtures |

**Class C cannot be met as written (KI-NEW-Z74).** It is the DR-B3 default as proposed, and the
oracle fails it against itself: re-seeding the V(s) booster gives corr(dV) 0.9891–0.9942 on the
legacy synthetic world, 0.9896–0.9931 on the defender-fixed one and 0.9940–0.9960 on real 2023
data (`value-model.md` §7.3), and under a fold-seed change the Layer-1 context residual
correlates with itself at only 0.9936–0.9974 (`layer1-credit.md` §5.3). The model specs therefore propose stage-specific
criteria in its place. Neither is ratified:

- **C-V** for V(s) (proposed — DR-D27; `value-model.md` §10.3). All four are required:
  corr(dV_r, dV_p) ≥ 0.98 over all rows; max `|V_r − V_p|` ≤ 0.30 EP on states with support
  ≥ 120; the Rust estimator passes its own spec goldens (P-V4); and the Class D gates hold with
  Rust dV (P-V8). The `value-model` fixture carries the oracle's seed envelope (seeds 1–10) from
  which the reference values are computed.
- **C-L1** for the Layer-1 context model (proposed amendment to DR-B3; `layer1-credit.md`
  §10.3). On the canonical fixed synth, with injected `x`, `dv` and fold map, all four are
  required: residual corr ≥ 0.99; weekly credit corr ≥ 0.99 with RMS Δ ≤ 0.05 EP per play;
  per-position season-credit corr ≥ 0.995; and the P-L1-7 Class D gates.

Both sit below the oracle's own measured envelope and MUST be pre-registered by the statistical
owner before any Rust result is seen (engine-spec §7.13.4). A Rust stage that fails them is a
decision request, not a reason to loosen them. engine-spec §7.12.5 also records the two other
proposed DR-B3 amendments: the Class B refinement (`rapm-attribution.md` §10.3) and Class D over
a seed ensemble (proposed — DR-D26).

**In every class:**

- integer, boolean, id and categorical outputs are compared **exactly**;
- NaN positions must coincide;
- v0 sentinels must match exactly;
- empty-versus-missing must match.

A Class D floor MUST NOT be frozen from a legacy-world number. The legacy Tier-0 values are
measured against a generator whose defenders come from the offense (KI-NEW-Y0; critic.md G-1).
They include pooled 0.8025, team 0.6643 and focus-QB NIS 4.58. On the defender-fixed world the
golden master fails 4 of 11 tests and pooled QB NIS is 1.829.

The Python golden master's own rtol 1e-5 / atol 1e-6 is a Python-to-Python regression bound on
Linux. **It is not a Rust tolerance.**

`tests/grid/test_performance.py` asserts a wall-clock speed-up. It is not a parity target
(critic.md G-6).

## 7. Legacy versus corrected oracle (proposed — DR-B1)

1. **Two levels.** `oracle.level = "legacy"` means the as-imported oracle: `59bce1d` plus import
   patches P1 and P2 (critic.md X-4 proposes the tag `oracle-legacy-59bce1d` for that commit), and
   ledger entry L0, the golden master's platform-pin regeneration. L0 changes no semantics, so a case
   exported after it is still `legacy`; its platform is recorded in `environment`. `oracle.level = "corrected"` means the
   oracle after the statistical-owner-approved entries of the **correction ledger** in
   `reference/python/PARITY.md`, each listed in `corrections_applied`.
2. **Ledger order.** Correction #1, the synthetic defenders, comes first, before any other
   correction that depends on the synthetic world. Then:
   - the team-strength convention (DR-B5);
   - the matchup-grade sign;
   - causal Kalman initialization;
   - ingest bias.

   Each correction lands with a failing test first and a golden regeneration with a model-spec
   note.
3. **Rust targets the corrected level.** A release or promotion gate for a component touched by
   any ledger entry MUST use `-corrected` cases.
4. **Legacy cases are for audit.** They MAY back Class A/A′/B parity of a *primitive* only when
   `oracle.ledger_independent = true`: no ledger entry changes that stage's semantics for the
   injected inputs. Examples are the ridge solve on given XᵀX and Xᵀy, the Kalman filter on
   explicit `x0`/`P0`, scoring and closed-form metrics. A one-line justification goes in
   `description`. Legacy cases are never deleted or overwritten. A correction adds a
   `-corrected` sibling.
5. The diff between legacy and corrected outputs **is** the documented divergence list.

## 8. Synthetic-only rule (DR-A11, adopted subject to ratification)

- Parity fixtures are generated **solely from synthetic inputs**: the synthetic generator, or
  hand-built toy frames like the oracle's own unit tests (for example nflverse-shaped pbp rows
  for the adapter cases).
- No byte may derive from nflverse, FTN, NGS, CFBD or any other third-party dataset. That
  includes:
  - real play-by-play or participation rows;
  - real GSIS ids and real player or team names, beyond the public team abbreviations used in
    toy frames;
  - real-data plays-contract frames;
  - real RAPM outputs.
- Reason: participation is CC-BY-SA 4.0, and its ShareAlike term would require licence
  segregation in this `MIT OR Apache-2.0` tree
  ([access-and-license.md](../04-providers/nflverse/access-and-license.md)).
- Real-data fixtures, if ever needed, are **provider fixtures**, not parity fixtures. They go
  under `fixtures/third-party/<provider>/` with a LICENSE/NOTICE carrying the attribution, and
  only after a Data/Licensing ruling.
- `backend/db/data/coaching_changes_2025.json` is unverified, illustrative data (KI-NEW-D1). It
  MUST NOT be used as a fixture or a fixture input.

## 9. Regeneration procedure

Fixtures are golden files. **They are never regenerated without a reviewed semantic
explanation** (superseded alpha-spec §6.6 rule 3; root `CLAUDE.md`).

1. **Valid triggers only:**
   - an approved correction-ledger entry;
   - an approved model-spec or contract change;
   - a `requirements.lock` change, which is an oracle change, not a chore;
   - a new case.

   "Make a failing Rust test pass" is never a trigger.
2. **Environment:**
   - Linux x86_64; the exact interpreter and packages of `reference/python/requirements.lock`
     (CPython 3.11.15);
   - `OPENBLAS_CORETYPE=Haswell`, the kernel every oracle run pins and every case records in
     `environment.blas` (KI-NEW-Z78; DR-D31, ratified 2026-10-08; ledger entry L0);
   - `OMP_NUM_THREADS=OPENBLAS_NUM_THREADS=MKL_NUM_THREADS=1` in the environment, **and**
     `threadpool_limits(1)` inside the exporter. Multi-threaded GBM diverges at about 1e-2;
   - working directory `reference/python` (oracle paths are cwd-relative);
   - the isolation guard active (no network).
3. Run the exporter for the affected cases. The exporter lives under
   `reference/python/tools/parity/` and is created by the first port WP; it is not part of P0-01.
4. **Run it twice.** The second run must be byte-identical to the first. Then rewrite
   `INDEX.tsv`.
5. **The PR carries:**
   - the semantic explanation (ledger id, model-spec section, or contract version);
   - a per-array change summary (max |Δ| and the largest movers, as the golden master prints
     them);
   - statistical-owner sign-off when a booster-stage case (C-V, C-L1 or Class C), a Class D case
     or a `-corrected` target moves.
6. Windows never regenerates fixtures. Rust tests never write fixtures.

## 10. How CI checks fixtures

| Job | Check | Status |
|---|---|---|
| `guards` (Linux) | A fixture guard, for example `scripts/check-parity-fixtures.sh`, using `sha256sum` as `check-evidence-claims.sh` already does. It checks: every listed file exists and its sha256 matches; no unlisted file is present; required manifest keys exist; `provenance.data == "synthetic"`; `INDEX.tsv` agrees; the fixture tree is marked `-text` | **proposed**. A new step in `.github/workflows/alpha-ci.yml` touches the workflow security boundary (Security/Release owner) |
| `linux-smoke`, `windows-authoritative` | The Rust parity tests run inside the existing Rust test step. They read the committed fixtures, apply the §6 classes, and need no Python | no recipe change; tests are added under the existing `test-rust` step |
| `reference-oracle` (**Linux only**) | Runs the oracle suite (`python -m pytest` from `reference/python`: 446 tests) with threads pinned to 1, then **regenerates every fixture into a temporary directory and byte-compares** it with the committed tree. It uses the runner image's tool-cache CPython 3.11 and fails if that is absent, with `pip install -r requirements.txt -c requirements.lock`. `actions/setup-python` would need Security/Release owner approval. The job pins `OPENBLAS_CORETYPE=Haswell`, and the golden master was regenerated under CPython 3.11 + Haswell by ledger entry L0, so the suite is expected green on GitHub's AMD runners (KI-NEW-Z78; DR-D31, ratified 2026-10-08) | suite run: in place since P0-01; fixture regeneration: **proposed — DR-B2** |

**Rules for the `reference-oracle` job:**

- It is never wired into `verify.ps1` or `windows-authoritative`. Golden Layer C and the cache
  TTL test fail on Windows (critic.md G-6).
- If the runner's Python or BLAS kernel cannot reproduce the fixtures under the platform pin, escalate
  to the owner (KI-NEW-Z78; DR-D31). Never loosen a tolerance or regenerate to absorb the drift.
- Adding an oracle smoke step to the frozen `verify` chain is an ADR-001 D5 amendment and needs
  its own ADR. This contract does not make it.

## 11. Rust harness

- **Reader.**
  - Reads the manifest with `serde_json` and the arrays with `std` (`from_le_bytes`).
  - Checks that `byte length = product(shape) × itemsize` and that the dtype is the declared
    one.
  - Exposes typed views: `Vec<f64>`, `Vec<i64>`, `Vec<bool>`, `Vec<String>`, and a ragged view.
- **Location.** `grid-domain` may not depend on `serde_json` (it has no external dependencies).
  The reader therefore lives in test-support code: integration tests of the crate under test,
  or the future `synth` crate (DR-A8). The plays-frame loader is `from_legacy_v0`
  ([plays-contract.md](plays-contract.md) §11).
- **Assertions.**
  - A tolerance helper per class reports the failing index and both values.
  - A Class D runner computes recovery statistics against the truth bundle.
- **Typed-failure tests (DR-B6).** Each oracle failure path listed in `PARITY.md` has a Rust
  test asserting the typed error, not a fixture.

## 12. Versioning

- `contracts.parity_fixture` is this contract's version (`1`).
- `manifest_version` is the manifest schema version.
- A breaking change to the encoding or the manifest schema bumps both. Every case is then
  regenerated under §9 in the same PR.
- A change to the plays contract that fixtures carry (`contracts.plays`) is handled under
  [plays-contract.md](plays-contract.md) §12.
