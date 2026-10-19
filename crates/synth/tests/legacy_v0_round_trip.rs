//! `from_legacy_v0` and `to_v0_columns` on a hand-built v0 world, and `load_world` over a
//! `synthetic-world` case written into a temporary directory (plays-contract §11;
//! synthetic-world.md §10.3 S-1 on a hand-built frame; P1-01 acceptance criteria for
//! `grid-synth`). The world carries a NaN `terminal_value`, −1 `n_*` sentinels, a duplicated
//! participant, `yardline_100 = 104` and a zero-yard play, so the legacy profile must accept it
//! and every column must come back bit-identical under the S-1 criterion; a `-0.0` yards, which
//! the oracle never writes and an `i16` cannot carry, is refused. The manifest's configuration
//! hash (`provenance.synth_config_sha256`, parity-fixture-contract §5.1) is carried into the
//! producer as written, refused when malformed, and its absence is reported by the key's name.

mod common;

use std::collections::BTreeMap;

use common::{toy_synth_config, toy_world, write_world_case, TempCase, TOY_SYNTH_CONFIG_SHA256};
use grid_domain::{
    ContractError, DrivePoints, FrameProducer, FrameProfile, NextState, ParticipantPosition,
    Participation, ParticipationStatus, PlayerKey, Season, SeasonType, TeamKey,
    GRID_PLAYS_CONTRACT_VERSION,
};
use grid_synth::fixture::{FixtureCase, FixtureError};
use grid_synth::legacy_v0::{
    defender_team_share, from_legacy_v0, load_world, to_v0_columns, LegacyV0World,
    LEGACY_GENERATOR_VERSION, LEGACY_PLAYS_CONTRACT, SYNTH_CONFIG_PARAMS_KEY,
    SYNTH_CONFIG_SHA256_KEY,
};
use serde_json::json;

const SEASON: Season = Season(2023);

#[test]
fn from_legacy_v0_upgrades_and_the_legacy_profile_accepts_the_world() {
    let v0 = toy_world();
    let up = from_legacy_v0(&v0, SEASON).expect("a consistent v0 world upgrades");
    // from_legacy_v0 validated already; validating again is idempotent.
    assert_eq!(up.plays.validate(&up.players), Ok(()));

    let meta = &up.plays.meta;
    assert_eq!(meta.contract, GRID_PLAYS_CONTRACT_VERSION);
    assert_eq!(
        meta.producer,
        FrameProducer::Synthetic {
            generator_version: LEGACY_GENERATOR_VERSION.into(),
            config_hash: TOY_SYNTH_CONFIG_SHA256.into(),
            seeds: vec![("college".into(), 106), ("synth".into(), 7)],
            profile: FrameProfile::SyntheticLegacy,
        }
    );
    assert_eq!(meta.producer.profile(), FrameProfile::SyntheticLegacy);
    assert!(meta.sources.is_empty());
    assert_eq!(
        meta.participation_availability,
        vec![(SEASON, ParticipationStatus::Synthetic)]
    );
    assert_eq!(meta.coverage.len(), 1);
    assert_eq!(meta.coverage[0].season, SEASON);
    assert_eq!(meta.coverage[0].offense_share, 1.0);
    assert_eq!(meta.coverage[0].defense_share, 1.0);

    assert_eq!(up.plays.plays.len(), 6);
    let p0 = &up.plays.plays[0];
    assert_eq!(p0.game_id.0, "synth-2023-01-0-1");
    assert_eq!(p0.play_id, 0);
    assert_eq!(p0.drive_seq, 1);
    assert_eq!(p0.season, SEASON);
    assert_eq!(p0.season_type, SeasonType::Regular);
    assert_eq!(p0.week.get(), 1);
    assert_eq!(p0.off_team, TeamKey::Synthetic(0));
    assert_eq!(p0.def_team, TeamKey::Synthetic(1));
    assert_eq!(p0.clock, None);
    assert_eq!(p0.yards, 12);
    assert_eq!(up.plays.plays[2].yards, 0);
    assert_eq!(up.plays.plays[3].yards, -8);
    assert_eq!(p0.drive_points, DrivePoints::FieldGoal);
    assert!(
        matches!(p0.next, NextState::Continue(s) if s.down == 1 && s.ydstogo == 10 && s.yardline_100 == 63)
    );
    // Order and the duplicate are kept (D-8).
    assert_eq!(
        p0.participation,
        Participation::Listed {
            offense: [0u32, 1, 2, 2, 4]
                .iter()
                .map(|&n| PlayerKey::Synthetic(n))
                .collect(),
            defense: (5u32..=11).map(PlayerKey::Synthetic).collect(),
        }
    );
    // The terminal row of drive 1, the 104-yard state of drive 2 (D-9), the week-2 game id.
    assert_eq!(up.plays.plays[2].next, NextState::Terminal);
    assert_eq!(up.plays.plays[4].state.yardline_100, 104);
    assert_eq!(up.plays.plays[3].game_id.0, "synth-2023-01-0-1");
    assert_eq!(up.plays.plays[5].game_id.0, "synth-2023-02-0-1");
    assert_eq!(up.plays.drives().len(), 3);
    // Defenders come from the offense on this legacy-shaped world (KI-NEW-Y0).
    assert_eq!(defender_team_share(&up), Some(0.0));

    assert_eq!(up.players.players.len(), 24);
    assert_eq!(up.players.players[5].position, ParticipantPosition::Def);
    assert_eq!(up.players.players[12].team, TeamKey::Synthetic(1));

    let truth = &up.truth;
    assert_eq!(truth.focus_qb, PlayerKey::Synthetic(0));
    assert_eq!(truth.focus_tau, vec![Some(0.075), None, Some(0.1)]);
    assert_eq!(truth.league_factor, 0.62);
    assert_eq!(truth.net_strength, None);
    assert_eq!(truth.ability.len(), 24);
    // The toy world plants `id × 0.01 − 0.1`; compare against the planted column bit for bit.
    assert_eq!(
        truth
            .ability
            .get(&PlayerKey::Synthetic(12))
            .map(|a| a.to_bits()),
        Some(v0.players.ability[12].to_bits())
    );
    assert_eq!(truth.is_starter.get(&PlayerKey::Synthetic(5)), Some(&false));
    assert_eq!(truth.is_starter.get(&PlayerKey::Synthetic(6)), Some(&true));
    let strengths: BTreeMap<TeamKey, f64> =
        [(TeamKey::Synthetic(0), 0.05), (TeamKey::Synthetic(1), 0.10)]
            .into_iter()
            .collect();
    assert_eq!(truth.team_strength_legacy, strengths);

    assert_eq!(up.college.rows.len(), 3);
    assert_eq!(up.college.rows[1].player_id, PlayerKey::Synthetic(12));
    assert_eq!(up.college.rows[1].feeder_snaps, Some(450));
    assert!(up.college.rows[1].is_rookie);
}

#[test]
fn to_v0_columns_rebuilds_identical_columns() {
    let v0 = toy_world();
    let up = from_legacy_v0(&v0, SEASON).expect("upgrade");
    let back = to_v0_columns(&up).expect("inverse on the image");
    assert_eq!(
        back.first_difference(&v0),
        None,
        "every column, id order, NaN position and sentinel must round-trip"
    );
    // The derived columns are exactly what the oracle wrote.
    assert!(back.plays.terminal_value[0].is_nan());
    assert_eq!(back.plays.terminal_value[2], 3.0);
    assert_eq!(back.plays.n_down[2], -1);
    assert_eq!(back.plays.points, v0.plays.points);
    assert!(back.truth.focus_tau[1].is_nan());
    // The derived float columns come back bit for bit, the zero-yard row and the −8 included.
    for (r, p) in back.plays.yards.iter().zip(&v0.plays.yards) {
        assert_eq!(r.to_bits(), p.to_bits());
    }
    assert_eq!(back.plays.yards[2].to_bits(), 0.0f64.to_bits());
    assert_eq!(
        back.provenance.config_hash.as_deref(),
        Some(TOY_SYNTH_CONFIG_SHA256)
    );
    // Upgrading again gives the same v1 world.
    assert_eq!(from_legacy_v0(&back, SEASON), Ok(up));
}

#[test]
fn from_legacy_v0_refuses_a_world_that_states_no_configuration_hash() {
    // `FrameProducer::Synthetic::config_hash` is a content hash (plays-contract §9), recorded by
    // the manifest key `provenance.synth_config_sha256` (parity-fixture-contract §5.1). A world
    // that states none is refused by that key's name rather than given a non-hash (engine-spec
    // Appendix D).
    let mut w = toy_world();
    w.provenance.config_hash = None;
    assert_eq!(
        from_legacy_v0(&w, SEASON),
        Err(ContractError::MissingField {
            field: "provenance.synth_config_sha256".into()
        })
    );
    assert_eq!(SYNTH_CONFIG_SHA256_KEY, "provenance.synth_config_sha256");
}

#[test]
fn first_difference_names_the_column_and_index() {
    let a = toy_world();
    let mut b = toy_world();
    assert_eq!(a.first_difference(&b), None);
    b.plays.off_players.values[3] = 3; // the duplicate WR becomes WR 3
    let (column, m) = a.first_difference(&b).expect("differs");
    assert_eq!(column, "plays.off_players.values");
    assert_eq!(m.index, 3);
    let mut c = toy_world();
    c.plays.terminal_value[0] = 0.0; // NaN position moves
    let (column, m) = a.first_difference(&c).expect("NaN position differs");
    assert_eq!(column, "plays.terminal_value");
    assert_eq!(m.index, 0);
    assert_eq!(
        m.observed, None,
        "a NaN-position mismatch has no observed number"
    );
    // A value change on an integral column is reported with |r − p|.
    let mut d = toy_world();
    d.plays.yards[1] = 4.0;
    let (column, m) = a.first_difference(&d).expect("value differs");
    assert_eq!(
        (column.as_str(), m.index, m.observed),
        ("plays.yards", 1, Some(1.0))
    );
    // A pass-through float column is compared bit for bit: a NaN payload change is a difference.
    let mut e = toy_world();
    e.truth.focus_tau[1] = f64::from_bits(0x7ff8_0000_0000_0001);
    let (column, m) = a.first_difference(&e).expect("payload differs");
    assert_eq!((column.as_str(), m.index), ("gt.focus_tau", 1));
}

fn expect_invalid(world: &LegacyV0World, field: &str) {
    match from_legacy_v0(world, SEASON) {
        Err(ContractError::InvalidValue { field: f, .. }) if f == field => {}
        other => panic!("expected InvalidValue on `{field}`, got {other:?}"),
    }
}

#[test]
fn inconsistent_v0_columns_are_rejected_not_coerced() {
    // A non-terminal row whose terminal_value is not NaN.
    let mut w = toy_world();
    w.plays.terminal_value[0] = 0.0;
    expect_invalid(&w, "plays.terminal_value");

    // A missing value that is a NaN with a non-canonical payload would not round-trip bit for bit
    // (to_v0_columns writes the canonical quiet NaN), so it is refused, not normalised.
    let mut w = toy_world();
    w.plays.terminal_value[0] = f64::from_bits(0x7ff8_0000_0000_0001);
    assert!(w.plays.terminal_value[0].is_nan());
    expect_invalid(&w, "plays.terminal_value");
    let mut w = toy_world();
    w.truth.focus_tau[1] = f64::from_bits(0xfff8_0000_0000_0000);
    expect_invalid(&w, "gt.focus_tau");
    // A terminal row whose n_* are not the −1 sentinel.
    let mut w = toy_world();
    w.plays.n_down[2] = 1;
    expect_invalid(&w, "plays.n_down");
    // A terminal row whose points differ from drive_points.
    let mut w = toy_world();
    w.plays.points[2] = 0.0;
    expect_invalid(&w, "plays.points");
    // A non-terminal row carrying points.
    let mut w = toy_world();
    w.plays.points[0] = 3.0;
    expect_invalid(&w, "plays.points");
    // drive_points outside {0, 3, 7}.
    let mut w = toy_world();
    w.plays.drive_points[5] = 6.0;
    expect_invalid(&w, "plays.drive_points");
    // Non-integral yards.
    let mut w = toy_world();
    w.plays.yards[1] = 2.5;
    expect_invalid(&w, "plays.yards");
    // A negative zero: integral, but `i16` has no signed zero, so the v0 view could not give the
    // bit back, and the oracle never writes one (NumPy 2.x `round` returns a Python `int`).
    let mut w = toy_world();
    w.plays.yards[2] = -0.0;
    expect_invalid(&w, "plays.yards");
    // A column of the wrong length.
    let mut w = toy_world();
    w.plays.week.pop();
    expect_invalid(&w, "plays.week");
    // Week 0 (KI-A4).
    let mut w = toy_world();
    w.plays.week[0] = 0;
    expect_invalid(&w, "week");
    // A negative participant id.
    let mut w = toy_world();
    w.plays.def_players.values[0] = -5;
    expect_invalid(&w, "plays.def_players");
    // Ragged offsets built from the public fields that break the §2.1 invariants: a typed error,
    // never the `Ragged::row` panic.
    let mut w = toy_world();
    w.plays.off_players.offsets[1] = 99;
    expect_invalid(&w, "plays.off_players.offsets");
    let mut w = toy_world();
    w.plays.def_players.offsets[0] = -1;
    expect_invalid(&w, "plays.def_players.offsets");
    let mut w = toy_world();
    w.plays.def_players.offsets.pop();
    w.plays.def_players.values.truncate(35); // offsets stays consistent, one row short
    expect_invalid(&w, "plays.def_players");
    // A duplicated player id in the truth columns.
    let mut w = toy_world();
    w.players.player_id[1] = 0;
    assert!(matches!(
        from_legacy_v0(&w, SEASON),
        Err(ContractError::DuplicateKey { frame, key }) if frame == "players" && key == "0"
    ));
    // The core rules still apply under the legacy profile: a drive whose last row continues.
    let mut w = toy_world();
    w.plays.terminal[5] = false;
    w.plays.terminal_value[5] = f64::NAN;
    w.plays.points[5] = 0.0;
    w.plays.n_down[5] = 2;
    w.plays.n_ydstogo[5] = 5;
    w.plays.n_yardline_100[5] = 2;
    assert!(matches!(
        from_legacy_v0(&w, SEASON),
        Err(ContractError::NonTerminalLastRow { drive }) if drive.drive_seq == 3
    ));
    // An unknown participant is counted, never dropped (DR-B6).
    let mut w = toy_world();
    w.plays.off_players.values[0] = 99;
    w.plays.off_players.values[5] = 99;
    assert_eq!(
        from_legacy_v0(&w, SEASON),
        Err(ContractError::UnknownParticipant { count: 2 })
    );
}

#[test]
fn to_v0_columns_fails_where_v0_has_no_encoding() {
    let up = from_legacy_v0(&toy_world(), SEASON).expect("upgrade");
    let mut no_snaps = up.clone();
    no_snaps.college.rows[0].feeder_snaps = None;
    assert!(matches!(
        to_v0_columns(&no_snaps),
        Err(ContractError::MissingField { field }) if field.contains("college.feeder_snaps")
    ));
    let mut real_key = up.clone();
    real_key.players.players[0].player_id = PlayerKey::Gsis("00-9900001".into());
    assert!(matches!(
        to_v0_columns(&real_key),
        Err(ContractError::InvalidValue { field, .. }) if field == "players.player_id"
    ));
    let mut nflverse = up;
    nflverse.plays.meta.producer = FrameProducer::Nflverse {
        adapter_version: "test".into(),
        normalization_map_version: "1".into(),
    };
    assert!(matches!(
        to_v0_columns(&nflverse),
        Err(ContractError::InvalidValue { field, .. }) if field == "meta.producer"
    ));
}

#[test]
fn load_world_reads_a_synthetic_world_case_from_a_temp_dir() {
    let case = TempCase::new("world");
    let v0 = toy_world();
    write_world_case(&case, &v0);
    let fx = FixtureCase::open(case.path()).expect("valid case");
    assert_eq!(fx.manifest().component, "synthetic-world");
    assert_eq!(fx.manifest().outputs.len(), 35);
    let loaded = load_world(&fx).expect("every array present");
    // The §5.1 hash is carried as the manifest states it; the label beside it and the hashed
    // configuration stay on the manifest.
    assert_eq!(
        loaded.provenance.config_hash.as_deref(),
        Some(TOY_SYNTH_CONFIG_SHA256)
    );
    assert_eq!(
        fx.manifest().provenance.synth_config_sha256.as_deref(),
        Some(TOY_SYNTH_CONFIG_SHA256)
    );
    assert_eq!(
        fx.manifest().provenance.synth_config.as_deref(),
        Some("CANONICAL_SYNTH")
    );
    assert_eq!(
        fx.manifest().params.as_ref().map(|p| &p["synth_config"]),
        Some(&toy_synth_config())
    );
    assert_eq!(
        loaded.provenance.seeds,
        vec![("college".to_owned(), 106), ("synth".to_owned(), 7)]
    );
    assert_eq!(
        loaded.first_difference(&v0),
        None,
        "the written and read worlds are identical, NaN included"
    );
    // End to end: the loaded world upgrades, the producer carries the manifest's hash unchanged,
    // the frame validates under the legacy profile and the v0 view round-trips.
    let up = from_legacy_v0(&loaded, SEASON).expect("upgrade");
    assert!(matches!(
        &up.plays.meta.producer,
        FrameProducer::Synthetic { config_hash, .. }
            if Some(config_hash.as_str()) == fx.manifest().provenance.synth_config_sha256.as_deref()
    ));
    assert_eq!(up.plays.validate(&up.players), Ok(()));
    let back = to_v0_columns(&up).expect("inverse");
    assert_eq!(back.first_difference(&v0), None);
}

#[test]
fn load_world_without_the_hash_key_loads_and_from_legacy_v0_refuses_the_world() {
    // parity-fixture-contract §5.1 requires the key on a `synthetic-world` case; the refusal
    // sits where the hash would be written into the producer (`from_legacy_v0`), so a case that
    // omits it still reads, states no hash, and is reported by the key's name.
    let loaded = load_edited("nohash", |m| {
        m["provenance"]
            .as_object_mut()
            .unwrap()
            .remove("synth_config_sha256");
    })
    .expect("a case without the hash key loads");
    assert_eq!(loaded.provenance.config_hash, None);
    assert_eq!(
        from_legacy_v0(&loaded, SEASON),
        Err(ContractError::MissingField {
            field: "provenance.synth_config_sha256".into()
        })
    );
    // `None` versus `Some` is a difference under S-1: a world that states no hash is not the
    // world that does.
    let (column, m) = loaded
        .first_difference(&toy_world())
        .expect("None versus Some is a difference");
    assert_eq!((column.as_str(), m.index), ("provenance.config_hash", 0));
    // A `null` value is the absent key, never a substituted value (KI-P1).
    let loaded = load_edited("nullhash", |m| {
        m["provenance"]["synth_config_sha256"] = json!(null);
    })
    .expect("null reads as absent");
    assert_eq!(loaded.provenance.config_hash, None);
}

#[test]
fn load_world_rejects_a_malformed_configuration_hash() {
    // §5.1: exactly 64 lowercase hexadecimal characters. The form is checked; the hash is never
    // recomputed (DR-D29 puts verification in the shell guard).
    for (label, bad) in [
        ("short", "0".repeat(63)),
        ("long", "0".repeat(65)),
        ("upper", TOY_SYNTH_CONFIG_SHA256.to_ascii_uppercase()),
        ("nothex", format!("{}zz", &TOY_SYNTH_CONFIG_SHA256[..62])),
        ("empty", String::new()),
    ] {
        let result = load_edited(label, |m| {
            m["provenance"]["synth_config_sha256"] = json!(bad);
        });
        assert!(
            matches!(
                &result,
                Err(FixtureError::Manifest { detail })
                    if detail.contains("provenance.synth_config_sha256")
            ),
            "{label}: {result:?}"
        );
    }
    // Not a string at all: the manifest parser refuses it before `load_world` runs.
    let case = TempCase::new("numhash");
    write_world_case(&case, &toy_world());
    case.edit_manifest(|m| m["provenance"]["synth_config_sha256"] = json!(12));
    assert!(matches!(
        FixtureCase::open(case.path()),
        Err(FixtureError::Manifest { detail }) if detail.contains("provenance.synth_config_sha256")
    ));
}

#[test]
fn load_world_requires_the_hashed_configuration_block() {
    // §5.1: `params.synth_config` is what the hash is taken over and what the §10 guard may
    // recompute it from, so a `synthetic-world` case must state it as a JSON object.
    assert_eq!(SYNTH_CONFIG_PARAMS_KEY, "params.synth_config");
    assert_eq!(
        load_edited("noconfig", |m| {
            m["params"].as_object_mut().unwrap().remove("synth_config");
        }),
        Err(FixtureError::MissingKey {
            key: "params.synth_config".into()
        })
    );
    assert_eq!(
        load_edited("noparams", |m| {
            m.as_object_mut().unwrap().remove("params");
        }),
        Err(FixtureError::MissingKey {
            key: "params.synth_config".into()
        })
    );
    assert!(matches!(
        load_edited("configlabel", |m| m["params"]["synth_config"] =
            json!("CANONICAL_SYNTH")),
        Err(FixtureError::Manifest { detail }) if detail.contains("params.synth_config")
    ));
    assert!(matches!(
        load_edited("paramsarray", |m| m["params"] = json!([])),
        Err(FixtureError::Manifest { detail }) if detail.contains("`params`")
    ));
}

/// Writes the toy world as a case, applies `edit` to its manifest and runs `load_world` on it.
fn load_edited(
    label: &str,
    edit: impl FnOnce(&mut serde_json::Value),
) -> Result<LegacyV0World, FixtureError> {
    let case = TempCase::new(label);
    write_world_case(&case, &toy_world());
    case.edit_manifest(edit);
    let fx = FixtureCase::open(case.path()).expect("the edited manifest is still valid");
    load_world(&fx)
}

#[test]
fn load_world_rejects_the_wrong_component_and_a_missing_array() {
    assert!(matches!(
        load_edited("wrongcomp", |m| m["component"] = json!("rapm-attribution")),
        Err(FixtureError::Manifest { detail }) if detail.contains("synthetic-world")
    ));
    assert_eq!(
        load_edited("missingarray", |m| {
            let outputs = m["outputs"].as_array_mut().unwrap();
            outputs.retain(|e| e["name"] != "gt.focus_tau");
        }),
        Err(FixtureError::UnknownArray {
            name: "gt.focus_tau".into()
        })
    );
    // parity-fixture-contract §4 lists `provenance.synth_config` in its example, not among the
    // required keys; §5.1 keeps it as a label distinct from the hash, and the loader does not
    // read it: a case without it still loads.
    assert!(load_edited("nosynthconfig", |m| {
        m["provenance"]
            .as_object_mut()
            .unwrap()
            .remove("synth_config");
    })
    .is_ok());
}

#[test]
fn load_world_rejects_a_plays_contract_it_does_not_implement() {
    // plays-contract §12: a reader MUST reject a MAJOR version it does not implement.
    assert_eq!(LEGACY_PLAYS_CONTRACT, "grid.plays/0");
    assert_eq!(
        load_edited("plays-v1", |m| m["contracts"]["plays"] =
            json!("grid.plays/1")),
        Err(FixtureError::UnsupportedContract {
            field: "contracts.plays".into(),
            found: "grid.plays/1".into(),
            supported: "grid.plays/0".into(),
        })
    );
    // A case that declares no plays contract at all is not read as v0 silently.
    assert_eq!(
        load_edited("plays-absent", |m| {
            m["contracts"].as_object_mut().unwrap().remove("plays");
        }),
        Err(FixtureError::MissingKey {
            key: "contracts.plays".into()
        })
    );
}

#[test]
fn load_world_reads_only_the_legacy_oracle_level_and_profile() {
    // parity-fixture-contract §7: the corrected world has the same v0 columns but is another
    // world; `from_legacy_v0` would label it `legacy-59bce1d` (Appendix D: no made-up provenance).
    assert!(matches!(
        load_edited("corrected", |m| m["oracle"]["level"] = json!("corrected")),
        Err(FixtureError::Manifest { detail }) if detail.contains("oracle.level")
    ));
    assert!(matches!(
        load_edited("ledger", |m| m["oracle"]["corrections_applied"] = json!(["1"])),
        Err(FixtureError::Manifest { detail }) if detail.contains("oracle.corrections_applied")
    ));
    // plays-contract §7: the legacy profile is flagged `profile = synthetic-legacy`.
    assert!(matches!(
        load_edited("realdata", |m| m["provenance"]["profile"] = json!("real-data")),
        Err(FixtureError::Manifest { detail }) if detail.contains("provenance.profile")
    ));
    // §4 does not require `provenance.profile`; an exporter that omits it is still legacy.
    assert!(load_edited("noprofile", |m| {
        m["provenance"].as_object_mut().unwrap().remove("profile");
    })
    .is_ok());
}
