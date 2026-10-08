# Manifest: archived cautious-nevermore documents

Every file archived under `docs/07-archive/cautious-nevermore/` is listed here. The archive is
**non-authoritative history** (`docs/07-archive/README.md`): nothing below overrides `engine-spec.md`,
an accepted ADR, a contract, a provider contract or a model spec.

- **Source repository.** `Seismic-Fate/cautious-nevermore` (private), called CN below.
- **Copied on.** 2026-10-07, by the P0-01 consolidation (DR-A9).
- **Method.** `git -C <CN> show <commit>:<original path>`, written unmodified. Each copy was compared
  byte for byte with that output, and with the inventory's scratch copies where they existed.
- **sha256** is the hash of the archived file's bytes. **Blob** is the git blob id of the original
  at the source commit. Neither changes if the copy is intact.
- **Paths.** "Archived path" is relative to `docs/07-archive/cautious-nevermore/`. It mirrors the
  original path, except that CN's hidden `.superpowers/` is stored as `dot-superpowers/` (README,
  "Provenance conventions" item 2).

Source commits:

| Commit | Full id | What it is |
|---|---|---|
| `165ccde` | `165ccded81daf90c5933dbda8d25394efef4a2da` | CN `main` immediately before PR #92 (`379750e`, 2026-07-16) deleted the eight `docs/superpowers/` documents; `379750e^` |
| `c33712e` | `c33712e644e35886ea0e9e86be60421e2fd5fbeb` | First commit of PR #92's branch `docs/consolidate-specs`; the whiteboard as written, before `ac7b55d` cleared it. Not reachable from `main`; reachable from `origin/docs/consolidate-specs` and GitHub `refs/pull/92/head` |
| `546de72` | `546de72319c3144f6ed2cce9ae7db1809bc83829` | Parent of `1aeb7ed` (2026-06-22, "untrack swept-in report files"), which deleted the Phase-4 subagent reports; `1aeb7ed^` |
| `59bce1d` | `59bce1d9f6aac55e823339c044620ddb1d673b7a` | CN `main` at the hand-over (2026-07-18), the commit `reference/python/` was imported from |

## Identity

| Archived path | Source commit | Original path | sha256 | Blob | Bytes / lines |
|---|---|---|---|---|---|
| `docs/superpowers/specs/2026-06-20-fantasy-football-dashboard-design.md` | `165ccde` | `docs/superpowers/specs/2026-06-20-fantasy-football-dashboard-design.md` | `28642e7cd41c80289ceaedf15641c86f963fb014d715ba761f7a1784c9f82284` | `c283e17cd79fa4053e04e629356bfd44a8166b7d` | 26673 / 548 |
| `docs/superpowers/plans/2026-06-22-fantasy-dashboard-phase4.md` | `165ccde` | `docs/superpowers/plans/2026-06-22-fantasy-dashboard-phase4.md` | `bf7aa0d83ceebf3a98bd5018ccc22bb538d18c9f5d8d1e7849462e804d048933` | `5ddb7370aad68d3ddaa9155811f8c518b5792155` | 64084 / 771 |
| `docs/superpowers/plans/2026-06-23-product-roadmap-to-alpha.md` | `165ccde` | `docs/superpowers/plans/2026-06-23-product-roadmap-to-alpha.md` | `7fc942962caba13862df4cc27e10610ff23a4862db37791fc7865d4bd61dc661` | `55f10d3bc6992ccca4ed9f33cb9f205074aaa45e` | 30735 / 436 |
| `docs/superpowers/plans/2026-06-23-retrospective-validation-suite.md` | `165ccde` | `docs/superpowers/plans/2026-06-23-retrospective-validation-suite.md` | `2f519f402405f7fac567573d6d0a78c2e74e9e0b00a12aa1d946635f5c73f524` | `9f702c1c450d01e6cdaf988872dbdb020e5480ba` | 28230 / 442 |
| `docs/superpowers/plans/2026-07-09-phase2c-verdict-and-ros-gap.md` | `165ccde` | `docs/superpowers/plans/2026-07-09-phase2c-verdict-and-ros-gap.md` | `0356c4be2907591eed33edf9451dc7feb0d81e399a1159daa808aa1335d6c1b1` | `a4a8b20e9cde25e633587a4e6fbe37660efea750` | 8338 / 151 |
| `docs/07-brainstorming-whiteboard.md` | `c33712e` | `docs/07-brainstorming-whiteboard.md` | `da18e96a4f6fbf12edf6079fe7a3daadbcd5f66427a003c8b0bc43cce2e6fe1a` | `0c78aece6310f9b0a034d9d43f246cf0f5562cc8` | 6242 / 80 |
| `docs/04-lessons-learned.md` | `59bce1d` | `docs/04-lessons-learned.md` | `d2f7b31f78ddbb297951ffbeb738773b71a86314b3f5db465edc312610de6c8e` | `4b89d0c25d69c4bc87a2c1a4a87eeacfb42db1c8` | 9092 / 77 |
| `docs/08-phase-task-context.md` | `59bce1d` | `docs/08-phase-task-context.md` | `ddcc8a818405f5e8f1fcd2230546b193f3e3b6fcaf408976be3f4eaa1b10f130` | `5017f00f82ada3bc9064757bcccd8216dad73ee9` | 9875 / 174 |
| `dot-superpowers/sdd/task-1-report.md` | `546de72` | `.superpowers/sdd/task-1-report.md` | `1d07c970d99cbd42031786402a59206216ae0b143af3fda3e054a5f71e46b653` | `56a2a88fec5234be37acb11d41908c6fefdc1c7e` | 3609 / 58 |
| `dot-superpowers/sdd/task-5-report.md` | `546de72` | `.superpowers/sdd/task-5-report.md` | `16324a78c031fdb68b00ea23ea4d102b396f6dfc26edc24d03eddd6cddc120e3` | `6fbb7da7d94ab0c78e33ce94d4317a27b62f566b` | 2262 / 30 |
| `dot-superpowers/sdd/task-6-report.md` | `546de72` | `.superpowers/sdd/task-6-report.md` | `aeeb09cb6793331c016614557b56446bb415587d1392e57b35411d556995b175` | `f12a14d283d595243b0ec2455b3890d9f448c8ce` | 2272 / 32 |
| `dot-superpowers/sdd/task-7-report.md` | `546de72` | `.superpowers/sdd/task-7-report.md` | `e2242ce886bdab1806a181ffb1477e016cfda78cc029d7ff791ad0217d52ac75` | `664769270b79627a6558f5ee6d10dc63bf3fa482` | 2630 / 40 |
| `dot-superpowers/sdd/task-8-report.md` | `546de72` | `.superpowers/sdd/task-8-report.md` | `8ab9988d78cb4158ca60f9e4aeda82698b03c38bf3a5f00c815a069da426987a` | `fa16098f5b5f116e0c073166f145a88a1928df24` | 5905 / 108 |

Re-check every row from `docs/07-archive/cautious-nevermore/` with:

```bash
sha256sum docs/superpowers/specs/*.md docs/superpowers/plans/*.md docs/0*.md dot-superpowers/sdd/*.md
```

The files written for this archive (`MANIFEST.md`, `HISTORY.md`, `real-data-results.md`) are not
copies and have no row.

## Per-file record

Section references to `engine-spec.md` follow its consolidated outline: §1 constraints and governance,
§2 scope, §3 success and evidence status, §4 data, §5 outputs, §6 modeling (§6.2 the GRID signal stack,
§6.9 the synthetic world), §7 validation (§7.12 oracle parity, §7.13 synthetic recovery gates, §7.14
retained GRID diagnostics), §8 architecture, §9 and §10 phases, §12 testing, §13 performance. "KI-"
identifiers are rows of `docs/00-meta/known-issues.md`; "DR-" identifiers are entries of
`docs/00-meta/decision-register.md` (A entries adopted by P0-01 pending ratification; B and C entries
proposed defaults).
"cn-docs", "cn-issues" and "critic" are reports of the consolidation inventory in
`docs/06-sessions/2026-10-01-consolidation-inventory/`.

### 1. `docs/superpowers/specs/2026-06-20-fantasy-football-dashboard-design.md`

- **What it is.** The original design specification of the fantasy-football dashboard. GitHub shows it
  added in `6b0eeee` ("initial: existing GRID engine files", 2026-06-21) together with the v0 engine
  and the Phase-1 plan. The local clones show only `f3b641f` (2026-06-21), a shallow-history boundary.
- **Why archived.** The genesis of the engine roadmap. It is about a quarter engine:
  - §3, the data contract, with the original column names;
  - §4, the fantasy projection layer;
  - §5, "GRID Engine Evolution" phase by phase, including the incremental pipeline and the rule that
    the value model stays fixed within a season;
  - §10, the performance and storage targets.

  Deleted by CN PR #92 and recoverable only from git history.
- **Superseded by.**
  - Scope: `engine-spec.md` §2. The application sections (dashboard, draft tools, trades, viz agent,
    league sync, packaging) have no successor; the engine-only pivot excludes them (ADR-011).
  - Data contract: `engine-spec.md` §4 and `docs/03-contracts/plays-contract.md`.
  - Engine evolution: `engine-spec.md` §6.2, and the phases of §9 and §10.
  - Performance targets: `engine-spec.md` §13.
- **Known errors and cautions.**
  - §3's "extended fields" belong to the box-score aggregation, not to the engine's plays contract
    (cn-docs §2.2). The names `next_down`, `next_ydstogo`, `next_yardline_100` and `yards_gained` are
    stale; the code uses `n_*` and `yards`.
  - Several Phase-3 items were never built: WR/TE target-share attribution (KI-#32) and separate
    FBS/FCS/UFL equivalency (KI-#23; DR-C9).

### 2. `docs/superpowers/plans/2026-06-22-fantasy-dashboard-phase4.md`

- **What it is.** The Phase-4 implementation plan ("Matchup Models + Smart Viz"), added in `8d9319d`
  and refined in `d585c50` (PR #55), both 2026-06-22. Executed by PR #56.
- **Why archived.** About half of it is engine design that the consolidated CN docs cut to a short
  summary. Tasks 1, 2 and 4–11 give the procedures and parameters for:
  - canonical player order and the accumulator order fingerprint;
  - matchup-grade semantics;
  - the situation classifier, situation RAPM and situation-keyed accumulators;
  - WR–CB interaction columns;
  - the 3-state Kalman filter and its `.npz` migration;
  - coaching-change scheme resets;
  - changepoint detection.

  Its "Known Constraints" table states the limits honestly, including the weak identification of
  talent versus scheme fit under `H = [1,1,1]`.
- **Superseded by.**
  - `engine-spec.md` §6.2.
  - `docs/05-model-specs/rapm-attribution.md`: design matrix and column order (§4.1, §4.9), matchup
    grade (§4.5), situation RAPM (§4.8), interactions (§4.1).
  - `docs/05-model-specs/state-space-kalman.md`: state and transition (§4.1), scheme reset and
    changepoints (§4.6), state persistence (§6.2).
  - Tasks 0, 3 and 12–19b (database, API, viz agent, frontend) have no successor (ADR-011).
- **Known errors and cautions.**
  - **Task 2's matchup-grade convention is inverted.** It reads the team-defence intercept,
    sign-flipped, as "tougher defence → higher grade". With defenders entered at −1, a stronger defence
    has a *larger* intercept, so the flip assigns elite defences the lowest grade (KI-NEW-A2; critic
    X-3; DR-B5).
  - Task 10 and PR #56 treat scheme resets as independent of interventions
    (`test_scheme_reset_independent_of_intervention`). That holds in one direction only: interventions
    also add `scheme_reset_var` to the scheme-fit variance (cn-docs §14 item 9;
    `state-space-kalman.md` §4.6).
  - The two-minute situation as specified (`quarter_seconds_remaining <= 120`) ignores the quarter
    (KI-NEW-A4; DR-C13).
  - The per-situation `matchup_grades` vocabulary does not match the `situation_grades` schema
    (KI-#57).

### 3. `docs/superpowers/plans/2026-06-23-product-roadmap-to-alpha.md`

- **What it is.** CN's roadmap from the Phase-4 state to an alpha release, added with the validation
  suite by PR #60 (`4fde294`, 2026-06-24). It governed the engine phases 0 to 2c (PRs #63–#91).
- **Why archived.** About 45% engine, and much of that was dropped by PR #92 (cn-docs §13):
  - §2, the decisions ledger, and §3, the architectural invariants. Invariants #2 to #4 and the cache
    rule of #6 are the origin of rules now in `engine-spec.md` §1.
  - §4.4, the real-data adapter, recognised as net-new L–XL work rather than plumbing.
  - §4.5, the projection architecture: fantasy points are volume × efficiency, and "GRID cannot *be*
    the projection — it is the most valuable *feature* in one".
  - §5, the Phase 0–2c done-when criteria, including the ≥ 99% participation coverage gate.
  - §6.5, the cross-cutting requirements #4, #8, #9 and #11.
  - §7, the risks, and §7.5, the three compute budgets.
- **Superseded by.**
  - Invariants: `engine-spec.md` §1.
  - Projection architecture: the `engine-spec.md` §6 preamble and §6.1 (volume × efficiency maps to
    Layers C × D; DR-C2, DR-C3).
  - Adapter and coverage gate: `engine-spec.md` §4, `docs/03-contracts/plays-contract.md`,
    `docs/04-providers/nflverse/README.md`.
  - Calibrate-then-gate: `engine-spec.md` §7.14.4.
  - Compute budgets: `engine-spec.md` §13.
  - Phases: re-planned as the engine work packages of `engine-spec.md` §9 and §10. The application
    phases 3–5 have no successor (ADR-011).
- **Known errors and cautions.**
  - **"H1 = kill criterion"** no longer holds. Rest-of-season and H1 are diagnostics, not the engine's
    contract or a gate (DR-C4, DR-C5; `engine-spec.md` §7.14.1).
  - §6.5 #11 states that "nflverse data is CC-BY-SA 4.0". Only participation and FTN charting are
    verified as CC-BY-SA 4.0. The nflverse-data repository is CC-BY-4.0, and the other datasets need a
    Data/Licensing ruling (`docs/04-providers/nflverse/access-and-license.md`).
  - The "GRID-powered or bust" stance, the Flutter/Avalonia note and the packaging phases are
    application history.

### 4. `docs/superpowers/plans/2026-06-23-retrospective-validation-suite.md`

- **What it is.** CN's validation design, added by PR #60 (`4fde294`, 2026-06-24) and implemented by
  PRs #63–#91. **The highest-value archived document.**
- **Why archived.** About 95% engine. It holds:
  - the tier structure (Tier 0 recovery, Tier 0.5 golden master and determinism, Tier 1 real
    walk-forward, Tier 2 attribution and matchup);
  - the target choice, including scoring both realized points and an opportunity target;
  - the three-layer golden master and its determinism hazards;
  - the calibration design: one-step predictive distributions, conditional versus unconditional on
    snaps, PIT reading, the aggregation escape for skewed weekly points;
  - the three leakage axes, the GRID-specific leaks, and the four guards with canaries;
  - the KPI targets per tier;
  - the participation data-source finding (§15).

  Most of the quantitative detail was cut by PR #92.
- **Superseded by.**
  - `engine-spec.md` §7: §7.12 oracle parity, §7.13 synthetic recovery gates, §7.14 GRID diagnostics.
  - `engine-spec.md` §12: §12.2 golden numerical tests, §12.3 point-in-time and leakage tests.
  - `docs/05-model-specs/evaluation-and-leakage.md` and `docs/05-model-specs/synthetic-world.md`.
  - `docs/03-contracts/parity-fixture-contract.md`.
  - Participation finding: `docs/04-providers/nflverse/README.md` and `access-and-license.md`.
- **Known errors and cautions.**
  - **The §9 KPI targets were never implemented or met.** For example, pooled attribution ≥ 0.85 was
    planned; the implemented gate is ≥ 0.77 against an observed 0.8025 (`engine-spec.md` §7.13.2;
    cn-docs §3.6). They are aspirations, not gates.
  - Every synthetic number behind the design was measured on the legacy generator, which draws a
    play's defenders from the offence's own roster (KI-NEW-Y0; critic G-1).
  - The confidence intervals it specifies are iid percentile bootstraps. The engine requires a
    week-clustered bootstrap (KI-NEW-V1; `engine-spec.md` §7.7).
  - H1 as the kill criterion is superseded (DR-C4).
  - The Tier-2 design rests on the matchup grade whose sign is inverted (KI-NEW-A2).
  - Its participation licence statement covers FTN data from 2023 only. Seasons 2016–2022 carry a
    different attribution, "NFL NextGenStats via nflverse" (`docs/04-providers/nflverse/access-and-license.md` §2).

### 5. `docs/superpowers/plans/2026-07-09-phase2c-verdict-and-ros-gap.md`

- **What it is.** The record of CN's real-data verdict runs. Added by PR #90 (`138e0cc`, 2026-07-09)
  and given its final "Update" section by PR #91 (`165ccde`, 2026-07-16).
- **Why archived.** The verdict reports and the per-run frozen gates were gitignored by design
  (PR #88). This document is the only committed transcription of them, and it was deleted by PR #92
  (critic G-7).
- **Superseded by.** `real-data-results.md` in this directory (historical, non-parity);
  `engine-spec.md` §3.3 and §7.14.5; DR-C4 and DR-C5.
- **Known errors and cautions.**
  - **No number in it is a parity target or citable evidence for this engine.** The reasons are
    biased labels (KI-NEW-I1 to KI-NEW-I4, KI-NEW-V0a), iid confidence intervals (KI-NEW-V1) and
    current-season participation used before its publication (DR-C1). See `real-data-results.md`.
  - The calibration figures it quotes (weekly NIS 1.00, rest-of-season NIS 1.25, PICP@80 0.75–0.87)
    are stated for the first run, before the universe and rest-of-season fixes.
  - The H1 re-run after PR #91 was never recorded.

### 6. `docs/07-brainstorming-whiteboard.md` (before it was cleared)

- **What it is.** CN's brainstorming whiteboard as written in `c33712e` (2026-07-16). `ac7b55d` cleared
  it to an empty template 27 minutes later; the copy on CN `main` is that template.
- **Why archived.** It holds engine ideas recorded nowhere else (cn-docs §1 row 7b), and the commit is
  not reachable from CN `main` (critic G-9).
- **Superseded by, idea by idea.**

  | Idea | Where it is tracked now |
  |---|---|
  | ADP as an automated volume (role-change) signal; promote the market baseline to a gate | `engine-spec.md` §7.6 benchmark registry; DR-C5 |
  | Smooth age curves instead of step functions | KI-#49; DR-C9 |
  | `prior_mean` for rookies only | `docs/05-model-specs/cross-league-priors.md`; KI-NEW-P2 |
  | Points-allowed proxy from play-by-play drive points, to unblock Tier 2 | KI-NEW-V2; DR-B5 |
  | Check the cross-season watermark unconditionally | KI-V2 |
  | Changepoint detection from Kalman innovations, wired to production | `docs/05-model-specs/state-space-kalman.md` §4.6; KI-#24 |
  | Calibrate per-position ridge λ by synthetic out-of-sample recovery | KI-#32 |
  | H2 on real post-draft rosters | DR-C11 |

  The Frontend, Packaging and Product sections are application ideas with no successor (ADR-011).
- **Known errors and cautions.**
  - "The synthetic H2 already passed (+0.848)" is misleading. The rosters were synthetic, but the data
    was real, and the result is not citable (`real-data-results.md`).
  - The NaN `prior_mean` crash it mentions was fixed by CN PR #94 (KI-P1).

### 7. `docs/04-lessons-learned.md`

- **What it is.** CN's lessons-learned log, created by PR #92 (`379750e`), as of `59bce1d`.
- **Why archived.** Provenance for `docs/00-meta/lessons-learned.md`, which re-verified and carried
  forward its engine entries.
- **Superseded by.** `docs/00-meta/lessons-learned.md`. The Frontend and most Data-pipeline entries are
  application history.
- **Known errors and cautions.**
  - **"fumbles_lost counts ALL fumbles" is false.** The column is read from `fumble_lost`
    (KI-A1, a false positive). The real defect is fumble *attribution* (KI-NEW-I3).
  - **"Market reconciliation sign error (G1 + V1)" prescribes `[+1,−1]`.** That change only aligns the
    code with the defective synthetic generator, and it is rejected (critic X-2; DR-B5; KI-G1). The bug
    also has a third site, `weekly_update.py:273`, that the entry misses.

### 8. `docs/08-phase-task-context.md`

- **What it is.** CN's agent guide, created by PR #92 (`379750e`), as of `59bce1d`. It lists
  conventions, the key file map, known traps and the PR → phase log for #63–#91.
- **Why archived.** The PR log and the traps are the compact index of the engine build history.
- **Superseded by.**
  - Traps: `docs/00-meta/lessons-learned.md`.
  - PR log: `HISTORY.md` in this directory.
  - Commands and layout of the Python engine: `reference/python/README.md`.
  - How agents work in this repository: `engine-spec.md` §8.14 to §8.17, and `CLAUDE.md`.
- **Known errors and cautions.**
  - **Not instructions for this repository.** Its commands and its CLAUDE-style rules are CN's.
  - The "Market reconciliation sign convention" trap prescribes `[+1,−1]`, which is rejected
    (critic X-2; DR-B5).

### 9. `dot-superpowers/sdd/task-1-report.md` (Phase-4 Task 1)

- **What it is.** A subagent report for the Phase-4 plan's Task 1: canonical `player_id` column order
  (`sorted(key=str)`, preserving dtype) and the persisted accumulator order fingerprint. The five
  reports were tracked in CN until `1aeb7ed` untracked them on 2026-06-22. When they were first
  committed is behind the local clones' shallow boundary.
- **Why archived.** It records why `key=str` was chosen over string coercion (the dtype of the ratings
  merge), and the order-fingerprint rule that a dimension-only check misses.
- **Superseded by.** `docs/05-model-specs/rapm-attribution.md` §4.9 (column order) and §6 (accumulators).
- **Known errors and cautions.**
  - The reinitialise-on-reorder guard it adds discards every prior week whenever the roster changes.
    The engine must not reproduce that (KI-NEW-W2).
  - Its `allow_pickle=True` load was later removed by CN PR #63.

### 10. `dot-superpowers/sdd/task-5-report.md` (Phase-4 Task 5)

- **What it is.** The report for `run_situation_rapm`: per-situation RAPM solves, `min_plays=200`, and
  a hard error when participation columns are absent.
- **Why archived.** The "honesty gate" (situation RAPM is exactly as dependent on participation as base
  RAPM) and the test design for situation independence.
- **Superseded by.** `docs/05-model-specs/rapm-attribution.md` §4.8.
- **Known errors and cautions.** Its statement that `load_participation` is a stub was overtaken by CN
  PR #68, which made it real.

### 11. `dot-superpowers/sdd/task-6-report.md` (Phase-4 Task 6)

- **What it is.** The report for situation-keyed incremental accumulators
  (`rapm_accumulators_{key}.npz`, `key='all'` for the legacy file) and the pipeline constant
  `MIN_PLAYS = 50`.
- **Why archived.** The file-keying and backward-compatibility rules of the situation pass.
- **Superseded by.** `docs/05-model-specs/rapm-attribution.md` §4.8 and §6.
- **Known errors and cautions.**
  - It inherits the week-only skip guard and the roster-change reinitialisation of the incremental path
    (KI-NEW-W1, KI-NEW-W2), and writes grades with the inverted sign (KI-NEW-A2).
  - Its "Commit" field was never filled in.

### 12. `dot-superpowers/sdd/task-7-report.md` (Phase-4 Task 7)

- **What it is.** The report for optional WR × CB interaction columns (`interactions=False`,
  `min_pair_plays=30`, ridge penalty 10.0) and the synthetic `cb_split` / `n_cb_per_team` knob.
- **Why archived.** The positional-approximation honesty note and the guarantee that the default path
  is byte-identical.
- **Superseded by.** `docs/05-model-specs/rapm-attribution.md` §4.1. Interactions are research-only in
  `engine-spec.md` §6.2.
- **Known errors and cautions.**
  - No oracle solve path ever builds interaction columns, so the penalty branch it adds is unreachable
    (`rapm-attribution.md` §4.1).
  - With `cb_split=True` the synthetic team strength excludes the relabelled CBs (KI-G3).
  - The synthetic world plants no interaction effect, so no recovery claim can be made
    (`engine-spec.md` §6.9).

### 13. `dot-superpowers/sdd/task-8-report.md` (Phase-4 Task 8)

- **What it is.** The report for growing the Kalman state from `[talent, form]` to
  `[talent, form, scheme_fit]`. It enumerates every two-dimensional site that had to become
  `N_STATE`-aware and lists the new parameters (`phi_scheme=0.985`, `q_scheme=0.0002`,
  `scheme_reset_var=0.04`).
- **Why archived.** It is the only site-by-site record of the port, a useful checklist for the Rust
  state-space port.
- **Superseded by.** `docs/05-model-specs/state-space-kalman.md` §4 and §6.2.
- **Known errors and cautions.**
  - The batch filter's start value `nanmean(y[:3])` looks ahead two weeks (KI-#15; DR-C10).
  - The batch and incremental filters it ports diverge after an intervention (KI-NEW-S1).
  - This report (CN `.superpowers/sdd/task-8-report.md`) is unrelated to CN's top-level
    `sdd/task-8-report.md` (pipeline automation), which is not archived.
