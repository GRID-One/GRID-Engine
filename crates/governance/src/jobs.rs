//! Durable-job policy (engine-spec §8.10; work package P1-02 D6–D9).
//!
//! The job ledger is a durable run ledger in SQLite that the invoking process executes (§8.10).
//! This module decides; `grid-persistence` executes the decision as a compare-and-set write, and
//! the `jobs` trigger of migration `0002_backbone.sql` is the backstop (D1). It holds:
//!
//! - the permitted status transitions ([`JOB_TRANSITIONS`]);
//! - readiness over upstream idempotency keys ([`readiness`]; D7);
//! - the retry, backoff, lease and shutdown policy read from the engine settings ([`JobPolicy`];
//!   D8);
//! - what a failure and a stale `RUNNING` row become ([`after_failure`], [`stale_decision`]; D8,
//!   D9);
//! - the idempotency key ([`idempotency_key`]; D6);
//! - the stages serialized globally ([`is_globally_serialized`]; §8.13).

use std::collections::BTreeMap;
use std::fmt;
use std::time::Duration;

use grid_domain::{DataVersion, JobStatus, StageId};

use crate::error::GovernanceError;

/// The six permitted job-status transitions (engine-spec §8.10; work package P1-02 D6–D9). No
/// other transition is permitted, and the `jobs` trigger of `0002_backbone.sql` admits exactly
/// these pairs.
///
/// | From | To | When |
/// |---|---|---|
/// | `PENDING` | `RUNNING` | the runner claims the job and counts an attempt |
/// | `RUNNING` | `SUCCEEDED` | the stage succeeded; its outputs commit in the same transaction (KI-NEW-Z14) |
/// | `RUNNING` | `SKIPPED_NO_NEW_DATA` | the stage itself reported no new data (D8) |
/// | `RUNNING` | `FAILED` | attempts are exhausted, or a stale job cannot restart (D8, D9); `JobFailed` is written in the same transaction |
/// | `RUNNING` | `PENDING` | a retry with persisted backoff, or the requeue of a stale restartable job (D8, D9) |
/// | `RUNNING` | `CANCELLED` | graceful shutdown of the current job (§8.10 "Cancellation"; D8) |
///
/// No `PENDING` job is ever cancelled, skipped or failed directly: a dependent behind a failed
/// upstream stays `PENDING` and is reported as blocked ([`Readiness::Blocked`]; D7).
pub const JOB_TRANSITIONS: [(JobStatus, JobStatus); 6] = [
    (JobStatus::Pending, JobStatus::Running),
    (JobStatus::Running, JobStatus::Succeeded),
    (JobStatus::Running, JobStatus::SkippedNoNewData),
    (JobStatus::Running, JobStatus::Failed),
    (JobStatus::Running, JobStatus::Pending),
    (JobStatus::Running, JobStatus::Cancelled),
];

/// Whether `from → to` is one of the [`JOB_TRANSITIONS`] (engine-spec §8.10).
pub fn job_transition_allowed(from: JobStatus, to: JobStatus) -> bool {
    JOB_TRANSITIONS.contains(&(from, to))
}

/// Whether a job status is terminal: `SUCCEEDED`, `FAILED`, `CANCELLED` and
/// `SKIPPED_NO_NEW_DATA`, the statuses no transition leaves (engine-spec §8.10). A terminal row is
/// never changed again; a `FAILED` or `CANCELLED` key re-enqueues as a new row (work package P1-02
/// D6).
pub const fn is_terminal(status: JobStatus) -> bool {
    match status {
        JobStatus::Pending | JobStatus::Running => false,
        JobStatus::Succeeded
        | JobStatus::Failed
        | JobStatus::Cancelled
        | JobStatus::SkippedNoNewData => true,
    }
}

/// The stages whose jobs are serialized globally: `MODEL_PROMOTE` and `ROLLBACK` (engine-spec
/// §8.10 "Serialization", §8.13). At most one job of these stages is `RUNNING` at a time; the
/// store's partial unique index over `job_type IN ('MODEL_PROMOTE', 'ROLLBACK')` is the backstop
/// (owner ruling R5, P1-02).
pub const GLOBALLY_SERIALIZED_STAGES: [StageId; 2] = [StageId::ModelPromote, StageId::Rollback];

/// Whether jobs of `stage` are serialized globally ([`GLOBALLY_SERIALIZED_STAGES`]).
pub fn is_globally_serialized(stage: StageId) -> bool {
    GLOBALLY_SERIALIZED_STAGES.contains(&stage)
}

/// The latest row of one upstream key of a job, as readiness sees it (work package P1-02 D7).
///
/// Edges are by idempotency key, so a re-enqueued upstream key reconnects its dependents; the
/// caller reads the latest row (the greatest `job_id`) of each key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UpstreamState {
    /// The upstream job's idempotency key ([`idempotency_key`]).
    pub key: String,
    /// The status of the latest row with that key; `None` when no row has the key, which the
    /// store's constraints should make impossible ([`BlockReason::UpstreamMissing`]).
    pub latest: Option<JobStatus>,
    /// Whether the edge accepts an upstream `SKIPPED_NO_NEW_DATA` as satisfied (engine-spec §8.10
    /// "Dependencies": "or `SKIPPED_NO_NEW_DATA` where the edge allows it").
    pub allow_skipped: bool,
}

/// Why a job cannot start until an operator or a later enqueue changes an upstream key (work
/// package P1-02 D7). **Closed** enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BlockReason {
    /// The upstream key's latest row is `FAILED`.
    UpstreamFailed,
    /// The upstream key's latest row is `CANCELLED`.
    UpstreamCancelled,
    /// The upstream key's latest row is `SKIPPED_NO_NEW_DATA` and the edge does not allow a skip.
    UpstreamSkippedOnStrictEdge,
    /// No row has the upstream key. The edge is dangling, which is an inconsistency of the store
    /// that the caller reports; it is never treated as satisfied.
    UpstreamMissing,
}

impl BlockReason {
    /// Every reason, in declaration order.
    pub const ALL: [BlockReason; 4] = [
        BlockReason::UpstreamFailed,
        BlockReason::UpstreamCancelled,
        BlockReason::UpstreamSkippedOnStrictEdge,
        BlockReason::UpstreamMissing,
    ];

    /// The reason's stable name, equal to its variant name.
    pub const fn name(self) -> &'static str {
        match self {
            BlockReason::UpstreamFailed => "UpstreamFailed",
            BlockReason::UpstreamCancelled => "UpstreamCancelled",
            BlockReason::UpstreamSkippedOnStrictEdge => "UpstreamSkippedOnStrictEdge",
            BlockReason::UpstreamMissing => "UpstreamMissing",
        }
    }
}

impl fmt::Display for BlockReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Whether a `PENDING` job may start (engine-spec §8.10 "Dependencies"; work package P1-02 D7).
///
/// This is a derived report, never a stored status: a blocked job stays `PENDING`, and no
/// [`JobStatus`] meaning changes (D7).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Readiness {
    /// Every upstream key is satisfied: its latest row is `SUCCEEDED`, or `SKIPPED_NO_NEW_DATA`
    /// on an edge that allows it. A job with no upstream is ready.
    Ready,
    /// No upstream key blocks, and at least one is still `PENDING` or `RUNNING`.
    Waiting,
    /// An upstream key blocks the job.
    Blocked {
        /// The first blocking upstream key, in input order.
        key: String,
        /// Why it blocks.
        reason: BlockReason,
    },
}

/// Readiness of a job from the latest row of each of its upstream keys (engine-spec §8.10
/// "Dependencies"; work package P1-02 D7).
///
/// | Latest row of an upstream key | Edge | Contributes |
/// |---|---|---|
/// | `SUCCEEDED` | any | satisfied |
/// | `SKIPPED_NO_NEW_DATA` | allows a skip | satisfied |
/// | `SKIPPED_NO_NEW_DATA` | strict | blocked, [`BlockReason::UpstreamSkippedOnStrictEdge`] |
/// | `PENDING`, `RUNNING` | any | waiting |
/// | `FAILED` | any | blocked, [`BlockReason::UpstreamFailed`] |
/// | `CANCELLED` | any | blocked, [`BlockReason::UpstreamCancelled`] |
/// | no row | any | blocked, [`BlockReason::UpstreamMissing`] |
///
/// Precedence is `Blocked` over `Waiting` over `Ready`. The result is deterministic: a blocked
/// result names the first blocking key in input order.
pub fn readiness(upstreams: &[UpstreamState]) -> Readiness {
    let mut waiting = false;
    for upstream in upstreams {
        let reason = match upstream.latest {
            None => Some(BlockReason::UpstreamMissing),
            Some(JobStatus::Failed) => Some(BlockReason::UpstreamFailed),
            Some(JobStatus::Cancelled) => Some(BlockReason::UpstreamCancelled),
            Some(JobStatus::SkippedNoNewData) if !upstream.allow_skipped => {
                Some(BlockReason::UpstreamSkippedOnStrictEdge)
            }
            Some(JobStatus::SkippedNoNewData) | Some(JobStatus::Succeeded) => None,
            Some(JobStatus::Pending) | Some(JobStatus::Running) => {
                waiting = true;
                None
            }
        };
        if let Some(reason) = reason {
            return Readiness::Blocked {
                key: upstream.key.clone(),
                reason,
            };
        }
    }
    if waiting {
        Readiness::Waiting
    } else {
        Readiness::Ready
    }
}

/// What a job row becomes after a failed attempt (work package P1-02 D8). **Closed** enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FailureDecision {
    /// `RUNNING → PENDING` with `not_before = now + JobPolicy::delay_before_attempt(attempt + 1)`.
    Retry,
    /// `RUNNING → FAILED`, with one `JobFailed` row in the same transaction ("Failed: attempts
    /// exhausted"; engine-spec §8.10 "Retries").
    Fail,
}

/// The decision after a failed attempt (engine-spec §8.10 "Retries"; work package P1-02 D8).
///
/// `attempt_count` is the number of attempts made, the failed one included (attempts are counted
/// at claim); `max_attempts` is the limit recorded on the job row at enqueue (D4). Every failure
/// retries until the limit: `Retry` when `attempt_count < max_attempts`, otherwise `Fail`.
pub const fn after_failure(attempt_count: u32, max_attempts: u32) -> FailureDecision {
    if attempt_count < max_attempts {
        FailureDecision::Retry
    } else {
        FailureDecision::Fail
    }
}

/// What a stale `RUNNING` row becomes at writer start-up (engine-spec §8.10 "Stale `RUNNING`";
/// owner ruling R5; work package P1-02 D9). **Closed** enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StaleDecision {
    /// The job can restart from durable inputs: `RUNNING → PENDING` with `not_before = now`. The
    /// crashed attempt counts.
    Requeue,
    /// The job cannot restart: `RUNNING → FAILED` with a `JobFailed` row, then a new `PENDING` row
    /// under the same idempotency key (§8.10: "marked `FAILED` and re-enqueued under the same
    /// key").
    ///
    /// The attempt bound of [`stale_decision`] is per row. This policy does not carry a count
    /// from the failed row to the new one, so a job that cannot restart and is lost on every
    /// attempt is failed and re-enqueued once per recovery, and never reaches [`Self::Fail`].
    FailAndReenqueue,
    /// The attempts are exhausted: `RUNNING → FAILED` with a `JobFailed` row, and nothing is
    /// re-enqueued.
    Fail,
}

/// The decision for a `RUNNING` row whose lease has expired (engine-spec §8.10 "Stale
/// `RUNNING`"; work package P1-02 D9). The engine never probes the owner process (owner ruling
/// R5); staleness is the caller's lease comparison.
///
/// | Attempts | `restartable` | Decision |
/// |---|---|---|
/// | `attempt_count >= max_attempts` | any | [`StaleDecision::Fail`] |
/// | `attempt_count < max_attempts` | `true` | [`StaleDecision::Requeue`] |
/// | `attempt_count < max_attempts` | `false` | [`StaleDecision::FailAndReenqueue`] |
pub const fn stale_decision(
    attempt_count: u32,
    max_attempts: u32,
    restartable: bool,
) -> StaleDecision {
    if attempt_count >= max_attempts {
        StaleDecision::Fail
    } else if restartable {
        StaleDecision::Requeue
    } else {
        StaleDecision::FailAndReenqueue
    }
}

/// The idempotency key of a job: `<STAGE>:<input data version or ->:<config hash>` (engine-spec
/// §8.10 "`idempotency_key` = `(job_type, input_data_version, config_hash)`"; work package P1-02
/// D6).
///
/// `<STAGE>` is [`StageId::spec_name`], the data version is written in decimal and `-` stands for
/// a job without an input data version, and the configuration hash is the sha256 of the job's
/// canonical configuration JSON as 64 lowercase hexadecimal characters (D6). For example
/// `PREDICT:42:<64 hex>` or `DATA_FETCH:-:<64 hex>`.
///
/// # Errors
///
/// [`GovernanceError::InvalidKey`] when `config_hash_hex` is not exactly 64 characters of
/// `0-9a-f`. Nothing is normalized: an upper-case or short hash is refused, never lower-cased or
/// padded.
pub fn idempotency_key(
    stage: StageId,
    input_data_version: Option<DataVersion>,
    config_hash_hex: &str,
) -> Result<String, GovernanceError> {
    let lowercase_hex = config_hash_hex
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if config_hash_hex.len() != 64 || !lowercase_hex {
        return Err(GovernanceError::InvalidKey(format!(
            "the config hash must be 64 lowercase hexadecimal characters; got {} bytes{}",
            config_hash_hex.len(),
            if lowercase_hex {
                ""
            } else {
                " with a character outside 0-9a-f"
            }
        )));
    }
    let version = match input_data_version {
        Some(version) => version.to_string(),
        None => "-".to_owned(),
    };
    Ok(format!("{}:{version}:{config_hash_hex}", stage.spec_name()))
}

/// The retry, backoff, lease and shutdown policy of the job runner, read from the engine settings
/// (engine-spec §8.10 "Retries": "the limits are engine settings fixed in P1-02"; work package
/// P1-02 D8, D9).
///
/// Build it with [`JobPolicy::from_settings`]; there is no `Default`, because the defaults live
/// in the store (migration `0002_backbone.sql` seeds them, D8) and a missing setting is a typed
/// error, never a value supplied here (DR-B6). A value built by hand is not checked against the
/// bounds of [`JobPolicy::from_settings`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JobPolicy {
    /// `jobs.max_attempts`: attempts per job row, the first included; recorded on the row at
    /// enqueue (D4). A count, `1..=100`. Seeded as 3.
    pub max_attempts: u32,
    /// `jobs.backoff_initial_secs`: the delay before the second attempt. Whole seconds,
    /// `0..=3600`. Seeded as 5 s.
    pub backoff_initial: Duration,
    /// `jobs.backoff_max_secs`: the cap of every retry delay. Whole seconds, at least
    /// `backoff_initial` and at most 86 400. Seeded as 60 s.
    pub backoff_max: Duration,
    /// `jobs.lease_secs`: how long a `RUNNING` row's lease lasts after a heartbeat; a row whose
    /// lease has expired is stale (D9). Whole seconds, `3..=86400`: `heartbeat` is at least 1 s and
    /// `2 × heartbeat < lease`, so no shorter lease admits a heartbeat. Seeded as 300 s.
    pub lease: Duration,
    /// `jobs.heartbeat_secs`: how often a running job extends its lease. Whole seconds, at least
    /// 1, with `2 × heartbeat < lease`. Seeded as 30 s.
    pub heartbeat: Duration,
    /// `engine.shutdown_grace_secs`: how long a graceful shutdown waits for the current job to
    /// reach a durable boundary before it is marked `CANCELLED` (§8.10 "Cancellation", §8.11).
    /// Whole seconds, `0..=3600`. Seeded as 30 s.
    pub shutdown_grace: Duration,
}

/// The upper bound of `jobs.max_attempts`.
const MAX_ATTEMPTS_LIMIT: u64 = 100;
/// The upper bound of `jobs.backoff_initial_secs` and `engine.shutdown_grace_secs`: one hour.
const ONE_HOUR_SECS: u64 = 3_600;
/// The upper bound of `jobs.backoff_max_secs` and `jobs.lease_secs`: one day.
const ONE_DAY_SECS: u64 = 86_400;
/// The lower bound of `jobs.lease_secs`.
const MIN_LEASE_SECS: u64 = 3;
/// How many characters of a setting key or value an error message shows.
const SHOWN_CHARS: usize = 64;

impl JobPolicy {
    /// The six settings keys the policy reads, in the order [`JobPolicy::from_settings`] checks
    /// them (work package P1-02 D8).
    pub const KEYS: [&'static str; 6] = [
        "jobs.max_attempts",
        "jobs.backoff_initial_secs",
        "jobs.backoff_max_secs",
        "jobs.lease_secs",
        "jobs.heartbeat_secs",
        "engine.shutdown_grace_secs",
    ];

    /// The settings namespaces the policy validates. A key's namespace is the text before its
    /// first `.`, or the whole key when it has none; the comparison is exact and case-sensitive.
    ///
    /// The policy owns both namespaces whole: an unknown key inside them is refused as
    /// `StateCorrupt`. A later setting in `jobs.*` or `engine.*` must therefore be added to
    /// [`JobPolicy::KEYS`], or every [`JobPolicy::from_settings`] refuses the store that holds it.
    pub const NAMESPACES: [&'static str; 2] = ["jobs", "engine"];

    /// Reads the policy from the effective engine settings: the latest value of each key, as
    /// text (work package P1-02 D8).
    ///
    /// Every key in a namespace of [`JobPolicy::NAMESPACES`] must be one of [`JobPolicy::KEYS`],
    /// and every key of [`JobPolicy::KEYS`] must be present. Keys of other namespaces are
    /// ignored. A value is an unsigned decimal integer: ASCII digits only, with no sign, space,
    /// fraction or exponent. The bounds are:
    ///
    /// | Key | Bounds |
    /// |---|---|
    /// | `jobs.max_attempts` | `1..=100` |
    /// | `jobs.backoff_initial_secs` | `0..=3600` |
    /// | `jobs.backoff_max_secs` | at least `jobs.backoff_initial_secs`, at most 86 400 |
    /// | `jobs.lease_secs` | `3..=86400` |
    /// | `jobs.heartbeat_secs` | at least 1, with `2 × heartbeat < lease` |
    /// | `engine.shutdown_grace_secs` | `0..=3600` |
    ///
    /// The checks run in a fixed order, so the error is deterministic: unknown keys first, in key
    /// order, then each key of [`JobPolicy::KEYS`] in turn (missing, malformed, out of bounds).
    ///
    /// # Errors
    ///
    /// [`GovernanceError::InvalidSetting`], naming the key, for an unknown key in a validated
    /// namespace, a missing key, a malformed value or a value out of bounds. It maps to
    /// `StateCorrupt` ([`GovernanceError::code`]); nothing is replaced by a default.
    pub fn from_settings(settings: &BTreeMap<String, String>) -> Result<Self, GovernanceError> {
        if let Some(unknown) = settings.keys().find(|key| {
            Self::NAMESPACES.contains(&namespace(key)) && !Self::KEYS.contains(&key.as_str())
        }) {
            return Err(GovernanceError::InvalidSetting(format!(
                "unknown key {} in a validated namespace (jobs, engine)",
                shown(unknown)
            )));
        }

        let [max_attempts_key, initial_key, max_key, lease_key, heartbeat_key, grace_key] =
            Self::KEYS;

        let max_attempts = read(settings, max_attempts_key)?;
        within(max_attempts_key, max_attempts, 1, MAX_ATTEMPTS_LIMIT)?;
        let max_attempts = u32::try_from(max_attempts)
            .map_err(|_| out_of_bounds(max_attempts_key, max_attempts, "1..=100"))?;

        let backoff_initial = read(settings, initial_key)?;
        within(initial_key, backoff_initial, 0, ONE_HOUR_SECS)?;

        let backoff_max = read(settings, max_key)?;
        if backoff_max < backoff_initial || backoff_max > ONE_DAY_SECS {
            return Err(out_of_bounds(
                max_key,
                backoff_max,
                &format!("{backoff_initial}..={ONE_DAY_SECS} (at least {initial_key})"),
            ));
        }

        let lease = read(settings, lease_key)?;
        within(lease_key, lease, MIN_LEASE_SECS, ONE_DAY_SECS)?;

        let heartbeat = read(settings, heartbeat_key)?;
        if heartbeat == 0 || heartbeat.saturating_mul(2) >= lease {
            return Err(out_of_bounds(
                heartbeat_key,
                heartbeat,
                &format!("at least 1, with 2 x {heartbeat_key} < {lease_key} = {lease}"),
            ));
        }

        let shutdown_grace = read(settings, grace_key)?;
        within(grace_key, shutdown_grace, 0, ONE_HOUR_SECS)?;

        Ok(JobPolicy {
            max_attempts,
            backoff_initial: Duration::from_secs(backoff_initial),
            backoff_max: Duration::from_secs(backoff_max),
            lease: Duration::from_secs(lease),
            heartbeat: Duration::from_secs(heartbeat),
            shutdown_grace: Duration::from_secs(shutdown_grace),
        })
    }

    /// The delay before attempt `n` of a job, counting from 1 (work package P1-02 D8).
    ///
    /// Attempt 1 starts at once: `n <= 1` gives zero. For `n >= 2` the delay is
    /// `min(backoff_initial × 2^(n−2), backoff_max)`, saturating: a product too large to
    /// represent is `backoff_max`. With the seeded settings, attempts 2 and 3 wait 5 s and 10 s.
    pub fn delay_before_attempt(&self, n: u32) -> Duration {
        if n <= 1 || self.backoff_initial.is_zero() {
            return Duration::ZERO;
        }
        2u32.checked_pow(n - 2)
            .and_then(|factor| self.backoff_initial.checked_mul(factor))
            .map_or(self.backoff_max, |delay| delay.min(self.backoff_max))
    }
}

/// The namespace of a settings key: the text before its first `.`, or the whole key.
fn namespace(key: &str) -> &str {
    key.split_once('.').map_or(key, |(namespace, _)| namespace)
}

/// Text from the store, escaped, quoted and cut to [`SHOWN_CHARS`] characters for a message.
fn shown(text: &str) -> String {
    let cut: String = text.chars().take(SHOWN_CHARS).collect();
    if cut.len() < text.len() {
        format!("{cut:?}...")
    } else {
        format!("{cut:?}")
    }
}

/// Reads one setting as an unsigned decimal integer.
fn read(settings: &BTreeMap<String, String>, key: &str) -> Result<u64, GovernanceError> {
    let Some(text) = settings.get(key) else {
        return Err(GovernanceError::InvalidSetting(format!(
            "`{key}` is missing"
        )));
    };
    let digits_only = !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit());
    match text.parse::<u64>() {
        Ok(value) if digits_only => Ok(value),
        _ => Err(GovernanceError::InvalidSetting(format!(
            "`{key}` = {} is not an unsigned decimal integer",
            shown(text)
        ))),
    }
}

/// Checks `low <= value <= high`.
fn within(key: &str, value: u64, low: u64, high: u64) -> Result<(), GovernanceError> {
    if (low..=high).contains(&value) {
        Ok(())
    } else {
        Err(out_of_bounds(key, value, &format!("{low}..={high}")))
    }
}

/// The error of a value outside its bounds.
fn out_of_bounds(key: &str, value: u64, bounds: &str) -> GovernanceError {
    GovernanceError::InvalidSetting(format!("`{key}` = {value} is outside {bounds}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use grid_domain::EngineErrorCode;

    use JobStatus as J;

    /// The settings migration `0002_backbone.sql` seeds (work package P1-02 D8).
    fn seeded() -> BTreeMap<String, String> {
        [
            ("jobs.max_attempts", "3"),
            ("jobs.backoff_initial_secs", "5"),
            ("jobs.backoff_max_secs", "60"),
            ("jobs.lease_secs", "300"),
            ("jobs.heartbeat_secs", "30"),
            ("engine.shutdown_grace_secs", "30"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
    }

    fn with(key: &str, value: &str) -> BTreeMap<String, String> {
        let mut settings = seeded();
        settings.insert(key.to_owned(), value.to_owned());
        settings
    }

    fn setting_error(settings: &BTreeMap<String, String>) -> String {
        match JobPolicy::from_settings(settings) {
            Err(error @ GovernanceError::InvalidSetting(_)) => {
                assert_eq!(error.code(), EngineErrorCode::StateCorrupt);
                error.to_string()
            }
            other => panic!("expected InvalidSetting, got {other:?}"),
        }
    }

    fn policy(initial: u64, max: u64) -> JobPolicy {
        JobPolicy {
            max_attempts: 3,
            backoff_initial: Duration::from_secs(initial),
            backoff_max: Duration::from_secs(max),
            lease: Duration::from_secs(300),
            heartbeat: Duration::from_secs(30),
            shutdown_grace: Duration::from_secs(30),
        }
    }

    #[test]
    fn the_transition_table_is_exact_over_all_36_pairs() {
        let expected = [
            (J::Pending, J::Running),
            (J::Running, J::Succeeded),
            (J::Running, J::SkippedNoNewData),
            (J::Running, J::Failed),
            (J::Running, J::Pending),
            (J::Running, J::Cancelled),
        ];
        assert_eq!(JOB_TRANSITIONS, expected);
        let mut checked = 0;
        let mut allowed = 0;
        for from in JobStatus::ALL {
            for to in JobStatus::ALL {
                let listed = expected.contains(&(from, to));
                assert_eq!(job_transition_allowed(from, to), listed, "{from} -> {to}");
                checked += 1;
                allowed += usize::from(listed);
            }
        }
        assert_eq!((checked, allowed), (36, 6));
    }

    #[test]
    fn terminal_statuses_are_exactly_those_no_transition_leaves() {
        for status in JobStatus::ALL {
            let has_exit = JOB_TRANSITIONS.iter().any(|(from, _)| *from == status);
            assert_eq!(is_terminal(status), !has_exit, "{status}");
        }
        assert!(!is_terminal(J::Pending));
        assert!(!is_terminal(J::Running));
        assert!(is_terminal(J::Succeeded));
        assert!(is_terminal(J::Failed));
        assert!(is_terminal(J::Cancelled));
        assert!(is_terminal(J::SkippedNoNewData));
        // No transition leads from a terminal status, and none skips, cancels or fails a PENDING
        // job directly (D7, D8).
        for (from, to) in JOB_TRANSITIONS {
            assert!(!is_terminal(from));
            if from == J::Pending {
                assert_eq!(to, J::Running);
            }
        }
    }

    fn upstream(key: &str, latest: Option<JobStatus>, allow_skipped: bool) -> UpstreamState {
        UpstreamState {
            key: key.to_owned(),
            latest,
            allow_skipped,
        }
    }

    #[test]
    fn readiness_over_every_status_and_edge_kind() {
        let blocked = |reason| Readiness::Blocked {
            key: "up".to_owned(),
            reason,
        };
        let cases = [
            (Some(J::Pending), false, Readiness::Waiting),
            (Some(J::Pending), true, Readiness::Waiting),
            (Some(J::Running), false, Readiness::Waiting),
            (Some(J::Running), true, Readiness::Waiting),
            (Some(J::Succeeded), false, Readiness::Ready),
            (Some(J::Succeeded), true, Readiness::Ready),
            (Some(J::Failed), false, blocked(BlockReason::UpstreamFailed)),
            (Some(J::Failed), true, blocked(BlockReason::UpstreamFailed)),
            (
                Some(J::Cancelled),
                false,
                blocked(BlockReason::UpstreamCancelled),
            ),
            (
                Some(J::Cancelled),
                true,
                blocked(BlockReason::UpstreamCancelled),
            ),
            (
                Some(J::SkippedNoNewData),
                false,
                blocked(BlockReason::UpstreamSkippedOnStrictEdge),
            ),
            (Some(J::SkippedNoNewData), true, Readiness::Ready),
            (None, false, blocked(BlockReason::UpstreamMissing)),
            (None, true, blocked(BlockReason::UpstreamMissing)),
        ];
        assert_eq!(cases.len(), (JobStatus::ALL.len() + 1) * 2);
        for (latest, allow_skipped, expected) in cases {
            assert_eq!(
                readiness(&[upstream("up", latest, allow_skipped)]),
                expected,
                "{latest:?}, allow_skipped = {allow_skipped}"
            );
        }
        assert_eq!(readiness(&[]), Readiness::Ready, "no upstream");
    }

    #[test]
    fn readiness_precedence_is_blocked_then_waiting_then_ready() {
        let done = upstream("a", Some(J::Succeeded), false);
        let skipped_ok = upstream("b", Some(J::SkippedNoNewData), true);
        let running = upstream("c", Some(J::Running), false);
        let failed = upstream("d", Some(J::Failed), false);
        let missing = upstream("e", None, true);

        assert_eq!(
            readiness(&[done.clone(), skipped_ok.clone()]),
            Readiness::Ready
        );
        assert_eq!(
            readiness(&[done.clone(), running.clone(), skipped_ok.clone()]),
            Readiness::Waiting
        );
        // A blocking key after a waiting one still blocks.
        assert_eq!(
            readiness(&[running.clone(), failed.clone()]),
            Readiness::Blocked {
                key: "d".to_owned(),
                reason: BlockReason::UpstreamFailed
            }
        );
        // The first blocking key in input order is named.
        assert_eq!(
            readiness(&[done.clone(), missing.clone(), failed.clone()]),
            Readiness::Blocked {
                key: "e".to_owned(),
                reason: BlockReason::UpstreamMissing
            }
        );
        assert_eq!(
            readiness(&[failed, missing, running, done]),
            Readiness::Blocked {
                key: "d".to_owned(),
                reason: BlockReason::UpstreamFailed
            }
        );
        assert_eq!(BlockReason::ALL.len(), 4);
        assert_eq!(
            BlockReason::UpstreamSkippedOnStrictEdge.to_string(),
            "UpstreamSkippedOnStrictEdge"
        );
    }

    #[test]
    fn backoff_doubles_from_the_second_attempt_and_is_capped() {
        let p = policy(5, 60);
        let secs = |n| p.delay_before_attempt(n).as_secs();
        assert_eq!(p.delay_before_attempt(0), Duration::ZERO);
        assert_eq!(p.delay_before_attempt(1), Duration::ZERO);
        assert_eq!(
            [secs(2), secs(3), secs(4), secs(5), secs(6), secs(7)],
            [5, 10, 20, 40, 60, 60],
            "min(5 s x 2^(n-2), 60 s)"
        );
        // Saturation: 2^(n-2) does not fit in u32, or the product does not fit in a Duration.
        assert_eq!(secs(34), 60);
        assert_eq!(secs(u32::MAX), 60);
        let huge = JobPolicy {
            backoff_initial: Duration::MAX,
            backoff_max: Duration::MAX,
            ..policy(0, 0)
        };
        assert_eq!(huge.delay_before_attempt(2), Duration::MAX);
        assert_eq!(huge.delay_before_attempt(3), Duration::MAX);
        // A zero initial delay is zero for every attempt, however large.
        let zero = policy(0, 60);
        for n in [0, 1, 2, 3, 40, u32::MAX] {
            assert_eq!(zero.delay_before_attempt(n), Duration::ZERO, "{n}");
        }
        // Equal initial and cap: every retry waits the cap.
        let flat = policy(3600, 3600);
        assert_eq!(flat.delay_before_attempt(2).as_secs(), 3600);
        assert_eq!(flat.delay_before_attempt(9).as_secs(), 3600);
    }

    #[test]
    fn from_settings_reads_the_seeded_defaults_and_ignores_other_namespaces() {
        let mut settings = seeded();
        assert_eq!(JobPolicy::from_settings(&settings), Ok(policy(5, 60)));
        for (key, value) in [
            ("ingestion.nflverse.enabled", "yes"),
            ("jobsx.max_attempts", "not a number"),
            ("JOBS.max_attempts", "-1"),
            ("engines.shutdown_grace_secs", ""),
            ("", "empty key"),
            (".jobs.lease_secs", "0"),
        ] {
            settings.insert(key.to_owned(), value.to_owned());
        }
        assert_eq!(JobPolicy::from_settings(&settings), Ok(policy(5, 60)));
        // Leading zeros are a decimal integer.
        let p = JobPolicy::from_settings(&with("jobs.max_attempts", "007")).unwrap();
        assert_eq!(p.max_attempts, 7);
        assert_eq!(JobPolicy::KEYS.len(), 6);
        for key in JobPolicy::KEYS {
            assert!(JobPolicy::NAMESPACES.contains(&namespace(key)), "{key}");
        }
    }

    #[test]
    fn from_settings_refuses_a_missing_key() {
        for key in JobPolicy::KEYS {
            let mut settings = seeded();
            settings.remove(key);
            let text = setting_error(&settings);
            assert_eq!(text, format!("invalid engine setting: `{key}` is missing"));
        }
        assert_eq!(
            setting_error(&BTreeMap::new()),
            "invalid engine setting: `jobs.max_attempts` is missing"
        );
    }

    #[test]
    fn from_settings_refuses_an_unknown_key_in_its_namespaces() {
        for key in [
            "jobs.max_attempt",
            "jobs.lease",
            "jobs.",
            "jobs",
            "engine",
            "engine.shutdown_grace",
            "engine.max_attempts",
        ] {
            let text = setting_error(&with(key, "3"));
            assert!(text.contains("unknown key"), "{key}: {text}");
            assert!(text.contains(&format!("{key:?}")), "{key}: {text}");
        }
        // Store text is escaped and cut in the message.
        let long = format!("jobs.\n{}", "x".repeat(200));
        let text = setting_error(&with(&long, "3"));
        assert!(!text.contains('\n'), "{text}");
        assert!(text.contains("\"jobs.\\nxxx"), "{text}");
        assert!(text.len() < 200, "{text}");
    }

    #[test]
    fn from_settings_refuses_a_malformed_value() {
        for key in JobPolicy::KEYS {
            for value in [
                "",
                " 3",
                "3 ",
                "+3",
                "-3",
                "3.0",
                "1e2",
                "0x10",
                "three",
                "\u{0663}",
                "18446744073709551616",
            ] {
                let text = setting_error(&with(key, value));
                assert!(
                    text.contains(&format!("`{key}`"))
                        && text.ends_with("is not an unsigned decimal integer"),
                    "{key} = {value:?}: {text}"
                );
            }
        }
    }

    #[test]
    fn from_settings_enforces_the_bounds() {
        let accepted = [
            vec![("jobs.max_attempts", "1")],
            vec![("jobs.max_attempts", "100")],
            vec![("jobs.backoff_initial_secs", "0")],
            vec![
                ("jobs.backoff_initial_secs", "3600"),
                ("jobs.backoff_max_secs", "3600"),
            ],
            vec![("jobs.backoff_max_secs", "5")],
            vec![("jobs.backoff_max_secs", "86400")],
            vec![("jobs.lease_secs", "86400")],
            vec![("jobs.lease_secs", "3"), ("jobs.heartbeat_secs", "1")],
            vec![("jobs.heartbeat_secs", "149")],
            vec![("engine.shutdown_grace_secs", "0")],
            vec![("engine.shutdown_grace_secs", "3600")],
        ];
        for overrides in accepted {
            let mut settings = seeded();
            for (key, value) in &overrides {
                settings.insert((*key).to_owned(), (*value).to_owned());
            }
            assert!(JobPolicy::from_settings(&settings).is_ok(), "{overrides:?}");
        }
        let refused = [
            (vec![("jobs.max_attempts", "0")], "jobs.max_attempts"),
            (vec![("jobs.max_attempts", "101")], "jobs.max_attempts"),
            (
                vec![("jobs.max_attempts", "4294967297")],
                "jobs.max_attempts",
            ),
            (
                vec![
                    ("jobs.backoff_initial_secs", "3601"),
                    ("jobs.backoff_max_secs", "86400"),
                ],
                "jobs.backoff_initial_secs",
            ),
            (
                vec![("jobs.backoff_max_secs", "4")],
                "jobs.backoff_max_secs",
            ),
            (
                vec![("jobs.backoff_max_secs", "86401")],
                "jobs.backoff_max_secs",
            ),
            (vec![("jobs.lease_secs", "2")], "jobs.lease_secs"),
            (vec![("jobs.lease_secs", "86401")], "jobs.lease_secs"),
            // A lease that admits no heartbeat of this length: 2 x 2 < 4 is false.
            (
                vec![("jobs.lease_secs", "4"), ("jobs.heartbeat_secs", "2")],
                "jobs.heartbeat_secs",
            ),
            (vec![("jobs.heartbeat_secs", "0")], "jobs.heartbeat_secs"),
            (vec![("jobs.heartbeat_secs", "150")], "jobs.heartbeat_secs"),
            (
                vec![("jobs.heartbeat_secs", "18446744073709551615")],
                "jobs.heartbeat_secs",
            ),
            (
                vec![("engine.shutdown_grace_secs", "3601")],
                "engine.shutdown_grace_secs",
            ),
        ];
        for (overrides, key) in refused {
            let mut settings = seeded();
            for (k, value) in &overrides {
                settings.insert((*k).to_owned(), (*value).to_owned());
            }
            let text = setting_error(&settings);
            assert!(
                text.contains(&format!("`{key}`")) && text.contains("is outside"),
                "{overrides:?}: {text}"
            );
        }
    }

    #[test]
    fn after_failure_retries_until_the_limit() {
        let cases = [
            (0, 3, FailureDecision::Retry),
            (1, 3, FailureDecision::Retry),
            (2, 3, FailureDecision::Retry),
            (3, 3, FailureDecision::Fail),
            (4, 3, FailureDecision::Fail),
            (1, 1, FailureDecision::Fail),
            (0, 0, FailureDecision::Fail),
            (u32::MAX, u32::MAX, FailureDecision::Fail),
            (u32::MAX - 1, u32::MAX, FailureDecision::Retry),
        ];
        for (attempts, max, expected) in cases {
            assert_eq!(after_failure(attempts, max), expected, "{attempts}/{max}");
        }
    }

    #[test]
    fn stale_decision_table() {
        let cases = [
            (1, 3, true, StaleDecision::Requeue),
            (1, 3, false, StaleDecision::FailAndReenqueue),
            (2, 3, true, StaleDecision::Requeue),
            (2, 3, false, StaleDecision::FailAndReenqueue),
            (3, 3, true, StaleDecision::Fail),
            (3, 3, false, StaleDecision::Fail),
            (4, 3, true, StaleDecision::Fail),
            (1, 1, true, StaleDecision::Fail),
            (1, 1, false, StaleDecision::Fail),
            (0, 0, true, StaleDecision::Fail),
        ];
        for (attempts, max, restartable, expected) in cases {
            assert_eq!(
                stale_decision(attempts, max, restartable),
                expected,
                "{attempts}/{max}, restartable = {restartable}"
            );
        }
    }

    #[test]
    fn idempotency_key_has_the_d6_format() {
        let hash = "0123456789abcdef".repeat(4);
        assert_eq!(
            idempotency_key(StageId::Predict, Some(DataVersion(42)), &hash),
            Ok(format!("PREDICT:42:{hash}"))
        );
        assert_eq!(
            idempotency_key(StageId::DataFetch, None, &hash),
            Ok(format!("DATA_FETCH:-:{hash}"))
        );
        assert_eq!(
            idempotency_key(
                StageId::EmpiricalBayesUpdate,
                Some(DataVersion(u64::MAX)),
                &hash
            ),
            Ok(format!(
                "EMPIRICAL_BAYES_UPDATE:18446744073709551615:{hash}"
            ))
        );
        // Every stage gives a distinct key for the same inputs.
        let keys: std::collections::BTreeSet<String> = StageId::ALL
            .iter()
            .map(|stage| idempotency_key(*stage, Some(DataVersion(1)), &hash).unwrap())
            .collect();
        assert_eq!(keys.len(), StageId::ALL.len());
    }

    #[test]
    fn idempotency_key_refuses_a_malformed_hash() {
        let good = "ab".repeat(32);
        for bad in [
            String::new(),
            "ab".repeat(31),
            format!("{good}0"),
            "AB".repeat(32),
            format!("{}g", &good[..63]),
            format!("{} ", &good[..63]),
            format!("{}\u{e9}", &good[..62]),
        ] {
            match idempotency_key(StageId::Snapshot, None, &bad) {
                Err(error @ GovernanceError::InvalidKey(_)) => {
                    assert_eq!(error.code(), EngineErrorCode::Internal);
                    assert!(!error.to_string().contains(&bad) || bad.is_empty());
                }
                other => panic!("{bad:?}: expected InvalidKey, got {other:?}"),
            }
        }
    }

    #[test]
    fn publication_stages_are_serialized_globally() {
        for stage in StageId::ALL {
            assert_eq!(
                is_globally_serialized(stage),
                matches!(stage, StageId::ModelPromote | StageId::Rollback),
                "{stage}"
            );
        }
    }
}
