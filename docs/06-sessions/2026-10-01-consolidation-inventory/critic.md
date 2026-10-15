# Completeness critic: what the eight inventory reports miss, where they disagree, and the owner decisions

**Author:** completeness-critic agent. **Date:** 2026-10-01.
**Reports reviewed (all read in full):** `alpha-spec.md`, `final-build-spec.md`, `bootstrap-infra.md`, `python-closure.md`,
`cn-docs.md`, `cn-issues.md`, `reconcile-code-first.md`, `reconcile-spec-first.md` (all under
`/tmp/claude-0/-home-user/693e74a1-f8af-5256-86e9-2299b8697223/scratchpad/inventory/`).
**Repos spot-checked (read-only, nothing modified):**
- `/home/user/GRID-Engine` @ `3823478`
- `/home/user/cautious-nevermore` (CN) @ `59bce1d`
- GitHub: `Seismic-Fate/cautious-nevermore` PRs #1–#96 and the review threads on #53–#94; `GRID-One/GRID-Engine` PRs #1–#3 and their comments; issues.

**My scratch:** `scratch-critic/`. It holds:
- `cn/`: a `git archive 59bce1d` copy;
- `cnfix/`: the same copy with the synth defender fix;
- `t0.py`: prints the Tier-0 values;
- `threads/`, `tc/`: the GitHub review-thread sweep.

Short names used below: **PC** python-closure, **CD** cn-docs, **CI** cn-issues, **CF** reconcile-code-first, **SF** reconcile-spec-first, **BI** bootstrap-infra, **AS** the alpha-spec report, **FB** the final-build-spec report.

---

## 0. Bottom line

1. **The synthetic oracle has a structural bug that only SF found.** It undercuts parts of four other reports. At
   `backend/grid/synth.py:193` the code is `off_pl, def_pl = _pick_onfield(tidx[off_team], rng)`, so every play's "defenders" are the **offense's own** DEF players.
   - I re-verified this. On canonical `load_synthetic()` (16,825 plays), `def_players` come from `off_team` on **100%** of plays and from `def_team` on **0%**.
   - Every team-strength, Layer-3, DEF-recovery and matchup-grade number in CI, CF and CD was measured against this generator. That includes the "[+1,−1] fixes G1" evidence (0.6644→0.7318 in CI; 0.682→0.739 in CF; 0.696→0.758 in CD).
   - Every parity-number table (PC §8, CF App. A.2, FB §5 Class D, CD §3.6) is a **legacy-generator** value.
   - I measured the effect of fixing it: **Tier-0 still passes 8/8, the golden master fails 4/11, and synth calibration fails 1/6** (pooled QB NIS 1.829 vs the band [0.8, 1.4]). So the QB `r_scale=0.55` calibration from PR #66 was tuned on the buggy world (§1, G-1).
2. **All of the inventory's evidence lives only in this session's scratchpad.** That covers the 8 reports, about 25 repro scripts, the patches and the simulated pivot branch. None of it is in either repo, and it will be lost when the session ends unless the consolidation PR commits it (§1, G-2).
3. **No report covers history before the shallow-clone boundary or the first engine audit.** Both CN clones are shallow (88 commits, oldest 2026-06-21). From GitHub:
   - the v0 engine upload is `6b0eeee`;
   - **PR #53** "GRID engine audit remediation (P0+P1)", findings C1–C7, W1, W7, is where the **`lstsq` fallback was added deliberately** (audit item C3), along with the Joseph form and the solve-based RTS;
   - a Phase-3 plan (`plans/2026-06-22-fantasy-dashboard-phase3-impl.md`, cited by PR #19) was **never committed** and is lost.

   FB/SF recommend replacing the `lstsq` fallback with a typed failure. That reverses a recorded audit decision, so the ADR should cite PR #53 (§1, G-3).
4. **`backend/db/data/coaching_changes_2025.json` contains wrong or implausible real-world rows,** and PC imports it verbatim as "default seed data". Examples:
   - 2024 hires labelled 2025;
   - a QB listed as an offensive coordinator;
   - Ben Johnson listed as CHI OC.

   It is never loaded by tests or by any automatic path, so no recorded number depends on it. It must not become a fixture or provider input (§1, G-4).
5. **Data licensing details that CD left open are now answered,** and they create a fixture-policy issue no report raised. nflverse participation is **CC-BY-SA 4.0**:
   - pre-2023 data comes from NFL NGS ("NFL NextGen Stats via nflverse");
   - 2023+ comes from FTN ("FTN Data via nflverse").

   The ShareAlike term means real-data-derived fixtures cannot simply be committed into an MIT/Apache tree (§1, G-5).
6. **The reports contradict each other in 21 places** (§2). The material ones:
   - whether to patch the Python oracle;
   - which Layer-3 / team-strength convention to adopt;
   - whether the matchup-grade sign is "inconclusive" or "inverted";
   - the import scope;
   - `typos` handling;
   - CI Python setup;
   - spec file disposition;
   - parity tolerances;
   - the Kalman observation semantics in FB;
   - how far the real-data verdict can be trusted.
7. **There are 33 owner decisions after de-duplication** (§3), each with a recommended default. Another 14 items have obvious defaults and need no owner time (§3.4).

---

## 1. Gaps: useful work or knowledge not captured by any report

Priority: **P0** blocks parity work or would cement a bug. **P1** would lose real knowledge. **P2** is hygiene or provenance.

### G-1 (P0). The synthetic generator draws defenders from the offense team

**What.**
- `cautious-nevermore/backend/grid/synth.py:193` is `off_pl, def_pl = _pick_onfield(tidx[off_team], rng)`. `_pick_onfield` (`:107-128`) returns both lists from one team index.
- `_team_strength` (`:285-294`) plants `mean(off starters) − mean(DEF starters)`. That is coherent *only* with this bug: a team's own defenders reduce its own offense's yards.
- The `def_team` intercept therefore never sees a planted signal.

**Coverage.** SF §1.4 and §4 C1 found it. CI §3.5 (G3, NEW-Y1, NEW-Y2), CF §1.2 and CD §14.7 all describe the synth without noticing it. They then draw sign and convention conclusions from it:
- CI recommends the `[+1,−1]` fix (0.6644→0.7318);
- CF builds the E_off/E_def evidence table on it;
- CD calls the `[+1,−1]` direction "modestly supported".

**Verification (mine).**
- `scratch-critic/cn`, `load_synthetic()`: `def_players ⊂ off_team` on 1.0 of plays, `⊂ def_team` on 0.0.
- `scratch-critic/cnfix` applies SF's one-line fix (two `_pick_onfield` calls). Gate results with `OMP/OPENBLAS/MKL_NUM_THREADS=1`:

| Suite (fixed synth) | Result |
|---|---|
| `tests/grid/test_tier0_recovery.py` | 8/8 pass |
| `tests/grid/test_determinism.py` | 2/2 pass |
| `tests/grid/test_golden_master.py` | **4 fail**: `layerB_top_player_ranking`, `layerB_team_ranking_order`, `layerC_player_and_team_ratings`, `layerC_qb_weekly_and_kalman` (qb_credit Δ up to 0.317) |
| `tests/grid/test_calibration_synth.py` | **1 fail**: `test_pooled_nis_bounded`, pooled NIS **1.829** (band [0.8, 1.4]) |

Tier-0 values, legacy generator vs fixed generator (same `fit(n_iter=3)`, market seed 1):

| Metric | Legacy (current gate observed) | Defender-fixed |
|---|---|---|
| pooled attribution | 0.8025 | 0.8276 |
| QB / RB / WR / TE / DEF | .869 / .7433 / .7973 / .7713 / .7653 | .8845 / .7711 / .8599 / .7784 / .7827 |
| team corr (vs the **legacy** off−def target, now semantically wrong) | 0.6643 | 0.6596 |
| Kalman total_smooth / tau_smooth | 0.958 / 0.6745 | 0.976 / 0.6538 |
| focus-QB NIS (gate ≤ 10) | 4.581 | **8.371** |
| equivalency slope / OOS R² | 1.3159 / 0.1489 | 1.2119 / 0.2134 |
| rookie prior corr | 0.5829 | 0.5829 |

**Why it matters.**
- (a) Every parity table in PC §8, CF App. A.2, FB §5 and CD §3.6 must be relabelled **"legacy synth (defender bug)"**. None should be frozen as a Rust target.
- (b) The G1 `[+1,−1]` evidence in CI/CF/CD is moot; see §2 X-2.
- (c) The QB `r_scale` 0.35→0.55 calibration (PR #66) and the NIS bands are tuned on the buggy world.
- (d) After the fix, focus-QB NIS of 8.37 sits close to the ≤ 10 guard.
- (e) The fix changes the RNG stream, because `_pick_onfield` is called twice. The corrected generator therefore needs its own planted-strength definition and a new golden. A one-line patch is not enough.

**Destination.**
- Correction #1 in the oracle-correction ledger (`reference/python/PARITY.md`, see §2 X-4), applied **before** any other synth-dependent correction: G1/NEW-A1, the NEW-A2 grade sign, #15 init.
- A model spec `docs/05-model-specs/synthetic-world.md` that defines the planted net strength and the role of the defender pool.
- A statistical-owner ADR.

### G-2 (P0). The inventory evidence is ephemeral and must be committed

None of the reports recommends preserving the reports themselves or their repro scripts. Everything below sits under `/tmp/claude-0/.../scratchpad/inventory/`:

| Artifact | What it proves | Recommended destination |
|---|---|---|
| The 8 reports plus `critic.md` | The consolidation analysis that downstream WPs build on | `docs/06-sessions/2026-10-01-consolidation-inventory/` (verbatim, with a provenance header; `docs/06-sessions/` is the canonical review/session location per `authority-index.md:45`) |
| `scratch-cnissues/{cmp_stats,real_contract,real_rapm}.py` | NEW-I1..I5 ingest bias (sacks, return TDs, fumbles −32%, postseason rows); real RAPM SD ≈ 0.04 at every position (NEW-A3/P4) | `reference/python/tools/investigations/` plus a `fetch_realdata.py` that pins URLs and sha256. **Do not commit the parquet files** (G-5). The 2023 inputs used: `pbp_2023` `bd348473…6776`, `part_2023` `b1577369…e5a6`, `roster_2023` `66dcb7d0…3c90`, `player_stats_2023` `94673091…c9`, `stats_player_week_2023` `ac776fbd…bc` |
| `scratch-cnissues/{rollover_test,reinit_test,kparity,psd}.py` | NEW-W1, NEW-W2, NEW-S1, G4 | same `tools/investigations/` |
| `scratch-cnissues/{sign_test,sign_test2,defsign_planted,defgrade_test,defgrade_test2}.py`; `scratch-reconcile-code-first/sign_check.py`; `scratch-specfirst/{sign_exp,sign_exp2,def_sign}.py` | Layer-3 and grade sign evidence. `defsign_planted.py` uses its own correct generator and stays valid. The rest are on the legacy synth (G-1) | same, tagged legacy vs fixed |
| `scratch-critic/{t0.py,cnfix}` | The G-1 impact table | same |
| `scratch-python/{lineup_sim.patch,run_demo.patch,requirements*.txt,requirements.lock,pytest.ini,manifest_files.txt,identity.tsv,imports.py,thirdparty.py,plugins/isolation_guard.py}` | Import closure, the 446-green proof and app-isolation enforcement | PC's manifest already covers the patches and requirements. **Add `isolation_guard.py`**, which PC's "new files" list omits, as `reference/python/tools/pytest_isolation_guard.py`, enabled through `pytest.ini` `addopts = -p tools.pytest_isolation_guard` so the "no app or network imports" property is enforced rather than one-off |
| `scratch-infra/ge` branch `sim/engine-pivot`; the `check-evidence-claims.sh` generalisation; the R4-1 `scan_files()` fix | Pivot feasibility, 54/54 guards and parity 15 | Apply in the pivot PR (BI §9 sequencing); keep the diffs in the pivot WP evidence |
| `scratch-cndocs/orig/*` (8 deleted docs, the whiteboard pre-clear, 7 Phase-4 SDD reports) and `engine-pr-dedup.txt` | The deleted engine knowledge | `docs/07-archive/cautious-nevermore/` (§3 decision A-9); CD §15 lists which files |

### G-3 (P1). Pre-boundary history, the first engine audit and the lost Phase-3 plan

Both local CN clones are shallow (`git rev-parse --is-shallow-repository` = true; 88 commits on main; oldest visible `f3b641f`, 2026-06-21). CI noted the shallow boundary but no report went to GitHub for what lies before it.

| Item | Where it lives | Engine knowledge | Recommended destination |
|---|---|---|---|
| `6b0eeee` "initial: existing GRID engine files" (2026-06-21) | GitHub only | The **v0 engine** at repo root (layers.py 201 lines, statespace.py 135, priors.py 111, synth.py 316, value.py 76, data_adapters.py 77, run_demo.py 148), the 2 PNGs, the original design spec and the phase-1 plan. Moved into `backend/grid/` by `87e227f` (renames, so PC's "no engine file ever deleted" holds substantively). `run_demo.py`'s `/mnt/user-data/outputs` shows v0 was written in a Claude.ai sandbox; any originating design notes are not in any repo | Record both SHAs in `reference/python/README.md` provenance. Owner question: does an originating GRID design conversation or doc exist outside git? If so, archive it |
| **PR #53** "fix: GRID engine audit remediation (P0+P1)" (2026-06-22) | PR body only; the audit document was never committed | C1 Joseph-form covariance update; C2 solve-based RTS; **C3 conditioning check + `lstsq` fallback (deliberate)**; C4 unknown-ID guard in `build_design`; C7 interventions wired through `kalman_step`; W1 FLEX-aware VOR; W7 `SSParams.from_position`. "P2/P3 items tracked as GitHub issues #42–#52." This is a **different, earlier audit** from the 2026-07-13 G/P/V/A/W audit in `docs/06-issues-log.md` that CI triaged | Engine history (`docs/07-archive/cautious-nevermore/HISTORY.md`). The ADR that makes ill-conditioning a typed failure (FB §5 item 1, SF R33, CF C11) must cite PR #53 C3 as the decision it reverses |
| Phase-3 plan `plans/2026-06-22-fantasy-dashboard-phase3-impl.md` | **Never committed**; cited in the PR #19 body | Origin of the accumulators, `kalman_step`, `weekly_update`, `lambda_by_pos` ("softer regularization for QB (more volatile) vs RB"), `layer1_all_qbs`, and the priors expansion (`LEAGUE_FACTORS` FBS .35 / FCS .20 / UFL·USFL·XFL .15 / CFL .18; age ±0.05; draft +0.08 / +0.03 / −0.03). Only PR bodies #8–#11, #13, #18, #19 survive | HISTORY.md, noting the plan is unrecoverable and its constants are undocumented hand-set values (consistent with CI NEW-P4) |
| **PR #56** body (Phase 4) | PR body | The original matchup-grade convention decision ("tougher defense → higher grade", sign-flipped `t_def`), which is the convention now shown to be inverted. The `.npz` "corrupt → reinit" behaviour as a deliberate design. The "honest limitations" list | HISTORY.md, plus the divergence ledger entry for NEW-A2 and the KalmanState-load typed failure |
| PR descriptions #63–#91, #94 | GitHub | Spot-checked #88 and #91. They are close to the squash bodies CD extracted. Extra facts: #88 "renders, never acts" (flipping `compute_valuations` is a user decision) and the recommended real-run years 2019–2024 (the actual run used 2022–2023); #91 says duplicate `player_id`s perturbed results at about 1e-6 and that the smoothed-talent GBM path is "more numerically sensitive" | CD's `engine-pr-dedup.txt` is an adequate source; add these two facts to HISTORY.md |
| CodeRabbit review threads on #63–#89 | GitHub | Sweep: **46 threads, all resolved**, 45 marked "Addressed in commit". **No decision lives only in review threads.** The one not marked addressed (#85 "Handle FLEX-only positions in the VOR ranking") is the A6 fix CI already dates to #85 | None needed; recorded here so nobody repeats the sweep |

### G-4 (P1). `coaching_changes_2025.json` is not trustworthy real-world data

`backend/db/data/coaching_changes_2025.json` has 14 rows. PC imports it verbatim (manifest #4), CD calls it "hand-curated", and CI uses it for the scheme_fit argument. Several rows are wrong or implausible, judged from general knowledge (to be confirmed by the Data owner against a sourced list):
- `TEN HC Vrabel→Callahan`, `CAR HC Reich→Canales`, `WAS DC Del Rio→Whitt` and `LAR OC LaFleur→Robinson` are 2024-cycle changes labelled 2025;
- `CHI OC Getsy→Ben Johnson` (Johnson was hired as HC);
- `DAL OC Schottenheimer→Kellen Moore` (Schottenheimer became DAL HC);
- the mid-season rows `ATL OC → Kirk Cousins` (a quarterback) and `HOU DC → Duce Staley` are implausible.

**Impact today: none.** The default JSON is loaded only by an explicit `seed_coaching_changes()` call. Tests pass explicit rows (`tests/grid/test_coaching_changes.py:304`), and no pipeline path seeds it automatically.

**Recommendation.**
- Keep the file verbatim in the oracle, with a README warning: "illustrative, unverified; never a fixture or provider input".
- In Rust, coaching changes become a provider contract (`docs/04-providers/coaching-changes/`) with sourced, dated records. That ties into the AsOf intervention-foreknowledge rule (CD §3.7).

### G-5 (P1). Data licensing specifics and the fixture policy

CD §7 asserted CC-BY-SA, asked for verification, and noted that "2016–2022 participation provenance is not documented in CN". Checked against nflreadr docs on 2026-10-01:
- `load_participation`: "released under the CC-BY-SA 4.0"; attribution **"NFL NextGen Stats via nflverse" for 2022 and earlier**, **"FTN Data via nflverse" for 2023 onwards**; "provided after all post-season games are completed".
- `load_ftn_charting`: CC-BY-SA 4.0, "FTN Data via nflverse".
- The nflverse-data repo shows CC-BY-4.0 at repo level; per-dataset terms for PBP and rosters were not stated on the PBP page.

**Gap no report raised:** ShareAlike. Real-data-derived fixtures (participation, or plays-contract frames built from it) committed into an `MIT OR Apache-2.0` tree would need license segregation. GRID-Engine alpha-spec §4.6 already requires fixtures from licensed material to be "sanitized and reviewed" (`alpha-spec.md` near line 446).

**Recommended defaults.**
- (a) Parity fixtures are **synthetic-only**.
- (b) Real-data fixtures, if any, go under `fixtures/third-party/<provider>/` with a LICENSE/NOTICE carrying the attribution, after a Data/Licensing ruling.
- (c) Record all of this in `docs/04-providers/nflverse/access-and-license.md`, which is required before P1-03 (`docs/04-providers/nflverse/README.md`).

Separately, CN has no LICENSE (BI §7 ADR-001 row). The GRID-Engine owner's existing licensing ruling lives on **GRID-Engine PR #1 comment 5357318508** ("2565acc change to permissive MIT/Apache …"). It is mirrored in `.ai/evidence/P1-00/verification-input.json:192,275`. The analogous ruling for `reference/python/` should be recorded the same way, on the pivot PR, before import.

### G-6 (P1). The oracle CI must be Linux regardless of the Windows-gate decision

- CD §0.6 and CI §1 establish that the golden Layer C and `test_cache.py::test_ttl_expired` **fail on Windows** and pass on Linux, the declared "platform of record" (val-suite §6).
- BI §10.1 and FB D4 leave Windows authoritative pending a decision. PC and BI design a Python CI job without pinning its OS.
- **Requirement:** the oracle job runs on `ubuntu-*` only and is never wired into `verify.ps1` / `windows-authoritative`. If a smoke subset is ever added to the frozen verify chain (BI §6), it must skip Layer C on Windows, which makes it weaker. That is another reason to keep it as a separate Linux job.
- `tests/grid/test_performance.py:187-198` asserts a **wall-clock speedup ≥ 1.5×**. It is flake-prone under CI CPU contention; PC saw OpenMP oversubscription slow things more than 10×. Run with the threads pinned to 1 and, if it flakes, mark it as a non-gating perf check in `PARITY.md`.
- Timing data: the 27 synth gate tests (Tier-0, golden, calibration, determinism) ran in **20.9 s** in my scratch with threads=1. The full 446 ran in about 144–149 s (PC). BI's "tens of minutes" was contention (§2 X-9).

### G-7 (P2). Real-data results were never committed anywhere

The verdict reports (`data/reports/phase2c_verdict.{json,md}`) and the per-run frozen gates (`h1_ros` +0.0564, `h2` +0.3164) were gitignored by design (PR #88). They survive only as transcriptions in the deleted phase-2c doc (`165ccde:docs/superpowers/plans/2026-07-09-phase2c-verdict-and-ros-gap.md`) and in CD §4. The real 2023 measurements from CI (ingest bias table, RAPM scale, postseason row counts) exist only in CI's report.

**Destination:** `docs/07-archive/cautious-nevermore/real-data-results.md`, explicitly labelled *historical, non-parity*. The labelling should note three reasons:
- the ingest bias (CI NEW-I1..I4);
- the iid CIs (CI NEW-V1);
- the current-season participation skew (CF C1, SF R14).

### G-8 (P2). Operational facts nobody recorded

- `scripts/run_pipeline.bat` and `setup_scheduler.ps1` (CN) run `data_pipeline → compute_valuations → sync_leagues` **4× per day** (06:00/12:00/18:00/00:00). They **never run `ingest_grid` or `weekly_update`**, so the GRID weekly path was never operationalised. The cadence also contradicts alpha-spec §1.1 "at most once per local calendar day". This is history only, with no port (PC and BI correctly exclude both scripts).
- Engine environment variables: the only one read by engine code is `DB_PATH` (`backend/db/connection.py:8`). This confirms PC. `.env.example` keys `ESPN_LEAGUE_ID` / `SLEEPER_LEAGUE_ID_1/2` don't even match what `sync_leagues.py:251-258` reads (`ESPN_LEAGUE_IDS`, `SLEEPER_LEAGUE_IDS`). That is app-only and irrelevant to the engine. Add `GRID_DEMO_OUT` (PC patch P2) and `OMP/OPENBLAS/MKL_NUM_THREADS` to the reference README.
- CN's working tree has ignored-only artefacts (`__pycache__/`, `data/logs/pipeline.log` timestamped 02:17) from an in-place run earlier in this session. `git status` shows no tracked change. This is hygiene only.

### G-9 (P2). Smaller items worth one line each in the destination docs

- **Whiteboard pre-clear** (CD §1 row 7b): `c33712e` is not reachable from `main` (verified). It is still recoverable through GitHub `refs/pull/92/head` (head `5f282cb` descends from it), so "at risk if the branch is deleted" overstates the risk. Archive it anyway.
- **GitHub repo rename:** evidence and CI links in `.ai/evidence/P1-00/*` and the PR #1 comments point to `Seismic-Fate/GRID-Alpha` actions runs. Keep them as-is (immutable record). BI already flags `ai-toolchain.lock` / `Cargo.toml` repository fields.
- **Washout table** (`priors.washout_table`): the "games to 50%" figures depend on an assumed per-game observation variance. `run_demo.py:91` hard-codes `0.12**2`, which is inconsistent with `SSParams` R = r_scale/snaps. It is not a parity target (see §2 X-18).

---

## 2. Inter-report contradictions and adjudication

| # | Topic | Side A | Side B | Adjudication (and what supports it) |
|---|---|---|---|---|
| **X-1** | Synth defender bug | SF §4 C1: defenders drawn from the offense team | CI, CF, CD: silent; analyses assume a correct generator | **SF is right.** `synth.py:193` verified empirically (100%/0%). Everything in X-2/X-3 measured on legacy synth is provisional. See G-1 |
| **X-2** | Layer-3 market row / team-strength convention | CI G1: apply `[+1,−1]` in all 3 sites (`layers.py:404-405`, `backtest.py:108-109`, `weekly_update.py:272-273`), plus owner decision NEW-A1 (option A: net strength, keep `[+1,+1]`). CD: `[+1,−1]` "matches synth semantics"; CN docs `04-lessons-learned`/`10-next-steps` queue `[+1,−1]` | SF D1: keep `[+1,+1]`, set `team_rating = γ_off+γ_def`, fix synth, reject `[+1,−1]`. CF C12: gauge-invariant `E_off+E_def` (intercept plus exposure-weighted on-field rating sums), with the market anchoring net strength | **Reject `[+1,−1]`.** It only aligns the code with the buggy generator. The algebra (defenders enter X at −1, so a better defense has larger β_def) means a spread prices **net = off + def quality**. Combine SF and CF: planted truth = net quality on the fixed synth; the reported team strength is CF's gauge-invariant aggregate (intercept-only ratings are weakly identified; CF measured 0.46–0.74 vs 0.82–0.92 for aggregates, on legacy synth, so re-measure); the market rows anchor that net quantity **as of lock** (AS §7 item 10). CI's option A is the same estimand. Statistical-owner decision B-5 |
| **X-3** | Matchup-grade sign | CD §0.4: "inconclusive" (corr −0.22) | CI NEW-A2: CONFIRMED inverted (independent planted-defense generator, `defsign_planted.py`); SF C2: inverted (+0.41/+0.59 corr of −β_def with points allowed); CF C13: comment reasoning wrong | **Inverted.** CD's −0.22 came from the buggy synth. CI's evidence uses a generator with correct defenders and stays valid; SF's fixed-synth result (+0.53 corr β_def vs planted) agrees. The obvious fix is `+β_def` (or `+E_def`), with a truth-anchored Layer-A test on the fixed synth. The current unit tests (`tests/pipeline/test_weekly_update.py:286-370`) and Tier-2 synth test (`tests/validation/test_verdict.py:47-51`) are tautological and must be rewritten |
| **X-4** | Oracle patching policy | PC: verbatim import plus P1 (+P2) only. CF §7.1(5): "the oracle keeps its old behaviour", fix in Rust behind model specs, parity against an oracle-patched variant "where needed" | SF §6.1: tag as-imported, then correct the oracle in Python first ("do not port known bugs and then fix in Rust"). CI §4: import as legacy, then reviewed oracle-fix commits, keep both goldens. FB §5: parity on the healthy path only, divergences in `PARITY.md`. AS App D #13: prohibit changing the oracle "to make a Rust parity test pass" | **These are compatible once sequenced.** (1) Verbatim import commit (PC manifest), tagged `oracle-legacy-59bce1d`. (2) Statistical-owner-approved correction commits in Python, each with a failing test first and a golden regeneration with a model-spec note: synth defenders → team-strength convention → grade sign → Kalman x0 look-ahead (#15/C16) → ingest bias (NEW-I1..I4, NEW-V0a). (3) Rust targets the **corrected** oracle; the legacy golden is kept for audit only. (4) Typed-failure divergences (lstsq, corrupt state, skip-on-failure) are listed in `PARITY.md`. AS #13 forbids changing the oracle *to make Rust pass*, not pre-approved corrections. CF's "oracle keeps old behaviour" is rejected, because with G-1 an uncorrected oracle gives meaningless team/DEF parity |
| **X-5** | `reference/python/` scope | PC: 105 files including `backend/db` (full `schema.sql` with app tables), `pipeline/{data_pipeline,compute_valuations,weekly_update,ingest_grid,health_check,_logging}`, `scoring/{vor,format_registry}`, `db/seed_coaching` | CD §16: `format_registry` is app; `weekly_update` is DB-coupled. CF §7.1: import grid/projection/validation/scoring{engine,columns,formats,vor}, vendor `snake_order` and `_logging`, and **exclude/adapt** `test_two_path_equivalence`, `test_coaching_changes`, `test_verdict` and a `test_changepoint` case. BI §4: illustrative subset (not import-closed) | **PC wins for the oracle.** It is the only manifest proven import-closed and 446-green in a clean venv with app imports blocked. CF's exclusions would drop the two-path leakage guard, which SF §6.5 calls "the best part of the reference". CF and CD describe the **Rust port** scope (VOR/format_registry/compute_valuations/data_pipeline not ported), which is a separate question. Write both lists into ADR-011: oracle scope = PC; port scope = CF §7.2 |
| **X-6** | `typos` | PC §7: add `"reference/python/"` to `extend-exclude` | BI §3.10: allowlist words and identifiers; exclusion "reads as weakening" | **BI wins.** The `_typos.toml` header requires justified additions; an allowlist keeps about 17k lines covered without touching oracle bytes. Re-run on the final 105-file tree: PC reports `fo` (verdict.py:166,169) while BI lists `fo_s`/`fo_w` identifiers, so the exact list needs one more run |
| **X-7** | Python in CI | PC §7: separate job with `setup-python 3.11` | BI §3.11: the workflow header calls `.github` a security boundary with no third-party actions beyond `actions/checkout`; use the runner's preinstalled `python3` plus a pinned lock | **BI's constraint stands** unless the Security/Release owner approves `actions/setup-python`. Default: preinstalled python3 on `ubuntu-*`, `pip install -r reference/python/requirements.txt -c requirements.lock`, threads pinned to 1, Linux only (G-6). If the runner's Python version fails the golden, escalate to the owner rather than loosening tolerances |
| **X-8** | `.gitignore` placement | PC §5: nested `reference/python/.gitignore` (`/data/`, `!tests/grid/golden/snapshot.npz`) | BI §0.4: root `.gitignore` must add `__pycache__/`, `*.py[cod]`, `.pytest_cache/`, `/reference/python/data/` (otherwise traceability fails, 103 of 171 paths) | **Both, and the root additions are mandatory.** The nested file is non-trivial for traceability, but the `` `reference/python/` `` directory form covers it |
| **X-9** | Python suite runtime | BI §2: "tens of minutes" (partial run under contention) | PC §3: 144–149 s on 4 vCPU; OpenMP oversubscription explains the slowness | **PC is right.** Use threads=1. My synth-gate subset took 20.9 s |
| **X-10** | Where specs and archives go | CD §15: `docs/model-specs/`, a validation spec in `docs/03-validation/`, an archive in `docs/archive/cautious-nevermore/` | Vault (`docs/00-meta/authority-index.md:17,42-45`): `docs/03-contracts/`, `docs/05-model-specs/` (level 4), `docs/06-sessions/`; CF, SF and AS use `docs/05-model-specs/` | **The vault wins.** `docs/model-specs/` is only the spec-alias name, and `03` is contracts. The validation spec goes into the consolidated engine spec (§ evaluation) plus model specs under `docs/05-model-specs/`. The archive needs a new numbered slot: decision A-9 |
| **X-11** | Spec file disposition | BI §3.4 option A: keep the `alpha-spec.md` / `final-build-spec.md` names with rewritten content (zero guard churn) | FB D7: retire both into one consolidated engine spec with a new authority order. AS §6: keep the filename or rename with an old→new § crosswalk | Owner decision A-1. Default: **rename + archive** (see §3). Reason: well over 100 citations of `alpha-spec.md §x` in immutable ADRs, evidence and guard comments (AS §6 count table) would silently point at the wrong sections if the same filename carried renumbered content |
| **X-12** | Crate changes in the pivot PR | CF §7.2 / SF §5.5: replace `application` with `pipeline` (+ `grid-cli`), add `synth`, drop `ffi` | BI §10.2: drop only `ffi` now; reword the `application`/`governance` docs; rename or add in the first engine WP (avoids evidence-claim churn: crate counts) | **BI's timing, CF/SF's end state.** Owner decision A-8 |
| **X-13** | What the Kalman observes | FB §0.3 and D1: "the Kalman filter consumes the weekly RAPM / Layer-1 output (`kalman_step` docstring: 'this week's per-player RAPM estimates')"; FB §12.1 DAG built on that | CI NEW-W3, CF C14, SF C3: the cumulative-RAPM observation in `weekly_update.py:307-309` is a **defect**; the validated path (batch, golden, verdict) uses **weekly Layer-1 credit** with real snap counts | **CI/CF/SF are right.** FB's replacement DAG must read V(s) → dV → {offseason RAPM → priors/features; weekly Layer-1 credit} → Kalman. Note for FB D1: Layer-1 credit is **also participation-dependent** (`layers.py:492-493,580` use `off_players`/`def_players`), so option (a) "define the in-season observation without RAPM" also needs a participation-free Layer-1′ (CF C1 / SF §5.1) |
| **X-14** | Trust in the real-data verdict | CD §4: "corrected record", "trustworthy run" (universe-fixed) | CI §0.1 and §1: no real-data number is trustworthy (biased box-score labels: attempts +7.3%, pass yds −7.4%, TDs +6–8%, fumbles −32%, postseason included; iid CIs). CF C1 / SF R14: current-season participation skew | **CI + CF/SF.** Record the numbers as historical, non-parity (G-7). The qualitative conclusion "GRID is not yet above the α§9.4 gate" is robust in direction, but the numbers must be re-measured after the ingest fixes and participation gating |
| **X-15** | Test counts | PC: 446 in-scope / 637 full | CI: "458 engine tests"; CF: 377; SF: 380 | 637 = full CN suite. **446 = oracle closure (authoritative, node-ID verified).** 377 = grid+projection+validation+scoring; 380 = 377 + `test_fantasy_scoring` (3). CI's 458 scope is undocumented in `pytest_engine.txt`, so do not cite it |
| **X-16** | Calibrate-then-gate | CF C10 / E9: keep for KPIs without a spec number, "frozen once and committed" | SF R51: CONTRA; allowed only on a calibration period **disjoint** from evaluation. CD §3.9 notes `thresholds.py` docstring vs empty-registry test | **SF is stricter and matches α§7.7/§12.7** ("versioned before results are seen"). Default in B-9 |
| **X-17** | Parity tolerances | CF: RAPM β rel ≤ 1e-8; Kalman ≤ 1e-12 abs; V(s) within ±0.15 of 4 grid values | SF: RAPM rtol 1e-8 (CG tol 1e-12); Kalman 1e-12; V(s) ±0.05 EP + corr(dV) ≥ 0.999. FB: Class A ≈1e-9 rel; B ≤ 10×CG tol; C correlation; D recovery floors | Owner decision B-3; default given there |
| **X-18** | Prior washout speed | PC §8.1 (run_demo): games to <50%: QB 1, RB 4, WR 4, TE 6, DEF 4 | CF §1.6: below 50% after 1 game for QB/RB/WR/DEF, 2 for TE | **Both are correct for their inputs.** run_demo uses `obs_var_per_game=0.12²` (`run_demo.py:91`); CF used R = 0.40/90. Not a parity target. The model spec must derive R from `SSParams` and exposures (SF: P0 = σ²/n0, n0 learned) |
| **X-19** | Layer-3 recovery numbers for the G1 fix | CI 0.6644→0.7318 (`fit`, n_iter=3); CD 0.696→0.758; CF 0.682→0.739 | — | All are on the legacy synth, so the question is moot (X-1/X-2). Do not cite any of them |
| **X-20** | `format_registry.py` / `vor.py` / `compute_valuations` | PC: import (hard dependencies of the closure) | CF §1.10 / CD §16: app-side | Same resolution as X-5: in the oracle, yes; in the Rust port, VOR only inside `evaluation::lineup` if H2 is kept (decision C-11) |
| **X-21** | `coaching_changes_2025.json` | PC: "default seed data", import verbatim; CD: "hand-curated" | — (nobody checked content) | Import verbatim, but label it unverified and illustrative (G-4) |

---

## 3. Consolidated owner decisions, de-duplicated, with recommended defaults

Sources in brackets. **[S]** statistical owner, **[P]** product/architecture, **[D]** data/licensing, **[R]** security/release.

### 3.1 Scope, authority, repo process

| # | Decision | Raised by | Recommended default |
|---|---|---|---|
| A-1 [P] | Spec consolidation form and filenames | FB D7, BI 6, AS §6 | One consolidated **`engine-spec.md`** (root) plus a byte-identical mirror at `docs/00-meta/specs/engine-spec.md`. It absorbs alpha-spec's engine sections and FB's surviving §7–§21 (FB §7 outline). Archive both originals **verbatim** under `docs/00-meta/specs/superseded/` so that immutable ADR, evidence and guard citations still resolve. Update `check-authority-sync.sh:15-16` and the `tests/guards/run.sh:228-231` fixture in the same commit. Ship an old→new § crosswalk. Fallback if minimal churn is preferred: BI option A |
| A-2 [P][S] | Authority order and oracle placement | AS §1.5, FB D7, BI §7 item 5 | 1 engine spec · 2 accepted ADRs · 3 contracts / model specs / provider manifests · 4 WP · 5 tests and fixtures (including committed oracle fixtures and goldens) · 6 code (including `reference/python/` source). The oracle never overrides the spec; a disagreement is a decision request; an accepted divergence is a statistical-owner ADR |
| A-3 [P][R] | Authoritative CI platform | BI 1, FB D4, AS §8.11, SF D9 | Pivot PR: keep `windows-authoritative` unchanged (no coverage reduction). Then a separate ADR making Linux authoritative with Windows as a matrix job. The oracle job is Linux-only in either case (G-6) |
| A-4 [P] | AI-governance apparatus (α§1.3–1.6, §8.7–8.12, App. B–E) | SF D10 | Keep it. ADRs, guards and evidence depend on it |
| A-5 [P] | WP ID scheme | BI 5, AS §5.1 | Keep `P1-NN` with the AS §5.2 re-scoping (P1-10 → CLI/reports, P1-11 → release/recovery, new P1-12 → GRID component port). Extend `check-traceability.sh:70` to `P[0-9]-[0-9]{2}` in the pivot PR, with a `run.sh` case and control (needed before P2 anyway) |
| A-6 [P] | PR #1 merge order and PRs #2/#3 | BI 7 | Merge PR #1 first with a merge commit (never squash; the manifest attests `8d43203`). Import the 4 reviews verbatim into `docs/06-sessions/`, then close #2 and #3 unmerged |
| A-7 [R] | Fix R4-1 / R4-2 in the pivot PR? | BI 8 | Yes. Guard-only, prototyped, no D5 amendment |
| A-8 [P] | Crate set | BI 2, CF §7.2, SF §5.5 | Pivot PR: drop `ffi` only (plus the FRB dependency, `app/`, `toolchains/flutter.version`); reword the `application`/`governance` docs. First engine WP: `application` → `pipeline` (+ `grid-cli` bin), add `synth`. `persistence` and SQLite stay (`check-sqlx`/`test-rust` depend on them) |
| A-9 [P] | Vault slot for the archived CN material | X-10, CD §15 | New `docs/07-archive/cautious-nevermore/`, registered in `authority-index.md` as **non-authoritative history**. Contents: the CD §15 archive set, the whiteboard pre-clear, Phase-4 SDD T1/T5–T8, `HISTORY.md` (G-3), `real-data-results.md` (G-7) |
| A-10 [D] | License for `reference/python/` (CN has no LICENSE) | BI 3 | The owner, as sole substantive author, records a ruling on the pivot PR bringing the CN-derived code under `MIT OR Apache-2.0`, in the same form as GRID-Engine PR #1 comment 5357318508. Real third-party data is excluded (G-5) |
| A-11 [D] | Fixture licensing policy | G-5 (new) | Synthetic-only parity fixtures. Any real-data fixture goes under `fixtures/third-party/<provider>/` with attribution ("NFL NextGen Stats via nflverse" ≤ 2022, "FTN Data via nflverse" ≥ 2023) and CC-BY-SA notice, and only after a ruling |
| A-12 [P] | Oracle retirement | AS §5.3 | Keep it as a frozen CI oracle until every ported component has parity evidence and Phase-2 live evidence exists. Review at P2-09 |

### 3.2 Oracle and parity

| # | Decision | Raised by | Recommended default |
|---|---|---|---|
| B-1 [S] | Oracle correction policy | X-4: PC, CF, SF, CI, FB, AS | Verbatim import, tagged legacy, then an approved Python correction ledger in the order G-1 synth → B-5 convention → grade sign → causal Kalman init → ingest bias. Rust targets the corrected oracle; the legacy golden is kept; `PARITY.md` lists the deliberate typed-failure divergences |
| B-2 [S][P] | Live oracle in CI vs committed fixtures only | FB D5, CF §7.1, SF §6.2 | Both. Committed, sha256-manifested stage fixtures (exported single-threaded) are the Rust contract. The Linux oracle job proves they regenerate |
| B-3 [S] | Parity tolerance table | X-17: CF, SF, FB | **A** element-wise closed forms (Kalman / RTS / fixed-lag, affine, metrics) ≤ 1e-12 abs. **A′** dense solves ≤ 1e-9 rel. **B** CG RAPM ≤ 10 × CG tolerance and `converged=true`. **C** booster stages: corr(dV) ≥ 0.999 and \|ΔV\| ≤ 0.10 EP on grid cells with support ≥ `min_samples_leaf`. **D** recovery floors re-set on the **fixed** synth (calibrate-below-observed, as the CN gates do). Stage-isolated injection of Python dV and credit for everything downstream of a booster |
| B-4 [S] | Synthetic world | SF D1/D8, CI NEW-Y2, CF E6/C25, G-1 | Now: fix the defenders, plant net strength, regenerate the goldens, re-calibrate the QB NIS bands. Before Layers A–F: a stat-vector synthetic world (volume, shares, TDs, availability, game coupling) plus a "realistic" profile (QB ~99% snaps, real RAPM scale) |
| B-5 [S] | Team-strength estimand, Layer-3 rows, matchup grade | X-2/X-3: CI #1, CF C12/C13, SF D1 | Net strength (off + def quality) from gauge-invariant aggregates; market rows anchor net strength **using the line at lock**; grade = `+E_def` (higher = tougher); reject `[+1,−1]`; truth-anchored Layer-A tests |
| B-6 [S][P] | Typed failure vs faithful port of oracle failure paths (lstsq, corrupt-state reinit, skip-on-failure, refit V(s) every run, non-atomic saves) | FB D5 §5, SF R33/R43, CF C11 | Typed failure in Rust, with an ADR that cites PR #53 C3 (G-3) as the decision being reversed. The oracle is left unchanged and the divergences go in `PARITY.md` |

### 3.3 Modeling and statistics

| # | Decision | Raised by | Recommended default |
|---|---|---|---|
| C-1 [P][S] | RAPM and participation on the live path | FB D1, CF C1/C28, SF D2, AS §0.5 | **Two-tier GRID.** Offseason RAPM is fitted once the season's participation is published (post-season) and seeds priors and features. In-season, a participation-free Layer-1′ involvement credit (passer, rusher, target, sacked QB; exposure from snap counts; team-level opponent adjustment) feeds the Kalman. Add a publication-lag axis to AsOf. H2 +0.848 is not citable until re-run under this |
| C-2 [P][S] | Which architecture governs | AS §0.6, SF §5.1, CF §5 | α§6.1 Layers A–F are the skeleton. GRID is a signal provider: Layer D efficiency latent, Layer E matchup, Layer B market anchor, plus the priors |
| C-3 [S] | Where GRID talent enters Layer D | SF D3 | Option (a): role-specific talent (dropback / carry / target) as covariates in EB-shrunk per-component rate models |
| C-4 [P] | Horizons | CF C3, AS §2.2, CD §15 ADR (e) | Weekly is the primary contract. ROS and preseason are derived sums of weekly draws (diagnostics). H1 is no longer a kill criterion |
| C-5 [S] | Primary metric and gates | CF C9/C10, SF D4/D11, FB D6 | PB-MAE primary; α§9.4 thresholds **kept verbatim and pre-registered**; week-clustered bootstrap; union pool with inactive = 0; acknowledge that GRID is currently below the gate. FB's open parameters (fixed-lag `L`, EB estimator, auto-rollback thresholds, home-field / garbage-time / OT treatment) are to be set in the model specs before code |
| C-6 [S] | Three-season window for stateful parts | CF C4, SF D5 | Per-season XtX/Xty blocks (exact); V(s) refit per season on the window; Kalman carries with discount in production and is re-initialised from the windowed prior on backtest replay |
| C-7 [S][P] | GBM on GRID's critical path; V(s) estimator | SF D6, CF C19, FB D2 | Deterministic in-house V(s) (binned and smoothed, or monotone-constrained) behind a `Regressor` trait. No nflfastR `ep` (scope leak). Layer-1 context model: ridge/GAM candidate vs GBM, decided on recovery evidence |
| C-8 [P] | Booster backend | FB D2 | Pure-Rust first, no native artifacts. `xgb` optional behind a feature once a booster earns its place (α§6.2) |
| C-9 [S] | NCAA / feeder prior form; feeder leagues beyond NCAA | SF D7, CF C27, AS (Decisions) | α§6.3 form with feeder-SV equivalency as one translated component (needs a CFBD PBP pass). NCAA only for alpha; UFL/USFL/XFL/CFL `LEAGUE_FACTORS` are documented but unused. Age and draft step functions (#49) go back to the statistical owner and are not ported as-is |
| C-10 [S] | Kalman semantics bundle | CF C14–C17, CI NEW-W1..W3/#24/#15/NEW-S1 | Weekly credit as the observation; R from real exposures; one filter core with a persisted `games_since_event`; prior-based x0/P0 (no look-ahead); state keyed by (season, week); the component discount documented as implemented (`P[0,0] /= d`) |
| C-11 [P] | VOR / lineup-sim scope | AS (Decisions), CF §7.2 | Keep the H2 lineup simulation as the start/sit decision metric in `evaluation`, with VOR internal to it. No VOR or tier output from the engine |
| C-12 [S][D] | Stat definitions and season type | CI owner #3 (NEW-I5), NEW-V0a/I4, CF C22 | Labels are official nflverse weekly player stats (versioned correction window); REG weeks only for training labels; two-point conversions modelled; official attempt and rush definitions. PBP aggregates are features or cross-checks only |
| C-13 [S] | `drive_points` vocabulary and situation set | CF C23/C24, CI NEW-V0b/NEW-A4 | Keep 7/3/0 for v1 (documented). The code's situation set is canonical; `two_minute` uses `half_seconds_remaining` (the adapter must emit it) |
| C-14 [P] | Engine operating model | FB D3 | One-shot `grid update` CLI under an external scheduler. The engine fetches itself with a once-per-day-per-source cap and raw retention plus hashing (α§4.1.1), and has an offline mode from the raw cache |
| C-15 [P] | Storage | SF §7 | SQLite is the source of truth; bulk artifacts (Parquet / `.npz` equivalents) are registered in a SQLite manifest |

### 3.4 Items with obvious defaults (no owner time needed)

1. Matchup-grade sign fix (X-3).
2. Kalman observation = weekly credit (X-13).
3. Causal Kalman init.
4. Remove `except Exception: pass` in `layer1_all_qbs` (or drop the function).
5. Labels from official stats (C-12 mechanics).
6. Union pool with inactive = 0.
7. Week-clustered bootstrap.
8. Season-keyed accumulator watermark (V2, NEW-W1).
9. `typos` allowlist rather than exclusion (X-6).
10. Root `.gitignore` Python ignores (X-8).
11. Threads pinned to 1 in CI.
12. PC patch P1 (`snake_order` inline) and P2 (`GRID_DEMO_OUT`).
13. Keep the `backend.*` package name.
14. Commit the inventory evidence (G-2) and archive the whiteboard pre-clear (G-9).

---

## 4. Checklist for the consolidation PR, in addition to the per-report plans

- [ ] Commit `docs/06-sessions/2026-10-01-consolidation-inventory/` (the 9 reports) and `reference/python/tools/investigations/` (scripts plus a real-data fetch script with sha256; no parquet).
- [ ] `reference/python/README.md` should cover:
  - provenance: `59bce1d`, v0 `6b0eeee`, `87e227f`, and PR #53 audit lineage;
  - the legacy-synth warning (G-1);
  - the `coaching_changes_2025.json` warning (G-4);
  - the Linux-only rule (G-6);
  - the environment variables `DB_PATH`, `GRID_DEMO_OUT` and `OMP/OPENBLAS/MKL_NUM_THREADS`;
  - "never put `DB_PATH` in the root `.env`" (PC §7).
- [ ] `reference/python/PARITY.md`, holding:
  - the correction ledger (B-1), with G-1 first;
  - the typed-failure divergences (B-6);
  - the non-gating perf test (G-6);
  - the legacy vs fixed Tier-0 table (G-1).
- [ ] Add the isolation-guard plugin to the oracle (G-2).
- [ ] ADR-011 names the oracle scope (PC's 105 files) **and** the port scope (CF §7.2) separately (X-5).
- [ ] Create `docs/07-archive/cautious-nevermore/` with `HISTORY.md` (G-3) and `real-data-results.md` (G-7), subject to decision A-9.
- [ ] Data/Licensing rulings on the pivot PR: `reference/python/` license (A-10) and fixture policy (A-11). Draft `docs/04-providers/nflverse/access-and-license.md` with the G-5 attributions.
