# PARITY: how the Rust engine uses the Python oracle

This file says which oracle outputs the Rust port may treat as targets, which are known
defects, and where Rust deliberately differs. Read `README.md` first.

Status words used here:

- **DR-A** decisions are adopted by the consolidation PR, subject to owner ratification.
- **DR-B** and **DR-C** decisions are **proposed defaults pending the owner**
  (`docs/00-meta/decision-register.md`).
- Nothing in the correction ledger below has been applied.

## (a) Oracle status

| Field | Value |
|---|---|
| Status | **legacy-59bce1d, as imported**. Proposed tag for the import commit: `oracle-legacy-59bce1d` (critic X-4). The tag is created by whoever commits; it is not created here. |
| Code | 105 files from `cautious-nevermore@59bce1d`: 103 verbatim, plus patches P1 (import cut) and P2 (demo output dir). Neither patch changes engine behaviour (`MANIFEST.tsv`). |
| Verified | 2026-10-01, clean venv, `requirements.lock`, Python 3.11.15, Linux x86_64, threads pinned to 1. **446 passed** (node-ID set identical to the upstream in-scope run), isolation guard 0 violations, `tools/verify_manifest.py` OK. |
| Usable as a parity target today | Only outputs that no known defect touches. The defect list is `docs/00-meta/known-issues.md`; the cn-issues trust map is summarised next. |
| Not usable as a parity target until the ledger runs | Every synthetic number that depends on defenders, team strength, the matchup grade or the filtered Kalman path. That covers golden Layer B and C team ratings, `qb_credit`, `k_total_filt` and `k_total_pred`, the Tier-0 team gate, and DEF recovery. It also covers anything from `weekly_update` and every real-data number. |

**What can be a target today:**

- scoring arithmetic (`scoring/engine.calculate_points`);
- `validation/metrics.py` on non-degenerate inputs;
- the leakage-guard *properties* (`tests/validation/test_leakage_guards.py`);
- Kalman/RTS closed forms on given inputs (`kalman_two_component` with explicit `x0` and
  `P0`);
- the RAPM solve on a given design and dV.

Even these are measured on legacy-synth inputs, and they are compared stage-isolated (see
section d).

**Procedure (DR-B1, proposed):**

1. Import verbatim. This is the current state.
2. Apply approved correction commits in the order of section (b). Each needs a failing
   test first, a model-spec note, and a golden regeneration with a reviewed semantic
   explanation.
3. Rust targets the **corrected** oracle. The legacy golden is kept for audit.
4. The deliberate typed-failure divergences in section (e) stay out of parity.

The oracle is never changed *to make a Rust parity test pass* (superseded alpha-spec
Appendix D #13). Only pre-approved corrections from this ledger may change it.

## (b) Correction ledger: PROPOSED, NOT APPLIED

The order is binding: every later entry is measured on the generator that entry 1 produces
(critic B-1 / X-4). Each entry gets its own commit inside `reference/python/`, and each
updates `MANIFEST.tsv` (status `patched:<ID>`, new `dest_sha256`) and `patches/`.

### 1. Synthetic defenders: KI-NEW-Y0 (critic G-1)

- **Defect.** `backend/grid/synth.py:193`
  `off_pl, def_pl = _pick_onfield(tidx[off_team], rng)`. Every play's defenders are the
  **offense's own** DEF players: 100% of canonical plays, 0% from `def_team`.
  `_team_strength` (`:285-294`) plants off − def, which is coherent only with this bug.
- **Evidence.**
  - `tools/investigations/tier0_legacy_vs_fixed.py`: the table in section (c).
  - `python3 -m pytest -p tools.investigations.pytest_defender_fix` on the four synth gate
    files: 5 failed, 22 passed.
  - `sign_exp2.py --synth legacy`: prints `def_players from def_team: 0.0`.
- **Proposed change.**
  - Draw defenders from `def_team`: two `_pick_onfield` calls per play. This changes the RNG
    stream.
  - Define the planted **net** strength (offence quality + defence quality) and the
    defender pool's role in `docs/05-model-specs/synthetic-world.md`.
  - Re-calibrate the QB NIS bands, because the PR #66 `r_scale` 0.35→0.55 was tuned on the
    legacy world.
  - Before Layers A–F, add a "realistic" profile: QB about 99% of snaps, real RAPM scale
    (DR-B4, KI-NEW-Y2).
- **Goldens and gates affected.**
  - `tests/grid/golden/snapshot.npz` fails on 4 of 11 tests: `test_layerB_top_player_ranking`,
    `test_layerB_team_ranking_order`, `test_layerC_player_and_team_ratings` and
    `test_layerC_qb_weekly_and_kalman` (`qb_credit` moves by up to 0.317).
  - `tests/grid/test_calibration_synth.py::test_pooled_nis_bounded` fails: pooled NIS 1.829
    against the band [0.8, 1.4].
  - Every Tier-0 observed value moves, but all 8 gates still pass. The floors must be
    re-calibrated on the fixed generator (DR-B3 class D).
  - The `run_demo.py` numbers and the test-comment "observed" values move too.
- **Approver.** Statistical owner, through an ADR (DR-B4). The data owner is not needed.

### 2. Team-strength estimand and Layer-3 rows: DR-B5 (KI-G1 / KI-V1, KI-NEW-A1, KI-NEW-Y1)

- **Defect.**
  - The Layer-3 market pseudo-row anchors `β_off + β_def`. It appears at three sites:
    `layers.py:400-408`, `backtest.py:101-112` and `weekly_update.py:269-276`.
  - `team_rating = β_off − β_def` (`layers.py:426`), and the planted truth is off − def.
  - Because defenders enter the design at −1, a larger `β_def` means a better defence. A
    spread therefore prices net = off + def, so the reported team strength is the wrong
    quantity.
- **Evidence.**
  - On the fixed generator: `sign_exp2.py --synth fixed` (realized margin vs net 0.855;
    `γ_off+γ_def` vs net 0.542 with a net-anchored `[+1,+1]` market, against 0.412 for the
    current `team_rating`). `sign_check.py --synth fixed` (gauge-invariant E_off+E_def vs
    planted 0.96).
  - On legacy, recorded as history: `sign_exp.py`, `sign_check.py`, `sign_test.py` and
    `sign_test2.py`. Their `[+1,−1]` gains are moot (critic X-2, X-19).
- **Proposed change.**
  - Report **net strength** from gauge-invariant aggregates: E_off = γ_off plus
    exposure-weighted on-field offensive ratings, and E_def likewise.
  - Keep the `[+1,+1]` rows, anchored to the spread-implied net strength **using the line at
    lock**.
  - **Reject `[+1,−1]`.**
  - Add truth-anchored Layer-A tests on the fixed generator.
- **Goldens and gates affected.**
  - Golden `team_rating` (Layer C) and team ranking (Layer B).
  - The Tier-0 `team_corr ≥ 0.60` gate (`tests/grid/test_tier0_recovery.py:99`) must be
    redefined against planted net strength.
  - `tests/validation/test_backtest.py` market-anchor expectations.
- **Approver.** Statistical owner (DR-B5).

### 3. Matchup-grade sign: KI-NEW-A2

- **Defect.** The grade is stored as `−β[t_def]` with the label "higher = tougher".
  - Sites: `weekly_update.py:84`, its docstring at `:62-66`, `backtest.py:185-186` and its
    docstring at `:48-51`.
  - A stronger defence has a larger `β_def`, so the stored grade is an "easiness" score.
  - The tests at `tests/pipeline/test_weekly_update.py:286-370` and
    `tests/validation/test_verdict.py:47-51` are tautological: they hand-plant the wrong
    sign.
- **Evidence.**
  - `defsign_planted.py`, on its own generator, so it is valid regardless of entry 1. ELITE
    `β_def = +0.409` gives shipped grade −0.409, and WEAK `β_def = −0.382` gives +0.382.
  - `def_sign.py --synth fixed`: corr(β_def, planted DEF quality) +0.53, and corr(stored
    grade, points allowed) +0.59.
- **Proposed change.**
  - Grade = **+E_def** (or `+β_def` while the grade stays intercept-only). Higher means
    tougher.
  - Rewrite the tautological tests as truth-anchored tests on the fixed generator.
  - The Tier-2 KPI must then be re-measured: `defgrade_test*.py`, KI-NEW-V2.
- **Goldens and gates affected.**
  - The golden snapshot holds no grade arrays, so it is unaffected.
  - The tests named above, and the Tier-2 synth test.
- **Approver.** Statistical owner (DR-B5; listed in the critic §3.4 as an obvious default).

### 4. Causal Kalman initialisation: KI-#15 (GitHub issue #15; reconcile-code-first C16)

- **Defect.**
  - `kalman_two_component` initialises talent at `nanmean(y[:3])` (`statespace.py:183-188`),
    so the week-1 *filtered* estimate already uses weeks 2–3. That is look-ahead.
  - `KalmanState.init` starts at zeros (`statespace.py:46-51`), so the batch and
    incremental filters differ from week 1. `kparity.py` holds `x0`/`P0` equal on purpose,
    to isolate KI-NEW-S1.
  - Priors are not wired into either path (KI-NEW-P2).
- **Evidence.** Read the code at the lines above. `tests/grid/golden_master.py:88-111`
  freezes the affected `k_total_filt` and `k_total_pred`.
- **Proposed change.** x0 and P0 come from the prior (P0 = σ²/n0) or a diffuse init, with no
  look-ahead. This is part of the DR-C10 bundle: weekly credit as the observation, R from
  real exposures, one filter core with a persisted `games_since_event`, and state keyed by
  (season, week).
- **Goldens and gates affected.**
  - Golden `test_layerC_qb_weekly_and_kalman`: `k_total_filt`, `k_total_pred` and
    `k_var_total_pred`.
  - Tier-0 NIS.
  - `test_calibration_synth.py`.
  - The smoothed (RTS) outputs are retrospective, so their use in the verdict is benign, but
    their values will still move.
- **Approver.** Statistical owner (DR-C10).

### 5. Ingest bias: KI-NEW-I1..I5 (and KI-NEW-V0a)

- **Defect.**
  - `pass_attempts` counts sacks, and `passing_yards` nets sack yards (`data_pipeline.py:50-62`,
    `nflverse_loader.py:66`).
  - `touchdown` credits return TDs (`nflverse_loader.py:73-77`).
  - Fumbles are attributed to the rusher or receiver (`data_pipeline.py:101-105`).
  - There is no `season_type` filter, so postseason is summed in (`nflverse_loader.py:53-54`,
    `nflverse_adapter.py:91-92`).
  - Two-point conversions are never populated, and kneels are excluded (KI-NEW-I5).
- **Evidence.** `cmp_stats.py` against the pinned 2023 inputs:
  - pass attempts +7.3%, passing yards −7.4%;
  - passing TDs 814 vs 754;
  - fumbles lost −32%;
  - 1,638 postseason rows.

  `real_contract.py` shows postseason plays in the plays contract.
- **Proposed change.** Labels come from official nflverse weekly player stats with a
  versioned correction window. Training labels use REG weeks only. Use `pass_touchdown` /
  `rush_touchdown` with `td_player_id`, and `fumbled_1_player_id` with `fumble_lost`.
  Two-point conversions are modelled, and there is an explicit season-type policy (DR-C12).
- **Goldens and gates affected.**
  - No synthetic golden.
  - `tests/pipeline/test_data_pipeline.py` and `tests/grid/test_nflverse_loader.py`
    expectations.
  - All real-data outputs, which are already non-parity.
- **Approver.** Statistical owner and data owner (DR-C12 is [S][D]).

## (c) Tier-0 values: legacy vs defender-fixed generator

These were produced by `python3 -m tools.investigations.tier0_legacy_vs_fixed` on
2026-10-01 with threads pinned to 1. They are identical to the critic's G-1 table. Both
columns use the canonical `load_synthetic()`, `fit(n_iter=3)`, market seed 1, and the
**legacy** planted `team_strength` (off − def).

| Tier-0 value | Gate | Legacy (as imported) | Defender-fixed |
|---|---|---|---|
| defenders from `off_team` / `def_team` (share of plays) | (none) | 1.000 / 0.000 | 0.000 / 1.000 |
| pooled attribution corr | ≥ 0.77 | 0.8025 | 0.8276 |
| QB / RB / WR / TE / DEF | ≥ .83 / .70 / .76 / .73 / .73 | .8690 / .7433 / .7973 / .7713 / .7653 | .8845 / .7711 / .8599 / .7784 / .7827 |
| team corr vs the **legacy** off − def target (semantically wrong on the fixed generator) | ≥ 0.60 | 0.6643 | 0.6596 |
| Kalman total_smooth / tau_smooth corr | ≥ 0.92 / ≥ 0.60 | 0.9580 / 0.6745 | 0.9760 / 0.6538 |
| focus-QB predictive NIS | ≤ 10 | 4.581 | **8.371** |
| equivalency slope / feeder→NFL OOS R² | [0.9, 1.8] / ≥ 0.05 | 1.3159 / 0.1489 | 1.2119 / 0.2134 |
| rookie prior corr | ≥ 0.50 | 0.5829 | 0.5829 |

On the fixed generator Tier-0 still passes 8/8. The golden master fails 4/11 and synth
calibration fails 1/6. Focus-QB NIS on the fixed world sits close to its ≤ 10 guard
(KI-NEW-S2), so the guard is too loose to be a meaningful parity number. None of the
legacy values may be frozen as a Rust target.

## (d) Tolerance classes: PROPOSED (DR-B3)

The normative home is `docs/03-contracts/parity-fixture-contract.md`. It defines the fixture
format, the sha256 manifest and the export rules (single-threaded, Linux), per DR-B2:
committed stage fixtures are the Rust contract, and the Linux oracle job proves they
regenerate. The proposed default classes are:

| Class | Applies to | Tolerance |
|---|---|---|
| A | Element-wise closed forms: Kalman / RTS / fixed-lag, affine maps, metrics | ≤ 1e-12 absolute |
| A′ | Dense linear solves | ≤ 1e-9 relative |
| B | Conjugate-gradient RAPM | ≤ 10 × CG tolerance, and `converged = true` |
| C | Booster stages (V(s), Layer-1 context model) | corr(dV) ≥ 0.999 and \|ΔV\| ≤ 0.10 EP on grid cells with support ≥ `min_samples_leaf` |
| D | End-to-end recovery | Floors re-set on the **fixed** generator, calibrated below observed (as the CN gates are) |

Comparisons are **stage-isolated**. Python-produced dV is fed into Rust RAPM, and
Python-produced weekly credit into the Rust Kalman filter, so Rust never has to bit-match
sklearn's HistGradientBoosting.

For reference, the oracle's own golden master checks Layer C at `rtol 1e-5, atol 1e-6`
single-threaded on Linux. That tolerance is an oracle self-consistency check, not a Rust
tolerance. It is also why `run_demo.py`, which does not pin threads, prints slightly
different state-space numbers at different thread counts (`README.md`).

## (e) Deliberate Rust divergences: typed failures (DR-B6, PROPOSED)

The proposed default: Rust turns each oracle failure path below into an explicit typed
failure, and the oracle is left unchanged. These paths are **excluded from parity**. Parity
is defined on the healthy path only, and Rust tests assert the typed error instead.

| # | Oracle behaviour (file:line at 59bce1d) | Rust (proposed) | Note |
|---|---|---|---|
| 1 | **`lstsq` fallback.** `layers._solve_ridge_prior` computes `np.linalg.cond(A)` on every solve and switches to `np.linalg.lstsq` when cond > 1e10, with only a log warning (`layers.py:355-358`). | Cholesky / CG with a typed singular or ill-conditioned error. | **Reverses a recorded decision.** The fallback was added deliberately by PR #53, audit item C3 ("Conditioning check + `lstsq` fallback in ridge solver"). The ADR that adopts the typed failure must cite PR #53 C3 (critic G-3). The cond() SVD is also O(n³) per solve (KI-NEW-A6). |
| 2 | **Corrupt-state reinit.** `KalmanState.load` returns `None` for any state width other than 2 or 3 (`statespace.py:116-118`), and the caller starts fresh. `weekly_update` reinitialises the accumulators on a dimension mismatch or player reorder (`weekly_update.py:245-252`), which is KI-NEW-W2. | A typed state-schema or version error, plus an explicit, logged migration or rebuild command. | PR #56 described "corrupt → reinit" as deliberate design (critic G-3). A 1-D `mu` on disk raises IndexError (KI-A8) instead of returning `None`. |
| 3 | **Skip-on-failure.** `weekly_update.run`: <ul><li>fetch errors become a health warning and a skipped week (`:215-220`);</li><li>an already-processed week is silently skipped (`:238-244`, KI-NEW-W1);</li><li>`kalman_trajectory` write failures are skipped (`:411`);</li><li>a `coaching_changes` query error becomes "no resets" (`:330`).</li></ul> Elsewhere, `layer1_all_qbs` swallows per-QB fit errors with a bare `except Exception: pass` (`layers.py:555-556`, KI-G6), and health is reported "ok" unconditionally (KI-NEW-W5). | Typed errors with a non-zero exit, and evidence records. | Never convert malformed data or a failed stage into a healthy default (superseded alpha-spec Appendix D). |
| 4 | **V(s) refit every run.** `weekly_update.run` refits V(s) on the whole season on every call (`weekly_update.py:213`). In backfill that also leaks future weeks into dV (KI-NEW-W4). | A versioned, as-of V(s) artifact, refit on a declared schedule (DR-C6, DR-C7). | Parity for V(s) uses class C on a pinned training window, not a per-run refit. |
| 5 | **Non-atomic saves.** <ul><li>`ParquetCache.put` writes with `to_parquet` in place (`cache.py:25-28`, KI-G9);</li><li>accumulators and Kalman state use `np.savez` in place (`layers.py:117`, `statespace.py:61`);</li><li>`ingest_grid` writes its snapshots and then the manifest last (`ingest_grid.py:63-81`, KI-V10);</li><li>`health_check.write_health` writes in place (`health_check.py:40`).</li></ul> | Temp file + rename, content hashes, and a SQLite-registered manifest (DR-C15). | A crash mid-write leaves a "fresh" partial file in the oracle. |
| 6 | **Accumulators saved before the solve.** `weekly_update.run` saves the RAPM accumulators (`weekly_update.py:259`, `np.savez` in place via `layers.py:117`) before the solve (`:278`) and the Kalman step (`:389-398`). A failure after the save leaves the week marked done, and the watermark skips the re-run (`:240-244`). KI-NEW-Z14. | Stage outputs commit atomically after the run succeeds, with a run record (DR-B6, DR-C15). | Same family as rows 3 and 5; excluded from parity like them. |

## (f) Non-gating checks

- **`tests/grid/test_performance.py::TestBuildDesignPerformance::test_vectorized_faster_than_reference`**
  (`:173-200`) asserts a wall-clock speedup of ≥ 1.5× of the vectorized `build_design` over
  a row loop.
  - It measures this machine's timing, not engine semantics, and is flake-prone under CI
    CPU contention.
  - It passed in every run here with threads pinned to 1.
  - If it flakes in the oracle CI job, treat it as a performance signal, never as a parity
    failure. Do not weaken it inside the verbatim tree. Record the flake against this entry
    and raise a decision request.
- **`run_demo.py`** is documentation. Its numbers are asserted by the Tier-0 gates, and its
  state-space lines depend on the thread count (`README.md`).
- **Windows.** Golden Layer C and `test_cache.py::test_ttl_expired` are recorded as failing
  on Windows. That is a platform limitation, not a parity signal. The oracle job is
  Linux-only (DR-A3, critic G-6).

## (g) Oracle lifecycle (DR-A12)

- **Frozen CI oracle.** It is kept until **every ported component has parity evidence** and
  **Phase-2 live evidence exists**. Retirement is reviewed at work package **P2-09**.
- **Changes only through section (b).** One reviewed commit per ledger entry, in order,
  each with its approver. `requirements.lock` bumps count as oracle changes, because they
  can move the golden (rtol 1e-5).
- **Hygiene.** No other edits to `backend/` or `tests/`. `tools/verify_manifest.py` fails
  on any unmanifested change.
- **Retirement.** On retirement the legacy and corrected goldens, `MANIFEST.tsv` and this
  file stay in history. Deleting the tree is an owner decision recorded in the decision
  register.
