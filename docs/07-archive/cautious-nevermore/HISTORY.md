# History of the GRID engine in cautious-nevermore

**Non-authoritative history** (`docs/07-archive/README.md`). This file was written for the archive on
2026-10-07. It records how the Python GRID engine was built in `Seismic-Fate/cautious-nevermore` (CN)
before the engine moved to this repository. Where it touches a current rule, the current rule lives in
`engine-spec.md`, an ADR, a model spec or `docs/00-meta/known-issues.md`, never here.

## Sources and their limits

- **CN git history.** Read from the local clone at `59bce1d`. Dates are git author dates in CN's
  local time zone (UTC−5) unless marked UTC.
- **The clone is shallow.** `main` has 88 reachable commits, and `.git/shallow` lists five boundary
  commits (`f3b641f`, `c58aebd`, `c1b6748`, `6a4d473`, `297534c`), all from 2026-06-21 and
  2026-06-22. History behind them is not in the clone. Facts from before the boundary were read from
  GitHub:
  - commits `6b0eeee` and `87e227f`;
  - the bodies of PRs #1–#20 (#15 and #16 are issues), #53–#56, #88 and #91, and the commits of
    PR #92.

  These were read on 2026-10-01 (critic report G-3) and re-read on 2026-10-07 for this file. What is
  in neither the clone nor GitHub's PR and commit records is unrecoverable from here.
- **Squash-commit bodies.** For PRs #63–#91 and #94, the consolidation inventory extracted the squash
  bodies (`engine-pr-dedup.txt`, summarised in cn-docs §5). Spot checks of the PR descriptions for #88
  and #91 matched them, apart from two extra facts recorded below.
- **Review threads.** The 46 CodeRabbit threads on #63–#89 are all resolved, and 45 are marked
  "Addressed in commit". No decision lives only in a review thread (critic G-3).
- **Inventory reports.** "cn-docs", "cn-issues", "critic" and "python-closure" are reports of the
  2026-10-01 consolidation inventory, committed in
  `docs/06-sessions/2026-10-01-consolidation-inventory/`.
- **Not covered here.** The application history (dashboard, draft tools, trades, viz agent, league
  sync, packaging) is mentioned only where it explains an engine fact.

## 1. Origin: the v0 upload and the package move (2026-06-21)

- **`6b0eeee` "initial: existing GRID engine files"** (2026-06-21 04:02 UTC). The v0 engine was
  uploaded at the repository root:
  - `layers.py` (201 lines), `statespace.py` (135), `priors.py` (111), `synth.py` (316), `value.py`
    (76), `data_adapters.py` (77), `run_demo.py` (148) and an `__init__.py` (34);
  - two demo images, `grid_focus_qb.png` and `grid_recovery.png`;
  - the dashboard design specification (archived here) and the Phase-1 plan.

  `run_demo.py` wrote to `/mnt/user-data/outputs`, a Claude.ai sandbox path, which indicates that v0
  was written in a Claude.ai sandbox. No originating design notes are in any repository the
  consolidation could read. If they exist elsewhere, the owner is asked to supply them (decision
  register, "Owner information request").
- **`87e227f` "feat: reorganize GRID engine into backend/grid package"** (2026-06-21 04:12 UTC). The
  six engine modules were renamed into `backend/grid/` with a re-exporting `__init__.py`, and
  `backend/db/connection.py`, `requirements.txt`, `.gitignore` and `.env.example` were scaffolded.
  These are renames, so no engine file was ever deleted from CN (python-closure report).

## 2. The dashboard phases and the lost Phase-3 plan (2026-06-21 to 2026-06-22)

- **Phase 1** built the dashboard foundation from the Phase-1 plan. Its engine-side tasks were the
  nflverse loader and `ParquetCache` (play-by-play TTL 168 hours), the vectorized `build_design`, and
  `joblib` persistence for V(s).
- **Phase 2** (PRs #1–#6) built draft tools. The design specification says it made "no GRID engine
  changes".
- **Phase 3** was executed from a plan that was **never committed**:
  - PR #19 (review-only, closed unmerged on 2026-06-22) says that "all 14 tasks from
    `plans/2026-06-22-fantasy-dashboard-phase3-impl.md`" were delivered through PRs #7–#18.
  - None of the 107 commits in the local clone contains that file, and no CN document reproduces it.
  - The engine tasks survive only as PR bodies:
    - **#8** (Task 2): incremental RAPM accumulators (`accumulate` / `fit_from_accumulators`), the
      `KalmanState` dataclass and `kalman_step`;
    - **#9** (Task 3): the weekly incremental runner `weekly_update.py`;
    - **#10** (Task 4): position-specific ridge scaling `lambda_by_pos` and the multi-QB Layer-1 loop
      `layer1_all_qbs`;
    - **#11** (Task 5): the priors expansion, adding `LEAGUE_FACTORS`, the age and draft-capital
      helpers, and the `birth_date` and `draft_round` columns;
    - **#13**: the merged Phase-3 summary.
- **The Phase-3 constants are undocumented, hand-set values.** No surviving document explains how
  any of them was chosen:
  - `LEAGUE_FACTORS`: FBS 0.35, FCS 0.20, UFL / USFL / XFL 0.15, CFL 0.18, unknown 0.25
    (`reference/python/backend/grid/priors.py:30-38`). They are computed and documented but never
    applied (KI-#23).
  - Age: +0.05 under 24, −0.05 over 30, otherwise 0 (`priors.py:41-52`; KI-#49).
  - Draft capital: round 1 +0.08, rounds 2–3 +0.03, undrafted −0.03, otherwise 0
    (`priors.py:55-68`; KI-#49).
  - `lambda_by_pos`: the only guidance is the docstring example `{"QB": 0.8, "RB": 1.2}`
    (`layers.py:366-367`). No production solver ever passes it (KI-#32).

  All of these sit on the synthetic rating scale. On real data that scale is four to five times too
  wide (KI-NEW-P4; `real-data-results.md`). The statistical owner decides their fate under DR-C9; they
  are not ported as-is.

## 3. The first engine audit: PR #53 (merged 2026-06-22)

PR #53, "fix: GRID engine audit remediation (P0+P1)", merged on 2026-06-22 at 15:08 UTC as
`c04ba9e38508a0ffd389edf0cb0fc053eb8f8be3`. It fixed the P0 and P1 findings of a full engine audit. **The audit document itself was never
committed**; only the PR body survives. The suite went from 230 to 257 tests.

| Item | Change |
|---|---|
| C1 | Joseph-form covariance update in `kalman_two_component` and `kalman_step` |
| C2 | Solve-based RTS smoother, replacing `np.linalg.inv` |
| C3 | Conditioning check plus an **`lstsq` fallback** in the ridge solver. This was **deliberate**: the test plan checks that the fallback "triggers on ill-conditioned matrices, logs warning" |
| C4 | Unknown player ids in `build_design` are skipped with a warning (fast and slow paths) |
| C5, C6 | Trade-model corrections (application) |
| C7 | `interventions` threaded through `kalman_step` into `weekly_update` |
| W1 | FLEX-aware VOR replacement level |
| W7 | `SSParams.from_position()` and the position-grouped `kalman_step` in the weekly pipeline |

"P2/P3 items tracked as GitHub issues #42–#52."

- **A different audit from the later one.** This audit is earlier than, and separate from, the
  2026-07-13 audit (G/P/V/A/W findings, plus a 2026-07-16 projection and validation section) in CN's
  `docs/06-issues-log.md`.
- **Colliding identifiers.** The two audits reuse identifiers with different meanings. In PR #53, W1
  is FLEX-aware VOR; in the later audit, W1 is the Sleeper two-point triple count fixed by PR #94.
  `docs/00-meta/known-issues.md` uses the later audit's identifiers.
- **Reversal.** The Rust engine is to replace the C3 fallback with a typed failure (proposed —
  DR-B6). The ADR that adopts it must cite PR #53 C3 as the decision it reverses
  (`reference/python/PARITY.md`; `docs/05-model-specs/rapm-attribution.md` §4.3).

## 4. Phase 4: matchup models and the matchup-grade convention (2026-06-22)

The Phase-4 plan (archived) was written in `8d9319d`, refined by PR #55 (`d585c50`) and executed by
**PR #56** "Phase 4: Matchup Models + Smart Viz" (`f6a07c5`, 2026-06-22 22:17, i.e. 2026-06-23 UTC).
Each task was implemented by a fresh subagent and reviewed by another; five of their reports are
archived under `dot-superpowers/sdd/`. Tests went from 257 to 369.

Engine content of PR #56:

- canonical `player_id` order and the accumulator order fingerprint;
- the matchup-grade change (below);
- the situation classifier, per-situation solves and per-situation keyed accumulators;
- optional WR × CB interaction columns, off by default;
- the 3-component Kalman state `[talent, form, scheme_fit]`, with every 2-D site made
  `N_STATE`-aware;
- a versioned, self-migrating `.npz` state file ("legacy 2-comp pads to 3-comp losslessly;
  corrupt → reinit"), where the reinitialisation of a corrupt file was deliberate;
- coaching-change scheme resets;
- changepoint detection from standardized innovations.

**The matchup-grade convention decision.** PR #56 records it as: "grades now read the team-defense
intercept, sign-flipped (tougher defense → higher grade); previously stored mislabeled offensive
means". The second half was a real fix: the grades had held offensive means (CN issue #25). The sign
flip was reasoned from "defenders enter X with −1", which gets the algebra backwards. A stronger
defence has a *larger* intercept, so the flipped grade ranks elite defences as the easiest matchups.

- It was confirmed inverted in the consolidation on a generator with correct defenders: the ELITE
  defence got grade −0.409 and the WEAK one +0.382 (KI-NEW-A2; critic X-3).
- The unit tests that pinned the convention used hand-planted betas, so they encoded the error.
- The corrected convention is a proposed default (DR-B5).

PR #56 also listed its honest limitations:

- situation RAPM was synthetic-only until participation existed;
- WR × CB pairing is a positional approximation;
- `situation_grades` had no producer;
- the plural/singular situation vocabulary mismatch (KI-#57).

## 5. The roadmap to alpha and the engine phases 0 to 2c (2026-06-24 to 2026-07-16)

**PR #60** (`4fde294`, 2026-06-24) added CN's `CLAUDE.md`, the roadmap to alpha and the
retrospective validation suite (both archived). Together they set the engine work that followed: make
the engine provable on synthetic truth, build the real-data path, then measure H1 and H2 on real
seasons. One line per PR follows. The test count is the full CN suite as reported in each body.

**Phase 0: engine prerequisites and deterministic gates (2026-06-28)**

| PR | Commit | What it did | Suite |
|---|---|---|---|
| #63 | `9d30abe` | Surfaced the one-step predictive `(mean, S)` from the batch filter and `kalman_step(return_pred=True)`. The trajectory write path now stores the honest predictive variance, including scheme fit, covariances and R. Cache paths made injectable; `allow_pickle` dropped | 384 |
| #64 | `bba75d8` | In-process determinism gate (< 1e-9) and the Tier-0 recovery gates, with thresholds set below the observed synthetic values | 394 |
| #65 | `1642892` | The three-layer golden master (A semantic invariants, B ordering, C numeric rtol 1e-5 / atol 1e-6) under `threadpool_limits(1)`. `snapshot.npz` was last written here | 405 |
| #66 | `d82f478` | Synthetic calibration gates on week-held-out QB innovations, and the QB `r_scale` raised from 0.35 to 0.55. At 0.35, pooled NIS was about 1.6 and PICP@80 about 0.71; at 0.55, about 1.21 and 0.78 | 411 |

The #66 calibration was later shown to be tuned on the defective synthetic world. Correcting the
defender draw moves pooled QB NIS to 1.829, outside its band (KI-NEW-Y0; critic G-1).

**Phase 1: the real-data path (2026-06-29)**

| PR | Commit | What it did | Suite |
|---|---|---|---|
| #67 | `af4b57d` | The nflverse → plays-contract adapter: drive segmentation; `drive_points` from `fixed_drive_result` (touchdown 7, field goal 3, else 0); within-drive next state with terminal `n_* = −1`; offensive participants filtered to skill positions. A manual 2023 smoke run gave V(1st & 10) 1.25 at the 90 rising to 4.91 at the 10, mean dV ≈ 0, 1.96 average drive points | 422 |
| #68 | `9ec1375` | The real participation loader and `load_grid_plays`. The join is normalized on `(game_id: str, play_id: int)`, so that a float/int `play_id` mismatch cannot silently drop all participation. Fixed a 404 roster URL and the `gsis_id`/`full_name` mapping. 2023 participation coverage was 100% | 429 |
| #69 | `a61f0fb` | `weekly_update` loads the plays contract instead of the stats-shaped pbp; `run_rapm` tolerates real rosters without synthetic columns | 431 |
| #70 | `89c7cbb` | `ingest_grid`: a frozen per-season snapshot plus a manifest with coverage; default years 2019–2024 | 438 |
| #71 | `680cfa3` | A known-player rank-band sniff gate (the roadmap's cross-cutting requirement #9) | — |

**Phase 2a: the validation trust layer (2026-06-29 to 2026-06-30)**

| PR | Commit | What it did | Suite |
|---|---|---|---|
| #72 | `374679d` | The numpy-only metrics module: accuracy, NIS, PIT, closed-form CRPS, PICP with PINAW, pinball, tie-aware Spearman, top-N and NDCG; Φ and an Acklam Φ⁻¹ without scipy | — |
| #73 | `be39053` | `AsOf` (outcome slices ≤ W−1, pre-game slices ≤ W, season-aware tuple cutoff, undated interventions rejected), `CacheNamespace` and `TripwireFrame` | — |
| #74 | `df6697c` | Baselines: persistence, season-to-date mean with a per-player spread, last-season per-game, and a market baseline that was never wired | — |
| #75 | `b9d7e0e` | The walk-forward driver: V(s) fitted once on a warm-up pre-period, in-memory incremental accumulators, a solve per origin; teams absent from the market are skipped rather than given a zero prior | — |
| #76 | `ad67dd5` | Four leakage guards, each with a deliberate-leak canary: future poisoning (bit-identical output), watermark, tripwire, and two-path equivalence with `weekly_update` | 499 |

**Phase 2b: the projection stack (2026-06-30 to 2026-07-01)**

| PR | Commit | What it did | Suite |
|---|---|---|---|
| #77 | `3f042d1` | The volume model: empirical-Bayes shrinkage of prior-season per-game usage (`k_shrink = 8.0`) with a manual override hook | — |
| #78 | `ea5e357` | Talent features: prior-season RAPM, an RTS-smoothed talent that genuinely re-runs the smoother, and rookie priors where NaN means "no prior" | — |
| #79 | `2785487` | The stat-line model: per-driver ridge of per-unit rates on standardized talent features, replacing the orphaned `fantasy_scoring.py` constants | — |
| #80 | `1d47419` | Per-position weekly Layer-1 credit through one shared cross-fitted context model (`layer1_all_players`) | 534 |
| #81 | `7443607` | The per-position affine SV → fantasy-points map, the weekly (H2) lever | — |
| #82 | `2690d72` | `estimate_equivalency` made to accept frames without the synthetic `ability` column. No real feeder-SV source was added, so the priors were never actually run on real data (KI-NEW-P2) | — |
| #83 | `e16dfb5` | The preseason path and an injectable projection in `compute_valuations` | 551 |

**Phase 2c: the verdict (2026-07-01 to 2026-07-16)**

| PR | Commit | What it did | Suite |
|---|---|---|---|
| #84 | `0260402` | `bootstrap_ci`, a seeded percentile bootstrap with an "excludes 0" verdict | — |
| #85 | `64216d6` | The H2 lineup simulation: deterministic VOR-greedy snake-draft rosters, one slot-filling rule shared by both methods, and the FLEX-only replacement fix (KI-A6) | — |
| #86 | `00325a3` | The Tier-1 runner: per-position skill against each baseline, and calibration from an expanding spread of strictly past errors | — |
| #87 | `0097987` | The Tier-2 matchup check and the calibrate-then-gate registry (gate = baseline mean + 1.96 · SE, frozen once, `force=True` to recalibrate) | — |
| #88 | `73d58ca` | The verdict runner and report (`SHIP_GRID` / `BASELINE_FALLBACK` / `INCONCLUSIVE`); gates promoted from each KPI's sign-flip null; `OriginResult.def_grades` with the sign-flipped convention of §4 | 602 |
| #89 | `562b721` (2026-07-09) | The RAPM player universe became the full roster. The first real run had dropped about 22,000 participants per origin as unknown ids | — |
| #90 | `138e0cc` (2026-07-09) | Recorded the first real verdict and moved H1 to the volume × efficiency model; recorded the trustworthy run (`real-data-results.md`) | — |
| #91 | `165ccde` (2026-07-16) | Wired `smoothed_talent` into the rest-of-season features, frozen on the pre-first-origin window; set the missing-week input contract | 628 |

Extra facts from the PR descriptions:

- **#88** says the verdict "renders, never acts": flipping `compute_valuations` to GRID was left as the
  owner's decision. It recommended real-run years 2019–2024; the recorded runs used 2022–2023.
- **#91** says a duplicate `player_id` had previously perturbed results at about 1e-6. It moved
  de-duplication to the entry of `run_verdict` because the new smoothed-talent layer, with its
  gradient-boosting context model, "is more numerically sensitive".
- **#91's review rounds** fixed the missing-week input contract. An absent week must be
  `played=False` **and** `y=NaN`, because the filter's start value `nanmean(y[:3])` reads `y` without
  consulting `played`. Seeding absences with 0.0 had pulled a leading-absence RB's smoothed talent from
  1.012 to 0.959.
- **The H1 re-run with the #91 feature was never recorded.** `prior_mean` stayed 0.0 in the verdict
  path.

## 6. Consolidation, Stage 0 and the hand-over (2026-07-16 to 2026-07-18)

- **PR #92** (`379750e`, 2026-07-16) consolidated CN's docs into the numbered `docs/01`–`docs/08`
  and deleted the eight `docs/superpowers/` originals. Its branch commit `5f282cb` states that "all
  content is captured". The consolidation inventory found that most of the quantitative engine content
  was dropped instead (cn-docs §13). That loss is why the originals are archived here. The same branch
  cleared the brainstorming whiteboard (`ac7b55d`); the pre-clear copy is archived too.
- **PR #94** (`91ae204`, 2026-07-17), "Stage 0", worked through a code-versus-docs audit:
  - fixed P1 (a present-but-NaN `prior_mean` crashed the stat-line model; KI-P1), A3 (the
    scoring-format registry returned the wrong id on a name collision) and W1 (the Sleeper adapter);
  - showed A1 (fumbles) and G2 (`complete_pass`) to be false positives;
  - deferred G1/V1, the Layer-3 market row, to a dedicated engine PR with a golden regeneration
    (CN `docs/10-next-steps-plan.md`). That PR never happened. The consolidation has since rejected the queued `[+1,−1]` change, which
    would only align the code with the defective generator (critic X-2; DR-B5).

  It reported three "pre-existing failures": golden-master Layer C twice and a cache TTL test. On
  Linux, the declared platform of record, the consolidation inventory ran the full suite at `59bce1d`:
  637 of 637 pass. The failures
  are Windows effects: cross-platform gradient-boosting variance and coarse file modification times
  (cn-docs §0.6; KI-G9).
- **PRs #93, #95 and #96** (2026-07-16 to 2026-07-18) were application work. **`59bce1d`**
  (2026-07-18) is the last CN commit and the one the oracle was imported from.

## 7. Operational facts

- **The scheduler never ran the GRID engine.** `scripts/run_pipeline.bat` runs
  `data_pipeline` → `compute_valuations` → `sync_leagues`. `scripts/setup_scheduler.ps1` registers it
  four times a day (06:00, 12:00, 18:00 and 00:00). Neither runs `ingest_grid` or `weekly_update`, so
  the GRID weekly path was never operationalised. Every verdict run was a manual developer run. The
  four-times-daily cadence also contradicts this engine's once-per-day fetch rule (`engine-spec.md`
  §1.1). Neither script is ported (critic G-8).
- **What CN shipped.** `compute_valuations`, which feeds the rankings, stayed on the last-season
  baseline (the Phase-2c "documented fallback"). It was never switched to the GRID projection.
- **Environment.** The only environment variable that engine code reads is `DB_PATH`
  (`backend/db/connection.py:8`). The oracle adds `GRID_DEMO_OUT` and the thread pins
  (`reference/python/README.md`).
- **Platform.** The validation suite (§6) declared Linux the golden master's platform of record and
  expected cross-platform differences of about 1e-6. The Windows failures were later misreported as
  regressions (§6 above).

## 8. Decisions in this history that the Rust engine revisits

| CN decision | Where | Status now |
|---|---|---|
| `lstsq` fallback on ill-conditioning | PR #53 C3 | Typed failure proposed; the ADR must cite C3 (DR-B6) |
| Corrupt `.npz` state → reinitialise | PR #56 | Typed failure proposed (DR-B6) |
| Matchup grade = `−β_def` | PR #56, #88 | Inverted; `+E_def` proposed (KI-NEW-A2; DR-B5) |
| QB `r_scale` 0.55 and the NIS bands | PR #66 | Tuned on the defective generator; to be recalibrated (KI-NEW-Y0; DR-B4) |
| Layer-3 row `[+1,−1]` (queued, never landed) | CN docs 04, 08, 10 | Rejected (DR-B5) |
| H1 rest-of-season margin as the kill criterion | Roadmap and validation suite | Diagnostic only (DR-C4, DR-C5) |
| Current-season participation in walk-forward RAPM | PR #75 | Not allowed on the live path (DR-C1) |
| Phase-3 hand-set prior constants | PRs #10, #11 | Back to the statistical owner (DR-C9) |

## 9. Hand-over

In October 2026 the engine moved to `GRID-One/GRID-Engine`. The P0-01 consolidation (2026-10-01 to
2026-10-07) did three things:

- it pivoted that repository to an engine-only scope with Rust as the target language (ADR-011,
  "Engine-only pivot");
- it imported the Python engine at `59bce1d` under `reference/python/` as an executable reference
  oracle (ADR-012, "Python reference oracle");
- it rewrote the specifications engine-only (`engine-spec.md`), with the decisions in
  `docs/00-meta/decision-register.md` and the defects in `docs/00-meta/known-issues.md`.

ADR-011 and ADR-012 and the A decisions are adopted by P0-01 pending owner ratification at merge.
CN itself was not modified. From here on, the engine's history is the history of this repository.
