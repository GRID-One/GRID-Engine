# Inventory: `final-build-spec.md` for the engine-only pivot

**Source analysed:** `/home/user/GRID-Engine/final-build-spec.md` (818 lines) at commit `3823478`, branch `claude/grid-engine-consolidation-e7kmh9`.
**Compared against:**
- `alpha-spec.md` (2186 lines). `docs/00-meta/specs/alpha-spec.md` is a byte-identical mirror; I checked with `diff -q`.
- The Python reference engine in `/home/user/cautious-nevermore` at `59bce1d`, which becomes the parity oracle under `reference/python/`.

**Line references:** `L<n>` means a line of `final-build-spec.md`. `A§x` means an alpha-spec section. Python paths are relative to the cautious-nevermore repository root.

**Disposition key:**
- **KEEP**: the requirement stands as written.
- **KEEP-R**: the requirement stands but must be reworded for an engine with no UI.
- **REPLACE**: the GRID-specific definition supersedes the generic one.
- **DROP**: app-only content that does not go into the engine spec.

---

## 0. Headline findings

1. **Roughly 60% of the binding content carries over to the engine.** §7–§21 nearly all survive. Only §19.5, two bullets of §20 and one bullet of §18 are app-only. §3, §4 and §6 are APP-ONLY apart from a few transferable rules. §1, §2, §5, §22, §23 and §24 are MIXED.
2. **§11 is too thin to serve as a Rust port spec for GRID.** The single documented objective (§11.3, L390–392: `argmin ‖y−Xβ‖² + λ‖β‖²`) leaves out four things the oracle uses:
   - observation weights;
   - per-column penalty scaling;
   - a non-zero prior mean;
   - market pseudo-observations.

   It also conflicts with §11.2 and §11.3's own lists, which require "weighted observations", "regularized intercept handling" and "possession weighting". The engine spec needs the generalized objective (see §11.3 below).
3. **The §12.1 pipeline topology does not fit GRID.**
   - It draws Kalman, RAPM and EB as parallel branches feeding a downstream "Gradient Boosting" step.
   - In GRID, boosting comes **first**: the V(s) value model (`backend/grid/value.py`) produces `dV`, and the Layer-1 cross-fitted context model also uses boosting.
   - RAPM consumes `dV`, and the Kalman filter **consumes** the weekly RAPM / Layer-1 output (`statespace.kalman_step` docstring: "this week's per-player RAPM estimates").
   - §23 Phase 4 orders the steps differently again: Kalman → RAPM.

   The engine spec must replace these with an explicit GRID DAG.
4. **The Kalman time index must be the NFL game-week, not the fetch day.** §11.4 says "apply the day's worth of observations as a batch". If the filter runs a predict step on every daily run, talent variance is inflated by `1/d` per *day* instead of per *week* (`statespace.py` `Pp[0,0] /= d`). Days with `NO_NEW_DATA` must not advance the filter.
5. **The oracle has no fixed-lag smoothing (§11.5).** It has full RTS only. There is an exact parity target for the new code: a fixed-lag backward pass over window `L` must equal full RTS on the same indices.
6. **`IncrementalBooster` (§11.8) is under-specified.**
   - There are no labels, weights, seed, hyperparameters, or diagnostics in the signatures.
   - The oracle uses sklearn `HistGradientBoostingRegressor` with no continuation.
   - Boosting parity can therefore only be statistical, not numerical.
7. **Snapshot-by-copy and rollback (§12.3) contradict immutable versioning** (§14 L581, §16 rule 5). The engine spec should implement rollback as moving a production pointer over immutable versions.
8. **The live RAPM path conflicts with alpha-spec.**
   - final-build §11.3 calls RAPM "a first-class production model" and runs an incremental RAPM update daily (§12.1).
   - A§1.2 and A§6.2 forbid RAPM as a hidden live dependency because in-season participation is unavailable.
   - GRID's Layer 2 **and** its Kalman observations depend on participation (`nflverse_adapter.py` L13–15, L50: participation is "real for completed seasons").

   This is an owner decision.
9. **The scheduler contradicts the non-goals.** §9.1 and §22 put a "background scheduler" inside the running process. §24 lists "Always-running server process" as a non-goal. For an engine with a CLI, the clean reading is a one-shot `update` command triggered by an OS scheduler; §9.6 already points that way.
10. **final-build-spec has about 12 internal defects** (§3 below). Examples:
    - "Four levels of testing" introduces five subsections.
    - The catch-up predicate `last_successful_update > configured_interval` compares a timestamp with a duration.
    - The §12.1 diagram is garbled.
    - The §12.3 snapshot list omits RAPM state.
    - `TrainingStatus` cannot represent the §9.5 failure states or the §13 model states.

---

## 1. Classification table (every numbered section and subsection)

| § | Title | Lines | Class | Disposition | One-line reason |
|---|---|---|---|---|---|
| (title) | "Native Windows Sports Analytics Application — Final Build Specification" | 1 | APP-ONLY | DROP (retitle) | Product framing is a Windows application. |
| 1 | Product and Architecture Requirements | 3–22 | MIXED | KEEP-R | Req 1 (Flutter) is dropped. Req 2 (seven methods) and Req 3 (daily incremental) are binding for the engine. |
| 2 | System Architecture | 26–93 | MIXED | KEEP-R | The Flutter box and FRB are dropped. The Rust core boxes become the engine architecture. The "no business logic in FFI" rule becomes "no business logic in the CLI layer". |
| 3 | Deployment Model | 97–134 | APP-ONLY (mostly) | DROP, salvage 2 rules | Installer, MSIX, MSVC and signing are dropped. The XGBoost pin/vendor rule and the stable-boundary rule carry over. |
| 3.1 | Windows-native requirement | 99–111 | APP-ONLY | DROP | Bans browser UI. With no UI it is moot. |
| 3.2 | Packaging | 113–134 | MIXED | KEEP-R L131, L134. DROP the rest | Pin and vendor the booster's native artifacts. Keep the engine behind a stable API. |
| 4 | Frontend: Flutter Desktop | 138–142 | APP-ONLY | DROP | — |
| 4.1 | State ownership | 144–147 | MIXED | KEEP-R the "Rust owns" list | That list becomes the engine's authoritative-state inventory. |
| 5 | Frontend State Synchronization | 151–153 | MIXED | KEEP-R | The command/query/event pattern becomes the engine API plus CLI plus event sink. |
| 5.1 | Commands | 155–157 | MIXED | KEEP-R 3 of 6 | `trigger_daily_update`, `request_model_rebuild`, `set_lambda` (as config). Drop the `select_*` and `set_date_range` commands. |
| 5.2 | Queries | 159–161 | MIXED | KEEP-R 5 of 6 | Drop `get_dashboard`. |
| 5.3 | Events | 163–167 | ENGINE/PLATFORM | KEEP-R | Becomes the engine event enum: identifiers only, no payload. |
| 5.4 | Training Progress Streams | 169–183 | MIXED | KEEP-R (extend enum) | Progress reaches the CLI and logs instead of the UI. |
| 6 | Visualization | 187–191 | APP-ONLY | DROP | Confidence-band and heatmap *data* remain engine outputs. |
| 6.1 | Performance requirements (viz) | 193–199 | MIXED | KEEP-R 3 of 5 | Keep: compute off the async runtime, paginate or window query results, avoid redundant transfer. Drop: chart rebuilds, downsampling. |
| 7 | Rust Core and Concurrency Model | 203–217 | PLATFORM | KEEP (resource modes KEEP-R) | Binding. |
| 8 | Durable Persistence and Database Strategy | 221–223 | PLATFORM | KEEP | SQLite is the source of truth. |
| 8.1 | Required data domains | 225–247 | DATA | REPLACE the domain names | Basketball `lineups/stints/possessions` become NFL `plays/drives/participation`. |
| 8.2 | SQLx workflow | 249–253 | PLATFORM | KEEP | Already implemented (ADR-002/003, `.sqlx/`, `migrations/0001`). |
| 8.3 | Raw data retention | 255–257 | DATA | KEEP | — |
| 9 | Data Ingestion and Failure Handling | 261 | DATA | KEEP-R | — |
| 9.1 | Daily cadence | 263–273 | DATA/PLATFORM | KEEP-R | Use an external scheduler plus `grid update` instead of an in-process scheduler. |
| 9.2 | Ingestion pipeline | 275–294 | DATA | KEEP | — |
| 9.3 | Idempotency | 296–298 | DATA | KEEP | — |
| 9.4 | Partial data acceptance | 300–302 | DATA | KEEP | — |
| 9.5 | Failure states | 304–325 | DATA/PLATFORM | KEEP (UI line KEEP-R) | Binding enum. The UI becomes `status` output and exit codes. |
| 9.6 | Catch-up behavior | 327–329 | PLATFORM | KEEP-R | Fix the predicate. Generalize "Windows Task Scheduler" to any OS scheduler. |
| 10 | Feature Engineering | 333–339 | ENGINE/DATA | KEEP-R | "Not transient UI state" becomes "not ad-hoc CLI input". |
| 11 | Statistical Engine | 343–345 | ENGINE | KEEP | — |
| 11.1 | Linear algebra | 347–355 | ENGINE | KEEP | — |
| 11.2 | Ridge Regression | 357–371 | ENGINE | KEEP + extend | Add prior-mean, penalty-mask and standardization semantics from the oracle. |
| 11.3 | RAPM | 373–406 | ENGINE | KEEP + REPLACE domain controls and objective | NFL play-level design and the generalized objective. |
| 11.4 | Kalman Filtering | 408–414 | ENGINE | KEEP + clarify time step | Persist model parameters as well as `x_k`/`P_k`. |
| 11.5 | RTS Smoothing | 416–423 | ENGINE | KEEP | Fixed-lag smoothing is new work with an exact parity target. |
| 11.6 | Empirical-Bayes Shrinkage | 425–438 | ENGINE | KEEP + needs estimator spec | Neither final-build nor the oracle defines the estimator fully. |
| 11.7 | Affine Mapping | 440–448 | ENGINE | KEEP | — |
| 11.8 | Gradient Boosting | 450–468 | ENGINE | KEEP-R (trait revision; backend priority is an owner decision) | — |
| 12 | Incremental Learning Pipeline | 472 | ENGINE/PLATFORM | KEEP-R | — |
| 12.1 | Normal daily pipeline | 474–503 | MIXED | REPLACE topology | Use the GRID DAG. "Notify UI" becomes emit event plus exit status. |
| 12.2 | Training modes for gradient boosting | 505–509 | ENGINE | KEEP-R | Define "bounded" and "representative". The V(s) model is periodic-rebuild only. |
| 12.3 | Snapshot and rollback | 511–513 | PLATFORM | KEEP-R | Use pointer semantics, add RAPM sufficient statistics, and flag failures via events and status instead of the UI. |
| 12.4 | Pipeline resumability | 515–517 | PLATFORM | KEEP | — |
| 13 | Model Promotion and Validation | 521–550 | VALIDATION | KEEP + must add promotion criteria | — |
| 14 | Model Versioning and Reproducibility | 554–581 | PLATFORM/VALIDATION | KEEP | — |
| 15 | Durable Job Queue | 585–604 | PLATFORM | KEEP + must add semantics | — |
| 16 | Crash Recovery | 608–626 | PLATFORM | KEEP-R ("Windows restart" becomes "host restart") | — |
| 17 | Observability and Diagnostics | 630–658 | PLATFORM | KEEP (L658 KEEP-R) | — |
| 18 | Security | 662–669 | PLATFORM (last bullet APP-ONLY) | KEEP 5 of 6 | Drop "Bundle MSVC redistributable and code-sign releases". |
| 19 | Testing Strategy | 673–675 | VALIDATION | KEEP | — |
| 19.1 | Unit tests | 677–679 | VALIDATION | KEEP | — |
| 19.2 | Numerical regression tests (Golden-file) | 681–683 | VALIDATION | KEEP + extend to cross-language parity | — |
| 19.3 | Integration tests | 685–687 | VALIDATION | KEEP | — |
| 19.4 | Failure tests | 689–691 | VALIDATION | KEEP-R | "Application restart" becomes "process kill and restart". |
| 19.5 | FFI boundary tests | 693–695 | APP-ONLY | DROP, keep the idea as serialization and interchange round-trip tests | — |
| 20 | Performance Requirements | 699–714 | PLATFORM | KEEP 6 of 8 bullets | Drop FFI transfer and Flutter chart rendering. |
| 21 | Dependency Strategy | 718–738 | PLATFORM | KEEP-R | Drop `flutter_rust_bridge`. `reqwest` and `polars` are listed but absent from the workspace. |
| 22 | Application Lifecycle | 742–778 | MIXED | KEEP-R | Becomes the CLI invocation lifecycle. Drop Flutter, the bridge, "render dashboard" and the background scheduler. |
| 23 | Implementation Order | 782–800 | PROCESS | REPLACE | Phases 1 and 5 are app work. Phases 3 and 4 orderings conflict with GRID. |
| 23 / Ph 1 | Native application shell | 784–785 | APP-ONLY (Rust init and SQLite are already done in P1-00) | DROP | — |
| 23 / Ph 2 | Data foundation | 787–788 | DATA/PROCESS | KEEP-R | — |
| 23 / Ph 3 | Statistical engine | 790–791 | ENGINE/PROCESS | KEEP-R (reorder) | — |
| 23 / Ph 4 | Incremental learning | 793–794 | ENGINE/PROCESS | REPLACE order | — |
| 23 / Ph 5 | Visualization | 796–797 | APP-ONLY | DROP | Diagnostics and band data move into engine queries. |
| 23 / Ph 6 | Production hardening | 799–800 | MIXED | KEEP-R | Drop the FFI round-trip, installer, code-signing and clean-machine items. Clean-machine becomes clean-checkout. |
| 24 | Explicit Non-Goals for V1 | 804–819 | MIXED | KEEP-R | Clarify that the Python *reference* is dev/test only. |

---

## 2. Section-by-section extraction

### §1 Product and Architecture Requirements (L3–22): MIXED

**Binding for the engine (verbatim):**
- Req 2 (L10–17): "The statistical engine must support all of the following as first-class capabilities: RAPM, Ridge regression, Kalman filtering, RTS smoothing, Gradient boosting, Empirical-Bayes shrinkage, Affine transformations/mapping."
- Req 3 (L18): "New external data is fetched **once per day**, after which the application performs incremental data processing and model updates without requiring full retraining of every model."
- L20, optimization priorities: "statistical correctness, reproducibility, deterministic model versioning, crash recovery, low UI latency, efficient CPU utilization, maintainable native deployment, and explainability of model outputs."
  - In the engine spec, "low UI latency" becomes "low query latency".
  - "Maintainable native deployment" becomes "reproducible build of library and CLI".
  - All other priorities are kept.

**Dropped:**
- Req 1 (L9, the native Flutter UI).
- L5 "native Windows desktop analytical application".
- L22 "single installable Windows application".

**Salvage from L22:** "the gradient-boosting dependency may require native shared-library deployment." This is the root of the XGBoost packaging risk and carries into the §11.8 and §21 decision.

**Oracle gap.** Req 3 says "without requiring full retraining of every model". `backend/pipeline/weekly_update.py` refits the V(s) value model on all plays on every run (`fit_value_model(all_plays)`, L213). For the V(s) model, the Rust spec must choose between "periodic rebuild only" and "frozen per model version" (see §12.2).

### §2 System Architecture (L26–93): MIXED

**Keep (engine architecture, from the diagram):**
- **Application Services (L49–52):** "commands, queries, events / training status streams". These become the engine's public API (Rust library) and the CLI subcommands.
- **Async I/O runtime, Tokio (L54–58):** "reqwest (HTTP fetch), daily scheduler, async database operations (SQLx), event notification".
  - Keep reqwest only if the engine fetches over the network (decision D3).
  - "daily scheduler" becomes external (§9.1).
- **CPU worker pool, spawn_blocking / Rayon (L60–65):** "feature generation, RAPM / Ridge solver, Kalman / RTS smoothing, Empirical-Bayes updates, Gradient boosting training".
- **Data / Ingestion (L67–70):** "normalization, validation, reconciliation + partial-data handling".
- **Persistence (L72–74):** "SQLite (authoritative source of truth)", "SQLx (compile-time checked queries, migrated schema)".
- **Statistical engine (L76–83):**
  - "Ridge regression (hand-rolled, nalgebra/sprs)"
  - "RAPM (sparse CG solver, domain-specific prep)"
  - "Kalman filtering (online state update)"
  - "RTS / fixed-lag smoothing (backward revision)"
  - "Empirical-Bayes shrinkage"
  - "Affine transformations"
  - "Gradient boosting (trait-abstracted adapter)"
- **Model governance (L85–89):** "versioning, time-aware validation, promotion / rejection, snapshot / rollback".

**Drop:** the Flutter UI box (L30–41) and `flutter_rust_bridge v2` (L44).

**L93, KEEP-R.** The original reads: "The FFI layer is an adapter between the Flutter UI and Rust application services. Business logic must not be embedded directly in generated FFI bindings." The engine version is: *the CLI crate is a thin adapter over the engine's application-service API; no business logic in argument parsing or output formatting.* Recommend a structural guard: the CLI crate depends only on the application crate.

**Crate implication (recommendation):**
- Keep the math crate(s) (`grid-models`) **synchronous, with no Tokio dependency**, so they can be benchmarked and parity-tested directly.
- Tokio and SQLx live only in persistence, ingestion, application and CLI.
- `crates/models/src/lib.rs` describes the crate as "Ridge, RAPM, Kalman, EB, Affine, Boosting trait" and **omits RTS**. A§8.1 has `models/rts/`. Fix this when the crate is rewritten.

### §3 Deployment Model (L97–134): APP-ONLY except two rules

- **§3.1 (L99–111):** prohibits WebView, Chromium, Electron, browser-based visualization, HTML dashboards, remote browser UI and JavaScript shells. "Flutter Windows is the sole presentation technology." **DROP.** It is trivially satisfied with no UI.
- **§3.2 (L113–134):** **DROP** the following:
  - the release tree (`SportsAnalytics.exe`, `flutter_windows.dll`, `rust_engine.dll`, …);
  - "Packaged via MSIX/installer technology";
  - "MSVC redistributable must be bundled" (L132);
  - "Plan for code signing" (L133).
- **KEEP-R, L131:** "**XGBoost version must be pinned** and native artifacts **vendored in-repo** where possible so CI/release builds do not depend on runtime network access to a third-party wheel host." This applies only if XGBoost stays (decision D2). The generalized rule is: *any native booster dependency is version-pinned, checksum-verified (A§14.2) and buildable offline in CI.*
- **KEEP-R, L134:** "The Rust engine is isolated behind a stable native boundary so that native dependencies can be changed without requiring architectural changes to the Flutter application." The generalized rule is: *the booster backend is swappable behind `IncrementalBooster` without changing the engine API.*

**Platform question.** "Windows-native" also drives the current repo's **Windows-authoritative merge gate**:
- `.github/workflows/alpha-ci.yml` L120–122 (`windows-authoritative`, `windows-latest`);
- `scripts/verify.ps1`;
- ADR-009;
- A§8.11 ("`verify.ps1` is authoritative for merge and release because the production target is Windows").

With no Windows app, whether Windows stays authoritative is an owner decision (D4).

### §4 Frontend: Flutter Desktop (L138–147): APP-ONLY, salvage §4.1

- **L140–142: DROP.**
- **§4.1 (L146), KEEP-R.** The original: "**Rust owns:** data, model parameters, model versions, predictions, confidence intervals, feature definitions, training status, ingestion status, persisted application state." This is the engine's complete authoritative-state inventory. Every item must be persisted (SQLite plus immutable artifacts) and queryable. "Persisted application state" becomes `engine_settings`.
- **L147 "Flutter owns …": DROP.**

### §5 Frontend State Synchronization (L151–183): MIXED

**L153, KEEP-R:** "command / query / event pattern. It does not stream the entire dashboard state on every change." This becomes: the engine API exposes commands (mutations, run as durable jobs), queries (read-only, paginated) and events (notifications carrying identifiers).

**§5.1 Commands (L157):**

| final-build command | Engine-only disposition |
|---|---|
| `select_team(team_id)`, `select_player(player_id)`, `set_date_range(start,end)` | DROP (UI selection state) |
| `set_lambda(lambda)` | KEEP-R. A hyperparameter change. It must create a **new model configuration**, then trigger rebuild, validation and a new model version (§14 L581). It must never mutate production in place. |
| `request_model_rebuild(model_id)` | KEEP. CLI `grid rebuild <model-family>` (A§8.2 uses `model_family`). |
| `trigger_daily_update()` | KEEP. CLI `grid update` (idempotent, catch-up aware). |

Add from alpha-spec where it remains in engine scope: `promote_candidate(model_version)` (only through governance checks) and `rollback_to_snapshot(snapshot_id)` (A§8.2).

**§5.2 Queries (L161):** `get_player_ratings(query)`, `get_predictions(query)`, `get_confidence_bands(query)`, `get_model_status()` and `get_ingestion_status()` are kept. They become CLI `grid query …` / `grid status` with `--json`. `get_dashboard(query)` is DROPPED.

Carry over §6.1's "Paginate or window very large datasets" and A§5.5's "Large payloads use paginated/query-specific DTOs rather than exposing database rows or model internals directly". The query DTOs replace the FFI DTOs, and their stability contract still applies (A§5.5: versioned, explicit units and nullability).

**§5.3 Events (L165–167):**
- Listed events: `DataFetchStarted, DataFetchCompleted, DataChanged, RAPMUpdated, KalmanUpdated, BoostingUpdated, ModelValidationCompleted, ModelPromoted, JobFailed`.
- Binding semantics (L167): "Events notify … that something changed; they do not carry the complete … payload. [The consumer] then requests the specific data it needs."
- Engine version: an `EngineEvent` enum. Each variant carries identifiers (`ingestion_run_id`, `data_version`, `model_version`, `job_id`). Events are emitted to:
  1. an in-process subscriber trait or channel;
  2. structured `tracing` logs (§17);
  3. a durable `diagnostic_events` table (A§8.5 Operations group).
- Merge with A§8.4 (see §4 below).
- The oracle has GRID-specific stages that need events too: `ValueModelFitted`, `Layer1CreditComputed`, `FixedLagSmoothed`, `RolledBack`.

**§5.4 TrainingStatus (L171–183):**

```rust
enum TrainingStatus { Idle, Fetching, Retraining { progress: f32 }, Complete, Failed { reason: String } }
```

- **KEEP-R.** "Prevents the UI from appearing frozen" becomes: the CLI reports progress (stderr or JSON-lines), and `grid status` returns the current `TrainingStatus`.
- **Defects to fix:** the enum cannot express:
  - `Validating`, `Promoting` or `RolledBack` (§13, §12.3);
  - `PartialSuccess` (§9.4);
  - the 14 ingestion failure states (§9.5).

  `Failed { reason: String }` is untyped, which conflicts with A§6.6 rule 4 ("explicit typed failures").
- **Recommendation:** `TrainingStatus { Idle, Fetching, BuildingFeatures, Fitting { stage: StageId, progress: f32 }, Validating, Promoting, Complete { outcome: Promoted|Rejected|NoNewData }, Failed { stage: StageId, error: EngineErrorCode } }`.

### §6 Visualization (L187–199): APP-ONLY, salvage 3 bullets

- **DROP:** `fl_chart`, custom painters, "Avoid rebuilding unrelated charts", "Downsample display-only series".
- **KEEP-R:**
  - "Keep heavy numerical computation outside the Flutter isolate" becomes the §7 rule: never on the async executor.
  - "Paginate or window very large datasets" applies to query results.
  - "Avoid sending redundant historical datasets across FFI" becomes: queries return deltas or windows, not full histories.
- **Engine output obligation:** confidence bands and heatmap matrices are **data** the engine must produce and persist. Rendering them is out of scope. The oracle already produces the honest predictive variance `S = H·Σ_pred·Hᵀ + R` for bands (`statespace.py` `kalman_step(return_pred=True)`; `var_total_pred`). The engine spec should name *predictive* variance, not filtered variance, as the band source.

### §7 Rust Core and Concurrency Model (L203–217): PLATFORM, KEEP

**Verbatim-binding:**
- L205: "Use **Tokio** for asynchronous work and a **dedicated CPU worker pool** (`tokio::task::spawn_blocking` or a dedicated `rayon` pool) for all mathematical work. CPU-heavy mathematical work must never block the async runtime."
- L215: "allow independent workloads to execute concurrently where safe (e.g., HTTP I/O concurrent with DB operations; RAPM and Kalman on separate CPU workers if safe). **Production model publication must be serialized** to prevent simultaneous model updates from corrupting model state."
- L217: "The runtime should support **resource modes** (`normal`, `background`, `manual_rebuild`) so CPU usage is desktop-friendly and does not interfere with user interaction."

**Engine-only notes:**
- **Resource modes** become a CLI flag or config that sizes the rayon pool and sets a priority hint. "User interaction" becomes "co-tenant processes".
- **RAPM and Kalman cannot run concurrently in GRID.** Kalman observes the RAPM / Layer-1 output for the same week (oracle dependency), so the L215 example is wrong for GRID. Within one stage they can run concurrently, for example per-player Kalman steps in parallel.
- **Determinism gap.** final-build says nothing about parallel determinism. A§6.6 rule 5 says: "Parallel execution must not make published predictions nondeterministic beyond documented tolerance." A§9.3 says: "byte-stable tabular predictions within documented floating-point tolerance". Rayon parallel float reductions are order-nondeterministic, so the engine spec must require deterministic reduction order (or per-player parallelism with no cross-player reductions). The oracle pins BLAS threads for its golden (`threadpoolctl.threadpool_limits` in `tests/grid/golden_master.py`) and asserts in-process determinism `< 1e-9` (`tests/grid/test_determinism.py`).
- **Serialization of publication.** In a single-process CLI this means a SQLite-level lock on the production pointer (`BEGIN IMMEDIATE` on promotion) plus a process lock file, so two `grid update` invocations cannot both promote.

### §8 Durable Persistence and Database Strategy (L221–257): PLATFORM/DATA, KEEP

- **L223:** "SQLite is the durable source of truth. In-memory Rust state is a cache that can be reconstructed from SQLite."
  - **Clarify.** §11.8's `save(path)`/`load_or_init(path)` and §14 L573 ("on-disk representation may use directories and metadata files") mean model artifacts live on the filesystem.
  - Engine-spec wording: *SQLite is authoritative for metadata, versions, pointers and job state. Artifact files are immutable, content-addressed (hash recorded in SQLite), and never mutated after write.*
  - The oracle violates this: its Kalman and RAPM state is mutable `data/cache/*.npz` files overwritten in place (`KalmanState.save`, `save_accumulators`).

**§8.1 Required data domains (L229–247):** `games, teams, players, lineups, stints, possessions, raw_data, feature_sets, predictions, player_ratings, model_versions, model_states, training_runs, ingestion_runs, jobs, application_settings, snapshots`.

REPLACE for the NFL/GRID domain:

| final-build domain | Disposition |
|---|---|
| `lineups`, `stints`, `possessions` | Replace with `drives`, `plays`, and `play_participation` (offense/defense player sets per play). In GRID every play is a RAPM observation; the "stint" is the play. |
| `application_settings` | Rename to `engine_settings`. |
| `player_ratings` | Keep. It holds RAPM ratings, situational RAPM, Layer-1 weekly credit and Kalman trajectories (the oracle writes `matchup_grades` and Kalman trajectories in `weekly_update.py`). |
| (missing) | Add `market_strength` (Layer 3 input, `{team → closing-line-implied strength}`), `feeder_player_seasons` (the `college` contract: `player_id, position, feeder_sv, feeder_snaps, is_rookie`), and RAPM sufficient statistics (`XᵀWX`, `XᵀWy`, column order) under `model_states`. |
| (missing) | Add `diagnostic_events` (A§8.5) and `quarantined_rows` (§9.4 "log/quarantine"). |

**Contract anchor.** The oracle's fixed engine contract is documented in `backend/grid/data_adapters.py` L1–20:
- `plays`: `[play_id, drive_id, week, off_team, def_team, down, ydstogo, yardline_100, yards, points, terminal, terminal_value, n_down, n_ydstogo, n_yardline_100, drive_points, off_players (tuple), def_players (tuple)]`
- `players`: `[player_id, team, position, is_starter, (ability if synthetic)]`
- `market`: `{team -> strength}`
- `college`: `[player_id, position, feeder_sv, feeder_snaps, is_rookie]`

The §8.1 replacement should be derived from this contract.

**§8.2 SQLx workflow (L251–253), KEEP verbatim:**
- "Use `sqlx` with async SQLite and **compile-time checked queries**."
- "Commit the `.sqlx` query cache to version control (`cargo sqlx prepare`)…"
- "Use `sqlx migrate` from day one."

These are already implemented: `.sqlx/` holds 2 query files, `migrations/0001_schema_meta.sql`, ADR-002 and ADR-003, and the `check-sqlx` recipe.

**§8.3 Raw data retention (L257), KEEP:** "Raw external responses are retained to enable reproducibility, debugging, provider schema changes, historical reprocessing, and auditability. The raw payload does not need to be queried during normal application operation." Merge in the A§4.1.1 per-source metadata: `source_name, source_version, source_timestamp, retrieved_at, ingestion_run_id, content hash, row count, schema version, validation status`. The oracle caches raw nflverse parquet (`nflverse_loader.py`) but records no hashes or metadata.

### §9 Data Ingestion and Failure Handling (L261–329): DATA, KEEP-R

**§9.1 Daily cadence (L265–273):** "External data is fetched **once per day** at a configurable local time. The scheduler supports: Configurable local time; Manual 'Run Update Now'; Retry after failure with exponential backoff; API rate limiting and timeout handling; Persisted last-successful-run status. The application does not require continuous polling."

Engine-only rewrite:
- `grid update` is a one-shot, idempotent command.
- Scheduling belongs to the host scheduler (cron, systemd timer, Task Scheduler).
- Backoff and retry happen inside the command, bounded.
- `last_successful_run` is persisted in `ingestion_runs`.

This avoids conflict with §24 "Always-running server process" (see D3).

Cadence semantics must reconcile with A§1.1 ("**at most** once per local calendar day") and A§8.6 ("day-of-week-aware local schedule, still capped at one external fetch per calendar day"). Recommended definition: *at most one successful fetch per source per local calendar day; retries of a failed fetch and idempotent re-runs that perform no network fetch do not count.*

**GRID-specific point.** NFL data changes weekly, not daily. A daily run will mostly end in `NO_NEW_DATA` (or carry stat corrections). Model state transitions must be keyed to **game-week completion**, not to the calendar day (see §11.4).

**§9.2 Ingestion pipeline (L277–294), KEEP:**

```text
External API → raw response (retained) → normalization → validation
  → deduplication / reconciliation → partial-data handling → SQLite transaction
  → commit → create durable learning job
```

L288 is garbled ("partial-data handling    ↓" on one line) and should be fixed in the rewrite. The oracle's equivalent chain is `nflverse_loader → nflverse_adapter.build_plays_contract → data_pipeline`. It has no validation or quarantine stage and no job creation.

**§9.3 Idempotency (L298), KEEP verbatim:** "Every ingestion operation receives a unique `ingestion_run_id`. The pipeline is idempotent: a daily update should be safe to execute twice."
- The oracle's weekly idempotency is keyed on `last_week` in the accumulator file: "re-running the same week re-computes but does not double-count (accumulator state is keyed by last_week and skips if already done)" (`weekly_update.py` L12–13, L241).
- The Rust version must key idempotency on `(source, content hash)` for data and `(input_data_version, stage)` for model jobs.
- A last-week watermark alone cannot absorb stat corrections to an already-ingested week. Under A§12.3, "stat corrections must create a new data version".

**§9.4 Partial data acceptance (L302), KEEP verbatim:** "reject malformed rows, log/quarantine them, and continue ingestion with the valid subset rather than aborting the entire day's batch. Status should reflect `PARTIAL_SUCCESS` when applicable." This is consistent with A Appendix D #8 because quarantine is not defaulting. The oracle partly does it: `build_design` skips unknown players with a log line (`tests/grid/test_rapm_robustness.py::test_build_design_unknown_player_skipped`), but persists no quarantine record.

**§9.5 Failure states (L308–325), KEEP as a binding enum:**

```text
NOT_RUN RUNNING SUCCESS NO_NEW_DATA PARTIAL_SUCCESS NETWORK_FAILURE TIMEOUT RATE_LIMITED
AUTHENTICATION_FAILURE INVALID_RESPONSE SCHEMA_FAILURE DATA_VALIDATION_FAILURE
DATABASE_FAILURE LEARNING_FAILURE
```

- L325 is binding: "A failure in the daily update must not corrupt or partially promote the previous production model."
- "The UI exposes the last successful update and the current error state" becomes `grid status` plus a **distinct, documented process exit code per terminal state** (recommendation; enables scheduler alerting).
- The oracle has no such states. It returns dicts like `{"rapm_solved": False, "kalman_updated": False}` and logs warnings when data load fails (`weekly_update.py` L216: "Could not load nflverse data … skipping RAPM update"). Do not port that swallow-and-continue behaviour.

**§9.6 Catch-up behavior (L329):** "When the application starts, it detects if `last_successful_update > configured_interval` and initiates a catch-up update. Optionally, register a Windows Task Scheduler job (non-interactive update mode)…"
- **Defect:** the predicate compares a timestamp with a duration. It should read `now − last_successful_update > configured_interval`.
- **Engine version:** every `grid update` invocation evaluates catch-up. Catch-up must process **each missed game-week in order** (the Kalman filter is sequential), not jump straight to "today".

### §10 Feature Engineering (L333–339): ENGINE/DATA, KEEP-R

**Verbatim-binding (L335):**
- "Feature generation must be deterministic and versioned."
- "Each feature definition receives a schema/version identifier (e.g., `feature_schema_version = 17`)."
- "A model record must reference the feature schema from which it was trained."
- "Changing a feature definition creates a new feature schema rather than silently modifying historical semantics."

L337: "Feature generation operates from durable source data, not transient UI state." For the engine: *not from ad-hoc CLI arguments*; CLI parameters that alter features must themselves be versioned config.

L339: "**Polars** may be used… Core services continue to exchange typed domain structures and numerical arrays." `polars` is **not** in `[workspace.dependencies]` today.

**GRID mapping.** The GRID "features" are:
- the situational state columns `STATE_COLS = ["down","ydstogo","yardline_100"]` (`value.py`);
- the RAPM design matrix construction (`layers.build_design`, including optional WR×CB interaction columns pruned at `min_pair_plays=30`);
- situation masks (`situations.py`: red_zone, passing_downs, two_minute, …);
- projection features (`projection/features.py`: `rapm_rating, smoothed_talent, prior_mean`).

All need feature schema versions. The A§4.5 as-of and leakage rules attach here; the oracle has `validation/asof.py` and `tests/validation/test_leakage_guards.py`. final-build-spec has **no** leakage requirement at all, which is an alpha-spec addition.

### §11 Statistical Engine (L343–468): ENGINE, KEEP

L345: "The engine is composed of independent modules with stable interfaces."

#### §11.1 Linear algebra (L349–355), KEEP verbatim

- "Use `nalgebra` for dense matrix/vector computation."
- "Small/fixed state-space matrices: statically sized types."
- "Large/dynamic matrices: dynamically sized types (`DMatrix`)."
- "Sparse lineup/stint structures: `sprs`."
- "Include dimension assertions and runtime validation for dynamic matrices."

For GRID, the Kalman state is fixed at `N_STATE = 3` (`[talent, form, scheme_fit]`), so use `SMatrix<f64,3,3>`. The oracle also keeps a legacy 2-component migration path (`KalmanState.migrate`); decide whether to port it (recommend no: there is no legacy Rust state).

#### §11.2 Ridge Regression (L359–371), KEEP + extend

**Verbatim-binding:**
- "independent, reusable statistical primitive, implemented directly with `nalgebra`/`sprs` rather than through an external ML framework like `linfa`."
- Supports: "Arbitrary feature matrices; Configurable λ; Regularized intercept handling; Weighted observations; Prediction; Coefficient output; Solver diagnostics".
- "For the dense case: solve via normal equations or Cholesky with regularization. For the sparse case: use conjugate gradient descent." ("Conjugate gradient descent" is a misnomer; it should read conjugate gradient.)

**The oracle requires these extra semantics, which the Rust primitive must support for parity:**
1. **Prior-mean ridge.** `min ‖y−Xb‖² + λ‖b−m‖²`, with `L = λ·diag(mask)`, `A = XᵀX + L`, `b = Xᵀy + L·m` (`layers.py` L347–359 `_solve_ridge_prior`).
2. **Per-column penalty mask** (generalizes "regularized intercept handling").
3. **Pseudo-observation rows** added to the normal equations with a weight (`w_market`).
4. **Standardized-feature ridge with an unpenalized intercept**, matching sklearn `Ridge(alpha)` + `StandardScaler` per `(volume_driver, stat)` (`projection/model.py` L39, L143–145).
5. **Accumulated normal equations** (`init_accumulators`, `accumulate`, `fit_from_accumulators`, `layers.py` L62–87) for incremental use.

**Failure semantics conflict.** The oracle falls back to `np.linalg.lstsq` when `cond(A) > 1e10` and only logs a warning (`layers.py` L353–357; tested in `tests/grid/test_rapm_robustness.py::test_solve_ridge_prior_lstsq_fallback`). That collides with final-build §11.3 L404 ("A numerically failed solve must not silently produce a production model") and A§6.6 rule 4 (singular systems are explicit typed failures). The Rust primitive should return diagnostics that include the condition estimate, with an ill-conditioning flag that **blocks promotion**. See D5.

#### §11.3 RAPM (L375–406), KEEP; REPLACE the domain controls and the objective

**Verbatim-binding:**
- "RAPM is a first-class production model with explicit domain controls."
- "Document and test the exact optimization objective."
- "The player/stint design matrix is constructed as a sparse matrix using `sprs`. Solve via iterative sparse conjugate gradient with explicit diagnostics: `converged, iterations, residual_norm, tolerance, regularization_lambda`."
- "**A numerically failed solve must not silently produce a production model.**"
- "Support preconditioning where beneficial and benchmark against expected matrix sizes."

**The objective must be generalized.** L390–392 states `β̂ = argmin ‖y−Xβ‖² + λ‖β‖²`. The engine spec must state the objective GRID actually solves (statistical-owner sign-off):

```text
β̂ = argmin_β  ‖W^{1/2}(y − Xβ)‖²  +  w_mkt · Σ_t (γ_off[t] + γ_def[t] − s_t)²  +  (β − m)ᵀ Λ (β − m)
Λ = λ · diag(mask);  mask = 1·pos_mult(p) for players, 0.05 for team intercepts, 10.0 for interaction columns
```

Oracle parameters: `lam=120.0`, `w_market=40.0`, `lambda_by_pos` optional, team intercept mask `0.05`, interactions `10.0` (`layers.py` L362–410). Note that the oracle's market row sets **both** `t_off` and `t_def` to `+1.0` while reporting `team_rating = β_off − β_def` (`layers.py` L404–405 vs L426). This sign convention must be documented, and preferably verified, before it is frozen into the Rust spec.

**Domain controls (L377–386) mapped to NFL/GRID:**

| final-build control | GRID / oracle definition |
|---|---|
| Stint boundaries | One observation per play (`build_design`). |
| Players on court | `off_players` +1, `def_players` −1, from nflverse/FTN participation. |
| Response variable | `dv = V(s') − V(s)` from the value model (`value.py`). |
| Possession weighting | Play weights. Uniform in the oracle, but the Rust primitive must support weights. |
| Home-court treatment | Home-field. **Not modelled in the oracle**; decide. |
| Team effects (if used) | Team offense and defense intercepts, which absorb OL and scheme (`layers.py` docstring L6–9). This is binding GRID intent: "keep OL as a nuisance rather than an estimand". |
| Intercept treatment | Lightly penalized (mask 0.05) plus Layer-3 market reconciliation. |
| Garbage-time handling | Not in base RAPM. `situations.py` masks exist; decide. |
| Overtime treatment | Not modelled; decide. |
| Minimum appearance thresholds | `run_situation_rapm(min_plays=200)`; interactions `min_pair_plays=30`. |

**Sparse CG vs the oracle's dense solve.** The oracle solves densely (`np.linalg.solve` on a dense `XᵀX` that is `n_cols × n_cols`). final-build mandates sparse CG, so parity is "CG solution within tolerance of the dense solution" (see §5 tolerance classes). For incremental RAPM, recommend accumulating sparse `XᵀWX` and `XᵀWy` (sprs) per week and warm-starting CG from the previous β.

The oracle detects reordering of the column player list (`player_order`, `layers.py` L94–142) and reinitializes. The Rust version must version the column index map as part of model state.

**GRID fixed point.** `fit(…, n_iter=3)` (`layers.py` L591–607) loops: RAPM → defender ratings → Layer-1 opponent-adjusted QB credit → re-seed the QB prior mean. final-build has no concept of this outer loop, so add it to the RAPM model spec.

**Live-path conflict.** See D1 and the alpha matrix row 2.

#### §11.4 Kalman Filtering (L410–414), KEEP + clarify

**Verbatim-binding:**
- "The Kalman Filter is the primary mechanism for genuine online/incremental state updating."
- "State (`x_k`, `P_k`) and the configured transition/observation model are persisted to SQLite between runs."
- "**Daily update mode:** Load persisted state, apply the day's worth of observations as a batch of sequential predict/update steps in a single CPU-bound task, then persist the new `x_k` / `P_k`."
- "For fixed state dimensions, the per-observation workload does not grow with historical observations. Complexity should be qualified with respect to state and observation dimensions."

**GRID / oracle definition (port target, `statespace.py`):**
- State `x = [talent τ, form f, scheme_fit s]`.
- Transition `F = diag(1, φ, φ_scheme)`; observation `H = [1, 1, 1]`.
- Predict step:
  - `P⁻ = F P Fᵀ`, then `P⁻[0,0] /= d` (West–Harrison discount on talent);
  - `P⁻[1,1] += q_form`, `P⁻[2,2] += q_scheme`;
  - at an intervention: `d → d_spike`, `P⁻[1,1] += 5·q_form`, `P⁻[2,2] += scheme_reset_var`.
- `R = r_mult · r_scale / max(snaps, 1)`, with `r_mult = post_event_r_mult` for `post_event_games` games after an event.
- Joseph-form update (`tests/grid/test_kalman_numerical.py`).
- Missing week: predict only, no update.
- Scheme reset (coaching change) is distinct from an intervention: it zeroes `mu[2]`, sets `σ[2,2] = scheme_reset_var` and zeroes the cross-covariances.
- Defaults (`SSParams`): `phi=0.50, phi_scheme=0.985, d_steady=0.90, d_spike=0.70, q_form=0.0008, q_scheme=0.0002, scheme_reset_var=0.04, r_scale=0.40, post_event_r_mult=2.0, post_event_games=2`.
- Position overrides (`_POSITION_PARAMS`): QB `d=0.95, r=0.55`; RB `0.85, 0.45`; WR `0.90, 0.40`; TE `0.90, 0.40`; DEF `0.95, 0.55`.
- Initial `P0 = diag(0.05, 0.02, 0.01)`.
- Required outputs: filtered, smoothed and **one-step predictive** `(mean, S)`.

**Gaps against final-build:**
1. **Time step.** One predict/update per player **game-week**, never per calendar day (see headline 4). "Day's worth of observations" must be redefined as "observations from newly completed game-weeks, in week order".
2. **Persisted model parameters.** final-build requires persisting "the configured transition/observation model". The oracle persists only `mu, sigma, player_ids, state_version` (`KalmanState.save`), not `SSParams`. The Rust version must persist `SSParams` (per position) as model-version hyperparameters (§14).
3. **Corrupt-state handling.** The oracle returns `None` (which triggers a silent reinit) on an unrecognized state width (`KalmanState.load`; `tests/grid/test_incremental.py::test_load_corrupt_returns_none`). This conflicts with §16 and A Appendix D #8. The Rust version should raise a typed `StateCorrupt` error and leave recovery to the operator or to rollback.

#### §11.5 RTS Smoothing (L418–423), KEEP

**Verbatim-binding:**
- "RTS smoothing is a distinct backward-looking operation from online Kalman filtering."
- Mode 1, "**Full RTS smoothing:** Used for periodic historical recalculation, backtesting, diagnostics, and explicit user-requested rebuilds."
- Mode 2, "**Fixed-lag smoothing:** Used for incremental production updates. After the daily Kalman filter batch, apply fixed-lag smoothing over a recent window to revise recent historical states without recomputing the entire sequence."

**Oracle:**
- Full RTS exists (`kalman_two_component`, `statespace.py` L237–246), with gain `C = P_f[w] Fᵀ (P⁻[w+1] + 1e-10·I)⁻¹`. That jitter is a numerical choice that must be specified.
- **There is no fixed-lag implementation** and `kalman_step` performs no smoothing.

**New-work parity target (exact):** RTS backward recursions use only stored filtered and predicted moments. A fixed-lag smoother with lag `L` at time `t` therefore equals full RTS over observations `1..t` evaluated at indices `t−L..t`. The test is that fixed-lag output equals full-RTS output on the same window to about `1e-12`.

This requires persisting the last `L` filtered `(x, P)` and predicted `(x⁻, P⁻)` per player in model state.

**Parameter undefined.** The window `L` is not specified anywhere (final-build, A§6.2/A§8.6, or the oracle). The statistical owner must set it; suggest 4 game-weeks as a starting default.

"Explicit user-requested rebuilds" becomes the CLI `grid rebuild kalman --full-rts`.

#### §11.6 Empirical-Bayes Shrinkage (L427–438), KEEP; needs an estimator spec

**Verbatim-binding:**
- "Maintain explicit prior parameters. Persist: prior mean, prior variance, estimated population variance, sample counts, shrinkage parameters, version."
- "The daily pipeline recalculates or incrementally updates the prior using newly available observations."
- "**The precise prior distribution and posterior estimator must be documented as part of the model specification.**"

**Oracle EB pieces (none of which fully satisfy §11.6):**
- **Volume shrinkage:** weight `w = g/(g + k_shrink)` with `k_shrink = 8.0` as a fixed default (`projection/volume.py` L55, L105). The docstring claims the weight is "learned from the data", but `k` is a parameter default. Verify before porting.
- **Rookie priors:** `PRIOR_SD = {QB:0.16, RB:0.07, WR:0.08, TE:0.06, DEF:0.07}`, hard-coded (`priors.py` L24). The population variance is **not estimated**.
- **Ridge prior-mean seeding** in RAPM.

A§6.3 supplies the formula final-build demands: `θ_posterior = (n0·θ_prior + n_eff·θ_obs)/(n0 + n_eff)`, with `n0` "learned by position and latent component through rolling-origin validation"; and `θ_prior = q·θ_ncaa_translated + (1−q)·θ_position_draft_prior`.

The Rust EB model spec should state the estimator explicitly, for example normal–normal EB with method-of-moments `τ̂² = max(0, Var(θ̂) − mean(σᵢ²))` and `Bᵢ = σᵢ²/(σᵢ² + τ̂²)`. It should also persist `n0`/`k`, `τ̂²` and sample counts per position.

#### §11.7 Affine Mapping (L442–448), KEEP verbatim

- "Affine transformations are a first-class numerical utility supporting `y = Ax + b`."
- "Expose: transformation matrix, offset vector, inverse where applicable, dimensionality validation, deterministic serialization."

GRID uses:
1. **Feeder→NFL equivalency** (`priors.estimate_equivalency`): a fitted slope and intercept from `LinearRegression` on shared players, scaled by `LEAGUE_FACTORS = {FBS 0.35, FCS 0.20, UFL 0.15, USFL 0.15, XFL 0.15, CFL 0.18, unknown 0.25}`. The oracle uses a 70/30 split with `default_rng(3)` for `oos_r2`.
2. **Fantasy scoring** as an affine map from stat vector to points (`scoring/engine.calculate_points`; A§5.4 `fantasy_points = scoring_weights · projected_stat_vector + scoring_offset`).
3. **SV-to-points scaling** (`projection/sv_to_points.py`).

"Inverse where applicable" is ill-defined for non-square `A`, such as scoring. Specify: exact inverse only when `A` is square and well-conditioned; otherwise "not invertible" is a typed result.

#### §11.8 Gradient Boosting (L452–468), KEEP-R

**Verbatim-binding:**
- "Gradient boosting is required behind an application-defined adapter."
- "**Primary implementation:** `xgb`-backed struct."
- "**Fallback implementation:** Hand-rolled gradient boosting using `linfa-trees` decision trees as weak learners, with a residual-fitting loop and learning-rate shrinkage."
- "The rest of the application must not depend directly on XGBoost-specific types."

The trait as written:

```rust
pub trait IncrementalBooster {
    fn load_or_init(path: &Path) -> Result<Self> where Self: Sized;
    fn continue_training(&mut self, new_data: &FeatureMatrix) -> Result<()>;
    fn predict(&self, features: &FeatureMatrix) -> Result<Vec<f64>>;
    fn save(&self, path: &Path) -> Result<()>;
    fn metadata(&self) -> ModelMetadata;
}
```

**Problems for the engine spec (needs an ADR):**
1. `load_or_init` takes no hyperparameters, seed, or objective.
2. `continue_training` has no labels, weights or bound (§12.2 "bounded"), and returns no diagnostics.
3. There is no full-fit entry point for "periodic rebuild" (§12.2-3).
4. `save(path)` to a mutable path conflicts with immutable, content-addressed artifacts (§14, §16.5).
5. The `linfa-trees` fallback conflicts with §11.2's anti-`linfa` stance and is absent from the §21 core list, so it needs dependency approval (A§14.2).

**Oracle usage:** sklearn `HistGradientBoostingRegressor`, with no continuation anywhere.
- **V(s) value model** (`value.py` L51–53): `max_depth=4, learning_rate=0.08, max_iter=300, min_samples_leaf=120, random_state=0`; features `[down, ydstogo, yardline_100]`; target `drive_points`.
- **Layer-1 context model** (`layers.py` L497–519): `max_depth=3, learning_rate=0.1, max_iter=200, min_samples_leaf=150, random_state=seed`; 5-fold `KFold(shuffle=True, random_state=0)` cross-fit; features are state plus summed on-field opponent-defense rating.

Neither XGBoost nor a linfa-trees GBM will reproduce sklearn's histogram binning. Boosting parity is therefore **statistical**: V(s) surface agreement and the downstream recovery gates (see §5). Phase 3 of final-build (L791) itself lists "IncrementalBooster trait + fallback", so the pure-Rust fallback is built first in any case. See D2.

### §12 Incremental Learning Pipeline (L472–517)

#### §12.1 Normal daily pipeline (L476–503): REPLACE topology

As written:

```text
Daily scheduler → Fetch → Validate/normalize/reconcile → SQLite txn → Build features
  → Snapshot current model state (rollback point)
  → [Kalman update | RAPM incremental | Empirical-Bayes update]
  → Gradient Boosting (continuation / replay) → Validation → Promote candidate OR reject → Notify UI
```

The diagram is garbled at L488–496.

**Problems:**
- Kalman, RAPM and EB are drawn as parallel; in GRID they are sequential.
- Boosting is placed last; in GRID it comes first.
- There is no fixed-lag step, although §11.5 requires it.
- "Notify UI".

**Recommended GRID DAG (from the oracle `weekly_update.py` and `layers.fit`):**

```text
host scheduler → grid update
  → fetch / raw retain / normalize / validate / quarantine → commit data_version
  → [new completed game-week(s)?  no → NO_NEW_DATA, exit]
  → build features (plays contract, state cols, design rows) for new weeks
  → snapshot = record current production pointer (immutable)
  → V(s) value model: reuse model-version's frozen V (rebuild only on schedule) → dV
  → RAPM: accumulate XᵀWX/XᵀWy for new week(s) → sparse-CG solve (+Layer-3 market rows)
  → fixed point (n_iter): defender ratings → Layer-1 cross-fitted credit → re-seed priors
  → Kalman step per player per new week (obs = weekly RAPM/Layer-1 credit, R from snaps)
  → fixed-lag RTS over last L weeks
  → EB / volume shrinkage update; priors for new entrants (affine feeder map)
  → downstream rate ridge + scoring affine → candidate predictions
  → validation (sanity invariants + time-aware holdout + recovery gates) → promote OR reject
  → emit events / write status / exit code
```

#### §12.2 Training modes for gradient boosting (L507–509), KEEP-R

Verbatim:
1. "**Daily continuation:** … bounded incremental update…"
2. "**Replay-window training:** … recent observations + representative historical observations (optionally weighted). This prevents overreacting to a small batch of new games."
3. "**Periodic rebuild:** At a configured interval or manual trigger, perform a full gradient boosting rebuild from the historical feature store, followed by validation and promotion."

Undefined terms the model spec must fill:
- "bounded" (for example, max trees added per update);
- "representative" (the sampling scheme);
- "optionally weighted" (the weighting scheme);
- the rebuild interval.

For GRID, the V(s) value model describes a stable state surface. **Recommend periodic rebuild only, frozen per model version**, so `dV` (and therefore the RAPM accumulators) is not silently re-based mid-season. If the oracle refits V on every run, accumulated `XᵀWy` from earlier weeks was computed with a different V. That is a latent inconsistency in `weekly_update.py`, and the Rust spec should not inherit it. There is also an as-of problem in the same code. `weekly_update.py` L210–213 fits V on `load_grid_plays([season])`, which is every available week of the season. If the job is re-run for an earlier week (backfill or catch-up), V is fitted on weeks **after** the target week. That is a leakage path under A§4.5 and A§12.3. The Rust value-model fit must be bounded by the as-of data version.

#### §12.3 Snapshot and rollback (L513), KEEP-R

Verbatim: "Before applying any daily update, the engine copies current model state (Kalman covariance/state, boosting model file, EB priors) to a versioned snapshot. If an update produces degenerate output (NaN, wildly divergent predictions exceeding sanity thresholds), the engine rolls back to the previous snapshot automatically, flags the failure to the UI, and leaves the previous production model untouched."

Changes:
- "Flags the failure to the UI" becomes a `RolledBack` event plus a `LEARNING_FAILURE` status and exit code.
- The snapshot list **omits** RAPM sufficient statistics and the column map, Layer-3 and team intercepts, the V(s) model, Layer-1 and `SSParams`. Add them all.
- The sanity thresholds are unspecified. Specify them per model in the model spec. A§10.2 adds "NaN, impossible stat totals, share overflow, and extreme week-over-week changes".
- **Copy versus immutable conflict.** Because every update writes a new model version and never mutates the old one (§14 L581, §16 rule 5), rollback is "do not advance, or revert, the production pointer". The "snapshot" is the prior immutable version record. Retain a `snapshots` table as the audit record of the pointer value before each update.

The oracle overwrites `data/cache/*.npz` in place and has no rollback. It also saves the accumulators **before** the solve and the Kalman step (`weekly_update.py` L259 `save_accumulators` before L278 `fit_from_accumulators` and L389 `kalman_step`), so a crash between the two leaves inconsistent state. This is exactly the failure §12.4 and §16 forbid; do not port it.

#### §12.4 Pipeline resumability (L517), KEEP verbatim

"The entire pipeline must be resumable. Each stage produces durable status so a crash does not require blindly repeating the entire process." Implement this as one durable job per stage (§15) with idempotent stage outputs keyed by `(input_data_version, stage, config_hash)`.

### §13 Model Promotion and Validation (L521–550): VALIDATION, KEEP + add criteria

**Verbatim-binding:**
- "No model is automatically production-quality merely because training completed."
- Flow: "training → candidate model → time-aware validation (rolling-origin, walk-forward, time-based holdout) → compare with current production model → promote OR reject".
- States: `TRAINING, VALIDATING, CANDIDATE, PRODUCTION, REJECTED, SUPERSEDED`.
- "A failed model update must leave the previous production model untouched."
- "Validation metrics must be persisted with each model. Possible metrics include MAE, RMSE, log loss, Brier score, calibration error, and rank correlation."

**Gaps:**
- The state order conflicts with the flow (the flow says training → candidate → validation; the list puts VALIDATING before CANDIDATE). Define the transitions explicitly: `TRAINING → CANDIDATE → VALIDATING → {PRODUCTION | REJECTED}` and `PRODUCTION → SUPERSEDED`.
- **No promotion criterion is defined.** "Compare with current production" has no metric, threshold or tie rule. The oracle has a full validation stack to draw from:
  - `backend/validation/{backtest,metrics,tier1,tier2,verdict,thresholds}.py`;
  - `provisional_thresholds.json`;
  - synthetic Tier-0 recovery gates (`tests/grid/test_tier0_recovery.py`);
  - Kalman calibration via NIS and PICP (`tests/grid/test_calibration_synth.py`; the QB comment cites "pooled NIS≈1.24 … PICP@80≈0.78").

Recommended engine promotion gate, for statistical-owner sign-off (D6):
1. numerical diagnostics are clean (CG converged, no NaN, no ill-conditioning flag);
2. synthetic recovery gates pass on the candidate code and config;
3. rolling-origin metrics are non-inferior to production within a declared margin;
4. calibration (NIS, interval coverage) is within band.

A§7.4–7.10's PB-MAE and market gates are product gates. Whether they remain engine scope is for the alpha-spec inventory to settle.

### §14 Model Versioning and Reproducibility (L554–581): KEEP verbatim

Every production model must reference: `model_id, model_version, model_type, feature_schema_version, data_snapshot_version, training_run_id, training_start, training_end, hyperparameters, random_seed (when applicable), validation_metrics, build/application version`.

Also binding:
- "A model must be reproducible from its recorded data and feature versions."
- "The on-disk representation may use directories and metadata files; model identity and metadata must be durable."
- `Data Version → Feature Version → Model Version → Prediction Version`.
- "An incremental update creates a new model version; it does not overwrite the historical definition of the previous model."

**Engine additions:**
- "build/application version" becomes the engine crate version plus git SHA.
- Hyperparameters must include `SSParams`, `λ`, the penalty mask, `w_market`, `n_iter`, booster params and the fixed-lag `L`.
- Record the column index map (player order) for RAPM.
- For parity runs, also record the reference/python commit and fixture hash.
- "random_seed, when applicable" should be strengthened per A§6.6 rule 5 to "all randomness seeded and recorded". The oracle seeds are synth `seed=7`/`seed+99`, value model `random_state=0`, KFold `seed=0`, and equivalency split `default_rng(3)`.

### §15 Durable Job Queue (L585–604): PLATFORM, KEEP + add semantics

- **Job types:** `DATA_FETCH, NORMALIZE_DATA, BUILD_FEATURES, RAPM_UPDATE, KALMAN_UPDATE, RTS_SMOOTH, EMPIRICAL_BAYES_UPDATE, BOOSTING_UPDATE, MODEL_VALIDATE, MODEL_PROMOTE`.
- **Fields:** `job_id, job_type, created_at, status, input_data_version, output_model_version, attempt_count, started_at, completed_at, error`.
- "This provides crash recovery and diagnostic history."

**Missing (the engine spec must define):**
- Job **status values**. Suggested: `PENDING, RUNNING, SUCCEEDED, FAILED, CANCELLED, SKIPPED_NO_NEW_DATA`.
- Max attempts, backoff, and dependency edges between jobs (the DAG above).
- Idempotency key `(job_type, input_data_version, config_hash)`.
- **Stale-RUNNING detection.** In a one-shot CLI, a `RUNNING` row with no live owner at startup means a crash; record `owner_pid` and a heartbeat or lease.
- Cancellation on graceful shutdown (§22).
- Missing job types for GRID: `VALUE_MODEL_FIT`, `COMPUTE_DV`, `LAYER1_CREDIT`, `FIXED_POINT_ITERATE` (or fold into `RAPM_UPDATE`), `FIXED_LAG_SMOOTH` (distinct from full `RTS_SMOOTH`), `PRIOR_TRANSLATE` (affine feeder map), `PREDICT`, `SNAPSHOT`, `ROLLBACK`.

### §16 Crash Recovery (L608–626): PLATFORM, KEEP-R

Recover from: application crash, Windows restart ("host restart" in the engine spec), network interruption, database interruption, failed model training, failed model serialization.

**Recovery rules (verbatim):**
1. "Never partially promote a model."
2. "Never overwrite the current production model before candidate validation succeeds."
3. "Resume incomplete jobs where possible."
4. "Rebuild in-memory state from SQLite when required."
5. "Treat model files and metadata as versioned immutable artifacts."
6. "Maintain daily snapshots before updates; auto-rollback on degenerate output."

Add: artifact writes use write-to-temp, fsync, atomic rename, record hash in SQLite, then commit. A dangling artifact without a SQLite row is garbage-collectable. A SQLite row whose artifact hash mismatches is `StateCorrupt`.

### §17 Observability and Diagnostics (L630–658): PLATFORM, KEEP

**Minimum log events:** "application startup/shutdown, daily fetch result, records added/modified, feature-build duration, RAPM duration, Kalman duration, RTS duration, boosting duration, model validation metrics, model promotion, job failures".

**Identifiers are required:** "Diagnostic logs must include identifiers, not only prose", e.g. `ingestion_run_id=1842 data_version=991 model_version=43 job_id=6201 duration_ms=18422`.

L658 is KEEP-R: "The UI displays the last successful update, current error state, and training progress via the `TrainingStatus` enum" becomes `grid status` output.

Implementation: `tracing` and `tracing-subscriber` are already in the workspace dependencies. Use a JSON formatter with spans per job carrying the identifiers. Add the GRID stages: V(s) fit duration, Layer-1 duration, fixed-point iterations, CG iterations and residual, and fixed-lag duration. The oracle uses Python `logging` (`backend/pipeline/_logging.py`) with prose messages, so there is nothing to port.

### §18 Security (L662–669): PLATFORM, KEEP 5 of 6

Keep, verbatim:
- "Require HTTPS/TLS for external APIs."
- "Validate external payloads; enforce response-size limits and timeouts."
- "Protect API credentials; avoid storing credentials in source-controlled configuration."
- "Validate filesystem paths and numeric input ranges."
- "Prevent external data from generating dynamic SQL."

Drop: "Bundle MSVC redistributable and code-sign releases."

Engine notes:
- "Validate filesystem paths" now also covers artifact paths and CLI path arguments.
- The oracle loads `.npz` with `allow_pickle=False`, a deliberate guard against "arbitrary deserialization from an injectable cache path" (`layers.py` L113–116, L138–140). The equivalent Rust rule: never deserialize executable or polymorphic formats from untrusted paths.
- From A§14, keep the following if the CLI exports or imports CSV: "Validate imported CSVs and reject formula injection on export/import paths", plus a source lineage record for every published prediction.

### §19 Testing Strategy (L673–695): VALIDATION, KEEP

L675 says "Four levels of testing", but five subsections follow. This is a defect; after dropping §19.5 there are four.

- **§19.1 Unit (L679), verbatim:** "Required for matrix operations, affine transforms, ridge regression, RAPM construction, Kalman predict/update, RTS smoothing, empirical-Bayes calculations, and feature transformations."
  - Add fixed-lag smoothing (A§12.1 lists it) and the CG solver diagnostics.
  - Port the oracle unit tests as parity cases: `tests/grid/test_incremental.py` (25 tests), `test_kalman_numerical.py` (Joseph symmetry/PD, near-singular RTS), `test_rapm_robustness.py`, `test_layers.py`, `test_priors.py`, `test_situations.py`, `test_changepoint.py`, `test_coaching_changes.py`.
- **§19.2 Golden-file (L683), verbatim:** "Known datasets must produce known or bounded expected outputs. Snapshot model outputs on fixed synthetic datasets; CI fails if changes to the math core silently shift predictions beyond a tolerance band."
  - This is where the **cross-language parity contract** lives (see §5).
  - The oracle golden is `tests/grid/golden/snapshot.npz`, built by `tests/grid/golden_master.py`. It freezes player ratings, team ratings, weekly QB credit and the focus-QB Kalman trajectory, and is checked at `rtol=1e-5, atol=1e-6` in-language (`test_golden_master.py` L132).
  - **Process conflict.** The oracle's golden is regenerated with `python -m tests.grid.golden_master`, and its docstring says "the golden diff in the PR is the feature". A§6.6 rule 3 and the repo CLAUDE.md say "Golden files are never regenerated without a reviewed semantic explanation". Under `reference/python/`, freeze the goldens: regeneration requires an approved model-spec change.
- **§19.3 Integration (L687), verbatim:** "API → normalization → SQLite → feature build → model update → model persistence." Here "API" means the raw-file fixture, with no live network (A§4.6: "Live network calls are prohibited in normal unit and integration tests").
- **§19.4 Failure (L691), verbatim:** "duplicate API responses, missing fields, malformed data, network timeout, database failure, interrupted training, interrupted model promotion, application restart during daily update."
  - "Application restart" becomes "process kill (SIGKILL) at each job boundary, then rerun".
  - Add from A§12.5: provider schema change, NaN model output, corrupt model file.
- **§19.5 FFI boundary (L695): DROP** "Round-trip tests confirming `Vec<f64>` ↔ `Float64List` conversions…". Keep the idea as **serialization round-trip tests**: `Vec<f64>`/matrices ↔ SQLite BLOB/artifact format ↔ parity-fixture interchange format (Parquet/JSON/npz) must preserve bits and ordering. This matters because the Rust↔Python oracle comparison crosses a language boundary.

**Repo impact.** The `test-ffi` recipe (`justfile` L55–56), `scripts/verify.sh` L36–37, `scripts/verify.ps1` L51–53 and `crates/ffi` feature `flutter-bridge-tests` all implement §19.5. The repo CLAUDE.md says "Verification recipes are a frozen contract", so removing them needs an ADR (D7).

### §20 Performance Requirements (L699–714): PLATFORM, KEEP 6 of 8

- L701: "Performance is benchmark-driven rather than assumption-based."
- Benchmarks must cover (keep):
  - "Typical daily data volume and maximum expected historical data volume";
  - "Sparse RAPM construction and solve";
  - "Kalman update and RTS smoothing window";
  - "Feature construction";
  - "Boosting update";
  - "Serialization/deserialization".
- DROP: "FFI transfer for realistically-sized vectors", "Flutter chart rendering with downsampling/pagination".
- L714 (keep): "No performance claim should rely solely on terms like 'zero-copy' or 'O(1)' without qualifying dimensions and workload."
- **final-build gives no numeric targets.** A§13 does, provisionally, on a reference machine with 8+ logical cores, 16 GB RAM and NVMe:
  - normal daily incremental pipeline < 10 min;
  - full three-season rebuild < 45 min;
  - memory < 4 GB normal / < 8 GB rebuild;
  - projection-board query p95 < 200 ms, player detail p95 < 250 ms (engine query latency);
  - all-player weekly simulation < 90 s.

  Carry the numbers that apply to the engine into the new spec as provisional.
- Oracle precedent: `tests/grid/test_performance.py` asserts that the vectorized `build_design` is ≥ 1.5× faster than the reference loop and byte-identical to it.
- Benches belong in `benches/` (deferred to P1-07 by ADR-006).

### §21 Dependency Strategy (L718–738): PLATFORM, KEEP-R

- "Keep the initial Rust dependency set intentionally small."
- **Core (L724–734):** `tokio, reqwest, sqlx + SQLite, flutter_rust_bridge, nalgebra, sprs, statrs, polars, rayon`.
- **Boosting:** "XGBoost-compatible native backend behind the `IncrementalBooster` trait. Pin the XGBoost version and vendor prebuilt artifacts when possible."
- **Other:** "Add only when they provide substantial value. Prefer direct implementations… (e.g., hand-rolled sparse ridge/RAPM instead of `linfa`)."

**Against the current `/home/user/GRID-Engine/Cargo.toml` `[workspace.dependencies]`:**

| Dependency | Status |
|---|---|
| `flutter_rust_bridge = "=2.5.0"` | **DROP** together with its comment block. |
| `reqwest` | **Absent.** Add only if the engine fetches over the network (D3). |
| `polars` | **Absent.** Optional per §10; heavy, so justify before adding. |
| `tokio 1.40`, `sqlx 0.9`, `nalgebra 0.33`, `sprs 0.11`, `statrs 0.17`, `rayon 1.10` | Present and kept. |
| `serde`, `serde_json`, `thiserror`, `anyhow`, `chrono`, `uuid`, `tracing`, `tracing-subscriber` | Present but not in the §21 list. Reasonable; record them in the consolidated dependency list. |
| CLI argument parser (e.g. `clap`) | **Needed and new.** Requires justification under A§14.2. |
| Seeded RNG crate | **Needed and new.** Required for the synthetic generator and simulation (`statrs` is not a seeding policy). |
| `xgb` or `linfa-trees` | **Needed and new.** Decision D2. |
| Parity fixture reader (`parquet`/`arrow`, or `polars`, or `ndarray-npy`) | **Needed and new.** Unless the goldens are converted to JSON or CSV. |

All crate names and APIs must be verified against pinned docs before adding (repo CLAUDE.md rule). I did **not** verify the current state of the `xgb` crate.

### §22 Application Lifecycle (L742–778): MIXED, KEEP-R

**Startup as written:** Flutter startup → Initialize Rust bridge → Initialize application core → Open SQLite / run migrations → Load latest model metadata/state → Load application settings → Check daily-update status (catch-up if needed) → Start background scheduler → Render dashboard.

**Engine CLI invocation:** init tracing → open SQLite → migrate (auto, or require `grid db migrate`; pick one; auto is the current `just db-migrate` style) → acquire process lock → load production pointer, model metadata and state lazily → load `engine_settings` → detect stale `RUNNING` jobs (resume or mark failed) → run the subcommand (for `update`, evaluate catch-up) → exit with a status code.

- DROP: Flutter startup, bridge init, background scheduler and render dashboard.
- **Shutdown, KEEP-R (L768–778):** "Stop accepting new jobs → Finish/cancel safe background work → Persist required state → Close database → Shutdown Rust runtime." For the CLI this becomes SIGINT/SIGTERM handling: finish the current job to a durable boundary (or cancel it), mark it in the job table, then close the database.

### §23 Implementation Order (L782–800): PROCESS, REPLACE

| Phase | Text | Disposition |
|---|---|---|
| 1 (L785) | "Flutter Windows + FRB + Rust initialization + SQLite" | DROP. Rust init and SQLite are already delivered by P1-00 (`migrations/0001`, `.sqlx`). |
| 2 (L788) | "Provider API + normalization + database schema + daily update + idempotency + SQLx migrations + query cache" | KEEP-R. |
| 3 (L791) | "Ridge → RAPM → Kalman → Empirical-Bayes → Affine → RTS → IncrementalBooster trait + fallback" | KEEP-R. Reorder for the port: Affine → Ridge (dense + prior-mean + mask) → sparse CG → Kalman → full RTS → fixed-lag → EB → booster trait + pure-Rust implementation → V(s) → RAPM design → fixed point and Layer-1. Gate each on Python parity fixtures. |
| 4 (L794) | "Daily feature rebuild/update → Kalman update → RAPM update → EB update → GB continuation/replay → validation → promotion → snapshot/rollback" | REPLACE with the GRID DAG (§12.1 above). Kalman-before-RAPM is wrong for GRID, and snapshot comes **before** updates, not after (§12.3). |
| 5 (L797) | "Dashboard + player views + team views + model diagnostics + confidence bands + heatmaps + `TrainingStatus` UI" | DROP. Keep "model diagnostics" and "confidence bands" only as engine query outputs. |
| 6 (L800) | "Crash recovery + logging + resource limits + golden-file tests + FFI round-trip tests + installer + code signing + clean-machine testing" | KEEP crash recovery, logging, resource limits and golden tests. DROP FFI, installer and signing. "Clean-machine" becomes "clean-checkout bootstrap, build, test" (A§18 item 13). |

A§9.5 (P1-00 … P1-11) and A§10.5 (P2-00 … P2-08) are a different decomposition; see the alpha matrix row 20.

### §24 Explicit Non-Goals for V1 (L804–819): MIXED, KEEP-R

| Non-goal | Disposition |
|---|---|
| WebView-based rendering | Moot. |
| Continuous network polling | KEEP. |
| Always-running server process | KEEP. Argues for a one-shot CLI (D3). |
| Python runtime; Pandas | KEEP for the **Rust engine runtime**. Clarify that `reference/python/` is a dev/test-only oracle that the engine binary never imports or launches. CI may run it (D5). |
| Browser-hosted dashboards | Moot. |
| Cloud database; cloud model inference | KEEP. |
| Full model retraining after every new game | KEEP. |
| Literal one-file `.exe` packaging | Moot. |

L819 (KEEP): "The application is intended to operate locally and remain useful without a continuously running cloud service."

---

## 3. Internal defects in `final-build-spec.md` (fix during the rewrite)

1. L675: "Four levels of testing" is followed by five subsections (19.1–19.5).
2. L329: `last_successful_update > configured_interval` is ill-typed. It should be `now − last_successful_update > configured_interval`.
3. L288: "partial-data handling    ↓" merges two diagram lines.
4. L488–496: the §12.1 parallel-branch diagram is garbled; the branch labels are interleaved ("RAPM incremental Empirical-Bayes    │              │ update").
5. L390–392 vs L364–366 / L380: the documented objective omits weights and intercept treatment that §11.2 and §11.3 require.
6. L371: "conjugate gradient descent" is a misnomer; CG is not gradient descent.
7. L513 vs L573/L581/L625: snapshot-by-copy and rollback versus immutable artifacts.
8. L513: the snapshot list omits RAPM state; RAPM is the only incrementally updated model other than Kalman.
9. L171–181 vs L308–323 / L540–545: `TrainingStatus` cannot express the failure or model states; `Failed{reason: String}` is untyped.
10. L539–546 vs L525–535: the model-state list order conflicts with the flow (CANDIDATE vs VALIDATING).
11. L215 vs L794: the L215 example runs RAPM and Kalman in parallel; Phase 4 runs them sequentially (Kalman → RAPM); §12.1 draws them in parallel. Three different relations.
12. L466 vs L359/L738: the boosting fallback is built on `linfa-trees` while §11.2 and §21 steer away from `linfa`; `linfa-trees` is missing from the dependency list.
13. L268 + L329 vs L273 and §24: an in-process daily scheduler plus "Start background scheduler" (L761) implies a long-running process; §24 lists "Always-running server process" as a non-goal.
14. §8.1 uses basketball entities (`lineups, stints, possessions`, "players on court", "home-court", "possession weighting") for an NFL engine.
15. L161 vs L146: queries expose `get_confidence_bands`, but §11 never defines how bands are produced (filtered vs predictive variance).

---

## 4. final-build-spec vs alpha-spec: overlap and disagreement matrix

A§1.5 places `final-build-spec.md` at priority 1. In each row, "Resolution" is my recommendation for the consolidated engine spec. Where it changes behaviour, it is listed as an owner decision.

| # | Topic | final-build | alpha-spec | Relation | Resolution for the engine spec |
|---|---|---|---|---|---|
| 1 | Domain entities | §8.1 L229–247: games, teams, players, **lineups, stints, possessions**, … | A§1.2 L58–73 replaces them with games, drives, plays, player/team-week stats, roster and depth-chart snapshots, snap counts, participation, college, identity links, projection runs, market benchmark snapshots. A§8.5 lists ~60 tables in 6 groups. | Alpha **interprets** final for the NFL. | Derive from the GRID plays contract plus A§8.5. A§8.5 has no `teams` or `player_ratings`; final does. Keep both. |
| 2 | **RAPM on the live path** | §1 Req 2, §11.3 L375 "first-class production model"; §12.1 daily "RAPM incremental"; §23 Ph 3–4 | A§1.2 L75: "a traditional basketball-style RAPM implementation is not allowed to become a hidden dependency of the live projection path… the production ensemble must remain valid when current-season participation fields are unavailable". A§6.2: "research-only if live feature parity is absent". A§6.4: "Optional sparse adjusted-effect model". A§8.6 daily pipeline omits RAPM. A§9.5 P1-06 omits RAPM. | **Conflict.** Alpha narrows final. For GRID, RAPM is Layer 2 and feeds the Kalman filter. | **Owner decision D1.** |
| 3 | Fetch cadence | §1 L18 / §9.1 "once per day at a configurable local time", manual run, retries with backoff | A§1.1 "**at most** once per local calendar day"; A§8.6 "day-of-week-aware local schedule, still capped at one external fetch per calendar day" | Overlap; alpha tighter and ambiguous about retries and manual runs. | Count one *successful* fetch per source per day; retries and no-fetch re-runs are exempt (§9.1 above). |
| 4 | Daily pipeline order | §12.1: features → snapshot → Kalman ‖ RAPM ‖ EB → boosting → validate → promote | A§8.6: raw/hash → normalize/quarantine → commit data version → **resolve identities** → **build only affected feature partitions** → snapshot → Kalman **+ fixed-lag smoothing** → EB → bounded boosting → **candidate weekly projections** → validate against invariants and recent holdouts → promote/reject | Overlap with differences: alpha adds identity resolution, partial feature rebuild, fixed-lag and the projection step, and drops RAPM. | Use the GRID DAG (§12.1 above). |
| 5 | RTS timing | §23 Ph 3 includes RTS in the statistical engine; §11.5 fixed-lag in production | A§9.5 P1-06 "scoring, ridge, Kalman, EB, affine, boosting adapter" (no RTS); fixed-lag in P2-03; A§9.4 golden list omits RTS | Phase disagreement. | Full RTS in the first numerical package (the oracle has it); fixed-lag immediately after (exact parity target). |
| 6 | Boosting role | §11.8 generic adapter; §12.2 modes | A§6.2: "Nonlinear residual correction, interaction effects, availability/workload models, component-rate models" | Overlap. GRID uses boosting for V(s) and the Layer-1 context — a third role. | The engine spec lists the GRID roles. A residual booster is optional. |
| 7 | Affine | §11.7 generic utility | A§5.4 scoring as a versioned affine transform; A§2.3 custom scoring "represented as a versioned affine mapping"; A§6.2 college-to-NFL translation | Compatible; alpha supplies the uses. | Keep both uses plus SV→points. |
| 8 | EB | §11.6 persistence list; "estimator must be documented" | A§6.3 explicit formula (`θ_posterior`, `n0` learned by rolling-origin, `q` mixing, influence cap, wider variance for undrafted and identity-uncertain players) | Compatible; alpha fills final's gap. | Adopt the A§6.3 form in the EB model spec. |
| 9 | Commands | §5.1 select_team, select_player, set_lambda, set_date_range, request_model_rebuild(model_id), trigger_daily_update | A§8.2 trigger_daily_update, request_projection_run(season, week, lock_type), request_model_rebuild(**model_family**), set_scoring_profile, import_availability_overrides, approve/reject_identity_link, import_benchmark_snapshot, promote_candidate (only through governance), rollback_to_snapshot | Partial overlap; `model_id` vs `model_family` differs. | The engine CLI takes the union minus UI-selection commands. Benchmark and identity commands depend on the alpha-scope decision. |
| 10 | Queries | §5.2 get_dashboard, get_player_ratings, get_predictions, get_confidence_bands, get_model_status, get_ingestion_status | A§8.3 get_week_projection_board, get_player_projection_detail, get_projection_distribution, get_projection_change_log, get_data_freshness, get_ingestion_status, **get_training_status**, get_identity_review_queue, get_model_scorecard, get_benchmark_results, get_data_quality_report | Partial overlap. `get_player_ratings` (GRID's core output) exists only in final. | Union, without dashboard and board naming. |
| 11 | Events | §5.3 DataFetchStarted, DataFetchCompleted, **DataChanged, RAPMUpdated, KalmanUpdated, BoostingUpdated, ModelValidationCompleted**, ModelPromoted, JobFailed | A§8.4 DataFetchStarted, DataFetchCompleted, **DataPartialSuccess, IdentityReviewRequired, FeaturesBuilt, LatentStatesUpdated, ProjectionRunCompleted, BenchmarkSnapshotImported, EvaluationCompleted, CandidateValidated, CandidateRejected**, ModelPromoted, **RollbackCompleted**, JobFailed | Partial overlap; same "no payload" semantics (§5.3 L167 = A§8.4 L1029). | Union, plus GRID stage events. |
| 12 | Training status | §5.4 `TrainingStatus` enum | A§8.3 `get_training_status()` query only | Overlap. | Extended enum (§5.4 above). |
| 13 | Validation design | §13 rolling-origin / walk-forward / time-based holdout; metrics MAE, RMSE, log loss, Brier, calibration error, rank correlation; **no criteria** | A§7.1 rolling-origin week-by-week; A§2.4 strict three-season window; A§7.4 PB-MAE primary; A§7.5 secondary (incl. CRPS, 50/80% coverage); A§9.4/A§7.8/A§7.9 numeric gates; A§7.10 implementation-correctness gate | Compatible; alpha adds the criteria final lacks. | Engine promotion gate per D6. A§7.10's correctness gate transfers directly. |
| 14 | Snapshot / rollback | §12.3 automatic copy and rollback | A§8.2 manual `rollback_to_snapshot(snapshot_id)`; A§10.2 "Automatic reject/rollback on degenerate output. Manual promotion is permitted only after the same validation report is generated."; A§10.3 "Candidate promotion is serialized" | Compatible; alpha adds the manual path. | Pointer semantics; both automatic and manual. |
| 15 | Testing | §19 unit, golden, integration, failure, FFI | A§12.1–12.9 add **point-in-time and leakage tests** (A§12.3), AI-specific regression protections (A§12.7), PR evidence (A§12.8), reviewer checklist (A§12.9); golden list (A§12.2) is projection-specific | Alpha superset. final has **no leakage tests**. | Keep A§12.3 leakage tests; the oracle has `tests/validation/test_leakage_guards.py`. |
| 16 | Performance | §20 benchmark list, no numbers | A§13 numeric provisional targets and reference machine | Compatible. | Carry over the engine-relevant numbers. |
| 17 | Security | §18 six bullets | A§14 adds Windows-native credential protection, license and attribution metadata, CSV formula injection, lineage, no competitor training; A§14.1 agent sandbox; A§14.2 dependency policy (justification, license, advisory, maintenance, Windows compatibility, SBOM) | Alpha superset. | Keep A§14.2 (dependency gate) and lineage. "Windows-native credential protection" becomes OS keychain or environment variables. |
| 18 | Dependencies | §21 core list including FRB, reqwest, polars | A§1.1 "Polars may be used inside Rust"; A§8.11 pins "Rust, Flutter/Dart, FRB, SQLx, XGBoost/native artifacts" | Overlap. Neither is satisfied by the current workspace (no reqwest, polars or XGBoost). | Engine dependency list per §21 above. |
| 19 | Deployment | §3.2 single installable application (MSIX), MSVC, signing | A§17.1 "Native Windows alpha installer"; A§8.11 Windows-authoritative verification; A§10.4 clean-machine installer test | Overlap; both app-only. | DROP; decide the authoritative CI platform (D4). |
| 20 | Phasing | §23 six phases (Shell → Data → Stat engine → Incremental → Viz → Hardening) | A§9.5 P1-00…P1-11; A§10.5 P2-00…P2-08. A P1-06 "Numerical primitives: scoring, ridge, Kalman, EB, affine, boosting adapter" vs final Ph 3 "Ridge → RAPM → Kalman → EB → Affine → RTS → Booster" | Different decompositions; different method sets in the first numerical phase. | New engine work-package sequence. |
| 21 | Non-goals | §24 (WebView, polling, server process, Python, Pandas, browser, cloud DB, cloud inference, full retrain per game, one-file exe) | A§16 (mobile, web, cloud inference, always-running server, real-time in-game, continuous polling, league sync, draft assistant, DFS, betting, IDP, social, NL news, paid data, redistribution, embedded Claude, agent merge/sign, agent confidence as evidence) | Overlap on cloud, server and polling; alpha adds product non-goals. | Merge; add "Python reference is not a runtime dependency". |
| 22 | Determinism | §14 "random_seed, when applicable"; silent on parallelism | A§6.6 rule 5 "Randomness is seeded and recorded. Parallel execution must not make published predictions nondeterministic beyond documented tolerance."; A§9.3 "byte-stable tabular predictions within documented floating-point tolerance" | Alpha stricter. | Adopt the alpha wording (§7 above). |
| 23 | Concurrency and resources | §7 resource modes; publication serialized | A§1.1 same; A§9.3 "All CPU-heavy work runs outside the Flutter isolate and Tokio async executor"; A§15 "Desktop compute contention: resource modes, bounded worker pool…" | Consistent. | Keep. |
| 24 | Python | §24 non-goal "Python runtime", "Pandas" | A§1 L37 "must not introduce a temporary Python service"; A§1.1 "No Python runtime in the installed application" | Consistent with each other. The owner's **reference/python import** is compatible only as a dev/test oracle. | Record in an ADR (D5). |
| 25 | Authority | (none) | A§1.5 final-build = 1, alpha = 2, ADRs = 3 … | — | After consolidation both documents should be archived and superseded by the engine spec, with a new authority order (D7). |

---

## 5. final-build-spec vs the Python oracle: parity regime

**Why fixtures, not a re-run of the synthetic generator.** The synthetic generator (`backend/grid/synth.py`) uses `np.random.default_rng(cfg.seed)` with `seed=7` (L45, L165) and `cfg.seed + 99` (L308). It uses numpy PCG64 streams and numpy's normal sampler; this environment has numpy 2.4.6 and sklearn 1.9.1. Rust will **not** reproduce those streams bit-for-bit without reimplementing numpy's PCG64 and its normal sampler. So:
- numeric parity must run on **Python-generated, committed fixture datasets** (plays, players, market, college, plus intermediate arrays) consumed by Rust;
- a native Rust synthetic generator can be validated only by **recovery gates** (statistical), never by byte parity.

**Recommended tolerance classes for §19.2 (for statistical-owner sign-off; these are recommendations, not existing requirements):**

| Class | Components | Parity criterion vs oracle on identical inputs |
|---|---|---|
| A: closed-form deterministic | affine; dense ridge (prior-mean, mask); Kalman predict/update (Joseph); full RTS; EB weights | relative error ≤ ~1e-9 element-wise (float64, BLAS order differences). |
| A′: exact internal | fixed-lag vs Rust full RTS on the same window | ≤ 1e-12. |
| B: iterative | sparse-CG RAPM vs oracle dense `np.linalg.solve` | `‖β_rs − β_py‖ / ‖β_py‖ ≤ 10 × CG tolerance`. Diagnostics must report `converged=true`. |
| C: algorithmically different | gradient boosting (V(s), Layer-1 context) | No value parity. V(s) surface correlation on the state grid, plus the downstream Class D gates. |
| D: end-to-end recovery | full fit on synthetic data | Oracle Tier-0 floors (`tests/grid/test_tier0_recovery.py`), listed below. |

**Class D floors:**
- pooled attribution correlation ≥ **0.77** (observed 0.8025);
- per position: QB ≥ 0.83, RB ≥ 0.70, WR ≥ 0.76, TE ≥ 0.73, DEF ≥ 0.73 (observed 0.869 / 0.743 / 0.797 / 0.771 / 0.765);
- team-strength correlation ≥ **0.60** (0.6643);
- Kalman `total_smooth` correlation ≥ **0.92** (0.958) and `tau_smooth` ≥ **0.60** (0.6745);
- injury return widens variance (≈ 0.0023 → 0.0061);
- filter NIS bounded;
- prior equivalency `oos_r2` ≥ **0.05** (0.149);
- rookie prior correlation ≥ **0.50** (0.5829).

**Oracle behaviours that violate final-build. Port faithfully, or fail with a typed error? (D5):**
1. Ridge/RAPM `lstsq` fallback when `cond > 1e10`, warning only (`layers.py` L353–357) vs §11.3 L404.
2. Kalman `load()` returns `None` on a corrupt width, which triggers a reinit (`statespace.py` `KalmanState.load`) vs §16 and A App. D #8.
3. `weekly_update.run` logs and skips on data-load failure and returns a dict vs §9.5 typed states.
4. Accumulators are saved before the solve and Kalman step (non-atomic) vs §12.4 and §16.
5. The V(s) model is refit on every weekly run on the whole season's plays (`weekly_update.py` L210–213) vs Req 3 "without requiring full retraining", the consistency of accumulated `XᵀWy`, and as-of leakage on backfill.
6. Mutable `.npz` state in `data/cache/` vs §8 L223 and §16 rule 5.
7. No persisted `SSParams` vs §11.4 "configured transition/observation model are persisted".

Recommendation: parity fixtures exercise only the **healthy path**. On the failure paths, Rust follows final-build (typed failure); oracle tests that encode the violating behaviour are marked "non-parity, intentional divergence" in `reference/python/PARITY.md`.

---

## 6. GRID-Engine repo artifacts that implement APP-ONLY final-build sections (pivot impact)

| Artifact | final-build basis | Action |
|---|---|---|
| `crates/ffi/` (feature `flutter-bridge-tests`; `src/generated/`) | §2 L44, L93; §19.5 | Remove the crate from the workspace `members`. |
| `Cargo.toml` `flutter_rust_bridge = "=2.5.0"` and its comment | §21 | Remove. |
| `app/pubspec.yaml`; `toolchains/flutter.version` | §1 Req 1, §4, §6 | Remove. |
| `justfile` `test-ffi` (L55–56), `verify` calling `just test-ffi` (L33), `serve-ui` (L85–86) | §19.5, §4 | Remove via ADR (frozen-recipe rule). |
| `scripts/verify.sh` L36–37; `scripts/verify.ps1` L51–53 | §19.5 | Same ADR. Keep three-way parity (`check-verify-parity.sh`). |
| `.github/workflows/alpha-ci.yml` `windows-authoritative` job (L120ff) | §3 (Windows target) via A§8.11 | Keep or demote per D4. |
| `CLAUDE.md` and `docs/CLAUDE.md` non-negotiables ("Native Flutter Windows UI only", "flutter_rust_bridge v2 is the sole UI/core boundary", "CPU-heavy work never runs on the Flutter isolate"; generated-files row `crates/ffi/src/generated/`) | §1, §2, §6.1 | Rewrite. |
| `deny.toml` L25–27 comment ("redistributing the Windows installer (final-build-spec.md 3.2)") | §3.2 | Re-justify the permissive-only policy for a library and CLI. |
| `crates/models/src/lib.rs` doc string (omits RTS) | §11.5 | Fix during the models work package. |
| ADR-001/002/003/009 cite final-build §3.2 and §8 | §3.2, §8 | §8 citations remain valid. §3.2 citations need a superseding note. |

---

## 7. Where each surviving final-build requirement should land in the consolidated engine spec (proposed outline)

1. **Scope and non-goals:** §1 Req 2 and Req 3, the L20 priorities, merged §24 and A§16 (engine-relevant parts), and the Python-oracle status.
2. **Architecture:** §2 (engine boxes), §4.1 "Rust owns" inventory, the §7 concurrency and determinism rules, and the crate map (models synchronous; no FFI; CLI as a thin adapter).
3. **Engine API:** §5 commands, queries and events merged with A§8.2–8.4, the extended `TrainingStatus`, CLI subcommands and exit codes.
4. **Data:** §8.1 (NFL/GRID domains from the plays contract), §8.3 and A§4.1.1 retention metadata, §9.2–9.5, leakage rules (A§4.5), and the game-week time index.
5. **Statistical model specs (one per method, A§6.6 template):**
   - Ridge (generalized objective);
   - RAPM (play-level design, team intercepts, Layer 3, fixed point, sparse CG, diagnostics, live-path decision);
   - Kalman (`SSParams`, interventions, scheme resets, predictive variance);
   - RTS full and fixed-lag (`L`);
   - EB (estimator);
   - Affine (uses, inverse rule);
   - Boosting (revised trait, roles, modes).
6. **Incremental pipeline:** the GRID DAG (replacing §12.1), §12.2 modes defined, §12.4.
7. **Governance:** §13 state machine and promotion criteria (D6), §14 version record, §12.3 and §16 pointer-based rollback.
8. **Durable jobs and recovery:** §15 with the full semantics, §16, §22 CLI lifecycle.
9. **Observability and security:** §17, §18 (+A§14.2).
10. **Verification:** §19.1–19.4, the §19.2 cross-language parity regime (§5 above), A§12.3 leakage tests, §20 benchmarks with the A§13 numbers.
11. **Dependencies:** §21 revised list.
12. **Work-package sequence:** replaces §23 and A§9.5/A§10.5.

---

## 8. Decisions needed (owner-level)

- **D1. RAPM and the live path.** final-build makes RAPM a production model updated daily. A§1.2/§6.2 forbid it as a live dependency because current-season participation is unavailable (nflverse participation is post-season only; A Appendix A). In GRID, the Kalman observations themselves are weekly RAPM / Layer-1 output. Options:
  - (a) Allow RAPM in-season only when participation exists, and define the in-season Kalman observation without it.
  - (b) Treat RAPM as a retrospective / completed-season model that seeds priors, with in-season updates from a participation-free signal.
  - (c) Accept the participation dependency and approve a licensed in-season participation source.
- **D2. Booster backend priority.** XGBoost primary (final §11.8, §21, §3.2 pin/vendor) vs a pure-Rust booster as primary for an engine with no installer. This includes approval of `linfa-trees` (or hand-rolled trees) and the `IncrementalBooster` trait revision.
- **D3. Engine operating model.**
  - One-shot `grid update` CLI driven by an OS scheduler, or the in-process Tokio scheduler of final §9.1/§22 (which conflicts with the §24 "always-running server" non-goal)?
  - Does the engine perform network fetches itself (`reqwest`), or ingest pre-downloaded raw files?
- **D4. Authoritative CI platform.** Should Windows (`verify.ps1`, the `windows-authoritative` job, ADR-009) remain the merge and release gate once there is no Windows app, or should Linux (or a cross-platform matrix) become authoritative?
- **D5. Cross-language parity regime.**
  - Approve tolerance classes A–D.
  - Should CI run the Python oracle live (a pinned Python toolchain in CI) or only consume committed Python-generated fixtures and goldens?
  - Should Rust port oracle behaviours that violate final-build (lstsq fallback, corrupt-state→reinit, skip-on-failure, refit-V-every-run) faithfully, or fail typed and document the divergence?
- **D6. Promotion criteria (statistical owner).** final §13 defines states but no rule. Approve the engine gate (numerical diagnostics clean, synthetic recovery floors, rolling-origin non-inferiority margin, calibration band). Separately decide whether A§7's PB-MAE and market gates stay in engine scope. Also set the open parameters: fixed-lag window `L`, the EB estimator and `n0`/`k` estimation, the sanity thresholds for auto-rollback, and the home-field, garbage-time and overtime treatment in RAPM.
- **D7. Authority and frozen contracts.** Retire `final-build-spec.md` and `alpha-spec.md` (archived as historical) in favour of the consolidated engine spec with a new authority order. Authorize an ADR that changes the "frozen" verification recipes (removing `test-ffi`, `serve-ui` and the Flutter toolchain) and amends the CLAUDE.md non-negotiables.
