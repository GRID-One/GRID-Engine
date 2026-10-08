# Fantasy Football Management Dashboard — Design Spec

**Date:** 2026-06-20
**Status:** Draft
**Target:** Draft-ready by mid-to-late August 2026

## 1. Overview

A full-featured, live-updating fantasy football management dashboard built on top of the GRID (Game-state Relative Individual Decomposition) player-value estimation engine. The dashboard covers the full fantasy lifecycle from draft preparation through in-season roster management, with deep analytics, customizable visualizations, and an AI-powered visualization agent.

### Goals

- Provide a competitive analytical edge over premium tools like FantasyPros
- Support multiple leagues (ESPN, Sleeper) and scoring formats (Standard, Half-PPR, Full PPR, custom)
- Automate data pipelines with zero-token, on-device scheduled processes
- Deliver rich, customizable data visualization with intelligent context-aware recommendations
- Build on and extend the existing GRID analytics engine

### Non-Goals

- Live gameday score tracking (may be added later, not in scope)
- Mobile native app (responsive web handles mobile use cases)
- Social features (chat, league message boards)
- Paid data API subscriptions (free data stack only)

## 2. Architecture

### Three-Layer Architecture

```
┌─────────────────────────────────────────────────┐
│                   Frontend                       │
│         React 18 + TypeScript + Vite             │
│    Tailwind CSS / Recharts / TanStack Table      │
├─────────────────────────────────────────────────┤
│                   API Layer                      │
│              Python — FastAPI                    │
│     REST endpoints + WebSocket (draft board)     │
├─────────────────────────────────────────────────┤
│                  Data Layer                      │
│   GRID Engine + Scoring Engine + Data Pipeline   │
│           SQLite + Parquet Cache                 │
└─────────────────────────────────────────────────┘
```

### Frontend Stack

| Technology | Purpose |
|---|---|
| React 18 | Component-based UI framework |
| TypeScript | Type safety across the frontend |
| Vite | Dev server and production build tooling |
| Tailwind CSS | Utility-first responsive styling |
| Recharts | Composable charting (scatter, bar, line, radar, heat map) |
| TanStack Table | Headless data tables with sort, filter, group |
| TanStack Query | Data fetching with caching, background refresh, stale-while-revalidate |

### Backend Stack

| Technology | Purpose |
|---|---|
| Python 3.11+ | Backend language (matches existing GRID engine) |
| FastAPI | REST API framework with automatic OpenAPI docs |
| SQLite | Local database for all structured data |
| Parquet | Cached bulk data files from nflverse |
| joblib | Model persistence for the value model |
| WebSockets (FastAPI) | Real-time updates for the live draft board |

### Responsive Breakpoints

- Desktop: 1200px+
- Tablet: 768px
- Mobile: 375px
- Mobile nav collapses to bottom tab bar; tables switch to card-based layouts

## 3. Data Pipeline

### Data Sources

| Source | Data | Auth | Cost |
|---|---|---|---|
| nflverse | Play-by-play, rosters, stats, participation (5 years) | None | Free |
| ESPN API | ADP, projections, league sync (rosters, scores, draft state) | Cookie-based for private leagues (one-time setup: user exports `swid` and `espn_s2` cookies from browser, stored locally in `.env`) | Free |
| Sleeper API | League sync (rosters, scores, transactions, waivers, draft state) | None (fully open API) | Free |

### Refresh Strategy

| Source | Trigger | Frequency | Method |
|---|---|---|---|
| nflverse historical | Initial setup | Once (bulk pull) | Scheduled script |
| nflverse weekly | In-season | Weekly, day after last game | Scheduled script |
| ESPN ADP/projections | Draft season | Every 6 hours | Scheduled script |
| ESPN league data | In-season | Every 6 hours, hourly on game days | Scheduled script |
| Sleeper league data | In-season | Every 6 hours, hourly on game days | Scheduled script |

### Automation Architecture

All data operations run as on-device scheduled scripts via Windows Task Scheduler — zero tokens consumed, no manual execution required.

- `data_pipeline.py` — orchestrator script registered with Windows Task Scheduler
- Runs on configurable intervals, logs to a local file
- Idempotent — safe to re-run, skips data already pulled
- Emits a health check file the dashboard reads to show "Data last updated: X"
- Delta-loading for weekly in-season updates (append-only)

### Local Cache Layer

- Parquet files stored in `data/cache/` directory
- TTL-based invalidation per data source
- nflverse bulk data cached as yearly Parquet files
- Projection and ADP data cached with 6-hour TTL

### SQLite Schema (Core Tables)

| Table | Purpose |
|---|---|
| `players` | Master player registry — name, team, position, IDs across ESPN/Sleeper/nflverse |
| `player_stats` | Weekly and seasonal stat lines, 5 years deep |
| `projections` | Sourced projections + GRID-generated projections, versioned by date |
| `scoring_formats` | User-defined scoring rules per league |
| `leagues` | League configs synced from ESPN/Sleeper (roster slots, draft order, etc.) |
| `rosters` | Current rosters per league, auto-synced |
| `valuations` | Pre-computed GRID player values per scoring format, refreshed by scheduled job |
| `draft_history` | Picks tracked during live drafts |
| `matchup_grades` | Pre-computed weekly matchup ratings (Phase 4) |
| `saved_views` | User-saved analyst workbench chart configurations |

### Data Contract (Extended from GRID)

The existing GRID data contract is extended with fantasy-relevant fields:

**plays** (extended):
- Original: `game_id, play_id, off_team, def_team, down, ydstogo, yardline_100, next_down, next_ydstogo, next_yardline_100, yards_gained, drive_points, off_players, def_players`
- Added: `pass_attempt, rush_attempt, passer_id, receiver_id, rusher_id, air_yards, yards_after_catch, td_type, fumble, interception`

**players** (unchanged): `player_id, team, position, is_starter`

**market** (unchanged): `team -> implied strength`

**college** (unchanged): `player_id, position, feeder_sv, feeder_snaps, is_rookie`

## 4. Scoring Engine

### Multi-Format Support

Scoring configs are JSON objects defining points-per-stat:

```json
{
  "name": "Half PPR",
  "passing_yards": 0.04,
  "passing_tds": 4,
  "interceptions": -2,
  "rushing_yards": 0.1,
  "rushing_tds": 6,
  "receptions": 0.5,
  "receiving_yards": 0.1,
  "receiving_tds": 6,
  "fumbles_lost": -2,
  "two_point_conversions": 2
}
```

- Built-in presets for Standard, Half-PPR, Full PPR
- Fully custom configs supported
- Each league has its own scoring config, synced from ESPN/Sleeper on setup and editable locally
- All downstream computations (rankings, valuations, projections) flow through this engine — changing a scoring format ripples everywhere automatically

### League Abstraction

A unified `League` interface wraps ESPN and Sleeper behind the same API:

- `sync()` — pull latest rosters, scores, transactions
- `get_roster(team)` — roster for a specific team in that league
- `get_scoring_config()` — that league's scoring rules
- `get_draft_info()` — draft order, pick slots, format (snake/auction)
- `get_available_players()` — free agents in that league
- `get_transactions()` — recent adds, drops, trades

The frontend never knows whether it's talking to ESPN or Sleeper. Adding a third platform (Yahoo, NFL.com) means writing one new adapter.

### Value-Over-Replacement (VOR) Calculator

- Computes positional baselines from league roster requirements (e.g., 12-team, start 2 RBs → RB24 is replacement level)
- Ranks players by VOR within each scoring format
- Tiering algorithm groups players into value clusters — tiers are more useful than ranks for draft decisions

### Fantasy Scoring Projection Layer

New module `fantasy_scoring.py`:

- Takes GRID EPA-based ratings and historical stat distributions to project fantasy points per scoring format
- Position-specific stat-line projectors: given a player's GRID rating and team context, project attempts/completions/yards/TDs by type
- Outputs feed directly into VOR and tiered rankings

## 5. GRID Engine Evolution

The existing GRID engine evolves in sync with each dashboard phase.

### Phase 1 — Foundation + Draft Rankings

**Data adapters:**
- Implement `load_nflfastr_pbp(years)` — pull 5 years of play-by-play from nflverse, map to the extended plays contract
- Implement `load_participation(years)` — nflverse participation data (2016+), join player IDs to plays
- Build local Parquet cache layer with TTL-based invalidation and delta-loading
- Wire up ESPN and Sleeper API adapters

**Scoring projection layer:**
- New `fantasy_scoring.py` module
- Position-specific stat-line projectors
- VOR calculator with league-specific roster requirements

**Performance improvements:**
- Vectorize `build_design()` — replace `itertuples` loops with sparse matrix construction via coordinate lists
- Add `joblib` model persistence for the value model
- Pre-compute and cache the design matrix

### Phase 2 — Draft Tools

No GRID engine changes — draft tools consume Phase 1 outputs (VOR tiers, projections, rankings).

### Phase 3 — In-Season Management

**Incremental pipeline:**
- Running `XtX`/`Xty` accumulators for RAPM — append each week's plays without re-solving the full season
- Kalman filter in append-only mode — new week's observation extends existing filtered state
- Value model stays fixed within a season (re-fit only on major data changes)
- Scheduled weekly pipeline: pull new data → update accumulators → re-solve RAPM → update Kalman → refresh projections → cache results

**Extended attribution:**
- Run weekly credit for all QBs (not just focus QB)
- Target-share attribution for WRs/TEs
- Position-specific lambda values in RAPM

**Priors expansion:**
- Draft capital and age adjustments in `build_priors()`
- Multi-league equivalency (FBS, FCS, UFL separately)

**Trade valuation model — new module `trade_model.py`:**

The trade valuation model computes a context-aware value for every player that accounts for factors raw fantasy points ignore:

- **Remaining-season value (RSV):** GRID-projected fantasy points summed across remaining weeks, discounted by injury probability per week (derived from Kalman filter variance — high form variance = higher injury/bust risk)
- **Schedule-adjusted RSV:** RSV weighted by matchup difficulty per week using RAPM defensive ratings at the player's position (a WR facing 3 tough CB matchups in the playoffs is worth less than one facing 3 soft matchups)
- **Positional scarcity multiplier:** how replaceable is this player's production from the waiver wire or your bench? Uses the VOR gap between the player and the best available replacement at their position in that specific league
- **Roster-context adjustment:** the marginal value of a player depends on who else is on your roster. A WR3 on a team with 3 elite WRs has less marginal value than the same WR3 on a WR-needy team. Computed as the delta in total projected roster points with vs. without the player
- **Trajectory premium/discount:** players with upward GRID talent trajectories (Kalman smoothed τ trending up) get a premium; declining players get a discount. This captures "buy low / sell high" dynamics that point-in-time projections miss

Trade scoring:
- Each side of a trade is scored as the sum of roster-context-adjusted RSV for all players involved
- A trade is "fair" when both sides gain RSV (positive-sum due to roster context differences)
- The Trade Finder uses this model to search the space of possible trades and rank by mutual benefit

### Phase 4 — Matchup Models + Smart Viz

**Matchup-level RAPM:**
- WR-vs-CB interaction columns in `build_design()` using positional approximation from nflverse (PFF/SIS alignment data if available)
- Situation-specific RAPM passes: red zone, passing downs, rushing downs, 2-minute drill
- Coverage-type features: zone vs. man, blitz rate, slot vs. outside alignment

**Extended Kalman state:**
- Grow `F` matrix to 3x3: `[talent, form, scheme_fit]`
- `scheme_fit` resets on coaching/coordinator changes
- Position-specific `SSParams` (QBs get higher `d_steady`, DEF gets higher `r_scale`)
- Automatic intervention detection via changepoint analysis on observations

**Smart visualization data:**
- All matchup grades, situation splits, and trajectory data exposed as structured JSON endpoints

## 6. Frontend Design

### Navigation

Top-level navigation with four main views:

| View | Purpose |
|---|---|
| **Dashboard** | Landing page — league selector, at-a-glance roster health cards, upcoming matchups, top waiver targets |
| **Rankings** | Tiered player rankings with scoring format selector, position filters, search |
| **Draft Room** | Mock draft simulator + live draft board with real-time pick tracking (Phase 2) |
| **Analyst Workbench** | Deep exploration space for custom visualizations |

On mobile, nav collapses to a bottom tab bar.

### Dashboard View

- League selector dropdown at the top
- Cards grid showing:
  - Your roster with injury/bye indicators
  - This week's matchup projections
  - Top available waiver targets (Phase 3)
  - GRID-powered suggestions panel ("Consider trading X — declining trajectory detected")
  - Data pipeline health status ("Last updated: 2 hours ago")
  - Trade Finder alerts ("3 trades found that improve your roster")

### Rankings View

- Scoring format selector (switches between league formats)
- Position filter tabs (All, QB, RB, WR, TE, K, DST)
- Sortable columns: rank, tier, player name, team, bye, projected points, VOR, ADP, ADP diff
- Tier bands visually separate value clusters with alternating background colors
- Click a player row to expand an inline profile card with trajectory chart and key stats
- Search bar for finding specific players

### Draft Room (Phase 2)

**Mock Draft Simulator:**
- Configure: league size, draft position, scoring format, snake/auction
- AI opponents draft using VOR-based positional need logic
- Your pick shows recommended players ranked by VOR with tier context
- Post-draft grade and roster analysis

**Live Draft Board:**
- Connects to ESPN/Sleeper draft via WebSocket
- Real-time pick grid showing all selections
- "My Queue" — pre-ranked watchlist of targets
- Available players panel with live VOR recalculation as picks happen
- Recommendation panel: "Best available by VOR" with positional need context

### In-Season Views (Phase 3)

- **Start/Sit Advisor** — side-by-side comparison of lineup options with projected points, matchup grades, floor/ceiling ranges
- **Waiver Wire** — ranked free agents by projected value-add to your roster, with GRID trajectory context
- **Trade Evaluator** — input a proposed trade, see projected roster impact across remaining weeks
- **Weekly Matchup** — your team vs. opponent with positional breakdowns and win probability

**Trade Finder & Analyzer (Phase 3):**

- **Trade Finder** — proactively scans all possible trades across your leagues and surfaces the best opportunities:
  - Identifies your roster weaknesses (positional scarcity, bye week conflicts, injury risk concentration)
  - Scans other teams' rosters for complementary surpluses (e.g., a team hoarding 3 startable RBs while thin at WR)
  - Generates trade proposals ranked by net roster improvement for both sides
  - Filters by trade likelihood — avoids proposing trades no rational manager would accept
  - Configurable: how many players per side (1-for-1, 2-for-1, 2-for-2), positions to target, teams to exclude
  - Refreshes automatically via the scheduled pipeline — new suggestions surface as rosters and projections change

- **Trade Analyzer** — evaluate any trade (proposed or hypothetical):
  - Input: select players from each side of the trade
  - Output: projected roster impact across remaining weeks for both teams
  - Shows: total projected points before/after, positional depth change, playoff schedule strength impact, GRID trajectory comparison for players involved
  - Win probability delta — how the trade shifts your expected wins for the rest of the season
  - "Fairness meter" — shows whether the trade is balanced, a steal, or an overpay based on the custom valuation model

- **Trade History & Regret Tracker** — logs all completed trades in the league and retroactively scores them as the season plays out (who "won" the trade based on actual production)

## 7. Visualization System

### Component 1: Analyst Workbench

A dedicated page for building custom views:

- **Axis picker** — drag any stat or metric onto X/Y axes of a scatter plot, or into a bar/line chart
- **Filter bar** — position, team, scoring format, date range, situation (red zone, passing downs, etc.)
- **Chart types** — scatter, bar, line, radar, heat map, distribution histogram
- **Comparison mode** — select 2-5 players and overlay their stats across any dimensions
- **Saved views** — save a chart configuration and recall it later (stored in SQLite `saved_views` table)
- **Export** — PNG or CSV of any chart or underlying data

All data comes from pre-computed API endpoints — chart rendering is instant.

### Component 2: Situation-Aware Recommended Charts

Context-sensitive visualizations that appear automatically based on the current view:

| Context | Recommended Charts |
|---|---|
| Player profile | GRID trajectory (Kalman talent + form), weekly fantasy point trend, upcoming matchup grade |
| Comparing two WRs | Radar chart of target share, air yards, red zone looks, YAC; CB matchup history |
| Viewing your roster | Positional balance chart, bye week distribution, projected points by week |
| Draft board active | VOR tier chart of remaining players, positional scarcity curves |
| Waiver wire | Value-add scatter (projected points vs. ownership %), GRID trajectory of top targets |
| Trade Finder | Side-by-side RSV comparison, roster projection before/after, fairness meter gauge |
| Trade Analyzer | Multi-week projected points chart (with/without trade), positional depth bar chart, trajectory overlay for traded players |

Each template is a React component that fetches its own data and renders independently.

### Component 3: Claude Viz Agent

A text input box available on any page:

- Input goes to a FastAPI endpoint that calls the Claude API with the query + available data schema
- Claude generates a chart specification (chart type, axes, filters, data query) as structured JSON
- The frontend renders the spec using the same Recharts components the workbench uses
- Query results are cached — repeated or similar queries return instantly without token cost

**Example queries:**
- "Show me RB1s who have soft run defense matchups weeks 1-4"
- "Compare Ja'Marr Chase and Amon-Ra St. Brown target share in the slot over the last 2 seasons"
- "Which QBs have the biggest gap between their GRID rating and their ADP?"
- "Histogram of WR fantasy points per game in half-PPR, colored by tier"

The Claude call is the only token-consuming operation in the dashboard. Cost is minimized by:
- Caching query → chart spec mappings
- Keeping the data schema context compact
- Using Claude Haiku 4.5 for chart spec generation (fast, cheap, handles structured JSON output well; upgrade to Sonnet if quality requires it)

## 8. Project Directory Structure

```
cautious-nevermore/
├── backend/
│   ├── grid/                  # Existing GRID engine (moved from root)
│   │   ├── __init__.py
│   │   ├── synth.py
│   │   ├── value.py
│   │   ├── layers.py
│   │   ├── statespace.py
│   │   ├── priors.py
│   │   └── data_adapters.py
│   ├── api/                   # FastAPI application
│   │   ├── __init__.py
│   │   ├── main.py            # FastAPI app entry point
│   │   ├── routes/            # Route modules (players, rankings, drafts, leagues, viz)
│   │   └── websockets/        # WebSocket handlers (draft board)
│   ├── adapters/              # Platform API adapters
│   │   ├── espn.py
│   │   └── sleeper.py
│   ├── scoring/               # Scoring engine
│   │   ├── engine.py
│   │   ├── formats.py         # Built-in scoring presets
│   │   └── vor.py             # Value-over-replacement calculator
│   ├── trades/                # Trade valuation system
│   │   ├── trade_model.py     # RSV, scarcity, roster-context valuation
│   │   ├── trade_finder.py    # Proactive trade opportunity scanner
│   │   └── trade_analyzer.py  # Evaluate specific trade proposals
│   ├── pipeline/              # Automated data pipeline
│   │   ├── data_pipeline.py   # Orchestrator (Task Scheduler entry point)
│   │   ├── compute_valuations.py
│   │   ├── sync_leagues.py
│   │   └── health_check.py
│   ├── viz_agent/             # Claude Viz Agent backend
│   │   ├── agent.py           # Claude API integration
│   │   ├── schema.py          # Data schema context for Claude
│   │   └── cache.py           # Query → chart spec cache
│   ├── db/                    # Database
│   │   ├── schema.sql         # SQLite schema definitions
│   │   └── migrations/        # Schema migration scripts
│   └── fantasy_scoring.py     # GRID → fantasy point projection layer
├── frontend/
│   ├── src/
│   │   ├── components/        # Shared UI components
│   │   ├── pages/             # Dashboard, Rankings, DraftRoom, Workbench
│   │   ├── charts/            # Recharts chart components
│   │   ├── hooks/             # Custom React hooks (data fetching, etc.)
│   │   ├── types/             # TypeScript type definitions
│   │   └── utils/             # Shared utilities
│   ├── public/
│   ├── index.html
│   ├── package.json
│   ├── tsconfig.json
│   ├── tailwind.config.js
│   └── vite.config.ts
├── data/
│   ├── cache/                 # Parquet cache (nflverse bulk data)
│   └── db/                    # SQLite database file
├── docs/
│   └── superpowers/specs/     # Design specs
├── .env                       # ESPN cookies, API keys, config
├── requirements.txt           # Python dependencies
└── README.md
```

## 9. Phased Delivery Plan

### Phase 1 — Foundation + Draft Rankings (~2-3 weeks)

**Deliverables:**
- nflverse data pipeline (bulk pull, Parquet cache, 5 years of data)
- ESPN and Sleeper API adapters (league sync, ADP, projections)
- SQLite database with core schema
- Scoring engine with multi-format support
- GRID engine extended with real data loaders, play-type decomposition, vectorized performance
- Fantasy scoring projection layer and VOR calculator
- FastAPI backend with REST endpoints
- React frontend: Dashboard view, Rankings view with tiered rankings
- Analyst Workbench (Component 1) with full chart type support
- Situation-aware recommended charts (Component 2)
- Windows Task Scheduler automation for data pipeline
- Responsive design (desktop + mobile)

### Phase 2 — Draft Tools (~2 weeks)

**Deliverables:**
- Mock draft simulator with AI opponents
- Live draft board with ESPN/Sleeper WebSocket integration
- Draft queue and recommendation panel
- Post-draft roster grade and analysis

### Phase 3 — In-Season Management (~2 weeks)

**Deliverables:**
- Incremental GRID pipeline (weekly RAPM updates, Kalman append, cached models)
- Start/Sit advisor with matchup grades
- Waiver wire ranker
- Custom trade valuation model (RSV, scarcity multiplier, roster-context adjustment, trajectory premium)
- Trade Finder — automated trade opportunity scanner across all leagues
- Trade Analyzer — evaluate any proposed or hypothetical trade
- Trade History & Regret Tracker
- Weekly matchup view with win probability
- Extended RAPM (all QBs, WR/TE target-share attribution, position-specific lambda)
- Priors expansion (draft capital, age, multi-league equivalency)

### Phase 4 — Matchup Models + Smart Viz (Ongoing)

**Deliverables:**
- Matchup-level RAPM (WR-vs-CB, situation splits, coverage types)
- Extended Kalman state (talent, form, scheme_fit)
- Automatic intervention detection
- Claude Viz Agent (Component 3) with natural language → chart pipeline
- Advanced situation-aware chart templates using matchup data

## 10. Disk Space & Performance Budget

### Storage

| Component | Size |
|---|---|
| nflverse historical data (5 years, Parquet) | 500MB - 1GB |
| Player stats, rosters, projections (SQLite) | ~50MB |
| User data (league configs, saved views) | <1MB |
| Python dependencies | 500MB - 1GB |
| Node modules | 200-400MB |
| **Total** | **~1.5 - 3GB** |

### Performance Targets

| Operation | Target Latency |
|---|---|
| Page load (cached data) | <500ms |
| Rankings table render | <200ms |
| Chart render (pre-computed data) | <300ms |
| RAPM full solve (vectorized) | <1 second |
| Weekly pipeline refresh | <60 seconds |
| Claude Viz Agent (uncached) | 2-5 seconds (API round trip) |
| Claude Viz Agent (cached) | <200ms |

## 11. Automation Summary

All recurring operations run on-device via Windows Task Scheduler — zero tokens, no manual execution:

| Script | Schedule | Purpose |
|---|---|---|
| `data_pipeline.py` | Configurable (6h default, 1h on game days) | Pull fresh data from nflverse, ESPN, Sleeper |
| `compute_valuations.py` | After each data pull | Re-run GRID pipeline, update projections and VOR |
| `sync_leagues.py` | Every 6 hours | Sync rosters, transactions, and standings from ESPN/Sleeper |
| `health_check.py` | Every hour | Verify data freshness, log pipeline status |

The dashboard reads pre-computed results from SQLite — the frontend never triggers model computation. Claude is only invoked for the Viz Agent (Component 3), and even those calls are cached.
