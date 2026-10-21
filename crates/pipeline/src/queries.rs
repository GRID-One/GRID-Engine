//! The engine queries (engine-spec §8.3): one request type and one response type per query, the
//! windowed response shapes, and the [`EngineQueries`] façade trait.
//!
//! Queries are read-only. They return paginated or windowed, query-specific DTOs; windows or
//! deltas rather than full histories; never a database row or a model internal (engine-spec
//! §5.5 rule 7); and every response carries the data, feature, model, prediction and scoring
//! version identifiers of what it returns, with the stale-data and manual-data flags
//! ([`ResponseVersions`], [`Page`], [`Versioned`]). Read-only queries MUST remain answerable while
//! a backtest or rebuild runs (§8.13).
//!
//! Record types the output contract defines (`PlayerWeekProjection`, `ProjectionChange`,
//! `DataQualityReport`, `IdentityReviewItem`, `ModelScorecard`, `BenchmarkResultRow`,
//! `SourceFreshness`) are reused from `grid-domain` wherever the §8.3 return is one of them. The
//! types defined here are the returns §8.3 names that no contract record carries (the projection
//! board row, the player-rating record, the band row, the model-status view) **and the two
//! status views of queries 6 and 7** ([`IngestionStatus`], [`TrainingStatusView`]), which do not
//! return the contract's §7 Status report (`grid_domain::StatusReport`), deliberately and
//! recorded here: engine-spec §8.3 query 6 requires "per-source results", which the Status
//! report row of engine-output-contract §7 (last successful ingestion per source, source
//! freshness, production model, job status and errors) does not carry, and §8.3 query 7 names
//! only the current `TrainingStatus` and the running jobs. The spec outranks the contract
//! (engine-spec §1.5), so the views follow §8.3. That the contract's `StatusReport` is returned
//! by no query, and that its §7 row lacks the per-source results §8.3 query 6 requires, is a
//! contract gap recorded in the P1-01 report as a decision request for the owner, not resolved
//! here: `StatusReport` stays in `grid-domain` as the contract fixes it.
//!
//! **No query has an implementation in P1-01.** [`EngineQueries`] has no implementor in this
//! workspace; the packages that own each stage provide one from P1-02 on (the GRID diagnostic
//! queries 12 and 13 are produced by P1-12). P1-10 builds the CLI subcommand of every Phase 1
//! query (engine-spec §8.1 target table; §8.2 "completed in P1-10"); each Phase 2 output, its
//! subcommand included, is delivered by the package §10.2.6 names as the owner of its content
//! (P2-01, P2-06, P2-07), as the owner ruled on 2026-10-19 (P0-03; P1-01 Amendments row 22).
//! `grid-cli` records the delivering package per subcommand.
//!
//! **Deviations from the indicative §8.3 signatures**, each deliberate and recorded here: query 2
//! adds `horizon`, and queries 8, 9, 11 and 14 take a page or filters where the table writes
//! `()`; the reason is on each query type.

use std::fmt;

use grid_domain::{
    BenchmarkResultRow, ConfidenceInterval, DataQualityReport, DataVersion, EngineError,
    EvaluationRunId, FantasyDistribution, FeatureSchemaVersion, FreshnessFlags, Horizon,
    IdentityReviewItem, IngestionRunId, JobSummary, ModelFamily, ModelScorecard, ModelVersionId,
    OffseasonRapm, PlayerKey, PlayerWeekProjection, Position, PredictionVersionId, PriorBlock,
    ProjectionChange, ProjectionKey, RegimeFlags, ResearchFlag, ScoringProfileRef, Season, Slot,
    SourceFreshness, StatComponent, StatDistribution, TeamKey, TerminalState, TrainingRunId,
    TrainingStatus, UtcInstant, Week,
};
use serde::{Deserialize, Serialize};

/// The §8.3 library function names, in table order (queries 1–14). The CLI pairs each of its
/// subcommand spellings with one of these, and its tests check that pairing against this list.
pub const QUERY_NAMES: [&str; 14] = [
    "get_week_projections",
    "get_player_projection_detail",
    "get_projection_distribution",
    "get_projection_change_log",
    "get_data_freshness",
    "get_ingestion_status",
    "get_training_status",
    "get_identity_review_queue",
    "get_model_scorecard",
    "get_benchmark_results",
    "get_data_quality_report",
    "get_player_ratings",
    "get_confidence_bands",
    "get_model_status",
];

// ---------------------------------------------------------------------------------------------
// Response shapes
// ---------------------------------------------------------------------------------------------

/// A page request for a windowed query (engine-spec §8.3 "paginated or windowed"; §12.6
/// "large-result query pagination"). Never null.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PageRequest {
    /// Items to skip, in the query's documented order.
    pub offset: u64,
    /// Maximum items to return. An implementor caps it at its documented maximum and reports the
    /// cap in the response's `items.len()`; it never fails the query for a large limit.
    pub limit: u32,
}

/// The version identifiers every query response carries (engine-spec §8.3 fourth rule). Each
/// field is `None` when the response is not derived from a thing of that kind, with the reason on
/// the field; it is never a zero id. A response derived from a prediction version carries all
/// five. Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResponseVersions {
    /// The committed data version the response was read from. `None` only before the first data
    /// version is committed (a status query on an empty engine).
    pub data_version: Option<DataVersion>,
    /// The feature schema behind the response. `None` for a response that reads no features (the
    /// status, freshness, data-quality, identity and model-status queries).
    pub feature_schema_version: Option<FeatureSchemaVersion>,
    /// The production model version behind the response. `None` for a response that reads no
    /// model output (the status, freshness, data-quality and identity queries).
    pub model_version_id: Option<ModelVersionId>,
    /// The prediction version behind the response. `None` for a response that is not derived from
    /// a prediction version (every query but 1–4). Queries 12 and 13 read the state-space model
    /// as of a slot, not a prediction version: the band is the state-space predictive variance
    /// (engine-spec §8.3 query 13; §5.5 "For GRID state, bands use the one-step predictive
    /// variance"), so neither carries one.
    pub prediction_version_id: Option<PredictionVersionId>,
    /// The scoring profile the response's fantasy points are in. `None` when the response carries
    /// no fantasy points.
    pub scoring_profile: Option<ScoringProfileRef>,
}

/// One page of a windowed response (engine-spec §8.3). Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page<T> {
    /// The items of this page, in the query's documented order.
    pub items: Vec<T>,
    /// The offset of `items[0]` in the full result.
    pub offset: u64,
    /// The total number of items in the full result, when the implementor can count it without a
    /// full scan; `None` otherwise (the caller pages until a short page).
    pub total: Option<u64>,
    /// The version identifiers of what the page returns.
    pub versions: ResponseVersions,
    /// The stale-data flag: true when any critical source behind the page is `Stale` or
    /// `Missing` (engine-spec §5.5 "Freshness"; §10.3).
    pub stale_data: bool,
    /// The manual-data flag: true when a timestamped manual override applied to any item
    /// (engine-spec §10.3).
    pub manual_data: bool,
}

/// A single (non-paginated) response with the same version and flag obligations as a [`Page`].
/// Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Versioned<T> {
    /// The response body.
    pub body: T,
    /// The version identifiers of what the body returns.
    pub versions: ResponseVersions,
    /// The stale-data flag (see [`Page::stale_data`]).
    pub stale_data: bool,
    /// The manual-data flag (see [`Page::manual_data`]).
    pub manual_data: bool,
}

// ---------------------------------------------------------------------------------------------
// Query 1 — week projections
// ---------------------------------------------------------------------------------------------

/// Query 1: `get_week_projections(query)` — `grid query projections`. Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WeekProjectionsQuery {
    /// The target season.
    pub season: Season,
    /// The target week (never 0).
    pub week: Week,
    /// The scoring profile the points are in.
    pub scoring_profile: ScoringProfileRef,
    /// Restrict to one position; `None` returns every projected position.
    pub position: Option<Position>,
    /// The prediction version to read; `None` reads the production version in force for the
    /// slot.
    pub prediction_version_id: Option<PredictionVersionId>,
    /// The page, over rows ordered by `mean_points` descending, then `player_id`.
    pub page: PageRequest,
}

/// One row of the weekly projection table (engine-spec §8.3 query 1: player, team, opponent,
/// position, mean and median points, P10/P90, active probability, role/snap projection,
/// data-freshness flag). Every value is read from the player-week record of the same prediction
/// version; nothing is recomputed. Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectionBoardRow {
    /// The key group: player, position, team, opponent, game and horizon.
    pub key: ProjectionKey,
    /// The scoring profile of the points below.
    pub scoring_profile: ScoringProfileRef,
    /// The unconditional (headline) mean, in fantasy points of `scoring_profile`.
    pub mean_points: f64,
    /// The unconditional median, in fantasy points.
    pub median_points: f64,
    /// The unconditional 10th percentile, in fantasy points.
    pub p10_points: f64,
    /// The unconditional 90th percentile, in fantasy points.
    pub p90_points: f64,
    /// Probability active, in `[0, 1]` (the availability block's `p_active`).
    pub p_active: f64,
    /// Expected snap multiplier if active. Units: ratio.
    pub snap_multiplier_if_active: f64,
    /// The conditional offensive snap-share summary. `None` for a QB, whose target vector has no
    /// snap-share component (engine-output-contract §3.2; DR-D7), never a zero summary.
    pub offensive_snap_share: Option<StatDistribution>,
    /// The record's freshness and provenance flags.
    pub flags: FreshnessFlags,
}

// ---------------------------------------------------------------------------------------------
// Query 2 — player projection detail
// ---------------------------------------------------------------------------------------------

/// Query 2: `get_player_projection_detail(player_id, season, week, scoring_profile)` —
/// `grid query player`. Never null.
///
/// `horizon` is added to the §8.3 signature, deliberately: a player-week record is keyed by its
/// typed horizon (engine-output-contract §2 `ProjectionKey`; DR-C4), so `(player_id, season,
/// week)` alone names up to three records and the query must say which.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerProjectionDetailQuery {
    /// The player.
    pub player_id: PlayerKey,
    /// The target season.
    pub season: Season,
    /// The target week (never 0).
    pub week: Week,
    /// The typed horizon (never a `week = 0` sentinel: KI-A4).
    pub horizon: Horizon,
    /// The scoring profile the headline points are in.
    pub scoring_profile: ScoringProfileRef,
}

/// One completed game-week of observed usage (engine-spec §8.3 query 2 "recent usage"). The
/// observed values are in the natural units of each component (engine-output-contract §3.2) and
/// are **outcomes**, so a week is listed only when it is outcome-usable at the as-of point of the
/// projection (`AsOf::outcome_usable`). Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageWeek {
    /// The game-week observed.
    pub slot: Slot,
    /// When the stat line was published; gates its presence in the list.
    pub published_at: UtcInstant,
    /// The observed value per component present in the player's target vector. A component with
    /// no observation (participation unavailable for the snap share) is absent, never `0`.
    pub observed: Vec<(StatComponent, f64)>,
}

/// The response of query 2: the full projection record (component stat line, NCAA prior
/// contribution, distribution summary and top drivers are its groups: engine-output-contract
/// §3–§4) plus the recent-usage window the record does not carry. Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerProjectionDetail {
    /// The player-week projection record.
    pub projection: PlayerWeekProjection,
    /// Recent usage, most recent first. The window length is the implementor's documented
    /// setting; empty when no outcome-usable week exists (a rookie before week 1).
    pub recent_usage: Vec<UsageWeek>,
}

// ---------------------------------------------------------------------------------------------
// Query 3 — projection distribution
// ---------------------------------------------------------------------------------------------

/// Query 3: `get_projection_distribution(...)` — `grid query distribution`. Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectionDistributionQuery {
    /// The player.
    pub player_id: PlayerKey,
    /// The target season.
    pub season: Season,
    /// The target week (never 0).
    pub week: Week,
    /// The typed horizon.
    pub horizon: Horizon,
    /// The scoring profile whose distribution is returned.
    pub scoring_profile: ScoringProfileRef,
}

/// The response of query 3: the §5.3 distribution outputs for one profile, read from the record's
/// `fantasy` list (engine-output-contract §3.6). The body is the contract's
/// [`FantasyDistribution`].
pub type ProjectionDistributionResponse = Versioned<FantasyDistribution>;

// ---------------------------------------------------------------------------------------------
// Query 4 — projection change log
// ---------------------------------------------------------------------------------------------

/// Query 4: `get_projection_change_log(...)` — `grid report changes`. Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectionChangeLogQuery {
    /// The target season.
    pub season: Season,
    /// The target week (never 0).
    pub week: Week,
    /// The scoring profile the deltas are in.
    pub scoring_profile: ScoringProfileRef,
    /// Restrict to one player; `None` returns every player with a change.
    pub player_id: Option<PlayerKey>,
    /// The page, over rows ordered by `|delta|` descending, then `player_id`.
    pub page: PageRequest,
}

// ---------------------------------------------------------------------------------------------
// Queries 5–7 — freshness, ingestion status, training status
// ---------------------------------------------------------------------------------------------

/// Query 5: `get_data_freshness()` — `grid status --freshness`. No parameters. Never null.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DataFreshnessQuery {}

/// Query 6: `get_ingestion_status()` — `grid status`. No parameters. Never null.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IngestionStatusQuery {}

/// Query 7: `get_training_status()` — `grid status`. No parameters. Never null.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrainingStatusQuery {}

/// One source's result in the latest daily update (engine-spec §8.3 query 6 "per-source
/// results"; §8.6.4). Carries the identifiers the §8.4 fetch events carry. Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceFetchResult {
    /// The source dataset, for example `nflverse/pbp`.
    pub source: String,
    /// The terminal state the fetch reached; `NotRun` when the update skipped it under the
    /// once-per-day cap, `Running` while it is in flight.
    pub terminal_state: TerminalState,
    /// The ingestion run; `None` when the fetch was not run.
    pub ingestion_run_id: Option<IngestionRunId>,
    /// When the fetch reached its terminal state; `None` when it has not.
    pub finished_at: Option<UtcInstant>,
    /// The typed error of a failed fetch; `None` otherwise.
    pub error: Option<EngineError>,
}

/// The response body of query 6 (engine-spec §8.3: last successful update, current terminal
/// state, per-source results). Not the contract's §7 `StatusReport`, which carries no per-source
/// results; see the module documentation. Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IngestionStatus {
    /// The last successful daily update; `None` when none has succeeded.
    pub last_successful_update: Option<UtcInstant>,
    /// The terminal state of the latest daily update (engine-spec §8.6.4).
    pub terminal_state: TerminalState,
    /// The per-source results of the latest daily update.
    pub sources: Vec<SourceFetchResult>,
}

/// The response body of query 7 (engine-spec §8.3: the current `TrainingStatus` and running
/// jobs, and nothing else). Not the contract's §7 `StatusReport`; see the module documentation.
/// Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainingStatusView {
    /// The current training status (engine-spec §8.4).
    pub status: TrainingStatus,
    /// The jobs currently `Running` (engine-spec §8.10).
    pub running_jobs: Vec<JobSummary>,
}

// ---------------------------------------------------------------------------------------------
// Query 8 — identity review queue
// ---------------------------------------------------------------------------------------------

/// Query 8: `get_identity_review_queue()` — `grid identity queue`. Never null.
///
/// `page` extends the parameterless §8.3 signature, deliberately: §8.3 requires every query to
/// "return paginated or windowed, query-specific DTOs", and an open queue has no other bound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IdentityReviewQueueQuery {
    /// The page, over open items ordered by `opened_at` ascending, then `review_id`.
    pub page: PageRequest,
}

// ---------------------------------------------------------------------------------------------
// Query 9 — model scorecard
// ---------------------------------------------------------------------------------------------

/// Query 9: `get_model_scorecard()` — `grid report scorecard`. The window is the filter: a
/// scorecard has at most weeks × positions × slices rows for one evaluation run. Never null.
///
/// The filters extend the parameterless §8.3 signature, deliberately: §8.3 requires "windows
/// or deltas rather than full histories", and one evaluation run of one model version is the
/// window a scorecard is bounded to.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelScorecardQuery {
    /// The model version; `None` reads the production version of the projection ensemble.
    pub model_version_id: Option<ModelVersionId>,
    /// The evaluation run; `None` reads the latest completed run for the model version.
    pub evaluation_run_id: Option<EvaluationRunId>,
    /// Restrict to one season; `None` returns every season of the run.
    pub season: Option<Season>,
    /// Restrict to one position; `None` returns every position and the all-position rows.
    pub position: Option<Position>,
}

// ---------------------------------------------------------------------------------------------
// Query 10 — benchmark results
// ---------------------------------------------------------------------------------------------

/// Query 10: `get_benchmark_results(query)` — `grid report benchmark`. Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BenchmarkResultsQuery {
    /// Restrict to one provider (named or anonymized as its licence requires); `None` returns
    /// every provider.
    pub provider: Option<String>,
    /// Restrict to one season; `None` returns every season.
    pub season: Option<Season>,
    /// The page, over rows ordered by `provider`, `observed_at`, `metric`.
    pub page: PageRequest,
}

// ---------------------------------------------------------------------------------------------
// Query 11 — data quality
// ---------------------------------------------------------------------------------------------

/// Query 11: `get_data_quality_report()` — `grid report data-quality`. Never null.
///
/// The window is one ingestion run: the report describes one daily update's quarantine set,
/// source changes and context gaps, never the quarantine history (engine-spec §8.3 "windows
/// rather than full histories"; §5.5 rule 7). `ingestion_run_id` extends the parameterless §8.3
/// signature, deliberately, to make that window explicit.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DataQualityQuery {
    /// The ingestion run whose report is read; `None` reads the latest completed daily update.
    pub ingestion_run_id: Option<IngestionRunId>,
}

// ---------------------------------------------------------------------------------------------
// Query 12 — player ratings (the GRID signal, engine-spec §6.2, §5.6 "GRID diagnostic outputs")
// ---------------------------------------------------------------------------------------------

/// Query 12: `get_player_ratings(query)` — `grid query ratings`. Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerRatingsQuery {
    /// The game-week the ratings are read as of.
    pub season: Season,
    /// The week (never 0).
    pub week: Week,
    /// Restrict to one position; `None` returns every projected position.
    pub position: Option<Position>,
    /// Restrict to one player; `None` returns every player with a track.
    pub player_id: Option<PlayerKey>,
    /// The page, over rows ordered by `player_id` (the string-form order of `PlayerKey`).
    pub page: PageRequest,
}

/// A role of the state-space talent tracks (engine-output-contract §4.2 `role_talent`; DR-C3).
/// **Closed** enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TalentRole {
    /// The dropback (passing) role.
    Dropback,
    /// The carry (rushing) role.
    Carry,
    /// The target (receiving) role. Whether this credit is an individual signal is open
    /// (DR-D13).
    Target,
}

impl fmt::Display for TalentRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            TalentRole::Dropback => "dropback",
            TalentRole::Carry => "carry",
            TalentRole::Target => "target",
        })
    }
}

/// One estimate of a latent state component: a mean and a variance. Units: `dV` (expected drive
/// points per play) for the mean, `dV²` for the variance. Never null when present.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StateEstimate {
    /// The estimate's mean.
    pub mean: f64,
    /// The estimate's variance, `≥ 0`.
    pub var: f64,
}

/// The three estimate kinds of one latent component, **separately named and never aliased**
/// (engine-spec §5.5 "Distinct estimate kinds"; §8.3 query 12 "filtered and smoothed … with
/// predictive variance"). Never null when present.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LatentTrack {
    /// The **filtered** (real-time) estimate: conditioned on game-weeks before the as-of slot.
    pub filtered: StateEstimate,
    /// The **one-step predictive** estimate: the filtered state advanced one step, whose variance
    /// is the confidence-band quantity `S = H·P_pred·Hᵀ + R` (engine-spec §5.5 "Confidence bands
    /// are data").
    pub predictive: StateEstimate,
    /// The **RTS-smoothed** estimate, a **retrospective diagnostic**: conditioned on the whole
    /// completed window, including game-weeks after the slot. It MUST NOT feed a projection, a
    /// gate or a published claim, and it never appears in a projection record
    /// (`GridSignalBlock` has no smoothed field by design). `None` when no smoothing pass covers
    /// the slot (the current week, or a window not yet smoothed).
    pub smoothed_retrospective: Option<StateEstimate>,
}

/// The talent track per role (engine-output-contract §4.2; DR-C3). A role the player does not
/// fill is `None` (no track exists), never a zero track. Never null.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RoleTracks {
    /// The dropback (passing) role.
    pub dropback: Option<LatentTrack>,
    /// The carry (rushing) role.
    pub carry: Option<LatentTrack>,
    /// The target (receiving) role.
    pub target: Option<LatentTrack>,
}

/// The sign convention of a matchup grade, carried as data in every output that holds one so
/// that a consumer of the record or of its generated schema never has to infer it (engine-spec
/// §5.6 "Team strength and matchup": "the matchup grade (`E_def`, higher = tougher) with the sign
/// convention stated in the output (DR-B5)"; the oracle's inverted sign, KI-NEW-A2, is the hazard
/// the rule exists for). Serialized as `higher_is_tougher`. **Closed** enum with one value: a
/// second convention would be a changed semantic, a MAJOR change (engine-spec §5.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchupSign {
    /// The grade is `+E_def`: a higher value is a tougher defense to face (DR-B5).
    HigherIsTougher,
}

impl fmt::Display for MatchupSign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            MatchupSign::HigherIsTougher => "higher_is_tougher",
        })
    }
}

/// A matchup grade with its sign convention (engine-spec §5.6; §6.2 Component 5; DR-B5). Never
/// null when present.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MatchupGrade {
    /// The opponent's defensive effect `+E_def`, in `dV` (expected drive points per play);
    /// never the oracle's `−β[t_def]` (KI-NEW-A2).
    pub e_def: f64,
    /// The sign convention of `e_def`: [`MatchupSign::HigherIsTougher`].
    pub convention: MatchupSign,
}

/// One player's GRID signal as of a game-week (engine-spec §8.3 query 12; §5.6 "GRID diagnostic
/// outputs"; §6.2). Every quantity is in `dV` currency and is never mixed with fantasy points.
/// Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerRatingRecord {
    /// The player.
    pub player_id: PlayerKey,
    /// The game-week the record is read as of.
    pub slot: Slot,
    /// The player's position.
    pub position: Position,
    /// The player's team at the slot.
    pub team: TeamKey,
    /// The offseason RAPM rating with its season and exposure; `None` until a season's
    /// participation is published before the lock (DR-C1).
    pub offseason_rapm: Option<OffseasonRapm>,
    /// The weekly Layer-1′ credit for the slot's completed game, in `dV` summed over the player's
    /// plays; `None` when the game-week is not complete at the as-of point or participation is
    /// unavailable.
    pub weekly_credit: Option<f64>,
    /// The talent tracks per role, each with its three estimate kinds.
    pub role_talent: RoleTracks,
    /// The form state component, with its three estimate kinds; `None` when no track exists yet.
    pub form: Option<LatentTrack>,
    /// The scheme-fit state component, with its three estimate kinds; `None` when no track
    /// exists yet.
    pub scheme_fit: Option<LatentTrack>,
    /// The season-boundary prior behind the tracks (mean, variance, `q`, `n0`, current weight);
    /// `None` when the player has none.
    pub prior: Option<PriorBlock>,
    /// The player's team net strength `E_off + E_def` (engine-spec §6.2 Component 4; DR-B5), in
    /// `dV`; higher = stronger; `None` when no team-level fit covers the slot.
    pub team_net_strength: Option<f64>,
    /// The matchup grade of the slot's opponent, with its sign convention carried as data
    /// ([`MatchupGrade`]; engine-spec §5.6 "with the sign convention stated in the output",
    /// DR-B5); `None` when no opponent rating exists for the slot.
    pub matchup_grade: Option<MatchupGrade>,
    /// The regime flags of the state-space layer at the slot.
    pub regime_flags: RegimeFlags,
    /// The row's freshness and provenance flags (engine-output-contract §3.8): the worst status
    /// across the sources that fed the row. Every row of every output carries them (engine-spec
    /// §5.6 "Freshness flags"; §5.5 "present on every row"); the page's `stale_data` and
    /// `manual_data` are the roll-up over its rows.
    pub flags: FreshnessFlags,
}

// ---------------------------------------------------------------------------------------------
// Query 13 — confidence bands
// ---------------------------------------------------------------------------------------------

/// Query 13: `get_confidence_bands(query)` — `grid query bands`. Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfidenceBandsQuery {
    /// The player.
    pub player_id: PlayerKey,
    /// The first game-week of the window, inclusive.
    pub from: Slot,
    /// The last game-week of the window, inclusive. Must not precede `from`; an implementor
    /// rejects an inverted window as a contract violation.
    pub to: Slot,
    /// The interval level, in `(0, 1)` (for example `0.8`).
    pub level: f64,
    /// The page, over rows ordered by `slot`, then `role`.
    pub page: PageRequest,
}

/// One band row: the one-step predictive band of one role's talent at one game-week
/// (engine-spec §8.3 query 13; §5.5 "Confidence bands are data"). Band **data** only; rendering
/// is out of scope. Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfidenceBandRow {
    /// The player.
    pub player_id: PlayerKey,
    /// The game-week.
    pub slot: Slot,
    /// The role of the talent track.
    pub role: TalentRole,
    /// The one-step predictive mean, in `dV`.
    pub predictive_mean: f64,
    /// The **one-step predictive variance** `S = H·P_pred·Hᵀ + R`, in `dV²`. The band is built
    /// from this variance only, **never from the filtered variance** (engine-spec §8.3 query 13;
    /// §5.5). An ex-ante band conditioned on projected exposure waits for Layer C (open —
    /// DR-D16).
    pub predictive_var: f64,
    /// The band at the requested level: `lower`, `upper` in `dV`, `level` in `(0, 1)`.
    pub interval: ConfidenceInterval,
    /// The row's freshness and provenance flags (engine-output-contract §3.8): the worst status
    /// across the sources that fed the row (engine-spec §5.6 "Freshness flags"; §5.5 "present
    /// on every row"); the page's `stale_data` and `manual_data` are the roll-up over its rows.
    pub flags: FreshnessFlags,
}

// ---------------------------------------------------------------------------------------------
// Query 14 — model status
// ---------------------------------------------------------------------------------------------

/// Query 14: `get_model_status()` — `grid model status`. Never null.
///
/// `model_family` extends the parameterless §8.3 signature, deliberately: it is the window
/// filter §8.3 requires ("paginated or windowed"), over a response that is otherwise every
/// family's version window.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelStatusQuery {
    /// Restrict to one model family; `None` returns every family.
    pub model_family: Option<ModelFamily>,
}

/// The state of a model version (engine-spec §8.8 "States"). Exactly one per version; the
/// permitted transitions are those §8.8 lists and no other. Serialized as the spec's
/// `SCREAMING_SNAKE_CASE` names. **Closed** enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModelState {
    /// Training in progress.
    Training,
    /// A candidate under time-aware validation.
    Validating,
    /// Trained, awaiting validation.
    Candidate,
    /// Named by a production pointer.
    Production,
    /// Failed validation, or training failed or was aborted. Never returns to `Production`.
    Rejected,
    /// Formerly production; a later version was promoted or a rollback moved the pointer.
    Superseded,
}

impl ModelState {
    /// Every state, in the order engine-spec §8.8 lists them.
    pub const ALL: [ModelState; 6] = [
        ModelState::Training,
        ModelState::Validating,
        ModelState::Candidate,
        ModelState::Production,
        ModelState::Rejected,
        ModelState::Superseded,
    ];

    /// The `SCREAMING_SNAKE_CASE` name engine-spec §8.8 writes.
    pub const fn spec_name(self) -> &'static str {
        match self {
            ModelState::Training => "TRAINING",
            ModelState::Validating => "VALIDATING",
            ModelState::Candidate => "CANDIDATE",
            ModelState::Production => "PRODUCTION",
            ModelState::Rejected => "REJECTED",
            ModelState::Superseded => "SUPERSEDED",
        }
    }
}

impl fmt::Display for ModelState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.spec_name())
    }
}

/// A reference to the validation report of one model version (engine-spec §8.3 query 14 "last
/// validation report"; §8.8 "Validation metrics must be persisted with each model"). The report's
/// rows are read through [`EngineQueries::get_model_scorecard`]. Never null when present.
///
/// The report is a run-level evaluation output, so the reference carries the run's research-only
/// marker (`evaluation-and-leakage.md` §4.1; engine-output-contract §2 `research`): a reader that
/// gates or promotes on "the last validation report" sees from the reference alone that a
/// `Research`-mode report MUST NOT feed it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ValidationReportRef {
    /// The evaluation run that produced the report.
    pub evaluation_run_id: EvaluationRunId,
    /// The model version validated.
    pub model_version_id: ModelVersionId,
    /// When the report was completed.
    pub completed_at: UtcInstant,
    /// The research-only marker of the evaluation run: the same flag as the referenced
    /// [`ModelScorecard`]'s `research`. Never null.
    pub research: ResearchFlag,
}

/// One model version's summary (engine-spec §8.3 query 14). Never null.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelVersionSummary {
    /// The version.
    pub model_version_id: ModelVersionId,
    /// Its state (engine-spec §8.8).
    pub state: ModelState,
    /// The training run that produced it; `None` for a version restored from an imported
    /// snapshot whose run is not in this ledger.
    pub training_run_id: Option<TrainingRunId>,
    /// When the version was created.
    pub created_at: UtcInstant,
}

/// One model family's status (engine-spec §8.3 query 14: versions, states, production pointer,
/// last validation report). Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelFamilyStatus {
    /// The family.
    pub family: ModelFamily,
    /// The version the production pointer names; `None` when the family has never had a
    /// production version.
    pub production: Option<ModelVersionId>,
    /// The versions in the window, newest first. The window length is the implementor's
    /// documented setting (a full history is never returned: engine-spec §8.3).
    pub versions: Vec<ModelVersionSummary>,
    /// The last validation report; `None` when no version of the family has been validated.
    pub last_validation: Option<ValidationReportRef>,
}

/// The response body of query 14. Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelStatus {
    /// One entry per family, ordered by family name.
    pub families: Vec<ModelFamilyStatus>,
}

// ---------------------------------------------------------------------------------------------
// The façade trait
// ---------------------------------------------------------------------------------------------

/// The engine's query façade (engine-spec §8.3). One method per query, in table order.
///
/// Every method takes `&self`, is read-only, synchronous (see the crate documentation) and returns
/// a windowed or single response that carries the version identifiers and the stale-data and
/// manual-data flags, or a typed [`EngineError`]. No method returns a fabricated response.
///
/// **No implementor exists in P1-01.** Implementors arrive with the packages that own each query's
/// data (P1-02 onward; P1-12 for queries 12 and 13).
pub trait EngineQueries {
    /// Query 1. The weekly projection table, one row per player-week record of the prediction
    /// version in force.
    fn get_week_projections(
        &self,
        query: WeekProjectionsQuery,
    ) -> Result<Page<ProjectionBoardRow>, EngineError>;

    /// Query 2. The full projection record of one player-week plus its recent usage (§6.7).
    fn get_player_projection_detail(
        &self,
        query: PlayerProjectionDetailQuery,
    ) -> Result<Versioned<PlayerProjectionDetail>, EngineError>;

    /// Query 3. The §5.3 distribution outputs of one player-week under one scoring profile.
    fn get_projection_distribution(
        &self,
        query: ProjectionDistributionQuery,
    ) -> Result<ProjectionDistributionResponse, EngineError>;

    /// Query 4. Prior versus current projection with the five-category change attribution
    /// (engine-output-contract §6).
    fn get_projection_change_log(
        &self,
        query: ProjectionChangeLogQuery,
    ) -> Result<Page<ProjectionChange>, EngineError>;

    /// Query 5. Per-source freshness against the source-specific thresholds.
    fn get_data_freshness(
        &self,
        query: DataFreshnessQuery,
    ) -> Result<Versioned<Vec<SourceFreshness>>, EngineError>;

    /// Query 6. The last successful update, the current terminal state (§8.6.4) and the
    /// per-source results.
    fn get_ingestion_status(
        &self,
        query: IngestionStatusQuery,
    ) -> Result<Versioned<IngestionStatus>, EngineError>;

    /// Query 7. The current `TrainingStatus` (§8.4) and the running jobs.
    fn get_training_status(
        &self,
        query: TrainingStatusQuery,
    ) -> Result<Versioned<TrainingStatusView>, EngineError>;

    /// Query 8. The open identity review items. Read-only in Phase 1; resolved by commands 6 and
    /// 7.
    fn get_identity_review_queue(
        &self,
        query: IdentityReviewQueueQuery,
    ) -> Result<Page<IdentityReviewItem>, EngineError>;

    /// Query 9. PB-MAE, MAE, RMSE, rank accuracy, Brier score and coverage by week and position,
    /// with the rookie/low-evidence slice (engine-output-contract §7).
    fn get_model_scorecard(
        &self,
        query: ModelScorecardQuery,
    ) -> Result<Versioned<ModelScorecard>, EngineError>;

    /// Query 10. Provider comparison with timestamps and confidence intervals. Aggregate rows
    /// only: no raw competitor row (§7.6).
    fn get_benchmark_results(
        &self,
        query: BenchmarkResultsQuery,
    ) -> Result<Page<BenchmarkResultRow>, EngineError>;

    /// Query 11. Stale sources, unresolved identities, quarantined rows, missing current-week
    /// context, source and row-count changes (§8.12), for one ingestion run: the one the query
    /// names, or the latest completed daily update.
    fn get_data_quality_report(
        &self,
        query: DataQualityQuery,
    ) -> Result<Versioned<DataQualityReport>, EngineError>;

    /// Query 12. The GRID signal per §6.2: offseason RAPM, weekly Layer-1′ credit, the filtered,
    /// one-step predictive and retrospective-smoothed talent, form and scheme-fit estimates, team
    /// net strength and the matchup grade (higher = tougher, DR-B5).
    fn get_player_ratings(
        &self,
        query: PlayerRatingsQuery,
    ) -> Result<Page<PlayerRatingRecord>, EngineError>;

    /// Query 13. Band data from the one-step predictive variance only, never the filtered
    /// variance. Rendering is out of scope.
    fn get_confidence_bands(
        &self,
        query: ConfidenceBandsQuery,
    ) -> Result<Page<ConfidenceBandRow>, EngineError>;

    /// Query 14. Model versions per family, their §8.8 states, the production pointers and the
    /// last validation report.
    fn get_model_status(
        &self,
        query: ModelStatusQuery,
    ) -> Result<Versioned<ModelStatus>, EngineError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_names_are_distinct_and_fourteen() {
        let mut names = QUERY_NAMES.to_vec();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 14);
    }

    #[test]
    fn model_state_serializes_as_the_spec_names() {
        for state in ModelState::ALL {
            let json = serde_json::to_string(&state).unwrap();
            assert_eq!(json, format!("\"{}\"", state.spec_name()));
            let back: ModelState = serde_json::from_str(&json).unwrap();
            assert_eq!(back, state);
        }
        assert_eq!(ModelState::Superseded.to_string(), "SUPERSEDED");
        assert_eq!(TalentRole::Dropback.to_string(), "dropback");
    }

    #[test]
    fn matchup_grade_carries_its_sign_convention_as_data() {
        let grade = MatchupGrade {
            e_def: 0.015625,
            convention: MatchupSign::HigherIsTougher,
        };
        let json = serde_json::to_string(&grade).unwrap();
        assert_eq!(
            json,
            "{\"e_def\":0.015625,\"convention\":\"higher_is_tougher\"}"
        );
        let back: MatchupGrade = serde_json::from_str(&json).unwrap();
        assert_eq!(back, grade);
        assert_eq!(
            MatchupSign::HigherIsTougher.to_string(),
            "higher_is_tougher"
        );
        // A record without the convention does not deserialize: the convention is not optional.
        assert!(serde_json::from_str::<MatchupGrade>("{\"e_def\":0.015625}").is_err());
    }

    #[test]
    fn page_and_versioned_round_trip() {
        let page = Page {
            items: vec![1u32, 2, 3],
            offset: 10,
            total: None,
            versions: ResponseVersions {
                data_version: Some(DataVersion(4)),
                feature_schema_version: None,
                model_version_id: None,
                prediction_version_id: None,
                scoring_profile: None,
            },
            stale_data: true,
            manual_data: false,
        };
        let json = serde_json::to_string(&page).unwrap();
        let back: Page<u32> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, page);
        let single = Versioned {
            body: "x".to_owned(),
            versions: page.versions.clone(),
            stale_data: false,
            manual_data: true,
        };
        let back: Versioned<String> =
            serde_json::from_str(&serde_json::to_string(&single).unwrap()).unwrap();
        assert_eq!(back, single);
    }
}
