# GRID Fantasy — Product Roadmap to Windows Alpha

**Date:** 2026-06-23
**Author:** Claude (brainstorm → roadmap)
**Status:** Draft for review
**Branch:** `claude/retrospective-validation-testing-ibbub8`
**Companion:** `docs/superpowers/plans/2026-06-23-retrospective-validation-suite.md` (the
validation suite, which is a **gating dependency** for this roadmap).

---

## 1. Vision & target

Ship an **installable Windows alpha** of GRID Fantasy: a **GRID-powered** fantasy-football
draft-prep tool that a few technical friends can install and use to prepare for the 2026
draft. **Quality over date.** The **late-Aug-2026 soft goal points at the pre-alpha demo**
(end of Phase 3) — but the verification pass grew the critical path (the §4.4 adapter +
weekly-signal scope), so under **solo + heavy agent use** (~10–13 IED/wk, §9) **demo ≈ September
and packaged alpha ≈ October** is the honest read; late-Aug is now a stretch. **No
baseline-interim ship** — the demo and alpha are **GRID-powered or they wait.** Quality governs:
the date yields, not the rigor.

**What "alpha" is:** the **Tier-A core loop** — Rankings, Draft Room (mock), Workbench, and a
health Dashboard — running on **real nflverse data**, with GRID talent driving the
projections (§4.5), installable with **zero config** (no cookies, no league IDs, no API key).

**What "alpha" is not:** in-season league management (Start/Sit, Waiver, Trades, live-draft
sync), which is a **pre-Week-1 fast-follow**; and the LLM viz agent, which ships **behind a
flag, default off**.

## 2. Decisions ledger (locked this cycle)

| Area | Decision |
|---|---|
| Core projections | **GRID-powered** — a fitted stat-line projection (GRID talent features + a volume model, §4.5) replaces the last-season baseline in `compute_valuations` |
| Participation data | Free via nflverse/FTN through 2025; the *join* is ~1–3 days, but it rides on the larger **nflverse→GRID adapter** (§4.4, L–XL) — not standalone. Live-weekly is a research track |
| Real-data GRID path | **nflverse→GRID `plays`-contract adapter is net-new** (§4.4) — drive/next-score/participation; gates all of Phase 2. (Corrected from "plumbing exists" after verification.) |
| H2 weekly signal | **Build per-position weekly Layer-1 credit + SV→points map in alpha** (Phase 2b) so the weekly lineup-margin headline works for RB/WR/TE, not QB-only |
| Validation | **Recurring gate** — must be green at every phase boundary |
| Alpha scope | **Tier-A core**; Tier-B (league-sync) = pre-Week-1 fast-follow; viz agent flagged off |
| Aesthetic | **Disciplined comic pop-art, dark palette** — pop chrome over a calm, readable table core |
| Design ambition | **Right-sized** — coherent design system + one polish pass; time-boxed |
| Packaging (alpha) | **conda-`constructor`** offline installer + **pywebview + tray** + **ingest on first run** |
| Packaging (beta) | **Tauri** (native window/menus/notifications; macOS near-free), reusing the React UI |
| Long-term native | Tauri-grade in beta; evaluate true-native (Flutter/Avalonia) only if feel falls short |
| Headlines | **H1** beat last-season on ROS skill (kill criterion); **H2** weekly lineup point margin |
| Timeline stance | **GRID-powered or bust.** Late-Aug soft goal → **pre-alpha demo**; packaged alpha ~Sept; date yields before rigor; no baseline-interim |
| Capacity | **Solo + heavy agent use** (~10–13 IED/wk on well-specified work) |
| Sizing basis | **Effort-relative** (T-shirt) + a calendar read under the capacity above (§9) |

## 3. Architectural invariants (checked at every phase boundary)

1. **Thin client, fat backend.** All logic and data access live behind the FastAPI HTTP/WS
   API; the UI only renders and calls endpoints. Keeps pywebview → Tauri → (maybe) true-native
   cheap, because each is a shell swap, not a rewrite.
2. **Engine is UI-agnostic.** The GRID engine never moves to accommodate a UI; the
   client/server seam stays clean.
3. **The synthetic-data contract is preserved** (`data_adapters.py`) through every engine
   change.
4. **Validation green** (the suite) before any phase is considered done.
5. **Whole `pytest -q` suite green** + frontend `tsc -b`/build/lint green per PR.
6. **User data is separable & versioned.** The shipped app keeps its DB + caches outside the
   install dir, schema-versioned for safe upgrades (§6.5 #4); generated caches are reproducible,
   never hand-migrated.

## 4. Current-state baseline (what exists today)

- **Built & working on real data:** the **baseline path** — `data_pipeline` → `player_stats`
  → `compute_valuations` (scores last-season stat lines). Scoring + VOR; ~28 API endpoints; 8
  React pages + chart library (~3,600 LOC); Windows Task Scheduler `.ps1`/`.bat`. ESPN
  `get_picks`/`get_available_players` and Sleeper `get_available_players` are **implemented**
  (not stubs) — the real stub surface is narrow (e.g. some `get_roster` miss-cases).
- **Built but synth-only:** the **GRID engine** (value/RAPM/Kalman/priors) is validated on
  `synth.py` but **cannot run on real data** — see §4.4. `priors.py` is synth-only too
  (`estimate_equivalency` needs an `ability` column that exists only in synth).
- **Missing / gaps:** **GRID does not power the product's projections** (last-season baseline —
  `compute_valuations.py:162`); **no nflverse→GRID adapter** (§4.4); no packaging; bare-Tailwind
  UI (no design system); frontend `tsc -b` historically red.

## 4.4 The real-data GRID adapter (the true critical-path long pole — added post-verification)

**Surfaced by independent review; previously mis-scoped as "plumbing exists / thin wrapper."**
There are two real-data paths and only one works today:
- **Baseline path** (`player_stats` → `compute_valuations`): works on real nflverse data now.
- **GRID engine path** (drive-value → dV → RAPM/Kalman): **blocked.** The engine consumes the
  `plays` contract (`data_adapters.py`: `drive_points, terminal, terminal_value, n_down/
  n_ydstogo/n_yardline_100, off_players, def_players, drive_id`). The real-data loader
  (`nflverse_loader._normalize_pbp` → `data_pipeline`) emits **none** of those — only a
  stats-aggregation contract. So `fit_value_model` (needs `drive_points`), `compute_dv` (needs
  `terminal`/`n_*`), and `build_design` (needs `off_players`/`def_players`) all fail on real
  data; `weekly_update` would `KeyError` but a broad try/except silently no-ops, so it *looks*
  like it runs.

**Net-new work (Phase 1, L–XL):** an nflverse→GRID adapter doing drive segmentation, next-state
derivation, next-score (`drive_points`) labeling, and the participation join — logic that today
exists **only in `synth.py`**. This is the gating dependency for *everything* in Phase 2
(projection model, all backtests, H1/H2, the two-path leakage guard). It is the real long pole.

## 4.5 Projection architecture (the product's core algorithm)

**The gap this closes:** today the projection is the last-season stat line scored per format
(`compute_valuations.py:160`). An orphaned sketch (`backend/fantasy_scoring.py`) tries
"historical-average stat line + `grid_rating × hand-picked constant`" but is **uncalibrated
and not wired in.** There is no real projection model yet; Phase 2 builds one.

**Key principle:** fantasy points = **volume × efficiency**. Volume (attempts/targets/carries/
snaps) is the dominant, sticky driver; GRID's RAPM/Kalman measures **per-play efficiency/
talent only**. So GRID cannot *be* the projection — it is the most valuable *feature* in one.

**Structure (decided): project a STAT LINE, then score** (format-flexible — one projection
scored through any scoring format, incl. custom; preserves interpretability; keeps the
`fantasy_scoring.py` shape but replaces magic constants with a fitted model):

```
projected_stat_line(player, season) = g( volume/role features,
                                          GRID talent features,
                                          age, prior, team/role-change signal )
projected_points(format) = calculate_points(projected_stat_line, format)
```

- **GRID talent features:** RAPM season rating + Kalman smoothed talent + cross-league prior
  (the efficiency signal). These enter the model as features; `g` is **fit on historical
  seasons** — which is exactly the Tier-1 ROS walk-forward target, so the projection model and
  Tier-1 validation are the same build.
- **Volume/role model (NEW workstream, decided):** a separate projection of usage
  (attempts/targets/carries/snaps) — even a simple regression (prior usage + age + games) with
  a manual **ADP/depth-chart override hook**. This addresses the dominant driver and the main
  H1 risk; GRID does not supply it.

**Preseason / draft-time cold-start** (the alpha's primary use, with no current-season weeks):
- **Talent side — design supports it, integration is net-new:** Kalman **RTS-smoothed
  end-of-prior-season** state ("best retrospective talent") + prior-season RAPM + **cross-league
  priors for rookies**. The engine *design* fits this, but none of it is wired into product
  rankings today and `priors.py` is **synth-only** (`estimate_equivalency` needs an `ability`
  column that only exists in synth) — so the preseason ranking path is net-new integration, not
  a flip of a switch.
- **Volume side — the hard part:** prior-season usage is a weak proxy for offseason role
  changes; the override hook (ADP/depth chart) is the upgrade path. **Documented known
  weakness for alpha: role-changers.**

**Weekly skill-position signal (added to alpha scope post-verification).** H2's weekly
lineup-margin headline needs a *weekly per-player fantasy projection for RB/WR/TE*, but the
engine produces weekly Layer-1 credit for **QB only** (`layer1_qb_weekly`) and runs in dV/SV
currency. So two engine pieces move from "future work" into **Phase 2b scope**: a
**per-position weekly Layer-1 credit** and an **SV→fantasy-points map**. Without them H2 is
structurally unwinnable for skill positions. This grows Phase 2b (see §9).

**H1 implication (sharpens the kill criterion):** last-season-actuals already encodes volume ×
efficiency. GRID's win must come from (a) regressing unsustainable efficiency/TD-rate to
talent, and (b) trajectory/aging — **not** (c) volume/role change, which GRID doesn't model.
The volume model exists to keep (c) from sinking H1 on role-changers.

## 5. Phases & success criteria

Two invariants apply to every phase (not repeated): whole `pytest -q` green; work merges via
PR. Validation is a recurring gate.

### Phase 0 — Engine prereqs + deterministic gates *(no data dependency)*
**Objective:** lock the math and the testability foundation.
**Done when:**
- `kalman_two_component` returns the one-step predictive `(mean, S)`; unit-tested. **Also fix
  `_write_kalman_trajectory`** (weekly_update.py) — it persists a 2-component, R-omitted
  variance that the UX surfaces, so the calibration fix must reach the DB write path, not just
  the return value.
- `_ACCUM_PATH`/`_KALMAN_PATH` injectable — including the `KalmanState.save/load` classmethods
  (no path param today; signature change touches all callers). A test proves backtest mode
  never writes `data/cache/`.
- Determinism: two **in-process** synth runs match < 1e-9 (`OMP_NUM_THREADS=1`, seeds + versions
  pinned). *In-process only* — the cross-platform golden master relies on Layer-A/B invariants +
  Layer-C `rtol 1e-5`, not a `<1e-9` cross-platform gate (floats diverge ~1e-6 across OS/BLAS).
- Tier 0 recovery gates green; Tier 0.5 golden-master + synth-calibration gates green.

### Phase 0/1 (parallel) — Design Foundation
**Objective:** establish the dark-comic design system before page polish.
**Done when:**
- Tailwind `@theme` tokens, dark-comic palette, **position/tier semantic color system** (tables + Recharts).
- Core primitives (Button, Card, Table, TierChip, StatCell, badge) + styled empty/loading/error states.
- **Rankings reskinned as the visual benchmark** matching the approved mockup.

### Phase 1 — Real data foundation + nflverse→GRID adapter *(L–XL; bigger than first scoped)*
**Objective:** real nflverse data flows through **both** contracts end-to-end — the baseline
stats path *and* the GRID `plays` contract.
**Done when:**
- `data_pipeline` ingests **2019–2025** idempotently; per-season row counts within expected bounds; no schema-drift crashes (`_normalize_pbp` verified per year).
- **nflverse→GRID `plays`-contract adapter (§4.4, the long pole)**: drive segmentation,
  next-state derivation, next-score (`drive_points`) labeling, and the participation join —
  producing the full `data_adapters.py` contract from real nflverse PBP. `fit_value_model`,
  `compute_dv`, and `build_design`/`run_rapm` all **execute on real plays** (today they
  `KeyError`).
- `load_participation` implemented; `off_players`/`def_players` populated on **≥99%** of run/pass plays.
- `compute_valuations` writes a real baseline `valuations` table; **encoded sniff test** passes (known-player fixture with rank bands — §6.5 #9).
- Data snapshot frozen to Parquet for reproducibility.
- **Baseline-measurement task** run: fills the provisional-number registry (§6.5 #8).
- *Gate:* Tier 0/0.5 still green; Tier 1/2 executable on real data (report-only OK).

### Phase 2 — GRID projection + headline validation *(the convergence; LONG POLE, ~40% of effort)*
Split into three sub-phases so the largest phase is manageable and the ship-GRID-vs-baseline
verdict is front-and-center. See §4.5 for the projection architecture.

**Phase 2a — Validation infrastructure** *(the trust layer; build first)*
**Done when:** `asof.py` accessor + cache namespace; **all five leakage guards + canaries
green** (poisoning, tripwire, watermark, two-path equivalence); `baselines.py`
(persistence/season-mean/last-season/market); `metrics.py` (skill, CRPS/PIT/coverage/pinball/
NIS, NDCG/top-N); `backtest.py` walk-forward driver. Every later backtest is leakage-safe by
construction. **Perf requirement (§7.5):** the driver must use **incremental RAPM
accumulators + a frozen pre-period V(s)** so the full weekly walk-forward stays in minutes,
not hours.

**Phase 2b — Projection + volume models + weekly skill-position signal** *(the core algorithm; grew post-verification)*
**Done when:** **fitted stat-line projection model** (§4.5) — GRID talent (RAPM + Kalman
smoothed + prior) + volume/role features, fit on historical seasons, scored per format via the
existing engine; `fantasy_scoring.py` magic constants replaced. **Volume/role model**
(usage regression + ADP/depth-chart override hook). **Per-position weekly Layer-1 credit +
SV→fantasy-points map** (new scope, §4.5) so weekly RB/WR/TE projections exist for H2.
**Preseason path** produces draft-time rankings from smoothed end-of-prior-season talent +
RAPM + rookie priors — including **wiring `priors.py` into product rankings** (net-new; today
synth-only). `compute_valuations` sources projections from the new model.

**Phase 2c — Headline run + verdict** *(gated on 2a+2b)*
**Done when:** **H1 and H2 measured** with CIs, per position; Tier-1 calibration report (ROS
PICP, CRPS skill, real NIS); Tier-2 real matchup-grade predictive corr. **Gate-promotion rule
committed (not open-ended):** the *single* first full backtest pass sets each directional KPI's
threshold at `baseline + 1.96·SE(baseline)` (the measured noise floor); thresholds are then
frozen and gating — no re-tuning to clear. **The verdict is rendered and acted on:** H1 clears
the kill criterion → ship GRID-powered; else documented fallback to baseline for alpha.

> Success across Phase 2 is **a trustworthy, leakage-free answer + the product reflecting it** —
> *not* "GRID wins." **H1 (beat last-season-actuals) is a *floor*, not proof of product value** —
> last-season-actuals is a weak fantasy baseline, so clearing it means "not broken," not "good."
> Beating **season-to-date-mean and market** (the harder baselines) is the real value signal.
> Defining success as "GRID wins" would create pressure to defeat the leakage guards.

### Phase 3 — App on real data (+ pre-alpha demo)
**Objective:** Tier-A core loop works end-to-end on real GRID-powered data.
**Done when:**
- Rankings, Draft Room (mock), Workbench, health Dashboard render real data; **no empty-states in the core loop**; app runs with **viz agent off and no `ANTHROPIC_API_KEY`**.
- Manual QA: open Rankings → change scoring format → sensible GRID rankings; mock draft start→pick→grade; explore Workbench — no errors.
- Design system applied to all Tier-A pages; styled states; responsive/mobile pass. **Visual-consistency review passes — no default-Tailwind screens in the core loop.**
- **Credits/Data Sources surface** crediting nflverse + FTN Data (CC-BY-SA) present (§6.5 #11).
- `tsc -b`, `npm run build`, lint **green**; backend serves the built `dist/` in dev.
- *Milestone:* **pre-alpha demo is droppable** (runs locally on real data).

### Phase 4 — Packaging + first-run (Windows)
**Objective:** installable on a clean Windows box.
**Done when:**
- conda-`constructor` builds an offline `.exe` installer; **installs + runs on a clean Windows machine** (no preinstalled Python/Node).
- **pywebview window** opens via launcher/tray; uvicorn on a background thread, webview on main, clean shutdown on window-close/tray-quit.
- **Zero required config for Tier A;** first run **ingests on first launch** behind a styled loading/onboarding state; health shows "data last updated."
- **App launch (serve only) < ~10–15s**, distinct from **first-run data setup** (network-bound minutes, behind the onboarding UI) — see §7.5 budget C. Disk < ~2GB documented.
- **Validation suite green *in the packaged environment*.**

### Phase 5 — Alpha hardening + ship
**Objective:** real-user-ready alpha.
**Done when:**
- **A friend installs it on their own Windows machine and completes the core-loop acceptance script unaided** (§6.5 #10; ≤ one-page README).
- **Upgrade test:** installing over a prior version preserves user data and auto-migrates (§6.5 #4).
- Graceful error/empty states (no white-screens); logging + health surfaced; bug-report path.
- **Looks intentional & coherent** on a real user's screen (final aesthetic QA: favicon/app-icon, launcher/window branding, microcopy).
- Known-issues doc; Tier-B labeled "coming." Tagged `v0.1-alpha`.

## 6. Post-alpha tracks (named, non-blocking)

1. **Tier-B in-season + league sync** (pre-Week-1): harden ESPN/Sleeper adapters, add live-draft
   sync + In-Season hub (Start/Sit, Waiver, Trades), leagues Dashboard. Folds in the **real-roster
   H2 update** (validation plan §14) — same Sept workstream. Brings cookie/ID config + first-run
   league setup.
2. **Tauri migration + macOS** (beta): native shell reusing the React UI; macOS from one codebase.
3. **Weekly in-season participation research** (validation plan §14): a per-play participation
   source on a weekly cadence to enable *live* in-season RAPM (currently yearly via FTN).

## 6.5 Cross-cutting requirements (gap closure: #4, #8, #9, #10, #11)

**#4 — App upgrade / data migration.** The shipped app has a long-lived local SQLite + engine
caches that must survive version bumps.
- **User data lives outside the install dir** (e.g. `%LOCALAPPDATA%\GRIDFantasy\`) so a
  conda-constructor reinstall/upgrade never wipes the DB/cache.
- **Schema versioning:** SQLite `PRAGMA user_version` + an ordered list of idempotent
  migrations run at startup (extends the existing `init_db` guarded-`CREATE/ALTER` pattern).
- **Caches are reproducible, not migrated:** on an engine-format bump, invalidate + rebuild
  `data/cache/*.npz|parquet` (the `state_version` mechanism already exists for Kalman) rather
  than migrate them. **Resolves the offline tension:** rebuild re-derives from the
  **locally-cached raw nflverse parquet** (kept on disk after first ingest) — *offline*, no
  re-download. Only a *data refresh* (new season/week) needs network. So an engine-bump upgrade
  stays offline; it does not silently re-download ~0.5–1GB.
- *Done-when (Phase 4):* installing v(N+1) over vN preserves user data and auto-migrates.

**#8 — Provisional-number registry + baseline-measurement task.** Every "defined bound" in this
doc and the validation plan is **provisional-until-measured** (calibrate-then-gate). Add an
explicit **baseline-measurement task at the end of Phase 1** that runs the first real ingest +
report and **fills a single registry** of: per-season row-count bounds (#1.x sniff), Tier-0
per-pos/equivalency thresholds, H2 bootstrap N for CI-excludes-0, backtest wall-time, app
cold-start, artifact size, disk/RAM. Until filled, these stay report-only; promotion to gates
happens against the measured noise floor.

**#9 — Encode the "elites rank near top" sniff test.** Replace the eyeball check with a
**known-player fixture**: a small hand-curated set of consensus elites per position/era with
expected **rank bands** (e.g. "≥3 of {set} in top-5 overall per format; none outside top-24").
Automatable as a soft Phase-1 gate; catches gross projection/scoring breakage.

**#10 — Core-loop acceptance script.** Phase 5's "completes the core loop unaided" is measured
against a concrete journey: **install → launch → first-run ingest completes → open Rankings →
switch scoring format → start mock draft → make a pick → view draft grade → open Workbench →
build a saved view.** Each step has a pass/fail. This *is* the alpha acceptance test.

**#11 — Data attribution (CC-BY-SA, required).** nflverse data is CC-BY-SA 4.0; FTN
participation (2023+) must credit **"FTN Data via nflverse."**
- *Done-when (Phase 3):* an in-app **Credits/Data Sources** surface (footer or About) crediting
  nflverse + FTN Data with the CC-BY-SA notice; a data-sources/licensing section in the README.
- Cheap and obligatory — not optional polish.

## 7. Risks

- **The nflverse→GRID adapter (§4.4) is the true long pole and was nearly invisible.** Drive
  segmentation + next-score labeling + participation join is L–XL, gates all of Phase 2, and
  exists only in `synth.py` today. Mis-scoping it as "plumbing" was the biggest planning error
  the verification pass caught; the timeline (§9) now reflects its real size.
- **The two-path leakage guard isn't "free"** — it depends on `weekly_update` running on real
  data, which is blocked until the §4.4 adapter exists.
- **nflverse schema drift** — first real-data contact (Phase 1); front-load it.
- **GRID may fail H1** — the kill criterion can fire. Validation surfaces this *early* (Phase 2);
  honest fallback is ship-baseline-for-alpha. This is a feature of the plan, not a failure.
- **Volume/role is the dominant driver and GRID doesn't model it** (§4.5). If the volume model
  is too naive, GRID's efficiency edge may not beat last-season-actuals on role-changers — the
  concentrated H1 risk. The volume workstream + ADP/depth-chart override hook is the mitigation.
- **Phase 2 is the long pole** (projection model + volume model + full backtest harness +
  leakage guards, all gated behind Phase 1). Size the plan around this; it is *not* a swap.
- **Packaging Python on Windows** — conda-constructor + pywebview threading/shutdown is fiddly;
  budget real Phase-4 time. Fallback: browser + launcher + tray helper.
- **In-season RAPM staleness** — draft-time RAPM (prior-season participation) doesn't refresh
  weekly; must be presented coherently alongside the weekly-updating Kalman (Tier-B UX item).
- **Design scope creep** — mitigated by "right-sized + time-boxed."

## 7.5 Performance & compute budget

Three *distinct* budgets — conflating them is the trap:

**A. CI budget (synth gates, every PR).** Tier 0/0.5 + synth-calibration + leakage canaries run
on the small synth fixture, deterministic. Target **full validation CI gate < ~60s**. This is
the "runs many times" surface; it stays cheap because it's synth.

**B. Real-data backtest budget (dev/nightly job, NOT per-PR).** Full 7-season **weekly**
walk-forward (H1/H2/calibration/Tier-2). Naive cost = origins × (rebuild design + RAPM solve +
**GBM refit** + Kalman); the GBM refits dominate. Target **< ~10 min on a dev machine**, which
is a **design requirement on Phase 2a**, achieved by:
- **Incremental RAPM accumulators** (`accumulate`/`fit_from_accumulators`) — add a week, don't
  rebuild per origin (also yields the two-path-equivalence leakage guard for free);
- **Freeze V(s) to a pre-period fit** — avoids ~126 GBM refits (defensible; V is slow-moving);
- cheap per-player Kalman; **sampled H2 bootstrap with logged caps** (no silent truncation);
- fast-iteration mode (subset of origins) for dev, full run only for the verdict.
*If the harness does naive refit-per-origin instead, B is hours, not minutes.*

**C. Shipped-app budget (no backtests).**
- **Launch (serve only) < ~10–15s** — and **launch ≠ data-ready**;
- **First-run data setup = network-bound minutes** (download ~0.5–1GB + one normalize/RAPM/
  Kalman/projection pass) **behind the onboarding/progress UI**, explicitly *not* counted in launch;
- **weekly/incremental update fast** (seconds–low-minutes — the incremental engine's purpose);
- **disk footprint < ~2GB** (frozen cache + SQLite); **ingest peak RAM ~1–2GB** — document both.

All numbers are **provisional-until-measured** (calibrate-then-gate); the first real backtest
sets the actual ceilings. Reconciliation of "validation runs many times": the *frequent* gate
is the fast synth one (A); the slow real backtest (B) runs at phase boundaries / nightly on the
frozen snapshot.

## 8. Sequencing summary

```
Phase 0  (engine prereqs + Tier 0/0.5)   ──┐  parallel: Design Foundation
Phase 1  (real data + nflverse→GRID adapter §4.4 — co-long-pole)
Phase 2a (validation infra + leakage)       │  validation gate each boundary
Phase 2b (projection + volume + weekly signal)
Phase 2c (H1/H2 run + verdict)              │
Phase 3  (app on real data + DEMO) ........ │ ◄── soft goal: demo ≈ Sept (late-Aug now a stretch)
Phase 4  (conda-constructor + pywebview)    │
Phase 5  (alpha hardening + v0.1-alpha)   ──┘ ◄── packaged alpha ≈ October
post-alpha: Tier-B league sync · Tauri+macOS · weekly-participation research
```

## 9. Sizing, interleave & critical path

**Sizing is effort-relative** (S ≈ 1–2 days · M ≈ 3–5 days · L ≈ 1–2 wks · XL ≈ 2–4 wks).
**Revised after the verification pass: ~115–135 IED total** (up from ~90–105) — the §4.4
nflverse→GRID adapter and the new weekly-skill-position signal (§4.5) were previously
unscoped/under-scoped. **Calendar read under the stated capacity** (solo + heavy agent use,
~10–13 IED/wk): the **pre-alpha demo is now a stretch for late-Aug** — more realistically
**demo ≈ September, packaged alpha ≈ October**. Per the GRID-or-bust / quality-over-date stance,
the date yields, not the rigor. Agent velocity on this stats codebase is lumpy, and the new
critical-path items (the adapter; the weekly Layer-1 credit) are exactly the integration/debugging
work that doesn't compress.

| Phase | Size | Notes |
|---|---|---|
| 0 — engine prereqs + Tier 0/0.5 | **L+** | golden master (3 layers); + Kalman-`S`/trajectory fix + cache-path/`KalmanState` refactor |
| Design Foundation (parallel) | **L** | only parallel with separate frontend capacity; else serial |
| **1 — real data + nflverse→GRID adapter** | **L–XL** | **co-long-pole**: drive/next-score/participation adapter (§4.4), not just schema-drift |
| **2 — projection + weekly signal + validation** | **XL+** | **the long pole; grew with weekly Layer-1 credit + SV→points map** |
| 3 — app on real data + polish + demo | **L+** | design application + wiring + responsive |
| 4 — packaging (conda-constructor + pywebview) | **L** | packaging Python on Windows is fiddly |
| 5 — hardening + ship | **M–L** | friend-install iteration always over-runs |

**Phase 2 is still the long pole, but Phase 1 is now a co-long-pole** — the §4.4 adapter gates
everything downstream, so the two largest, riskiest chunks are back-to-back on the critical path.

### The validation ⟷ product interleave (why "gating dependency" ≠ "extra phase")
The validation suite is **not** a separate effort bolted on — most of its components *are*
product components, which is why they fold into Phases 0–2:

| Validation component | Also serves | Phase |
|---|---|---|
| engine prereqs (predictive `S`, cache paths, determinism) | the product engine itself | 0 |
| Tier 0 / 0.5 | regression safety for all later changes | 0 |
| nflverse→GRID adapter (§4.4) | **pure product** — gates the whole engine path; *not* a validation freebie | 1 |
| `load_participation` | **the product's RAPM data unblock** | 1 |
| `backtest.py` walk-forward harness | **the projection model's train/eval harness** | 2 |
| Tier-1 ROS target model | **literally the stat-line projection model** | 2 |
| `lineup_sim.py` (H2) | reusable for in-season start/sit (Tier-B) | 2 |
| leakage guards · metrics · baselines | the trust layer the H1 verdict rests on | 2 |

**The only pure-validation tax** beyond what the product needs anyway is the leakage guards,
Tier 0/0.5, and the metrics/report scaffolding (~2–3 wks of Phase 2's XL). Everything else does
double duty — building validation *is* building the product.

### Long pole & critical path
- **Critical path:** 0 → 1 → 2 → 3 → 4 → 5 (strictly serial under the GRID-or-bust stance,
  since the demo and any usable app sit behind Phase 2).
- **Long poles (two, back-to-back):** the §4.4 nflverse→GRID adapter (Phase 1) and Phase 2
  (split 2a→2b→2c). De-risking the timeline = de-risking these two. Spike the adapter first.
- **Phase 2 internal long poles:** the stat-line projection model (2b, L–XL) and the
  leakage-guard harness (2a, L). **2a is built first** so every later backtest is leakage-safe by
  construction; **2c is gated on 2a+2b** and front-loads the ship-GRID-vs-baseline verdict.

### Considered & rejected: baseline-interim decoupling
An alternative was to ship a **baseline-powered** app early (Phases 0→1→3-lite→4) for the draft,
then upgrade projections in place once Phase 2 lands — giving an on-time draft-usable fallback
and an early demo. **Rejected** per the "GRID-powered or bust, let it slip" decision: the alpha
waits for GRID rather than shipping the baseline it's meant to beat. Recorded here so the option
is visible if the stance changes.
