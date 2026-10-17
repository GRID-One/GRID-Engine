//! The player-week projection record and its groups (engine-output-contract §2, §3; engine-spec
//! §5.1–§5.5).

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::asof::{Lock, ResearchFlag};
use crate::error::{OutputInvariant, OutputInvariantError};
use crate::ids::{
    ContractVersion, DataVersion, EngineVersion, FeatureSchemaVersion, GameId, ModelVersionId,
    PlayerKey, PredictionVersionId, ScoringProfileRef, Season, TeamKey, Week,
};
use crate::output::explanation::ExplanationRecord;
use crate::output::{Horizon, Position};
use crate::plays::SourceSnapshot;
use crate::time::UtcInstant;

/// The key group of a projection record (engine-output-contract §2). Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectionKey {
    /// The target season.
    pub season: Season,
    /// The target week (1–18 are produced; never 0).
    pub week: Week,
    /// True for week 18, which is evaluated separately (engine-spec §2.2).
    pub is_week_18: bool,
    /// The typed horizon.
    pub horizon: Horizon,
    /// The canonical player key.
    pub player_id: PlayerKey,
    /// The player's position.
    pub position: Position,
    /// The player's team from the schedule snapshot in force at the lock.
    pub team: TeamKey,
    /// The opponent from the same snapshot.
    pub opponent: TeamKey,
    /// The game.
    pub game_id: GameId,
    /// Whether `team` is the home team.
    pub is_home: bool,
    /// True when the run's data snapshot is synthetic, so that `player_id` is a synthetic id.
    pub synthetic_ids: bool,
}

/// The lineage group (engine-output-contract §2; engine-spec §8.9, §9.4). Every projection
/// traces to its data, feature, model, scoring and engine versions. Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectionLineage {
    /// Unique per emitted projection set; a post-lock re-projection is a new version.
    pub prediction_version_id: PredictionVersionId,
    /// The committed data version the run read.
    pub data_snapshot_id: DataVersion,
    /// The feature schema the model was trained on.
    pub feature_schema_version: FeatureSchemaVersion,
    /// The production model version that produced the record.
    pub model_version_id: ModelVersionId,
    /// The engine build: crate version plus git SHA.
    pub engine_version: EngineVersion,
    /// The scoring profile of the headline fantasy distribution.
    pub scoring_profile: ScoringProfileRef,
    /// The Layer-F simulation seed, deterministic per prediction version.
    pub seed: u64,
    /// The Layer-F draw count.
    pub n_draws: u32,
    /// The lock the projection was made at.
    pub lock: Lock,
    /// Wall-clock time of computation. Metadata, not an input.
    pub computed_at: UtcInstant,
    /// The research-only marker of the producing run (evaluation-and-leakage.md §4.1; engine-spec
    /// §4.5). `research_only = false` with no relaxed rule for a `Backtest` or `Production` run;
    /// `research_only = true` naming the relaxed rule for a `Research` run, whose output MUST NOT
    /// feed a gate, a promotion or a published claim. Never null; a flag that fails
    /// [`ResearchFlag::is_consistent`] is a producer defect.
    pub research: ResearchFlag,
}

/// The per-projection source lineage record (engine-output-contract §2): which source snapshots,
/// with which publication timestamps, fed a prediction version. Queryable separately, keyed by
/// `prediction_version_id`. Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceLineageRecord {
    /// The prediction version.
    pub prediction_version_id: PredictionVersionId,
    /// The source snapshots that fed it.
    pub sources: Vec<SourceSnapshot>,
}

/// Where the availability inputs came from (engine-output-contract §3.3). `Missing` means no
/// availability data: it widens uncertainty and MUST NOT be read as healthy. **Closed** enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AvailabilitySource {
    /// A timestamped operator override.
    Override,
    /// A provider injury or status report.
    Provider,
    /// The Layer-A model with no report.
    Model,
    /// No availability data.
    Missing,
}

impl fmt::Display for AvailabilitySource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            AvailabilitySource::Override => "Override",
            AvailabilitySource::Provider => "Provider",
            AvailabilitySource::Model => "Model",
            AvailabilitySource::Missing => "Missing",
        })
    }
}

/// The Layer-A availability block (engine-output-contract §3.3; engine-spec §6.1). Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AvailabilityBlock {
    /// Probability the player is active. Units: probability in `[0, 1]`. A known `OUT`, `IR`,
    /// `PUP`, suspension, bye or inactive state forces `0`.
    pub p_active: f64,
    /// Probability the player starts. QB only: `None` for every other position, by the stat
    /// vector's intended asymmetry (DR-D7), never `0`.
    pub p_start: Option<f64>,
    /// Expected snap multiplier if active. Units: ratio, `≥ 0`; `1` means a full role.
    pub snap_multiplier_if_active: f64,
    /// Probability of a materially limited role. Units: probability in `[0, 1]`.
    pub p_limited_role: f64,
    /// Where the inputs came from.
    pub availability_source: AvailabilitySource,
    /// When the availability input was observed. `None` only when `availability_source` is
    /// `Missing`.
    pub availability_observed_at: Option<UtcInstant>,
    /// True when a timestamped operator override applied; part of the prediction snapshot.
    pub manual_override: bool,
}

/// The unit of a stat component (engine-output-contract §3.2 "Unit and support"). **Closed**
/// enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StatUnit {
    /// A probability in `[0, 1]`.
    Probability,
    /// A fraction of the team's offensive snaps, in `[0, 1]`.
    Share,
    /// A per-game count, `≥ 0`.
    Count,
    /// Yards per game.
    Yards,
}

/// The stat-vector components (engine-output-contract §3.2; engine-spec §5.1). Eighteen values;
/// which apply to a position follows the §3.2 table ([`StatComponent::applies_to`]), with the
/// intended asymmetry of DR-D7: QB has no snap share and no receiving components; RB, WR and TE
/// have no start probability. Definitions follow the official nflverse weekly statistics
/// (DR-C12). **Closed** enum. Never null.
///
/// Serialized as the contract's `snake_case` component name (`active_probability`, …,
/// `two_point_conversions`), the same string [`StatComponent::contract_name`] returns: the
/// serialized form is the contract's spelling (ADR-014 decision 2; engine-output-contract §9
/// makes a later rename a MAJOR change).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatComponent {
    /// `active_probability`: probability.
    ActiveProbability,
    /// `start_probability`: probability; QB only.
    StartProbability,
    /// `offensive_snap_share`: share of team offensive snaps; RB, WR, TE.
    OffensiveSnapShare,
    /// `pass_attempts`: count, sacks excluded (KI-NEW-I1); QB.
    PassAttempts,
    /// `completions`: count, `≤ pass_attempts` per draw; QB.
    Completions,
    /// `passing_yards`: yards, sack yardage not netted (KI-NEW-I1); QB.
    PassingYards,
    /// `passing_tds`: count, return TDs excluded (KI-NEW-I2); QB.
    PassingTds,
    /// `interceptions`: count; QB.
    Interceptions,
    /// `sacks_taken`: count; QB.
    SacksTaken,
    /// `rushing_attempts` (carries): count; every position.
    RushingAttempts,
    /// `rushing_yards`: yards; every position.
    RushingYards,
    /// `rushing_tds`: count; every position.
    RushingTds,
    /// `targets`: count; RB, WR, TE.
    Targets,
    /// `receptions`: count, `≤ targets` per draw; RB, WR, TE.
    Receptions,
    /// `receiving_yards`: yards; RB, WR, TE.
    ReceivingYards,
    /// `receiving_tds`: count; RB, WR, TE.
    ReceivingTds,
    /// `fumbles_lost`: count, official definition (KI-NEW-I3); every position.
    FumblesLost,
    /// `two_point_conversions`: count, modelled and never a silent 0 (KI-NEW-I5); every
    /// position.
    TwoPointConversions,
}

impl StatComponent {
    /// Every component, in the row order of engine-output-contract §3.2.
    pub const ALL: [StatComponent; 18] = [
        StatComponent::ActiveProbability,
        StatComponent::StartProbability,
        StatComponent::OffensiveSnapShare,
        StatComponent::PassAttempts,
        StatComponent::Completions,
        StatComponent::PassingYards,
        StatComponent::PassingTds,
        StatComponent::Interceptions,
        StatComponent::SacksTaken,
        StatComponent::RushingAttempts,
        StatComponent::RushingYards,
        StatComponent::RushingTds,
        StatComponent::Targets,
        StatComponent::Receptions,
        StatComponent::ReceivingYards,
        StatComponent::ReceivingTds,
        StatComponent::FumblesLost,
        StatComponent::TwoPointConversions,
    ];

    /// Whether the component is part of `position`'s target vector (engine-output-contract §3.2
    /// table; DR-D7).
    pub const fn applies_to(self, position: Position) -> bool {
        match self {
            StatComponent::ActiveProbability
            | StatComponent::RushingAttempts
            | StatComponent::RushingYards
            | StatComponent::RushingTds
            | StatComponent::FumblesLost
            | StatComponent::TwoPointConversions => true,
            StatComponent::StartProbability
            | StatComponent::PassAttempts
            | StatComponent::Completions
            | StatComponent::PassingYards
            | StatComponent::PassingTds
            | StatComponent::Interceptions
            | StatComponent::SacksTaken => matches!(position, Position::Qb),
            StatComponent::OffensiveSnapShare
            | StatComponent::Targets
            | StatComponent::Receptions
            | StatComponent::ReceivingYards
            | StatComponent::ReceivingTds => {
                matches!(position, Position::Rb | Position::Wr | Position::Te)
            }
        }
    }

    /// The component's unit.
    pub const fn unit(self) -> StatUnit {
        match self {
            StatComponent::ActiveProbability | StatComponent::StartProbability => {
                StatUnit::Probability
            }
            StatComponent::OffensiveSnapShare => StatUnit::Share,
            StatComponent::PassAttempts
            | StatComponent::Completions
            | StatComponent::PassingTds
            | StatComponent::Interceptions
            | StatComponent::SacksTaken
            | StatComponent::RushingAttempts
            | StatComponent::RushingTds
            | StatComponent::Targets
            | StatComponent::Receptions
            | StatComponent::ReceivingTds
            | StatComponent::FumblesLost
            | StatComponent::TwoPointConversions => StatUnit::Count,
            StatComponent::PassingYards
            | StatComponent::RushingYards
            | StatComponent::ReceivingYards => StatUnit::Yards,
        }
    }

    /// The contract's `snake_case` field name of the component (engine-output-contract §3.2).
    pub const fn contract_name(self) -> &'static str {
        match self {
            StatComponent::ActiveProbability => "active_probability",
            StatComponent::StartProbability => "start_probability",
            StatComponent::OffensiveSnapShare => "offensive_snap_share",
            StatComponent::PassAttempts => "pass_attempts",
            StatComponent::Completions => "completions",
            StatComponent::PassingYards => "passing_yards",
            StatComponent::PassingTds => "passing_tds",
            StatComponent::Interceptions => "interceptions",
            StatComponent::SacksTaken => "sacks_taken",
            StatComponent::RushingAttempts => "rushing_attempts",
            StatComponent::RushingYards => "rushing_yards",
            StatComponent::RushingTds => "rushing_tds",
            StatComponent::Targets => "targets",
            StatComponent::Receptions => "receptions",
            StatComponent::ReceivingYards => "receiving_yards",
            StatComponent::ReceivingTds => "receiving_tds",
            StatComponent::FumblesLost => "fumbles_lost",
            StatComponent::TwoPointConversions => "two_point_conversions",
        }
    }
}

impl fmt::Display for StatComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.contract_name())
    }
}

/// The distribution summary of one stat component, computed from the Layer-F draws
/// (engine-output-contract §3.5). Units: the component's unit. Invariants: `sd ≥ 0`,
/// `q10 ≤ q25 ≤ q50 ≤ q75 ≤ q90`, every value finite. Never null.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StatDistribution {
    /// The mean of the draws.
    pub mean: f64,
    /// The standard deviation of the draws, `≥ 0`.
    pub sd: f64,
    /// The 10th percentile.
    pub q10: f64,
    /// The 25th percentile.
    pub q25: f64,
    /// The median.
    pub q50: f64,
    /// The 75th percentile.
    pub q75: f64,
    /// The 90th percentile.
    pub q90: f64,
}

impl StatDistribution {
    /// The quantiles in order, with their field names.
    fn quantiles(&self) -> [(&'static str, f64); 5] {
        [
            ("q10", self.q10),
            ("q25", self.q25),
            ("q50", self.q50),
            ("q75", self.q75),
            ("q90", self.q90),
        ]
    }

    /// Every value with its field name.
    fn fields(&self) -> [(&'static str, f64); 7] {
        [
            ("mean", self.mean),
            ("sd", self.sd),
            ("q10", self.q10),
            ("q25", self.q25),
            ("q50", self.q50),
            ("q75", self.q75),
            ("q90", self.q90),
        ]
    }

    /// Checks finiteness, `sd ≥ 0` and quantile monotonicity, naming fields under `path`.
    pub fn validate(&self, path: &str) -> Result<(), OutputInvariantError> {
        check_finite(self.fields().iter().copied(), path)?;
        if self.sd < 0.0 {
            return Err(OutputInvariantError::new(
                OutputInvariant::NegativeSd,
                format!("{path}.sd"),
                format!("sd = {}", self.sd),
            ));
        }
        check_monotone(&self.quantiles(), path)
    }

    /// Whether every value is exactly zero (the zero vector of an inactive player).
    fn is_zero(&self) -> bool {
        self.fields().iter().all(|(_, v)| *v == 0.0)
    }
}

/// One component's summary (engine-output-contract §3.5). Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentSummary {
    /// The component.
    pub component: StatComponent,
    /// Its distribution summary.
    pub distribution: StatDistribution,
}

impl ComponentSummary {
    /// Checks the distribution ([`StatDistribution::validate`]) and then the range the
    /// component's unit imposes (engine-output-contract §3.9; engine-spec §5.5 "Field rules"):
    /// a `Probability` component's mean and quantiles lie in `[0, 1]`
    /// (`ProbabilityOutOfRange`); a `Share` component's mean and quantiles lie in `[0, 1]`, a
    /// value above 1 being `ShareOverflow` (the team sum then exceeds 1) and a negative value
    /// `ProbabilityOutOfRange`; `Count` and `Yards` components have no range check here. Fields
    /// are named under `path` (`{path}.mean`, `{path}.q90`, …).
    pub fn validate(&self, path: &str) -> Result<(), OutputInvariantError> {
        self.distribution.validate(path)?;
        match self.component.unit() {
            StatUnit::Probability => {
                for (field, v) in self.distribution.fields() {
                    if field != "sd" {
                        check_probability(v, &format!("{path}.{field}"))?;
                    }
                }
            }
            StatUnit::Share => {
                for (field, v) in self.distribution.fields() {
                    if field == "sd" {
                        continue;
                    }
                    if v > 1.0 {
                        return Err(OutputInvariantError::new(
                            OutputInvariant::ShareOverflow,
                            format!("{path}.{field}"),
                            format!("share {v} exceeds 1"),
                        ));
                    }
                    check_probability(v, &format!("{path}.{field}"))?;
                }
            }
            StatUnit::Count | StatUnit::Yards => {}
        }
        Ok(())
    }
}

/// A stat-vector summary: one entry per component of the position's target vector
/// (engine-output-contract §3.4, §3.5). Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatVectorSummary {
    /// The components present.
    pub components: Vec<ComponentSummary>,
}

impl StatVectorSummary {
    /// The summary of one component, if present.
    pub fn get(&self, component: StatComponent) -> Option<&StatDistribution> {
        self.components
            .iter()
            .find(|c| c.component == component)
            .map(|c| &c.distribution)
    }
}

/// A DR-D8 versioned-setting identifier: the version of an open output definition (a percentile
/// definition, a threshold set, a starter-threshold table, an attribution method) recorded per
/// output version. The **values** are set with Statistical-owner approval before the package that
/// uses them is Ready (P1-08, P2-07); this type carries only the version. Never null.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SettingVersion(pub u32);

impl fmt::Display for SettingVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A floor or ceiling value tagged with the version of its percentile definition
/// (engine-output-contract §3.6; DR-D8). Units: fantasy points of the profile. Never null.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LabelledQuantile {
    /// The value, in fantasy points.
    pub value: f64,
    /// The percentile definition behind the label.
    pub percentile_definition: SettingVersion,
}

/// The probability of exceeding one configurable fantasy-point threshold
/// (engine-output-contract §3.6). Never null.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ExceedanceProbability {
    /// The threshold, in fantasy points of the profile.
    pub threshold_points: f64,
    /// `P(points > threshold)`, in `[0, 1]`.
    pub probability: f64,
}

/// A reference to the persisted draw matrix the record was scored from
/// (engine-output-contract §3.7 `draws_ref`). Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DrawsRef {
    /// The registered artifact id of the draw matrix.
    pub matrix_id: String,
    /// The draw count.
    pub n_draws: u32,
}

/// The fantasy-point distribution under one scoring profile (engine-output-contract §3.6;
/// engine-spec §5.3). Every quantity is computed from the Layer-F draws re-scored through the
/// profile; a Gaussian never stands in. Units: fantasy points of `profile` unless stated.
/// Invariants: `sd ≥ 0`, `p10 ≤ p25 ≤ median ≤ p75 ≤ p90`, probabilities in `[0, 1]`, every value
/// finite. Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyDistribution {
    /// The scoring profile.
    pub profile: ScoringProfileRef,
    /// The mean. The **unconditional** value is the headline expected-points field.
    pub mean: f64,
    /// The median.
    pub median: f64,
    /// The standard deviation, `≥ 0`.
    pub sd: f64,
    /// The 10th percentile.
    pub p10: f64,
    /// The 25th percentile.
    pub p25: f64,
    /// The 75th percentile.
    pub p75: f64,
    /// The 90th percentile.
    pub p90: f64,
    /// The floor, tagged with its percentile definition (open value: DR-D8).
    pub floor: LabelledQuantile,
    /// The ceiling, tagged with its percentile definition (open value: DR-D8).
    pub ceiling: LabelledQuantile,
    /// Probability of zero points or inactive, in `[0, 1]`.
    pub p_zero_or_inactive: f64,
    /// Exceedance probabilities for the configurable threshold set.
    pub p_exceed: Vec<ExceedanceProbability>,
    /// The version of the threshold set behind `p_exceed` (open value: DR-D8).
    pub threshold_set: SettingVersion,
    /// Probability of a boom relative to the positional starter threshold, in `[0, 1]`.
    pub p_boom: f64,
    /// Probability of a bust relative to the positional starter threshold, in `[0, 1]`.
    pub p_bust: f64,
    /// The version of the starter-threshold table behind `p_boom` and `p_bust` (open value:
    /// DR-D8).
    pub starter_thresholds: SettingVersion,
    /// The draw matrix the distribution was scored from.
    pub draws: DrawsRef,
}

impl FantasyDistribution {
    /// Checks finiteness, `sd ≥ 0`, quantile monotonicity and every probability's range, naming
    /// fields under `path`.
    pub fn validate(&self, path: &str) -> Result<(), OutputInvariantError> {
        let scalars = [
            ("mean", self.mean),
            ("median", self.median),
            ("sd", self.sd),
            ("p10", self.p10),
            ("p25", self.p25),
            ("p75", self.p75),
            ("p90", self.p90),
            ("floor.value", self.floor.value),
            ("ceiling.value", self.ceiling.value),
            ("p_zero_or_inactive", self.p_zero_or_inactive),
            ("p_boom", self.p_boom),
            ("p_bust", self.p_bust),
        ];
        check_finite(scalars.iter().copied(), path)?;
        for (i, e) in self.p_exceed.iter().enumerate() {
            check_finite(
                [
                    ("threshold_points", e.threshold_points),
                    ("probability", e.probability),
                ]
                .into_iter(),
                &format!("{path}.p_exceed[{i}]"),
            )?;
        }
        if self.sd < 0.0 {
            return Err(OutputInvariantError::new(
                OutputInvariant::NegativeSd,
                format!("{path}.sd"),
                format!("sd = {}", self.sd),
            ));
        }
        check_monotone(
            &[
                ("p10", self.p10),
                ("p25", self.p25),
                ("median", self.median),
                ("p75", self.p75),
                ("p90", self.p90),
            ],
            path,
        )?;
        check_probability(
            self.p_zero_or_inactive,
            &format!("{path}.p_zero_or_inactive"),
        )?;
        check_probability(self.p_boom, &format!("{path}.p_boom"))?;
        check_probability(self.p_bust, &format!("{path}.p_bust"))?;
        for (i, e) in self.p_exceed.iter().enumerate() {
            check_probability(e.probability, &format!("{path}.p_exceed[{i}].probability"))?;
        }
        Ok(())
    }
}

/// The freshness status of a row's critical sources (engine-output-contract §3.8). **Closed**
/// enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Freshness {
    /// Every critical source is within its provider threshold.
    Fresh,
    /// At least one critical source is beyond its threshold.
    Stale,
    /// At least one critical source is absent. Never inherits an earlier healthy status.
    Missing,
}

impl fmt::Display for Freshness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Freshness::Fresh => "Fresh",
            Freshness::Stale => "Stale",
            Freshness::Missing => "Missing",
        })
    }
}

/// The freshness and provenance flags present on every row and in every export
/// (engine-output-contract §3.8; engine-spec §5.5 "Freshness and flags"). Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FreshnessFlags {
    /// The worst status across the critical sources that fed the row.
    pub freshness: Freshness,
    /// The stale or missing sources by name; empty when `Fresh`.
    pub stale_sources: Vec<String>,
    /// The oldest publication timestamp among the critical sources.
    pub oldest_source_timestamp: UtcInstant,
    /// True when a timestamped manual availability override applied (the manual-data flag).
    pub manual_data: bool,
    /// True when current-week context (schedule, depth chart, availability) was missing.
    pub missing_current_week_context: bool,
    /// True until the Phase-1 exit gate passes (claim discipline, engine-spec §3).
    pub experimental: bool,
}

/// The evidence-level claim wording in force (engine-spec §3.2; engine-spec §5.5 "Record
/// groups", row "Claim label"). **Open** vocabulary: the wording is set by the spec's claim
/// discipline and consumers display it verbatim. Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClaimLabel(pub String);

impl fmt::Display for ClaimLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The player-week projection record (engine-output-contract §2–§4; engine-spec §5.5 "Record
/// groups"). One per player × week × horizon × prediction version. Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerWeekProjection {
    /// The output contract version the record conforms to.
    pub contract: ContractVersion,
    /// The key group.
    pub key: ProjectionKey,
    /// The lineage group.
    pub lineage: ProjectionLineage,
    /// The Layer-A availability block.
    pub availability: AvailabilityBlock,
    /// The conditional stat vector: production if active.
    pub conditional: StatVectorSummary,
    /// The unconditional stat vector: after `p_active`, `p_start` and workload suppression. The
    /// headline.
    pub unconditional: StatVectorSummary,
    /// One fantasy distribution per scoring profile; the lineage's profile is the headline.
    pub fantasy: Vec<FantasyDistribution>,
    /// The explanation record.
    pub explanation: ExplanationRecord,
    /// The freshness and provenance flags.
    pub flags: FreshnessFlags,
    /// The claim label in force.
    pub claim_label: ClaimLabel,
    /// Attribution strings of every upstream source whose data fed the run (engine-spec §4.8).
    pub attribution: Vec<String>,
}

impl PlayerWeekProjection {
    /// Checks the §3.9 record invariants (engine-output-contract §3.9; engine-spec §5.3, §5.5).
    /// Returns the first violation as a typed [`OutputInvariantError`]; the record is never
    /// clamped or repaired.
    ///
    /// Checked on the record:
    ///
    /// - every probability and every share is in `[0, 1]` (`ProbabilityOutOfRange`; a share
    ///   above 1 is `ShareOverflow`, since the team sum then exceeds 1);
    /// - quantiles are monotone in every [`StatDistribution`] and [`FantasyDistribution`]
    ///   (`QuantilesNotMonotone`);
    /// - every standard deviation is `≥ 0` (`NegativeSd`);
    /// - no NaN or infinity anywhere in the record (`NonFinite`);
    /// - the summary-level necessary conditions of the per-draw invariants: the mean and every
    ///   quantile of `completions` ≤ those of `pass_attempts`, and of `receptions` ≤ those of
    ///   `targets`, in both blocks (`CompletionsExceedAttempts`, `ReceptionsExceedTargets`); and
    ///   when `p_active = 0`, every unconditional component summary is the zero vector
    ///   (`InactiveDrawNotZero`; engine-output-contract §3.3 and §3.9 state the invariant on the
    ///   stat vector).
    ///
    /// The fantasy distributions of an inactive player are **not** required to be located at
    /// zero: `fantasy_points = weights · stat_vector + offset` (engine-spec §5.4), so an inactive
    /// draw scores the profile's offset, which the record does not carry. The fantasy-point
    /// consequence of an inactive draw is checked on the draws, against the profile, by
    /// `grid-simulation`.
    ///
    /// The per-draw forms of the last three invariants, and the team-level share sum, are
    /// checked on the draw matrices by `grid-simulation` before this record is built; this check
    /// cannot see draws and does not claim to.
    ///
    /// **Documented but not checked here** (producer obligations): `snap_multiplier_if_active ≥
    /// 0` on the [`AvailabilityBlock`], `filtered_var ≥ 0` and `predictive_var ≥ 0` on every
    /// [`RoleTalent`](crate::output::RoleTalent), `var ≥ 0` and `weight_now` in `[0, 1]` on the
    /// [`PriorBlock`](crate::output::PriorBlock). The closed [`OutputInvariant`] enum, fixed by
    /// P1-01, has no variant for a negative multiplier or variance, and a variant is a contract
    /// decision, not an implementer's; these ranges are checked for finiteness only. The
    /// `[0, 1]` ranges that engine-spec §5.5 names (probabilities and shares) are checked.
    pub fn validate(&self) -> Result<(), OutputInvariantError> {
        self.validate_availability()?;
        for (block_name, block) in [
            ("conditional", &self.conditional),
            ("unconditional", &self.unconditional),
        ] {
            Self::validate_block(block_name, block)?;
        }
        for (i, dist) in self.fantasy.iter().enumerate() {
            dist.validate(&format!("fantasy[{i}]"))?;
        }
        self.explanation.validate("explanation")?;
        if self.availability.p_active == 0.0 {
            for c in &self.unconditional.components {
                if !c.distribution.is_zero() {
                    return Err(OutputInvariantError::new(
                        OutputInvariant::InactiveDrawNotZero,
                        format!("unconditional.{}", c.component),
                        "p_active = 0 but the unconditional summary is not the zero vector",
                    ));
                }
            }
        }
        Ok(())
    }

    fn validate_availability(&self) -> Result<(), OutputInvariantError> {
        let a = &self.availability;
        let mut scalars = vec![
            ("p_active", a.p_active),
            ("snap_multiplier_if_active", a.snap_multiplier_if_active),
            ("p_limited_role", a.p_limited_role),
        ];
        if let Some(p) = a.p_start {
            scalars.push(("p_start", p));
        }
        check_finite(scalars.iter().copied(), "availability")?;
        check_probability(a.p_active, "availability.p_active")?;
        if let Some(p) = a.p_start {
            check_probability(p, "availability.p_start")?;
        }
        check_probability(a.p_limited_role, "availability.p_limited_role")
    }

    fn validate_block(name: &str, block: &StatVectorSummary) -> Result<(), OutputInvariantError> {
        for c in &block.components {
            c.validate(&format!("{name}.{}", c.component))?;
        }
        Self::check_bounded_pair(
            name,
            block,
            StatComponent::Completions,
            StatComponent::PassAttempts,
            OutputInvariant::CompletionsExceedAttempts,
        )?;
        Self::check_bounded_pair(
            name,
            block,
            StatComponent::Receptions,
            StatComponent::Targets,
            OutputInvariant::ReceptionsExceedTargets,
        )
    }

    /// `lower ≤ upper` in the mean and in every quantile, when both components are present.
    fn check_bounded_pair(
        name: &str,
        block: &StatVectorSummary,
        lower: StatComponent,
        upper: StatComponent,
        invariant: OutputInvariant,
    ) -> Result<(), OutputInvariantError> {
        let (Some(lo), Some(hi)) = (block.get(lower), block.get(upper)) else {
            return Ok(());
        };
        for ((field, l), (_, h)) in lo.fields().iter().zip(hi.fields().iter()) {
            if *field == "sd" {
                continue;
            }
            if l > h {
                return Err(OutputInvariantError::new(
                    invariant,
                    format!("{name}.{lower}.{field}"),
                    format!("{lower}.{field} = {l} exceeds {upper}.{field} = {h}"),
                ));
            }
        }
        Ok(())
    }
}

/// Fails with `NonFinite` on the first NaN or infinity among `values`.
pub(crate) fn check_finite<'a>(
    values: impl Iterator<Item = (&'a str, f64)>,
    path: &str,
) -> Result<(), OutputInvariantError> {
    for (field, v) in values {
        if !v.is_finite() {
            return Err(OutputInvariantError::new(
                OutputInvariant::NonFinite,
                format!("{path}.{field}"),
                format!("value is {v}"),
            ));
        }
    }
    Ok(())
}

/// Fails with `ProbabilityOutOfRange` unless `0 ≤ p ≤ 1`.
pub(crate) fn check_probability(p: f64, path: &str) -> Result<(), OutputInvariantError> {
    if !(0.0..=1.0).contains(&p) {
        return Err(OutputInvariantError::new(
            OutputInvariant::ProbabilityOutOfRange,
            path,
            format!("value {p} is outside [0, 1]"),
        ));
    }
    Ok(())
}

/// Fails with `QuantilesNotMonotone` unless the named values are non-decreasing.
pub(crate) fn check_monotone(
    ordered: &[(&str, f64)],
    path: &str,
) -> Result<(), OutputInvariantError> {
    for pair in ordered.windows(2) {
        let (lo_name, lo) = pair[0];
        let (hi_name, hi) = pair[1];
        if lo > hi {
            return Err(OutputInvariantError::new(
                OutputInvariant::QuantilesNotMonotone,
                format!("{path}.{hi_name}"),
                format!("{lo_name} = {lo} exceeds {hi_name} = {hi}"),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod test_support {
    //! Builders of a valid representative record, shared by the output tests.

    use super::*;
    use crate::asof::LockKind;
    use crate::ids::{GameId, ScoringProfileId, ScoringProfileVersion, Season, TeamKey, Week};
    use crate::output::explanation::{
        ExplanationRecord, GridSignalBlock, RegimeFlags, RoleTalentBlock, TeamGameRef,
    };

    pub fn dist(mean: f64) -> StatDistribution {
        StatDistribution {
            mean,
            sd: mean.abs() * 0.3,
            q10: mean * 0.6,
            q25: mean * 0.8,
            q50: mean,
            q75: mean * 1.2,
            q90: mean * 1.4,
        }
    }

    pub fn profile_ref() -> ScoringProfileRef {
        ScoringProfileRef {
            id: ScoringProfileId("half-ppr".into()),
            version: ScoringProfileVersion(1),
        }
    }

    pub fn fantasy(mean: f64) -> FantasyDistribution {
        FantasyDistribution {
            profile: profile_ref(),
            mean,
            median: mean * 0.95,
            sd: mean * 0.4,
            p10: mean * 0.4,
            p25: mean * 0.7,
            p75: mean * 1.25,
            p90: mean * 1.5,
            floor: LabelledQuantile {
                value: mean * 0.4,
                percentile_definition: SettingVersion(1),
            },
            ceiling: LabelledQuantile {
                value: mean * 1.5,
                percentile_definition: SettingVersion(1),
            },
            p_zero_or_inactive: if mean == 0.0 { 1.0 } else { 0.08 },
            p_exceed: vec![ExceedanceProbability {
                threshold_points: 10.0,
                probability: if mean == 0.0 { 0.0 } else { 0.7 },
            }],
            threshold_set: SettingVersion(1),
            p_boom: if mean == 0.0 { 0.0 } else { 0.2 },
            p_bust: if mean == 0.0 { 1.0 } else { 0.15 },
            starter_thresholds: SettingVersion(1),
            draws: DrawsRef {
                matrix_id: "draws-1".into(),
                n_draws: 10_000,
            },
        }
    }

    pub fn qb_vector(scale: f64) -> StatVectorSummary {
        let d = |c: StatComponent, m: f64| ComponentSummary {
            component: c,
            distribution: if c.unit() == StatUnit::Probability {
                StatDistribution {
                    mean: m,
                    sd: 0.0,
                    q10: m,
                    q25: m,
                    q50: m,
                    q75: m,
                    q90: m,
                }
            } else {
                dist(m * scale)
            },
        };
        StatVectorSummary {
            components: vec![
                d(
                    StatComponent::ActiveProbability,
                    if scale == 0.0 { 0.0 } else { 0.95 },
                ),
                d(
                    StatComponent::StartProbability,
                    if scale == 0.0 { 0.0 } else { 0.9 },
                ),
                d(StatComponent::PassAttempts, 34.0),
                d(StatComponent::Completions, 22.0),
                d(StatComponent::PassingYards, 250.0),
                d(StatComponent::PassingTds, 1.8),
                d(StatComponent::Interceptions, 0.7),
                d(StatComponent::SacksTaken, 2.1),
                d(StatComponent::RushingAttempts, 4.0),
                d(StatComponent::RushingYards, 18.0),
                d(StatComponent::RushingTds, 0.2),
                d(StatComponent::FumblesLost, 0.1),
                d(StatComponent::TwoPointConversions, 0.05),
            ],
        }
    }

    pub fn explanation() -> ExplanationRecord {
        ExplanationRecord {
            team_environment: TeamGameRef {
                prediction_version_id: PredictionVersionId(7),
                game_id: GameId("2099_05_KC_MIN".into()),
                team: TeamKey::Nfl("KC".into()),
            },
            role_opportunity: vec![],
            availability_adjustment: Some(-0.8),
            matchup_adjustment: Some(0.4),
            ncaa_prior_contribution: None,
            recent_evidence_contribution: Some(1.1),
            top_positive_drivers: vec![],
            top_negative_drivers: vec![],
            uncertainty_drivers: vec![],
            delta_from_prior: None,
            grid: Some(GridSignalBlock {
                role_talent: RoleTalentBlock {
                    dropback: None,
                    carry: None,
                    target: None,
                },
                form: Some(0.01),
                scheme_fit: None,
                offseason_rapm: None,
                prior: None,
                regime_flags: RegimeFlags {
                    intervention: false,
                    observation_noise_inflation: false,
                    scheme_reset: false,
                    changepoint_z: None,
                },
            }),
        }
    }

    pub fn qb_projection() -> PlayerWeekProjection {
        PlayerWeekProjection {
            contract: crate::output::GRID_OUTPUT_CONTRACT_VERSION,
            key: ProjectionKey {
                season: Season(2023),
                week: Week::new(5).unwrap(),
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
                scoring_profile: profile_ref(),
                seed: 12_345,
                n_draws: 10_000,
                lock: Lock {
                    kind: LockKind::Sunday,
                    at: UtcInstant::from_unix_seconds(1_696_770_000),
                },
                computed_at: UtcInstant::from_unix_seconds(1_696_769_000),
                research: ResearchFlag::none(),
            },
            availability: AvailabilityBlock {
                p_active: 0.95,
                p_start: Some(0.9),
                snap_multiplier_if_active: 1.0,
                p_limited_role: 0.05,
                availability_source: AvailabilitySource::Provider,
                availability_observed_at: Some(UtcInstant::from_unix_seconds(1_696_700_000)),
                manual_override: false,
            },
            conditional: qb_vector(1.0),
            unconditional: qb_vector(0.95),
            fantasy: vec![fantasy(18.5)],
            explanation: explanation(),
            flags: FreshnessFlags {
                freshness: Freshness::Fresh,
                stale_sources: vec![],
                oldest_source_timestamp: UtcInstant::from_unix_seconds(1_696_600_000),
                manual_data: false,
                missing_current_week_context: false,
                experimental: true,
            },
            claim_label: ClaimLabel("experimental".into()),
            attribution: vec!["nflverse".into()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;

    fn set(
        p: &mut PlayerWeekProjection,
        block: bool,
        component: StatComponent,
        d: StatDistribution,
    ) {
        let target = if block {
            &mut p.conditional
        } else {
            &mut p.unconditional
        };
        let entry = target
            .components
            .iter_mut()
            .find(|c| c.component == component)
            .expect("component present");
        entry.distribution = d;
    }

    #[test]
    fn representative_record_is_valid() {
        assert_eq!(qb_projection().validate(), Ok(()));
    }

    #[test]
    fn stat_component_table_matches_the_contract() {
        use StatComponent as S;
        let qb: Vec<S> = S::ALL
            .iter()
            .copied()
            .filter(|c| c.applies_to(Position::Qb))
            .collect();
        assert_eq!(
            qb,
            vec![
                S::ActiveProbability,
                S::StartProbability,
                S::PassAttempts,
                S::Completions,
                S::PassingYards,
                S::PassingTds,
                S::Interceptions,
                S::SacksTaken,
                S::RushingAttempts,
                S::RushingYards,
                S::RushingTds,
                S::FumblesLost,
                S::TwoPointConversions,
            ]
        );
        let skill = vec![
            S::ActiveProbability,
            S::OffensiveSnapShare,
            S::RushingAttempts,
            S::RushingYards,
            S::RushingTds,
            S::Targets,
            S::Receptions,
            S::ReceivingYards,
            S::ReceivingTds,
            S::FumblesLost,
            S::TwoPointConversions,
        ];
        for pos in [Position::Rb, Position::Wr, Position::Te] {
            let v: Vec<S> = S::ALL
                .iter()
                .copied()
                .filter(|c| c.applies_to(pos))
                .collect();
            assert_eq!(v, skill, "{pos}");
        }
        // DR-D7 asymmetry: QB has no snap share, skill positions have no start probability.
        assert!(!S::OffensiveSnapShare.applies_to(Position::Qb));
        assert!(!S::StartProbability.applies_to(Position::Rb));
        assert_eq!(S::OffensiveSnapShare.unit(), StatUnit::Share);
        assert_eq!(S::PassingYards.unit(), StatUnit::Yards);
        assert_eq!(S::Targets.unit(), StatUnit::Count);
        assert_eq!(S::ActiveProbability.unit(), StatUnit::Probability);
        assert_eq!(S::TwoPointConversions.to_string(), "two_point_conversions");
        assert_eq!(S::ALL.len(), 18);
    }

    /// `#[serde(rename_all = "snake_case")]` derives the serialized name from the variant name
    /// by serde's rule: an underscore before every uppercase letter after the first, then
    /// lowercase. This mirrors that rule over the `Debug` name (which is the variant name for a
    /// fieldless enum) so the serialized spelling and [`StatComponent::contract_name`] are shown
    /// to agree for every variant; the serialized form itself is checked where `serde_json` lives
    /// (`grid-pipeline`).
    #[test]
    fn stat_component_serde_name_equals_contract_name() {
        fn snake_case(variant: &str) -> String {
            let mut out = String::with_capacity(variant.len() + 4);
            for (i, ch) in variant.chars().enumerate() {
                if ch.is_ascii_uppercase() {
                    if i > 0 {
                        out.push('_');
                    }
                    out.push(ch.to_ascii_lowercase());
                } else {
                    out.push(ch);
                }
            }
            out
        }
        for c in StatComponent::ALL {
            assert_eq!(snake_case(&format!("{c:?}")), c.contract_name(), "{c:?}");
        }
    }

    #[test]
    fn probability_out_of_range_is_rejected() {
        let mut p = qb_projection();
        p.availability.p_active = 1.2;
        let err = p.validate().unwrap_err();
        assert_eq!(err.invariant, OutputInvariant::ProbabilityOutOfRange);
        assert_eq!(err.field, "availability.p_active");

        let mut p = qb_projection();
        p.fantasy[0].p_boom = -0.1;
        let err = p.validate().unwrap_err();
        assert_eq!(err.invariant, OutputInvariant::ProbabilityOutOfRange);
        assert_eq!(err.field, "fantasy[0].p_boom");

        let mut p = qb_projection();
        p.fantasy[0].p_exceed[0].probability = 1.5;
        let err = p.validate().unwrap_err();
        assert_eq!(err.invariant, OutputInvariant::ProbabilityOutOfRange);
        assert_eq!(err.field, "fantasy[0].p_exceed[0].probability");

        let mut p = qb_projection();
        let mut d = dist(0.0);
        d.mean = 1.01;
        d.q10 = 1.01;
        d.q25 = 1.01;
        d.q50 = 1.01;
        d.q75 = 1.01;
        d.q90 = 1.01;
        set(&mut p, true, StatComponent::StartProbability, d);
        let err = p.validate().unwrap_err();
        assert_eq!(err.invariant, OutputInvariant::ProbabilityOutOfRange);
        assert_eq!(err.field, "conditional.start_probability.mean");
    }

    #[test]
    fn quantiles_not_monotone_is_rejected() {
        let mut p = qb_projection();
        let mut d = dist(250.0);
        d.q75 = d.q50 - 1.0;
        set(&mut p, true, StatComponent::PassingYards, d);
        let err = p.validate().unwrap_err();
        assert_eq!(err.invariant, OutputInvariant::QuantilesNotMonotone);
        assert_eq!(err.field, "conditional.passing_yards.q75");

        let mut p = qb_projection();
        p.fantasy[0].p10 = p.fantasy[0].p25 + 1.0;
        let err = p.validate().unwrap_err();
        assert_eq!(err.invariant, OutputInvariant::QuantilesNotMonotone);
        assert_eq!(err.field, "fantasy[0].p25");

        let mut p = qb_projection();
        p.fantasy[0].median = p.fantasy[0].p75 + 1.0;
        let err = p.validate().unwrap_err();
        assert_eq!(err.invariant, OutputInvariant::QuantilesNotMonotone);
        assert_eq!(err.field, "fantasy[0].p75");
    }

    #[test]
    fn negative_sd_is_rejected() {
        let mut p = qb_projection();
        let mut d = dist(34.0);
        d.sd = -0.5;
        set(&mut p, false, StatComponent::PassAttempts, d);
        let err = p.validate().unwrap_err();
        assert_eq!(err.invariant, OutputInvariant::NegativeSd);
        assert_eq!(err.field, "unconditional.pass_attempts.sd");

        let mut p = qb_projection();
        p.fantasy[0].sd = -1e-9;
        let err = p.validate().unwrap_err();
        assert_eq!(err.invariant, OutputInvariant::NegativeSd);
        assert_eq!(err.field, "fantasy[0].sd");
    }

    #[test]
    fn non_finite_is_rejected_anywhere() {
        let mut p = qb_projection();
        p.availability.snap_multiplier_if_active = f64::NAN;
        let err = p.validate().unwrap_err();
        assert_eq!(err.invariant, OutputInvariant::NonFinite);
        assert_eq!(err.field, "availability.snap_multiplier_if_active");

        let mut p = qb_projection();
        let mut d = dist(18.0);
        d.q90 = f64::INFINITY;
        set(&mut p, true, StatComponent::RushingYards, d);
        let err = p.validate().unwrap_err();
        assert_eq!(err.invariant, OutputInvariant::NonFinite);
        assert_eq!(err.field, "conditional.rushing_yards.q90");

        let mut p = qb_projection();
        p.fantasy[0].floor.value = f64::NEG_INFINITY;
        let err = p.validate().unwrap_err();
        assert_eq!(err.invariant, OutputInvariant::NonFinite);
        assert_eq!(err.field, "fantasy[0].floor.value");

        let mut p = qb_projection();
        p.explanation.matchup_adjustment = Some(f64::NAN);
        let err = p.validate().unwrap_err();
        assert_eq!(err.invariant, OutputInvariant::NonFinite);
        assert_eq!(err.field, "explanation.matchup_adjustment");

        let mut p = qb_projection();
        p.explanation.grid.as_mut().unwrap().form = Some(f64::NAN);
        let err = p.validate().unwrap_err();
        assert_eq!(err.invariant, OutputInvariant::NonFinite);
        assert_eq!(err.field, "explanation.grid.form");
    }

    #[test]
    fn share_overflow_is_rejected() {
        let mut p = qb_projection();
        // A share component on a record, with a value above 1.
        let mut d = dist(0.0);
        d.mean = 0.9;
        d.q10 = 0.8;
        d.q25 = 0.85;
        d.q50 = 0.9;
        d.q75 = 1.0;
        d.q90 = 1.05;
        p.conditional.components.push(ComponentSummary {
            component: StatComponent::OffensiveSnapShare,
            distribution: d,
        });
        let err = p.validate().unwrap_err();
        assert_eq!(err.invariant, OutputInvariant::ShareOverflow);
        assert_eq!(err.field, "conditional.offensive_snap_share.q90");
        // A valid share passes; a negative share is a range violation.
        d.q90 = 1.0;
        p.conditional.components.last_mut().unwrap().distribution = d;
        assert_eq!(p.validate(), Ok(()));
        d.q10 = -0.1;
        p.conditional.components.last_mut().unwrap().distribution = d;
        let err = p.validate().unwrap_err();
        assert_eq!(err.invariant, OutputInvariant::ProbabilityOutOfRange);
    }

    #[test]
    fn completions_exceeding_attempts_is_rejected() {
        let mut p = qb_projection();
        let mut d = dist(22.0);
        d.q90 = 60.0; // above pass_attempts.q90 = 34 * 1.4
        set(&mut p, true, StatComponent::Completions, d);
        let err = p.validate().unwrap_err();
        assert_eq!(err.invariant, OutputInvariant::CompletionsExceedAttempts);
        assert_eq!(err.field, "conditional.completions.q90");
    }

    #[test]
    fn receptions_exceeding_targets_is_rejected() {
        let mut p = qb_projection();
        p.key.position = Position::Wr;
        p.conditional.components.push(ComponentSummary {
            component: StatComponent::Targets,
            distribution: dist(8.0),
        });
        p.conditional.components.push(ComponentSummary {
            component: StatComponent::Receptions,
            distribution: dist(9.0),
        });
        let err = p.validate().unwrap_err();
        assert_eq!(err.invariant, OutputInvariant::ReceptionsExceedTargets);
        assert_eq!(err.field, "conditional.receptions.mean");
    }

    #[test]
    fn inactive_player_must_have_zero_unconditional_summaries() {
        let mut p = qb_projection();
        p.availability.p_active = 0.0;
        p.availability.p_start = Some(0.0);
        p.availability.availability_source = AvailabilitySource::Provider;
        // Unconditional still carries the active-case numbers: a violation.
        let err = p.validate().unwrap_err();
        assert_eq!(err.invariant, OutputInvariant::InactiveDrawNotZero);
        assert!(err.field.starts_with("unconditional."));
        // Zero the unconditional block but leave a non-zero fantasy distribution: valid at record
        // level. The §3.3/§3.9 invariant is on the stat vector; under an affine profile with a
        // non-zero offset (engine-spec §5.4) every inactive draw scores the offset, which the
        // record does not carry, so no zero-location rule applies to the fantasy distribution.
        // The conditional block may still describe the if-active case.
        p.unconditional = qb_vector(0.0);
        assert_eq!(p.validate(), Ok(()));
        // A fantasy distribution at zero is of course also valid.
        p.fantasy = vec![fantasy(0.0)];
        assert_eq!(p.validate(), Ok(()));
    }

    #[test]
    fn lineage_carries_the_research_marker() {
        let p = qb_projection();
        assert!(!p.lineage.research.research_only);
        assert!(p.lineage.research.is_consistent());
        let mut p = qb_projection();
        p.lineage.research =
            ResearchFlag::relaxed("publication-lag: in-season participation admitted");
        assert!(p.lineage.research.research_only);
        assert_eq!(
            p.lineage.research.relaxed_rule.as_deref(),
            Some("publication-lag: in-season participation admitted")
        );
    }

    #[test]
    fn stat_vector_lookup() {
        let v = qb_vector(1.0);
        assert!(v.get(StatComponent::PassAttempts).is_some());
        assert!(v.get(StatComponent::Targets).is_none());
    }
}
