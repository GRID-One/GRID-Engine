# Native Windows Sports Analytics Application — Final Build Specification

## 1. Product and Architecture Requirements

The application is a **native Windows desktop analytical application** with a native Flutter UI and a high-performance Rust computation/data engine.

Three requirements are non-negotiable:

1. The user interface must be a **native Flutter desktop UI**. No WebView, embedded browser, Electron, HTML-based dashboard, or equivalent browser-rendering layer may be used.
2. The statistical engine must support all of the following as first-class capabilities:
   - RAPM
   - Ridge regression
   - Kalman filtering
   - RTS smoothing
   - Gradient boosting
   - Empirical-Bayes shrinkage
   - Affine transformations/mapping
3. New external data is fetched **once per day**, after which the application performs incremental data processing and model updates without requiring full retraining of every model.

The implementation optimizes for statistical correctness, reproducibility, deterministic model versioning, crash recovery, low UI latency, efficient CPU utilization, maintainable native deployment, and explainability of model outputs.

The application is delivered as a **single installable Windows application** with all required native dependencies packaged together. A literal single-file executable is not required because the gradient-boosting dependency may require native shared-library deployment.

---

## 2. System Architecture

```text
┌─────────────────────────────────────────────────────────────┐
│                     Flutter Windows UI                      │
│                                                             │
│  Riverpod                                                   │
│  ├── UI state                                               │
│  ├── filters                                                │
│  ├── selections                                             │
│  └── ephemeral presentation state                           │
│                                                             │
│  Visualization                                              │
│  ├── fl_chart (line, scatter, bar, radar)                 │
│  ├── custom Flutter painters (heatmaps, confidence bands)    │
│  └── specialized rendering layers                           │
└──────────────────────────────┬──────────────────────────────┘
                               │
                    flutter_rust_bridge v2
                               │
┌──────────────────────────────▼──────────────────────────────┐
│                     Rust Application Core                   │
│                                                             │
│  Application Services                                       │
│  ├── commands                                                │
│  ├── queries                                                 │
│  └── events / training status streams                       │
│                                                             │
│  Async I/O Runtime (Tokio)                                  │
│  ├── reqwest (HTTP fetch)                                   │
│  ├── daily scheduler                                        │
│  ├── async database operations (SQLx)                       │
│  └── event notification                                     │
│                                                             │
│  CPU Worker Pool (spawn_blocking / Rayon)                   │
│  ├── feature generation │
│  ├── RAPM / Ridge solver │
│  ├── Kalman / RTS smoothing │
│  ├── Empirical-Bayes updates                               │
│  └── Gradient boosting training                            │
│                                                             │
│  Data / Ingestion                                            │
│  ├── normalization                                           │
│  ├── validation                                              │
│  └── reconciliation + partial-data handling │
│                                                             │
│  Persistence                                                 │
│  ├── SQLite (authoritative source of truth)                │
│  └── SQLx (compile-time checked queries, migrated schema)  │
│                                                             │
│  Statistical Engine │
│  ├── Ridge regression (hand-rolled, nalgebra/sprs)        │
│  ├── RAPM (sparse CG solver, domain-specific prep)          │
│  ├── Kalman filtering (online state update)                 │
│  ├── RTS / fixed-lag smoothing (backward revision)          │
│  ├── Empirical-Bayes shrinkage                              │
│  ├── Affine transformations                                 │
│  └── Gradient boosting (trait-abstracted adapter)           │
│                                                             │
│  Model Governance                                            │
│  ├── versioning                                              │
│  ├── time-aware validation                                   │
│  ├── promotion / rejection                                   │
│  └── snapshot / rollback                                     │
└─────────────────────────────────────────────────────────────┘
```

The FFI layer is an adapter between the Flutter UI and Rust application services. Business logic must not be embedded directly in generated FFI bindings.

---

## 3. Deployment Model

### 3.1 Windows-native requirement

The application must not use:

- WebView
- Chromium
- Electron
- browser-based visualization
- HTML dashboards embedded in the application
- remote browser UI
- JavaScript application shells

Flutter Windows is the sole presentation technology.

### 3.2 Packaging

The deployment target is a **single installable application**, not necessarily a single PE executable.

A release contains:

```text
SportsAnalytics/
├── SportsAnalytics.exe
├── flutter_windows.dll
├── rust_engine.dll
├── native ML/runtime dependencies (XGBoost libs, MSVC redist)
├── data/
└── assets/
```

Packaged via MSIX/installer technology.

- **XGBoost version must be pinned** and native artifacts **vendored in-repo** where possible so CI/release builds do not depend on runtime network access to a third-party wheel host.
- The **MSVC redistributable must be bundled** with the installer. Statically linked C++ dependencies from XGBoost still typically require the VC++ runtime on clean Windows machines.
- Plan for **code signing** if distributing outside a fully controlled environment, to avoid Windows SmartScreen warnings.
- The Rust engine is isolated behind a stable native boundary so that native dependencies can be changed without requiring architectural changes to the Flutter application.

---

## 4. Frontend: Flutter Desktop

Flutter is responsible for rendering, user interaction, navigation, chart presentation, user preferences, transient UI state, displaying job/model status, and formatting model outputs.

Flutter must not own authoritative model state or persistence.

### 4.1 State ownership

- **Rust owns:** data, model parameters, model versions, predictions, confidence intervals, feature definitions, training status, ingestion status, persisted application state.
- **Flutter owns:** transient presentation state only (selected team/player, date range, chart zoom/cursor, active tab, expanded/collapsed panels, filter controls).

---

## 5. Frontend State Synchronization

The application uses a **command / query / event** pattern. It does not stream the entire dashboard state on every change.

### 5.1 Commands

Examples: `select_team(team_id)`, `select_player(player_id)`, `set_lambda(lambda)`, `set_date_range(start, end)`, `request_model_rebuild(model_id)`, `trigger_daily_update()`.

### 5.2 Queries

Examples: `get_dashboard(query)`, `get_player_ratings(query)`, `get_predictions(query)`, `get_confidence_bands(query)`, `get_model_status()`, `get_ingestion_status()`.

### 5.3 Events

Examples: `DataFetchStarted`, `DataFetchCompleted`, `DataChanged`, `RAPMUpdated`, `KalmanUpdated`, `BoostingUpdated`, `ModelValidationCompleted`, `ModelPromoted`, `JobFailed`.

Events notify Flutter that something changed; they do not carry the complete dashboard payload. Flutter then requests the specific data it needs.

### 5.4 Training Progress Streams

Because daily retraining is a heavier, bursty compute event, the backend emits an explicit `TrainingStatus` enum to the UI:

```rust
enum TrainingStatus {
    Idle,
    Fetching,
    Retraining { progress: f32 },
    Complete,
    Failed { reason: String },
}
```

This prevents the UI from appearing frozen during long retrain bursts.

---

## 6. Visualization

- **Standard charts:** `fl_chart` for line charts, scatter plots, bar charts, and radar-style visualizations.
- **Confidence bands:** Rendered through a dedicated chart layer or custom Flutter painter, not forced into a generic chart API.
- **Heatmaps:** Use a dedicated custom painter or specialized heatmap implementation.

### 6.1 Performance requirements

- Avoid rebuilding unrelated charts.
- Paginate or window very large datasets.
- Downsample display-only series where appropriate.
- Keep heavy numerical computation outside the Flutter isolate.
- Avoid sending redundant historical datasets across FFI.

---

## 7. Rust Core and Concurrency Model

Use **Tokio** for asynchronous work and a **dedicated CPU worker pool** (`tokio::task::spawn_blocking` or a dedicated `rayon` pool) for all mathematical work. CPU-heavy mathematical work must never block the async runtime.

```rust
// Async I/O: Tokio handles HTTP requests, scheduling, async DB ops, job coordination.
// CPU-heavy computation: spawn_blocking / Rayon handles RAPM, Ridge, Kalman, boosting, EB.
let result = tokio::task::spawn_blocking(move || {
    run_incremental_update(new_rows)
}).await??;
```

The engine should allow independent workloads to execute concurrently where safe (e.g., HTTP I/O concurrent with DB operations; RAPM and Kalman on separate CPU workers if safe). Production model publication must be serialized to prevent simultaneous model updates from corrupting model state.

The runtime should support **resource modes** (`normal`, `background`, `manual_rebuild`) so CPU usage is desktop-friendly and does not interfere with user interaction.

---

## 8. Durable Persistence and Database Strategy

SQLite is the durable source of truth. In-memory Rust state is a cache that can be reconstructed from SQLite.

### 8.1 Required data domains

The schema includes at minimum:

```text
games
teams
players
lineups
stints
possessions
raw_data
feature_sets
predictions
player_ratings
model_versions
model_states
training_runs
ingestion_runs
jobs
application_settings
snapshots
```

### 8.2 SQLx workflow

- Use `sqlx` with async SQLite and **compile-time checked queries**.
- Commit the `.sqlx` query cache to version control (`cargo sqlx prepare`) so builds are reproducible without requiring a live database during compilation.
- Use `sqlx migrate` from day one. Retrofitting migrations onto an evolving schema is more painful than starting with them.

### 8.3 Raw data retention

Raw external responses are retained to enable reproducibility, debugging, provider schema changes, historical reprocessing, and auditability. The raw payload does not need to be queried during normal application operation.

---

## 9. Data Ingestion and Failure Handling

### 9.1 Daily cadence

External data is fetched **once per day** at a configurable local time. The scheduler supports:

- Configurable local time
- Manual "Run Update Now"
- Retry after failure with exponential backoff
- API rate limiting and timeout handling
- Persisted last-successful-run status

The application does not require continuous polling.

### 9.2 Ingestion pipeline

```text
External API
    ↓
raw response (retained)
    ↓
normalization
    ↓
validation
    ↓
deduplication / reconciliation
    ↓
partial-data handling    ↓
SQLite transaction
    ↓
commit
    ↓
create durable learning job
```

### 9.3 Idempotency

Every ingestion operation receives a unique `ingestion_run_id`. The pipeline is idempotent: a daily update should be safe to execute twice.

### 9.4 Partial data acceptance

When possible, the ingestion pipeline should reject malformed rows, log/quarantine them, and continue ingestion with the valid subset rather than aborting the entire day's batch. Status should reflect `PARTIAL_SUCCESS` when applicable.

### 9.5 Failure states

The application must explicitly distinguish:

```text
NOT_RUN
RUNNING
SUCCESS
NO_NEW_DATA
PARTIAL_SUCCESS
NETWORK_FAILURE
TIMEOUT
RATE_LIMITED
AUTHENTICATION_FAILURE
INVALID_RESPONSE
SCHEMA_FAILURE
DATA_VALIDATION_FAILURE
DATABASE_FAILURE
LEARNING_FAILURE
```

The UI exposes the last successful update and the current error state. A failure in the daily update must not corrupt or partially promote the previous production model.

### 9.6 Catch-up behavior

When the application starts, it detects if `last_successful_update > configured_interval` and initiates a catch-up update. Optionally, register a Windows Task Scheduler job (non-interactive update mode) to preserve the "once daily" intent while the app is closed.

---

## 10. Feature Engineering

Feature generation must be deterministic and versioned. Each feature definition receives a schema/version identifier (e.g., `feature_schema_version = 17`). A model record must reference the feature schema from which it was trained. Changing a feature definition creates a new feature schema rather than silently modifying historical semantics.

Feature generation operates from durable source data, not transient UI state.

**Polars** may be used for analytical transformations and DataFrame operations (eager/lazy execution, parallel processing). Core services continue to exchange typed domain structures and numerical arrays.

---

## 11. Statistical Engine

The engine is composed of independent modules with stable interfaces.

### 11.1 Linear algebra

Use `nalgebra` for dense matrix/vector computation.

- **Small/fixed state-space matrices:** statically sized types.
- **Large/dynamic matrices:** dynamically sized types (`DMatrix`).
- **Sparse lineup/stint structures:** `sprs` sparse matrix implementation.

Include dimension assertions and runtime validation for dynamic matrices.

### 11.2 Ridge Regression

Ridge regression is an independent, reusable statistical primitive, implemented directly with `nalgebra`/`sprs` rather than through an external ML framework like `linfa`.

It supports:

- Arbitrary feature matrices
- Configurable λ
- Regularized intercept handling
- Weighted observations
- Prediction
- Coefficient output
- Solver diagnostics

For the dense case: solve via normal equations or Cholesky with regularization. For the sparse case: use conjugate gradient descent.

### 11.3 RAPM

RAPM is a first-class production model with explicit domain controls:

- Stint boundaries
- Players on court
- Response variable
- Possession weighting
- Home-court treatment
- Team effects (if used)
- Intercept treatment
- Garbage-time handling
- Overtime treatment
- Minimum appearance thresholds

**Objective:** Document and test the exact optimization objective:

\[
\hat{\beta} = \arg\min_{\beta} \left[ \|y-X\beta\|^2 + \lambda\|\beta\|^2 \right]
\]

**Sparse representation:** The player/stint design matrix is constructed as a sparse matrix using `sprs`. Solve via iterative sparse conjugate gradient with explicit diagnostics:

```text
converged
iterations
residual_norm
tolerance
regularization_lambda
```

A numerically failed solve must not silently produce a production model.

**Preconditioning:** Support preconditioning where beneficial and benchmark against expected matrix sizes.

### 11.4 Kalman Filtering

The Kalman Filter is the primary mechanism for genuine online/incremental state updating. State (`x_k`, `P_k`) and the configured transition/observation model are persisted to SQLite between runs.

**Daily update mode:** Load persisted state, apply the day's worth of observations as a batch of sequential predict/update steps in a single CPU-bound task, then persist the new `x_k` / `P_k`.

For fixed state dimensions, the per-observation workload does not grow with historical observations. Complexity should be qualified with respect to state and observation dimensions.

### 11.5 RTS Smoothing

RTS smoothing is a distinct backward-looking operation from online Kalman filtering.

**Two modes:**

1. **Full RTS smoothing:** Used for periodic historical recalculation, backtesting, diagnostics, and explicit user-requested rebuilds.
2. **Fixed-lag smoothing:** Used for incremental production updates. After the daily Kalman filter batch, apply fixed-lag smoothing over a recent window to revise recent historical states without recomputing the entire sequence.

### 11.6 Empirical-Bayes Shrinkage

Maintain explicit prior parameters. Persist:

```text
prior mean
prior variance
estimated population variance
sample counts
shrinkage parameters
version
```

The daily pipeline recalculates or incrementally updates the prior using newly available observations. The precise prior distribution and posterior estimator must be documented as part of the model specification.

### 11.7 Affine Mapping

Affine transformations are a first-class numerical utility supporting:

\[
y = Ax + b
\]

Expose: transformation matrix, offset vector, inverse where applicable, dimensionality validation, deterministic serialization.

### 11.8 Gradient Boosting

Gradient boosting is required behind an application-defined adapter.

```rust
pub trait IncrementalBooster {
    fn load_or_init(path: &Path) -> Result<Self> where Self: Sized;
    fn continue_training(&mut self, new_data: &FeatureMatrix) -> Result<()>;
    fn predict(&self, features: &FeatureMatrix) -> Result<Vec<f64>>;
    fn save(&self, path: &Path) -> Result<()>;
    fn metadata(&self) -> ModelMetadata;
}
```

**Primary implementation:** `xgb`-backed struct.

**Fallback implementation:** Hand-rolled gradient boosting using `linfa-trees` decision trees as weak learners, with a residual-fitting loop and learning-rate shrinkage. This is a pure-Rust fallback if the XGBoost packaging/build story becomes painful.

The rest of the application must not depend directly on XGBoost-specific types.

---

## 12. Incremental Learning Pipeline

### 12.1 Normal daily pipeline

```text
Daily scheduler
    ↓
Fetch external data
    ↓
Validate / normalize / reconcile
    ↓
SQLite transaction
    ↓
Build features
    ↓
Snapshot current model state (rollback point)
    ↓ ┌──────────────┐ ┌────────────────┐
    ↓              ↓    ↓                ↓
Kalman update    RAPM incremental Empirical-Bayes    │              │ update
    └──────┬───────┘    └──────┬───────┘
           │                   │
           └────────┬──────────┘
                    ↓
            Gradient Boosting
 (continuation / replay)
 ↓
            Validation
                    ↓
            Promote candidate OR reject
                    ↓
            Notify UI
```

### 12.2 Training modes for gradient boosting

1. **Daily continuation:** Use the existing model and newly available data for a bounded incremental update to incorporate latest information quickly.
2. **Replay-window training:** Construct a training sample containing recent observations + representative historical observations (optionally weighted). This prevents overreacting to a small batch of new games.
3. **Periodic rebuild:** At a configured interval or manual trigger, perform a full gradient boosting rebuild from the historical feature store, followed by validation and promotion.

### 12.3 Snapshot and rollback

Before applying any daily update, the engine copies current model state (Kalman covariance/state, boosting model file, EB priors) to a versioned snapshot. If an update produces degenerate output (NaN, wildly divergent predictions exceeding sanity thresholds), the engine rolls back to the previous snapshot automatically, flags the failure to the UI, and leaves the previous production model untouched.

### 12.4 Pipeline resumability

The entire pipeline must be resumable. Each stage produces durable status so a crash does not require blindly repeating the entire process.

---

## 13. Model Promotion and Validation

No model is automatically production-quality merely because training completed.

```text
training
   ↓
candidate model
   ↓
time-aware validation (rolling-origin, walk-forward, time-based holdout)
   ↓
compare with current production model
   ↓
promote OR reject
```

Every model has an explicit state:

```text
TRAINING
VALIDATING
CANDIDATE
PRODUCTION
REJECTED
SUPERSEDED
```

A failed model update must leave the previous production model untouched.

Validation metrics must be persisted with each model. Possible metrics include MAE, RMSE, log loss, Brier score, calibration error, and rank correlation.

---

## 14. Model Versioning and Reproducibility

Every production model must reference:

```text
model_id
model_version
model_type
feature_schema_version
data_snapshot_version
training_run_id
training_start
training_end
hyperparameters
random_seed, when applicable
validation_metrics
build/application version
```

A model must be reproducible from its recorded data and feature versions. The on-disk representation may use directories and metadata files; model identity and metadata must be durable.

Treat models as versioned scientific artifacts, not mutable global variables:

```text
Data Version → Feature Version → Model Version → Prediction Version
```

An incremental update creates a new model version; it does not overwrite the historical definition of the previous model.

---

## 15. Durable Job Queue

Learning work is represented as durable jobs. Example job types:

```text
DATA_FETCH
NORMALIZE_DATA
BUILD_FEATURES
RAPM_UPDATE
KALMAN_UPDATE
RTS_SMOOTH
EMPIRICAL_BAYES_UPDATE
BOOSTING_UPDATE
MODEL_VALIDATE
MODEL_PROMOTE
```

Each job stores: `job_id`, `job_type`, `created_at`, `status`, `input_data_version`, `output_model_version`, `attempt_count`, `started_at`, `completed_at`, `error`.

This provides crash recovery and diagnostic history.

---

## 16. Crash Recovery

The application must recover from:

- Application crash
- Windows restart
- Network interruption
- Database interruption
- Failed model training
- Failed model serialization

**Recovery rules:**

1. Never partially promote a model.
2. Never overwrite the current production model before candidate validation succeeds.
3. Resume incomplete jobs where possible.
4. Rebuild in-memory state from SQLite when required.
5. Treat model files and metadata as versioned immutable artifacts.
6. Maintain daily snapshots before updates; auto-rollback on degenerate output.

---

## 17. Observability and Diagnostics

The Rust engine provides structured logging. At minimum, record:

```text
application startup/shutdown
daily fetch result
records added/modified
feature-build duration
RAPM duration
Kalman duration
RTS duration
boosting duration
model validation metrics
model promotion
job failures
```

Diagnostic logs must include identifiers, not only prose:

```text
ingestion_run_id=1842
data_version=991
model_version=43
job_id=6201
duration_ms=18422
```

The UI displays the last successful update, current error state, and training progress via the `TrainingStatus` enum.

---

## 18. Security

- Require HTTPS/TLS for external APIs.
- Validate external payloads; enforce response-size limits and timeouts.
- Protect API credentials; avoid storing credentials in source-controlled configuration.
- Validate filesystem paths and numeric input ranges.
- Prevent external data from generating dynamic SQL.
- Bundle MSVC redistributable and code-sign releases.

---

## 19. Testing Strategy

Four levels of testing:

### 19.1 Unit tests

Required for matrix operations, affine transforms, ridge regression, RAPM construction, Kalman predict/update, RTS smoothing, empirical-Bayes calculations, and feature transformations.

### 19.2 Numerical regression tests (Golden-file)

Known datasets must produce known or bounded expected outputs. Snapshot model outputs on fixed synthetic datasets; CI fails if changes to the math core silently shift predictions beyond a tolerance band.

### 19.3 Integration tests

Test: API → normalization → SQLite → feature build → model update → model persistence.

### 19.4 Failure tests

Explicitly test: duplicate API responses, missing fields, malformed data, network timeout, database failure, interrupted training, interrupted model promotion, application restart during daily update.

### 19.5 FFI boundary tests

Round-trip tests confirming `Vec<f64>` ↔ `Float64List` conversions preserve precision and ordering for realistically-sized payloads.

---

## 20. Performance Requirements

Performance is benchmark-driven rather than assumption-based.

Benchmarks must cover:

- Typical daily data volume and maximum expected historical data volume
- Sparse RAPM construction and solve
- Kalman update and RTS smoothing window
- Feature construction
- Boosting update
- Serialization/deserialization
- FFI transfer for realistically-sized vectors
- Flutter chart rendering with downsampling/pagination

No performance claim should rely solely on terms like "zero-copy" or "O(1)" without qualifying dimensions and workload.

---

## 21. Dependency Strategy

Keep the initial Rust dependency set intentionally small.

**Core:**

```text
tokio
reqwest
sqlx + SQLite
flutter_rust_bridge
nalgebra
sprs
statrs
polars
rayon
```

**Gradient boosting:** XGBoost-compatible native backend behind the `IncrementalBooster` trait. Pin the XGBoost version and vendor prebuilt artifacts when possible.

**Other dependencies:** Add only when they provide substantial value. Prefer direct implementations using numerical primitives where they improve control and transparency (e.g., hand-rolled sparse ridge/RAPM instead of `linfa`).

---

## 22. Application Lifecycle

**Startup:**

```text
Flutter startup
    ↓
Initialize Rust bridge
    ↓
Initialize application core
    ↓
Open SQLite / run migrations
    ↓
Load latest model metadata/state
    ↓
Load application settings
    ↓
Check daily-update status (catch-up if needed)
    ↓
Start background scheduler
    ↓
Render dashboard
```

**Shutdown:**

```text
Stop accepting new jobs
    ↓
Finish/cancel safe background work
    ↓
Persist required state
    ↓
Close database
    ↓
Shutdown Rust runtime
```

---

## 23. Implementation Order

### Phase 1 — Native application shell
Flutter Windows + FRB + Rust initialization + SQLite

### Phase 2 — Data foundation
Provider API + normalization + database schema + daily update + idempotency + SQLx migrations + query cache

### Phase 3 — Statistical engine
Ridge → RAPM → Kalman → Empirical-Bayes → Affine → RTS → IncrementalBooster trait + fallback

### Phase 4 — Incremental learning
Daily feature rebuild/update → Kalman update → RAPM update → empirical-Bayes update → gradient boosting continuation/replay → validation → promotion → snapshot/rollback

### Phase 5 — Visualization
Dashboard + player views + team views + model diagnostics + confidence bands + heatmaps + `TrainingStatus` UI

### Phase 6 — Production hardening
Crash recovery + logging + resource limits + golden-file tests + FFI round-trip tests + installer + code signing + clean-machine testing

---

## 24. Explicit Non-Goals for V1

The following are not required:

- WebView-based rendering
- Continuous network polling
- Always-running server process
- Python runtime
- Pandas
- Browser-hosted dashboards
- Cloud database
- Cloud model inference
- Full model retraining after every new game
- Literal one-file `.exe` packaging

The application is intended to operate locally and remain useful without a continuously running cloud service.