# Lessons learned

These are engine-relevant lessons from two sources: building the Python GRID engine (cautious-nevermore,
CN, now the reference oracle under `reference/python/`), and consolidating it into this repository (WP
P0-01). Each lesson is written so that it changes what we do. The rule says what is required from now
on, and "Enforced in" says where that requirement lives or will live.

- **Not an authority.** This file explains *why* rules exist. The rules bind through the documents named
  under "Enforced in": `engine-spec.md`, model specs, contracts, tests, guards and templates.
- **Sources.**
  - CN `docs/04-lessons-learned.md`, engine items only. App items (frontend, JSON nesting in
    `sync_trade_history`, Sleeper scoring, format-registry upsert) are excluded.
  - CN `docs/08-phase-task-context.md` "Known Traps".
  - The consolidation inventory, `docs/06-sessions/2026-10-01-consolidation-inventory/`: `cn-docs.md` §8
    and §13–§14, `cn-issues.md`, and `critic.md` §1–§2.
  - The consolidation itself, including the model specs and contracts written in it (section E).
- **CN's own lessons file contains two wrong lessons.**
  - "Market reconciliation sign error (G1 + V1)" prescribes the `[+1,−1]` change, which the consolidation
    rejected (critic X-2; KI-G1).
  - "fumbles_lost counts ALL fumbles" describes a defect that did not exist (KI-A1).

  Both are corrected below, in LL-16, LL-21 and LL-22. Do not port CN's text.
- **"Enforced in" vocabulary.**
  - **in place**: exists in this repository now.
  - **P0-01**: delivered by the consolidation PR.
  - **planned (\<WP\>)**: a requirement whose enforcement a later WP must build.

  The specific paths named here are listed in `engine-spec.md` §8.19 and §12.

---

## A. Validation and the synthetic oracle

### LL-01 Validate the synthetic generator before trusting recovery numbers

- **What happened.**
  - The CN synthetic generator drew every play's "defenders" from the **offense's** own roster:
    `reference/python/backend/grid/synth.py:193`. On the canonical 16,825-play dataset, 100% of defenders
    belong to `off_team` and 0% to `def_team`.
  - The planted team strength (off − def) was coherent only with that bug.
  - Built on it:
    - CN's queued `[+1,−1]` market-row "fix";
    - three inventory reports' sign conclusions;
    - the QB `r_scale` calibration (CN PR #66);
    - every team, DEF and matchup parity number.
  - Fixing it fails 4 of 11 golden tests and 1 of 6 calibration tests, and moves focus-QB NIS from 4.58 to
    8.37.
  - Only one review, which checked what the generator does rather than what the estimators recover, found
    it.
- **Evidence.**
  - KI-NEW-Y0, KI-NEW-Y1, KI-NEW-S2;
  - critic G-1 and X-1/X-2;
  - `reference/python/tools/investigations/tier0_legacy_vs_fixed.py`, `pytest_defender_fix.py`,
    `sign_exp.py`, `sign_exp2.py`.
- **Rule.**
  - The synthetic generator is code under test.
  - Every planted quantity has a generator-semantics test before any recovery number derived from it is
    cited, frozen or used for calibration. Examples:
    - defenders belong to `def_team`;
    - realised scoring margin rises with planted net strength;
    - planted effects have the documented sign.
  - Every generator change gets a new generator version, and every number is labelled with that version
    (LL-05).
- **Enforced in.**
  - `engine-spec.md` §6.9 (synthetic-world contract) and §7.13 (recovery gates);
  - `docs/05-model-specs/synthetic-world.md`;
  - correction-ledger position 1 in `reference/python/PARITY.md` (DR-B1, DR-B4);
  - generator-semantics tests in the Rust `synth` crate: planned (P1-12; crate per DR-A8).

### LL-02 Tests that derive the expected value from the estimate cannot catch sign errors

- **What happened.** The matchup grade `−β_def` is inverted (KI-NEW-A2), yet both tests that cover it
  pass.
  - `reference/python/tests/pipeline/test_weekly_update.py:286-370` plants `β_ELITE = −3` by hand. That
    encodes the same wrong algebra as the code.
  - The Tier-2 synth test builds `points_allowed = 20 − 30·grade` from the grades under test
    (`reference/python/tests/validation/test_verdict.py:47-51`).
- **Evidence.**
  - KI-NEW-A2; critic X-3;
  - `reference/python/tools/investigations/defsign_planted.py` (an independent generator: the elite
    defence gets the lowest grade).
- **Rule.**
  - A test's expected value comes from planted truth or from an independent computation. It never comes
    from the estimate under test, or from hand-set parameters that restate the implementation's own
    assumption.
  - Every sign or orientation convention has a truth-anchored test.
- **Enforced in.**
  - `engine-spec.md` §12 (testing) and §7.13 (golden Layer A truth-anchored invariants);
  - the "Explanation fields: sign conventions" section of `docs/99-templates/template-model-spec.md` (in
    place);
  - the rewritten grade tests: planned (correction-ledger position 3, DR-B1; P1-12).

### LL-03 Tests that assert only a property do not catch wrong values

- **What happened.**
  - `var_total = σ00 + σ11` omitted the covariances and the third state component, in three places.
  - The tests asserted only `var > 0`.
  - The correct value is `H Σ Hᵀ` with `H = [1,1,1]`. It was fixed in CN PR #63 (KI-G5).
- **Evidence.** CN `docs/04-lessons-learned.md` ("Variance of sum missing covariance term", "Tests that
  only assert > 0"); KI-G5.
- **Rule.** Numerical tests assert the expected value to a stated tolerance. A sign, positivity or
  finiteness check is a supplement, never the whole test.
- **Enforced in.**
  - `engine-spec.md` §12 (golden numerical tests);
  - the "Reference examples" and "Tolerances" sections of `template-model-spec.md` (in place).

### LL-04 Goldens are platform-specific: declare a platform of record

- **What happened.**
  - CN's next-steps plan (`docs/10-next-steps-plan.md:73-75`) recorded golden-master Layer C ×2 and
    `test_cache.py::test_ttl_expired` as "pre-existing failures", and blamed drift from CN PR #80.
  - Both pass on Linux, the declared golden platform of record.
    - Layer C differs on Windows because of cross-platform float differences in gradient boosting.
    - The TTL test fails because of Windows mtime granularity.
  - The golden snapshot was last written in CN PR #65 and still matches at 1e-5 on Linux.
- **Evidence.** cn-issues §1; cn-docs §0.6; critic G-6.
- **Rule.**
  - Every golden declares its platform of record.
  - A golden failure is triaged by platform before anyone attributes it to a code change.
  - Rust goldens state per-stage cross-platform tolerances.
  - The Python oracle runs on Linux only.
- **Enforced in.**
  - `engine-spec.md` §7.12 and §8.19;
  - the Linux-only oracle CI job (DR-A3, DR-B2): planned;
  - `reference/python/PARITY.md` (P0-01).

### LL-05 Label every parity number with the version of the generator and oracle that produced it

- **What happened.**
  - Recovery, golden and calibration numbers were copied between reports with no generator version.
  - After the defender fix, the same named metric has different values. QB attribution moves from .869 to
    .8845. Focus-QB NIS moves from 4.581 to 8.371.
  - Four reports' parity tables turned out to be legacy-generator values.
- **Evidence.** Critic G-1(a) (the legacy vs fixed table, reproduced in `known-issues.md` §5); critic
  X-19.
- **Rule.**
  - Every recorded parity or recovery number carries the oracle commit, the generator version (legacy or
    fixed), the seed, the thread count and the platform.
  - A number without these labels is not citable.
  - Legacy-generator numbers are audit-only, never Rust targets.
- **Enforced in.**
  - `docs/03-contracts/parity-fixture-contract.md` (manifest fields);
  - `reference/python/PARITY.md` and `reference/python/MANIFEST.tsv` (P0-01);
  - `engine-spec.md` §7.12.

### LL-06 Aspirational targets are not gates

- **What happened.**
  - CN `docs/02-backend-spec.md:273-282` listed Tier-0 targets (pooled ≥ 0.85, team ≥ 0.90, NIS in
    [0.8, 1.25]) as if they were the gates.
  - The implemented gates are floors calibrated below the observed values: 0.77, 0.60 and NIS ≤ 10. The
    observed values were 0.8025, 0.6643 and 4.58 (`reference/python/tests/grid/test_tier0_recovery.py:84-143`).
- **Evidence.** cn-docs §0.2 and §3.6; KI-NEW-S2.
- **Rule.**
  - A spec states a gate only as an implemented threshold, together with its test and its last observed
    value.
  - Aspirational numbers are labelled "stretch" and are never presented as evidence.
- **Enforced in.**
  - `engine-spec.md` §3.3 (current evidence status) and §7.13 (each gate names its test);
  - `engine-spec.md` Appendix F (open decisions).

### LL-07 Pre-register gates; calibrate only on data the evaluation does not use

- **What happened.**
  - CN's calibrate-then-gate procedure set each gate from the first full backtest pass (a sign-flip null
    plus 1.96·SE).
  - Its registry ships empty by contract, while the `thresholds.py` docstring says the gates are
    committed.
  - The per-run gates lived only in gitignored reports.
- **Evidence.** cn-docs §3.9 and §14 item 8; critic X-16 and G-7.
- **Rule.**
  - Gates are versioned before the results they judge are seen.
  - Calibration is allowed only on a period disjoint from evaluation.
  - Frozen gates are committed.
- **Enforced in.**
  - `engine-spec.md` §7.4 and §9.4 (DR-C5, proposed);
  - the "Validation and promotion" section of `template-model-spec.md`: "thresholds are versioned before
    results are seen" (in place).

### LL-08 Check identifiability before interpreting a coefficient

- **What happened.** Three CN quantities were read as estimands without an identifiability check:
  - **Team intercepts are gauge-dependent.** Each team has an exact null direction: its players +a, its
    intercept −n·a. Raw intercepts recovered planted strength at 0.46–0.74, against 0.82–0.92 for
    gauge-invariant aggregates (legacy synth).
  - **The starting QB is nearly collinear with the team-offense intercept** on real data (KI-NEW-A3).
  - **Talent and scheme_fit are weakly identified** under additive `H = [1,1,1]` (CN Phase-4 plan, Known
    Constraints).
  - **Layer-1 "player credit" is a unit quantity** (found 2026-10). Every on-field offensive player gets
    the same play residual. RB/WR/TE credit tracks team starting-offense ability at 0.96–0.97 on the
    fixed synth, yet the verdict reads it as individual talent (KI-NEW-Z1).
- **Evidence.** CF C12 (`sign_check.py`); KI-NEW-A3 (`real_rapm.py`); cn-docs §8 item 14;
  `layer1-credit.md` §7.4 and §7.7.
- **Rule.**
  - Report only identified or gauge-invariant quantities.
  - Each model spec states which parameters are identified, and under what exposure.
- **Enforced in.**
  - the "Known limitations" and "Constraints" sections of `template-model-spec.md` (in place);
  - `docs/05-model-specs/rapm-attribution.md` and `state-space-kalman.md`;
  - DR-B5 (proposed).

## B. Numerics and determinism

### LL-09 Pin threads to one: oversubscription and reduction order

- **What happened.**
  - Multi-threaded runs move the golden by about 1e-2, so it passes only under `threadpool_limits(1)`
    (`reference/python/tests/grid/golden_master.py`).
  - The golden's comment blames V(s). The 2026-10 measurement found V(s) thread-invariant (0.0 difference
    at 1 vs 4 threads). The drift appears downstream: ratings 9.6e-4, `qb_credit` 0.021, `k_total_filt`
    0.0185 (KI-NEW-Z75).
  - OpenMP oversubscription slowed the oracle suite more than tenfold. One report recorded "tens of
    minutes" for a suite that runs in 144–149 s single-threaded.
- **Evidence.** CN `docs/08-phase-task-context.md` "Thread determinism"; PC §3; critic G-6 and X-9;
  `value-model.md` §5.2.
- **Rule.**
  - Every oracle run and fixture export sets `OMP_NUM_THREADS`, `OPENBLAS_NUM_THREADS` and
    `MKL_NUM_THREADS` to 1.
  - Rust parallel reductions use a fixed order, so the thread count never changes a published number.
- **Enforced in.**
  - `reference/python/README.md` and the oracle CI job environment (DR-B2);
  - the golden's own `threadpool_limits(1)` (in place);
  - the "Seed policy" section of `template-model-spec.md` (in place);
  - `engine-spec.md` §7.12.

### LL-10 Wall-clock assertions are benchmarks, not unit tests

- **What happened.** `reference/python/tests/grid/test_performance.py:187-198` asserts that a vectorised
  path is at least 1.5× faster. It is flake-prone under CI CPU contention.
- **Evidence.** Critic G-6.
- **Rule.**
  - Timing claims live in benchmarks with a declared reference machine.
  - A wall-clock check in a test suite runs single-threaded and is marked non-gating.
- **Enforced in.** `reference/python/PARITY.md` (non-gating list, P0-01); `engine-spec.md` §8.19.

### LL-11 Make "missing" explicit in types; never let NaN, 0 and absent blur together

- **What happened.** Each of these went wrong in CN:
  - `dict.get(k, 0.0)` returns NaN for a present-NaN key, which crashed `Ridge.predict` (KI-P1).
  - `max(0.0, NaN)` returns 0.0 (KI-P2).
  - Float truthiness appears in `x or 0.0` (KI-G7).
  - `prior_mean = 0.0` conflates "no prior" with "prior of 0" (KI-P1 residual).
  - Seeding absent weeks with 0.0 instead of `played=False` and `y=NaN` biased a leading-absence RB's
    smoothed talent from 1.012 to 0.959 (CN PR #91 round 2).
- **Evidence.** CN `docs/04-lessons-learned.md` (three NaN lessons); cn-docs §8 items 4–5.
- **Rule.**
  - Absence is a typed `Option` or an explicit missing-indicator feature, never a sentinel number.
  - The Kalman filter's weekly input is a typed "not played" for missing weeks. The filter never reads `y`
    for them.
  - Malformed critical data is a typed failure (DR-B6).
- **Enforced in.**
  - the "Inputs: null semantics" section of `template-model-spec.md` (in place);
  - `docs/05-model-specs/state-space-kalman.md`;
  - `engine-spec.md` §6.4 (typed numerical failures).

### LL-12 Keep filtered, smoothed and predictive quantities distinct

- **What happened.**
  - Using the stored *filtered* talent where *RTS-smoothed* talent was meant is the canonical
    "correct-looking wrong semantic" (CN PR #78).
  - The filtered variance is circular and omits R. Only the one-step predictive S is an honest forecast
    variance (CN PR #63).
  - The ROS verdict was first scored by the weekly SV → points map instead of the season stat-line model.
    That produced the −0.864 artifact.
  - Found again in 2026-10: the verdict's `smoothed_talent` equals the filtered endpoint exactly, by the
    RTS boundary condition (KI-NEW-Z51). Its `credit` column holds RAPM ratings (KI-NEW-Z59).
- **Evidence.** cn-docs §2.5, §3.5 and §8 items 6–7; CN `docs/04-lessons-learned.md` ("The ROS verdict
  was scored by the wrong projection").
- **Rule.**
  - Filtered, smoothed and predictive estimates are distinct types.
  - Each output horizon names the model that produced it, in the output contract.
  - Smoothed covariances never feed forecast calibration.
- **Enforced in.**
  - `engine-spec.md` §5.5 (engine output contract);
  - `docs/03-contracts/engine-output-contract.md`;
  - `docs/05-model-specs/state-space-kalman.md` and `projection-stack.md`.

### LL-13 Silent fallbacks and catch-all handlers hide failures

- **What happened.** CN had four silent-degradation paths:
  - `weekly_update` "looked like it ran" because a broad `try/except` swallowed the `KeyError` raised when
    the plays lacked participation (cn-docs §8 item 1).
  - `layer1_all_qbs` drops QBs with `except Exception: pass` (KI-G6).
  - Health is reported "ok" even when every year failed (KI-NEW-W5).
  - The ridge solve falls back to `lstsq` with only a log line (KI-NEW-A6).
- **Evidence.** The KIs above; final-build-spec §5 list (FB report).
- **Rule.**
  - No catch-all handlers on critical paths.
  - Every run writes a record with per-stage status and exits non-zero on failure.
  - Numerical degeneracy is a typed failure with diagnostics.
  - Any deliberate fallback is documented in a model spec and an ADR (see LL-24).
- **Enforced in.**
  - `engine-spec.md` §8.6 (daily pipeline run records) and §6.4;
  - DR-B6 (proposed);
  - the CLAUDE.md prohibited shortcuts ("converting malformed data to a healthy default"; in place).

## C. Data, time and leakage

### LL-14 Key all state and every guard by (season, week), and test the season seam

- **What happened.** Two CN mechanisms broke at the season boundary:
  - **The walk-forward watermark guard.** It fires only when `last_slot[0] == s`
    (`reference/python/backend/validation/backtest.py:178-181`), so it switches off at a season seam,
    exactly where ordering is most fragile (KI-V2).
  - **The `weekly_update` accumulators.** They are keyed by week only, so week 1 of the next season is
    skipped forever (KI-NEW-W1, reproduced by `rollover_test.py`).
- **Evidence.** CN `docs/04-lessons-learned.md` ("Watermark guard skips season boundaries"); KI-V2;
  KI-NEW-W1.
- **Rule.**
  - Every time-keyed state, cutoff and leakage guard uses (season, week) tuples.
  - Each guard has a test placed at a season seam, plus a deliberate-leak canary that the guard must catch.
- **Enforced in.**
  - `engine-spec.md` §4.5 and §12 (leakage tests);
  - `docs/05-model-specs/evaluation-and-leakage.md`;
  - the leakage harness: planned (P1-05).

### LL-15 Free data has a publication lag; filter by information time, not week index

- **What happened.**
  - nflverse participation is published once, after the postseason
    (`reference/python/backend/grid/nflverse_loader.py:175-178`; nflreadr docs, critic G-5).
  - CN's walk-forward and `weekly_update` nevertheless used same-season participation at every origin.
  - The headline H2 result (+0.848) was measured that way. It is therefore not live evidence.
- **Evidence.** AS §0 finding 5; CF C1; SF R14; DR-C1.
- **Rule.**
  - Every source carries an information timestamp (published or available at).
  - AsOf filters by information time against the forecast's lock, not by week index.
  - An evaluation that uses data not published by lock is labelled research-only.
- **Enforced in.**
  - `engine-spec.md` §4.5 (as-of and publication lag) and §6.3 (live vs offseason);
  - DR-C1 (proposed);
  - the publication-lag axis: planned (P1-05).

### LL-16 Take labels from the official source; reconstructions are biased

- **What happened.**
  - CN rebuilt weekly box scores from play-by-play, and the result was biased on 2023 data:
    - attempts +7.3% (sacks counted);
    - pass yards −7.4%;
    - pass TDs +8% (return TDs credited);
    - fumbles lost −32% (wrong attribution);
    - postseason weeks summed in.
  - CN's own lesson on fumbles ("counts ALL fumbles") was wrong: the column was `fumble_lost`, and the
    real defect was attribution.
- **Evidence.** KI-NEW-I1 to I5, KI-NEW-V0a, KI-A1; `cmp_stats.py`, `real_contract.py`; critic X-14.
- **Rule.**
  - Training labels are official nflverse weekly player stats, REG weeks only, under a versioned
    correction window.
  - PBP aggregates are features or cross-checks only.
  - Any reconstruction is reconciled against the official table, with a tolerance test.
  - Column semantics are verified against the data dictionary **and** the code.
- **Enforced in.**
  - `engine-spec.md` §4.7 (training labels);
  - `docs/04-providers/nflverse/README.md`;
  - DR-C12 (proposed);
  - provider contract tests: planned (P1-03).

### LL-17 Verify that the real-data path produces the contract the engine expects

- **What happened.**
  - The nflverse-to-plays-contract adapter was planned as "plumbing". It turned out to be net-new work:
    drive segmentation, next-state derivation, next-score labelling and the participation join.
  - A float/int `play_id` mismatch silently dropped all participation until both sides were normalised.
  - CN docs also conflated the engine `plays` contract with the box-score `stat_events` contract.
- **Evidence.** CN `docs/04-lessons-learned.md` ("The nflverse→GRID adapter was nearly invisible");
  cn-docs §0.7 and §2.4; KI-NEW-V0c.
- **Rule.**
  - Each named contract has a validator at the boundary and a fixture test on real-shaped input.
  - A docstring saying "stubs sketched" is not evidence that a path works.
- **Enforced in.**
  - `docs/03-contracts/plays-contract.md`;
  - `engine-spec.md` §4 and §8.19;
  - contract validation tests: planned (P1-03).

### LL-18 The RAPM universe is every participant, under a stable, fingerprinted index

- **What happened.** CN lost RAPM data in two ways:
  - **A partial universe.** The first real verdict built RAPM over a fantasy-skill-only player frame and
    silently dropped about 22k participants per origin as unknown IDs (CN PR #89).
  - **Reinitialisation on any roster change.** Separately, any change in roster size or order
    reinitialises the accumulators (KI-NEW-W2). Dimension-only guards would also corrupt silently on a
    reorder (CN Phase-4 T1).
- **Evidence.** CN `docs/04-lessons-learned.md` ("The verdict ran on a broken player universe");
  cn-docs §8 items 8–9; KI-NEW-W2 (`reinit_test.py`).
- **Rule.**
  - The universe is every participant known as of W.
  - Unknown participant IDs are counted and reported, never silently dropped.
  - Player indices are stable and append-only, and the order fingerprint is persisted.
  - A new player grows the system. It never resets it.
- **Enforced in.**
  - `docs/05-model-specs/rapm-attribution.md`;
  - `engine-spec.md` §4.4 (identity);
  - P1-12: planned.

### LL-19 Constants calibrated on synthetic data do not transfer to real data

- **What happened.**
  - Real 2023 RAPM rating SD is about 0.04 at every position. The synth QB SD is 0.259.
  - `PRIOR_SD`, the Kalman P0, the age/draft adjustments and the `SSParams` q/r values were all set on the
    synth scale. On real data they are 4–5× too wide or large.
- **Evidence.** KI-NEW-P4, KI-NEW-Y2, KI-NEW-A3; `real_rapm.py`.
- **Rule.**
  - Scale constants are estimated from the data in each run and recorded with the run.
  - The synthetic world includes a profile at the real scale before any constant is calibrated on it.
- **Enforced in.**
  - the "Priors: where they come from" section of `template-model-spec.md` (in place);
  - DR-B4 and DR-C9 (proposed);
  - `docs/05-model-specs/cross-league-priors.md` and `state-space-kalman.md`.

## D. Records, process and evidence

### LL-20 Status logs drift from code; establish status against code at a commit

- **What happened.** CN's status documents disagreed with its code:
  - The issues log said "48 open, 0 closed". GitHub had 39 open, and #50 was missing from the log.
  - Fixed items (KI-#25, KI-P1, KI-G5) were listed as open.
  - "Cross-league priors wired to real data" was false (KI-NEW-P2).
  - The "pre-existing failures" were Windows-only (LL-04).
  - CN `docs/02-backend-spec.md:43` describes the Layer-3 row as `[+1, -1]`. The code does not contain
    that, and should not.
  - `docs/04-lessons-learned.md`, `docs/08-phase-task-context.md:120` and `docs/10-next-steps-plan.md:71`
    all prescribe that change.
- **Evidence.** cn-issues §1; cn-docs §14.
- **Rule.**
  - A status is established against the code at a named commit, and the commit is recorded with it.
  - Documents never claim a fix without the commit that made it.
  - The issue register records the verified status, not a log's.
- **Enforced in.**
  - `docs/00-meta/known-issues.md` (verified-at commit; P0-01);
  - `engine-spec.md` §1 ("evidence over assertion");
  - `scripts/check-evidence-claims.sh` (in place). It checks numeric claims in evidence, not prose; that
    is a known limitation of the P1-00 evidence chain.

### LL-21 Findings, audits and recorded lessons are claims to verify, not facts

- **What happened.** Audit and review claims were often wrong:
  - Several CN audit CRITICALs were false positives (A1, G2) or over-graded (G3: CRIT → MINOR).
  - CN's lessons file recorded one wrong fix (`[+1,−1]`) and one non-defect (fumbles).
  - In this consolidation, the critic overturned the sign conclusions of three of the eight inventory
    reports (X-1 to X-3).
  - CN's own process lesson already said it: "subagent reports are self-reports, not verified facts".
- **Evidence.** CN `docs/04-lessons-learned.md` (Process); cn-issues §0 item 5; critic §2.
- **Rule.**
  - A finding changes code, status or a register only after verification against the code, with a repro
    where one is possible.
  - Read the tests to tell an intentional design from a bug.
  - Verified status and evidence are recorded with the finding.
- **Enforced in.**
  - the "Status" and "Evidence" columns of `docs/00-meta/known-issues.md` (P0-01);
  - the fresh-context review rule (`engine-spec.md` §1, carried from alpha-spec §1.6; in place in
    `CLAUDE.md`).

### LL-22 When a bug is found in one path, check every sibling; prefer one implementation

- **What happened.**
  - The market-row pattern exists in three solvers: `layers.py`, `backtest.py` and `weekly_update.py`.
    CN's lesson knew of two.
  - `var_total` was wrong in three files.
  - The batch and incremental Kalman filters diverge after interventions (KI-NEW-S1).
- **Evidence.** KI-G1 (three sites), KI-G5, KI-NEW-S1; CN `docs/04-lessons-learned.md` (sibling-path
  lesson, kept; its sign prescription is rejected).
- **Rule.**
  - One implementation per primitive: one ridge solver, one filter core and one market-row builder, shared
    by batch and incremental paths.
  - A parity test asserts batch == incremental.
  - When a duplicate is unavoidable, the fix PR lists every site.
- **Enforced in.**
  - `engine-spec.md` §8.1 (crate boundaries);
  - `docs/05-model-specs/state-space-kalman.md` (batch == incremental);
  - P1-06 and P1-12: planned.

### LL-23 Unexercised production paths rot

- **What happened.**
  - CN's scheduler ran `data_pipeline → compute_valuations → sync_leagues` four times a day. It never ran
    `ingest_grid` or `weekly_update`.
  - The in-season GRID path was therefore never operated. It accumulated seven defects that surfaced
    only when the consolidation reviewed and ran it: KI-NEW-W1 to W5, KI-#24 and KI-NEW-S1. Three of them
    were confirmed by running the code.
- **Evidence.** Critic G-8; `known-issues.md` §2.
- **Rule.**
  - The production update path is exercised in CI on synthetic multi-season data, including a season
    rollover and a roster change.
  - The offline backtest and the production path must agree (the two-path equivalence guard).
- **Enforced in.**
  - `engine-spec.md` §8.6 and §12 (two-path equivalence);
  - DR-C14 (proposed);
  - planned (P1-05, P1-12).

### LL-24 Search history before reversing a behaviour

- **What happened.**
  - Three inventory reports recommended replacing the RAPM `lstsq` fallback with a typed failure.
  - None knew that the fallback was added **deliberately** in CN PR #53, audit item C3. That earlier audit
    is recorded only in the PR body, before the shallow-clone boundary.
- **Evidence.** Critic G-3; KI-NEW-A6; DR-B6.
- **Rule.**
  - Before reversing a behaviour, search the history for the decision that introduced it. That includes
    PR bodies and the hosted history beyond any shallow clone.
  - The reversing ADR cites that decision.
- **Enforced in.**
  - the "Reversals" rule of `docs/00-meta/decision-register.md` (P0-01);
  - `docs/07-archive/cautious-nevermore/HISTORY.md` (P0-01);
  - the DR-B6 ADR: planned.

### LL-25 Decisions, plans and constants must be committed, with provenance

- **What happened.** Several CN decisions and constants existed only outside the repository:
  - The Phase-3 plan (`plans/2026-06-22-fantasy-dashboard-phase3-impl.md`) was never committed.
    `LEAGUE_FACTORS`, the age ±0.05 and draft +0.08/+0.03/−0.03 adjustments, and `lambda_by_pos`
    originate there and are undocumented hand-set values (KI-#23, KI-#49).
  - The matchup-grade convention decision, now known to be inverted, lived only in the CN PR #56 body.
  - The real-data verdict reports and frozen gates were gitignored by design.
- **Evidence.** Critic G-3 and G-7; KI-#49; KI-NEW-A2.
- **Rule.**
  - Decisions go to the decision register and an ADR, and plans to a WP file. They never live only in PR
    bodies, chat or local files.
  - Every model constant states its provenance: estimated, calibrated with a named test, or hand-set with
    a rationale and an owner.
- **Enforced in.**
  - `docs/00-meta/decision-register.md` (P0-01);
  - the "Priors" and "Equations" sections of `template-model-spec.md` (in place);
  - `docs/07-archive/cautious-nevermore/real-data-results.md` (historical, non-parity; P0-01).

### LL-26 Commit the evidence with the decision; record the history boundary

- **What happened.**
  - All of this consolidation's evidence (nine inventory reports and about 25 repro scripts) lived only in
    a session scratchpad.
  - Both local CN clones were shallow (88 commits), so the v0 engine (`6b0eeee`) and the first audit were
    visible only on GitHub.
- **Evidence.** Critic G-2 and G-3.
- **Rule.**
  - The evidence behind a decision is committed in the same change: reports to `docs/06-sessions/` and
    scripts to `reference/python/tools/investigations/`.
  - A history claim ("never deleted", "fixed before X") states the clone boundary it was checked against.
- **Enforced in.**
  - `docs/06-sessions/2026-10-01-consolidation-inventory/` and
    `reference/python/tools/investigations/` (P0-01);
  - provenance in `reference/python/README.md` (P0-01).

### LL-27 Record run configuration and negative results with every reported number

- **What happened.**
  - CN's verdict ran in STANDARD scoring with 8-team, 8-round VOR-greedy rosters and a 4-week warm-up.
    No document said so; the alpha default is Half-PPR.
  - CN `docs/02-backend-spec.md:311-316` attributed the ALL-row "+0.957 vs persistence" to TE.
  - The same table omitted the key negative result: GRID loses to the season-to-date mean, −0.194
    [−0.270, −0.120].
- **Evidence.** cn-docs §0.3, §0.5 and §4; `docs/07-archive/cautious-nevermore/real-data-results.md`.
- **Rule.**
  - Every reported metric links to a run record: label, commit, scoring profile, configuration and
    sample size.
  - Negative and null results are recorded with the same prominence as positive ones.
  - Numbers are never transcribed without their source.
- **Enforced in.**
  - `engine-spec.md` §3.3 (evidence status), §5.6 (reports) and §2.3 (default scoring profile).

### LL-28 Validation success is not "GRID wins"

- **What happened.**
  - CN's roadmap set beating last-season actuals as a floor, not proof. It warned that defining success
    as "GRID wins" would create pressure to defeat the leakage guards.
  - The real verdict was a tie on ROS. The weekly win was measured with same-season participation.
- **Evidence.** cn-docs §3.1 and §9; critic X-14; DR-C5.
- **Rule.**
  - Success is a trustworthy, leakage-free answer.
  - The α§9.4 gates stay as pre-registered even though GRID is currently below them.
  - Claim language follows the evidence level.
- **Enforced in.**
  - `engine-spec.md` §3 (success and claims) and §3.3 (current evidence status);
  - DR-C5 (proposed).

## E. Found while writing the model specs (2026-10)

### LL-29 Library defaults are part of the estimator: set them explicitly and record them

- **What happened.**
  - The oracle's V(s) and Layer-1 context boosters leave scikit-learn's `early_stopping='auto'` in place.
    That default turns early stopping on exactly when the training frame exceeds 10,000 rows, and holds
    out a seeded random 10% split (scikit-learn 1.9.1 `gradient_boosting.py:533-534`).
  - The same call is therefore two estimators.
    - Full-season V(s) fits stop after 53–87 iterations at seed 0 (47–182 across seeds 1–10), and train
      on 90% of the rows.
    - The backtest warm-up (4,784 and 8,017 rows) runs all 300 iterations on every row.
    - Layer-1 `n_iter_` varies from 25 to 200 across folds.
  - The oracle, its documents and the inventory all missed it. It was found only by reading the fitted
    objects while writing the model specs.
- **Evidence.** KI-NEW-Z41, KI-NEW-Z3; `value-model.md` §4.1; `layer1-credit.md` §4.9.
- **Rule.**
  - Every hyperparameter that can change the algorithm is set explicitly in code, even where the library
    default would do.
  - No estimator depends on an implicit, data-size-dependent mode switch.
  - A model spec lists every effective hyperparameter, read from the fitted object, with its provenance
    ("library default" included) and the pinned library version.
  - The model version records data-dependent outcomes such as `n_iter_` and any validation split.
- **Enforced in.**
  - the constants tables of `docs/05-model-specs/value-model.md` §4.4 and `layer1-credit.md` §4.9 (P0-01);
  - DR-C7 and DR-D14 (proposed);
  - the Rust `Regressor` trait and booster configuration: planned (P1-06).

### LL-30 Calibrate a parity tolerance against the oracle's own envelope before adopting it

- **What happened.**
  - The proposed Class C tolerance (DR-B3: corr(dV) ≥ 0.999 and |ΔV| ≤ 0.10 EP) was written before anyone
    measured how far the oracle moves against itself.
  - Re-seeding the oracle's own boosters reaches corr(dV) 0.989–0.996 for V(s) and 0.9936–0.9974 for the
    Layer-1 residual, and moves V by up to 0.27 EP on supported states.
  - No seed meets the proposed bound, and a Rust estimator is a larger perturbation than a seed.
  - The Tier-0 recovery floors had the same flaw at the generator level. They were calibrated below one
    seed's values, and they fail on every one of the ten other seeds.
- **Evidence.** KI-NEW-Z74, KI-NEW-Z34; `value-model.md` §7.3; `layer1-credit.md` §5.3;
  `synthetic-world.md` §7.6.
- **Rule.**
  - Before a tolerance or floor is proposed, measure the oracle's self-perturbation envelope, one factor
    at a time: estimator seeds, fold schemes, thread counts and generator seeds.
  - An equivalence tolerance sits outside that envelope. A recovery floor sits below an ensemble
    statistic, never below a single realization.
  - The envelope is stored with the fixture. The statistical owner pre-registers the criterion before any
    Rust result is seen.
  - A Rust result that fails a calibrated criterion is a decision request, not a reason to loosen it.
- **Enforced in.**
  - the DR-B3 amendments, all proposed: C-V (DR-D27) and C-L1 (`layer1-credit.md` §10.3);
  - DR-D26, seed-ensemble Class D floors (proposed);
  - the seed envelope in the V(s) parity fixture (`value-model.md` §10.3): planned (P1-12).
