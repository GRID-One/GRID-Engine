//! Output-record round trips through the `grid.output` JSON envelope (engine-output-contract §11
//! "Serialize/deserialize round-trip of a representative record per type"; engine-spec §5.5 rule
//! 5, §12.6), and the reader's MAJOR rejection (engine-output-contract §9).
//!
//! One representative record per output type goes through `to_json` and `from_json` and must come
//! back equal. The projection record is also checked against its §3.9 invariants, so that the
//! representative is a record the engine would emit. The public request, response and event types
//! of `grid-pipeline` itself (engine-spec §5.5 rule 2: "commands and queries in the orchestration
//! crate"; rule 5: representative records round-tripped) go through the same helper: every §8.3
//! response body in its `Page` or `Versioned` wrapper, every §8.3 query and §8.2 request type, the
//! command receipt, and an `EventEnvelope` per §8.4 event variant.
//!
//! The representative values are dyadic rationals (`0.75`, `0.0625`, `18.5`, …), which every JSON
//! float parser reads exactly, so the record round trips assert the envelope and the field mapping
//! independently of float parsing. The parser's own obligation, engine-spec §12.6 "precision-
//! preserving serialization round trips … JSON exports preserve bits", is asserted separately by
//! `json_round_trip_preserves_computed_f64_bit_for_bit` on computed values.

use grid_domain::{
    AvailabilityBlock, AvailabilityReportRow, AvailabilitySource, BenchmarkResultRow,
    BenchmarkResults, ChangeAttribution, ClaimLabel, ComponentSummary, ConfidenceInterval,
    ContractVersion, DataQualityReport, DataVersion, DrawsRef, Driver, DriverUnits, EngineError,
    EngineErrorCode, EngineVersion, EnvironmentSummary, EvaluationRunId, ExceedanceProbability,
    ExplanationRecord, FantasyDistribution, FeatureSchemaVersion, Freshness, FreshnessFlags,
    GameId, GridSignalBlock, Horizon, IdentityReviewItem, IngestionRunId, JobId, JobStatus,
    JobSummary, LabelledQuantile, Lock, LockKind, ModelFamily, ModelScorecard, ModelVersionId,
    OffseasonRapm, PlayerKey, PlayerWeekProjection, Position, PredictionVersionId, PriorBlock,
    PriorVersionDelta, ProjectionChange, ProjectionKey, ProjectionLineage, QuarantinedRowRef,
    RegimeFlags, ResearchFlag, ReviewId, RoleTalent, RoleTalentBlock, ScorecardRow, ScorecardSlice,
    ScoringProfileId, ScoringProfileRef, ScoringProfileVersion, Season, SettingVersion, Slot,
    SnapshotId, SourceChange, SourceFreshness, SourceLineageRecord, SourceSnapshot, StageId,
    StatComponent, StatDistribution, StatVectorSummary, StatusReport, TeamGameEnvironment,
    TeamGameRef, TeamKey, TerminalState, TouchdownType, TrainingRunId, TrainingStatus, UtcInstant,
    Week, GRID_OUTPUT_CONTRACT_VERSION,
};
use grid_pipeline::commands::{
    ApproveIdentityLink, CommandReceipt, EvaluationSpec, ExportFormat, ExportProjections,
    ImportAvailabilityOverrides, ImportBenchmarkSnapshot, PromoteCandidate, RejectIdentityLink,
    RequestEvaluationRun, RequestModelRebuild, RequestProjectionRun, RollbackToSnapshot,
    ScoringProfileDefinition, SetModelConfig, SetScoringProfile, TriggerDailyUpdate,
};
use grid_pipeline::events::{EngineEvent, EventEnvelope};
use grid_pipeline::queries::{
    BenchmarkResultsQuery, ConfidenceBandRow, ConfidenceBandsQuery, DataFreshnessQuery,
    DataQualityQuery, IdentityReviewQueueQuery, IngestionStatus, IngestionStatusQuery, LatentTrack,
    MatchupGrade, MatchupSign, ModelFamilyStatus, ModelScorecardQuery, ModelState, ModelStatus,
    ModelStatusQuery, ModelVersionSummary, Page, PageRequest, PlayerProjectionDetail,
    PlayerProjectionDetailQuery, PlayerRatingRecord, PlayerRatingsQuery, ProjectionBoardRow,
    ProjectionChangeLogQuery, ProjectionDistributionQuery, ProjectionDistributionResponse,
    ResponseVersions, RoleTracks, SourceFetchResult, StateEstimate, TalentRole,
    TrainingStatusQuery, TrainingStatusView, UsageWeek, ValidationReportRef, Versioned,
    WeekProjectionsQuery,
};
use grid_pipeline::serialization::{from_json, to_json, ColumnSpec, ExportSidecar, RecordEnvelope};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;
use std::path::PathBuf;

fn week(w: u8) -> Week {
    Week::new(w).expect("non-zero week")
}

fn at(seconds: i64) -> UtcInstant {
    UtcInstant::from_unix_seconds(seconds)
}

fn profile() -> ScoringProfileRef {
    ScoringProfileRef {
        id: ScoringProfileId("half-ppr".into()),
        version: ScoringProfileVersion(1),
    }
}

/// A distribution around a dyadic `mean`, with dyadic multipliers so every value is exact.
fn dist(mean: f64) -> StatDistribution {
    StatDistribution {
        mean,
        sd: mean.abs() / 4.0,
        q10: mean * 0.5,
        q25: mean * 0.75,
        q50: mean,
        q75: mean * 1.25,
        q90: mean * 1.5,
    }
}

fn probability(p: f64) -> StatDistribution {
    StatDistribution {
        mean: p,
        sd: 0.0,
        q10: p,
        q25: p,
        q50: p,
        q75: p,
        q90: p,
    }
}

fn qb_vector(scale: f64) -> StatVectorSummary {
    let c = |component: StatComponent, mean: f64| ComponentSummary {
        component,
        distribution: dist(mean * scale),
    };
    StatVectorSummary {
        components: vec![
            ComponentSummary {
                component: StatComponent::ActiveProbability,
                distribution: probability(0.875),
            },
            ComponentSummary {
                component: StatComponent::StartProbability,
                distribution: probability(0.75),
            },
            c(StatComponent::PassAttempts, 34.0),
            c(StatComponent::Completions, 22.0),
            c(StatComponent::PassingYards, 250.0),
            c(StatComponent::PassingTds, 1.75),
            c(StatComponent::Interceptions, 0.75),
            c(StatComponent::SacksTaken, 2.0),
            c(StatComponent::RushingAttempts, 4.0),
            c(StatComponent::RushingYards, 18.0),
            c(StatComponent::RushingTds, 0.25),
            c(StatComponent::FumblesLost, 0.125),
            c(StatComponent::TwoPointConversions, 0.0625),
        ],
    }
}

fn fantasy(mean: f64) -> FantasyDistribution {
    FantasyDistribution {
        profile: profile(),
        mean,
        median: mean * 0.875,
        sd: mean * 0.375,
        p10: mean * 0.5,
        p25: mean * 0.75,
        p75: mean * 1.25,
        p90: mean * 1.5,
        floor: LabelledQuantile {
            value: mean * 0.5,
            percentile_definition: SettingVersion(1),
        },
        ceiling: LabelledQuantile {
            value: mean * 1.5,
            percentile_definition: SettingVersion(1),
        },
        p_zero_or_inactive: 0.0625,
        p_exceed: vec![
            ExceedanceProbability {
                threshold_points: 10.0,
                probability: 0.75,
            },
            ExceedanceProbability {
                threshold_points: 20.0,
                probability: 0.375,
            },
        ],
        threshold_set: SettingVersion(1),
        p_boom: 0.25,
        p_bust: 0.125,
        starter_thresholds: SettingVersion(1),
        draws: DrawsRef {
            matrix_id: "draws-7".into(),
            n_draws: 10_000,
        },
    }
}

fn explanation() -> ExplanationRecord {
    ExplanationRecord {
        team_environment: TeamGameRef {
            prediction_version_id: PredictionVersionId(7),
            game_id: GameId("2099_05_KC_MIN".into()),
            team: TeamKey::Nfl("KC".into()),
        },
        role_opportunity: vec![],
        availability_adjustment: Some(-0.75),
        matchup_adjustment: Some(0.5),
        ncaa_prior_contribution: Some(0.375),
        recent_evidence_contribution: Some(0.875),
        top_positive_drivers: vec![Driver {
            name: "team_pass_rate".into(),
            contribution: 1.25,
            units: DriverUnits::ExpectedFantasyPoints,
        }],
        top_negative_drivers: vec![Driver {
            name: "opponent_E_def".into(),
            contribution: -0.625,
            units: DriverUnits::DvPerPlay,
        }],
        uncertainty_drivers: vec![],
        delta_from_prior: Some(PriorVersionDelta {
            prior_prediction_version_id: PredictionVersionId(6),
            delta_points: 0.625,
        }),
        grid: Some(GridSignalBlock {
            role_talent: RoleTalentBlock {
                dropback: Some(RoleTalent {
                    filtered_mean: 0.03125,
                    filtered_var: 0.00048828125,
                    predictive_var: 0.0009765625,
                }),
                carry: None,
                target: None,
            },
            form: Some(0.015625),
            scheme_fit: None,
            offseason_rapm: Some(OffseasonRapm {
                rating: 0.03125,
                season: Season(2022),
                n_plays: 612,
            }),
            prior: Some(PriorBlock {
                mean: 0.015625,
                var: 0.0009765625,
                q: 0.875,
                n0: 200.0,
                weight_now: 0.25,
            }),
            regime_flags: RegimeFlags {
                intervention: false,
                observation_noise_inflation: false,
                scheme_reset: false,
                changepoint_z: Some(0.75),
            },
        }),
    }
}

fn projection() -> PlayerWeekProjection {
    PlayerWeekProjection {
        contract: GRID_OUTPUT_CONTRACT_VERSION,
        key: ProjectionKey {
            season: Season(2023),
            week: week(5),
            is_week_18: false,
            horizon: Horizon::Week,
            player_id: PlayerKey::Gsis("00-9900001".into()),
            position: Position::Qb,
            team: TeamKey::Nfl("KC".into()),
            opponent: TeamKey::Nfl("MIN".into()),
            game_id: GameId("2099_05_KC_MIN".into()),
            is_home: false,
            synthetic_ids: false,
        },
        lineage: ProjectionLineage {
            prediction_version_id: PredictionVersionId(7),
            data_snapshot_id: DataVersion(3),
            feature_schema_version: FeatureSchemaVersion(17),
            model_version_id: ModelVersionId(42),
            engine_version: EngineVersion {
                crate_version: "0.1.0-alpha.1".into(),
                git_sha: Some("73a8e3b".into()),
            },
            scoring_profile: profile(),
            seed: 12_345,
            n_draws: 10_000,
            lock: Lock {
                kind: LockKind::Sunday,
                at: at(1_696_770_000),
            },
            computed_at: at(1_696_769_000),
            research: ResearchFlag::none(),
        },
        availability: AvailabilityBlock {
            p_active: 0.875,
            p_start: Some(0.75),
            snap_multiplier_if_active: 1.0,
            p_limited_role: 0.0625,
            availability_source: AvailabilitySource::Provider,
            availability_observed_at: Some(at(1_696_700_000)),
            manual_override: false,
        },
        conditional: qb_vector(1.0),
        unconditional: qb_vector(0.875),
        fantasy: vec![fantasy(18.5)],
        explanation: explanation(),
        flags: fresh_flags(),
        claim_label: ClaimLabel("experimental".into()),
        attribution: vec!["nflverse".into()],
    }
}

fn fresh_flags() -> FreshnessFlags {
    FreshnessFlags {
        freshness: Freshness::Fresh,
        stale_sources: vec![],
        oldest_source_timestamp: at(1_696_600_000),
        manual_data: false,
        missing_current_week_context: false,
        experimental: true,
    }
}

fn stale_flags() -> FreshnessFlags {
    FreshnessFlags {
        freshness: Freshness::Stale,
        stale_sources: vec!["nflverse/injuries".into()],
        oldest_source_timestamp: at(1_696_400_000),
        manual_data: true,
        missing_current_week_context: true,
        experimental: true,
    }
}

fn source_snapshot() -> SourceSnapshot {
    SourceSnapshot {
        source_name: "nflverse/pbp".into(),
        source_version: "2023-10-05".into(),
        source_timestamp: at(1_696_500_000),
        retrieved_at: at(1_696_510_000),
        available_at: at(1_696_510_001),
        ingestion_run_id: Some(IngestionRunId(5)),
        sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into(),
        row_count: 48_212,
        schema_version: "pbp-2023.1".into(),
        validation_status: "valid".into(),
    }
}

fn environment_summary(mean: f64) -> EnvironmentSummary {
    EnvironmentSummary {
        mean,
        sd: mean / 8.0,
        q10: mean * 0.75,
        q50: mean,
        q90: mean * 1.25,
    }
}

fn team_game_environment() -> TeamGameEnvironment {
    TeamGameEnvironment {
        prediction_version_id: PredictionVersionId(7),
        season: Season(2023),
        week: week(5),
        game_id: GameId("2099_05_KC_MIN".into()),
        team: TeamKey::Nfl("KC".into()),
        opponent: TeamKey::Nfl("MIN".into()),
        is_home: false,
        offensive_plays: environment_summary(64.0),
        drives: environment_summary(11.0),
        pass_attempts: environment_summary(36.0),
        rush_attempts: environment_summary(25.0),
        sacks: environment_summary(2.0),
        touchdowns_by_type: vec![
            (TouchdownType::Passing, environment_summary(1.9)),
            (TouchdownType::Rushing, environment_summary(1.1)),
            (
                TouchdownType::Other("Defensive".into()),
                environment_summary(0.1),
            ),
        ],
        red_zone_opportunities: environment_summary(3.5),
        pace: environment_summary(27.5),
        neutral_pass_tendency: environment_summary(0.5625),
        game_script: environment_summary(3.5),
        implied_points: environment_summary(26.5),
        shared_latent_ids: vec!["game-2099_05_KC_MIN".into()],
        research: ResearchFlag::none(),
    }
}

fn projection_change() -> ProjectionChange {
    ProjectionChange {
        player_id: PlayerKey::Gsis("00-9900001".into()),
        season: Season(2023),
        week: week(5),
        scoring_profile: profile(),
        prior_prediction_version_id: PredictionVersionId(6),
        current_prediction_version_id: PredictionVersionId(7),
        prior_points: 17.875,
        current_points: 18.5,
        delta: 0.625,
        attribution: ChangeAttribution {
            role: 0.125,
            availability: 0.0,
            matchup: 0.25,
            team_environment: 0.25,
            model_update: 0.0,
        },
        attribution_method: SettingVersion(1),
        research: ResearchFlag::relaxed("publication-lag: in-season participation admitted"),
    }
}

fn data_quality_report() -> DataQualityReport {
    DataQualityReport {
        stale_sources: vec!["nflverse/injuries".into()],
        unresolved_identities: 3,
        quarantined_rows: vec![QuarantinedRowRef {
            ingestion_run_id: IngestionRunId(5),
            source: "nflverse/pbp".into(),
            reason_code: "possession_change_within_drive".into(),
            raw_row_ref: "pbp/2023/part-0003#1187".into(),
        }],
        missing_current_week_context: vec!["depth_chart".into()],
        source_changes: vec![SourceChange {
            source: "nflverse/pbp".into(),
            rows_before: 48_000,
            rows_after: 48_212,
            schema_changed: false,
        }],
    }
}

fn status_report() -> StatusReport {
    StatusReport {
        last_successful_update: Some(at(1_696_510_000)),
        terminal_state: TerminalState::PartialSuccess,
        training_status: TrainingStatus::Fitting {
            stage: StageId::KalmanUpdate,
            progress: 0.5,
        },
        sources: vec![SourceFreshness {
            source: "nflverse/pbp".into(),
            status: Freshness::Fresh,
            last_published: Some(at(1_696_500_000)),
            threshold_seconds: 86_400,
        }],
        production_models: vec![(ModelFamily("rapm".into()), ModelVersionId(42))],
        jobs: vec![JobSummary {
            job_id: JobId(11),
            stage: StageId::DataFetch,
            status: JobStatus::Failed,
            error: Some(EngineError::new(
                EngineErrorCode::SourceUnavailable,
                "nflverse/injuries returned 503",
            )),
        }],
    }
}

fn availability_report_row() -> AvailabilityReportRow {
    AvailabilityReportRow {
        player_id: PlayerKey::Synthetic(17),
        season: Season(2023),
        week: week(5),
        state: "QUESTIONABLE".into(),
        p_active: 0.75,
        snap_multiplier_if_active: 0.875,
        availability_source: AvailabilitySource::Override,
        observed_at: Some(at(1_696_700_000)),
        manual_override: true,
    }
}

fn identity_review_item() -> IdentityReviewItem {
    IdentityReviewItem {
        review_id: ReviewId(9),
        canonical_candidate: Some(PlayerKey::Gsis("00-9900003".into())),
        source_name: "cfbd".into(),
        source_player_id: "4431611".into(),
        tier: 3,
        confidence: 0.75,
        opened_at: at(1_696_000_000),
    }
}

fn model_scorecard() -> ModelScorecard {
    ModelScorecard {
        model_version_id: ModelVersionId(42),
        evaluation_run_id: EvaluationRunId(4),
        rows: vec![
            ScorecardRow {
                season: Season(2023),
                week: Some(week(5)),
                position: Some(Position::Qb),
                slice: ScorecardSlice::All,
                pb_mae: Some(0.40625),
                mae: Some(5.25),
                rmse: Some(6.875),
                rank_accuracy: Some(0.625),
                brier_active: Some(0.0390625),
                interval_coverage: Some(0.78125),
            },
            ScorecardRow {
                season: Season(2023),
                week: None,
                position: None,
                slice: ScorecardSlice::RookieLowEvidence,
                pb_mae: Some(0.5625),
                mae: Some(6.125),
                rmse: None,
                rank_accuracy: None,
                brier_active: None,
                interval_coverage: Some(0.75),
            },
        ],
    }
}

fn benchmark_results() -> BenchmarkResults {
    BenchmarkResults {
        rows: vec![BenchmarkResultRow {
            provider: "provider-A".into(),
            observed_at: at(1_696_700_000),
            metric: "pb_mae".into(),
            value: 0.4375,
            interval: ConfidenceInterval {
                lower: 0.40625,
                upper: 0.46875,
                level: 0.875,
            },
            n_player_weeks: 1_812,
        }],
    }
}

fn export_sidecar() -> ExportSidecar {
    ExportSidecar {
        contract: GRID_OUTPUT_CONTRACT_VERSION,
        columns: vec![
            ColumnSpec {
                name: "player_id".into(),
                unit: "identifier".into(),
                nullable: false,
            },
            ColumnSpec {
                name: "mean_points".into(),
                unit: "fantasy points (half-ppr@1)".into(),
                nullable: false,
            },
            ColumnSpec {
                name: "p_start".into(),
                unit: "probability".into(),
                nullable: true,
            },
        ],
        prediction_version_id: PredictionVersionId(7),
        scoring_profile: profile(),
        row_count: 312,
        data_sha256: "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08".into(),
        attribution: vec!["nflverse".into()],
    }
}

fn round_trip<T>(label: &str, record: &T)
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    let json = to_json(record).unwrap_or_else(|e| panic!("{label}: to_json failed: {e}"));
    let envelope: RecordEnvelope<T> =
        from_json(&json).unwrap_or_else(|e| panic!("{label}: from_json failed: {e}"));
    assert_eq!(envelope.contract.0, "grid.output", "{label}");
    assert_eq!(envelope.version, GRID_OUTPUT_CONTRACT_VERSION, "{label}");
    assert_eq!(
        &envelope.record, record,
        "{label}: record changed in the round trip"
    );
    // A second trip through the same path is the identity as well.
    let again = to_json(&envelope.record).unwrap();
    assert_eq!(again, json, "{label}: serialization is not stable");
}

#[test]
fn representative_projection_is_valid() {
    projection()
        .validate()
        .expect("the representative projection satisfies §3.9");
    team_game_environment()
        .validate()
        .expect("the representative environment satisfies §3.9");
}

#[test]
fn player_week_projection_round_trips() {
    round_trip("PlayerWeekProjection", &projection());
}

#[test]
fn source_lineage_record_round_trips() {
    round_trip(
        "SourceLineageRecord",
        &SourceLineageRecord {
            prediction_version_id: PredictionVersionId(7),
            sources: vec![source_snapshot()],
        },
    );
}

#[test]
fn team_game_environment_round_trips() {
    round_trip("TeamGameEnvironment", &team_game_environment());
}

#[test]
fn projection_change_round_trips() {
    round_trip("ProjectionChange", &projection_change());
}

#[test]
fn data_quality_report_round_trips() {
    round_trip("DataQualityReport", &data_quality_report());
}

#[test]
fn status_report_round_trips() {
    round_trip("StatusReport", &status_report());
}

#[test]
fn availability_report_row_round_trips() {
    round_trip("AvailabilityReportRow", &availability_report_row());
}

#[test]
fn identity_review_item_round_trips() {
    round_trip("IdentityReviewItem", &identity_review_item());
}

#[test]
fn model_scorecard_round_trips() {
    round_trip("ModelScorecard", &model_scorecard());
}

#[test]
fn benchmark_results_round_trips() {
    round_trip("BenchmarkResults", &benchmark_results());
}

#[test]
fn export_sidecar_round_trips() {
    round_trip("ExportSidecar", &export_sidecar());
}

#[test]
fn reader_rejects_an_unknown_major() {
    let json = to_json(&projection()).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
    value["version"]["major"] = serde_json::Value::from(2);
    let err = from_json::<PlayerWeekProjection>(&value.to_string()).unwrap_err();
    assert_eq!(err.code, EngineErrorCode::UnsupportedVersion);
    assert!(err.message.contains("MAJOR 2"), "{}", err.message);
    // MAJOR 0 is equally unknown.
    value["version"]["major"] = serde_json::Value::from(0);
    let err = from_json::<PlayerWeekProjection>(&value.to_string()).unwrap_err();
    assert_eq!(err.code, EngineErrorCode::UnsupportedVersion);
}

#[test]
fn reader_tolerates_a_minor_addition() {
    let json = to_json(&projection()).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
    // A later MINOR adds an optional field a 1.0 reader does not know.
    value["version"]["minor"] = serde_json::Value::from(3);
    value["record"]["flags"]["added_in_1_3"] = serde_json::Value::from(true);
    let envelope = from_json::<PlayerWeekProjection>(&value.to_string()).unwrap();
    assert_eq!(envelope.version, ContractVersion::new(1, 3));
    assert_eq!(envelope.record, projection());
}

#[test]
fn reader_rejects_a_missing_required_field_as_contract_violation() {
    let json = to_json(&projection()).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
    value["record"]["lineage"]
        .as_object_mut()
        .unwrap()
        .remove("prediction_version_id");
    let err = from_json::<PlayerWeekProjection>(&value.to_string()).unwrap_err();
    assert_eq!(err.code, EngineErrorCode::ContractViolation);
}

#[test]
fn week_zero_is_rejected_on_read() {
    // KI-A4: a `week = 0` sentinel cannot enter through deserialization.
    let json = to_json(&projection_change()).unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
    value["record"]["week"] = serde_json::Value::from(0);
    let err = from_json::<ProjectionChange>(&value.to_string()).unwrap_err();
    assert_eq!(err.code, EngineErrorCode::ContractViolation);
}

#[test]
fn to_json_rejects_non_finite_inside_option() {
    // engine-output-contract §3.9 "no NaN or infinity anywhere"; engine-spec §5.5 "It is not
    // emitted". `serde_json` alone writes `Some(NaN)` as `null`, which a reader takes back as
    // `None`: a non-finite value turned into a declared-missing one (KI-P1; Appendix D
    // "converting malformed data to a healthy default"). The writer must refuse instead.
    let mut record = projection();
    record.explanation.grid.as_mut().unwrap().form = Some(f64::NAN);
    let unguarded = serde_json::to_value(&record).unwrap();
    assert!(
        unguarded["explanation"]["grid"]["form"].is_null(),
        "serde_json writes Some(NaN) as null; the guard in to_json exists for this"
    );
    let err = to_json(&record).unwrap_err();
    assert_eq!(err.code, EngineErrorCode::NonFinite);
    assert!(
        err.message.contains("`record.explanation.grid.form`"),
        "{}",
        err.message
    );

    let mut record = projection();
    record
        .explanation
        .grid
        .as_mut()
        .unwrap()
        .regime_flags
        .changepoint_z = Some(f64::INFINITY);
    let err = to_json(&record).unwrap_err();
    assert_eq!(err.code, EngineErrorCode::NonFinite);
    assert!(
        err.message
            .contains("`record.explanation.grid.regime_flags.changepoint_z`"),
        "{}",
        err.message
    );

    let mut record = projection();
    record.explanation.ncaa_prior_contribution = Some(f64::NEG_INFINITY);
    let err = to_json(&record).unwrap_err();
    assert_eq!(err.code, EngineErrorCode::NonFinite);

    // A query response, through its page wrapper: the path names the item.
    let mut rows = player_rating_records();
    rows[1].weekly_credit = Some(f64::NAN);
    let err = to_json(&page(rows, model_versions())).unwrap_err();
    assert_eq!(err.code, EngineErrorCode::NonFinite);
    assert!(
        err.message.contains("`record.items.1.weekly_credit`"),
        "{}",
        err.message
    );

    // And a required field, for completeness: the writer refuses before the reader would.
    let mut record = projection();
    record.availability.p_active = f64::NAN;
    let err = to_json(&record).unwrap_err();
    assert_eq!(err.code, EngineErrorCode::NonFinite);
    assert!(
        err.message.contains("`record.availability.p_active`"),
        "{}",
        err.message
    );
}

#[test]
fn json_round_trip_preserves_computed_f64_bit_for_bit() {
    // engine-spec §12.6: a JSON round trip preserves bits. These are products and quotients the
    // engine computes routinely (a quantile of a count, a scaled share); their shortest decimal
    // forms run to 16 or 17 significant digits. `serde_json` writes each in that shortest
    // round-tripping form; the reader must parse it back to the same f64, which needs correctly
    // rounded float parsing (`serde_json`'s `float_roundtrip` feature). Without that feature the
    // parser is best-effort on long significands and lands one ULP away.
    let values: [f64; 10] = [
        36.0 * 1.2,
        1.1 / 10.0,
        0.58 / 10.0,
        0.58 * 0.8,
        18.0 * 0.6,
        18.0 * 1.2,
        0.7 * 1.4,
        2.1 * 0.95,
        0.1 * 1.4,
        0.19 * 0.6,
    ];
    let json = to_json(&values).unwrap();
    let back: RecordEnvelope<[f64; 10]> = from_json(&json).unwrap();
    for (original, parsed) in values.iter().zip(back.record.iter()) {
        assert_eq!(
            original.to_bits(),
            parsed.to_bits(),
            "{original:?} came back as {parsed:?} (bits {:x} vs {:x})",
            original.to_bits(),
            parsed.to_bits()
        );
    }
}

// ---------------------------------------------------------------------------------------------
// The orchestration crate's own public request, response and event types (engine-spec §5.5
// rules 2 and 5)
// ---------------------------------------------------------------------------------------------

fn slot(w: u8) -> Slot {
    Slot {
        season: Season(2023),
        week: week(w),
    }
}

fn page_request() -> PageRequest {
    PageRequest {
        offset: 0,
        limit: 50,
    }
}

/// The versions of a response read from a prediction version: all five present.
fn prediction_versions() -> ResponseVersions {
    ResponseVersions {
        data_version: Some(DataVersion(3)),
        feature_schema_version: Some(FeatureSchemaVersion(17)),
        model_version_id: Some(ModelVersionId(42)),
        prediction_version_id: Some(PredictionVersionId(7)),
        scoring_profile: Some(profile()),
    }
}

/// The versions of a status-type response: a data version and nothing else.
fn status_versions() -> ResponseVersions {
    ResponseVersions {
        data_version: Some(DataVersion(3)),
        feature_schema_version: None,
        model_version_id: None,
        prediction_version_id: None,
        scoring_profile: None,
    }
}

/// The versions of a GRID diagnostic response: read from the state-space model, not a
/// prediction version.
fn model_versions() -> ResponseVersions {
    ResponseVersions {
        data_version: Some(DataVersion(3)),
        feature_schema_version: Some(FeatureSchemaVersion(17)),
        model_version_id: Some(ModelVersionId(42)),
        prediction_version_id: None,
        scoring_profile: None,
    }
}

fn page<T>(items: Vec<T>, versions: ResponseVersions) -> Page<T> {
    Page {
        items,
        offset: 0,
        total: Some(312),
        versions,
        stale_data: true,
        manual_data: true,
    }
}

fn versioned<T>(body: T, versions: ResponseVersions) -> Versioned<T> {
    Versioned {
        body,
        versions,
        stale_data: false,
        manual_data: false,
    }
}

fn projection_board_rows() -> Vec<ProjectionBoardRow> {
    let qb = projection();
    let mut rb_key = qb.key.clone();
    rb_key.player_id = PlayerKey::Synthetic(17);
    rb_key.position = Position::Rb;
    rb_key.synthetic_ids = true;
    vec![
        ProjectionBoardRow {
            key: qb.key,
            scoring_profile: profile(),
            mean_points: 18.5,
            median_points: 16.1875,
            p10_points: 9.25,
            p90_points: 27.75,
            p_active: 0.875,
            snap_multiplier_if_active: 1.0,
            offensive_snap_share: None,
            flags: fresh_flags(),
        },
        ProjectionBoardRow {
            key: rb_key,
            scoring_profile: profile(),
            mean_points: 11.25,
            median_points: 10.5,
            p10_points: 4.75,
            p90_points: 19.5,
            p_active: 0.75,
            snap_multiplier_if_active: 0.875,
            offensive_snap_share: Some(dist(0.625)),
            flags: stale_flags(),
        },
    ]
}

fn player_projection_detail() -> PlayerProjectionDetail {
    PlayerProjectionDetail {
        projection: projection(),
        recent_usage: vec![
            UsageWeek {
                slot: slot(4),
                published_at: at(1_696_200_000),
                observed: vec![
                    (StatComponent::PassAttempts, 31.0),
                    (StatComponent::Completions, 20.0),
                    (StatComponent::PassingYards, 243.0),
                ],
            },
            UsageWeek {
                slot: slot(3),
                published_at: at(1_695_600_000),
                observed: vec![(StatComponent::PassAttempts, 28.0)],
            },
        ],
    }
}

fn ingestion_status() -> IngestionStatus {
    IngestionStatus {
        last_successful_update: Some(at(1_696_510_000)),
        terminal_state: TerminalState::PartialSuccess,
        sources: vec![
            SourceFetchResult {
                source: "nflverse/pbp".into(),
                terminal_state: TerminalState::Success,
                ingestion_run_id: Some(IngestionRunId(5)),
                finished_at: Some(at(1_696_510_000)),
                error: None,
            },
            SourceFetchResult {
                source: "nflverse/injuries".into(),
                terminal_state: TerminalState::NetworkFailure,
                ingestion_run_id: Some(IngestionRunId(5)),
                finished_at: Some(at(1_696_509_000)),
                error: Some(EngineError::new(
                    EngineErrorCode::SourceUnavailable,
                    "nflverse/injuries returned 503",
                )),
            },
            SourceFetchResult {
                source: "cfbd/plays".into(),
                terminal_state: TerminalState::NotRun,
                ingestion_run_id: None,
                finished_at: None,
                error: None,
            },
        ],
    }
}

fn training_status_view() -> TrainingStatusView {
    TrainingStatusView {
        status: TrainingStatus::Fitting {
            stage: StageId::KalmanUpdate,
            progress: 0.25,
        },
        running_jobs: vec![JobSummary {
            job_id: JobId(12),
            stage: StageId::KalmanUpdate,
            status: JobStatus::Running,
            error: None,
        }],
    }
}

fn estimate(mean: f64, var: f64) -> StateEstimate {
    StateEstimate { mean, var }
}

/// A track with all three estimate kinds populated.
fn track(mean: f64) -> LatentTrack {
    LatentTrack {
        filtered: estimate(mean, 0.0009765625),
        predictive: estimate(mean, 0.001953125),
        smoothed_retrospective: Some(estimate(mean * 1.25, 0.00048828125)),
    }
}

fn player_rating_records() -> Vec<PlayerRatingRecord> {
    let prior = PriorBlock {
        mean: 0.015625,
        var: 0.0009765625,
        q: 0.875,
        n0: 200.0,
        weight_now: 0.25,
    };
    vec![
        PlayerRatingRecord {
            player_id: PlayerKey::Gsis("00-9900001".into()),
            slot: slot(5),
            position: Position::Qb,
            team: TeamKey::Nfl("KC".into()),
            offseason_rapm: Some(OffseasonRapm {
                rating: 0.03125,
                season: Season(2022),
                n_plays: 612,
            }),
            weekly_credit: Some(1.25),
            role_talent: RoleTracks {
                dropback: Some(track(0.03125)),
                carry: Some(track(0.0078125)),
                target: None,
            },
            form: Some(track(0.015625)),
            scheme_fit: Some(track(-0.00390625)),
            prior: Some(prior),
            team_net_strength: Some(0.0625),
            matchup_grade: Some(MatchupGrade {
                e_def: 0.015625,
                convention: MatchupSign::HigherIsTougher,
            }),
            regime_flags: RegimeFlags {
                intervention: true,
                observation_noise_inflation: true,
                scheme_reset: false,
                changepoint_z: Some(2.5),
            },
            flags: fresh_flags(),
        },
        PlayerRatingRecord {
            player_id: PlayerKey::Synthetic(17),
            slot: slot(5),
            position: Position::Rb,
            team: TeamKey::Synthetic(3),
            offseason_rapm: None,
            weekly_credit: None,
            role_talent: RoleTracks {
                dropback: None,
                carry: Some(LatentTrack {
                    smoothed_retrospective: None,
                    ..track(0.0078125)
                }),
                target: Some(track(0.00390625)),
            },
            form: None,
            scheme_fit: None,
            prior: None,
            team_net_strength: None,
            matchup_grade: None,
            regime_flags: RegimeFlags {
                intervention: false,
                observation_noise_inflation: false,
                scheme_reset: false,
                changepoint_z: None,
            },
            flags: stale_flags(),
        },
    ]
}

fn confidence_band_rows() -> Vec<ConfidenceBandRow> {
    [TalentRole::Dropback, TalentRole::Carry, TalentRole::Target]
        .into_iter()
        .enumerate()
        .map(|(i, role)| ConfidenceBandRow {
            player_id: PlayerKey::Gsis("00-9900001".into()),
            slot: slot(5),
            role,
            predictive_mean: 0.03125 / (i as f64 + 1.0).exp2(),
            predictive_var: 0.001953125,
            interval: ConfidenceInterval {
                lower: -0.0625,
                upper: 0.125,
                level: 0.875,
            },
            flags: if i == 0 { fresh_flags() } else { stale_flags() },
        })
        .collect()
}

fn model_status() -> ModelStatus {
    ModelStatus {
        families: vec![
            ModelFamilyStatus {
                family: ModelFamily("rapm".into()),
                production: Some(ModelVersionId(42)),
                versions: vec![
                    ModelVersionSummary {
                        model_version_id: ModelVersionId(43),
                        state: ModelState::Candidate,
                        training_run_id: Some(TrainingRunId(9)),
                        created_at: at(1_696_800_000),
                    },
                    ModelVersionSummary {
                        model_version_id: ModelVersionId(42),
                        state: ModelState::Production,
                        training_run_id: None,
                        created_at: at(1_696_000_000),
                    },
                ],
                last_validation: Some(ValidationReportRef {
                    evaluation_run_id: EvaluationRunId(4),
                    model_version_id: ModelVersionId(42),
                    completed_at: at(1_696_100_000),
                }),
            },
            ModelFamilyStatus {
                family: ModelFamily("value".into()),
                production: None,
                versions: vec![],
                last_validation: None,
            },
        ],
    }
}

fn scoring_profile_definition() -> ScoringProfileDefinition {
    ScoringProfileDefinition {
        name: "half-ppr".into(),
        weights: vec![
            (StatComponent::Receptions, 0.5),
            (StatComponent::ReceivingYards, 0.125),
            (StatComponent::ReceivingTds, 6.0),
            (StatComponent::Interceptions, -2.0),
        ],
        offset: 0.0,
    }
}

#[test]
fn query_responses_round_trip() {
    // Query 1.
    round_trip(
        "Page<ProjectionBoardRow>",
        &page(projection_board_rows(), prediction_versions()),
    );
    // Query 2.
    round_trip(
        "Versioned<PlayerProjectionDetail>",
        &versioned(player_projection_detail(), prediction_versions()),
    );
    // Query 3.
    let distribution: ProjectionDistributionResponse =
        versioned(fantasy(18.5), prediction_versions());
    round_trip("ProjectionDistributionResponse", &distribution);
    // Query 4.
    round_trip(
        "Page<ProjectionChange>",
        &page(vec![projection_change()], prediction_versions()),
    );
    // Query 5.
    round_trip(
        "Versioned<Vec<SourceFreshness>>",
        &versioned(status_report().sources, status_versions()),
    );
    // Query 6.
    round_trip(
        "Versioned<IngestionStatus>",
        &versioned(ingestion_status(), status_versions()),
    );
    // Query 7.
    round_trip(
        "Versioned<TrainingStatusView>",
        &versioned(training_status_view(), status_versions()),
    );
    // Query 8.
    round_trip(
        "Page<IdentityReviewItem>",
        &page(vec![identity_review_item()], status_versions()),
    );
    // Query 9.
    round_trip(
        "Versioned<ModelScorecard>",
        &versioned(model_scorecard(), model_versions()),
    );
    // Query 10.
    round_trip(
        "Page<BenchmarkResultRow>",
        &page(benchmark_results().rows, model_versions()),
    );
    // Query 11.
    round_trip(
        "Versioned<DataQualityReport>",
        &versioned(data_quality_report(), status_versions()),
    );
    // Query 12, with all three estimate kinds populated.
    round_trip(
        "Page<PlayerRatingRecord>",
        &page(player_rating_records(), model_versions()),
    );
    // Query 13.
    round_trip(
        "Page<ConfidenceBandRow>",
        &page(confidence_band_rows(), model_versions()),
    );
    // Query 14.
    round_trip(
        "Versioned<ModelStatus>",
        &versioned(model_status(), model_versions()),
    );
}

#[test]
fn query_requests_round_trip() {
    round_trip(
        "WeekProjectionsQuery",
        &WeekProjectionsQuery {
            season: Season(2023),
            week: week(5),
            scoring_profile: profile(),
            position: Some(Position::Wr),
            prediction_version_id: None,
            page: page_request(),
        },
    );
    round_trip(
        "PlayerProjectionDetailQuery",
        &PlayerProjectionDetailQuery {
            player_id: PlayerKey::Gsis("00-9900001".into()),
            season: Season(2023),
            week: week(5),
            horizon: Horizon::RestOfSeason,
            scoring_profile: profile(),
        },
    );
    round_trip(
        "ProjectionDistributionQuery",
        &ProjectionDistributionQuery {
            player_id: PlayerKey::Synthetic(17),
            season: Season(2023),
            week: week(5),
            horizon: Horizon::Week,
            scoring_profile: profile(),
        },
    );
    round_trip(
        "ProjectionChangeLogQuery",
        &ProjectionChangeLogQuery {
            season: Season(2023),
            week: week(5),
            scoring_profile: profile(),
            player_id: Some(PlayerKey::Gsis("00-9900001".into())),
            page: page_request(),
        },
    );
    round_trip("DataFreshnessQuery", &DataFreshnessQuery {});
    round_trip("IngestionStatusQuery", &IngestionStatusQuery {});
    round_trip("TrainingStatusQuery", &TrainingStatusQuery {});
    round_trip(
        "IdentityReviewQueueQuery",
        &IdentityReviewQueueQuery {
            page: page_request(),
        },
    );
    round_trip(
        "ModelScorecardQuery",
        &ModelScorecardQuery {
            model_version_id: Some(ModelVersionId(42)),
            evaluation_run_id: None,
            season: Some(Season(2023)),
            position: Some(Position::Te),
        },
    );
    round_trip(
        "BenchmarkResultsQuery",
        &BenchmarkResultsQuery {
            provider: Some("provider-A".into()),
            season: None,
            page: page_request(),
        },
    );
    round_trip(
        "DataQualityQuery",
        &DataQualityQuery {
            ingestion_run_id: Some(IngestionRunId(5)),
        },
    );
    round_trip(
        "PlayerRatingsQuery",
        &PlayerRatingsQuery {
            season: Season(2023),
            week: week(5),
            position: None,
            player_id: Some(PlayerKey::Synthetic(17)),
            page: page_request(),
        },
    );
    round_trip(
        "ConfidenceBandsQuery",
        &ConfidenceBandsQuery {
            player_id: PlayerKey::Gsis("00-9900001".into()),
            from: slot(1),
            to: slot(5),
            level: 0.875,
            page: page_request(),
        },
    );
    round_trip(
        "ModelStatusQuery",
        &ModelStatusQuery {
            model_family: Some(ModelFamily("rapm".into())),
        },
    );
}

#[test]
fn command_requests_and_receipt_round_trip() {
    round_trip("TriggerDailyUpdate", &TriggerDailyUpdate { offline: true });
    round_trip(
        "RequestProjectionRun",
        &RequestProjectionRun {
            season: Season(2023),
            week: week(5),
            lock_kind: LockKind::Sunday,
        },
    );
    round_trip(
        "RequestModelRebuild",
        &RequestModelRebuild {
            model_family: ModelFamily("rapm".into()),
        },
    );
    round_trip(
        "SetScoringProfile",
        &SetScoringProfile {
            profile: scoring_profile_definition(),
        },
    );
    round_trip(
        "ImportAvailabilityOverrides",
        &ImportAvailabilityOverrides {
            path: PathBuf::from("overrides/2023-05.csv"),
        },
    );
    round_trip(
        "ApproveIdentityLink",
        &ApproveIdentityLink {
            review_id: ReviewId(9),
            canonical_player_id: PlayerKey::Gsis("00-9900003".into()),
        },
    );
    round_trip(
        "RejectIdentityLink",
        &RejectIdentityLink {
            review_id: ReviewId(9),
        },
    );
    round_trip(
        "ImportBenchmarkSnapshot",
        &ImportBenchmarkSnapshot {
            provider: "provider-A".into(),
            path: PathBuf::from("benchmarks/provider-A/2023-05.csv"),
            observed_at: at(1_696_700_000),
        },
    );
    round_trip(
        "PromoteCandidate",
        &PromoteCandidate {
            model_version: ModelVersionId(43),
        },
    );
    round_trip(
        "RollbackToSnapshot",
        &RollbackToSnapshot {
            snapshot_id: SnapshotId(3),
        },
    );
    for format in [ExportFormat::Csv, ExportFormat::Json] {
        round_trip(
            "ExportProjections",
            &ExportProjections {
                season: Season(2023),
                week: week(5),
                lock_kind: LockKind::Thursday,
                scoring_profile: profile(),
                format,
            },
        );
    }
    round_trip(
        "SetModelConfig",
        &SetModelConfig {
            model_family: ModelFamily("rapm".into()),
            config_path: PathBuf::from("config/rapm-lambda.toml"),
        },
    );
    round_trip(
        "RequestEvaluationRun",
        &RequestEvaluationRun {
            spec: EvaluationSpec {
                seasons: vec![Season(2021), Season(2022), Season(2023)],
                lock_kind: LockKind::Operational,
                scoring_profile: profile(),
                metrics_version: SettingVersion(1),
            },
        },
    );
    round_trip(
        "CommandReceipt",
        &CommandReceipt {
            job_ids: vec![JobId(11), JobId(12)],
            ingestion_run_id: Some(IngestionRunId(5)),
            prediction_version_id: Some(PredictionVersionId(7)),
            model_version_id: Some(ModelVersionId(43)),
            snapshot_id: Some(SnapshotId(3)),
            evaluation_run_id: Some(EvaluationRunId(4)),
        },
    );
    round_trip("CommandReceipt::default", &CommandReceipt::default());
}

#[test]
fn event_envelopes_round_trip_for_every_variant() {
    let events = [
        EngineEvent::DataFetchStarted {
            ingestion_run_id: IngestionRunId(5),
            source: "nflverse/pbp".into(),
        },
        EngineEvent::DataFetchCompleted {
            ingestion_run_id: IngestionRunId(5),
            source: "nflverse/pbp".into(),
            terminal_state: TerminalState::Success,
            duration_ms: 4_250,
        },
        EngineEvent::DataPartialSuccess {
            ingestion_run_id: IngestionRunId(5),
            data_version: DataVersion(3),
        },
        EngineEvent::DataChanged {
            data_version: DataVersion(3),
        },
        EngineEvent::IdentityReviewRequired {
            review_id: ReviewId(9),
        },
        EngineEvent::FeaturesBuilt {
            data_version: DataVersion(3),
            feature_schema_version: FeatureSchemaVersion(17),
        },
        EngineEvent::ValueModelFitted {
            model_version_id: ModelVersionId(40),
        },
        EngineEvent::Layer1CreditComputed {
            data_version: DataVersion(3),
            slot: slot(4),
        },
        EngineEvent::RapmUpdated {
            model_version_id: ModelVersionId(42),
        },
        EngineEvent::LatentStatesUpdated {
            model_version_id: ModelVersionId(41),
            slot: slot(5),
        },
        EngineEvent::FixedLagSmoothed {
            model_version_id: ModelVersionId(41),
            slot: slot(3),
        },
        EngineEvent::BoostingUpdated {
            model_version_id: ModelVersionId(44),
        },
        EngineEvent::ProjectionRunCompleted {
            prediction_version_id: PredictionVersionId(7),
        },
        EngineEvent::BenchmarkSnapshotImported {
            provider: "provider-A".into(),
            observed_at: at(1_696_700_000),
        },
        EngineEvent::EvaluationCompleted {
            evaluation_run_id: EvaluationRunId(4),
        },
        EngineEvent::CandidateValidated {
            model_version_id: ModelVersionId(43),
            evaluation_run_id: EvaluationRunId(4),
        },
        EngineEvent::CandidateRejected {
            model_version_id: ModelVersionId(45),
            evaluation_run_id: EvaluationRunId(5),
        },
        EngineEvent::ModelPromoted {
            model_family: ModelFamily("rapm".into()),
            model_version_id: ModelVersionId(43),
        },
        EngineEvent::RollbackCompleted {
            snapshot_id: SnapshotId(3),
        },
        EngineEvent::JobFailed {
            job_id: JobId(11),
            stage: StageId::DataFetch,
            error: EngineErrorCode::SourceUnavailable,
        },
        EngineEvent::TrainingStatusChanged {
            status: TrainingStatus::Fitting {
                stage: StageId::KalmanUpdate,
                progress: 0.25,
            },
        },
    ];
    let names: Vec<&str> = events.iter().map(EngineEvent::name).collect();
    assert_eq!(
        names,
        EngineEvent::NAMES,
        "one representative per §8.4 event variant, in table order"
    );
    for (i, event) in events.into_iter().enumerate() {
        let label = format!("EventEnvelope<{}>", event.name());
        let envelope = EventEnvelope {
            emitted_at: at(1_696_510_000 + i as i64),
            job_id: if i % 2 == 0 { Some(JobId(11)) } else { None },
            event,
        };
        round_trip(&label, &envelope);
    }
}
