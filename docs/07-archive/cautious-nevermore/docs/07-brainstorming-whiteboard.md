# GRID Fantasy — Brainstorming Whiteboard

> Open questions, ideas, and design decisions to revisit. Not commitments — thinking out loud.

## Projection Model

### Volume model enhancement
The current volume model is an empirical-Bayes shrinkage of prior-season usage. The dominant weakness is offseason role changes — a player who changes teams or gets a new coaching staff may have a completely different role. The ADP/depth-chart override hook is the mitigation, but it's manual.

**Idea:** Could we use ADP as an automated volume signal? ADP reflects market consensus on expected role. A player whose ADP jumped from round 8 to round 3 likely got a volume bump. This could auto-detect role-changers without manual overrides.

**Idea:** Age curves. The current model treats a 28-year-old RB the same as a 22-year-old. Aging is a step function in the current code (GitHub #49). A smooth age curve (e.g., quadratic in age with position-specific coefficients) would be more realistic.

### Talent feature integration
Only `rapm_rating` is live in the ROS forecast. `smoothed_talent` was wired in (PR #91) but `prior_mean` stays 0.0 (blocked on wiring `priors.py` to real data). The H1 verdict was a tie (-0.015) — would adding `prior_mean` move it?

**Idea:** The prior is the rookie cold-start signal. For non-rookies it's NaN (currently crashes the model — see P1). If we fix the NaN crash and wire `prior_mean` for rookies only, does H1 move for the rookie subset specifically?

### Market baseline
The validation suite has a `market` baseline (ADP/ESPN projections) but it's report-only. If GRID beats the market baseline, that's the real value signal — last-season-actuals is a weak bar. Should we promote market to a gate?

## Validation

### Tier 2 is still skipped
Tier 2 (attribution/matchup backtest) is scaffolded but skipped because there's no points-allowed feed. The matchup grades exist in the DB but haven't been validated against real defensive performance.

**Idea:** Could we derive a points-allowed proxy from nflverse play-by-play? The drive-points column already exists. Group by defense team and week → weekly points allowed. This might unblock Tier 2 without a new data source.

### Cross-season watermark gap
The watermark guard skips season boundaries (V2). This is a defense-in-depth blind spot, not a primary-path leak. But it's exactly where multi-season ordering is most fragile.

**Idea:** Change the guard to check `last_slot` against the current origin's cutoff unconditionally, regardless of season. The slot ordering is already chronological, so the last accumulated week should always be < the current origin week.

## Frontend

### Dead WebSocket
`useLiveDraftSocket` is fully implemented but never wired up. The draft UI relies on synchronous mutation responses. If the server processes AI picks asynchronously, the UI gets stuck.

**Question:** Should we wire it up or remove it? If the mock draft API processes all AI picks synchronously in the response, the WebSocket is unnecessary for mock drafts. For live drafts (Tier-B), it would be needed. Decision: keep for now, wire when live draft sync is implemented (GitHub #34).

### LLM chart spec validation
`getVal` in ChartRenderer returns 0 for missing axis fields. An LLM-generated spec with a hallucinated field name renders all points at (0,0) silently.

**Idea:** Add client-side validation: after receiving a ChartSpec from the viz agent, check that the xAxis and yAxis fields exist in the fetched data. If not, show an error state instead of rendering a degenerate chart.

### No error boundaries
No React error boundaries anywhere. A single unhandled TypeError (e.g., `null.toFixed(1)`) crashes the entire page with no recovery.

**Idea:** Add an error boundary at the page level. When a page crashes, show a "Something went wrong" state with a retry button instead of a white screen.

## Engine

### Scheme_fit component
The 3-component Kalman state `[talent, form, scheme_fit]` is implemented, but `scheme_fit` resets on coaching changes. The coaching changes are hand-curated (`seed_coaching.py`). How sensitive is the model to missing coaching changes? If we miss a coordinator change, `scheme_fit` doesn't reset and the model carries a stale value.

**Idea:** Automatic changepoint detection via the Kalman innovation sequence. A spike in the innovation at week W is evidence of a regime change. This is scaffolded in the validation suite (Tier 0.5 changepoint detection) but not wired to the production engine.

### Position-specific lambda
RAPM uses a single ridge penalty for all players. Position-specific lambda (e.g., lighter penalty for QBs who have more informative per-play signals, heavier for defensive players) is a GitHub issue (#32). The infrastructure exists (`lambda_by_pos` in `run_rapm`) but the position-specific values aren't calibrated.

**Idea:** Calibrate lambda per position by maximizing OOS recovery correlation on synth data. The synth generator knows the true abilities, so we can measure which lambda gives the best per-position recovery.

## Packaging

### conda-constructor vs. alternatives
The plan is conda-constructor + pywebview. But Python packaging on Windows is fiddly. Alternatives:
- **PyInstaller** — simpler but larger bundle and less reproducible
- **Nuitka** — compiles to C, faster startup but harder to debug
- **Tauri** — the plan says Tauri for beta, but could we skip to Tauri for alpha and use Rust for the backend shell?

**Current decision:** conda-constructor for alpha (Phase 4). Defer Tauri to beta.

## Product

### Real roster H2
The H2 verdict used synthetic VOR-drafted rosters. The validation plan calls for replacing these with real post-draft league rosters (synced via `sync_leagues`) between draft day and Week 1. This would validate H2 against actual lineups managers fielded.

**Question:** Is this worth doing before alpha? The synthetic H2 already passed (+0.848). Real rosters would make the result more convincing but require league sync (Tier-B).

### Multi-league arbitrage
GitHub #36: compare rankings across leagues to find players who are valued differently. Not implemented. Would be a unique feature — most fantasy tools are single-league.