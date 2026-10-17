//! The `grid.plays` v1 contract types and validator (`docs/03-contracts/plays-contract.md` §2.1,
//! §4–§7, §9, §11, §12; DR-C1, DR-C13).
//!
//! One row per modelled play. The v0 columns `terminal`, `terminal_value`, `points` and `n_*` are
//! **derived** from [`NextState`] and [`DrivePoints`]; the columnar v0 view that reproduces them,
//! NaN and −1 sentinels included, lives in `grid-synth` (`legacy_v0`). Nothing here reads the
//! oracle or a file.
//!
//! Validation ([`PlaysFrame::validate`]) applies core rules 1–7 of plays-contract §7 to every
//! producer and real-data rules 1–2 under the [`FrameProfile::RealData`] profile. Real-data rules
//! 3 (ids belong to their team on the as-of roster) and 4 (participation coverage ≥ 99 %) need the
//! as-of roster and the coverage policy: they are P1-03's and P1-05's. Every violation is a typed
//! [`ContractError`]; nothing is coerced.

use std::collections::{HashMap, HashSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::ContractError;
use crate::ids::{
    ContractId, ContractVersion, DriveKey, GameId, IngestionRunId, PlayKey, PlayerKey, Season,
    TeamKey, Week,
};
use crate::time::UtcInstant;

/// The contract id of the plays frames: `grid.plays` (plays-contract §12).
pub const GRID_PLAYS_CONTRACT_ID: &str = "grid.plays";

/// The contract version this crate implements: `grid.plays` v1.0 (plays-contract §12). A frame
/// whose MAJOR differs fails validation with [`ContractError::UnsupportedVersion`].
pub const GRID_PLAYS_CONTRACT_VERSION: ContractVersion = ContractVersion::new(1, 0);

/// The contract id as a typed [`ContractId`].
pub fn grid_plays_contract_id() -> ContractId {
    ContractId(GRID_PLAYS_CONTRACT_ID.to_owned())
}

/// Regular season or postseason (plays-contract §9, D-2). Engine policy is regular season only
/// for V(s), RAPM, features and labels (DR-D4, DR-C12); postseason rows are carried so that they
/// can be excluded explicitly rather than by omission. **Closed** enum. Never null.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SeasonType {
    /// A regular-season game.
    Regular,
    /// A postseason game.
    Post,
}

/// The game state `s` of a play (plays-contract §2.1 `down`, `ydstogo`, `yardline_100`).
///
/// Units: `down` in `1..=4`; `ydstogo` yards to a first down, `≥ 1`; `yardline_100` yards to the
/// opponent goal line, `≥ 1`, and `≤ 99` under the real-data profile (the legacy synthetic world
/// reaches 104: plays-contract D-9). Never null.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayState {
    /// The down, `1..=4`.
    pub down: u8,
    /// Yards to a first down, `≥ 1`.
    pub ydstogo: u8,
    /// Yards to the opponent goal line, `≥ 1`.
    pub yardline_100: u8,
}

/// The drive's eventual points, broadcast to every row of the drive: the V(s) training label
/// (plays-contract §2.1 `drive_points`; DR-C13 keeps the 7/3/0 vocabulary for v1). **Closed**
/// enum. Never null.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DrivePoints {
    /// No score on the drive: 0 points. Defensive scores and safeties also map here in v1
    /// (plays-contract D-3; a signed-EP label would be a v2 change).
    Zero,
    /// A field goal: 3 points.
    FieldGoal,
    /// A touchdown: 7 points.
    Touchdown,
}

impl DrivePoints {
    /// The points value: 0, 3 or 7.
    pub const fn points(self) -> u8 {
        match self {
            DrivePoints::Zero => 0,
            DrivePoints::FieldGoal => 3,
            DrivePoints::Touchdown => 7,
        }
    }
}

/// The next state `s′` of a play (plays-contract §2.1 `n_down`, `n_ydstogo`, `n_yardline_100`,
/// `terminal`). Replaces the v0 `terminal` flag and the −1 sentinels. **Closed** enum. Never
/// null.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NextState {
    /// The drive continues: `s′` is the state of the next modelled row of the same drive.
    Continue(PlayState),
    /// The last modelled row of the drive; the realized value is the drive's `drive_points`.
    Terminal,
}

/// The game clock of a play (plays-contract D-1). Required by the `two_minute` mask
/// (`half_seconds_remaining ≤ 120`, DR-C13). Synthetic frames have no clock, so a clock-dependent
/// mask on them is [`ContractError::ClockUnavailable`], never silently omitted.
///
/// Units: `qtr` quarter number as nflverse publishes it (the contract fixes no domain);
/// `half_seconds_remaining` seconds left in the half. Nullable at row level (`Option<GameClock>`):
/// `None` means the producer has no clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GameClock {
    /// The quarter.
    pub qtr: u8,
    /// Seconds remaining in the half.
    pub half_seconds_remaining: u16,
}

/// On-field participation of a play (plays-contract §2.1 `off_players`, `def_players`; D-5, D-8).
///
/// Order is kept as listed. Under the legacy synthetic profile a side is a **multiset** (a
/// duplicated id counts twice in the design, plays-contract D-8); the real-data profile forbids
/// duplicates (real-data rule 2). "Not available" and "unmatched" are distinct from "empty": the
/// oracle's `()` for all three let RAPM fit a team-intercept-only design without error (KI-NEW-Z48).
/// **Closed** enum. Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Participation {
    /// Participation is listed for this play.
    Listed {
        /// Offensive skill players on the field (QB, RB, WR, TE, FB on real data). OL never
        /// appears: the offense intercept absorbs it.
        offense: Vec<PlayerKey>,
        /// Defenders on the field, as listed.
        defense: Vec<PlayerKey>,
    },
    /// Participation was published, but no row matched this play.
    Unmatched,
    /// Participation was not published as of the lock, or was never produced.
    NotAvailable,
}

/// One modelled play (plays-contract §2.1, §9, §11). Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayRecord {
    /// The game (explicit in v1; v0 embedded it in `drive_id`).
    pub game_id: GameId,
    /// The play id, unique within the game.
    pub play_id: u32,
    /// The drive sequence within the game.
    pub drive_seq: u16,
    /// The season. Required: a frame without a season is a contract error, never a week-only
    /// as-of comparison.
    pub season: Season,
    /// Regular season or postseason.
    pub season_type: SeasonType,
    /// The week within the season (never 0).
    pub week: Week,
    /// The possession team.
    pub off_team: TeamKey,
    /// The defending team.
    pub def_team: TeamKey,
    /// The state `s` before the play.
    pub state: PlayState,
    /// The game clock, when the producer has one.
    pub clock: Option<GameClock>,
    /// Yards gained on the play. Units: yards; may be negative. Carried for traceability; no
    /// engine stage reads it (plays-contract §1).
    pub yards: i16,
    /// The next state `s′`, or `Terminal`.
    pub next: NextState,
    /// The drive's eventual points, constant within the drive.
    pub drive_points: DrivePoints,
    /// On-field participation.
    pub participation: Participation,
}

impl PlayRecord {
    /// The play's key `(game_id, play_id)`.
    pub fn play_key(&self) -> PlayKey {
        PlayKey {
            game_id: self.game_id.clone(),
            play_id: self.play_id,
        }
    }

    /// The play's drive key `(game_id, drive_seq)`.
    pub fn drive_key(&self) -> DriveKey {
        DriveKey {
            game_id: self.game_id.clone(),
            drive_seq: self.drive_seq,
        }
    }
}

/// The validation profile a frame is held to (plays-contract §7). Serialized as
/// `synthetic-legacy` / `real-data`. **Closed** enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FrameProfile {
    /// The v0 synthetic world as the oracle generated it: exempt from the real-data rules
    /// (duplicate participants, `yardline_100` up to 104, `ydstogo > yardline_100`; D-8, D-9).
    SyntheticLegacy,
    /// nflverse data and the corrected synthetic world: the real-data rules apply.
    RealData,
}

impl fmt::Display for FrameProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            FrameProfile::SyntheticLegacy => "synthetic-legacy",
            FrameProfile::RealData => "real-data",
        })
    }
}

/// Who produced the frame (plays-contract §9 `producer`). **Closed** enum. Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FrameProducer {
    /// The synthetic generator.
    Synthetic {
        /// The generator version (`legacy-59bce1d` for the oracle's world).
        generator_version: String,
        /// A content hash of the generator configuration.
        config_hash: String,
        /// Every seed used, named (engine-spec §8.9 addition 4).
        seeds: Vec<(String, u64)>,
        /// Which validation profile the world satisfies.
        profile: FrameProfile,
    },
    /// The nflverse adapter (P1-03).
    Nflverse {
        /// The adapter version.
        adapter_version: String,
        /// The version of the closed normalization map (plays-contract D-3).
        normalization_map_version: String,
    },
}

impl FrameProducer {
    /// The validation profile: the declared one for a synthetic frame, `RealData` for nflverse.
    pub fn profile(&self) -> FrameProfile {
        match self {
            FrameProducer::Synthetic { profile, .. } => *profile,
            FrameProducer::Nflverse { .. } => FrameProfile::RealData,
        }
    }
}

/// One source dataset partition that fed a frame (plays-contract §9 `sources`; superseded
/// alpha-spec §4.1.1 freshness fields; engine-spec §4.5 timestamps). Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceSnapshot {
    /// The dataset name, for example `nflverse/pbp`.
    pub source_name: String,
    /// The provider's version or release tag of the dataset.
    pub source_version: String,
    /// The source **publication** timestamp: the time the datum became knowable. Gates
    /// eligibility at a lock (engine-spec §4.5).
    pub source_timestamp: UtcInstant,
    /// When the engine retrieved it. Never gates eligibility for backfilled data.
    pub retrieved_at: UtcInstant,
    /// When the partition became available to the engine's as-of views.
    pub available_at: UtcInstant,
    /// The ingestion run that retrieved it; `None` for a synthetic frame no run produced.
    pub ingestion_run_id: Option<IngestionRunId>,
    /// The sha256 of the retained raw partition, lowercase hex.
    pub sha256: String,
    /// Row count of the partition.
    pub row_count: u64,
    /// The provider schema version the partition was validated against.
    pub schema_version: String,
    /// The validation outcome recorded by ingestion (provider vocabulary; open).
    pub validation_status: String,
}

/// Where published participation came from (plays-contract §9). **Closed** enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ParticipationProvenance {
    /// Next Gen Stats participation.
    Ngs,
    /// FTN participation.
    Ftn,
}

/// The publication status of a season's participation (plays-contract §9; DR-C1). **Closed**
/// enum. Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ParticipationStatus {
    /// Published at `at` by the named provenance. Usable only at locks after `at`.
    Published {
        /// The publication instant.
        at: UtcInstant,
        /// Who published it.
        provenance: ParticipationProvenance,
    },
    /// Not published as of the frame's construction.
    NotPublished,
    /// Synthetic participation, generated with the world.
    Synthetic,
}

/// Participation coverage per season (plays-contract §9 "coverage statistics", as
/// `ingest_grid.py:29-39` writes them). Units: fraction of rows in `[0, 1]`. Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParticipationCoverage {
    /// The season.
    pub season: Season,
    /// Share of rows with listed offensive participation.
    pub offense_share: f64,
    /// Share of rows with listed defensive participation.
    pub defense_share: f64,
}

/// Frame-level metadata (plays-contract §9). Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaysFrameMeta {
    /// The contract version the frame was built to.
    pub contract: ContractVersion,
    /// Who produced the frame.
    pub producer: FrameProducer,
    /// One entry per dataset partition and season that fed the frame.
    pub sources: Vec<SourceSnapshot>,
    /// Publication status of participation, per season.
    pub participation_availability: Vec<(Season, ParticipationStatus)>,
    /// Coverage statistics, per season.
    pub coverage: Vec<ParticipationCoverage>,
}

/// The plays frame: metadata plus rows in file order (plays-contract §2, §9). Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaysFrame {
    /// Frame-level metadata.
    pub meta: PlaysFrameMeta,
    /// The plays, in file order. Rows of one drive are contiguous and chronological.
    pub plays: Vec<PlayRecord>,
}

/// A participant's position (plays-contract §4 `position`; D-12).
///
/// **Open** vocabulary: the known labels are variants and anything else is `Other(raw)`.
/// Serialized as the raw string (`QB`, `RB`, `WR`, `TE`, `FB`, `DEF`, `CB`, or the raw label),
/// and parsed back case-sensitively.
///
/// A reader never produces `Other` with a known label: deserialization (`From<String>`) maps a
/// known label to its variant. `Other("QB")` can still be built by hand; it serializes to the same
/// text `"QB"` as `Qb`, and therefore deserializes to `Qb`. Equality after a serialization round
/// trip holds only for values a reader produced. The synthetic↔real mapping is owned by `rapm-attribution.md` (open). Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum ParticipantPosition {
    /// Quarterback.
    Qb,
    /// Running back.
    Rb,
    /// Wide receiver.
    Wr,
    /// Tight end.
    Te,
    /// Fullback.
    Fb,
    /// The synthetic world's undifferentiated defender label.
    Def,
    /// Cornerback (the WR–CB proxy when `cb_split` is on).
    Cb,
    /// Any other provider label, kept verbatim.
    Other(String),
}

impl ParticipantPosition {
    /// The raw label.
    pub fn as_str(&self) -> &str {
        match self {
            ParticipantPosition::Qb => "QB",
            ParticipantPosition::Rb => "RB",
            ParticipantPosition::Wr => "WR",
            ParticipantPosition::Te => "TE",
            ParticipantPosition::Fb => "FB",
            ParticipantPosition::Def => "DEF",
            ParticipantPosition::Cb => "CB",
            ParticipantPosition::Other(raw) => raw,
        }
    }
}

impl From<String> for ParticipantPosition {
    fn from(raw: String) -> Self {
        match raw.as_str() {
            "QB" => ParticipantPosition::Qb,
            "RB" => ParticipantPosition::Rb,
            "WR" => ParticipantPosition::Wr,
            "TE" => ParticipantPosition::Te,
            "FB" => ParticipantPosition::Fb,
            "DEF" => ParticipantPosition::Def,
            "CB" => ParticipantPosition::Cb,
            _ => ParticipantPosition::Other(raw),
        }
    }
}

impl From<&str> for ParticipantPosition {
    fn from(raw: &str) -> Self {
        Self::from(raw.to_owned())
    }
}

impl From<ParticipantPosition> for String {
    fn from(position: ParticipantPosition) -> Self {
        position.as_str().to_owned()
    }
}

impl fmt::Display for ParticipantPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One row of the `players` frame (plays-contract §4). The v0 truth columns `ability` and
/// `is_starter` are **not** here: they live in `grid-synth`'s `SyntheticTruth`, which estimator
/// code cannot reach (D-13). Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerRecord {
    /// The universe key.
    pub player_id: PlayerKey,
    /// The player's team as of the frame's construction.
    pub team: TeamKey,
    /// The position label.
    pub position: ParticipantPosition,
}

/// The `players` frame: the player universe as of the projection time (plays-contract §4, §9).
/// Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayersFrame {
    /// The players, `player_id` unique.
    pub players: Vec<PlayerRecord>,
}

/// The units of a market quote's `strength` (plays-contract §5 v1 "`strength` (with declared
/// units)"; `docs/05-model-specs/synthetic-world.md` §4.10 G-5). **Closed** enum: a new unit is a
/// MAJOR change of `grid.plays`. Serialized as the variant name (`DvPerPlay`,
/// `SyntheticAbility`).
///
/// DR-D10 (the spread → EP-per-play conversion of a real market line) is **open, with no
/// default**. P1-01 therefore fixes **no real-data market unit**: no real-data market reader
/// exists, and none may label a converted line until DR-D10 is decided. The two variants are
/// the units the contract and the synthetic harness name today; the plays-contract §9 sketch
/// lists only `DvPerPlay`, and `SyntheticAbility` is added here from synthetic-world.md G-5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MarketUnits {
    /// Expected drive points per play, the `dV` scale the Layer-3 estimand is stated in
    /// (rapm-attribution.md). No real-data line is converted onto it until DR-D10 is decided.
    DvPerPlay,
    /// The synthetic harness's ability units: `market_t = κ·N*(t) + N(0, σ_m²)` with `κ = 1` and
    /// `σ_m = 0.01`, kept until DR-D10 fixes the spread → EP-per-play conversion
    /// (`docs/05-model-specs/synthetic-world.md` §4.10 G-5; KI-NEW-Z12). No Layer-3 recovery
    /// claim is made from a quote in these units. Synthetic frames only.
    SyntheticAbility,
}

/// A market-implied net team strength quote as published at or before a lock (plays-contract §5
/// v1; DR-B5; engine-spec §4.3). A team with no eligible quote gets **no** anchor row; a `0.0`
/// default is forbidden (KI-NEW-Z71). Never null; a quote without `quoted_at` cannot be built.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketQuote {
    /// The season of the game the quote refers to.
    pub season: Season,
    /// The week of the game the quote refers to.
    pub week: Week,
    /// The team.
    pub team: TeamKey,
    /// Net team strength (offense plus defense quality) in `units`.
    pub strength: f64,
    /// The units of `strength`.
    pub units: MarketUnits,
    /// The observation timestamp of the line. Gates eligibility: a closing line is post-lock
    /// for any game after the lock.
    pub quoted_at: UtcInstant,
    /// The source name (no odds provider is selected: DR-D2).
    pub source: String,
}

/// One feeder-league record (plays-contract §6). Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollegeRecord {
    /// Joins to NFL ratings.
    pub player_id: PlayerKey,
    /// Selects the position-specific prior scale.
    pub position: ParticipantPosition,
    /// The feeder-league situational value. Units: the feeder's `dV`-like scale before
    /// equivalency translation.
    pub feeder_sv: f64,
    /// Feeder snaps; produced by the synthetic generator and read by no consumer. `None` when
    /// the source does not carry it.
    pub feeder_snaps: Option<u32>,
    /// Rookies are excluded from the equivalency fit and receive priors.
    pub is_rookie: bool,
}

/// The `college` frame (plays-contract §6). Never null; empty when no feeder source exists.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollegeFrame {
    /// The rows.
    pub rows: Vec<CollegeRecord>,
}

impl PlaysFrame {
    /// Validates the frame against plays-contract §7: core rules 1–7 for every producer and, when
    /// the producer's profile is [`FrameProfile::RealData`], real-data rules 1 and 2. The frame's
    /// contract MAJOR must be the one this crate implements (plays-contract §12).
    ///
    /// Rule semantics as checked here:
    ///
    /// 1. Within a drive, rows are contiguous in file order; every non-last row is
    ///    `Continue(s′)` with `s′` equal to the following row's `state`.
    /// 2. Exactly one `Terminal` row per drive, and it is the last row.
    /// 3. `drive_points` is constant within a drive.
    /// 4. `off_team ≠ def_team`, and both are constant within a drive.
    /// 5. `down ∈ 1..=4`, `ydstogo ≥ 1`, `yardline_100 ≥ 1`.
    /// 6. Every team is in `players.team`; every participant is in `players.player_id`
    ///    (unknown participants are counted across the frame).
    /// 7. `player_id` is unique in `players`; `(game_id, play_id)` is unique in `plays`.
    ///
    /// Real-data rule 1: `yardline_100 ≤ 99` and `ydstogo ≤ yardline_100`. Real-data rule 2: no
    /// duplicate id within a side. The legacy synthetic profile is exempt from both (D-8, D-9).
    /// Real-data rules 3 and 4 are not checked here (see the module doc).
    ///
    /// Returns the first violation found, as a typed [`ContractError`]. Nothing is coerced.
    pub fn validate(&self, players: &PlayersFrame) -> Result<(), ContractError> {
        if self.meta.contract.major != GRID_PLAYS_CONTRACT_VERSION.major {
            return Err(ContractError::UnsupportedVersion {
                found: self.meta.contract,
                supported: GRID_PLAYS_CONTRACT_VERSION,
            });
        }
        let profile = self.meta.producer.profile();

        // Rule 7 (players side): player_id unique.
        let mut known_players: HashSet<&PlayerKey> = HashSet::with_capacity(players.players.len());
        let mut known_teams: HashSet<&TeamKey> = HashSet::new();
        for player in &players.players {
            if !known_players.insert(&player.player_id) {
                return Err(ContractError::DuplicateKey {
                    frame: "players".to_owned(),
                    key: player.player_id.to_string(),
                });
            }
            known_teams.insert(&player.team);
        }

        // Rule 7 (plays side): (game_id, play_id) unique.
        let mut seen_plays: HashSet<(&GameId, u32)> = HashSet::with_capacity(self.plays.len());
        // Rule 1: a drive's rows are contiguous in file order.
        let mut closed_drives: HashSet<DriveKey> = HashSet::new();
        let mut unknown_participants: u64 = 0;

        let mut start = 0usize;
        while start < self.plays.len() {
            let drive = self.plays[start].drive_key();
            let mut end = start;
            while end < self.plays.len() && self.plays[end].drive_key() == drive {
                end += 1;
            }
            if !closed_drives.insert(drive.clone()) {
                return Err(ContractError::InvalidValue {
                    field: "drive_seq".to_owned(),
                    detail: format!("rows of {drive} are not contiguous in file order (rule 1)"),
                });
            }
            let rows = &self.plays[start..end];
            Self::validate_drive(&drive, rows)?;

            for play in rows {
                let key = play.play_key();
                if !seen_plays.insert((&play.game_id, play.play_id)) {
                    return Err(ContractError::DuplicateKey {
                        frame: "plays".to_owned(),
                        key: key.to_string(),
                    });
                }
                Self::validate_state(&key, &play.state, profile)?;
                for team in [&play.off_team, &play.def_team] {
                    if !known_teams.contains(team) {
                        return Err(ContractError::UnknownTeam {
                            team: team.clone(),
                            play: key,
                        });
                    }
                }
                if let Participation::Listed { offense, defense } = &play.participation {
                    for side in [offense, defense] {
                        if profile == FrameProfile::RealData {
                            let mut seen_side: HashSet<&PlayerKey> =
                                HashSet::with_capacity(side.len());
                            for id in side {
                                if !seen_side.insert(id) {
                                    return Err(ContractError::DuplicateParticipant {
                                        play: key.clone(),
                                        player_id: id.clone(),
                                    });
                                }
                            }
                        }
                        unknown_participants +=
                            side.iter().filter(|id| !known_players.contains(id)).count() as u64;
                    }
                }
            }
            start = end;
        }

        if unknown_participants > 0 {
            return Err(ContractError::UnknownParticipant {
                count: unknown_participants,
            });
        }
        Ok(())
    }

    /// Rules 1–4 on one drive's contiguous rows.
    fn validate_drive(drive: &DriveKey, rows: &[PlayRecord]) -> Result<(), ContractError> {
        let first = &rows[0];
        if first.off_team == first.def_team {
            return Err(ContractError::InvalidValue {
                field: "def_team".to_owned(),
                detail: format!(
                    "off_team and def_team are both `{}` on {drive} (rule 4)",
                    first.off_team
                ),
            });
        }
        let terminals = rows
            .iter()
            .filter(|p| matches!(p.next, NextState::Terminal))
            .count();
        if terminals > 1 {
            return Err(ContractError::MultipleTerminals {
                drive: drive.clone(),
            });
        }
        let last = rows.last().expect("a drive group holds at least one row");
        if !matches!(last.next, NextState::Terminal) {
            return Err(ContractError::NonTerminalLastRow {
                drive: drive.clone(),
            });
        }
        for (i, play) in rows.iter().enumerate() {
            if play.drive_points != first.drive_points {
                return Err(ContractError::DrivePointsNotConstant {
                    drive: drive.clone(),
                });
            }
            if play.off_team != first.off_team || play.def_team != first.def_team {
                return Err(ContractError::PossessionChangeWithinDrive {
                    drive: drive.clone(),
                });
            }
            if i + 1 < rows.len() {
                match play.next {
                    NextState::Continue(next_state) if next_state == rows[i + 1].state => {}
                    NextState::Continue(next_state) => {
                        return Err(ContractError::InvalidValue {
                            field: "next".to_owned(),
                            detail: format!(
                                "play {} declares next state {next_state:?} but the following \
                                 row of {drive} has state {:?} (rule 1)",
                                play.play_key(),
                                rows[i + 1].state
                            ),
                        });
                    }
                    // Unreachable once `terminals <= 1` and the last row is terminal, kept as
                    // a typed outcome rather than a silent pass.
                    NextState::Terminal => {
                        return Err(ContractError::MultipleTerminals {
                            drive: drive.clone(),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    /// Rule 5 and real-data rule 1 on one play's state.
    fn validate_state(
        key: &PlayKey,
        state: &PlayState,
        profile: FrameProfile,
    ) -> Result<(), ContractError> {
        let invalid = |field: &str, detail: String| ContractError::InvalidValue {
            field: field.to_owned(),
            detail: format!("{detail} on play {key}"),
        };
        if !(1..=4).contains(&state.down) {
            return Err(invalid(
                "down",
                format!("down {} is outside 1..=4 (rule 5)", state.down),
            ));
        }
        if state.ydstogo < 1 {
            return Err(invalid(
                "ydstogo",
                "ydstogo 0 is below 1 (rule 5)".to_owned(),
            ));
        }
        if state.yardline_100 < 1 {
            return Err(invalid(
                "yardline_100",
                "yardline_100 0 is below 1 (rule 5)".to_owned(),
            ));
        }
        if profile == FrameProfile::RealData {
            if state.yardline_100 > 99 {
                return Err(invalid(
                    "yardline_100",
                    format!(
                        "yardline_100 {} exceeds 99 under the real-data profile (real-data rule 1)",
                        state.yardline_100
                    ),
                ));
            }
            if state.ydstogo > state.yardline_100 {
                return Err(invalid(
                    "ydstogo",
                    format!(
                        "ydstogo {} exceeds yardline_100 {} under the real-data profile \
                         (real-data rule 1)",
                        state.ydstogo, state.yardline_100
                    ),
                ));
            }
        }
        Ok(())
    }

    /// The drives of the frame in file order, as `(drive, row range)` pairs. Rows of a drive are
    /// contiguous in a valid frame; on an invalid one a key may repeat.
    pub fn drives(&self) -> Vec<(DriveKey, std::ops::Range<usize>)> {
        let mut out = Vec::new();
        let mut start = 0usize;
        while start < self.plays.len() {
            let drive = self.plays[start].drive_key();
            let mut end = start;
            while end < self.plays.len() && self.plays[end].drive_key() == drive {
                end += 1;
            }
            out.push((drive, start..end));
            start = end;
        }
        out
    }

    /// Rows per team as `off_team`, a convenience for coverage and diagnostics.
    pub fn plays_per_offense(&self) -> HashMap<&TeamKey, usize> {
        let mut counts = HashMap::new();
        for play in &self.plays {
            *counts.entry(&play.off_team).or_insert(0) += 1;
        }
        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn week(w: u8) -> Week {
        Week::new(w).expect("non-zero week")
    }

    fn meta(profile: FrameProfile) -> PlaysFrameMeta {
        PlaysFrameMeta {
            contract: GRID_PLAYS_CONTRACT_VERSION,
            producer: FrameProducer::Synthetic {
                generator_version: "test".into(),
                config_hash: "0".into(),
                seeds: vec![("world".into(), 1)],
                profile,
            },
            sources: vec![],
            participation_availability: vec![(Season(2023), ParticipationStatus::Synthetic)],
            coverage: vec![ParticipationCoverage {
                season: Season(2023),
                offense_share: 1.0,
                defense_share: 1.0,
            }],
        }
    }

    fn players() -> PlayersFrame {
        PlayersFrame {
            players: vec![
                PlayerRecord {
                    player_id: PlayerKey::Synthetic(0),
                    team: TeamKey::Synthetic(0),
                    position: ParticipantPosition::Qb,
                },
                PlayerRecord {
                    player_id: PlayerKey::Synthetic(1),
                    team: TeamKey::Synthetic(0),
                    position: ParticipantPosition::Wr,
                },
                PlayerRecord {
                    player_id: PlayerKey::Synthetic(2),
                    team: TeamKey::Synthetic(1),
                    position: ParticipantPosition::Def,
                },
            ],
        }
    }

    fn state(down: u8, ydstogo: u8, yardline_100: u8) -> PlayState {
        PlayState {
            down,
            ydstogo,
            yardline_100,
        }
    }

    fn play(play_id: u32, drive_seq: u16, s: PlayState, next: NextState) -> PlayRecord {
        PlayRecord {
            game_id: GameId("synth-2023-01-0-1".into()),
            play_id,
            drive_seq,
            season: Season(2023),
            season_type: SeasonType::Regular,
            week: week(1),
            off_team: TeamKey::Synthetic(0),
            def_team: TeamKey::Synthetic(1),
            state: s,
            clock: None,
            yards: 5,
            next,
            drive_points: DrivePoints::FieldGoal,
            participation: Participation::Listed {
                offense: vec![PlayerKey::Synthetic(0), PlayerKey::Synthetic(1)],
                defense: vec![PlayerKey::Synthetic(2)],
            },
        }
    }

    /// Two drives, two and three rows, every rule satisfied.
    fn valid_frame(profile: FrameProfile) -> PlaysFrame {
        let s1 = state(1, 10, 75);
        let s2 = state(2, 5, 70);
        let t1 = state(1, 10, 60);
        let t2 = state(2, 7, 57);
        let t3 = state(3, 2, 52);
        PlaysFrame {
            meta: meta(profile),
            plays: vec![
                play(0, 1, s1, NextState::Continue(s2)),
                play(1, 1, s2, NextState::Terminal),
                play(2, 2, t1, NextState::Continue(t2)),
                play(3, 2, t2, NextState::Continue(t3)),
                play(4, 2, t3, NextState::Terminal),
            ],
        }
    }

    #[test]
    fn a_valid_frame_passes_under_both_profiles() {
        assert_eq!(
            valid_frame(FrameProfile::SyntheticLegacy).validate(&players()),
            Ok(())
        );
        assert_eq!(
            valid_frame(FrameProfile::RealData).validate(&players()),
            Ok(())
        );
        assert_eq!(valid_frame(FrameProfile::RealData).drives().len(), 2);
    }

    #[test]
    fn unsupported_major_is_rejected() {
        let mut frame = valid_frame(FrameProfile::RealData);
        frame.meta.contract = ContractVersion::new(2, 0);
        assert!(matches!(
            frame.validate(&players()),
            Err(ContractError::UnsupportedVersion { found, supported })
                if found == ContractVersion::new(2, 0) && supported == GRID_PLAYS_CONTRACT_VERSION
        ));
        // A MINOR difference is tolerated.
        frame.meta.contract = ContractVersion::new(1, 7);
        assert_eq!(frame.validate(&players()), Ok(()));
    }

    #[test]
    fn rule_1_next_state_must_match_the_following_row() {
        let mut frame = valid_frame(FrameProfile::SyntheticLegacy);
        frame.plays[0].next = NextState::Continue(state(3, 1, 1));
        assert!(matches!(
            frame.validate(&players()),
            Err(ContractError::InvalidValue { field, .. }) if field == "next"
        ));
    }

    #[test]
    fn rule_1_drive_rows_must_be_contiguous() {
        let mut frame = valid_frame(FrameProfile::SyntheticLegacy);
        // Interleave: drive 2's first row between drive 1's rows breaks contiguity of both.
        let moved = frame.plays.remove(2);
        frame.plays.insert(1, moved);
        let err = frame.validate(&players()).expect_err("interleaved drives");
        assert!(
            matches!(err, ContractError::InvalidValue { ref field, .. } if field == "drive_seq")
                || matches!(err, ContractError::NonTerminalLastRow { .. }),
            "{err}"
        );
        // And an explicit split of one drive by another complete drive.
        let s1 = state(1, 10, 75);
        let s2 = state(2, 5, 70);
        let u1 = state(1, 10, 50);
        let frame = PlaysFrame {
            meta: meta(FrameProfile::SyntheticLegacy),
            plays: vec![
                play(0, 1, s1, NextState::Continue(s2)),
                play(9, 3, u1, NextState::Terminal),
                play(1, 1, s2, NextState::Terminal),
            ],
        };
        // Drive 1's first group ends with a Continue row (rule 2 trips first on the split
        // group), which is still a typed failure of the contiguity requirement.
        assert!(frame.validate(&players()).is_err());
        let frame = PlaysFrame {
            meta: meta(FrameProfile::SyntheticLegacy),
            plays: vec![
                play(0, 1, s1, NextState::Terminal),
                play(9, 3, u1, NextState::Terminal),
                play(1, 1, s2, NextState::Terminal),
            ],
        };
        assert!(matches!(
            frame.validate(&players()),
            Err(ContractError::InvalidValue { field, .. }) if field == "drive_seq"
        ));
    }

    #[test]
    fn rule_2_exactly_one_terminal_and_last() {
        let mut frame = valid_frame(FrameProfile::SyntheticLegacy);
        frame.plays[4].next = NextState::Continue(state(4, 1, 50));
        assert!(matches!(
            frame.validate(&players()),
            Err(ContractError::NonTerminalLastRow { drive }) if drive.drive_seq == 2
        ));
        let mut frame = valid_frame(FrameProfile::SyntheticLegacy);
        frame.plays[2].next = NextState::Terminal;
        assert!(matches!(
            frame.validate(&players()),
            Err(ContractError::MultipleTerminals { drive }) if drive.drive_seq == 2
        ));
    }

    #[test]
    fn rule_3_drive_points_constant() {
        let mut frame = valid_frame(FrameProfile::SyntheticLegacy);
        frame.plays[3].drive_points = DrivePoints::Touchdown;
        assert!(matches!(
            frame.validate(&players()),
            Err(ContractError::DrivePointsNotConstant { drive }) if drive.drive_seq == 2
        ));
        assert_eq!(DrivePoints::Zero.points(), 0);
        assert_eq!(DrivePoints::FieldGoal.points(), 3);
        assert_eq!(DrivePoints::Touchdown.points(), 7);
    }

    #[test]
    fn rule_4_teams_differ_and_are_constant() {
        let mut frame = valid_frame(FrameProfile::SyntheticLegacy);
        frame.plays[0].def_team = TeamKey::Synthetic(0);
        assert!(matches!(
            frame.validate(&players()),
            Err(ContractError::InvalidValue { field, .. }) if field == "def_team"
        ));
        let mut frame = valid_frame(FrameProfile::SyntheticLegacy);
        frame.plays[3].off_team = TeamKey::Synthetic(1);
        frame.plays[3].def_team = TeamKey::Synthetic(0);
        assert!(matches!(
            frame.validate(&players()),
            Err(ContractError::PossessionChangeWithinDrive { drive }) if drive.drive_seq == 2
        ));
    }

    #[test]
    fn rule_5_state_domain() {
        for (field, bad) in [
            ("down", state(0, 10, 75)),
            ("down", state(5, 10, 75)),
            ("ydstogo", state(1, 0, 75)),
            ("yardline_100", state(1, 10, 0)),
        ] {
            let mut frame = valid_frame(FrameProfile::SyntheticLegacy);
            frame.plays[1].state = bad;
            frame.plays[0].next = NextState::Continue(bad);
            assert!(
                matches!(
                    frame.validate(&players()),
                    Err(ContractError::InvalidValue { field: f, .. }) if f == field
                ),
                "{field}"
            );
        }
    }

    #[test]
    fn rule_6_unknown_team_and_participants() {
        let mut frame = valid_frame(FrameProfile::SyntheticLegacy);
        // Both rows of drive 1, so that rule 4 (constancy within a drive) still holds.
        frame.plays[0].def_team = TeamKey::Synthetic(9);
        frame.plays[1].def_team = TeamKey::Synthetic(9);
        assert!(matches!(
            frame.validate(&players()),
            Err(ContractError::UnknownTeam { team, play })
                if team == TeamKey::Synthetic(9) && play.play_id == 0
        ));
        let mut frame = valid_frame(FrameProfile::SyntheticLegacy);
        frame.plays[0].participation = Participation::Listed {
            offense: vec![PlayerKey::Synthetic(77), PlayerKey::Synthetic(0)],
            defense: vec![PlayerKey::Synthetic(88)],
        };
        frame.plays[4].participation = Participation::Listed {
            offense: vec![PlayerKey::Synthetic(77)],
            defense: vec![],
        };
        // Counted across the frame, never dropped (DR-B6).
        assert_eq!(
            frame.validate(&players()),
            Err(ContractError::UnknownParticipant { count: 3 })
        );
        // Unmatched and NotAvailable name no participant and pass rule 6.
        let mut frame = valid_frame(FrameProfile::SyntheticLegacy);
        frame.plays[0].participation = Participation::Unmatched;
        frame.plays[1].participation = Participation::NotAvailable;
        assert_eq!(frame.validate(&players()), Ok(()));
    }

    #[test]
    fn rule_7_keys_unique() {
        let mut frame = valid_frame(FrameProfile::SyntheticLegacy);
        frame.plays[3].play_id = 2;
        assert!(matches!(
            frame.validate(&players()),
            Err(ContractError::DuplicateKey { frame, key })
                if frame == "plays" && key == "synth-2023-01-0-1#2"
        ));
        let mut ps = players();
        ps.players.push(PlayerRecord {
            player_id: PlayerKey::Synthetic(1),
            team: TeamKey::Synthetic(1),
            position: ParticipantPosition::Te,
        });
        assert!(matches!(
            valid_frame(FrameProfile::SyntheticLegacy).validate(&ps),
            Err(ContractError::DuplicateKey { frame, key }) if frame == "players" && key == "1"
        ));
        // The same play_id in a different game is a different key.
        let mut frame = valid_frame(FrameProfile::SyntheticLegacy);
        for p in &mut frame.plays[2..] {
            p.game_id = GameId("synth-2023-01-2-3".into());
            p.play_id -= 2;
        }
        assert_eq!(frame.validate(&players()), Ok(()));
    }

    #[test]
    fn real_data_rule_1_yardline_and_ydstogo_bounds() {
        // yardline_100 = 104 (D-9): rejected under RealData, allowed under SyntheticLegacy.
        let deep = state(1, 10, 104);
        for (profile, ok) in [
            (FrameProfile::RealData, false),
            (FrameProfile::SyntheticLegacy, true),
        ] {
            let mut frame = valid_frame(profile);
            frame.plays[2].state = deep;
            let result = frame.validate(&players());
            if ok {
                assert_eq!(result, Ok(()), "{profile}");
            } else {
                assert!(
                    matches!(
                        result,
                        Err(ContractError::InvalidValue { ref field, .. }) if field == "yardline_100"
                    ),
                    "{profile}: {result:?}"
                );
            }
        }
        // ydstogo > yardline_100.
        let long = state(1, 12, 8);
        for (profile, ok) in [
            (FrameProfile::RealData, false),
            (FrameProfile::SyntheticLegacy, true),
        ] {
            let mut frame = valid_frame(profile);
            frame.plays[2].state = long;
            let result = frame.validate(&players());
            if ok {
                assert_eq!(result, Ok(()), "{profile}");
            } else {
                assert!(
                    matches!(
                        result,
                        Err(ContractError::InvalidValue { ref field, .. }) if field == "ydstogo"
                    ),
                    "{profile}: {result:?}"
                );
            }
        }
    }

    #[test]
    fn real_data_rule_2_no_duplicate_within_a_side() {
        let duplicated = Participation::Listed {
            offense: vec![
                PlayerKey::Synthetic(1),
                PlayerKey::Synthetic(0),
                PlayerKey::Synthetic(1),
            ],
            defense: vec![PlayerKey::Synthetic(2)],
        };
        let mut frame = valid_frame(FrameProfile::RealData);
        frame.plays[1].participation = duplicated.clone();
        assert!(matches!(
            frame.validate(&players()),
            Err(ContractError::DuplicateParticipant { play, player_id })
                if play.play_id == 1 && player_id == PlayerKey::Synthetic(1)
        ));
        // The legacy profile keeps the multiset (D-8).
        let mut frame = valid_frame(FrameProfile::SyntheticLegacy);
        frame.plays[1].participation = duplicated;
        assert_eq!(frame.validate(&players()), Ok(()));
        // An nflverse producer is always held to the real-data profile.
        let mut frame = valid_frame(FrameProfile::SyntheticLegacy);
        frame.meta.producer = FrameProducer::Nflverse {
            adapter_version: "test".into(),
            normalization_map_version: "1".into(),
        };
        assert_eq!(frame.meta.producer.profile(), FrameProfile::RealData);
        frame.plays[2].state = state(1, 10, 104);
        assert!(frame.validate(&players()).is_err());
    }

    #[test]
    fn participant_position_round_trips_through_its_raw_string() {
        assert_eq!(ParticipantPosition::from("QB"), ParticipantPosition::Qb);
        assert_eq!(ParticipantPosition::from("DEF"), ParticipantPosition::Def);
        assert_eq!(
            ParticipantPosition::from("OLB"),
            ParticipantPosition::Other("OLB".into())
        );
        assert_eq!(String::from(ParticipantPosition::Cb), "CB");
        assert_eq!(String::from(ParticipantPosition::Other("S".into())), "S");
        assert_eq!(ParticipantPosition::Te.to_string(), "TE");
    }

    #[test]
    fn a_hand_built_other_with_a_known_label_reads_back_as_the_known_variant() {
        // The serde form goes through `Into<String>` then `From<String>`, so this is the round
        // trip a writer and a reader perform.
        let hand_built = ParticipantPosition::Other("QB".into());
        let text = String::from(hand_built.clone());
        assert_eq!(text, String::from(ParticipantPosition::Qb));
        let read_back = ParticipantPosition::from(text);
        assert_eq!(read_back, ParticipantPosition::Qb);
        assert_ne!(read_back, hand_built);
        // A value a reader produced survives the round trip unchanged.
        for label in ["QB", "RB", "WR", "TE", "FB", "DEF", "CB", "OLB"] {
            let read = ParticipantPosition::from(label);
            assert_eq!(ParticipantPosition::from(String::from(read.clone())), read);
        }
    }

    #[test]
    fn frame_profile_display_matches_its_serialized_form() {
        assert_eq!(
            FrameProfile::SyntheticLegacy.to_string(),
            "synthetic-legacy"
        );
        assert_eq!(FrameProfile::RealData.to_string(), "real-data");
    }

    #[test]
    fn plays_per_offense_counts_rows() {
        let frame = valid_frame(FrameProfile::RealData);
        let counts = frame.plays_per_offense();
        assert_eq!(counts.get(&TeamKey::Synthetic(0)), Some(&5));
    }
}
