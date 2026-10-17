//! The typed error vocabulary (engine-spec §6.8 rule 4, §8.10; plays-contract §11;
//! engine-output-contract §3.9; `evaluation-and-leakage.md` §4.1).
//!
//! Every failure in the engine is typed. Nothing here is a string-only error, and nothing in this
//! crate converts a typed error into a silent default (DR-B6). `Display` and
//! [`std::error::Error`] are implemented by hand (ADR-014 decision 4) so that the error texts
//! are visible in one place and the crate keeps a single external dependency.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ids::{ContractVersion, DriveKey, PlayKey, PlayerKey, Season, TeamKey};

/// The closed enumeration of engine error codes (engine-spec §8.4 `TrainingStatus::Failed`,
/// §8.10 "`error` stores a typed error code plus a message").
///
/// **Closed** enum: adding a value is a MAJOR change of every contract that carries it. The
/// variants are the failure classes the spec names as typed failures; `Internal` is the only
/// catch-all and maps to the process exit code [`crate::status::EXIT_INTERNAL`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EngineErrorCode {
    /// An input frame or record violated its contract (a [`ContractError`]).
    ContractViolation,
    /// Information from after the as-of point reached a stage (a [`LeakageError`]).
    Leakage,
    /// Persisted state failed its schema or integrity check; never reinitialised silently (KI-A8).
    StateCorrupt,
    /// A linear system or covariance was ill-conditioned beyond the declared threshold.
    IllConditioned,
    /// An iterative solver did not converge within its declared budget.
    NotConverged,
    /// A probability left `[0, 1]`.
    InvalidProbability,
    /// Team-role shares summed above 1.
    ShareOverflow,
    /// A stat line is impossible (for example completions above attempts).
    ImpossibleStat,
    /// A NaN or an infinity appeared where a finite number is required.
    NonFinite,
    /// A required input was absent; never replaced by a default (KI-P1).
    MissingData,
    /// Participation was not published as of the lock, or never produced (DR-C1).
    ParticipationUnavailable,
    /// A declared source has no producer or could not be reached.
    SourceUnavailable,
    /// A contract, state or fixture version this build does not implement.
    UnsupportedVersion,
    /// A scoring profile contained a threshold bonus or another non-affine rule (DR-D6).
    NonAffineScoringRule,
    /// A defender on the field has no rating coverage (engine-spec §6.2 Component 3).
    UncoveredDefender,
    /// The evaluation player pool is too small for the declared metric.
    InsufficientPool,
    /// Any other failure. Maps to the internal-error exit code.
    Internal,
}

impl EngineErrorCode {
    /// Every code, in declaration order, for exhaustive listings and tests.
    pub const ALL: [EngineErrorCode; 17] = [
        EngineErrorCode::ContractViolation,
        EngineErrorCode::Leakage,
        EngineErrorCode::StateCorrupt,
        EngineErrorCode::IllConditioned,
        EngineErrorCode::NotConverged,
        EngineErrorCode::InvalidProbability,
        EngineErrorCode::ShareOverflow,
        EngineErrorCode::ImpossibleStat,
        EngineErrorCode::NonFinite,
        EngineErrorCode::MissingData,
        EngineErrorCode::ParticipationUnavailable,
        EngineErrorCode::SourceUnavailable,
        EngineErrorCode::UnsupportedVersion,
        EngineErrorCode::NonAffineScoringRule,
        EngineErrorCode::UncoveredDefender,
        EngineErrorCode::InsufficientPool,
        EngineErrorCode::Internal,
    ];

    /// The code's stable name, equal to its serialized form.
    pub const fn name(self) -> &'static str {
        match self {
            EngineErrorCode::ContractViolation => "ContractViolation",
            EngineErrorCode::Leakage => "Leakage",
            EngineErrorCode::StateCorrupt => "StateCorrupt",
            EngineErrorCode::IllConditioned => "IllConditioned",
            EngineErrorCode::NotConverged => "NotConverged",
            EngineErrorCode::InvalidProbability => "InvalidProbability",
            EngineErrorCode::ShareOverflow => "ShareOverflow",
            EngineErrorCode::ImpossibleStat => "ImpossibleStat",
            EngineErrorCode::NonFinite => "NonFinite",
            EngineErrorCode::MissingData => "MissingData",
            EngineErrorCode::ParticipationUnavailable => "ParticipationUnavailable",
            EngineErrorCode::SourceUnavailable => "SourceUnavailable",
            EngineErrorCode::UnsupportedVersion => "UnsupportedVersion",
            EngineErrorCode::NonAffineScoringRule => "NonAffineScoringRule",
            EngineErrorCode::UncoveredDefender => "UncoveredDefender",
            EngineErrorCode::InsufficientPool => "InsufficientPool",
            EngineErrorCode::Internal => "Internal",
        }
    }
}

impl fmt::Display for EngineErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A typed engine error: a closed code plus a human-readable message (engine-spec §8.10).
///
/// The message is for operators and logs; consumers branch on `code`. Never null where an error
/// is reported.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EngineError {
    /// The closed error code.
    pub code: EngineErrorCode,
    /// A message naming what failed. Contains no credential, absolute path or host name
    /// (engine-output-contract §8 rule 5).
    pub message: String,
}

impl EngineError {
    /// Builds an error from a code and a message.
    pub fn new(code: EngineErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for EngineError {}

/// A violation of the `grid.plays` contract (plays-contract §7, §9, §11, §12), raised at frame
/// construction or validation. Nothing is coerced to a default.
///
/// P1-01 defines the sixteen variants plays-contract §11 lists. Each payload names the offending
/// key, field or drive so that ingestion can quarantine the row and the data-quality report can
/// list it (DR-B6).
///
/// **Open** enum (`#[non_exhaustive]`): plays-contract §11 never declares its list closed, and
/// the model specifications name further variants that the packages implementing them add
/// (`evaluation-and-leakage.md` §5.4 `MissingSeasonKey`, added by P1-05; `value-model.md` §4.6
/// `InvalidLabel`, `NonFinite`, `DriveStructure`, `SentinelOnContinuingRow`;
/// `rapm-attribution.md` §5.4 `NonFinitePrior`; `projection-stack.md` §5.3 `MissingColumn`,
/// `UnknownPosition`). A `match` outside this crate therefore needs a wildcard arm.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ContractError {
    /// The frame's contract MAJOR is not implemented by this reader (plays-contract §12). The
    /// parser is never "made flexible".
    UnsupportedVersion {
        /// The version the frame declares.
        found: ContractVersion,
        /// The version this reader implements.
        supported: ContractVersion,
    },
    /// A required field or column is absent. A season-less plays frame cannot be built (every
    /// [`PlayRecord`](crate::plays::PlayRecord) requires a `Season`); the frame reader that
    /// meets a source without a season column raises `MissingSeasonKey`, which
    /// `evaluation-and-leakage.md` §5.4 names and P1-05 adds, never a week-only fallback.
    MissingField {
        /// The missing field, by its contract name.
        field: String,
    },
    /// A field holds a value outside its declared domain (core rule 5, real-data rule 1, a zero
    /// week).
    InvalidValue {
        /// The offending field, by its contract name.
        field: String,
        /// What was found and why it is invalid.
        detail: String,
    },
    /// The last row of a drive is not `Terminal` (core rule 2).
    NonTerminalLastRow {
        /// The drive whose last row continues.
        drive: DriveKey,
    },
    /// A drive holds more than one `Terminal` row (core rule 2).
    MultipleTerminals {
        /// The drive with more than one terminal row.
        drive: DriveKey,
    },
    /// `drive_points` differs between rows of one drive (core rule 3).
    DrivePointsNotConstant {
        /// The drive whose label is not constant.
        drive: DriveKey,
    },
    /// `off_team` or `def_team` changes within a drive (core rule 4; plays-contract D-4).
    PossessionChangeWithinDrive {
        /// The drive that spans a change of possession.
        drive: DriveKey,
    },
    /// A team on a play is absent from `players.team` (core rule 6).
    UnknownTeam {
        /// The unknown team.
        team: TeamKey,
        /// The first play that names it.
        play: PlayKey,
    },
    /// Participants absent from `players.player_id` (core rule 6; DR-B6). The count covers the
    /// whole frame so the data-quality report can state the scale; the ids are never dropped.
    UnknownParticipant {
        /// How many participant entries, across the frame, name an unknown player.
        count: u64,
    },
    /// A player id appears twice on one side of one play (real-data rule 2). The legacy
    /// synthetic profile allows it (multiset, plays-contract D-8).
    DuplicateParticipant {
        /// The play with the duplicated id.
        play: PlayKey,
        /// The duplicated player.
        player_id: PlayerKey,
    },
    /// A key is not unique: `player_id` within `players`, `(game_id, play_id)` within `plays`
    /// (core rule 7; plays-contract D-6).
    DuplicateKey {
        /// The frame whose key repeats: `players` or `plays`.
        frame: String,
        /// The repeated key, in its display form.
        key: String,
    },
    /// A source value is outside the closed normalization table (plays-contract D-3), or a
    /// column's type drifted from the contract (D-12).
    SchemaDrift {
        /// The field whose vocabulary or type drifted.
        field: String,
        /// The unexpected value or type.
        detail: String,
    },
    /// Participation for the season was not published as of the lock, or was never produced
    /// (plays-contract D-5, §9; DR-C1).
    ParticipationUnavailable {
        /// The season without usable participation.
        season: Season,
    },
    /// A clock-dependent mask was requested on a frame without a clock (plays-contract D-1).
    ClockUnavailable {
        /// The first play without a clock.
        play: PlayKey,
    },
    /// A pre-game record carries no timestamp (plays-contract §9: "a record with no timestamp is a
    /// typed error"; a market quote without `quoted_at`, an undated intervention).
    MissingTimestamp {
        /// The record kind and key, for example `market quote (2023, 5, KC)`.
        record: String,
    },
    /// A declared source has no producer or could not be read (plays-contract D-7).
    SourceUnavailable {
        /// The source name.
        source: String,
    },
}

impl fmt::Display for ContractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContractError::UnsupportedVersion { found, supported } => write!(
                f,
                "unsupported contract version {found}: this reader implements major {}",
                supported.major
            ),
            ContractError::MissingField { field } => write!(f, "missing required field `{field}`"),
            ContractError::InvalidValue { field, detail } => {
                write!(f, "invalid value in `{field}`: {detail}")
            }
            ContractError::NonTerminalLastRow { drive } => {
                write!(f, "last row of {drive} is not terminal")
            }
            ContractError::MultipleTerminals { drive } => {
                write!(f, "{drive} has more than one terminal row")
            }
            ContractError::DrivePointsNotConstant { drive } => {
                write!(f, "drive_points is not constant within {drive}")
            }
            ContractError::PossessionChangeWithinDrive { drive } => {
                write!(f, "possession changes within {drive}")
            }
            ContractError::UnknownTeam { team, play } => {
                write!(
                    f,
                    "team `{team}` on play {play} is not in the players frame"
                )
            }
            ContractError::UnknownParticipant { count } => {
                write!(
                    f,
                    "{count} participant entries name a player not in the players frame"
                )
            }
            ContractError::DuplicateParticipant { play, player_id } => {
                write!(
                    f,
                    "player `{player_id}` is listed twice on one side of play {play}"
                )
            }
            ContractError::DuplicateKey { frame, key } => {
                write!(f, "duplicate key `{key}` in frame `{frame}`")
            }
            ContractError::SchemaDrift { field, detail } => {
                write!(f, "schema drift in `{field}`: {detail}")
            }
            ContractError::ParticipationUnavailable { season } => {
                write!(
                    f,
                    "participation for season {season} is not available as of the lock"
                )
            }
            ContractError::ClockUnavailable { play } => {
                write!(f, "play {play} carries no game clock")
            }
            ContractError::MissingTimestamp { record } => {
                write!(f, "record {record} carries no timestamp")
            }
            ContractError::SourceUnavailable { source } => {
                write!(f, "source `{source}` is unavailable")
            }
        }
    }
}

impl std::error::Error for ContractError {}

/// The class of a leakage violation (engine-spec §4.5 "Leakage tests"; `evaluation-and-leakage.md`
/// §4.1, §4.2). **Closed** enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LeakageKind {
    /// A state's watermark is at or beyond the as-of slot (guard G2).
    Watermark,
    /// A record was published at or after the lock (the publication-lag axis).
    PublishedAfterLock,
    /// A record's event slot is beyond what its class may use at the as-of slot (the temporal
    /// axis), including a future-season low week (KI-NEW-Z61).
    FutureSlot,
    /// A record reached an as-of comparison without a season; a week-only comparison is never
    /// substituted.
    MissingSeason,
    /// An intervention, coaching change or other dated record carries no announcement time
    /// (guard G6).
    UndatedRecord,
}

impl fmt::Display for LeakageKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            LeakageKind::Watermark => "watermark",
            LeakageKind::PublishedAfterLock => "published after lock",
            LeakageKind::FutureSlot => "future slot",
            LeakageKind::MissingSeason => "missing season",
            LeakageKind::UndatedRecord => "undated record",
        })
    }
}

/// A typed leakage failure: information from after the as-of point reached a stage. It fails the
/// run; it is never logged and skipped (engine-spec §4.5).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LeakageError {
    /// The violated axis.
    pub kind: LeakageKind,
    /// What leaked and where, naming the slots and instants involved.
    pub detail: String,
}

impl fmt::Display for LeakageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "leakage ({}): {}", self.kind, self.detail)
    }
}

impl std::error::Error for LeakageError {}

/// The record invariants of engine-output-contract §3.9, plus the two shape invariants of the
/// §3.2 component table and §3.5 (`ComponentNotForPosition`, `DuplicateComponent`). A record
/// that violates one is **not emitted**; the run fails and the prior production projection stays
/// in force. **Closed** enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OutputInvariant {
    /// A probability (or a share, which lies in the same range) is outside `[0, 1]`.
    ProbabilityOutOfRange,
    /// Quantiles are not monotone (`q10 ≤ q25 ≤ q50 ≤ q75 ≤ q90`; `p10 ≤ … ≤ p90`).
    QuantilesNotMonotone,
    /// A standard deviation is negative.
    NegativeSd,
    /// A NaN or an infinity appears in a field.
    NonFinite,
    /// Team-role shares sum above 1; at record level, a single share above 1.
    ShareOverflow,
    /// Completions exceed pass attempts (per draw; at record level, in a summary statistic).
    CompletionsExceedAttempts,
    /// Receptions exceed targets (per draw; at record level, in a summary statistic).
    ReceptionsExceedTargets,
    /// An inactive draw is not the zero vector (per draw; at record level, a player with
    /// `p_active = 0` whose unconditional summaries are not all zero).
    InactiveDrawNotZero,
    /// A stat-vector summary carries a component that is not part of the record's position's
    /// target vector (engine-output-contract §3.2 table, §3.5 "for each §3.2 component";
    /// DR-D7: QB has no snap share and no receiving components, RB/WR/TE have no start
    /// probability).
    ComponentNotForPosition,
    /// A stat-vector summary carries the same component twice (engine-output-contract §3.5: one
    /// summary per component and block).
    DuplicateComponent,
}

impl fmt::Display for OutputInvariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            OutputInvariant::ProbabilityOutOfRange => "probability out of [0, 1]",
            OutputInvariant::QuantilesNotMonotone => "quantiles not monotone",
            OutputInvariant::NegativeSd => "negative standard deviation",
            OutputInvariant::NonFinite => "non-finite value",
            OutputInvariant::ShareOverflow => "share overflow",
            OutputInvariant::CompletionsExceedAttempts => "completions exceed attempts",
            OutputInvariant::ReceptionsExceedTargets => "receptions exceed targets",
            OutputInvariant::InactiveDrawNotZero => "inactive draw not zero",
            OutputInvariant::ComponentNotForPosition => "component not for position",
            OutputInvariant::DuplicateComponent => "duplicate component",
        })
    }
}

/// A typed §3.9 invariant violation on an output record. The record is never clamped or repaired
/// (engine-spec §6.8 rule 12).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OutputInvariantError {
    /// The violated invariant.
    pub invariant: OutputInvariant,
    /// The field path of the violation, for example `conditional.completions.q50`.
    pub field: String,
    /// The offending value(s).
    pub detail: String,
}

impl OutputInvariantError {
    /// Builds an invariant error.
    pub fn new(
        invariant: OutputInvariant,
        field: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            invariant,
            field: field.into(),
            detail: detail.into(),
        }
    }
}

impl fmt::Display for OutputInvariantError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "output invariant violated ({}) at `{}`: {}",
            self.invariant, self.field, self.detail
        )
    }
}

impl std::error::Error for OutputInvariantError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::GameId;
    use std::collections::HashSet;
    use std::error::Error;

    #[test]
    fn engine_error_codes_are_distinct_and_named() {
        let names: HashSet<&str> = EngineErrorCode::ALL.iter().map(|c| c.name()).collect();
        assert_eq!(names.len(), EngineErrorCode::ALL.len());
        assert_eq!(EngineErrorCode::ALL.len(), 17);
        let err = EngineError::new(EngineErrorCode::NotConverged, "CG exceeded 500 iterations");
        assert_eq!(err.to_string(), "NotConverged: CG exceeded 500 iterations");
        let dynamic: &dyn Error = &err;
        assert!(dynamic.source().is_none());
    }

    #[test]
    fn contract_errors_display_their_payload() {
        let drive = DriveKey {
            game_id: GameId("synth-2023-01-0-1".into()),
            drive_seq: 7,
        };
        let shown = ContractError::NonTerminalLastRow {
            drive: drive.clone(),
        }
        .to_string();
        assert_eq!(
            shown,
            "last row of synth-2023-01-0-1/drive 7 is not terminal"
        );
        let shown = ContractError::UnknownParticipant { count: 3 }.to_string();
        assert!(shown.starts_with("3 participant entries"));
        let shown = ContractError::UnsupportedVersion {
            found: ContractVersion::new(2, 0),
            supported: ContractVersion::new(1, 0),
        }
        .to_string();
        assert!(shown.contains("2.0") && shown.contains("major 1"));
        let boxed: Box<dyn Error> = Box::new(ContractError::MissingField {
            field: "season".into(),
        });
        assert_eq!(boxed.to_string(), "missing required field `season`");
    }

    #[test]
    fn leakage_and_invariant_errors_display() {
        let leak = LeakageError {
            kind: LeakageKind::PublishedAfterLock,
            detail: "market quote at 1001s, lock at 1000s".into(),
        };
        assert_eq!(
            leak.to_string(),
            "leakage (published after lock): market quote at 1001s, lock at 1000s"
        );
        let inv =
            OutputInvariantError::new(OutputInvariant::NegativeSd, "fantasy[0].sd", "sd = -1");
        assert_eq!(
            inv.to_string(),
            "output invariant violated (negative standard deviation) at `fantasy[0].sd`: sd = -1"
        );
    }
}
