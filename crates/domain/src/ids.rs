//! Canonical identifiers and version identifiers (engine-spec §4.4.1, §8.9).
//!
//! Every model state, projection and artifact is keyed by the types here. Identity is structural
//! (two keys are equal when their variant and payload are equal), never the text of a serialized
//! form (KI-NEW-C1). The oracle's string-sorted column order is reproduced by [`PlayerKey`]'s
//! `Ord` so that golden column order is preserved (engine-spec §4.4.1 "Synthetic IDs";
//! `reference/python/backend/grid/layers.py:252`).

use std::cmp::Ordering;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::ContractError;

/// An NFL season, by its starting calendar year (for example `2023`).
///
/// Units: calendar year. Never null. Closed numeric domain, no sentinel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Season(pub u16);

impl fmt::Display for Season {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A week index within a season, `1..=u8::MAX`; **never 0**.
///
/// The oracle stores a season total under a `week = 0` sentinel (KI-A4). The engine encodes a
/// season total through [`crate::output::Horizon`] instead, so this type rejects zero at
/// construction and at deserialization. The inner value is private for that reason; read it with
/// [`Week::get`]. No upper bound is fixed by any authority at the identifier level (regular season
/// 1–18, postseason weeks reach 22 in nflverse data: plays-contract §2.1), so none is enforced
/// here; the plays contract's `season_type` carries the regular/post distinction.
///
/// Units: week number. Never null.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct Week(u8);

impl Week {
    /// Builds a week, rejecting `0` with [`ContractError::InvalidValue`] (KI-A4).
    ///
    /// ```
    /// use grid_domain::Week;
    /// assert!(Week::new(0).is_err());
    /// assert_eq!(Week::new(7).unwrap().get(), 7);
    /// ```
    pub fn new(week: u8) -> Result<Self, ContractError> {
        if week == 0 {
            return Err(ContractError::InvalidValue {
                field: "week".to_owned(),
                detail: "week 0 is the oracle's season-total sentinel and is not a week (KI-A4)"
                    .to_owned(),
            });
        }
        Ok(Self(week))
    }

    /// The week number, always `>= 1`.
    pub fn get(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for Week {
    type Error = ContractError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Week> for u8 {
    fn from(week: Week) -> Self {
        week.0
    }
}

impl fmt::Display for Week {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A `(season, week)` game-week slot, ordered lexicographically by season then week.
///
/// Every watermark, cutoff and event slot is a `Slot`, never a bare week: a week-only cutoff admits
/// future-season low weeks (engine-spec §4.5 "Temporal"; KI-NEW-Z61, KI-NEW-W1). The derived
/// `Ord` compares `season` first, then `week`, which is the lexicographic order `≤_lex` of
/// `docs/05-model-specs/evaluation-and-leakage.md` §4.1.
///
/// Never null.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Slot {
    /// The season of the slot.
    pub season: Season,
    /// The week within that season (never 0).
    pub week: Week,
}

impl Slot {
    /// Builds a slot from a season and a week.
    pub fn new(season: Season, week: Week) -> Self {
        Self { season, week }
    }
}

impl fmt::Display for Slot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.season, self.week)
    }
}

/// The canonical player key (engine-spec §4.4.1).
///
/// `Gsis` is the nflverse `gsis_id` (for example `00-0033873`), the canonical NFL key whenever
/// available. `Synthetic` is a synthetic-world id; it appears only in runs whose data snapshot is
/// synthetic, and such runs are flagged (engine-output-contract §2). Closed enum.
///
/// **Ordering and display are by the string form** (`layers.py:252`, `sorted(..., key=str)`), so
/// synthetic ids order as `"0" < "1" < "10" < "100"` and golden column order is preserved. Equal
/// string forms of different variants are ordered `Gsis` before `Synthetic`, so that `Ord` agrees
/// with the derived structural `Eq`. Never null.
///
/// ```
/// use grid_domain::PlayerKey;
/// let mut keys = vec![
///     PlayerKey::Synthetic(100),
///     PlayerKey::Synthetic(10),
///     PlayerKey::Synthetic(1),
///     PlayerKey::Synthetic(0),
/// ];
/// keys.sort();
/// let shown: Vec<String> = keys.iter().map(ToString::to_string).collect();
/// assert_eq!(shown, ["0", "1", "10", "100"]);
/// // Numeric 2 sorts after numeric 10 because "2" > "10" as text.
/// assert!(PlayerKey::Synthetic(10) < PlayerKey::Synthetic(2));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlayerKey {
    /// An nflverse `gsis_id`.
    Gsis(String),
    /// A synthetic-world player number.
    Synthetic(u32),
}

impl PlayerKey {
    /// The string form the ordering uses: the `gsis_id` itself, or the decimal synthetic number.
    pub fn as_str_form(&self) -> String {
        match self {
            PlayerKey::Gsis(id) => id.clone(),
            PlayerKey::Synthetic(n) => n.to_string(),
        }
    }

    fn variant_rank(&self) -> u8 {
        match self {
            PlayerKey::Gsis(_) => 0,
            PlayerKey::Synthetic(_) => 1,
        }
    }
}

impl fmt::Display for PlayerKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlayerKey::Gsis(id) => f.write_str(id),
            PlayerKey::Synthetic(n) => write!(f, "{n}"),
        }
    }
}

impl PartialOrd for PlayerKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PlayerKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str_form()
            .cmp(&other.as_str_form())
            .then_with(|| self.variant_rank().cmp(&other.variant_rank()))
    }
}

/// The canonical team key.
///
/// `Nfl` is an nflverse team abbreviation (`KC`, `SF`, …); `Synthetic` is a synthetic-world team
/// number. Closed enum. Never null.
///
/// **Ordering is native** (`layers.py:255`, `sorted(players.team.unique())`): numeric among
/// `Synthetic`, lexicographic among `Nfl`, and every `Synthetic` sorts before every `Nfl`. A frame
/// never mixes the two, so the cross-variant rule only makes the order total.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TeamKey {
    /// An nflverse team abbreviation.
    Nfl(String),
    /// A synthetic-world team number.
    Synthetic(u16),
}

impl fmt::Display for TeamKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TeamKey::Nfl(abbr) => f.write_str(abbr),
            TeamKey::Synthetic(n) => write!(f, "{n}"),
        }
    }
}

impl PartialOrd for TeamKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TeamKey {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (TeamKey::Synthetic(a), TeamKey::Synthetic(b)) => a.cmp(b),
            (TeamKey::Nfl(a), TeamKey::Nfl(b)) => a.cmp(b),
            (TeamKey::Synthetic(_), TeamKey::Nfl(_)) => Ordering::Less,
            (TeamKey::Nfl(_), TeamKey::Synthetic(_)) => Ordering::Greater,
        }
    }
}

/// A game identifier: the nflverse `game_id` (`2023_01_ARI_WAS`; plays-contract §2.1) or the
/// synthetic form `synth-{season}-{week:02}-{a}-{b}` with `a ≤ b` the two team numbers, fixed by
/// P1-01 for `grid-synth::legacy_v0::from_legacy_v0` (the plays contract defines no synthetic
/// game id). Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GameId(pub String);

impl fmt::Display for GameId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Play identity: `play_id` is unique **within a game only** (plays-contract §2.1), so the key
/// carries the game. Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PlayKey {
    /// The game the play belongs to.
    pub game_id: GameId,
    /// The nflverse `play_id`, or the synthetic generation-order id.
    pub play_id: u32,
}

impl fmt::Display for PlayKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}#{}", self.game_id, self.play_id)
    }
}

/// Drive identity: a drive sequence number within a game (plays-contract §2.1 `drive_id`). It
/// scopes the next-state chain and `drive_points`. Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DriveKey {
    /// The game the drive belongs to.
    pub game_id: GameId,
    /// The drive's sequence within the game (nflverse `fixed_drive`), or the legacy synthetic
    /// global `drive_id`, which is in `1..=2016` and globally unique (plays-contract §2.1
    /// `drive_id`).
    pub drive_seq: u16,
}

impl fmt::Display for DriveKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/drive {}", self.game_id, self.drive_seq)
    }
}

macro_rules! u64_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
        )]
        pub struct $name(pub u64);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

macro_rules! u32_version {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
        )]
        pub struct $name(pub u32);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

u64_id! {
    /// A committed data version: the first link of `Data → Feature → Model → Prediction`
    /// (engine-spec §8.9). Also the `data_snapshot_id` of a projection's lineage
    /// (engine-output-contract §2). Never null.
    DataVersion
}

u32_version! {
    /// A feature-schema version (engine-spec §8.9, for example `feature_schema_version = 17`).
    /// Changing a feature definition creates a new schema version. Never null.
    FeatureSchemaVersion
}

u64_id! {
    /// A model version: a versioned scientific artifact, never a mutable global (engine-spec §8.9).
    /// Never null.
    ModelVersionId
}

u64_id! {
    /// A prediction version: unique per emitted projection set. A post-lock re-projection is a new
    /// version, never an overwrite (engine-output-contract §2). Never null.
    PredictionVersionId
}

/// A scoring-profile identifier (engine-spec §5.4; `Standard`, `Half-PPR`, `PPR` or a custom
/// profile's content-hash id, never key-order-sensitive JSON text: KI-NEW-C1). Open vocabulary.
/// Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ScoringProfileId(pub String);

impl fmt::Display for ScoringProfileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

u32_version! {
    /// A scoring-profile version. A profile version is never edited; a change creates a new
    /// version (engine-spec §5.4). Never null.
    ScoringProfileVersion
}

/// A reference to one immutable scoring profile version: `(id, version)`
/// (engine-output-contract §2 `scoring_profile`). Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ScoringProfileRef {
    /// The profile identifier.
    pub id: ScoringProfileId,
    /// The immutable profile version.
    pub version: ScoringProfileVersion,
}

impl fmt::Display for ScoringProfileRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}@{}", self.id, self.version)
    }
}

u64_id! {
    /// An ingestion run (engine-spec §8.10 `ingestion_run_id`; plays-contract §9 `SourceSnapshot`).
    /// Never null where a run exists; `Option::None` on a synthetic frame that no ingestion run
    /// produced.
    IngestionRunId
}

u64_id! {
    /// A durable job in the run ledger (engine-spec §8.10 `job_id`). Never null.
    JobId
}

u64_id! {
    /// A training run (engine-spec §8.9 `training_run_id`). Never null.
    TrainingRunId
}

u64_id! {
    /// A snapshot record (engine-spec §8.10 `SNAPSHOT`, §14.3). Never null.
    SnapshotId
}

u64_id! {
    /// An evaluation run (engine-output-contract §7 model scorecard). Never null.
    EvaluationRunId
}

u64_id! {
    /// An identity review item (engine-spec §4.4.3; engine-output-contract §7). Never null.
    ReviewId
}

/// A model family name (engine-spec §8.9 `model_type`), for example the per-layer families of
/// engine-spec §6.1. **Open** vocabulary: consumers tolerate unknown values. Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ModelFamily(pub String);

impl fmt::Display for ModelFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The build version of engine-spec §8.9 addition 1: the engine crate version plus the git commit
/// SHA. Never null; `git_sha` is `None` when the build was made outside a git checkout, which the
/// lineage then records as unknown rather than as an empty string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EngineVersion {
    /// The workspace package version (`CARGO_PKG_VERSION` of the engine crates).
    pub crate_version: String,
    /// The git commit SHA of the build, when known.
    pub git_sha: Option<String>,
}

impl fmt::Display for EngineVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.git_sha {
            Some(sha) => write!(f, "{}+{}", self.crate_version, sha),
            None => f.write_str(&self.crate_version),
        }
    }
}

/// A contract identifier such as `grid.plays` or `grid.output` (plays-contract §12,
/// engine-output-contract §9). Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ContractId(pub String);

impl fmt::Display for ContractId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A contract version `MAJOR.MINOR` (engine-output-contract §9; plays-contract §12).
///
/// A reader MUST reject a MAJOR it does not implement and MUST tolerate an additive MINOR. The
/// derived `Ord` is lexicographic (major, then minor). Never null.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ContractVersion {
    /// Incremented by a breaking change.
    pub major: u16,
    /// Incremented by an additive change.
    pub minor: u16,
}

impl ContractVersion {
    /// Builds a version.
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }
}

impl fmt::Display for ContractVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn week(w: u8) -> Week {
        Week::new(w).expect("non-zero week")
    }

    #[test]
    fn week_zero_is_rejected() {
        let err = Week::new(0).expect_err("week 0 must be rejected (KI-A4)");
        assert!(matches!(err, ContractError::InvalidValue { ref field, .. } if field == "week"));
        assert!(Week::try_from(0u8).is_err());
        assert_eq!(Week::try_from(18u8).map(Week::get), Ok(18));
        assert_eq!(u8::from(week(3)), 3);
    }

    #[test]
    fn slot_order_is_lexicographic_across_a_season_seam() {
        let late_2022 = Slot::new(Season(2022), week(18));
        let early_2023 = Slot::new(Season(2023), week(1));
        let mid_2023 = Slot::new(Season(2023), week(9));
        // A week-only comparison would put week 1 before week 18; the season decides first.
        assert!(late_2022 < early_2023);
        assert!(early_2023 < mid_2023);
        assert!(late_2022 < mid_2023);
        assert_eq!(late_2022.cmp(&late_2022), Ordering::Equal);
        let mut slots = vec![mid_2023, late_2022, early_2023];
        slots.sort();
        assert_eq!(slots, vec![late_2022, early_2023, mid_2023]);
    }

    #[test]
    fn player_key_orders_by_string_form() {
        let mut keys = [
            PlayerKey::Synthetic(100),
            PlayerKey::Synthetic(1),
            PlayerKey::Synthetic(10),
            PlayerKey::Synthetic(0),
            PlayerKey::Synthetic(2),
        ];
        keys.sort();
        let shown: Vec<String> = keys.iter().map(ToString::to_string).collect();
        assert_eq!(shown, ["0", "1", "10", "100", "2"]);
        assert!(PlayerKey::Synthetic(0) < PlayerKey::Synthetic(1));
        assert!(PlayerKey::Synthetic(1) < PlayerKey::Synthetic(10));
        assert!(PlayerKey::Synthetic(10) < PlayerKey::Synthetic(100));
        // GSIS ids order lexicographically as text as well.
        assert!(PlayerKey::Gsis("00-0033873".into()) < PlayerKey::Gsis("00-0034796".into()));
    }

    #[test]
    fn player_key_order_agrees_with_structural_equality() {
        // Equal string forms across variants are not equal keys, and Ord must say so.
        let gsis = PlayerKey::Gsis("5".into());
        let synthetic = PlayerKey::Synthetic(5);
        assert_ne!(gsis, synthetic);
        assert_ne!(gsis.cmp(&synthetic), Ordering::Equal);
        assert_eq!(gsis.cmp(&gsis), Ordering::Equal);
        assert_eq!(synthetic.cmp(&PlayerKey::Synthetic(5)), Ordering::Equal);
    }

    #[test]
    fn team_key_orders_natively() {
        let mut synthetic = vec![
            TeamKey::Synthetic(10),
            TeamKey::Synthetic(2),
            TeamKey::Synthetic(0),
        ];
        synthetic.sort();
        assert_eq!(
            synthetic,
            vec![
                TeamKey::Synthetic(0),
                TeamKey::Synthetic(2),
                TeamKey::Synthetic(10)
            ]
        );
        let mut nfl = vec![
            TeamKey::Nfl("SF".into()),
            TeamKey::Nfl("ARI".into()),
            TeamKey::Nfl("KC".into()),
        ];
        nfl.sort();
        assert_eq!(
            nfl,
            vec![
                TeamKey::Nfl("ARI".into()),
                TeamKey::Nfl("KC".into()),
                TeamKey::Nfl("SF".into())
            ]
        );
        assert!(TeamKey::Synthetic(u16::MAX) < TeamKey::Nfl("ARI".into()));
    }

    #[test]
    fn contract_version_orders_major_then_minor() {
        assert!(ContractVersion::new(1, 9) < ContractVersion::new(2, 0));
        assert!(ContractVersion::new(1, 0) < ContractVersion::new(1, 1));
        assert_eq!(ContractVersion::new(1, 0).to_string(), "1.0");
    }

    #[test]
    fn display_forms() {
        assert_eq!(Slot::new(Season(2023), week(4)).to_string(), "(2023, 4)");
        assert_eq!(
            PlayKey {
                game_id: GameId("2023_01_ARI_WAS".into()),
                play_id: 42
            }
            .to_string(),
            "2023_01_ARI_WAS#42"
        );
        assert_eq!(
            EngineVersion {
                crate_version: "0.1.0-alpha.1".into(),
                git_sha: Some("abc123".into())
            }
            .to_string(),
            "0.1.0-alpha.1+abc123"
        );
        assert_eq!(
            EngineVersion {
                crate_version: "0.1.0-alpha.1".into(),
                git_sha: None
            }
            .to_string(),
            "0.1.0-alpha.1"
        );
        assert_eq!(
            ScoringProfileRef {
                id: ScoringProfileId("half-ppr".into()),
                version: ScoringProfileVersion(1)
            }
            .to_string(),
            "half-ppr@1"
        );
    }
}
