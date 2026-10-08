# GRID Fantasy — Lessons Learned

> A log of mistakes, near-misses, and corrections discovered during the build. Each entry is a concrete lesson, not a generic best practice. Organized by category.

## Architecture / Planning

### The nflverse→GRID adapter was nearly invisible
The biggest planning error: the nflverse→GRID plays-contract adapter was scoped as "plumbing exists / thin wrapper" when it was actually net-new work (drive segmentation, next-state derivation, next-score labeling, participation join — logic that existed only in `synth.py`). This was caught by a verification pass, not by the original plan. The lesson: verify that the real-data path actually produces the contract the engine expects, don't trust the docstring that says "stubs sketched."

### Baseline-interim decoupling was rejected
An alternative was to ship a baseline-powered app early for the draft, then upgrade to GRID. Rejected per "GRID-powered or bust." Recorded so the option is visible if the stance changes.

## Statistical / Numerical

### Market reconciliation sign error (G1 + V1)
The design matrix uses `+1` for offense and `-1` for defense, so team rating = `beta[t_off] - beta[t_def]`. But the Layer 3 market pseudo-observation used `+1` for BOTH intercepts, constraining `gamma_off + gamma_def ~ market_strength` instead of `gamma_off - gamma_def ~ market_strength`. This bug existed in TWO places: `layers.py:404-405` (production) and `backtest.py:101-102` (validation). The prior audit caught the first; the second was missed because `backtest.py` has a separate `solve_rapm` function. Lesson: when a bug exists in one code path, check all sibling paths that duplicate the same pattern.

### Variance of sum missing covariance term
`var_total = sigma[0,0] + sigma[1,1]` omits the `2*covariance` term. This appeared in `weekly_update.py:124`, `stats.py:262`, and `statespace.py` — three independent locations with the same bug. For a 3-component state `[talent, form, scheme_fit]`, the correct formula is `H @ sigma @ H^T` where `H=[1,1,1]`. The tests only asserted `var > 0`, not that the value was correct — too weak to catch the bug.

### fumbles_lost counts ALL fumbles, not just fumbles lost
nflverse `fumble` column means "a fumble occurred," not "fumble was lost." The pipeline summed all fumbles, penalizing players for fumbles their team recovered. Lesson: always verify column semantics against the data dictionary, don't infer from the name.

### NaN prior_mean crashes the stat-line model
`talent_row.get(c, 0.0)` returns NaN (not 0.0) when the key is present with value NaN. `features.py:67` intentionally fills `prior_mean` with `float("nan")` for non-rookies. The `.get(c, 0.0)` pattern doesn't handle present-but-NaN — it only handles absent keys. This crashed `Ridge.predict` for every non-rookie. Lesson: `dict.get(key, default)` handles missing keys, not NaN values. When a dataclass intentionally fills NaN, the consumer must guard for it.

### max(0.0, NaN) silently returns 0.0
In Python, `max(0.0, float('nan'))` returns `0.0` — silently discarding the NaN value. In `fantasy_scoring.py`, this meant empty historical columns silently zeroed out GRID rating contributions. An absent column correctly got the GRID contribution; a present-but-empty column got 0.0. Same logical case, different results. Lesson: never use `max()` with potentially-NaN values; use `np.nanmax` or guard explicitly.

### Float truthiness in default-value patterns
`drive_points = drive_points or 0.0` uses Python truthiness on a float. `0.0` is falsy, so `or` replaces it. Currently works by coincidence (both operands are 0.0) but would break if the fallback value changed. Lesson: use explicit assignment, never `or` for default-value assignment on numeric types where 0.0 is a valid value.

## Data Pipeline

### JSON nesting mismatch between writer and reader
`ScoringConfig.to_json()` produces `{"name": ..., "rules": {...}}`, but `sync_trade_history.py` did `config.get(col, 0.0)` at the top level instead of `config["rules"].get(col, 0.0)`. Always returned 0.0 for every stat. Lesson: always use `from_json()` to deserialize, or access through the nested structure. Add a round-trip test.

### INSERT OR IGNORE returns wrong record on name collision
`resolve_or_create_format` checks for an existing record by content, then falls back to `INSERT OR IGNORE` + lookup by name. If a row with the same name but different content exists, the INSERT is silently ignored and the lookup returns the wrong record. Lesson: use `INSERT ... ON CONFLICT(name) DO UPDATE SET config_json = excluded.config_json` (upsert by content), or after INSERT OR IGNORE, check whether the row's content matches.

### Projections week=0 vs queries for weeks 1-18
One module writes season-aggregate projections with `week=0`. Another queries `WHERE week IN (1,...,18)`. Week 0 never matches. RSV always returns 0.0. Lesson: cross-module contracts on sentinel values must be documented and tested at both ends.

### Health check reports "ok" on total failure
The pipeline wraps each year in try-except-continue, then writes `status="ok"` unconditionally. If every load failed, health check still says "ok" with `total_players=0`. Lesson: check if totals are zero after the loop.

### Sleeper 2-point conversion scoring triple-counted
Three source keys (`pass_2pt`, `rush_2pt`, `rec_2pt`) all map to the same target (`two_point_conversions`). The accumulation loop sums them: 2+2+2=6, not 2. Lesson: when multiple source keys map to one target, determine whether they're additive components or alternatives. Use max/first if alternatives, not sum.

## Validation / Backtesting

### The ROS verdict was scored by the wrong projection
`verdict.py` fed the weekly SV→points affine map to both the weekly and ROS horizon reports. The design assigns ROS to the season stat-line model, not the weekly map. The H1 verdict (−0.864) reflected scoring the wrong projection, not a settled verdict on GRID. After fixing, the margin moved to −0.015 (a tie). Lesson: the docstring of one module explicitly says "that one drives ROS, this one the weekly lever" — the wiring contradicted the module's own documentation.

### The verdict ran on a broken player universe
The CLI fed `build_design` a fantasy-skill-only players frame, so RAPM dropped ~22k participants/origin as unknown IDs. The fix (full nflverse roster as the RAPM universe) barely moved ROS (volume-dominated) but materially helped H2. Lesson: the player universe for RAPM must include all participants, not just the players you care about scoring.

### Watermark guard skips season boundaries
The defense-in-depth watermark check in `walk_forward` only fires `if last_slot[0] == s` (same season). At a season seam, the guard silently disables — exactly where multi-season ordering is most fragile. Lesson: defense-in-depth guards must cover the edge case they're most needed for, not just the common case.

## Frontend

### .toFixed(1) on potentially null API values crashes tables
`RankedPlayer.vor` and `projected_points` are typed `number` (non-nullable), but the API can return null. `null.toFixed(1)` throws TypeError with no error boundary to catch it. Lesson: TypeScript types are compile-time contracts, not runtime guarantees. When the API is untyped (no Pydantic response model), use optional chaining or null guards in render code.

### useLiveDraftSocket is dead code
The WebSocket hook is fully implemented but never imported by any component. Combined with `staleTime: Infinity` and `refetchInterval: false` on `useDraftState`, the draft UI relies entirely on synchronous server-side AI processing in mutation responses. If the server doesn't auto-advance AI picks, the UI gets permanently stuck. Lesson: a feature that's implemented but not wired is a liability — either wire it or remove it.

## Process

### Subagent self-correction is valuable
The validation subagent flagged `solve_rapm` market mutation as a CRITICAL bug, then re-checked the code, found the `.copy()` calls protect the accumulators, and withdrew the finding. This is the system working correctly — a subagent that verifies its own claims before reporting. Lesson: trust but verify — subagent reports are self-reports, not verified facts. The verification step is what separates a trustworthy audit from a plausible-looking one.

### Tests that only assert > 0 don't catch wrong values
The `var_total` bug (missing covariance term) had a passing test that only asserted `var > 0`. The test was too weak to catch the bug. Lesson: tests should assert the expected VALUE, not just a property of it. A test that only checks `> 0` would pass for any positive number, including wrong ones.

### Reading test files distinguishes bugs from intentional design
The `np.nanmean(y[:3])` guard looked like a potential bug (could warn on all-NaN input), but reading the test confirmed `np.any(np.isfinite(y[:3]))` prevents the all-NaN call. The guard was intentionally tested. Lesson: read the tests to understand the contract the code is supposed to satisfy.