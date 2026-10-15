# Phase 4 Implementation Plan — Matchup Models + Smart Viz
**Date:** 2026-06-22
**Author:** Claude (via /make-plan)
**Builds on:** Phase 3 (complete, 257 tests passing, merged to main at c04ba9e / PR #53)
**Design spec:** `docs/superpowers/specs/2026-06-20-fantasy-football-dashboard-design.md` §5 (GRID Engine Evolution → Phase 4) and §7 (Component 3: Claude Viz Agent)

Phase 4 turns the GRID engine from a single season-long talent estimator into a *matchup-aware* one and exposes its outputs through a natural-language chart builder. The engine work adds situation-specific RAPM passes (red zone / passing downs / rushing downs, with two-minute auto-activating once a clock column lands), a best-effort WR-vs-CB interaction layer, and a 3-component Kalman state `[talent, form, scheme_fit]` with self-migrating persistence and automatic changepoint detection. The product work exposes matchup grades, situation splits, and Kalman trajectories as structured JSON endpoints, then adds a Claude Viz Agent that maps NL queries to a validated `ChartSpec` rendered through the existing Recharts components. Three verifier-identified realities shape this plan: situation RAPM is **blocked on real data** by the same upstream participation gap as base RAPM (it works on synth today); `matchup_grades` currently stores **mislabeled offensive ratings**, which must be fixed before any matchup grade is exposed; and the frontend `tsc -b` gate is **already red on main** and must be greened before the integration branch is cut.

---

## Global Constraints

Inherits all Phase-1 constraints (`docs/superpowers/plans/2026-06-20-fantasy-dashboard-phase1.md` → `## Global Constraints`, lines 13–26). Phase-4 additions and the test-green gate:

- **Test-green gate.** The FULL `python -m pytest -q` suite (257 baseline + new) must be green at every task PR and stay green on the integration branch — not just the task's own tests. A regression in any area blocks the PR.
- **Frontend type gate, and it is a prerequisite.** The repo build is `tsc -b && vite build` (`frontend/package.json:8`, typescript `~6.0.2`); the gate is `npx tsc -b --noEmit` and the **exit code must be checked directly** (do not pipe through `head`/`grep`, which masks tsc's status). This gate is **RED on current main** (4 pre-existing errors — see Task 0). It must be greened before `feat/phase4` is cut.
- **Additive-only frontend.** New components self-fetch and render independently; `RecommendedCharts` gains optional props and new `case`s only. No existing chart page may regress. Every new template must short-circuit to an empty-state because empty data is the COMMON case in Phase 4 until pipelines backfill.
- **DB entrypoint.** `from backend.db.connection import init_db` — `init_db(conn=None)`. Never reference `backend.db.database` (stale Phase-2 text; the module does not exist). There is no migrations directory; schema changes are made idempotent via `CREATE TABLE/INDEX IF NOT EXISTS` plus guarded `ALTER TABLE … ` wrapped in `try/except sqlite3.OperationalError` inside `init_db`.
- **Model-explicitness for the viz agent.** Every Anthropic call passes an explicit `model=` (from `VIZ_AGENT_MODEL`); never an implicit SDK default. Per the SDD model-selection memory, match tier to task complexity and surface the default as an explicit user-facing choice in `.env.example`. `anthropic` must be ADDED to `requirements.txt` (today only `ANTHROPIC_API_KEY` is in `.env.example`). The key is read from `.env`, never hardcoded.
- **Offline-safe CI.** Viz-agent unit tests MUST mock the Anthropic client — zero live API calls, no key required to keep the suite green. Viz endpoints degrade gracefully when `ANTHROPIC_API_KEY` is absent (`VIZ_AGENT_CACHE_ONLY=true` → serve cache hits, 503 on miss).
- **Pipeline / persistence conventions.** Pipeline `run(..., _conn=None)` does not close a passed conn; idempotent `ON CONFLICT`/`INSERT OR IGNORE` upserts; pydantic v2 models; tests mirror source under `tests/<area>/`.
- **Data-availability caveats (honest, up front).** Situation passes depend only on `down`/`ydstogo`/`yardline_100` and are FREE *on synth*; on the **real** nflverse path they are as blocked as base RAPM because `build_design` requires `off_players`/`def_players` that the real `_normalize_pbp` never emits and `load_participation` is `NotImplementedError`. Two-minute needs a game-clock column not in the contract. True WR-vs-CB coverage is paid/unavailable; WR-CB is a documented positional approximation. Coaching/coordinator changes are not in nflverse and require a hand-curated seed.
- **Branch/PR.** One task → one `p4/task-N-<slug>` sub-branch → one PR into `feat/phase4`. Suite green at every PR (the gate above).

---

## Branch & PR Workflow

Phase 4 mirrors the Phase-3 branch-folding model: a single long-lived integration branch, per-task sub-branches that PR **into the integration branch** (never main), one integration-review branch, then **exactly one** merge to main.

- **Integration branch:** `feat/phase4`, cut from `main` once main is green on BOTH `pytest -q` and `npx tsc -b --noEmit`.
- **Per-task sub-branches:** `p4/task-N-<slug>` (e.g. `p4/task-1-canonical-player-order`, `p4/task-7-viz-agent-backend`), each cut FROM `feat/phase4`, implementing exactly one task, ending with the Standard Verification Block, PR'd with `--base feat/phase4`.
- **Integration review:** `p4/review-fixes`, cut from `feat/phase4` after all task PRs fold in; carries cross-task seam fixes; PRs back into `feat/phase4`.
- **Single merge:** one PR `--base main --head feat/phase4`. No task ever merges to main directly.

**Standard Verification Block** (closes every task section; PowerShell/`bash`-friendly, run from repo root):
```bash
# 1. Full suite green (the test-green gate — WHOLE suite, not just new tests)
python -m pytest -q
# 2. Import smoke check (task-specific modules)
python -c "from backend.<mod> import <sym>; print('import ok')"
# 3. OpenAPI route check (tasks adding endpoints) — mirrors tests/api/test_draft_routes.py
python -c "from fastapi.testclient import TestClient; from backend.api.main import app; p=TestClient(app).get('/openapi.json').json()['paths']; assert '/api/<new-route>' in p; print('route ok')"
# 4. Frontend type gate (tasks touching frontend/) — check exit code directly
cd frontend && npx tsc -b --noEmit   # must exit 0
```

**Worked sequence:**
```bash
# Prereq (on main): green the tsc gate, then cut the integration branch
git checkout main && git pull
python -m pytest -q                 # 257 passing
cd frontend && npx tsc -b --noEmit  # MUST be 0 after Task 0 lands on main
git checkout -b feat/phase4 && git push -u origin feat/phase4

# Each task:
git checkout feat/phase4 && git pull
git checkout -b p4/task-7-viz-agent-backend
# ...implement + Standard Verification Block...
gh pr create --base feat/phase4 --head p4/task-7-viz-agent-backend \
  --title 'p4 task 7: Claude Viz Agent backend' --body '<verification output>'

# After all task PRs fold in:
git checkout feat/phase4 && git pull
python -m pytest -q && (cd frontend && npx tsc -b --noEmit)
git checkout -b p4/review-fixes        # seam fixes
gh pr create --base feat/phase4 --head p4/review-fixes --title 'p4: integration review fixes'

# Final, single merge to main:
gh pr create --base main --head feat/phase4 --title 'Phase 4: Matchup Models + Smart Viz'
```

---

## Implementation Tasks

Tasks are numbered globally and ordered by dependency. Foundational fixes that multiple workstreams depend on (canonical player ordering, the matchup-grade semantics fix, the grades-schema migration) come first.

---

### Task 0 — Prerequisite: green the frontend `tsc -b` gate on main (branch: `p4/task-0-tsc-baseline`)

**Why first:** the verifier confirmed `npx tsc -b --noEmit` exits 2 on current main with 4 pre-existing errors, so the gate every later task must pass is unsatisfiable from day one. This lands on **main** (or as the first PR into `feat/phase4` if you prefer to keep main untouched, but main-first is cleaner because the baseline-green check gates the branch cut).

**Files touched:** `frontend/src/charts/PlayerProfileCharts.tsx`, `frontend/src/charts/TradeCharts.tsx`, `frontend/src/charts/WaiverCharts.tsx`.

**Approach:** Fix the 4 errors only — no behavior change. `PlayerProfileCharts.tsx:10` TS6133 unused `playerName` param (remove or consume it); recharts `Formatter` type mismatches at `TradeCharts.tsx:32`, `TradeCharts.tsx:58`, `WaiverCharts.tsx:23` (type the formatter callback args, e.g. `(value: number) => …`, or cast to the recharts `Formatter` signature).

**Tests:** none (type-only fix). Verified by the gate itself.

**Verification:**
```bash
cd frontend && npx tsc -b --noEmit   # MUST exit 0
python -m pytest -q                   # 257 passing, unaffected
```

---

### Task 1 — Canonical `player_id` ordering in `build_design` + order fingerprint in accumulators (branch: `p4/task-1-canonical-player-order`)

**Why early:** the verifier flagged this as a BLOCKER for per-situation incremental accumulation. `build_design` column order = `players.player_id` insertion order (`layers.py:94`), and the only persistence guard is `XtX.shape[0] != n_cols` (`weekly_update.py:159`). A week where `players_df` rows are reordered (same count) silently corrupts every accumulated `XtX`. The per-situation accumulators inherit this hazard, so it must be fixed before they ship.

**Files touched:** `backend/grid/layers.py`, `tests/grid/test_layers.py`.

**Approach:** Inside `build_design`, sort `pids` canonically before building `p_col`:
```python
pids = sorted(players['player_id'].astype(str).tolist())   # was list(players.player_id.values)
p_col = {p: i for i, p in enumerate(pids)}
```
Add a stable order fingerprint to the accumulator npz so `load_accumulators` can reinit on REORDER, not just on dimension change:
```python
# save_accumulators: also store np.array(player_order) (the sorted pid list)
# load_accumulators: return (XtX, Xty, week, player_order); weekly_update reinit if
#   XtX.shape[0] != n_cols OR list(player_order) != current_sorted_pids
```
`run_rapm` is unaffected (it rebuilds colidx from its own `build_design` call).

**Tests:**
- `test_build_design_player_columns_sorted` — column index of a known pid matches its sorted position.
- `test_build_design_order_stable_across_row_shuffle` — shuffling `players_df` rows yields identical `colidx['p_col']`.
- `test_accumulator_reinit_on_player_reorder` — same player count, different order → load triggers reinit (not silent corruption).

**Verification:**
```bash
python -m pytest -q tests/grid/test_layers.py
python -c "from backend.grid.layers import build_design; print('import ok')"
```

---

### Task 2 — Fix matchup-grade sign/semantics in `_upsert_matchup_grades` (branch: `p4/task-2-matchup-grade-semantics`)

**Why before any matchup exposure:** three verifiers independently confirmed that `_upsert_matchup_grades` (body at `weekly_update.py:68–89`) groups `ratings_df` by each player's OWN `team`/`position` and stores that mean under the `def_team` column as a "defensive" grade, and that the `schedule` arg (passed at `weekly_update.py:249`) is **never referenced in the function body**, so the intended off→def mapping cannot happen regardless of whether `schedule` is populated (re-verified 2026-06-22; the 18 rows now in `matchup_grades` are these mislabeled offensive means). Exposing this as a matchup difficulty grade (Tasks 6, 12, 18, the viz agent) would ship a visibly wrong, wrong-signed axis. Fix it here, once, before it is read by four downstream consumers and multiplied across situations.

**Files touched:** `backend/pipeline/weekly_update.py`, `tests/pipeline/test_weekly_update.py`.

**Approach:** A matchup grade for `def_team` against `position` must aggregate that **defense's** player ratings (defenders enter `X` with `-1` per `layers.py:162–166`), sign-flipped so that a *more negative* defensive contribution (suppresses offense) maps to a *harder* matchup. Concretely: group ratings by the defenders' team and position-faced, not by the offensive player's own team. Until a real opponent schedule is wired, compute a team-level defensive grade from the `team_def` intercept columns in `colidx['t_def']` rather than mislabeling offensive player means.
```python
# replace the offense-keyed loop: read team_def intercepts from beta/colidx
team_def = colidx['t_def']                      # {team: col}
for team, col in team_def.items():
    def_strength = -float(beta[col])            # sign flip: higher = tougher defense
    for pos in ('QB','RB','WR','TE'):
        _insert_grade(conn, week, season, def_team=team, position=pos,
                      rapm_grade=def_strength, situation='overall')
```
If position-specific defensive splits are not yet available, store the team-level `def_strength` for every position and document that per-position defensive resolution arrives with the situation passes (Tasks 4–6). This is flagged in the API note (Task 12) as team-level until then.

**Signature + call-site change (required).** `_upsert_matchup_grades` today takes `(conn, ratings_df, week, season, schedule)` and is called at `weekly_update.py:249` with `schedule`. The new body has no `colidx['t_def']`/`beta` to read, so this task must change the signature to receive `beta`/`colidx` from the solve (drop the unused `schedule`) and update that call site to pass them. Both live in `backend/pipeline/weekly_update.py` (already in Files touched).

**Real-data caveat (honest).** The `t_def` intercepts come from the RAPM solve, which on the real nflverse path is blocked by the same participation gap as base RAPM (`data_adapters.py:58` `load_participation` is a `NotImplementedError` stub). This fix makes the grade *correct in sign and semantics*, but it stays **synth-only in practice** until participation lands — on real data the grade is honest-empty, never silently wrong.

**Tests:**
- `test_matchup_grade_uses_def_intercept_not_offense` — a planted strong defense yields a tougher (higher) grade for the *defending* team, not its offense.
- `test_matchup_grade_sign_convention` — stronger defense → higher `rapm_grade`.
- Update the existing `_upsert_matchup_grades` test to the new semantics.

**Verification:**
```bash
python -m pytest -q tests/pipeline/test_weekly_update.py
```

---

### Task 3 — Extend `matchup_grades` with a `situation` dimension (idempotent migration) (branch: `p4/task-3-grades-schema`)

**Files touched:** `backend/db/schema.sql`, `backend/db/connection.py`, `backend/pipeline/weekly_update.py`, `tests/db/test_schema.py`, `tests/pipeline/test_weekly_update.py`.

**Approach (verifier-corrected):** Add a `situation TEXT NOT NULL DEFAULT 'overall'` column and move uniqueness to a named `CREATE UNIQUE INDEX IF NOT EXISTS` (you cannot retro-edit a table-level `UNIQUE`). Because there is no migrations dir and `init_db` only `executescript`s `CREATE TABLE IF NOT EXISTS`, fresh DBs get the column from the updated `CREATE` but **existing DBs do not** — so add an idempotent guard in `init_db` that runs the `ALTER` and the index creation on **every** startup, swallowing the duplicate-column error. The `ON CONFLICT(...,situation)` upsert will raise on an upgraded DB unless BOTH the column and the unique index exist, so both guards are mandatory.

```sql
-- schema.sql
CREATE TABLE IF NOT EXISTS matchup_grades (
  ... position TEXT NOT NULL CHECK(position IN ('QB','RB','WR','TE')),
  situation TEXT NOT NULL DEFAULT 'overall',
  rapm_grade REAL NOT NULL, computed_at TEXT DEFAULT (datetime('now')), ...
);   -- drop the inline UNIQUE(week,season,def_team,position)
CREATE UNIQUE INDEX IF NOT EXISTS ux_matchup_grades
  ON matchup_grades(week, season, def_team, position, situation);
```
```python
# connection.py init_db, AFTER executescript(schema.sql):
try:
    conn.execute("ALTER TABLE matchup_grades ADD COLUMN situation TEXT NOT NULL DEFAULT 'overall'")
except sqlite3.OperationalError:
    pass  # column already exists
conn.execute("CREATE UNIQUE INDEX IF NOT EXISTS ux_matchup_grades "
             "ON matchup_grades(week, season, def_team, position, situation)")
conn.commit()
```
`_upsert_matchup_grades` gains `situation='overall'` and includes it in the INSERT + `ON CONFLICT` target.

**Tests:**
- `test_matchup_grades_has_situation_column`
- `test_init_db_idempotent_on_existing_db_adds_situation` — create the OLD-shape table first, run `init_db`, then assert an `ON CONFLICT(...,situation)` upsert SUCCEEDS (exercises the upgrade path, not just a fresh DB).
- `test_upsert_overall_and_situation_coexist` — same week/team/pos, `'overall'` vs `'red_zone'` both persist.

**Verification:**
```bash
python -m pytest -q tests/db/test_schema.py tests/pipeline/test_weekly_update.py
```

---

### Task 4 — Situation classifier on the plays contract (branch: `p4/task-4-situation-masks`)

**Files touched:** `backend/grid/situations.py`, `tests/grid/test_situations.py`.

**Approach:** Pure module returning ordered boolean masks using ONLY contract columns (`down`, `ydstogo`, `yardline_100`). Two-minute requires a clock column not in the contract, so `classify()` omits it and logs a warning, auto-activating if `quarter_seconds_remaining` is later added. No input mutation.
```python
SITUATIONS = {
  'red_zone':      lambda p: p['yardline_100'].to_numpy() <= 20,
  'passing_downs': lambda p: ((p['down']==3)&(p['ydstogo']>=7)) | (p['down']==4),
  'rushing_downs': lambda p: ((p['down']<=2)&(p['ydstogo']<=4)),
}
TWO_MIN_COL = 'quarter_seconds_remaining'
def classify(plays):
    out = {name: np.asarray(fn(plays), bool) for name, fn in SITUATIONS.items()}
    if TWO_MIN_COL in plays.columns:
        out['two_minute'] = plays[TWO_MIN_COL].to_numpy() <= 120
    else:
        logger.warning('two_minute skipped: %s absent', TWO_MIN_COL)
    return out
```

**Tests:** `test_red_zone_mask_matches_yardline_threshold`, `test_passing_downs_includes_4th_down`, `test_two_minute_omitted_when_clock_col_absent`, `test_masks_are_boolean_and_length_matches_plays`, `test_no_input_mutation`.

**Verification:**
```bash
python -m pytest -q tests/grid/test_situations.py
python -c "from backend.grid.situations import classify; print('import ok')"
```

---

### Task 5 — `run_situation_rapm`: per-situation full-season solve (synth-capable; real-data gated) (branch: `p4/task-5-situation-rapm`)

**Files touched:** `backend/grid/layers.py`, `tests/grid/test_layers_situations.py`.

**Approach (verifier-corrected framing):** Thin orchestration over the EXISTING `run_rapm`. For each mask with ≥ `min_plays` rows, slice `plays.loc[mask]` and call `run_rapm` unchanged (`build_design` rebuilds a self-consistent colidx on the subframe). **Honesty fix:** this delivers value on **synthetic** data only; on real data it is as blocked as base RAPM because `build_design` requires `off_players`/`def_players` that the real loader never emits. So `run_situation_rapm` must **assert the participation columns exist** and raise a clear error rather than failing deep in sparse assembly.
```python
def run_situation_rapm(plays, players, situations=None, min_plays=200, **kw):
    from backend.grid import situations as S
    if 'off_players' not in plays.columns or 'def_players' not in plays.columns:
        raise ValueError('situation RAPM requires participation columns (off_players/def_players); '
                         'real nflverse path does not yet emit them (load_participation is a stub)')
    masks = situations or S.classify(plays)
    out = {}
    for name, m in masks.items():
        sub = plays.loc[m]
        if len(sub) < min_plays:
            logger.info('skip %s: %d<%d plays', name, len(sub), min_plays); continue
        ratings, team_df, beta, colidx = run_rapm(sub, players, **kw)
        out[name] = ratings
    return out
```
Players absent from a situation subframe get a ridge-prior 0 and must be surfaced downstream as "no data," not a real 0 grade.

**Tests:** `test_run_situation_rapm_returns_subset_of_situations`, `test_low_volume_situation_skipped`, `test_red_zone_ratings_have_expected_player_columns`, `test_situation_ratings_independent` (synth with a planted red-zone-only bump shows higher red_zone rating than overall), `test_missing_participation_columns_raises`.

**Verification:**
```bash
python -m pytest -q tests/grid/test_layers_situations.py
```

---

### Task 6 — Per-situation keyed incremental accumulators (branch: `p4/task-6-situation-accumulators`)

**Files touched:** `backend/grid/layers.py`, `backend/pipeline/weekly_update.py`, `tests/pipeline/test_weekly_situations.py`.

**Approach:** Make the accumulator path situation-keyed. Add optional `key='all'` to BOTH `save_accumulators` and `load_accumulators` (update both call sites at `weekly_update.py:151,168`), producing `data/cache/rapm_accumulators_{key}.npz` (`key='all'` keeps the legacy `_ACCUM_PATH` name). In `weekly_update.run`, after the overall pass, loop situations: `build_design` on the subframe, apply the SAME dim+order reinit guard (now order-aware from Task 1) PER situation, accumulate, save, `fit_from_accumulators`, upsert grades tagged with the situation. Key the `last_week` skip guard per situation so a re-run does not double-count. Keep `interactions=False` on this path (documented) — interactions change `nCols` and need their own keyed accumulator.
```python
def _accum_path(key='all'):
    return _ACCUM_PATH if key=='all' else _ACCUM_PATH.with_name(f'rapm_accumulators_{key}.npz')
# weekly_update.run, after overall solve:
for name, m in classify(plays_df).items():
    sub = plays_df.loc[m]
    if len(sub) < MIN_PLAYS: continue
    Xs, ys, cidx = build_design(sub, players_df); nc = cidx['nCols']
    ld = load_accumulators(key=name)
    if ld and (ld[0].shape[0]!=nc or list(ld[3])!=sorted_pids): XtXs,Xtys = init_accumulators(nc)
    elif ld: XtXs,Xtys = ld[0], ld[1]
    else: XtXs,Xtys = init_accumulators(nc)
    XtXs,Xtys = accumulate(XtXs,Xtys,Xs,ys); save_accumulators(XtXs,Xtys,week,key=name)
    beta_s = fit_from_accumulators(XtXs,Xtys,_LAM,np.zeros(nc),_sit_mask(cidx))
    _upsert_matchup_grades(conn,_ratings_df(beta_s,cidx,players_df),week+1,season,situation=name)
```

**Tests:** `test_situation_accumulator_files_created_per_situation` (assert `data/cache/rapm_accumulators_{name}.npz` exist AND `key='all'` resolves to legacy `_ACCUM_PATH`), `test_situation_dim_mismatch_reinitialises`, `test_rerun_week_skips_all_situation_accumulators`, `test_situation_grades_written_with_situation_tag`.

**Verification:**
```bash
python -m pytest -q tests/pipeline/test_weekly_situations.py
python -c "from backend.grid.layers import save_accumulators, load_accumulators; print('import ok')"
```

---

### Task 7 — Best-effort WR-vs-CB interaction columns in `build_design` (branch: `p4/task-7-wr-cb-interaction`)

**Files touched:** `backend/grid/layers.py`, `backend/grid/synth.py`, `tests/grid/test_design_interactions.py`.

**Approach:** Add `interactions=False` (default OFF so `run_rapm`/`weekly_update` are byte-for-byte unchanged and the colidx contract is preserved). When enabled, append interaction columns AFTER the `[players, team_off, team_def]` block (existing indices stay stable), record `colidx['interactions']={(wr,cb):col}`, bump `nCols`. Because real alignment data is a stub, build the matchup by **positional approximation** pairing each on-field WR with each on-field DEF (CB proxy), pruning pairs seen `< min_pair_plays`. In `run_rapm`, penalize interaction columns heavily in the ridge mask. Add a synth knob to split a few `DEF` players into a CB role so the planted-recovery test is generatable (synth has only one DEF bucket today).
```python
def build_design(plays, players, interactions=False, min_pair_plays=30):
    # ... existing X/y/colidx assembly unchanged ...
    if not interactions:
        return X, y, colidx
    # pair on-field WRs with on-field CB-proxy DEFs, prune low-support, hstack block
    colidx['interactions'] = pair_col; colidx['nCols'] += len(pair_col)
    return X, y, colidx
```

**Tests:** `test_build_design_default_no_interactions_unchanged_nCols` (regression: colidx identical with `interactions=False`), `test_interactions_append_columns_after_base_block`, `test_pruning_drops_low_cooccurrence_pairs`, `test_missing_participation_falls_back_to_positional_only`, `test_planted_wr_cb_interaction_recovered` (gated on the synth CB-split knob; mark real-data-only/skip if the knob is absent).

**Verification:**
```bash
python -m pytest -q tests/grid/test_design_interactions.py
```

---

### Task 8 — Grow Kalman latent state to 3 components `[talent, form, scheme_fit]` (branch: `p4/task-8-kalman-3comp`)

**Files touched:** `backend/grid/statespace.py`, `tests/grid/test_incremental.py`, `tests/grid/test_kalman_numerical.py`.

**Approach (verifier-corrected — ALL 2-d sites enumerated):** Introduce `N_STATE=3` and make EVERY hardcoded 2-d site `N_STATE`-aware, not just `init`/`F`/`H`. The verifier flagged these concrete omissions that will otherwise break the offline smoother:
- `kalman_step` F at `statespace.py:179`, H at `:180`, `np.eye(2)` at `:212`.
- `kalman_two_component` F at `:106`, H at `:108`, `np.eye(2)` at `:135`, **RTS smoother `np.eye(2)` at `:150`**, **P0 default `[[0.05,0],[0,0.02]]` at `:112`**, **x0 init `nanmean(y[:3])` at `:110–111`** (fills talent only; needs a 3-vector).

`scheme_fit` is a very-slow random walk: `F = diag(1, phi, phi_scheme)` with `phi_scheme≈0.985`; `H=[1,1,1]`; predict adds `sigma_pred[:,2,2] += q_scheme` (small, `≈0.0002`). `init`: `mu=zeros((n,3))`, `sigma=diag([0.05,0.02,0.01])` per player. Keep Joseph form with `np.eye(N_STATE)`. **Drop the unsound "2-component path keeps passing unchanged" claim** — `kalman_two_component` itself grows to 3 components; the numerical tests are PORTED to 3×3 in the SAME commit, not "kept unchanged."
```python
N_STATE = 3
@dataclass
class SSParams:
    phi: float = 0.50
    phi_scheme: float = 0.985
    d_steady: float = 0.90; d_spike: float = 0.70
    q_form: float = 0.0008; q_scheme: float = 0.0002
    scheme_reset_var: float = 0.04
    r_scale: float = 0.40; post_event_r_mult: float = 2.0; post_event_games: int = 2
def _F(p): return np.array([[1.,0.,0.],[0.,p.phi,0.],[0.,0.,p.phi_scheme]])
_H = np.array([1.,1.,1.])
```
Confirm `SSParams.from_position` (which only overrides `d_steady`/`r_scale`) leaves the new fields at defaults intentionally (scheme params position-invariant).

**Tests:** `test_kalman_state_init` (shapes `(n,3)`/`(n,3,3)`, diag `[0.05,0.02,0.01]`), `test_kalman_step_three_component_symmetry` (3×3 sigma symmetric, eigvals > −1e-12), `test_scheme_fit_drifts_slowly`, `test_two_component_recovery_unchanged` (planted-talent recovery corr within tolerance of pre-change baseline), and a **3-component RTS PD test** in `test_kalman_numerical.py` (the smoother the design originally omitted).

**Verification:**
```bash
python -m pytest -q tests/grid/test_incremental.py tests/grid/test_kalman_numerical.py
```

---

### Task 9 — Versioned, self-migrating Kalman `.npz` (pad legacy 2-comp → 3-comp) (branch: `p4/task-9-kalman-npz-migration`)

**Files touched:** `backend/grid/statespace.py`, `tests/grid/test_incremental.py`.

**Approach:** Write `state_version=N_STATE`. `load` detects legacy state by absent `state_version` OR `mu.shape[1]==2`, and pads losslessly: `mu = hstack([mu2, zeros((n,1))])`; embed each old 2×2 into the top-left of a fresh 3×3 with `[2,2]=scheme_init_var (0.01)`, off-diagonals 0. Corrupt/unknown (e.g. width 5, future >3) → return `None` so the caller reinits (`weekly_update.py:203` handles `None`). `migrate()` is a classmethod for isolated unit testing. Remove the dead `ver` computation in the load sketch (decide by shape).
```python
@classmethod
def migrate(cls, mu2, sigma2, pids, scheme_var=0.01):
    n = mu2.shape[0]
    mu = np.hstack([mu2, np.zeros((n,1))])
    sigma = np.zeros((n,3,3)); sigma[:,:2,:2] = sigma2; sigma[:,2,2] = scheme_var
    return cls(mu=mu, sigma=sigma, player_ids=list(pids))
```

**Tests:** `test_migrate_2comp_to_3comp`, `test_load_legacy_npz_pads` (savez a 2-comp file WITHOUT `state_version` to a monkeypatched `_KALMAN_PATH`), `test_load_corrupt_returns_none` (`mu.shape[1]==5`), `test_save_writes_version`.

**Verification:**
```bash
python -m pytest -q tests/grid/test_incremental.py
```

---

### Task 10 — `coaching_changes` seed table + scheme-reset wiring (branch: `p4/task-10-coaching-changes`)

**Files touched:** `backend/db/schema.sql`, `backend/db/seed_coaching.py`, `backend/db/data/coaching_changes_2025.json`, `backend/pipeline/weekly_update.py`, `tests/grid/test_coaching_changes.py`.

**Approach:** Add idempotent `coaching_changes(season, week_effective, team, role CHECK in ('HC','OC','DC'), old_name, new_name, UNIQUE(season,week_effective,team,role))`, seeded from a checked-in JSON via `seed_coaching.py` (explicit call, not part of `init_db`, so tests run without it). In `weekly_update.run`, query the week's changed teams, map team→player_ids via `players_df` (OC/HC → skill positions; DC → DEF), and pass a **`scheme_resets`** set into `kalman_step` as a NEW param distinct from `interventions`: a reset zeroes `mu[i,2]` and sets `sigma[i,2,2]=scheme_reset_var` (does NOT touch talent/form), whereas an intervention spikes talent/form variance. **Per-position threading (verifier fix):** `kalman_step` runs per-position on sub-states (`weekly_update.py:233–241`); pass `scheme_resets` into each per-position call exactly like `sub_intv` at `:240` (or rely on `kalman_step`'s `pid_to_idx` filter) — make the call site explicit, not elided.
```python
def kalman_step(state, obs, snaps=None, params=None, interventions=None, scheme_resets=None):
    ...
    for i in {pid_to_idx[p] for p in (scheme_resets or set()) if p in pid_to_idx}:
        new_mu[i,2] = 0.0
        sigma_pred[i,2,2] = p.scheme_reset_var
        sigma_pred[i,2,:2] = 0.0; sigma_pred[i,:2,2] = 0.0
```

**Tests:** `test_coaching_changes_schema_idempotent`, `test_scheme_reset_zeroes_scheme_only` (p1 scheme reset; talent/form of p1 and ALL of p2 unchanged), `test_scheme_reset_independent_of_intervention`, `test_weekly_update_applies_coaching_reset` (seed an HC change for team X week 5; assert team X's QB scheme_fit reset in the saved `KalmanState`).

**Verification:**
```bash
python -m pytest -q tests/grid/test_coaching_changes.py tests/pipeline/test_weekly_update.py
python -c "from backend.db.seed_coaching import seed_coaching_changes; print('import ok')"
```

---

### Task 11 — Automatic changepoint detection replacing `interventions={9}` (branch: `p4/task-11-changepoint`)

**Files touched:** `backend/grid/statespace.py`, `backend/pipeline/weekly_update.py`, `tests/grid/test_changepoint.py`.

**Approach:** Add stateless `detect_changepoints(state, obs, snaps, params, z_thresh=3.0)` computing the standardized innovation `z = (obs - H@mu_pred)/sqrt(S)` with `S = H@sigma_pred@H.T + R` (the same quantities `kalman_step` forms at `:209–210`). Flag `|z| > z_thresh`. CUSUM is a documented, version-gated opt-in (`params.use_cusum=False` default) to avoid coupling to the npz migration. **Integration fix (verifier):** run detection INSIDE the per-position loop with that position's `SSParams.from_position(pos)` and the position's sub-slices — NOT a single global `SSParams()` before the loop — so detection `S`/`z` match the update's params. Build the auto-intervention set as `(interventions or set()) | auto`. The demo `interventions={9}` stays only in `statespace.py` `__main__`.
```python
# weekly_update, per-position:
sp = SSParams.from_position(pos)
auto = detect_changepoints(sub_state, sub_obs, sub_snaps, sp)
sub_intv = (interventions_for_pos or set()) | auto
ks_sub = kalman_step(sub_state, sub_obs, sub_snaps, sp, interventions=sub_intv, scheme_resets=sub_resets)
```

**Tests:** `test_detect_no_changepoint_quiet_series`, `test_detect_flags_large_jump` (obs 5σ above prediction), `test_detect_ignores_missing_obs` (NaN never flagged), `test_detect_threshold_monotone`, `test_weekly_update_auto_interventions_recovers_demo` (synth focus-QB series flags the injury-return week without the hardcoded `{9}`).

**Verification:**
```bash
python -m pytest -q tests/grid/test_changepoint.py
```

---

### Task 12 — Viz/data read endpoints: matchup grades, situation splits, trajectory + `/stats/fields` extension (branch: `p4/task-12-viz-data-api`)

**Files touched:** `backend/db/schema.sql`, `backend/api/routes/stats.py`, `backend/api/routes/season.py`, `backend/pipeline/weekly_update.py`, `tests/api/test_routes.py`, `tests/api/test_season_routes.py`.

**Approach (verifier-corrected):**
- **`GET /matchup-grades`** (stats router) over the now-correctly-signed `matchup_grades` (Tasks 2–3): filter by `season` + optional `week`/`def_team`/`position`/`situation` (default all situations); return dict rows. Derive an A–F letter from `rapm_grade` percentile within `(week, season, position)`. **Must add `from fastapi import HTTPException`** to `stats.py` (currently imports only `APIRouter, Depends, Query`) for the trajectory 404 path. Empty table → `[]` with 200.
- **`GET /situation-splits`** over a new `situation_grades(player_id, season, week, situation CHECK in ('red_zone','passing_down','rushing_down','two_minute'), rapm_grade, snaps, PRIMARY KEY(player_id,season,week,situation))` table (DDL here; rows produced by Tasks 5–6). Join `players` for name/position/team. Empty until pipeline backfills → render-safe `[]`.
- **`GET /trajectory/{player_id}`** reads a new `kalman_trajectory(player_id, season, week, talent, form, total, var_total, scheme_fit, ON CONFLICT(player_id,season,week))` table written by a hook in **`weekly_update.py`** (NOT `statespace.py`) right after `ks.save()` (`weekly_update.py:244`), guarded by the `_conn`/`_owns_conn` pattern. Read mu width dynamically (include `scheme_fit` when width==3). Fall back to the npz current state if no rows; 404 if neither (monkeypatch `_KALMAN_PATH` in the fallback test).
- **`/stats/fields` extension:** append `rapm_grade`, `situation_rapm`, `talent`, `form`, `scheme_fit`, `trajectory`, each with an optional `source` tag (`matchup_grades`/`situation_grades`/`kalman`). The response is **wrapped in `{'fields': [...]}`** — tests assert on `body['fields']`. **Note: the existing list has 14 fields, not 13** — any "all keys present" assertion counts the legacy 14 plus the new ones.

```python
# stats.py (note the import fix)
from fastapi import APIRouter, Depends, Query, HTTPException
@router.get('/matchup-grades')
def get_matchup_grades(season:int=CURRENT_SEASON, week:int|None=None, def_team:str|None=None,
                       position:str|None=None, situation:str|None=None, db=Depends(get_db_dep)):
    q='SELECT week,season,def_team,position,situation,rapm_grade,computed_at FROM matchup_grades WHERE season=?'
    args=[season]
    for col,val in (('week',week),('def_team',def_team),('position',position),('situation',situation)):
        if val is not None: q+=f' AND {col}=?'; args.append(val)   # fixed col names only; values bound
    q+=' ORDER BY week, def_team, position, situation'
    return [dict(r) for r in db.execute(q,args).fetchall()]
```

**Tests:** `test_matchup_grades_endpoint_returns_rows`, `test_filter_by_situation`, `test_empty_situation_returns_empty_list_not_500`, `test_situation_splits_join_players`, `test_trajectory_series_ascending`, `test_trajectory_404_when_no_rows_and_no_npz` (monkeypatch `_KALMAN_PATH`), `test_trajectory_includes_scheme_fit_when_width_3`, `test_stats_fields_includes_new_keys_and_sources` (assert on `body['fields']`, legacy 14 still present).

**Verification:**
```bash
python -m pytest -q tests/api/test_routes.py tests/api/test_season_routes.py
python -c "from fastapi.testclient import TestClient; from backend.api.main import app; p=TestClient(app).get('/openapi.json').json()['paths']; assert '/api/matchup-grades' in p and '/api/situation-splits' in p; print('routes ok')"
```

---

### Task 13 — Viz-agent dependency + env wiring (branch: `p4/task-13-viz-deps-env`)

**Files touched:** `requirements.txt`, `.env.example`.

**Approach:** Append `anthropic>=0.49` (httpx already satisfies the transitive constraint). Add `VIZ_AGENT_MODEL` (default surfaced explicitly) and `VIZ_AGENT_CACHE_ONLY=false` under the existing `ANTHROPIC_API_KEY` block. The model id is read in `agent.py` via `os.getenv('VIZ_AGENT_MODEL', ...)` and passed explicitly to every call.
```bash
# requirements.txt (append)
anthropic>=0.49
# .env.example (append)
VIZ_AGENT_MODEL=claude-haiku-4-5   # cost-tier default; quality upgrade: claude-opus-4-8
VIZ_AGENT_CACHE_ONLY=false
```

**Tests:** `tests/viz_agent/test_config.py` — importing `backend.viz_agent.agent` does not require `ANTHROPIC_API_KEY` at import time (lazy client); `VIZ_AGENT_MODEL` read at call time.

**Verification:**
```bash
python -m pytest -q tests/viz_agent/test_config.py
python -c "import anthropic; print('anthropic ok')"
```

---

### Task 14 — `ChartSpec` model + cache table created in production + schema context (branch: `p4/task-14-viz-schema-cache`)

**Files touched:** `backend/viz_agent/__init__.py`, `backend/viz_agent/schema.py`, `backend/viz_agent/cache.py`, `backend/db/schema.sql`, `backend/api/main.py`, `tests/viz_agent/test_schema.py`, `tests/viz_agent/test_cache.py`.

**Approach (verifier-corrected):**
- `schema.py`: pydantic v2 `ChartSpec` mirroring the TS type EXACTLY (`chartType` Literal, `xAxis`, `yAxis`, `position: str|None`, `season: int`, `colorBy` Literal). `allowed_field_keys()` imports `get_stat_fields()` (single source of truth) — returns **14** keys today (not 13). `build_schema_context()` renders a compact deterministic block with `cache_control` wired (a no-op below Haiku's 4096-token cacheable prefix; auto-activates when the allowlist grows with the Task 12 fields).
- `cache.py`: `make_key(query, season, position)` → sha256 of `strip().lower()` normalized text + filters; `get_cached`/`put_cached` use the `_conn` pattern and `ON CONFLICT` upsert.
- **BLOCKER fix — create the cache table in production.** `init_db`/`schema.sql` only runs via pipeline scripts and the test fixture; `main.py` has no startup hook, so a server booting against an existing `fantasy.sqlite` would hit "no such table: viz_query_cache." Add a FastAPI startup event in `main.py` that runs `CREATE TABLE IF NOT EXISTS viz_query_cache (...)` (or have `get_cached`/`put_cached` create-if-missing). Add a regression test that boots the app WITHOUT running the pipeline and confirms the cache path works.
```sql
CREATE TABLE IF NOT EXISTS viz_query_cache (
  query_hash TEXT PRIMARY KEY, query_text TEXT NOT NULL, spec_json TEXT NOT NULL,
  hit_count INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL DEFAULT (datetime('now')));
```
```python
# main.py
@app.on_event('startup')
def _ensure_viz_cache():
    conn = init_db()  # idempotent; creates viz_query_cache if absent
    conn.close()
```

**Tests:** `test_chartspec_serializes_to_ts_keys` (`{chartType,xAxis,yAxis,position,season,colorBy}`), `test_allowed_field_keys_returns_14`, `test_build_schema_context_deterministic_contains_every_key`, `test_make_key_case_whitespace_insensitive`, `test_put_then_get_round_trips_and_increments_hit_count`, `test_cache_table_exists_on_fresh_app_boot_without_pipeline`.

**Verification:**
```bash
python -m pytest -q tests/viz_agent/test_schema.py tests/viz_agent/test_cache.py
```

---

### Task 15 — Claude call → structured `ChartSpec` with allowlist repair (branch: `p4/task-15-viz-agent`)

**Files touched:** `backend/viz_agent/agent.py`, `tests/viz_agent/test_agent.py`.

**Approach:** Lazy `anthropic.Anthropic()` client (constructed only on a non-cached query). `generate_spec` calls Claude with the EXPLICIT `VIZ_AGENT_MODEL`, structured outputs behind a thin `_structured_call()` selecting `messages.parse(output_format=ChartSpec)` when available, else `messages.create(output_config={'format':{'type':'json_schema','schema':ChartSpec.model_json_schema()}})` + `model_validate`. System prompt = `build_schema_context()` with `cache_control={'type':'ephemeral'}`. `validate_and_repair` enforces the allowlist: off-list `xAxis`/`yAxis` → fuzzy repair to nearest allowed key, else `VizValidationError`; `position` coerced to QB/RB/WR/TE/None; **clamp `season` to the supported set** (Workbench `SEASONS = [2024,2023,2022]`). **Axis-family guard (verifier):** reject/repair specs that pair a raw-stat axis (e.g. `passing_yards`) with a valuation axis (`projected_points`/`vor`), since the frontend's single `needsValuations` switch fetches only one dataset and `getVal` returns 0 for the absent family — otherwise the allowlist permits silently-empty charts. The model returns ONLY a ChartSpec — no SQL ever crosses the model boundary. Without a key and `VIZ_AGENT_CACHE_ONLY=true`, raise a clean "agent unavailable" rather than constructing the client.

**Tests:** clean spec untouched; off-allowlist `xAxis` with no near match → `VizValidationError`; near-miss `rushingyards`→`rushing_yards` repairs; bad position → None; season 2019 clamped; mixed raw+valuation axes rejected/repaired; system block carries `cache_control` ephemeral and `model == VIZ_AGENT_MODEL` (assert on mocked client kwargs) — **NO real API call**.

**Verification:**
```bash
python -m pytest -q tests/viz_agent/test_agent.py
```

---

### Task 16 — `POST /api/viz-agent/query` route (cache-first + allowlist guard) (branch: `p4/task-16-viz-route`)

**Files touched:** `backend/api/routes/viz_agent.py`, `backend/api/main.py`, `tests/api/test_viz_agent_routes.py`.

**Approach:** New router (mirror `stats.py`: `APIRouter`, `Depends(get_db_dep)`). `POST /viz-agent/query` body `{query, season=2024, position}`. Flow: `make_key` + `get_cached` → `{spec, cached:true}` on hit (zero tokens); miss → `agent.generate_spec`; `VizValidationError` → HTTP 422 with the offending field; `put_cached`; return `{spec, cached:false}`. Gate on `VIZ_AGENT_CACHE_ONLY`/missing key: serve hits, 503 on miss. `model_dump()` field names are already camelCase, matching the TS `ChartSpec` (no alias layer). Register in `main.py:4` import tuple + `include_router(viz_agent.router, prefix='/api')`.

**Tests:** cache-miss path (agent monkeypatched) → `cached:false` + valid spec; immediate repeat → `cached:true` and does NOT call the patched agent; off-allowlist agent result → 422; response JSON keys exactly `{chartType,xAxis,yAxis,position,season,colorBy}`; **fresh-app-boot test** confirms the route works without the pipeline (depends on Task 14's startup table creation).

**Verification:**
```bash
python -m pytest -q tests/api/test_viz_agent_routes.py
python -c "from fastapi.testclient import TestClient; from backend.api.main import app; p=TestClient(app).get('/openapi.json').json()['paths']; assert '/api/viz-agent/query' in p; print('route ok')"
```

---

### Task 17 — Frontend types + react-query hooks for new endpoints (branch: `p4/task-17-frontend-types-hooks`)

**Files touched:** `frontend/src/types/index.ts`, `frontend/src/hooks/useApi.ts`.

**Approach:** Add `MatchupGradeCell`, `SituationSplit`, `TrajectoryPoint`, `PlayerTrajectory` interfaces and hooks `useMatchupGrades`, `useSituationSplits`, `useTrajectory`, plus `useVizAgentQuery` (mutation via `mutateJson('POST','/viz-agent/query', body)`). **EDIT** `StatField` to add `source?: string` (do not redeclare — duplicate-identifier error). **Keep `MatchupGradeCell` DISTINCT from the existing `MatchupGrade`** at `types/index.ts:228` (that is the per-roster matchup VIEW row, a different shape — do not reuse it for the heat strip). Trajectory 404 surfaces as a react-query error; consumers render empty-state.
```typescript
export interface MatchupGradeCell { week:number; season:number; def_team:string; position:string; rapm_grade:number; grade:'A'|'B'|'C'|'D'|'F'; situation:string; }
export interface TrajectoryPoint { week:number|null; talent:number; form:number; total:number; scheme_fit?:number; var_total?:number; }
export interface PlayerTrajectory { player_id:string; series:TrajectoryPoint[]; }
export interface StatField { key:string; label:string; positions:string[]; source?:string; }  // EDIT, not new
```

**Tests:** type-check via the build gate; ensure no duplicate `StatField`.

**Verification:**
```bash
cd frontend && npx tsc -b --noEmit   # exit 0
```

---

### Task 18 — Frontend chart templates + `RecommendedCharts` wiring (branch: `p4/task-18-frontend-charts`)

**Files touched:** `frontend/src/charts/MatchupGradeStrip.tsx`, `frontend/src/charts/SituationRadar.tsx`, `frontend/src/charts/TrajectoryChart.tsx`, `frontend/src/charts/RecommendedCharts.tsx`.

**Approach:** Three self-fetching templates (mirror `PlayerProfileCharts.tsx`, each short-circuits to an empty-state). `MatchupGradeStrip`: dependency-free grid of A–F colored cells (independent ramp, not `POSITION_COLORS`). `SituationRadar`: reuse recharts `RadarChart` (already imported in `ChartRenderer`) with the four situations as axes. `TrajectoryChart`: recharts `LineChart` with `talent`/`form`/`total` lines, plus `scheme_fit` only when present. Wire additively into `RecommendedCharts`: extend the context union with `'matchup'|'situation'`, add optional `week`/`playerIds`/`position`/`season` props, inject `TrajectoryChart` into the existing `player_profile` case via a fragment (render both). All existing cases untouched. Restrict agent `colorBy` to `position|team` (or add `tier` to the picker) so picker and output agree.

**Tests:** Python tests/ is the primary harness (no JS test runner present); verify via the run-skill — type an NL query, confirm a chart renders through `ChartRenderer` and the `generated`→`cached` badge flips on repeat. Backend coverage (Tasks 12, 16) is the gate; the type gate guards these files.

**Verification:**
```bash
cd frontend && npx tsc -b --noEmit   # exit 0
```

---

### Task 19 — Frontend: NL input box wired into Workbench (branch: `p4/task-19-viz-agent-frontend`)

**Files touched:** `frontend/src/components/VizAgentBox.tsx`, `frontend/src/pages/Workbench.tsx`.

**Approach:** `VizAgentBox` = text input + submit calling `useVizAgentQuery`; on success it lifts the returned `ChartSpec` via `onSpec`. In `Workbench.tsx`, render `VizAgentBox` above the Chart Builder and wire `onSpec={setSpec}` — because the page already drives `ChartRenderer` purely from `spec` state (`:194,:416`), setting spec from the agent reuses the ENTIRE existing safe data-fetch + render path with no new rendering code. Show a `cached`/`generated` badge. On 422, surface `m.error` rather than crashing. **Document the season caveat:** on the valuations branch `/rankings` ignores `season` (no season param), so agent-emitted season only affects the `/stats` path — `validate_and_repair` clamps season but the valuations path does not filter by it (a known limit, listed in Trade-offs).

**Tests:** verified via the run-skill (no JS harness); the type gate guards the wiring.

**Verification:**
```bash
cd frontend && npx tsc -b --noEmit   # exit 0
```

---

### Task 19b — Frontend QA fixes: waiver/trade error latency + browser tab title (branch: `p4/task-19b-frontend-qa-polish`)

**Source:** Browser dogfooding of `main` (gstack `/gstack` QA pass, 2026-06-22). Two defects surfaced that are independent of the matchup/viz work but live in `frontend/` files Phase 4 already touches, so they fold in here rather than spawning a separate cleanup. Both are frontend-only, additive, and gated by the same `tsc -b` green gate (Task 0); neither touches the engine or API. Land before the viz agent (T19) starts driving users into the In-Season pages.

**Files touched:** `frontend/src/hooks/useApi.ts`, `frontend/src/pages/WaiverWire.tsx`, `frontend/src/pages/TradeCenter.tsx`, `frontend/index.html`. (The error UI already exists — `WaiverWire.tsx:68–69` and `TradeCenter.tsx:32–33` render `isLoading`/`isError`; the bug is retry latency in the hooks plus a missing not-configured empty-state, not absent error rendering.)

**Approach:**

- **Fix 1 — waiver & trade pages hang ~3–7s on a guaranteed 404 before the error shows.** `useWaiverWire` (`useApi.ts:281`) and `useFindTrades` (`useApi.ts:308`; its `/trades/find` query is consumed at `TradeCenter.tsx:16`) default `league_id=1`/`my_team_id=1`, which 404s whenever no league is configured — the COMMON case, since the Dashboard already shows "No leagues configured yet." react-query's default 3× exponential-backoff retry keeps both pages on "Loading…/Finding…" for 3–7s before the existing `isError` branch renders, and emits repeated 404s to the console. Verified live (poll showed `still_loading → ERROR_SHOWN` only after the retry window). Two-part fix:
  - Set `retry: false` on both query options — a 404 here is deterministic, so retrying only adds latency and console noise.
  - Gate the fetch with `enabled: leagueId > 0 && week > 0` (plus `myTeamId > 0` for trades). With `enabled:false` react-query stays `pending` with `fetchStatus:'idle'` (so `isLoading` is false and `data` is undefined); add a not-configured empty-state to each page keyed off `!isLoading && !isError && !data` → "Enter a league ID to see waiver/trade suggestions." This stops the doomed auto-fetch against league 1 entirely.

```typescript
// useApi.ts — useWaiverWire / useFindTrades query options
return useQuery<WaiverTarget[]>({
  queryKey: ['waiver-wire', leagueId, week, position ?? 'All'],
  queryFn: () => fetchJson<WaiverTarget[]>(`/waiver-wire?${params}`),
  retry: false,
  enabled: leagueId > 0 && week > 0,
});
```

- **Fix 2 — browser tab title is the Vite template default.** `frontend/index.html:7` is `<title>frontend</title>`; set it to `<title>GRID Fantasy</title>` to match the in-app brand.

**Tests:** type gate only (no JS test runner present — consistent with T17–T19). Manual verify via the run/browse skill: load `/inseason/waiver` and `/inseason/trades` with no league configured → not-configured empty-state, zero console 404s, no multi-second spinner; entering a real league ID still fetches; confirm the tab title reads "GRID Fantasy."

**Verification:**
```bash
cd frontend && npx tsc -b --noEmit   # exit 0
```

---

### Task 20 — Integration review on `feat/phase4` (branch: `p4/review-fixes`)

**Files touched:** any seam fixes across the above; `docs/superpowers/plans/2026-06-22-fantasy-dashboard-phase4.md` (changelog note).

**Approach:** After all task PRs fold in, run the FULL Standard Verification Block once against `feat/phase4`. Review cross-task seams: (a) Kalman 3×3 state ↔ all `statespace.py` consumers and the npz migration ↔ `weekly_update` roster-rebuild; (b) situation accumulators ↔ canonical ordering (Task 1) ↔ grades-schema (Task 3); (c) corrected matchup-grade semantics (Task 2) ↔ `/matchup-grades` API ↔ heat strip; (d) viz-agent route ↔ frontend render pipeline ↔ axis-family guard; (e) confirm the Anthropic offline-mock constraint holds suite-wide and the viz cache table is created on a pipeline-free boot. Land all fixes on this single branch.

**Verification:**
```bash
python -m pytest -q                 # whole integration branch green, count > 257
cd frontend && npx tsc -b --noEmit  # exit 0
python -c "from fastapi.testclient import TestClient; from backend.api.main import app; p=TestClient(app).get('/openapi.json').json()['paths']; assert all(r in p for r in ('/api/matchup-grades','/api/situation-splits','/api/viz-agent/query')); print('all routes ok')"
```

---

### Task 21 — Single merge of `feat/phase4` into main (branch: merge PR only)

**Files touched:** `.git`, `memory/project-fantasy-dashboard.md`.

**Approach:** Once `feat/phase4` is green on all gates with `p4/review-fixes` folded, open exactly ONE PR `--base main --head feat/phase4`. Re-run the full Standard Verification Block on the merge result, confirm the test count exceeds the 257 baseline with zero failures, update the project memory note (Phases 1-3 → 1-4 complete), and record the merge.

**Verification:**
```bash
git checkout main && git pull && python -m pytest -q   # 0 failures, count > 257
cd frontend && npx tsc -b --noEmit                      # exit 0
```

---

## Task Dependency Graph

```
T0 (tsc baseline, on main) ──► [cut feat/phase4]

T1 (canonical player order) ─┬─► T6 (situation accumulators)
T2 (matchup-grade semantics) ┼─► T12 (viz/data API) ─┬─► T16 (viz route) ─► T19 (agent frontend)
T3 (grades-schema migration) ┘                        │
T4 (situation masks) ─► T5 (situation RAPM) ─► T6 ────┘ (situation_grades rows)
T7 (WR-CB interactions)  [independent of T1–T6 ordering, off by default]

T8 (kalman 3-comp) ─► T9 (npz migration) ─► T10 (coaching changes)
T8 ─► T11 (changepoint detect)
T8/T9 ─► T12 (trajectory table + endpoint reads mu width)

T13 (viz deps/env) ─► T14 (schema + cache table) ─► T15 (agent) ─► T16 (route)
T12 + T13..T16 ─► T17 (FE types/hooks) ─► T18 (FE charts) ─► T19 (agent FE box)
T19b (FE QA fixes: waiver/trade retry + tab title)  [independent FE polish, gated only by T0 tsc-green]

(all task PRs) ─► T20 (p4/review-fixes) ─► T21 (single merge to main)
```

**Edge list (gates):**
- T0 → cut `feat/phase4` (baseline-green prerequisite, includes tsc).
- T1 → T6; T2 → T12; T3 → T6, T12, T9(grades coexistence) ; T4 → T5 → T6.
- T8 → T9, T11, T12(trajectory width); T9 → T10; T9 → T12(trajectory hook).
- T13 → T14 → T15 → T16; T12 → T16(allowlist fields); T16 → T19.
- T12 → T17 → T18 → T19. T19b → T20 (independent; only needs T0's green gate). All → T20 → T21.

**Recommended execution order (with parallelism):**
1. **T0** (on main) → cut `feat/phase4`.
2. **T1, T2, T3, T4, T7, T8, T13** (parallel — independent foundations).
3. **T5, T9** (after T4 / after T8).
4. **T6, T10, T11, T14** (after T1+T3+T5 / T9 / T8 / T13).
5. **T12** (after T2+T3+T5/T6+T8/T9).
6. **T15, T16** (after T14 / after T12+T15).
7. **T17 → T18 → T19** (after T12+T16); **T19b** (independent FE polish — runs any time after T0).
8. **T20** (integration review) → **T21** (merge).

---

## Known Constraints & Trade-offs

| Decision / limit | Honest rationale |
|---|---|
| Situation RAPM works on **synth only**; real-data is blocked | `build_design` requires `off_players`/`def_players`; the real `_normalize_pbp` never emits them and `load_participation` is a `NotImplementedError` stub. Masking on state does not remove that requirement. `run_situation_rapm` asserts the columns exist and raises a clear error — it is exactly as blocked as base RAPM on real data. Wiring participation is an upstream Phase-1 gap, out of scope here. |
| Two-minute situation deferred | Needs a game-clock column (`quarter_seconds_remaining`) not in the plays contract. The classifier auto-activates it the moment the column appears in the contract + `_normalize_pbp`. |
| WR-vs-CB is a **positional approximation**, labeled approximate | True man/zone, slot/outside, blitz coverage is paid (PFF/SIS) and unavailable; free nflverse core pbp lacks it. The interaction pairs on-field WRs with on-field DEF (CB proxy), pruned for support, heavily ridge-penalized, and `interactions=False` by default so the main path and accumulator dimensions are untouched. API output flags it as approximate. |
| Coaching/coordinator changes are **hand-curated** | Not in nflverse rosters (team only). `coaching_changes` is a checked-in seed (~10–20 rows/season) and is the source of truth; `scheme_fit` defaults to 0 (no reset) when absent, so the pipeline runs offline without it. |
| Kalman state-migration risk | Growing 2→3 components is a self-migrating npz pad (talent/form lossless, scheme_fit init `[2,2]=0.01`). The risk: **every** hardcoded 2-d site (`kalman_step` and the offline RTS smoother — `eye(2)` at `:150`, P0 at `:112`, x0 at `:110–111`) must become `N_STATE`-aware in the same commit, and the numerical tests are PORTED to 3×3, not "kept unchanged." Identifiability of talent vs scheme_fit under the additive `H=[1,1,1]` is weak; mitigated by `q_scheme << q_form` and letting scheme_fit move materially only via explicit coaching resets. Validate smoothed scheme_fit on synth before trusting it. |
| Viz-agent security model | The model emits ONLY a validated `ChartSpec` — never SQL, never a free-form query. The server validates `xAxis`/`yAxis`/`position`/`colorBy`/`season` against the `/stats/fields` allowlist (14 fields), repairs near-misses, rejects off-list specs (422), and guards against mixed raw-stat/valuation axis families that would render empty. Data is fetched only through the existing safe `/stats` and `/rankings` endpoints. SQL uses bound parameters with fixed literal column names only. |
| Viz cache scope and a production gotcha | `viz_query_cache` stores the spec (axes+filters), never data, so pipeline refreshes never stale it; no TTL needed. Critical fix: the table must be created on app startup (FastAPI startup event / create-if-missing), because `init_db` only runs via pipeline scripts and the test fixture — a server booting against an existing DB would otherwise hit "no such table." A pipeline-free boot test guards this. |
| Model / cost note | `VIZ_AGENT_MODEL` defaults to `claude-haiku-4-5` for cost; this is surfaced explicitly in `.env.example` (quality upgrade: `claude-opus-4-8`). Every call passes the model explicitly; exact-repeat queries cost zero tokens via the cache. `cache_control` is a no-op below Haiku's 4096-token cacheable prefix and auto-activates once the Task-12 fields enlarge the schema context. |
| `/rankings` ignores `season` | On the valuations axis family the existing `/rankings` route has no season param, so agent-emitted season affects only the `/stats` path. Documented limit; the cache key still includes season for the `/stats` path. |
| Matchup grade is **team-level** until per-position defensive splits land | Task 2 fixes the sign/semantics (defense intercept, sign-flipped) but per-position defensive resolution arrives with the situation passes; the `/matchup-grades` API note flags grades as team-level in the interim and percentile A–F grading is only meaningful with all 32 def_teams present. |

---

## Files Created / Modified Summary

| File | Action |
|---|---|
| `frontend/src/charts/PlayerProfileCharts.tsx` | Modified — T0: fix TS6133 unused param |
| `frontend/src/charts/TradeCharts.tsx` | Modified — T0: fix recharts Formatter types |
| `frontend/src/charts/WaiverCharts.tsx` | Modified — T0: fix recharts Formatter type |
| `backend/grid/layers.py` | Modified — canonical pid sort + order fingerprint (T1); `run_situation_rapm` (T5); situation-keyed accumulators (T6); `interactions=` flag (T7) |
| `backend/pipeline/weekly_update.py` | Modified — matchup-grade semantics fix (T2); situation passes + grades (T6); scheme-reset wiring (T10); per-position changepoint detection (T11); trajectory write hook (T12) |
| `backend/db/schema.sql` | Modified — `situation` column + `ux_matchup_grades` (T3); `coaching_changes` (T10); `situation_grades`, `kalman_trajectory`, `viz_query_cache` (T12, T14) |
| `backend/db/connection.py` | Modified — idempotent `ALTER`/index guards in `init_db` (T3) |
| `backend/grid/situations.py` | Created — situation classifier (T4) |
| `backend/grid/statespace.py` | Modified — 3-component state, all 2-d sites N_STATE-aware (T8); versioned self-migrating npz (T9); `scheme_resets` param (T10); `detect_changepoints` (T11) |
| `backend/grid/synth.py` | Modified — optional CB-split knob for interaction test (T7) |
| `backend/db/seed_coaching.py` | Created — coaching-changes seed loader (T10) |
| `backend/db/data/coaching_changes_2025.json` | Created — hand-curated seed (T10) |
| `backend/api/routes/stats.py` | Modified — add `HTTPException` import; `/matchup-grades`, `/situation-splits`, `/trajectory`; extend `/stats/fields` (T12) |
| `backend/api/routes/season.py` | Modified — situation/matchup grade reads if surfaced via season router (T12) |
| `backend/viz_agent/__init__.py` | Created (T14) |
| `backend/viz_agent/schema.py` | Created — `ChartSpec` + `allowed_field_keys` + `build_schema_context` (T14) |
| `backend/viz_agent/cache.py` | Created — `make_key`/`get_cached`/`put_cached` (T14) |
| `backend/viz_agent/agent.py` | Created — `generate_spec` + `validate_and_repair` + `_structured_call` (T15) |
| `backend/api/routes/viz_agent.py` | Created — `POST /viz-agent/query` (T16) |
| `backend/api/main.py` | Modified — register viz_agent router; startup event to ensure `viz_query_cache` exists (T14, T16) |
| `requirements.txt` | Modified — add `anthropic>=0.49` (T13) |
| `.env.example` | Modified — `VIZ_AGENT_MODEL`, `VIZ_AGENT_CACHE_ONLY` (T13) |
| `frontend/src/types/index.ts` | Modified — `MatchupGradeCell`/`SituationSplit`/`TrajectoryPoint`/`PlayerTrajectory`; EDIT `StatField` add `source?` (T17) |
| `frontend/src/hooks/useApi.ts` | Modified — `useMatchupGrades`/`useSituationSplits`/`useTrajectory`/`useVizAgentQuery` (T17); `retry:false` + `enabled` gate on `useWaiverWire`/`useFindTrades` (T19b) |
| `frontend/src/pages/WaiverWire.tsx` | Modified — T19b: not-configured empty-state |
| `frontend/src/pages/TradeCenter.tsx` | Modified — T19b: not-configured empty-state |
| `frontend/index.html` | Modified — T19b: tab title `frontend` → `GRID Fantasy` |
| `frontend/src/charts/MatchupGradeStrip.tsx` | Created — A–F heat strip (T18) |
| `frontend/src/charts/SituationRadar.tsx` | Created — situation radar (T18) |
| `frontend/src/charts/TrajectoryChart.tsx` | Created — Kalman trajectory line chart (T18) |
| `frontend/src/charts/RecommendedCharts.tsx` | Modified — `matchup`/`situation` cases + trajectory in `player_profile` (T18) |
| `frontend/src/components/VizAgentBox.tsx` | Created — NL input box (T19) |
| `frontend/src/pages/Workbench.tsx` | Modified — mount `VizAgentBox`, wire `onSpec={setSpec}` (T19) |
| `memory/project-fantasy-dashboard.md` | Modified — mark Phase 4 complete (T21) |
| `tests/grid/test_layers.py` | Created/Modified — canonical order + reorder reinit (T1) |
| `tests/grid/test_situations.py` | Created — classifier (T4) |
| `tests/grid/test_layers_situations.py` | Created — `run_situation_rapm` (T5) |
| `tests/grid/test_design_interactions.py` | Created — WR-CB interactions (T7) |
| `tests/grid/test_incremental.py` | Modified — 3-comp shapes + npz migration (T8, T9) |
| `tests/grid/test_kalman_numerical.py` | Modified — port to 3×3 + RTS PD test (T8) |
| `tests/grid/test_coaching_changes.py` | Created — scheme resets (T10) |
| `tests/grid/test_changepoint.py` | Created — changepoint detection (T11) |
| `tests/db/test_schema.py` | Modified — situation column + upgrade-path idempotency (T3) |
| `tests/pipeline/test_weekly_update.py` | Modified — grade semantics (T2), schema (T3) |
| `tests/pipeline/test_weekly_situations.py` | Created — per-situation accumulators (T6) |
| `tests/api/test_routes.py` | Modified — matchup/situation/trajectory + `/stats/fields` (T12) |
| `tests/api/test_season_routes.py` | Modified — situation grade reads (T12) |
| `tests/viz_agent/__init__.py` | Created (T13) |
| `tests/viz_agent/test_config.py` | Created — lazy client/config (T13) |
| `tests/viz_agent/test_schema.py` | Created — ChartSpec/allowlist (14 keys) (T14) |
| `tests/viz_agent/test_cache.py` | Created — cache round-trip + fresh-boot table (T14) |
| `tests/viz_agent/test_agent.py` | Created — repair/clamp/axis-family, mocked client (T15) |
| `tests/api/test_viz_agent_routes.py` | Created — cache-first route, 422, fresh-boot (T16) |
