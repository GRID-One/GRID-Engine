//! The engine events (engine-spec §8.4): the 21-variant [`EngineEvent`] carrying identifiers
//! only, the [`EventEnvelope`], the [`EventSubscriber`] trait and the in-process [`EventBus`].
//!
//! Events notify consumers that something changed. They carry identifiers, never payloads: a run
//! id, a version id, a job id, a stage, a terminal-state or error code, a duration. A consumer
//! learns of a state change from an event and fetches what it needs through a query
//! (engine-spec §8.3). No event is emitted before its payload is committed (§12.6 "event-log and
//! query consistency").
//!
//! **Delivery.** Every event MUST reach all three of: (1) the durable `diagnostic_events` table
//! (§8.5), (2) structured `tracing` log records (§8.12), and (3) the registered in-process
//! subscribers. **P1-01 provides (3) only**: [`EventBus`] fans an envelope out to the subscribers
//! registered on it. Delivery to `diagnostic_events` and to `tracing` is P1-02's, in the
//! orchestration internals that own the database connection and the subscriber registration; this
//! module has no database and no logging dependency and claims neither.

use std::fmt;
use std::sync::Arc;

use grid_domain::{
    DataVersion, EngineErrorCode, EvaluationRunId, FeatureSchemaVersion, IngestionRunId, JobId,
    ModelFamily, ModelVersionId, PredictionVersionId, ReviewId, Slot, SnapshotId, StageId,
    TerminalState, TrainingStatus, UtcInstant,
};
use serde::{Deserialize, Serialize};

/// The engine events of engine-spec §8.4, in table order. Twenty-one variants. Each carries the
/// identifiers the table's "emitted when" column implies and nothing else; the serialized variant
/// names are the spec's event names (so `RapmUpdated` serializes as `RAPMUpdated`). **Closed**
/// enum: a consumer need not tolerate an unknown event name, so adding an event is a **MAJOR**
/// change of the library API (engine-spec §5.5 "Compatibility": adding a value to a closed enum
/// is a major change), recorded in the §8.4 table first.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EngineEvent {
    /// A source fetch begins.
    DataFetchStarted {
        /// The ingestion run.
        ingestion_run_id: IngestionRunId,
        /// The source dataset, for example `nflverse/pbp`.
        source: String,
    },
    /// A source fetch reaches a terminal state (§8.6.4).
    DataFetchCompleted {
        /// The ingestion run.
        ingestion_run_id: IngestionRunId,
        /// The source dataset.
        source: String,
        /// The terminal state reached.
        terminal_state: TerminalState,
        /// Wall-clock duration of the fetch. Units: milliseconds.
        duration_ms: u64,
    },
    /// Rows were quarantined and the valid subset committed.
    DataPartialSuccess {
        /// The ingestion run.
        ingestion_run_id: IngestionRunId,
        /// The data version the valid subset was committed as.
        data_version: DataVersion,
    },
    /// A new data version is committed (new rows or stat corrections).
    DataChanged {
        /// The new data version.
        data_version: DataVersion,
    },
    /// A new identity review item is opened.
    IdentityReviewRequired {
        /// The review item.
        review_id: ReviewId,
    },
    /// Affected feature partitions are built.
    FeaturesBuilt {
        /// The data version the features were built from.
        data_version: DataVersion,
        /// The feature schema they conform to.
        feature_schema_version: FeatureSchemaVersion,
    },
    /// V(s) is refit (season boundary or rebuild).
    ValueModelFitted {
        /// The V(s) model version.
        model_version_id: ModelVersionId,
    },
    /// Weekly Layer-1′ credit is computed for a completed game-week.
    Layer1CreditComputed {
        /// The data version the credit was computed from.
        data_version: DataVersion,
        /// The completed game-week.
        slot: Slot,
    },
    /// Offseason RAPM blocks are added or a windowed solve completes.
    #[serde(rename = "RAPMUpdated")]
    RapmUpdated {
        /// The RAPM model version.
        model_version_id: ModelVersionId,
    },
    /// State-space (Kalman) states advance for a game-week; absorbs the superseded
    /// `KalmanUpdated`.
    LatentStatesUpdated {
        /// The state-space model version.
        model_version_id: ModelVersionId,
        /// The game-week advanced to.
        slot: Slot,
    },
    /// Fixed-lag smoothing completes for a game-week.
    FixedLagSmoothed {
        /// The state-space model version.
        model_version_id: ModelVersionId,
        /// The game-week smoothed.
        slot: Slot,
    },
    /// A boosted component is continued, replayed or rebuilt.
    BoostingUpdated {
        /// The boosted component's model version.
        model_version_id: ModelVersionId,
    },
    /// A prediction version is written.
    ProjectionRunCompleted {
        /// The prediction version.
        prediction_version_id: PredictionVersionId,
    },
    /// A benchmark snapshot is stored and hashed.
    BenchmarkSnapshotImported {
        /// The benchmark provider.
        provider: String,
        /// The snapshot's observation timestamp; with `provider`, the snapshot's identity
        /// (engine-spec §7.6).
        observed_at: UtcInstant,
    },
    /// A rolling-origin or validation evaluation completes.
    EvaluationCompleted {
        /// The evaluation run.
        evaluation_run_id: EvaluationRunId,
    },
    /// A candidate passes §8.8; absorbs the passing `ModelValidationCompleted`.
    CandidateValidated {
        /// The candidate.
        model_version_id: ModelVersionId,
        /// The validation run.
        evaluation_run_id: EvaluationRunId,
    },
    /// A candidate fails §8.8; absorbs the failing `ModelValidationCompleted`.
    CandidateRejected {
        /// The candidate.
        model_version_id: ModelVersionId,
        /// The validation run.
        evaluation_run_id: EvaluationRunId,
    },
    /// A production pointer moves to a new version.
    ModelPromoted {
        /// The family whose pointer moved.
        model_family: ModelFamily,
        /// The version now in production.
        model_version_id: ModelVersionId,
    },
    /// A production pointer is moved back, automatically or manually (§8.7.4).
    RollbackCompleted {
        /// The snapshot restored.
        snapshot_id: SnapshotId,
    },
    /// A job reaches `FAILED` (§8.10).
    JobFailed {
        /// The job.
        job_id: JobId,
        /// Its stage.
        stage: StageId,
        /// The typed error code.
        error: EngineErrorCode,
    },
    /// `TrainingStatus` changes; carries the new value (engine-spec §8.4 "Training status").
    TrainingStatusChanged {
        /// The new status.
        status: TrainingStatus,
    },
}

impl EngineEvent {
    /// The spec's event names, in table order (engine-spec §8.4).
    pub const NAMES: [&'static str; 21] = [
        "DataFetchStarted",
        "DataFetchCompleted",
        "DataPartialSuccess",
        "DataChanged",
        "IdentityReviewRequired",
        "FeaturesBuilt",
        "ValueModelFitted",
        "Layer1CreditComputed",
        "RAPMUpdated",
        "LatentStatesUpdated",
        "FixedLagSmoothed",
        "BoostingUpdated",
        "ProjectionRunCompleted",
        "BenchmarkSnapshotImported",
        "EvaluationCompleted",
        "CandidateValidated",
        "CandidateRejected",
        "ModelPromoted",
        "RollbackCompleted",
        "JobFailed",
        "TrainingStatusChanged",
    ];

    /// The spec's name of this event (engine-spec §8.4).
    pub const fn name(&self) -> &'static str {
        match self {
            EngineEvent::DataFetchStarted { .. } => "DataFetchStarted",
            EngineEvent::DataFetchCompleted { .. } => "DataFetchCompleted",
            EngineEvent::DataPartialSuccess { .. } => "DataPartialSuccess",
            EngineEvent::DataChanged { .. } => "DataChanged",
            EngineEvent::IdentityReviewRequired { .. } => "IdentityReviewRequired",
            EngineEvent::FeaturesBuilt { .. } => "FeaturesBuilt",
            EngineEvent::ValueModelFitted { .. } => "ValueModelFitted",
            EngineEvent::Layer1CreditComputed { .. } => "Layer1CreditComputed",
            EngineEvent::RapmUpdated { .. } => "RAPMUpdated",
            EngineEvent::LatentStatesUpdated { .. } => "LatentStatesUpdated",
            EngineEvent::FixedLagSmoothed { .. } => "FixedLagSmoothed",
            EngineEvent::BoostingUpdated { .. } => "BoostingUpdated",
            EngineEvent::ProjectionRunCompleted { .. } => "ProjectionRunCompleted",
            EngineEvent::BenchmarkSnapshotImported { .. } => "BenchmarkSnapshotImported",
            EngineEvent::EvaluationCompleted { .. } => "EvaluationCompleted",
            EngineEvent::CandidateValidated { .. } => "CandidateValidated",
            EngineEvent::CandidateRejected { .. } => "CandidateRejected",
            EngineEvent::ModelPromoted { .. } => "ModelPromoted",
            EngineEvent::RollbackCompleted { .. } => "RollbackCompleted",
            EngineEvent::JobFailed { .. } => "JobFailed",
            EngineEvent::TrainingStatusChanged { .. } => "TrainingStatusChanged",
        }
    }
}

impl fmt::Display for EngineEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// An emitted event with its emission metadata (engine-spec §8.4). Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventEnvelope {
    /// When the event was emitted, after its payload was committed (§12.6).
    pub emitted_at: UtcInstant,
    /// The durable job that emitted it; `None` for an event emitted outside a job (a manual
    /// command's own bookkeeping before its first job starts).
    pub job_id: Option<JobId>,
    /// The event.
    pub event: EngineEvent,
}

/// An in-process event consumer (engine-spec §8.4 delivery target 3: "registered in-process
/// subscribers (a subscriber trait), for library consumers").
///
/// `on_event` is called synchronously on the publishing thread, once per envelope, in
/// registration order; a subscriber that needs to do more than record the envelope hands it to
/// its own worker. `Send + Sync` because the orchestration internals publish from worker threads
/// (§8.13).
pub trait EventSubscriber: Send + Sync {
    /// Receives one envelope.
    fn on_event(&self, envelope: &EventEnvelope);
}

/// The in-process fan-out of engine events to registered [`EventSubscriber`]s.
///
/// This is delivery target 3 of engine-spec §8.4 and nothing more: **delivery to the durable
/// `diagnostic_events` table and to `tracing` is P1-02's** (see the module documentation). The
/// bus holds `Arc`s so that a subscriber can be shared with its owner; it is not itself shared
/// across threads mutably (register subscribers, then hand the bus to the publisher).
#[derive(Default)]
pub struct EventBus {
    subscribers: Vec<Arc<dyn EventSubscriber>>,
}

impl EventBus {
    /// An empty bus.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a subscriber. Subscribers are delivered to in registration order.
    pub fn subscribe(&mut self, subscriber: Arc<dyn EventSubscriber>) {
        self.subscribers.push(subscriber);
    }

    /// The number of registered subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.subscribers.len()
    }

    /// Delivers one envelope to every registered subscriber, in registration order.
    pub fn publish(&self, envelope: &EventEnvelope) {
        for subscriber in &self.subscribers {
            subscriber.on_event(envelope);
        }
    }
}

impl fmt::Debug for EventBus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventBus")
            .field("subscribers", &self.subscribers.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    struct Recorder(Mutex<Vec<String>>);

    impl EventSubscriber for Recorder {
        fn on_event(&self, envelope: &EventEnvelope) {
            self.0
                .lock()
                .unwrap()
                .push(envelope.event.name().to_owned());
        }
    }

    #[test]
    fn bus_fans_out_in_registration_order() {
        let first = Arc::new(Recorder(Mutex::new(Vec::new())));
        let second = Arc::new(Recorder(Mutex::new(Vec::new())));
        let mut bus = EventBus::new();
        assert_eq!(bus.subscriber_count(), 0);
        bus.subscribe(first.clone());
        bus.subscribe(second.clone());
        assert_eq!(bus.subscriber_count(), 2);
        let envelope = EventEnvelope {
            emitted_at: UtcInstant::from_unix_seconds(1_700_000_000),
            job_id: Some(JobId(3)),
            event: EngineEvent::DataChanged {
                data_version: DataVersion(9),
            },
        };
        bus.publish(&envelope);
        bus.publish(&EventEnvelope {
            job_id: None,
            event: EngineEvent::RapmUpdated {
                model_version_id: ModelVersionId(2),
            },
            ..envelope.clone()
        });
        assert_eq!(*first.0.lock().unwrap(), ["DataChanged", "RAPMUpdated"]);
        assert_eq!(*second.0.lock().unwrap(), ["DataChanged", "RAPMUpdated"]);
        assert!(format!("{bus:?}").contains("subscribers: 2"));
    }

    #[test]
    fn names_are_the_spec_names_and_serialized_tags() {
        let mut names = EngineEvent::NAMES.to_vec();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 21);
        let event = EngineEvent::RapmUpdated {
            model_version_id: ModelVersionId(1),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.starts_with("{\"RAPMUpdated\":"), "{json}");
        assert_eq!(event.name(), "RAPMUpdated");
        assert_eq!(event.to_string(), "RAPMUpdated");
        let back: EngineEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, event);
        let failed = EngineEvent::JobFailed {
            job_id: JobId(1),
            stage: StageId::RapmUpdate,
            error: EngineErrorCode::IllConditioned,
        };
        assert!(EngineEvent::NAMES.contains(&failed.name()));
    }
}
