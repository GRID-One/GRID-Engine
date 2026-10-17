//! Terminal states, exit codes, job and training status (engine-spec §8.4, §8.6.4, §8.10;
//! engine-output-contract §13).

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::EngineErrorCode;

/// The process exit code of a usage error: an unknown subcommand, a malformed argument
/// (engine-output-contract §13; P1-01 CLI skeleton).
pub const EXIT_USAGE: i32 = 2;

/// The process exit code of an internal error that no [`TerminalState`] describes
/// (engine-output-contract §13).
pub const EXIT_INTERNAL: i32 = 70;

/// The terminal state of `grid update` and every daily-update run (engine-spec §8.6.4, kept as a
/// binding enum from the superseded final-build-spec §9.5). Fourteen values. **Closed** enum.
///
/// `NotRun` and `Running` are not terminal; the twelve others each map to a distinct, documented
/// process exit code so that an external scheduler can alert on it ([`TerminalState::exit_code`]).
/// A failure MUST NOT corrupt or partially promote the previous production model, and the oracle's
/// swallow-and-continue paths (`weekly_update.run`, unconditional `"ok"` health: KI-NEW-W5) are
/// not ported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TerminalState {
    /// The update has not run.
    NotRun,
    /// The update is in progress.
    Running,
    /// Every stage succeeded.
    Success,
    /// Every source reported no new data; nothing was recomputed.
    NoNewData,
    /// Rows were quarantined and the valid subset was committed.
    PartialSuccess,
    /// A source could not be reached.
    NetworkFailure,
    /// A fetch or a stage exceeded its time budget.
    Timeout,
    /// A source rate-limited the engine.
    RateLimited,
    /// A source rejected the engine's credentials.
    AuthenticationFailure,
    /// A source answered with a malformed response.
    InvalidResponse,
    /// A source's schema drifted from its provider contract.
    SchemaFailure,
    /// Fetched data failed validation and was not committed.
    DataValidationFailure,
    /// The SQLite store failed.
    DatabaseFailure,
    /// A learning stage failed (a typed numerical or statistical failure).
    LearningFailure,
}

impl TerminalState {
    /// Every state, in declaration order (the order of engine-spec §8.6.4), for exhaustive
    /// listings such as `grid exit-codes` and for tests.
    pub const ALL: [TerminalState; 14] = [
        TerminalState::NotRun,
        TerminalState::Running,
        TerminalState::Success,
        TerminalState::NoNewData,
        TerminalState::PartialSuccess,
        TerminalState::NetworkFailure,
        TerminalState::Timeout,
        TerminalState::RateLimited,
        TerminalState::AuthenticationFailure,
        TerminalState::InvalidResponse,
        TerminalState::SchemaFailure,
        TerminalState::DataValidationFailure,
        TerminalState::DatabaseFailure,
        TerminalState::LearningFailure,
    ];

    /// The process exit code of engine-output-contract §13, or `None` for the two non-terminal
    /// states.
    ///
    /// | State | Code |
    /// |---|---|
    /// | `Success` | 0 |
    /// | `NoNewData` | 10 |
    /// | `PartialSuccess` | 11 |
    /// | `NetworkFailure` | 20 |
    /// | `Timeout` | 21 |
    /// | `RateLimited` | 22 |
    /// | `AuthenticationFailure` | 23 |
    /// | `InvalidResponse` | 24 |
    /// | `SchemaFailure` | 25 |
    /// | `DataValidationFailure` | 26 |
    /// | `DatabaseFailure` | 30 |
    /// | `LearningFailure` | 40 |
    /// | `NotRun`, `Running` | none |
    ///
    /// A usage error exits [`EXIT_USAGE`] and an internal error [`EXIT_INTERNAL`].
    pub const fn exit_code(self) -> Option<i32> {
        match self {
            TerminalState::NotRun | TerminalState::Running => None,
            TerminalState::Success => Some(0),
            TerminalState::NoNewData => Some(10),
            TerminalState::PartialSuccess => Some(11),
            TerminalState::NetworkFailure => Some(20),
            TerminalState::Timeout => Some(21),
            TerminalState::RateLimited => Some(22),
            TerminalState::AuthenticationFailure => Some(23),
            TerminalState::InvalidResponse => Some(24),
            TerminalState::SchemaFailure => Some(25),
            TerminalState::DataValidationFailure => Some(26),
            TerminalState::DatabaseFailure => Some(30),
            TerminalState::LearningFailure => Some(40),
        }
    }

    /// Whether the state is terminal: every value except `NotRun` and `Running`.
    pub const fn is_terminal(self) -> bool {
        self.exit_code().is_some()
    }

    /// The `SCREAMING_SNAKE_CASE` name engine-spec §8.6.4 writes.
    pub const fn spec_name(self) -> &'static str {
        match self {
            TerminalState::NotRun => "NOT_RUN",
            TerminalState::Running => "RUNNING",
            TerminalState::Success => "SUCCESS",
            TerminalState::NoNewData => "NO_NEW_DATA",
            TerminalState::PartialSuccess => "PARTIAL_SUCCESS",
            TerminalState::NetworkFailure => "NETWORK_FAILURE",
            TerminalState::Timeout => "TIMEOUT",
            TerminalState::RateLimited => "RATE_LIMITED",
            TerminalState::AuthenticationFailure => "AUTHENTICATION_FAILURE",
            TerminalState::InvalidResponse => "INVALID_RESPONSE",
            TerminalState::SchemaFailure => "SCHEMA_FAILURE",
            TerminalState::DataValidationFailure => "DATA_VALIDATION_FAILURE",
            TerminalState::DatabaseFailure => "DATABASE_FAILURE",
            TerminalState::LearningFailure => "LEARNING_FAILURE",
        }
    }
}

impl fmt::Display for TerminalState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.spec_name())
    }
}

/// The status of a durable job (engine-spec §8.10 "Status values"). Six values. **Closed** enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JobStatus {
    /// Enqueued; upstream edges not yet satisfied or not yet started.
    Pending,
    /// Leased by a live process.
    Running,
    /// Completed; its registered outputs are reusable under its idempotency key.
    Succeeded,
    /// Attempts exhausted; `JobFailed` was emitted. Never silently skipped.
    Failed,
    /// Cancelled at a durable boundary on graceful shutdown (engine-spec §8.11).
    Cancelled,
    /// Skipped because its input data version carried no new data.
    SkippedNoNewData,
}

impl JobStatus {
    /// Every status, in declaration order.
    pub const ALL: [JobStatus; 6] = [
        JobStatus::Pending,
        JobStatus::Running,
        JobStatus::Succeeded,
        JobStatus::Failed,
        JobStatus::Cancelled,
        JobStatus::SkippedNoNewData,
    ];

    /// The `SCREAMING_SNAKE_CASE` name engine-spec §8.10 writes.
    pub const fn spec_name(self) -> &'static str {
        match self {
            JobStatus::Pending => "PENDING",
            JobStatus::Running => "RUNNING",
            JobStatus::Succeeded => "SUCCEEDED",
            JobStatus::Failed => "FAILED",
            JobStatus::Cancelled => "CANCELLED",
            JobStatus::SkippedNoNewData => "SKIPPED_NO_NEW_DATA",
        }
    }
}

impl fmt::Display for JobStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.spec_name())
    }
}

/// The job types of engine-spec §8.10: ten kept from the superseded final-build-spec §15 and ten
/// added for the engine and GRID. Twenty values. **Closed** enum: the engine dispatches on the
/// stage, so a consumer cannot tolerate an unknown one. P1-02 fixes the full enumeration
/// (engine-spec §8.10); adding a value to this closed enum is a MAJOR contract change
/// (engine-spec §5.5 "Compatibility"), never a MINOR one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StageId {
    /// `DATA_FETCH`: fetch a source (once per day per source).
    DataFetch,
    /// `NORMALIZE_DATA`: normalize and quarantine raw rows.
    NormalizeData,
    /// `BUILD_FEATURES`: build the affected feature partitions.
    BuildFeatures,
    /// `RAPM_UPDATE`: block accumulation and the windowed solve (DR-D11).
    RapmUpdate,
    /// `KALMAN_UPDATE`: advance the state-space filter for a game-week.
    KalmanUpdate,
    /// `RTS_SMOOTH`: retrospective smoothing (diagnostics only; never a live record).
    RtsSmooth,
    /// `EMPIRICAL_BAYES_UPDATE`: update the empirical-Bayes rate components.
    EmpiricalBayesUpdate,
    /// `BOOSTING_UPDATE`: continue, replay or rebuild a boosted component.
    BoostingUpdate,
    /// `MODEL_VALIDATE`: the §8.8 promotion gate.
    ModelValidate,
    /// `MODEL_PROMOTE`: move a production pointer (serialized globally, §8.13).
    ModelPromote,
    /// `IDENTITY_RESOLVE`: resolve cross-source identities and open review items.
    IdentityResolve,
    /// `VALUE_MODEL_FIT`: fit V(s).
    ValueModelFit,
    /// `COMPUTE_DV`: attach `dV = V(s′) − V(s)` to plays.
    ComputeDv,
    /// `LAYER1_CREDIT`: weekly Layer-1′ credit.
    Layer1Credit,
    /// `FIXED_LAG_SMOOTH`: fixed-lag smoothing for a game-week.
    FixedLagSmooth,
    /// `PRIOR_TRANSLATE`: translate feeder-league priors to the NFL scale.
    PriorTranslate,
    /// `PREDICT`: Layers A–F and scoring for one prediction run.
    Predict,
    /// `EVALUATE`: a rolling-origin or validation evaluation.
    Evaluate,
    /// `SNAPSHOT`: a snapshot record.
    Snapshot,
    /// `ROLLBACK`: move a production pointer back (serialized globally, §8.13).
    Rollback,
}

impl StageId {
    /// Every stage, in the order engine-spec §8.10 lists them.
    pub const ALL: [StageId; 20] = [
        StageId::DataFetch,
        StageId::NormalizeData,
        StageId::BuildFeatures,
        StageId::RapmUpdate,
        StageId::KalmanUpdate,
        StageId::RtsSmooth,
        StageId::EmpiricalBayesUpdate,
        StageId::BoostingUpdate,
        StageId::ModelValidate,
        StageId::ModelPromote,
        StageId::IdentityResolve,
        StageId::ValueModelFit,
        StageId::ComputeDv,
        StageId::Layer1Credit,
        StageId::FixedLagSmooth,
        StageId::PriorTranslate,
        StageId::Predict,
        StageId::Evaluate,
        StageId::Snapshot,
        StageId::Rollback,
    ];

    /// The `SCREAMING_SNAKE_CASE` job-type name engine-spec §8.10 writes.
    pub const fn spec_name(self) -> &'static str {
        match self {
            StageId::DataFetch => "DATA_FETCH",
            StageId::NormalizeData => "NORMALIZE_DATA",
            StageId::BuildFeatures => "BUILD_FEATURES",
            StageId::RapmUpdate => "RAPM_UPDATE",
            StageId::KalmanUpdate => "KALMAN_UPDATE",
            StageId::RtsSmooth => "RTS_SMOOTH",
            StageId::EmpiricalBayesUpdate => "EMPIRICAL_BAYES_UPDATE",
            StageId::BoostingUpdate => "BOOSTING_UPDATE",
            StageId::ModelValidate => "MODEL_VALIDATE",
            StageId::ModelPromote => "MODEL_PROMOTE",
            StageId::IdentityResolve => "IDENTITY_RESOLVE",
            StageId::ValueModelFit => "VALUE_MODEL_FIT",
            StageId::ComputeDv => "COMPUTE_DV",
            StageId::Layer1Credit => "LAYER1_CREDIT",
            StageId::FixedLagSmooth => "FIXED_LAG_SMOOTH",
            StageId::PriorTranslate => "PRIOR_TRANSLATE",
            StageId::Predict => "PREDICT",
            StageId::Evaluate => "EVALUATE",
            StageId::Snapshot => "SNAPSHOT",
            StageId::Rollback => "ROLLBACK",
        }
    }
}

impl fmt::Display for StageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.spec_name())
    }
}

/// The outcome of a completed training run (engine-spec §8.4 `TrainingStatus::Complete`).
/// **Closed** enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RunOutcome {
    /// The candidate passed the §8.8 gate and a production pointer moved.
    Promoted,
    /// The candidate failed the §8.8 gate; the production pointer is unchanged.
    Rejected,
    /// No source reported new data; nothing was trained.
    NoNewData,
}

impl fmt::Display for RunOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            RunOutcome::Promoted => "Promoted",
            RunOutcome::Rejected => "Rejected",
            RunOutcome::NoNewData => "NoNewData",
        })
    }
}

/// The training status of engine-spec §8.4, exactly as written there. It is the value carried by
/// the `TrainingStatusChanged` event and returned by `grid status`. **Closed** enum.
///
/// `Fitting::progress` is a fraction in `[0, 1]` of the named stage. `Failed` is typed: a stage
/// and a closed [`EngineErrorCode`], never a free-text reason (engine-spec §6.8).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TrainingStatus {
    /// No training in progress.
    Idle,
    /// Fetching sources.
    Fetching,
    /// Building features.
    BuildingFeatures,
    /// Fitting the named stage, with a progress fraction in `[0, 1]`.
    Fitting {
        /// The stage being fitted.
        stage: StageId,
        /// Progress fraction in `[0, 1]`.
        progress: f32,
    },
    /// Running the §8.8 validation gate.
    Validating,
    /// Moving a production pointer.
    Promoting,
    /// The run completed with the given outcome.
    Complete {
        /// How the run ended.
        outcome: RunOutcome,
    },
    /// The run failed at a stage with a typed error code.
    Failed {
        /// The stage that failed.
        stage: StageId,
        /// The typed error.
        error: EngineErrorCode,
    },
}

impl fmt::Display for TrainingStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrainingStatus::Idle => f.write_str("Idle"),
            TrainingStatus::Fetching => f.write_str("Fetching"),
            TrainingStatus::BuildingFeatures => f.write_str("BuildingFeatures"),
            TrainingStatus::Fitting { stage, progress } => {
                write!(f, "Fitting {stage} ({:.0}%)", progress * 100.0)
            }
            TrainingStatus::Validating => f.write_str("Validating"),
            TrainingStatus::Promoting => f.write_str("Promoting"),
            TrainingStatus::Complete { outcome } => write!(f, "Complete ({outcome})"),
            TrainingStatus::Failed { stage, error } => write!(f, "Failed at {stage}: {error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn exit_codes_match_the_contract_table_and_are_distinct() {
        let expected = [
            (TerminalState::Success, 0),
            (TerminalState::NoNewData, 10),
            (TerminalState::PartialSuccess, 11),
            (TerminalState::NetworkFailure, 20),
            (TerminalState::Timeout, 21),
            (TerminalState::RateLimited, 22),
            (TerminalState::AuthenticationFailure, 23),
            (TerminalState::InvalidResponse, 24),
            (TerminalState::SchemaFailure, 25),
            (TerminalState::DataValidationFailure, 26),
            (TerminalState::DatabaseFailure, 30),
            (TerminalState::LearningFailure, 40),
        ];
        for (state, code) in expected {
            assert_eq!(state.exit_code(), Some(code), "{state}");
            assert!(state.is_terminal(), "{state}");
        }
        assert_eq!(TerminalState::NotRun.exit_code(), None);
        assert_eq!(TerminalState::Running.exit_code(), None);
        assert!(!TerminalState::NotRun.is_terminal());
        assert!(!TerminalState::Running.is_terminal());

        let codes: Vec<i32> = TerminalState::ALL
            .iter()
            .filter_map(|s| s.exit_code())
            .collect();
        let distinct: HashSet<i32> = codes.iter().copied().collect();
        assert_eq!(codes.len(), 12);
        assert_eq!(distinct.len(), 12, "every terminal exit code is distinct");
        assert!(!distinct.contains(&EXIT_USAGE));
        assert!(!distinct.contains(&EXIT_INTERNAL));
        assert_eq!(EXIT_USAGE, 2);
        assert_eq!(EXIT_INTERNAL, 70);
        assert_eq!(TerminalState::ALL.len(), 14);
    }

    #[test]
    fn spec_names_are_distinct() {
        let names: HashSet<&str> = TerminalState::ALL.iter().map(|s| s.spec_name()).collect();
        assert_eq!(names.len(), 14);
        assert_eq!(
            TerminalState::DataValidationFailure.to_string(),
            "DATA_VALIDATION_FAILURE"
        );
        let stages: HashSet<&str> = StageId::ALL.iter().map(|s| s.spec_name()).collect();
        assert_eq!(stages.len(), 20);
        assert_eq!(
            StageId::EmpiricalBayesUpdate.to_string(),
            "EMPIRICAL_BAYES_UPDATE"
        );
        let jobs: HashSet<&str> = JobStatus::ALL.iter().map(|s| s.spec_name()).collect();
        assert_eq!(jobs.len(), 6);
        assert_eq!(
            JobStatus::SkippedNoNewData.to_string(),
            "SKIPPED_NO_NEW_DATA"
        );
    }

    #[test]
    fn training_status_has_the_spec_shape() {
        // Every variant of engine-spec §8.4, constructed with its typed payload.
        let all = [
            TrainingStatus::Idle,
            TrainingStatus::Fetching,
            TrainingStatus::BuildingFeatures,
            TrainingStatus::Fitting {
                stage: StageId::RapmUpdate,
                progress: 0.5,
            },
            TrainingStatus::Validating,
            TrainingStatus::Promoting,
            TrainingStatus::Complete {
                outcome: RunOutcome::Promoted,
            },
            TrainingStatus::Failed {
                stage: StageId::KalmanUpdate,
                error: EngineErrorCode::NotConverged,
            },
        ];
        assert_eq!(all.len(), 8);
        match all[3] {
            TrainingStatus::Fitting { stage, progress } => {
                assert_eq!(stage, StageId::RapmUpdate);
                assert!((0.0..=1.0).contains(&progress));
            }
            other => panic!("unexpected {other:?}"),
        }
        match all[7] {
            TrainingStatus::Failed { stage, error } => {
                assert_eq!(stage, StageId::KalmanUpdate);
                assert_eq!(error, EngineErrorCode::NotConverged);
            }
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(all[3].to_string(), "Fitting RAPM_UPDATE (50%)");
        assert_eq!(all[6].to_string(), "Complete (Promoted)");
        assert_eq!(all[7].to_string(), "Failed at KALMAN_UPDATE: NotConverged");
    }
}
