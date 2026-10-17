//! The legacy v0 synthetic world: the oracle's columnar frames, their upgrade to the v1 contract
//! types and the exact columnar view back (plays-contract §2.1, §4, §6, §6.1, §6.2, §11;
//! synthetic-world.md §5.3 (a), §10.1, §10.3 S-1; parity-fixture-contract §5 first row).
//!
//! The v0 columns `terminal`, `terminal_value`, `points` and `n_*` are **derived** in v1 from
//! `next` and `drive_points`. [`from_legacy_v0`] checks that a v0 world is consistent with that
//! derivation and rejects it with a typed [`ContractError`] otherwise; nothing is coerced
//! (engine-spec Appendix D "converting malformed data to a healthy default"). [`to_v0_columns`]
//! rebuilds the v0 columns, NaN and −1 sentinels included, so that a loaded world round-trips
//! exactly (S-1: every column, id order, NaN position and sentinel).
//!
//! Known oracle defects of this world are loaded **as they are** and labelled `synthetic-legacy`:
//! duplicated participant ids (a multiset, D-8, KI-NEW-Z38), `yardline_100` up to 104 (D-9,
//! KI-NEW-Z39) and defenders drawn from the offense (D-10, KI-NEW-Y0). The legacy profile is
//! exempt from the real-data rules, so [`grid_domain::PlaysFrame::validate`] accepts them; the
//! engine's own generator (P1-12) does not reproduce them.
//!
//! What the frames do not carry, the fixture manifest does: the seeds (`seeds`) and the content
//! hash of the generator configuration (`provenance.synth_config_sha256`, the sha256 of the
//! canonical JSON serialization of `params.synth_config`; parity-fixture-contract §4, §5.1),
//! which `FrameProducer::Synthetic` requires (plays-contract §9). [`load_world`] carries the hash
//! as the manifest states it, never recomputed (DR-D29), and refuses a malformed one;
//! [`from_legacy_v0`] refuses a world that states none.

use std::collections::BTreeMap;

use grid_domain::{
    CollegeFrame, CollegeRecord, ContractError, DrivePoints, FrameProducer, FrameProfile, GameId,
    NextState, ParticipantPosition, Participation, ParticipationCoverage, ParticipationStatus,
    PlayRecord, PlayState, PlayerKey, PlayerRecord, PlayersFrame, PlaysFrame, PlaysFrameMeta,
    Season, SeasonType, TeamKey, Week, GRID_PLAYS_CONTRACT_VERSION,
};

use crate::fixture::{
    compare_bitwise, compare_exact, FixtureCase, FixtureError, Mismatch, OracleLevel, Ragged,
};
use crate::truth::SyntheticTruth;

/// The `generator_version` of every world the legacy oracle produced
/// (`FrameProducer::Synthetic`; plays-contract §9; oracle `59bce1d`, ADR-012).
pub const LEGACY_GENERATOR_VERSION: &str = "legacy-59bce1d";

/// The v0 sentinel of `n_down`, `n_ydstogo` and `n_yardline_100` on a terminal row
/// (plays-contract §2.1; parity-fixture-contract §2.1 keeps it verbatim).
pub const V0_NEXT_SENTINEL: i64 = -1;

/// The v0 `plays` frame as columns (plays-contract §2.1, "Synth" dtypes). Every column has one
/// entry per play, in file order; the two ragged columns have one row per play.
///
/// Floats are `f64` as the oracle wrote them: `terminal_value` is NaN on every non-terminal row;
/// `yards`, `points` and `drive_points` are integral. A zero gain is written as `0.0`, never
/// `-0.0`: the generator's `float(np.clip(round(yards), -8, 60))` (`synth.py:215`) rounds through
/// a Python `int`, because `round` of a NumPy 2.x float scalar with no `ndigits` returns `int`
/// (NumPy's `_RoundMixin.__round__`; the oracle pins `numpy==2.4.6`). A world that carries a
/// `-0.0` is therefore not an oracle world and [`from_legacy_v0`] rejects it (see there). The
/// derived `PartialEq` treats NaN as unequal; use [`LegacyV0World::first_difference`] for the
/// S-1 column comparison.
#[derive(Debug, Clone, PartialEq)]
pub struct LegacyV0Plays {
    /// `play_id`, int64: `0..N−1`, globally unique, generation order.
    pub play_id: Vec<i64>,
    /// `drive_id`, int64: `1..=2016` on the canonical world, globally unique.
    pub drive_id: Vec<i64>,
    /// `week`, int64: `1..=14` on the canonical world; never 0 (KI-A4).
    pub week: Vec<i64>,
    /// `off_team`, int64: `0..=11` on the canonical world.
    pub off_team: Vec<i64>,
    /// `def_team`, int64.
    pub def_team: Vec<i64>,
    /// `down`, int64: `1..=4`.
    pub down: Vec<i64>,
    /// `ydstogo`, int64: `≥ 1`.
    pub ydstogo: Vec<i64>,
    /// `yardline_100`, int64: `1..=104` on the legacy world (D-9).
    pub yardline_100: Vec<i64>,
    /// `yards`, float64 integral: yards gained, clipped to `[−8, 60]` by the generator.
    pub yards: Vec<f64>,
    /// `points`, float64: `drive_points` on the terminal row, `0.0` elsewhere.
    pub points: Vec<f64>,
    /// `terminal`, bool: true on the last modelled row of the drive and only there.
    pub terminal: Vec<bool>,
    /// `terminal_value`, float64: `{0, 3, 7}` on terminal rows, **NaN on every other row**.
    pub terminal_value: Vec<f64>,
    /// `n_down`, int64: the next row's `down`, or `−1` on a terminal row.
    pub n_down: Vec<i64>,
    /// `n_ydstogo`, int64: the next row's `ydstogo`, or `−1`.
    pub n_ydstogo: Vec<i64>,
    /// `n_yardline_100`, int64: the next row's `yardline_100`, or `−1`.
    pub n_yardline_100: Vec<i64>,
    /// `drive_points`, float64: `{0, 3, 7}`, constant within a drive.
    pub drive_points: Vec<f64>,
    /// `off_players`: the on-field offense ids, order and duplicates kept (D-8).
    pub off_players: Ragged<i64>,
    /// `def_players`: the on-field defender ids (7 per row on the canonical world).
    pub def_players: Ragged<i64>,
}

/// The v0 `players` frame as columns (plays-contract §4, "Synth" dtypes), truth columns included.
#[derive(Debug, Clone, PartialEq)]
pub struct LegacyV0Players {
    /// `player_id`, int64: `0..=287` on the canonical world, unique.
    pub player_id: Vec<i64>,
    /// `team`, int64.
    pub team: Vec<i64>,
    /// `position`, str: `QB`, `RB`, `WR`, `TE`, `DEF` (`CB` under `cb_split`).
    pub position: Vec<String>,
    /// `is_starter`, bool: synthetic bookkeeping; truth in v1.
    pub is_starter: Vec<bool>,
    /// `ability`, float64: the planted truth; never reaches an estimator in v1 (D-13).
    pub ability: Vec<f64>,
}

/// The v0 truth bundle `gt` as columns (plays-contract §6.1).
#[derive(Debug, Clone, PartialEq)]
pub struct LegacyV0Truth {
    /// The team ids that index `team_strength` (`gt.team_strength.team`).
    pub team_strength_team: Vec<i64>,
    /// The legacy planted team strength per team (`gt.team_strength`), void after ledger entry 1.
    pub team_strength: Vec<f64>,
    /// `gt.focus_qb`, int64.
    pub focus_qb: i64,
    /// `gt.focus_tau`, float64 per week, NaN = missed game.
    pub focus_tau: Vec<f64>,
    /// `gt.league_factor`, float64 (0.62 on the canonical world).
    pub league_factor: f64,
}

/// The v0 `college` frame as columns (plays-contract §6).
#[derive(Debug, Clone, PartialEq)]
pub struct LegacyV0College {
    /// `player_id`, int64.
    pub player_id: Vec<i64>,
    /// `position`, str.
    pub position: Vec<String>,
    /// `feeder_sv`, float64.
    pub feeder_sv: Vec<f64>,
    /// `feeder_snaps`, int64 in `[250, 900)` on the canonical world; never null in v0.
    pub feeder_snaps: Vec<i64>,
    /// `is_rookie`, bool.
    pub is_rookie: Vec<bool>,
}

/// What a v0 world knows about how it was generated, beyond its frames: the fields
/// `FrameProducer::Synthetic` requires that the frames themselves do not carry.
///
/// The oracle's `gt` bundle records neither a configuration hash nor its seeds
/// (plays-contract §6.1). The fixture manifest records both: the seeds under `seeds` and the
/// configuration hash under `provenance.synth_config_sha256` (parity-fixture-contract §5.1). The
/// configuration **name** (`provenance.synth_config`, `CANONICAL_SYNTH`) is a label, distinct
/// from the hash, and stays where it lives: `crate::fixture::Provenance::synth_config`.
/// [`load_world`] fills this block from the manifest; a hand-built world states it directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyV0Provenance {
    /// The content hash of the generator configuration, what [`from_legacy_v0`] writes into
    /// `FrameProducer::Synthetic::config_hash` (plays-contract §9): the manifest key
    /// `provenance.synth_config_sha256` of parity-fixture-contract §5.1, the sha256 of the
    /// canonical JSON serialization of `params.synth_config`, as exactly 64 lowercase
    /// hexadecimal characters ([`SYNTH_CONFIG_SHA256_KEY`]). [`load_world`] carries the value as
    /// the manifest states it and never recomputes it (DR-D29 puts hash verification in the
    /// shell guard); it refuses a value of another form. `None` when the world states no hash: a
    /// hand-built world, or one loaded from a case whose manifest omits the key.
    /// [`from_legacy_v0`] refuses such a world with a typed [`ContractError::MissingField`]
    /// naming the key rather than write a non-hash into the producer's hash field (engine-spec
    /// Appendix D).
    pub config_hash: Option<String>,
    /// Every named seed of the world (`seeds` of the manifest), sorted by name.
    pub seeds: Vec<(String, u64)>,
}

/// A complete v0 synthetic world: the `data_adapters.py` contract
/// `{plays, players, gt, college}` as columns, plus its provenance.
#[derive(Debug, Clone, PartialEq)]
pub struct LegacyV0World {
    /// The plays frame.
    pub plays: LegacyV0Plays,
    /// The players frame, truth columns included.
    pub players: LegacyV0Players,
    /// The truth bundle.
    pub truth: LegacyV0Truth,
    /// The college frame.
    pub college: LegacyV0College,
    /// Configuration hash (when the world states one) and seeds.
    pub provenance: LegacyV0Provenance,
}

impl LegacyV0World {
    /// The first column, if any, on which two worlds differ, as `(column, mismatch)`: the S-1
    /// round-trip criterion as synthetic-world.md §10.3 states it, "every column, id order, NaN
    /// position and v0 sentinel round-trips", under parity-fixture-contract §6 ("integer,
    /// boolean, id and categorical outputs are compared exactly; NaN positions must coincide; v0
    /// sentinels must match exactly").
    ///
    /// Integer, boolean, string and provenance columns are compared exactly ([`compare_exact`]).
    /// **Every** float column is compared **bit for bit** ([`compare_bitwise`]): NaN position and
    /// payload and the sign of zero must survive, both on the columns the upgrade carries
    /// untouched (`ability`, `gt.*`, `feeder_sv`) and on the four it carries in an integer type
    /// and derives back (`yards`, `points`, `terminal_value`, `drive_points`). The derived
    /// columns can promise this because their oracle values are integral and never `-0.0`
    /// ([`LegacyV0Plays`]), and [`from_legacy_v0`] refuses the one value an integer carrier would
    /// alter (a `-0.0`) instead of coercing it.
    pub fn first_difference(&self, other: &Self) -> Option<(String, Mismatch)> {
        macro_rules! exact {
            ($name:literal, $($field:tt).+) => {
                if let Err(m) = compare_exact(&self.$($field).+, &other.$($field).+) {
                    return Some(($name.to_owned(), m));
                }
            };
        }
        macro_rules! bits {
            ($name:literal, $($field:tt).+) => {
                if let Err(m) = compare_bitwise(&self.$($field).+, &other.$($field).+) {
                    return Some(($name.to_owned(), m));
                }
            };
        }
        exact!("plays.play_id", plays.play_id);
        exact!("plays.drive_id", plays.drive_id);
        exact!("plays.week", plays.week);
        exact!("plays.off_team", plays.off_team);
        exact!("plays.def_team", plays.def_team);
        exact!("plays.down", plays.down);
        exact!("plays.ydstogo", plays.ydstogo);
        exact!("plays.yardline_100", plays.yardline_100);
        bits!("plays.yards", plays.yards);
        bits!("plays.points", plays.points);
        exact!("plays.terminal", plays.terminal);
        bits!("plays.terminal_value", plays.terminal_value);
        exact!("plays.n_down", plays.n_down);
        exact!("plays.n_ydstogo", plays.n_ydstogo);
        exact!("plays.n_yardline_100", plays.n_yardline_100);
        bits!("plays.drive_points", plays.drive_points);
        exact!("plays.off_players.offsets", plays.off_players.offsets);
        exact!("plays.off_players.values", plays.off_players.values);
        exact!("plays.def_players.offsets", plays.def_players.offsets);
        exact!("plays.def_players.values", plays.def_players.values);
        exact!("players.player_id", players.player_id);
        exact!("players.team", players.team);
        exact!("players.position", players.position);
        exact!("players.is_starter", players.is_starter);
        bits!("players.ability", players.ability);
        exact!("gt.team_strength.team", truth.team_strength_team);
        bits!("gt.team_strength", truth.team_strength);
        if let Err(m) = compare_exact(&[self.truth.focus_qb], &[other.truth.focus_qb]) {
            return Some(("gt.focus_qb".to_owned(), m));
        }
        bits!("gt.focus_tau", truth.focus_tau);
        if let Err(m) = compare_bitwise(&[self.truth.league_factor], &[other.truth.league_factor]) {
            return Some(("gt.league_factor".to_owned(), m));
        }
        exact!("college.player_id", college.player_id);
        exact!("college.position", college.position);
        bits!("college.feeder_sv", college.feeder_sv);
        exact!("college.feeder_snaps", college.feeder_snaps);
        exact!("college.is_rookie", college.is_rookie);
        if let Err(m) = compare_exact(
            std::slice::from_ref(&self.provenance.config_hash),
            std::slice::from_ref(&other.provenance.config_hash),
        ) {
            // `None` versus `Some` is a difference: a world that states no hash is not the same
            // world as one that does.
            return Some(("provenance.config_hash".to_owned(), m));
        }
        exact!("provenance.seeds", provenance.seeds);
        None
    }
}

/// A v0 world upgraded to the v1 contract types: the three input frames and the truth bundle,
/// separated (plays-contract D-13).
#[derive(Debug, Clone, PartialEq)]
pub struct UpgradedWorld {
    /// The v1 plays frame, validated under the `synthetic-legacy` profile.
    pub plays: PlaysFrame,
    /// The v1 players frame (no truth columns).
    pub players: PlayersFrame,
    /// The v1 college frame.
    pub college: CollegeFrame,
    /// The planted truth.
    pub truth: SyntheticTruth,
}

fn invalid(field: &str, detail: String) -> ContractError {
    ContractError::InvalidValue {
        field: field.to_owned(),
        detail,
    }
}

fn check_len(field: &str, len: usize, expected: usize) -> Result<(), ContractError> {
    if len == expected {
        Ok(())
    } else {
        Err(invalid(
            field,
            format!("column has {len} entries, the frame has {expected} rows"),
        ))
    }
}

fn to_u8(field: &str, value: i64, row: usize) -> Result<u8, ContractError> {
    u8::try_from(value).map_err(|_| invalid(field, format!("{value} at row {row} does not fit u8")))
}

fn to_u16(field: &str, value: i64, row: usize) -> Result<u16, ContractError> {
    u16::try_from(value)
        .map_err(|_| invalid(field, format!("{value} at row {row} does not fit u16")))
}

fn to_u32(field: &str, value: i64, row: usize) -> Result<u32, ContractError> {
    u32::try_from(value)
        .map_err(|_| invalid(field, format!("{value} at row {row} does not fit u32")))
}

fn player_key(field: &str, id: i64, row: usize) -> Result<PlayerKey, ContractError> {
    to_u32(field, id, row).map(PlayerKey::Synthetic)
}

fn team_key(field: &str, id: i64, row: usize) -> Result<TeamKey, ContractError> {
    to_u16(field, id, row).map(TeamKey::Synthetic)
}

fn synthetic_player(field: &str, key: &PlayerKey) -> Result<i64, ContractError> {
    match key {
        PlayerKey::Synthetic(n) => Ok(i64::from(*n)),
        PlayerKey::Gsis(id) => Err(invalid(
            field,
            format!("GSIS id `{id}` has no legacy v0 synthetic encoding"),
        )),
    }
}

fn synthetic_team(field: &str, key: &TeamKey) -> Result<i64, ContractError> {
    match key {
        TeamKey::Synthetic(n) => Ok(i64::from(*n)),
        TeamKey::Nfl(abbr) => Err(invalid(
            field,
            format!("team abbreviation `{abbr}` has no legacy v0 synthetic encoding"),
        )),
    }
}

fn drive_points_of(field: &str, value: f64, row: usize) -> Result<DrivePoints, ContractError> {
    if value == 0.0 {
        Ok(DrivePoints::Zero)
    } else if value == 3.0 {
        Ok(DrivePoints::FieldGoal)
    } else if value == 7.0 {
        Ok(DrivePoints::Touchdown)
    } else {
        Err(invalid(
            field,
            format!("{value:?} at row {row} is not one of 0, 3, 7 (DR-C13)"),
        ))
    }
}

/// The synthetic game id fixed by P1-01: `synth-{season}-{week:02}-{a}-{b}` with `a ≤ b` the two
/// team numbers (the plays contract defines no synthetic game id; `grid_domain::GameId`).
pub fn synthetic_game_id(season: Season, week: Week, off_team: u16, def_team: u16) -> GameId {
    let (a, b) = if off_team <= def_team {
        (off_team, def_team)
    } else {
        (def_team, off_team)
    };
    GameId(format!("synth-{season}-{:02}-{a}-{b}", week.get()))
}

/// Upgrades a v0 world to the v1 frames (plays-contract §11 "`from_legacy_v0`").
///
/// Sets `season` (the manifest declares it: the v0 frames carry none), `season_type = Regular`,
/// `clock = None` (synthetic frames have no clock, D-1), participation `Listed` keeping order and
/// duplicates (D-8), `producer = Synthetic { generator_version: "legacy-59bce1d", config_hash,
/// seeds, profile: SyntheticLegacy }` from the world's provenance,
/// `participation_availability = [(season, Synthetic)]`, `sources = []` (no ingestion run
/// produced the world and no publication timestamp exists to record; KI-P1 forbids inventing
/// one) and one `coverage` entry for the season (the share of rows with a non-empty side; omitted
/// on a world with no play). `game_id` is [`synthetic_game_id`]; `drive_seq` is the v0 global
/// `drive_id`, which is unique and `≤ 2016` (plays-contract §2.1), so a drive key is unique
/// even though v1 scopes `drive_seq` to a game; `play_id` is the v0 `play_id`.
///
/// Rejects, with a typed [`ContractError`] and nothing coerced: a world whose
/// [`LegacyV0Provenance::config_hash`] is `None` ([`ContractError::MissingField`] naming the
/// manifest key `provenance.synth_config_sha256`, [`SYNTH_CONFIG_SHA256_KEY`]: the producer's
/// `config_hash` is a content hash and this function never writes anything else into it); a
/// column of the wrong length; a participation column whose offsets break the ragged invariants
/// ([`Ragged::validate`]; a world built from the public fields is never trusted to index); an
/// id or state value outside its v1 type; a `week` of 0 (KI-A4); `yards` that is not integral,
/// or that is `-0.0` (v1 carries `yards` as `i16`, which has no signed zero, so the v0 view
/// could not reproduce it exactly as plays-contract §11 requires, and the oracle never writes
/// one: [`LegacyV0Plays`]); a `drive_points` outside `{0, 3, 7}`; a terminal row whose `n_*`
/// are not all `−1` or whose `terminal_value` or `points` differ from `drive_points`; a
/// non-terminal row whose `terminal_value` is not NaN, whose `points` is not `0.0` or whose
/// `n_*` do not fit a state; a duplicated `player_id` or `team` in the truth columns; a negative
/// `feeder_snaps`. The upgraded plays frame is then validated under the `synthetic-legacy`
/// profile ([`PlaysFrame::validate`]), which accepts duplicated participants and `yardline_100`
/// up to 104 but applies every core rule.
pub fn from_legacy_v0(
    world: &LegacyV0World,
    season: Season,
) -> Result<UpgradedWorld, ContractError> {
    let config_hash =
        world
            .provenance
            .config_hash
            .clone()
            .ok_or_else(|| ContractError::MissingField {
                field: SYNTH_CONFIG_SHA256_KEY.to_owned(),
            })?;
    let (players, ability, is_starter) = upgrade_players(&world.players)?;
    let plays = upgrade_plays(&world.plays, season, config_hash, &world.provenance.seeds)?;
    let college = upgrade_college(&world.college)?;
    let truth = upgrade_truth(&world.truth, ability, is_starter)?;
    plays.validate(&players)?;
    Ok(UpgradedWorld {
        plays,
        players,
        college,
        truth,
    })
}

type PlayerMaps = (
    PlayersFrame,
    BTreeMap<PlayerKey, f64>,
    BTreeMap<PlayerKey, bool>,
);

fn upgrade_players(v0: &LegacyV0Players) -> Result<PlayerMaps, ContractError> {
    let n = v0.player_id.len();
    check_len("players.team", v0.team.len(), n)?;
    check_len("players.position", v0.position.len(), n)?;
    check_len("players.is_starter", v0.is_starter.len(), n)?;
    check_len("players.ability", v0.ability.len(), n)?;
    let mut players = Vec::with_capacity(n);
    let mut ability = BTreeMap::new();
    let mut is_starter = BTreeMap::new();
    for row in 0..n {
        let key = player_key("players.player_id", v0.player_id[row], row)?;
        if ability.contains_key(&key) {
            return Err(ContractError::DuplicateKey {
                frame: "players".to_owned(),
                key: key.to_string(),
            });
        }
        ability.insert(key.clone(), v0.ability[row]);
        is_starter.insert(key.clone(), v0.is_starter[row]);
        players.push(PlayerRecord {
            player_id: key,
            team: team_key("players.team", v0.team[row], row)?,
            position: ParticipantPosition::from(v0.position[row].as_str()),
        });
    }
    Ok((PlayersFrame { players }, ability, is_starter))
}

fn upgrade_plays(
    v0: &LegacyV0Plays,
    season: Season,
    config_hash: String,
    seeds: &[(String, u64)],
) -> Result<PlaysFrame, ContractError> {
    let n = v0.play_id.len();
    check_len("plays.drive_id", v0.drive_id.len(), n)?;
    check_len("plays.week", v0.week.len(), n)?;
    check_len("plays.off_team", v0.off_team.len(), n)?;
    check_len("plays.def_team", v0.def_team.len(), n)?;
    check_len("plays.down", v0.down.len(), n)?;
    check_len("plays.ydstogo", v0.ydstogo.len(), n)?;
    check_len("plays.yardline_100", v0.yardline_100.len(), n)?;
    check_len("plays.yards", v0.yards.len(), n)?;
    check_len("plays.points", v0.points.len(), n)?;
    check_len("plays.terminal", v0.terminal.len(), n)?;
    check_len("plays.terminal_value", v0.terminal_value.len(), n)?;
    check_len("plays.n_down", v0.n_down.len(), n)?;
    check_len("plays.n_ydstogo", v0.n_ydstogo.len(), n)?;
    check_len("plays.n_yardline_100", v0.n_yardline_100.len(), n)?;
    check_len("plays.drive_points", v0.drive_points.len(), n)?;
    // The ragged columns are validated before any row is indexed: `Ragged::row` panics on
    // offsets that were never checked, and a `LegacyV0Plays` can be built from public fields.
    for (field, ragged) in [
        ("plays.off_players", &v0.off_players),
        ("plays.def_players", &v0.def_players),
    ] {
        ragged
            .validate(field)
            .map_err(|e| invalid(&format!("{field}.offsets"), e.to_string()))?;
        check_len(field, ragged.len(), n)?;
    }

    let mut plays = Vec::with_capacity(n);
    let mut offense_listed = 0usize;
    let mut defense_listed = 0usize;
    for row in 0..n {
        let week = Week::new(to_u8("plays.week", v0.week[row], row)?)?;
        let off = to_u16("plays.off_team", v0.off_team[row], row)?;
        let def = to_u16("plays.def_team", v0.def_team[row], row)?;
        let state = PlayState {
            down: to_u8("plays.down", v0.down[row], row)?,
            ydstogo: to_u8("plays.ydstogo", v0.ydstogo[row], row)?,
            yardline_100: to_u8("plays.yardline_100", v0.yardline_100[row], row)?,
        };
        let yards_f = v0.yards[row];
        if !yards_f.is_finite() || yards_f.fract() != 0.0 {
            return Err(invalid(
                "plays.yards",
                format!("{yards_f:?} at row {row} is not an integral value"),
            ));
        }
        if yards_f.to_bits() == (-0.0f64).to_bits() {
            // `-0.0 as i16` is 0 and comes back as `0.0`: a bit the v0 view could not reproduce.
            // The oracle never writes it (`round` of a NumPy 2.x scalar is a Python `int`), so a
            // world carrying it is refused, not coerced (engine-spec Appendix D).
            return Err(invalid(
                "plays.yards",
                format!(
                    "-0.0 at row {row}: a negative zero has no i16 encoding and the oracle \
                     never writes one"
                ),
            ));
        }
        let yards = i16::try_from(yards_f as i64).map_err(|_| {
            invalid(
                "plays.yards",
                format!("{yards_f:?} at row {row} does not fit i16"),
            )
        })?;
        let drive_points = drive_points_of("plays.drive_points", v0.drive_points[row], row)?;
        let expected_points = f64::from(drive_points.points());
        let next = if v0.terminal[row] {
            for (field, value) in [
                ("plays.n_down", v0.n_down[row]),
                ("plays.n_ydstogo", v0.n_ydstogo[row]),
                ("plays.n_yardline_100", v0.n_yardline_100[row]),
            ] {
                if value != V0_NEXT_SENTINEL {
                    return Err(invalid(
                        field,
                        format!("terminal row {row} carries {value}, not the −1 sentinel"),
                    ));
                }
            }
            if v0.terminal_value[row] != expected_points {
                return Err(invalid(
                    "plays.terminal_value",
                    format!(
                        "terminal row {row} carries {:?}, not its drive_points {expected_points}",
                        v0.terminal_value[row]
                    ),
                ));
            }
            if v0.points[row] != expected_points {
                return Err(invalid(
                    "plays.points",
                    format!(
                        "terminal row {row} carries {:?}, not its drive_points {expected_points}",
                        v0.points[row]
                    ),
                ));
            }
            NextState::Terminal
        } else {
            if !v0.terminal_value[row].is_nan() {
                return Err(invalid(
                    "plays.terminal_value",
                    format!(
                        "non-terminal row {row} carries {:?}, not NaN",
                        v0.terminal_value[row]
                    ),
                ));
            }
            if v0.points[row] != 0.0 {
                return Err(invalid(
                    "plays.points",
                    format!(
                        "non-terminal row {row} carries {:?}, not 0.0",
                        v0.points[row]
                    ),
                ));
            }
            NextState::Continue(PlayState {
                down: to_u8("plays.n_down", v0.n_down[row], row)?,
                ydstogo: to_u8("plays.n_ydstogo", v0.n_ydstogo[row], row)?,
                yardline_100: to_u8("plays.n_yardline_100", v0.n_yardline_100[row], row)?,
            })
        };
        let offense = v0
            .off_players
            .row(row)
            .iter()
            .map(|&id| player_key("plays.off_players", id, row))
            .collect::<Result<Vec<_>, _>>()?;
        let defense = v0
            .def_players
            .row(row)
            .iter()
            .map(|&id| player_key("plays.def_players", id, row))
            .collect::<Result<Vec<_>, _>>()?;
        offense_listed += usize::from(!offense.is_empty());
        defense_listed += usize::from(!defense.is_empty());
        plays.push(PlayRecord {
            game_id: synthetic_game_id(season, week, off, def),
            play_id: to_u32("plays.play_id", v0.play_id[row], row)?,
            drive_seq: to_u16("plays.drive_id", v0.drive_id[row], row)?,
            season,
            season_type: SeasonType::Regular,
            week,
            off_team: TeamKey::Synthetic(off),
            def_team: TeamKey::Synthetic(def),
            state,
            clock: None,
            yards,
            next,
            drive_points,
            participation: Participation::Listed { offense, defense },
        });
    }

    let coverage = if n == 0 {
        Vec::new()
    } else {
        vec![ParticipationCoverage {
            season,
            offense_share: offense_listed as f64 / n as f64,
            defense_share: defense_listed as f64 / n as f64,
        }]
    };
    Ok(PlaysFrame {
        meta: PlaysFrameMeta {
            contract: GRID_PLAYS_CONTRACT_VERSION,
            producer: FrameProducer::Synthetic {
                generator_version: LEGACY_GENERATOR_VERSION.to_owned(),
                config_hash,
                seeds: seeds.to_vec(),
                profile: FrameProfile::SyntheticLegacy,
            },
            sources: Vec::new(),
            participation_availability: vec![(season, ParticipationStatus::Synthetic)],
            coverage,
        },
        plays,
    })
}

fn upgrade_college(v0: &LegacyV0College) -> Result<CollegeFrame, ContractError> {
    let n = v0.player_id.len();
    check_len("college.position", v0.position.len(), n)?;
    check_len("college.feeder_sv", v0.feeder_sv.len(), n)?;
    check_len("college.feeder_snaps", v0.feeder_snaps.len(), n)?;
    check_len("college.is_rookie", v0.is_rookie.len(), n)?;
    let mut rows = Vec::with_capacity(n);
    for row in 0..n {
        rows.push(CollegeRecord {
            player_id: player_key("college.player_id", v0.player_id[row], row)?,
            position: ParticipantPosition::from(v0.position[row].as_str()),
            feeder_sv: v0.feeder_sv[row],
            feeder_snaps: Some(to_u32("college.feeder_snaps", v0.feeder_snaps[row], row)?),
            is_rookie: v0.is_rookie[row],
        });
    }
    Ok(CollegeFrame { rows })
}

fn upgrade_truth(
    v0: &LegacyV0Truth,
    ability: BTreeMap<PlayerKey, f64>,
    is_starter: BTreeMap<PlayerKey, bool>,
) -> Result<SyntheticTruth, ContractError> {
    check_len(
        "gt.team_strength",
        v0.team_strength.len(),
        v0.team_strength_team.len(),
    )?;
    let mut team_strength_legacy = BTreeMap::new();
    for (row, (&team, &strength)) in v0
        .team_strength_team
        .iter()
        .zip(&v0.team_strength)
        .enumerate()
    {
        let key = team_key("gt.team_strength.team", team, row)?;
        if team_strength_legacy.insert(key.clone(), strength).is_some() {
            return Err(ContractError::DuplicateKey {
                frame: "gt.team_strength".to_owned(),
                key: key.to_string(),
            });
        }
    }
    Ok(SyntheticTruth {
        ability,
        is_starter,
        team_strength_legacy,
        focus_qb: player_key("gt.focus_qb", v0.focus_qb, 0)?,
        focus_tau: v0
            .focus_tau
            .iter()
            .map(|t| if t.is_nan() { None } else { Some(*t) })
            .collect(),
        league_factor: v0.league_factor,
        net_strength: None,
    })
}

/// Rebuilds the v0 columns of an upgraded world: the exact inverse of [`from_legacy_v0`] on its
/// image (plays-contract §11 "a columnar v0 view … reproduces them exactly, NaN and −1 sentinels
/// included").
///
/// `terminal`, `terminal_value` (NaN on a non-terminal row, written as `f64::NAN`, the quiet NaN
/// NumPy writes), `points` and the `n_*` sentinels are derived from `next` and `drive_points`;
/// `yards` is `f64::from(i16)`, which is bit-identical to the oracle's integral value because
/// [`from_legacy_v0`] admits no `-0.0`; `is_starter` and `ability` come back from the truth;
/// `gt.team_strength` is emitted in ascending team order; `focus_tau` writes `None` as NaN; the
/// provenance block carries the producer's `config_hash` as `Some` and its seeds. The v1-only
/// fields have no v0 column and are dropped: `season`, `season_type`, `clock`, `game_id`
/// (derived), `meta.sources`, `meta.coverage`, `meta.participation_availability` and
/// `truth.net_strength`. A `Participation::Unmatched` or `NotAvailable` row becomes the empty
/// tuple `()`, which is the v0 encoding of both (plays-contract §2.1, D-5); [`from_legacy_v0`]
/// never produces either.
///
/// Fails with a typed [`ContractError`] where v0 has no encoding: a `Gsis` player key or an `Nfl`
/// team key, an `Nflverse` producer, a player with no truth entry, or `feeder_snaps = None`
/// (v0 always carries it: [`ContractError::MissingField`]). It never writes a sentinel the
/// contract does not define.
pub fn to_v0_columns(world: &UpgradedWorld) -> Result<LegacyV0World, ContractError> {
    let frame = &world.plays;
    let n = frame.plays.len();
    let mut plays = LegacyV0Plays {
        play_id: Vec::with_capacity(n),
        drive_id: Vec::with_capacity(n),
        week: Vec::with_capacity(n),
        off_team: Vec::with_capacity(n),
        def_team: Vec::with_capacity(n),
        down: Vec::with_capacity(n),
        ydstogo: Vec::with_capacity(n),
        yardline_100: Vec::with_capacity(n),
        yards: Vec::with_capacity(n),
        points: Vec::with_capacity(n),
        terminal: Vec::with_capacity(n),
        terminal_value: Vec::with_capacity(n),
        n_down: Vec::with_capacity(n),
        n_ydstogo: Vec::with_capacity(n),
        n_yardline_100: Vec::with_capacity(n),
        drive_points: Vec::with_capacity(n),
        off_players: Ragged::from_rows(std::iter::empty()),
        def_players: Ragged::from_rows(std::iter::empty()),
    };
    let mut off_rows = Vec::with_capacity(n);
    let mut def_rows = Vec::with_capacity(n);
    for play in &frame.plays {
        plays.play_id.push(i64::from(play.play_id));
        plays.drive_id.push(i64::from(play.drive_seq));
        plays.week.push(i64::from(play.week.get()));
        plays
            .off_team
            .push(synthetic_team("plays.off_team", &play.off_team)?);
        plays
            .def_team
            .push(synthetic_team("plays.def_team", &play.def_team)?);
        plays.down.push(i64::from(play.state.down));
        plays.ydstogo.push(i64::from(play.state.ydstogo));
        plays.yardline_100.push(i64::from(play.state.yardline_100));
        plays.yards.push(f64::from(play.yards));
        let points = f64::from(play.drive_points.points());
        plays.drive_points.push(points);
        match play.next {
            NextState::Continue(next) => {
                plays.terminal.push(false);
                plays.terminal_value.push(f64::NAN);
                plays.points.push(0.0);
                plays.n_down.push(i64::from(next.down));
                plays.n_ydstogo.push(i64::from(next.ydstogo));
                plays.n_yardline_100.push(i64::from(next.yardline_100));
            }
            NextState::Terminal => {
                plays.terminal.push(true);
                plays.terminal_value.push(points);
                plays.points.push(points);
                plays.n_down.push(V0_NEXT_SENTINEL);
                plays.n_ydstogo.push(V0_NEXT_SENTINEL);
                plays.n_yardline_100.push(V0_NEXT_SENTINEL);
            }
        }
        let (offense, defense): (Vec<i64>, Vec<i64>) = match &play.participation {
            Participation::Listed { offense, defense } => (
                offense
                    .iter()
                    .map(|k| synthetic_player("plays.off_players", k))
                    .collect::<Result<_, _>>()?,
                defense
                    .iter()
                    .map(|k| synthetic_player("plays.def_players", k))
                    .collect::<Result<_, _>>()?,
            ),
            Participation::Unmatched | Participation::NotAvailable => (Vec::new(), Vec::new()),
        };
        off_rows.push(offense);
        def_rows.push(defense);
    }
    plays.off_players = Ragged::from_rows(off_rows);
    plays.def_players = Ragged::from_rows(def_rows);

    let truth = &world.truth;
    let mut players = LegacyV0Players {
        player_id: Vec::with_capacity(world.players.players.len()),
        team: Vec::with_capacity(world.players.players.len()),
        position: Vec::with_capacity(world.players.players.len()),
        is_starter: Vec::with_capacity(world.players.players.len()),
        ability: Vec::with_capacity(world.players.players.len()),
    };
    for record in &world.players.players {
        let key = &record.player_id;
        // The v0 encoding question comes first: a key v0 cannot encode is reported as such even
        // when the truth has no entry for it either.
        let player_id = synthetic_player("players.player_id", key)?;
        let team = synthetic_team("players.team", &record.team)?;
        let ability = *truth
            .ability
            .get(key)
            .ok_or_else(|| ContractError::MissingField {
                field: format!("truth.ability for player {key}"),
            })?;
        let is_starter = *truth
            .is_starter
            .get(key)
            .ok_or_else(|| ContractError::MissingField {
                field: format!("truth.is_starter for player {key}"),
            })?;
        players.player_id.push(player_id);
        players.team.push(team);
        players.position.push(record.position.as_str().to_owned());
        players.is_starter.push(is_starter);
        players.ability.push(ability);
    }

    let mut v0_truth = LegacyV0Truth {
        team_strength_team: Vec::with_capacity(truth.team_strength_legacy.len()),
        team_strength: Vec::with_capacity(truth.team_strength_legacy.len()),
        focus_qb: synthetic_player("gt.focus_qb", &truth.focus_qb)?,
        focus_tau: truth
            .focus_tau
            .iter()
            .map(|t| t.unwrap_or(f64::NAN))
            .collect(),
        league_factor: truth.league_factor,
    };
    for (team, strength) in &truth.team_strength_legacy {
        v0_truth
            .team_strength_team
            .push(synthetic_team("gt.team_strength.team", team)?);
        v0_truth.team_strength.push(*strength);
    }

    let mut college = LegacyV0College {
        player_id: Vec::with_capacity(world.college.rows.len()),
        position: Vec::with_capacity(world.college.rows.len()),
        feeder_sv: Vec::with_capacity(world.college.rows.len()),
        feeder_snaps: Vec::with_capacity(world.college.rows.len()),
        is_rookie: Vec::with_capacity(world.college.rows.len()),
    };
    for row in &world.college.rows {
        college
            .player_id
            .push(synthetic_player("college.player_id", &row.player_id)?);
        college.position.push(row.position.as_str().to_owned());
        college.feeder_sv.push(row.feeder_sv);
        college
            .feeder_snaps
            .push(i64::from(row.feeder_snaps.ok_or_else(|| {
                ContractError::MissingField {
                    field: format!("college.feeder_snaps for player {}", row.player_id),
                }
            })?));
        college.is_rookie.push(row.is_rookie);
    }

    let provenance = match &frame.meta.producer {
        FrameProducer::Synthetic {
            config_hash, seeds, ..
        } => LegacyV0Provenance {
            config_hash: Some(config_hash.clone()),
            seeds: seeds.clone(),
        },
        FrameProducer::Nflverse { .. } => {
            return Err(invalid(
                "meta.producer",
                "an nflverse frame has no legacy v0 synthetic encoding".to_owned(),
            ))
        }
    };

    Ok(LegacyV0World {
        plays,
        players,
        truth: v0_truth,
        college,
        provenance,
    })
}

/// The component a `synthetic-world` case declares (parity-fixture-contract §3).
pub const SYNTHETIC_WORLD_COMPONENT: &str = "synthetic-world";

/// The plays-contract version a legacy v0 case declares in `contracts.plays`
/// (parity-fixture-contract §4; plays-contract §12). [`load_world`] implements this MAJOR only.
pub const LEGACY_PLAYS_CONTRACT: &str = "grid.plays/0";

/// The manifest key that records the content hash of the generator configuration on a
/// `synthetic-world` case (parity-fixture-contract §5.1): the sha256, as 64 lowercase
/// hexadecimal characters, of the canonical JSON serialization of [`SYNTH_CONFIG_PARAMS_KEY`].
/// [`load_world`] carries it into [`LegacyV0Provenance::config_hash`]; [`from_legacy_v0`] names
/// it in the [`ContractError::MissingField`] it raises on a world that states none.
pub const SYNTH_CONFIG_SHA256_KEY: &str = "provenance.synth_config_sha256";

/// The manifest key that holds the `SynthConfig` field values the exporter used, as a JSON
/// object (parity-fixture-contract §5.1); required on a `synthetic-world` case, because it is
/// what [`SYNTH_CONFIG_SHA256_KEY`] hashes and what the §10 shell guard may recompute the hash
/// from. [`load_world`] requires it and leaves it on the manifest (`params`).
pub const SYNTH_CONFIG_PARAMS_KEY: &str = "params.synth_config";

/// Whether `value` has the form parity-fixture-contract §5.1 fixes for the configuration hash:
/// exactly 64 characters, each `0-9` or `a-f`. Form only; the hash is never recomputed here
/// (DR-D29).
fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Reads a v0 world from a `synthetic-world` case (synthetic-world.md §5.3 (a); S-1).
///
/// Reads only the world the legacy oracle exported, and fails closed on anything else, because
/// [`from_legacy_v0`] labels what it loads `generator_version = "legacy-59bce1d"`, `profile =
/// synthetic-legacy` (plays-contract §7, §11) and Appendix D forbids a made-up provenance:
///
/// - `contracts.plays` must be present ([`FixtureError::MissingKey`]) and equal to
///   [`LEGACY_PLAYS_CONTRACT`]; another MAJOR is [`FixtureError::UnsupportedContract`]
///   (plays-contract §12 "a reader MUST reject a MAJOR version it does not implement"; the parser
///   is never made flexible);
/// - `oracle.level` must be `legacy` and `oracle.corrections_applied` empty
///   ([`FixtureError::Manifest`] naming the key): the corrected world has the same v0 columns
///   but is another world (parity-fixture-contract §7 rules 1 and 3; ledger entry 1), and its
///   loader arrives with the corrected fixture (P1-05, P1-12);
/// - `provenance.profile`, when the exporter wrote it, must be `synthetic-legacy`
///   ([`FixtureError::Manifest`]); an absent key is accepted because §4 does not require it;
/// - `params.synth_config` ([`SYNTH_CONFIG_PARAMS_KEY`]) must be present
///   ([`FixtureError::MissingKey`]) and a JSON object ([`FixtureError::Manifest`]): the
///   `SynthConfig` field values the exporter used, which the configuration hash is taken over
///   (parity-fixture-contract §5.1);
/// - `provenance.synth_config_sha256` ([`SYNTH_CONFIG_SHA256_KEY`]), when present, must be
///   exactly 64 lowercase hexadecimal characters ([`FixtureError::Manifest`] naming the key). It
///   is carried, never recomputed (DR-D29). A case that omits it loads with `config_hash =
///   None`, and [`from_legacy_v0`] then refuses the world with [`ContractError::MissingField`]
///   naming the key.
///
/// Array names, fixed by P1-01: `plays.play_id`, `plays.drive_id`, `plays.week`,
/// `plays.off_team`, `plays.def_team`, `plays.down`, `plays.ydstogo`, `plays.yardline_100`,
/// `plays.n_down`, `plays.n_ydstogo`, `plays.n_yardline_100` (`i64le`); `plays.yards`,
/// `plays.points`, `plays.terminal_value`, `plays.drive_points` (`f64le`); `plays.terminal`
/// (`u8`); `plays.off_players` and `plays.def_players` (ragged: `.offsets` + `.values`, both
/// `i64le`); `players.player_id`, `players.team` (`i64le`), `players.position` (`json`),
/// `players.is_starter` (`u8`), `players.ability` (`f64le`); `gt.team_strength` (`f64le`,
/// indexed by `gt.team_strength.team`, `i64le`), `gt.focus_qb` (`i64le`, shape `[1]`),
/// `gt.focus_tau` (`f64le`), `gt.league_factor` (`f64le`, shape `[1]`); `college.player_id`
/// (`i64le`), `college.position` (`json`), `college.feeder_sv` (`f64le`),
/// `college.feeder_snaps` (`i64le`), `college.is_rookie` (`u8`).
///
/// The provenance block takes the manifest's `seeds` and `provenance.synth_config_sha256`. The
/// configuration **label** (`provenance.synth_config`, `CANONICAL_SYNTH`) is distinct from the
/// hash and is not read into the world: the v1 frame has no field for it; it stays readable on
/// [`FixtureCase::manifest`]`().provenance.synth_config`, as the hashed configuration stays on
/// `().params`. A case whose `component` is not `synthetic-world` is [`FixtureError::Manifest`].
/// Column lengths are not cross-checked here; [`from_legacy_v0`] does that with a typed
/// [`ContractError`].
pub fn load_world(case: &FixtureCase) -> Result<LegacyV0World, FixtureError> {
    let manifest = case.manifest();
    if manifest.component != SYNTHETIC_WORLD_COMPONENT {
        return Err(FixtureError::Manifest {
            detail: format!(
                "component is `{}`; load_world reads `{SYNTHETIC_WORLD_COMPONENT}` cases",
                manifest.component
            ),
        });
    }
    let plays_contract =
        manifest
            .contracts
            .get("plays")
            .ok_or_else(|| FixtureError::MissingKey {
                key: "contracts.plays".to_owned(),
            })?;
    if plays_contract != LEGACY_PLAYS_CONTRACT {
        return Err(FixtureError::UnsupportedContract {
            field: "contracts.plays".to_owned(),
            found: plays_contract.clone(),
            supported: LEGACY_PLAYS_CONTRACT.to_owned(),
        });
    }
    if manifest.oracle.level != OracleLevel::Legacy {
        return Err(FixtureError::Manifest {
            detail: format!(
                "oracle.level is `{}`; load_world reads the `{}` oracle's world only (the \
                 corrected world's loader arrives with its fixture)",
                manifest.oracle.level,
                OracleLevel::Legacy
            ),
        });
    }
    if !manifest.oracle.corrections_applied.is_empty() {
        return Err(FixtureError::Manifest {
            detail: format!(
                "oracle.corrections_applied lists {:?}; a legacy world has no ledger entry applied",
                manifest.oracle.corrections_applied
            ),
        });
    }
    let legacy_profile = FrameProfile::SyntheticLegacy.to_string();
    if let Some(profile) = &manifest.provenance.profile {
        if *profile != legacy_profile {
            return Err(FixtureError::Manifest {
                detail: format!(
                    "provenance.profile is `{profile}`; load_world reads `{legacy_profile}` \
                     worlds only"
                ),
            });
        }
    }
    // parity-fixture-contract §5.1: the hashed configuration must be stated as an object, and
    // the hash, when stated, must have the form the contract fixes. Neither is recomputed.
    let params = match &manifest.params {
        Some(serde_json::Value::Object(obj)) => obj,
        Some(_) => {
            return Err(FixtureError::Manifest {
                detail: "`params` must be a JSON object".to_owned(),
            })
        }
        None => {
            return Err(FixtureError::MissingKey {
                key: SYNTH_CONFIG_PARAMS_KEY.to_owned(),
            })
        }
    };
    match params.get("synth_config") {
        Some(serde_json::Value::Object(_)) => {}
        Some(_) => {
            return Err(FixtureError::Manifest {
                detail: format!("`{SYNTH_CONFIG_PARAMS_KEY}` must be a JSON object"),
            })
        }
        None => {
            return Err(FixtureError::MissingKey {
                key: SYNTH_CONFIG_PARAMS_KEY.to_owned(),
            })
        }
    }
    let config_hash = manifest.provenance.synth_config_sha256.clone();
    if let Some(hash) = &config_hash {
        if !is_sha256_hex(hash) {
            return Err(FixtureError::Manifest {
                detail: format!(
                    "`{SYNTH_CONFIG_SHA256_KEY}` is `{hash}`; it must be a sha256 written as 64 \
                     lowercase hexadecimal characters (parity-fixture-contract §5.1)"
                ),
            });
        }
    }
    let seeds = manifest
        .seeds
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();

    let plays = LegacyV0Plays {
        play_id: case.read_i64("plays.play_id")?.data,
        drive_id: case.read_i64("plays.drive_id")?.data,
        week: case.read_i64("plays.week")?.data,
        off_team: case.read_i64("plays.off_team")?.data,
        def_team: case.read_i64("plays.def_team")?.data,
        down: case.read_i64("plays.down")?.data,
        ydstogo: case.read_i64("plays.ydstogo")?.data,
        yardline_100: case.read_i64("plays.yardline_100")?.data,
        yards: case.read_f64("plays.yards")?.data,
        points: case.read_f64("plays.points")?.data,
        terminal: case.read_bool("plays.terminal")?.data,
        terminal_value: case.read_f64("plays.terminal_value")?.data,
        n_down: case.read_i64("plays.n_down")?.data,
        n_ydstogo: case.read_i64("plays.n_ydstogo")?.data,
        n_yardline_100: case.read_i64("plays.n_yardline_100")?.data,
        drive_points: case.read_f64("plays.drive_points")?.data,
        off_players: case.read_ragged_i64("plays.off_players")?,
        def_players: case.read_ragged_i64("plays.def_players")?,
    };
    let players = LegacyV0Players {
        player_id: case.read_i64("players.player_id")?.data,
        team: case.read_i64("players.team")?.data,
        position: case.read_strings("players.position")?.data,
        is_starter: case.read_bool("players.is_starter")?.data,
        ability: case.read_f64("players.ability")?.data,
    };
    let truth = LegacyV0Truth {
        team_strength_team: case.read_i64("gt.team_strength.team")?.data,
        team_strength: case.read_f64("gt.team_strength")?.data,
        focus_qb: case.read_scalar_i64("gt.focus_qb")?,
        focus_tau: case.read_f64("gt.focus_tau")?.data,
        league_factor: case.read_scalar_f64("gt.league_factor")?,
    };
    let college = LegacyV0College {
        player_id: case.read_i64("college.player_id")?.data,
        position: case.read_strings("college.position")?.data,
        feeder_sv: case.read_f64("college.feeder_sv")?.data,
        feeder_snaps: case.read_i64("college.feeder_snaps")?.data,
        is_rookie: case.read_bool("college.is_rookie")?.data,
    };
    Ok(LegacyV0World {
        plays,
        players,
        truth,
        college,
        provenance: LegacyV0Provenance { config_hash, seeds },
    })
}

/// The share of listed defender ids whose roster team equals the play's `def_team`, over every
/// `Participation::Listed` row of the world: dimensionless, in `[0, 1]`; a defender with no
/// roster entry counts in the denominator only. `None` when no play lists a defender, since a
/// share over an empty set has no value (never 0, KI-P1). A diagnostic for synthetic-world.md
/// §10.4 W-5: on the legacy world every `def_players` id belongs to `off_team`, so the share is
/// `0.0` (KI-NEW-Y0); on a Rust-native world it must be exactly `1.0`.
pub fn defender_team_share(world: &UpgradedWorld) -> Option<f64> {
    let team_of: BTreeMap<&PlayerKey, &TeamKey> = world
        .players
        .players
        .iter()
        .map(|p| (&p.player_id, &p.team))
        .collect();
    let mut total = 0usize;
    let mut on_def_team = 0usize;
    for play in &world.plays.plays {
        if let Participation::Listed { defense, .. } = &play.participation {
            for id in defense {
                total += 1;
                if team_of.get(id).is_some_and(|t| **t == play.def_team) {
                    on_def_team += 1;
                }
            }
        }
    }
    if total == 0 {
        None
    } else {
        Some(on_def_team as f64 / total as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_game_id_orders_the_teams() {
        let week = Week::new(3).unwrap();
        assert_eq!(
            synthetic_game_id(Season(2023), week, 7, 2).0,
            "synth-2023-03-2-7"
        );
        assert_eq!(
            synthetic_game_id(Season(2023), week, 2, 7).0,
            "synth-2023-03-2-7"
        );
        assert_eq!(
            synthetic_game_id(Season(1), Week::new(14).unwrap(), 0, 0).0,
            "synth-1-14-0-0"
        );
    }

    #[test]
    fn drive_points_vocabulary_is_closed() {
        assert_eq!(drive_points_of("f", 0.0, 0), Ok(DrivePoints::Zero));
        assert_eq!(drive_points_of("f", 3.0, 0), Ok(DrivePoints::FieldGoal));
        assert_eq!(drive_points_of("f", 7.0, 0), Ok(DrivePoints::Touchdown));
        for bad in [6.0, -3.0, f64::NAN, 2.9999] {
            assert!(matches!(
                drive_points_of("f", bad, 0),
                Err(ContractError::InvalidValue { field, .. }) if field == "f"
            ));
        }
    }

    #[test]
    fn keys_outside_the_synthetic_encoding_are_rejected() {
        assert!(synthetic_player("p", &PlayerKey::Gsis("00-0033873".into())).is_err());
        assert_eq!(synthetic_player("p", &PlayerKey::Synthetic(7)), Ok(7));
        assert!(synthetic_team("t", &TeamKey::Nfl("KC".into())).is_err());
        assert_eq!(synthetic_team("t", &TeamKey::Synthetic(3)), Ok(3));
        assert!(player_key("p", -1, 4).is_err());
        assert!(team_key("t", i64::from(u16::MAX) + 1, 0).is_err());
        assert!(to_u8("d", 256, 0).is_err());
    }

    #[test]
    fn configuration_hash_form_is_64_lowercase_hex() {
        assert!(is_sha256_hex(&"0".repeat(64)));
        assert!(is_sha256_hex(&"0123456789abcdef".repeat(4)));
        assert!(!is_sha256_hex(&"0".repeat(63)), "too short");
        assert!(!is_sha256_hex(&"0".repeat(65)), "too long");
        assert!(!is_sha256_hex(&"0123456789ABCDEF".repeat(4)), "uppercase");
        assert!(!is_sha256_hex(&"0123456789abcdeg".repeat(4)), "not hex");
        assert!(!is_sha256_hex(""), "empty");
        assert!(
            !is_sha256_hex(&format!("{}é", "0".repeat(62))),
            "63 characters and 64 bytes is not 64 hex digits"
        );
    }
}
