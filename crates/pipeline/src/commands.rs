//! The engine commands (engine-spec §8.2): one request type per command, the receipt every
//! command returns, and the [`EngineCommands`] façade trait.
//!
//! Commands mutate state. Each one runs as one or more durable jobs (engine-spec §8.10), is
//! recorded with its identifiers, and never overwrites a production artifact or a lock snapshot in
//! place: every change produces a new version (§8.9). Each command is a library function here and
//! a CLI subcommand (`grid-cli`; the adapter arrives with the package engine-spec §9.2.7 or
//! §10.2.6 names for its output, where one is named); `docs/03-contracts/engine-output-contract.md`
//! freezes the CLI spelling, arguments and exit codes.
//!
//! **No command has an implementation in P1-01.** [`EngineCommands`] has no implementor in this
//! workspace; the packages that own each stage provide one from P1-02 on.

use std::fmt;
use std::path::PathBuf;

use grid_domain::{
    AsOfMode, EngineError, EvaluationRunId, IngestionRunId, JobId, LockKind, ModelFamily,
    ModelVersionId, PlayerKey, PredictionVersionId, ReviewId, ScoringProfileRef, Season,
    SettingVersion, SnapshotId, StatComponent, Week,
};
use serde::{Deserialize, Serialize};

/// The §8.2 library function names, in table order (commands 1–13). The CLI pairs each of its
/// subcommand spellings with one of these, and its tests check that pairing against this list.
pub const COMMAND_NAMES: [&str; 13] = [
    "trigger_daily_update",
    "request_projection_run",
    "request_model_rebuild",
    "set_scoring_profile",
    "import_availability_overrides",
    "approve_identity_link",
    "reject_identity_link",
    "import_benchmark_snapshot",
    "promote_candidate",
    "rollback_to_snapshot",
    "export_projections",
    "set_model_config",
    "request_evaluation_run",
];

/// A versioned **affine** scoring profile to create (engine-spec §5.4; engine-output-contract
/// §3.7; DR-D6). `fantasy_points = Σ weights[c] · stat[c] + offset`. A non-affine rule (a bonus
/// threshold, a tiered yardage rule) cannot be expressed by this type at all, which is how the
/// engine rejects it: there is no field to put it in. A profile version is never mutated; a change
/// is a new version. Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoringProfileDefinition {
    /// The profile name (`Standard`, `Half-PPR`, `PPR`, or a custom name). Open vocabulary.
    pub name: String,
    /// The weight per stat component, in fantasy points per unit of the component
    /// ([`StatComponent::unit`]). A component absent from the list has weight 0. A component
    /// listed twice is a contract violation the implementor rejects, never a silent sum.
    pub weights: Vec<(StatComponent, f64)>,
    /// The affine offset, in fantasy points. `0` for every standard profile.
    pub offset: f64,
}

/// The file format of an export (engine-spec §8.2 command 11; engine-output-contract §8 rule 2).
/// CSV is required; JSON is the record-envelope form of [`crate::serialization`]. Parquet is not a
/// variant: it needs a dependency that needs owner approval. **Closed** enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExportFormat {
    /// UTF-8 CSV with the §8 rule 4 hygiene, plus the JSON sidecar.
    Csv,
    /// JSON records in the `grid.output` envelope, plus the JSON sidecar.
    Json,
}

impl fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ExportFormat::Csv => "csv",
            ExportFormat::Json => "json",
        })
    }
}

/// The specification of a rolling-origin evaluation run (engine-spec §8.2 command 13; §7.1,
/// §7.7). Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EvaluationSpec {
    /// The seasons whose weeks are the rolling origins. Each origin is a `(season, week)` slot,
    /// never a bare week (KI-NEW-Z61). Must be non-empty; an empty list is a contract violation
    /// the implementor rejects.
    pub seasons: Vec<Season>,
    /// The lock every origin is evaluated at.
    pub lock_kind: LockKind,
    /// The scoring profile the fantasy-point metrics are computed in.
    pub scoring_profile: ScoringProfileRef,
    /// The pre-registered metric definitions in force (engine-spec §7.7; a DR-D8 versioned
    /// setting).
    pub metrics_version: SettingVersion,
    /// The as-of mode every origin runs in (`evaluation-and-leakage.md` §4.1; engine-spec §4.5).
    /// `Backtest` for an evaluation that may feed a gate or a promotion. `Research` relaxes the
    /// rule the run names, and every output of such a run (scorecard, benchmark results,
    /// validation report reference) carries `research_only = true` and MUST NOT feed a gate, a
    /// promotion or a published claim. `Production` is the live projection mode; whether an
    /// evaluation run may request it is stated by no document (open; raised in the P1-01 report).
    pub mode: AsOfMode,
}

/// Command 1: `trigger_daily_update()` — `grid update [--offline]`. Never null.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TriggerDailyUpdate {
    /// When true, replay from the retained raw cache with no network access (engine-spec §8.6.1).
    pub offline: bool,
}

/// Command 2: `request_projection_run(season, week, lock_type)` — `grid project`. Never null.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RequestProjectionRun {
    /// The target season.
    pub season: Season,
    /// The target week (never 0: KI-A4).
    pub week: Week,
    /// The lock the projection is made at.
    pub lock_kind: LockKind,
}

/// Command 3: `request_model_rebuild(model_family)` — `grid rebuild <model-family>`. Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RequestModelRebuild {
    /// The model family to rebuild (open vocabulary).
    pub model_family: ModelFamily,
}

/// Command 4: `set_scoring_profile(profile)` — `grid scoring add|set-default <profile>`. Never
/// null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetScoringProfile {
    /// The affine profile to create as a new version.
    pub profile: ScoringProfileDefinition,
}

/// Command 5: `import_availability_overrides(file)` — `grid availability import <file>`. Never
/// null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ImportAvailabilityOverrides {
    /// The CSV file of timestamped overrides. Validated on import; formula injection is rejected
    /// (engine-spec §14). The path never appears in an output, sidecar or log
    /// (engine-output-contract §8 rule 5).
    pub path: PathBuf,
}

/// Command 6: `approve_identity_link(review_id, canonical_player_id)` — `grid identity approve`.
/// Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ApproveIdentityLink {
    /// The review item to resolve.
    pub review_id: ReviewId,
    /// The canonical player the source id is linked to.
    pub canonical_player_id: PlayerKey,
}

/// Command 7: `reject_identity_link(review_id)` — `grid identity reject`. Never null.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RejectIdentityLink {
    /// The review item to resolve.
    pub review_id: ReviewId,
}

/// Command 8: `import_benchmark_snapshot(provider, file, observed_at)` — `grid benchmark import`.
/// Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ImportBenchmarkSnapshot {
    /// The benchmark provider, as named in `docs/04-providers/` (open vocabulary).
    pub provider: String,
    /// The snapshot file. Never appears in an output, sidecar or log.
    pub path: PathBuf,
    /// When the provider published the snapshot: the lock-time eligibility timestamp
    /// (engine-spec §7.6).
    pub observed_at: grid_domain::UtcInstant,
}

/// Command 9: `promote_candidate(model_version)` — `grid model promote <model-version>`. Never
/// null.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PromoteCandidate {
    /// The candidate model version.
    pub model_version: ModelVersionId,
}

/// Command 10: `rollback_to_snapshot(snapshot_id)` — `grid model rollback <snapshot-id>`. Never
/// null.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RollbackToSnapshot {
    /// The snapshot whose recorded versions become production again.
    pub snapshot_id: SnapshotId,
}

/// Command 11: `export_projections(season, week, lock_type, scoring_profile, format)` —
/// `grid export …`. Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExportProjections {
    /// The season of the prediction version to export.
    pub season: Season,
    /// The week of the prediction version to export (never 0).
    pub week: Week,
    /// The lock of the prediction version to export.
    pub lock_kind: LockKind,
    /// The immutable scoring profile version whose fantasy distribution is exported.
    pub scoring_profile: ScoringProfileRef,
    /// The file format.
    pub format: ExportFormat,
}

/// Command 12: `set_model_config(model_family, config)` — `grid model configure`. Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SetModelConfig {
    /// The model family the configuration applies to.
    pub model_family: ModelFamily,
    /// The configuration file (for example a λ change). Never appears in an output, sidecar or
    /// log.
    pub config_path: PathBuf,
}

/// Command 13: `request_evaluation_run(evaluation_spec)` — `grid backtest …`. Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RequestEvaluationRun {
    /// The evaluation to run.
    pub spec: EvaluationSpec,
}

/// What a command returns: the identifiers it recorded (engine-spec §8.2 "is recorded with its
/// identifiers"; §12.6 "every §8.2 command records its identifiers and creates new versions").
///
/// Every field other than `job_ids` is `None` when the command creates no identifier of that kind
/// (for example `reject_identity_link` creates no version); it is never a zero id. `job_ids` lists
/// the durable jobs the command enqueued, in enqueue order; a command that completes synchronously
/// with no job (none exists in §8.2 today) would leave it empty. Never null.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommandReceipt {
    /// The durable jobs enqueued (engine-spec §8.10).
    pub job_ids: Vec<JobId>,
    /// The ingestion run started, for `trigger_daily_update` and the import commands.
    pub ingestion_run_id: Option<IngestionRunId>,
    /// The prediction version the command created (`request_projection_run`) or exported
    /// (`export_projections`, which exports an existing version and creates none).
    pub prediction_version_id: Option<PredictionVersionId>,
    /// The candidate or production model version the command created or moved a pointer to.
    pub model_version_id: Option<ModelVersionId>,
    /// The snapshot recorded before a promotion or restored by a rollback (engine-spec §8.7.4).
    pub snapshot_id: Option<SnapshotId>,
    /// The evaluation run started, for `request_evaluation_run`.
    pub evaluation_run_id: Option<EvaluationRunId>,
}

/// The engine's command façade (engine-spec §8.2). One method per command, in table order.
///
/// Every method takes `&self`: the engine's authoritative state is in SQLite and the artifact
/// store, never in the façade value, so a shared handle can serve commands and queries at once
/// (read-only queries MUST remain answerable while a backtest or rebuild runs: engine-spec §8.13).
/// Every method is synchronous (see the crate documentation) and returns the identifiers it
/// recorded or a typed [`EngineError`]; no method returns a fabricated receipt.
///
/// **No implementor exists in P1-01.** Implementors arrive with the packages that own each
/// command's stages (P1-02 onward).
///
/// **The implementing agent MUST NOT run [`promote_candidate`](Self::promote_candidate) or
/// [`rollback_to_snapshot`](Self::rollback_to_snapshot) against a production database on its own
/// authority** (engine-spec §8.2 last paragraph; §1.3; Appendix D "merging, signing, publishing,
/// promoting, or deploying on the agent's own authority"). Promoting a production model is a human
/// gate (§10.5).
pub trait EngineCommands {
    /// Command 1. A one-shot, idempotent, catch-up-aware run of the §8.6 pipeline, under the
    /// once-per-day-per-source fetch cap; `offline` replays from the retained raw cache with no
    /// network access. Its terminal state (§8.6.4) maps to a process exit code in the CLI.
    fn trigger_daily_update(
        &self,
        request: TriggerDailyUpdate,
    ) -> Result<CommandReceipt, EngineError>;

    /// Command 2. Produces a prediction version for `(season, week)` at `lock_kind`. A lock
    /// snapshot is immutable; a post-lock run is a new prediction version, never an overwrite
    /// (§7.2, §10.3).
    fn request_projection_run(
        &self,
        request: RequestProjectionRun,
    ) -> Result<CommandReceipt, EngineError>;

    /// Command 3. A periodic or manual rebuild of one model family (§8.7.1). Produces a new
    /// **candidate** model version that must pass §8.8 before promotion; it never touches
    /// production.
    fn request_model_rebuild(
        &self,
        request: RequestModelRebuild,
    ) -> Result<CommandReceipt, EngineError>;

    /// Command 4. Creates a new versioned affine profile and never mutates an existing version.
    /// Existing projections are re-scored from the stored draws without re-running the football
    /// model (§5.4). A non-affine rule is rejected with `EngineErrorCode::NonAffineScoringRule`
    /// (DR-D6); the request type cannot express one.
    fn set_scoring_profile(
        &self,
        request: SetScoringProfile,
    ) -> Result<CommandReceipt, EngineError>;

    /// Command 5. Imports timestamped, audited and versioned availability overrides. The CSV is
    /// validated and formula injection is rejected (§14). Every output that uses an override
    /// carries the manual-data flag (§10.3).
    fn import_availability_overrides(
        &self,
        request: ImportAvailabilityOverrides,
    ) -> Result<CommandReceipt, EngineError>;

    /// Command 6. Resolves one identity review item by linking it to a canonical player
    /// (§4.4.3). Audited.
    fn approve_identity_link(
        &self,
        request: ApproveIdentityLink,
    ) -> Result<CommandReceipt, EngineError>;

    /// Command 7. Resolves one identity review item by rejecting the link (§4.4.3). Audited. A
    /// rejected link produces no NCAA prior.
    fn reject_identity_link(
        &self,
        request: RejectIdentityLink,
    ) -> Result<CommandReceipt, EngineError>;

    /// Command 8. Stores a benchmark snapshot for evaluation only, with its lock-time hash and
    /// legal-use metadata (§7.6). It never enters a training feature (§10.3).
    fn import_benchmark_snapshot(
        &self,
        request: ImportBenchmarkSnapshot,
    ) -> Result<CommandReceipt, EngineError>;

    /// Command 9. Promotes a candidate only through the governance checks (§8.8). Manual promotion
    /// is permitted only after the same validation report has been generated. Promoting a
    /// production model is a human gate (§10.5): **never run by the implementing agent against
    /// production** (§8.2 last paragraph).
    fn promote_candidate(&self, request: PromoteCandidate) -> Result<CommandReceipt, EngineError>;

    /// Command 10. Moves the production pointer(s) back to the versions the snapshot records
    /// (§8.7.4). Never deletes a version. **Never run by the implementing agent against
    /// production** (§8.2 last paragraph).
    fn rollback_to_snapshot(
        &self,
        request: RollbackToSnapshot,
    ) -> Result<CommandReceipt, EngineError>;

    /// Command 11. Exports projections and quantiles only, with the JSON sidecar
    /// ([`crate::serialization::ExportSidecar`]). No competitor data, ever (§5.6;
    /// engine-output-contract §8 rule 1).
    fn export_projections(&self, request: ExportProjections)
        -> Result<CommandReceipt, EngineError>;

    /// Command 12. A hyperparameter change (for example λ). Creates a new model configuration,
    /// then triggers rebuild, validation and a new model version. Never mutates production in
    /// place.
    fn set_model_config(&self, request: SetModelConfig) -> Result<CommandReceipt, EngineError>;

    /// Command 13. A rolling-origin evaluation (§7.1) under pre-registered metric definitions
    /// (§7.7). Emits `EvaluationCompleted`.
    fn request_evaluation_run(
        &self,
        request: RequestEvaluationRun,
    ) -> Result<CommandReceipt, EngineError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_names_are_distinct_and_thirteen() {
        let mut names = COMMAND_NAMES.to_vec();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 13);
    }

    #[test]
    fn receipt_default_records_nothing() {
        let receipt = CommandReceipt::default();
        assert!(receipt.job_ids.is_empty());
        assert_eq!(receipt.prediction_version_id, None);
        assert_eq!(ExportFormat::Csv.to_string(), "csv");
    }
}
