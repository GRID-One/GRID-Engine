---
date: 2026-10-08 to 2026-10-09
branch: claude/grid-engine-consolidation-e7kmh9
model: unrecorded         # see ai-toolchain.lock and the P0-01 PR, "Known limitations"
phase: 0
work-package: P0-01
harness: claude-code-on-the-web
permission-profile: unrecorded
---

# Session log — 2026-10-09 — P0-01 — owner decision interview

## Objective

The owner asked the implementing agent to "interview me to close any decisions that I can make right
now". The owner holds every owner role (`docs/00-meta/decision-register.md`, "Owner roles"). The agent
asked about every open register entry except DR-D9 and DR-D10 (see "Not asked" below), one question
at a time, through the harness's structured question tool. Each question offered two to four options.
The first option was marked "(Recommended)", except in question 56 (a multi-select) and question 62
(an information request), and the tool always offered free text ("Other") as well. This record gives every
question, the options offered and the owner's verbatim answer. The register's Status lines cite it.

What the agent did and did not do:

- **Recording.** The agent transcribed the owner's answers into the register (DR-D31 precedent) and
  did not decide any entry itself. The owner names the ratified entries again in the P0-01 merge
  comment, so the ratification also stands in the owner's own words on GitHub.
- **Who recommended.** Where an option was marked "(Recommended)" but the register had no default, or
  the agent recommended against the register's literal default, the record marks it *agent's
  recommendation* below. Some of those questions said so in their own text; for the others the mark
  is added here, so the record shows who recommended what.
- **Not asked.** DR-D9 and DR-D10 were not asked: both need real-data measurements that no work
  package has produced yet. DR-D31 was already ratified (2026-10-08).

## Authority loaded

`docs/00-meta/decision-register.md` (every entry), and the documents each entry cites. A fresh-context
read-only workflow drafted one plain-language question card per B, C and D entry, and a second agent
per batch adversarially checked each card against the register; those checks made 205 corrections
before any question was asked. The A-decision questions were drafted from the register directly.

## Decisions and rulings

Answers are verbatim from the question tool. Options are listed in the order offered.

### Merge blockers and A-decisions (2026-10-08)

| # | Entry | Question (condensed) | Options offered | Owner's answer |
|---|---|---|---|---|
| 1 | DR-A10 | Under what licence does the CN-derived code in `reference/python/` sit in an MIT OR Apache-2.0 repository? | MIT/Apache, sole author (Recommended) · Separate licence + NOTICE · Not yet | "MIT/Apache, sole author (Recommended)" |
| 2 | DR-A10 | How should the ruling get onto PR #4? | I'll post it; draft it (Recommended) · Post it for me · Repo record only | "I'll post it; draft it (Recommended)" |
| 3 | DR-A1, A2, A4–A9, A11, A12 | Ratify the ten A-decisions PR #4 already implements? | Ratify all ten (Recommended) · One by one · Ratify at merge only | "Ratify all ten (Recommended)" |
| 4 | DR-A3 | With no Windows app, which platform's run should count for merge and release? (Follow-up ADR before P1-11; PR #4 changes nothing.) | Linux counts; Windows extra (Recommended) · Keep Windows deciding · Both must pass · Decide later | "Linux counts; Windows extra (Recommended)" |
| 5 | ADR-011, ADR-012 | Accept both? (An agent never marks an ADR Accepted.) | Accept in merge comment (Recommended) · Accept now; mark files · Not yet | "Accept in merge comment (Recommended)" |

### B, C and D decisions (answers 6–20 on 2026-10-08, answers 21–62 on 2026-10-09, UTC)

| # | Entry | Question (condensed) | Options offered | Owner's answer |
|---|---|---|---|---|
| 6 | DR-B1 | Policy: correct the oracle's known defects in Python first, as separately approved ledger entries in a fixed order, then make Rust match the corrected oracle? | Fix in Python, then match (Recommended) · Never correct the oracle · Healthy-path parity only · Keep open | "Fix in Python, then match (Recommended)" |
| 7 | DR-B2 | Should CI also re-run Python to prove the committed fixtures still regenerate? | Both, separate Linux job (Recommended) · Committed fixtures only · Live oracle only · Oracle inside verify chain | "Both, separate Linux job (Recommended)" |
| 8 | DR-D29 | On-disk format for parity fixtures? | JSON index + raw binary (Recommended) · NumPy .npz archives · Parquet · Plain JSON throughout | "JSON index + raw binary (Recommended)" |
| 9 | DR-C4 | Which horizons does the engine promise? | Weekly primary, ROS derived (Recommended) · Weekly only · Rest-of-season primary · Keep open | "Weekly primary, ROS derived (Recommended)" |
| 10 | DR-D7 | Is the per-position stat-list asymmetry intended? | Keep as written (Recommended; *agent's recommendation*, register had none) · Make symmetric · Keep open | "Keep as written (Recommended)" |
| 11 | DR-D8 | Build the P1-01 output skeleton on the rule "every open definition is versioned and set with owner approval before the package that uses it is Ready"? | Rule now, values later (Recommended; *agent's recommendation*, register had none) · Set the values now · Keep open | "Rule now, values later (Recommended)" |
| 12 | DR-B6 | Policy: copy the oracle's silent-failure paths, or stop with a typed error? (Numeric cut-off excluded.) | Stop with named error (Recommended) · Copy the oracle · Keep open | "Stop with named error (Recommended)" |
| 13 | DR-C15 | Where does the engine keep its saved state? | SQLite + registered files (Recommended) · SQLite only · Data files only · Keep open | Free text: "Show me the pros and cons of each option" |
| 14 | DR-C15 | (Asked again after a written pros-and-cons comparison.) | SQLite + registered files (Recommended) · SQLite only · Data files only · Keep open | "SQLite + registered files (Recommended)" |
| 15 | DR-C12 | Statistical half: where do the training labels come from? | Official stats, regular season (Recommended) · Both, reconciled · Fixed play-by-play rebuild · Keep open | "Both, reconciled" |
| 16 | DR-C12 | When the official line and the play-by-play rebuild disagree, what is the label? | Official wins, gap flagged (Recommended; *agent's recommendation*, register had no reconciliation rule) · Quarantine until reviewed · Rebuild wins · Leave rule to model spec | "Official wins, gap flagged (Recommended)" |
| 17 | DR-C12 | Keep labels regular-season only, with two-point conversions included? | Regular season + 2-pt (Recommended) · Include playoff weeks · Regular season, no 2-pt | "Regular season + 2-pt (Recommended)" |
| 18 | DR-C13 | Drive value, official situations, two-minute clock? | All three defaults (Recommended) · Defaults, signed points now · Defaults + 3 situations · Defaults, quarter clock | "Defaults + 3 situations" |
| 19 | DR-C13 | Definitions of the three added situations? | Suggested cut-offs (Recommended; *agent's suggestion*: goal_line ≤ 5 yards, third_and_long = 3rd and ≥ 7, fourth_down = every 4th down) · Tighter goal line · Defer to value-model spec | Free text: "Suggested but with a goal_line wr carve-out set at 8" |
| 20 | DR-C13 | Clarification: two plain masks (5 yards, and a separate 8-yard WR mask), or one position-aware mask? | Two masks: 5 and 8 (Recommended; *agent's recommendation*) · One mask, position-aware · Something else | "Two masks: 5 and 8 (Recommended)" |
| 21 | DR-C14 | Run-once command or resident scheduler; self-fetch or pre-downloaded files? | Run-once command, self-fetch (Recommended) · Always-running scheduler · Read pre-downloaded files · Keep open | "Run-once command, self-fetch (Recommended)" |
| 22 | DR-C9 | NCAA only; drop the oracle's age/draft steps; alpha §6.3 blend governs (source of the college rating left open)? | NCAA only, drop, blend (Recommended) · Add other leagues too · Box scores only, no PBP · Keep open | "NCAA only, drop, blend (Recommended)" |
| 23 | DR-C1 | Structure: RAPM only between seasons; in season, participation-free inputs only? | Two-tier: RAPM offseason (Recommended) · RAPM in season if data exists · Licensed in-season feed · Estimate lineups from snaps | "Two-tier: RAPM offseason (Recommended)" |
| 24 | DR-D4 | May prior-season playoff plays train V(s) and RAPM? | Regular season only (Recommended) · Admit past playoffs · Keep open | "Regular season only (Recommended)" |
| 25 | DR-C6 | (a) RAPM blocks summed over the window, S−2/S−1 in season; (b) V(s) fit once a season and frozen? | Accept both (Recommended) · Blocks yes; V(s) in season · Keep open | "Accept both (Recommended)" |
| 26 | DR-B4 | Direction for the synthetic world? | Fix, re-baseline, extend (Recommended) · Fix and re-baseline only · Fix defenders only · Keep open | "Fix, re-baseline, extend (Recommended)" |
| 27 | DR-C10 | The Kalman bundle, and how old evidence is faded? | Bundle, oracle's fade (Recommended) · Bundle, textbook fade · Bundle, wide start · Keep open | "Bundle, oracle's fade (Recommended)" |
| 28 | DR-D6 | Allow non-affine scoring rules (threshold bonuses)? | Points-per-stat only (Recommended) · Support bonuses · Keep open | "Points-per-stat only (Recommended)" |
| 29 | DR-C5 | The evaluation framework (PB-MAE, pre-set gates, week-clustered intervals, union pool) and four written sub-items? | PB-MAE, preset gates (Recommended) · PB-MAE, no tuned gates · cautious-nevermore's H1/H2 · Keep open | "PB-MAE, preset gates (Recommended)" |
| 30 | DR-C5 | Accept the Layer C Gamma–Poisson shrinkage now, or wait for the written comparison? | Wait for the comparison (Recommended; *agent's recommendation* over the register's proposed Gamma–Poisson) · Accept Gamma–Poisson now · Prefer normal–normal | "Wait for the comparison (Recommended)" |
| 31 | DR-C7 | V(s): a deterministic in-house estimator, nflfastR `ep` ruled out? | Deterministic in-house (Recommended) · Keep a booster · Use nflfastR ep · Keep open | Free text: "Which would be best for model accuracy?" |
| 32 | DR-C7 | (Asked again after a written accuracy comparison.) | Deterministic in-house (Recommended) · Keep a booster · Use nflfastR ep · Keep open | "Deterministic in-house (Recommended)" |
| 33 | DR-C8 | Booster backend? | Pure Rust first (Recommended) · XGBoost primary · No booster until needed · Keep open | "Pure Rust first (Recommended)" |
| 34 | DR-C8 | May a WP propose an existing crate (linfa-trees) as an exception to spec §6.4.3? | May propose a crate (Recommended; *agent's recommendation*, register had none) · Hand-written only · Decide when proposed | "May propose a crate (Recommended)" |
| 35 | DR-C2 | Which architecture governs: Layers A–F with GRID feeding in, or GRID as the projection? | Six steps, GRID feeds in (Recommended) · GRID is the projection · Hybrid · Keep open | "Six steps, GRID feeds in (Recommended)" |
| 36 | DR-C3 | How does GRID talent enter Layer D, and does the tracker split by role? | Per-role talent as inputs (Recommended; includes the role split) · One tracker per stat · One talent number (oracle) · Keep open | "Per-role talent as inputs (Recommended)" |
| 37 | DR-B5 | Team-strength estimand, market anchor rows, matchup-grade sign? | Net strength, team totals (Recommended) · Net strength, baselines only · Oracle's offense minus defense · Keep open | "Net strength, team totals (Recommended)" |
| 38 | DR-B3 | Parity pass lines? | Table + amendments (Recommended; *agent's recommendation* over the register's literal default) · Table as written · An earlier proposal · Keep open | "Table + amendments (Recommended)" |
| 39 | DR-D26 | Statistic for the end-to-end recovery gates? | Median, plus low-quantile (Recommended; *agent's recommendation*, the register made the low quantile optional) · Median only · Single world, as now · Keep open | "Median, plus low-quantile (Recommended)" |
| 40 | DR-D15 | The proposed Layer-1′ rules, and per role event or per snap? | Rules + per event (Recommended; *agent's recommendation*, the register had no default for this part) · Rules + per snap · Rules only · Keep all open | "Rules + per event (Recommended)" |
| 41 | DR-D13 | Keep RB/WR/TE credit a unit number, or work toward an individual one? | Pursue individual (Recommended; *agent's recommendation*, register had none) · Unit meaning, permanently · Keep open, no direction | "Pursue individual (Recommended)" |
| 42 | DR-D19 | How is the feeder prior put on the tracker's scale? | Fit to tracker talent (Recommended; *agent's suggestion*, computed without the college prior) · Rating + versioned map · Keep open | "Fit to tracker talent (Recommended)" |
| 43 | DR-D21 | Binomial GLM for the probability components? | Binomial for rates (Recommended) · Poisson counts · Weighted regression throughout · Wait for evidence | "Binomial for rates (Recommended)" |
| 44 | DR-D25 | Approve the recency-baseline form; values by calibration? | Approve form (Recommended; *agent's recommendation*, register had none for the values) · Keep fully open | "Approve form (Recommended)" |
| 45 | DR-D2 | Historical betting lines in backtests? | Keep open, seek a source (Recommended; *agent's recommendation*, register had none) · Never use historical lines · Use nflverse schedule lines | "Keep open, seek a source (Recommended)" |
| 46 | DR-D16 | What the published range assumes; did-not-play weeks? | Projected time; DNP = n/a (Recommended; *agent's suggestion*) · DNP = n/a only · Keep open (interim) | "Projected time; DNP = n/a (Recommended)" |
| 47 | DR-D20 | Ensemble stacking level and weight rules? | Blend shares and rates (Recommended) · Blend finished stats · Blend at both levels · Keep open | "Blend shares and rates (Recommended)" |
| 48 | DR-C11 | VOR and tiers: publish, test-league only, or neither? | VOR only inside test league (Recommended) · Publish VOR and tiers · Neither | "VOR only inside test league (Recommended)" |
| 49 | DR-D5 | Accept "corrected as retrieved" for the Phase 1 proof? | Accept; corrections only (Recommended; *agent's suggestion*, register had none) · Accept; wider scope · Reject · Keep open | "Accept; corrections only (Recommended)" |
| 50 | DR-D24 | Does last-season per-game join the must-beat baselines? | Add to must-beat list (Recommended; *agent's pick* given the owner's accuracy priority, register had none) · Report only · Keep open | "Add to must-beat list (Recommended)" |
| 51 | DR-D3 | ShareAlike terms on participation-derived outputs? | Keep private until release (Recommended; *agent's recommendation*, register had none) · Also fix notices now · Full ruling now | "Keep private until release (Recommended)" |
| 52 | DR-D14 | Layer-1 cross-fit design? | Proposed design (Recommended) · Last window's ratings · Pool seasons · Keep open | "Proposed design (Recommended)" |
| 53 | DR-D28 | How to prove the Rust generator exact? | Draw tape + statistics (Recommended) · Statistics only · Match numpy streams · Keep open | "Draw tape + statistics (Recommended)" |
| 54 | DR-D1 | Source of coaching-change records (form only)? | Dated hand-kept list (Recommended) · Licensed data feed · No explicit changes · Keep open | "Dated hand-kept list (Recommended)" |
| 55 | DR-D11 | Generalise the fixed-point loop, or drop it? | Drop it; single pass (Recommended; *agent's suggestion*, register had none) · Generalise until it converges · Keep open; port as is | "Drop it; single pass (Recommended)" |
| 56 | DR-D12 | Which QB-identifiability remedies must the calibration study compare? (Multi-select; the method was stated in the question.) | Multi-season pooling · QB-specific penalty/prior · Layer-1 credit for QBs | "Multi-season pooling, QB-specific penalty/prior, Layer-1 credit for QBs" |
| 57 | DR-D17 | At a season boundary: restart from the prior, or carry the state? (Also settles DR-C6's Kalman clause.) | Restart from prior (Recommended; *agent's suggestion*, register had none) · Carry, with offseason steps · Carry, offseason discount · Keep open | "Restart from prior (Recommended)" |
| 58 | DR-D18 | Does a changepoint act in the same week or from the next week? | From next week (Recommended; *agent's suggestion*, register had none) · Same week · Keep open | "From next week (Recommended)" |
| 59 | DR-D23 | Stand-in for a provider's missing projection? | Service's lowest at position (Recommended; adds: a provider with nothing at that position that week is excluded and counted) · Zero points · Keep open | "Service's lowest at position (Recommended)" |
| 60 | DR-D22 | May nflverse ep/epa be used as features anywhere? | Engine's own values only (Recommended; *agent's recommendation*, register had none) · Allow ep/epa · Keep open | "Engine's own values only (Recommended)" |
| 61 | DR-D30 | Is the reference-oracle job a required merge check? | Always required (Recommended; *agent's pick* over the register's parity-touching default) · Only parity-touching PRs · Informational only · Keep open | "Always required (Recommended)" |
| 62 | Owner information request | Does an originating GRID design document exist outside git? | Nothing to add · Yes, I'll supply it · Not sure | "Nothing to add" |

DR-D27 was not asked separately. Answer 38 (DR-B3, "Table + amendments") named C-V and its two numeric
limits, which is DR-D27's decidable part.

### Context the owner asked for

- **Answer 13 (DR-C15).** The agent wrote pros and cons of each option, grounded in
  `rapm-attribution.md` §5.1, `engine-spec.md` §8.11 and §14.3, and recommended SQLite with registered
  files before asking again.
- **Answer 31 (DR-C7).** The agent explained that no held-out accuracy comparison exists
  (`value-model.md` §7.5). It said three things favour the deterministic in-house estimator on
  accuracy:
  - its approval rule requires held-out error no worse than the booster's (`value-model.md` §10.2);
  - the booster's seed noise of up to 0.27 EP is pure error;
  - the booster produces values outside the 0–7 hull and out of order (`value-model.md` §4.6).

  It also explained that nflfastR `ep` would inflate backtest accuracy through later-season training.
- **Accuracy priority.** After answer 31 the agent stated each option's expected effect on model
  accuracy in the remaining questions, because accuracy was the owner's stated criterion.

## Work completed

The outcomes were transcribed into `docs/00-meta/decision-register.md`: each entry's Status and
Blocks fields and the summary table. The documents that tag these decisions "(proposed — DR-xx)"
were updated to match, in the same commit series. The decisions that remain open or partly open are
listed in each entry's Status line.

## Verification run

Recorded in the P0-01 evidence manifest for the commit that carries this record (`just evidence
P0-01`), not here.

## Blockers and escalations

None new. DR-A10 needs the owner's own licence-ruling comment on PR #4 (the agent drafted the text).
ADR-011 and ADR-012 stay Proposed until the owner accepts them in the P0-01 merge comment (answer 5), as
`docs/02-adr/README.md` requires.

## Follow-ups

- The owner posts the DR-A10 ruling on PR #4. ADR-012 and `reference/python/README.md` then cite it.
- The merge comment names the ratified A-decisions, accepts ADR-011 and ADR-012, and confirms this
  record.
- After merge, the owner makes `reference-oracle (Linux only)` a required status check on `main`
  (DR-D30).
- The DR-A3 follow-up ADR, before P1-11.
- DR-D9 and DR-D10 stay open until real-data evidence exists.
