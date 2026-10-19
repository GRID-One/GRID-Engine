//! The report record skeletons (engine-output-contract §7). Each report is a versioned record
//! type under `grid.output`. The CLI renders them: P1-10 builds the rendering, and each Phase 2
//! report arrives with its content package (engine-spec §10.2.6). Nothing here renders.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::asof::ResearchFlag;
use crate::error::EngineError;
use crate::ids::{
    EvaluationRunId, IngestionRunId, JobId, ModelFamily, ModelVersionId, PlayerKey, ReviewId,
    Season, Week,
};
use crate::output::projection::{AvailabilitySource, Freshness};
use crate::output::Position;
use crate::status::{JobStatus, StageId, TerminalState, TrainingStatus};
use crate::time::UtcInstant;

/// A reference to a quarantined source row (engine-output-contract §7 data-quality report item
/// 3; plays-contract D-4, DR-B6). Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QuarantinedRowRef {
    /// The ingestion run that quarantined it.
    pub ingestion_run_id: IngestionRunId,
    /// The source dataset.
    pub source: String,
    /// The reason code (the ingestion spec's vocabulary; open).
    pub reason_code: String,
    /// A reference into the retained raw partition (never the row's content).
    pub raw_row_ref: String,
}

/// A source and row-count change diagnostic (engine-output-contract §7 data-quality report).
/// Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceChange {
    /// The source dataset.
    pub source: String,
    /// Rows before the latest fetch.
    pub rows_before: u64,
    /// Rows after the latest fetch.
    pub rows_after: u64,
    /// Whether the provider schema changed.
    pub schema_changed: bool,
}

/// The data-quality report (engine-output-contract §7). Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DataQualityReport {
    /// 1. Stale sources, by name.
    pub stale_sources: Vec<String>,
    /// 2. Count of unresolved identities.
    pub unresolved_identities: u64,
    /// 3. Quarantined rows.
    pub quarantined_rows: Vec<QuarantinedRowRef>,
    /// 4. Missing current-week context, by item name.
    pub missing_current_week_context: Vec<String>,
    /// Source and row-count change diagnostics.
    pub source_changes: Vec<SourceChange>,
}

/// One source's freshness (engine-output-contract §7 status report). Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceFreshness {
    /// The source dataset.
    pub source: String,
    /// Its status under its provider threshold.
    pub status: Freshness,
    /// The last publication timestamp seen; `None` when never fetched successfully.
    pub last_published: Option<UtcInstant>,
    /// The provider's freshness threshold. Units: seconds.
    pub threshold_seconds: u64,
}

/// One job's summary (engine-output-contract §7 status report; engine-spec §8.10). Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobSummary {
    /// The job.
    pub job_id: JobId,
    /// Its stage.
    pub stage: StageId,
    /// Its status.
    pub status: JobStatus,
    /// Its typed error, when `Failed`.
    pub error: Option<EngineError>,
}

/// The status report of `grid status` (engine-output-contract §7; engine-spec §8.6.4). Never
/// null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusReport {
    /// The last successful update; `None` when none has succeeded.
    pub last_successful_update: Option<UtcInstant>,
    /// The terminal state of the latest update.
    pub terminal_state: TerminalState,
    /// The current training status.
    pub training_status: TrainingStatus,
    /// Freshness per source.
    pub sources: Vec<SourceFreshness>,
    /// The production model version per family.
    pub production_models: Vec<(ModelFamily, ModelVersionId)>,
    /// Job statuses and errors.
    pub jobs: Vec<JobSummary>,
}

/// One row of the availability report (engine-output-contract §7). Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AvailabilityReportRow {
    /// The player.
    pub player_id: PlayerKey,
    /// The season.
    pub season: Season,
    /// The week (never 0).
    pub week: Week,
    /// The provider's availability state (`OUT`, `QUESTIONABLE`, …; provider vocabulary, open).
    pub state: String,
    /// Probability active, in `[0, 1]`.
    pub p_active: f64,
    /// Expected snap multiplier if active. Units: ratio.
    pub snap_multiplier_if_active: f64,
    /// Where the inputs came from.
    pub availability_source: AvailabilitySource,
    /// When observed; `None` only when the source is `Missing`.
    pub observed_at: Option<UtcInstant>,
    /// True when a timestamped operator override applied.
    pub manual_override: bool,
}

/// One pending identity link (engine-output-contract §7 identity review queue; engine-spec
/// §4.4.3). Read-only in Phase 1. Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IdentityReviewItem {
    /// The review item.
    pub review_id: ReviewId,
    /// The proposed canonical player; `None` when no candidate reached the review tier.
    pub canonical_candidate: Option<PlayerKey>,
    /// The source the unlinked id comes from.
    pub source_name: String,
    /// The source's player id.
    pub source_player_id: String,
    /// The matching tier reached (1–5 of engine-spec §4.4.3).
    pub tier: u8,
    /// The match confidence, in `[0, 1]`.
    pub confidence: f64,
    /// When the item was opened.
    pub opened_at: UtcInstant,
}

/// The slice of a scorecard row (engine-output-contract §7 model scorecard). **Closed** enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScorecardSlice {
    /// The full player pool.
    All,
    /// The rookie and low-evidence slice.
    RookieLowEvidence,
}

impl fmt::Display for ScorecardSlice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ScorecardSlice::All => "All",
            ScorecardSlice::RookieLowEvidence => "RookieLowEvidence",
        })
    }
}

/// One row of the model scorecard (engine-output-contract §7; engine-spec §7.4). A metric that
/// was not computed for the row is `None` (for example `brier_active` on a slice without
/// availability labels), never `0`. Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScorecardRow {
    /// The season.
    pub season: Season,
    /// The week; `None` for a season-level row.
    pub week: Option<Week>,
    /// The position; `None` for an all-position row.
    pub position: Option<Position>,
    /// The slice.
    pub slice: ScorecardSlice,
    /// PB-MAE, the primary metric (engine-spec §7.4; evaluation-and-leakage.md §4.5). Units:
    /// dimensionless, the mean over the four positions of `NMAE_p = MAE_p / scale_p`, both in
    /// fantasy points of the evaluation's scoring profile.
    pub pb_mae: Option<f64>,
    /// MAE. Units: fantasy points of the evaluation's scoring profile.
    pub mae: Option<f64>,
    /// RMSE. Units: fantasy points of the evaluation's scoring profile.
    pub rmse: Option<f64>,
    /// Rank accuracy. Units: dimensionless, in `[0, 1]`.
    pub rank_accuracy: Option<f64>,
    /// The active/inactive Brier score `mean (p_active − 1{active})²` (evaluation-and-leakage.md
    /// §4.5). Units: dimensionless, in `[0, 1]`.
    pub brier_active: Option<f64>,
    /// Interval coverage. Units: dimensionless, in `[0, 1]`.
    pub interval_coverage: Option<f64>,
}

/// The model scorecard (engine-output-contract §7). Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelScorecard {
    /// The model evaluated.
    pub model_version_id: ModelVersionId,
    /// The evaluation run.
    pub evaluation_run_id: EvaluationRunId,
    /// The rows.
    pub rows: Vec<ScorecardRow>,
    /// The research-only marker of the evaluation run (engine-output-contract §2 `research`;
    /// `evaluation-and-leakage.md` §4.1). `research_only = true`, naming the relaxed rule, for a
    /// `Research`-mode run: such a scorecard MUST NOT feed a gate, a promotion or a published
    /// claim. `research_only = false` with no relaxed rule otherwise. Never null.
    pub research: ResearchFlag,
}

/// A confidence interval with its level (engine-output-contract §7 benchmark results). Units of
/// the bounds: those of the metric the interval belongs to. Never null.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ConfidenceInterval {
    /// The lower bound. Units: those of the metric the interval belongs to.
    pub lower: f64,
    /// The upper bound. Units: those of the metric the interval belongs to.
    pub upper: f64,
    /// The level, in `(0, 1)` (for example `0.95`).
    pub level: f64,
}

/// One aggregate benchmark comparison row (engine-output-contract §7, §8 rule 1). Aggregate
/// metrics only: **no raw competitor row** ever appears. Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkResultRow {
    /// The provider, named or anonymized according to its licence.
    pub provider: String,
    /// The provider snapshot's observation timestamp.
    pub observed_at: UtcInstant,
    /// The metric name (engine-spec §7.4 vocabulary; open).
    pub metric: String,
    /// The metric value. Units: those of `metric` (engine-spec §7.4, §7.5): fantasy points of the
    /// comparison's scoring profile for MAE and RMSE; dimensionless for PB-MAE, rank accuracy,
    /// Brier and coverage.
    pub value: f64,
    /// Its week-clustered bootstrap interval, in the units of `metric`.
    pub interval: ConfidenceInterval,
    /// The number of player-weeks behind the value.
    pub n_player_weeks: u64,
}

/// The benchmark results report (engine-output-contract §7). Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkResults {
    /// The rows.
    pub rows: Vec<BenchmarkResultRow>,
    /// The research-only marker of the evaluation run that produced the rows
    /// (engine-output-contract §2 `research`; `evaluation-and-leakage.md` §4.1).
    /// `research_only = true`, naming the relaxed rule, for a `Research`-mode run: such results
    /// MUST NOT feed a gate, a promotion or a published claim. `research_only = false` with no
    /// relaxed rule otherwise. Never null.
    pub research: ResearchFlag,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::EngineErrorCode;

    #[test]
    fn status_report_carries_typed_status_and_errors() {
        let report = StatusReport {
            last_successful_update: None,
            terminal_state: TerminalState::LearningFailure,
            training_status: TrainingStatus::Failed {
                stage: StageId::RapmUpdate,
                error: EngineErrorCode::IllConditioned,
            },
            sources: vec![SourceFreshness {
                source: "nflverse/pbp".into(),
                status: Freshness::Missing,
                last_published: None,
                threshold_seconds: 86_400,
            }],
            production_models: vec![(ModelFamily("rapm".into()), ModelVersionId(3))],
            jobs: vec![JobSummary {
                job_id: JobId(11),
                stage: StageId::RapmUpdate,
                status: JobStatus::Failed,
                error: Some(EngineError::new(
                    EngineErrorCode::IllConditioned,
                    "condition number above threshold",
                )),
            }],
        };
        assert_eq!(report.terminal_state.exit_code(), Some(40));
        assert_eq!(
            report.jobs[0].error.as_ref().unwrap().code,
            EngineErrorCode::IllConditioned
        );
        assert_eq!(
            ScorecardSlice::RookieLowEvidence.to_string(),
            "RookieLowEvidence"
        );
    }
}
