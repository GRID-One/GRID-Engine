//! The serialized spelling of the contract types (ADR-014 decision 2: "Field names in the
//! serialized form are the contract's field names; a rename is a contract change"
//! (engine-output-contract §9)).
//!
//! The round-trip tests in `output_round_trip.rs` prove that a value comes back equal; they would
//! still pass if a variant or a field were renamed on both sides. This file pins the text itself,
//! with `serde_json` literal values, so that a rename fails here and is reviewed as the contract
//! change it is.

use grid_domain::{
    AsOfMode, AvailabilitySource, FrameProfile, Freshness, Horizon, Lock, LockKind, MarketUnits,
    ParticipantPosition, PlayerKey, Position, RegimeFlags, StatComponent, StatDistribution,
    TeamKey, TerminalState, TouchdownType, UtcInstant,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};
use std::fmt::Debug;

/// Serializes `value`, checks the JSON equals `expected`, and checks that `expected` reads back
/// to `value`.
fn pins<T>(value: &T, expected: Value)
where
    T: Serialize + DeserializeOwned + PartialEq + Debug,
{
    let written = serde_json::to_value(value).expect("serializes");
    assert_eq!(written, expected, "serialized form of {value:?}");
    let read: T = serde_json::from_value(expected).expect("deserializes");
    assert_eq!(&read, value, "read back");
}

#[test]
fn position_is_upper_case() {
    // engine-output-contract §2 `position`: QB, RB, WR, TE.
    pins(&Position::Qb, json!("QB"));
    pins(&Position::Rb, json!("RB"));
    pins(&Position::Wr, json!("WR"));
    pins(&Position::Te, json!("TE"));
}

#[test]
fn every_stat_component_serializes_to_its_contract_name() {
    // engine-output-contract §3.2: the snake_case component names.
    for c in StatComponent::ALL {
        pins(&c, json!(c.contract_name()));
    }
    pins(
        &StatComponent::OffensiveSnapShare,
        json!("offensive_snap_share"),
    );
    pins(
        &StatComponent::TwoPointConversions,
        json!("two_point_conversions"),
    );
}

#[test]
fn frame_profile_is_kebab_case() {
    // plays-contract §7 (`profile = synthetic-legacy`); parity-fixture-contract §4
    // `provenance.profile`.
    pins(&FrameProfile::SyntheticLegacy, json!("synthetic-legacy"));
    pins(&FrameProfile::RealData, json!("real-data"));
}

#[test]
fn market_units_are_the_variant_names() {
    // plays-contract §5 v1; synthetic-world.md §4.10 G-5.
    pins(&MarketUnits::DvPerPlay, json!("DvPerPlay"));
    pins(&MarketUnits::SyntheticAbility, json!("SyntheticAbility"));
}

#[test]
fn closed_output_enums_are_the_contract_spellings() {
    // engine-output-contract §2 `horizon`: `Week | RestOfSeason | Preseason`.
    pins(&Horizon::Week, json!("Week"));
    pins(&Horizon::RestOfSeason, json!("RestOfSeason"));
    pins(&Horizon::Preseason, json!("Preseason"));
    // engine-output-contract §2 `lock.kind`: `Thursday | Sunday | Operational`.
    pins(&LockKind::Thursday, json!("Thursday"));
    pins(&LockKind::Sunday, json!("Sunday"));
    pins(&LockKind::Operational, json!("Operational"));
    // engine-output-contract §3.8 `freshness`: `Fresh | Stale | Missing`.
    pins(&Freshness::Fresh, json!("Fresh"));
    pins(&Freshness::Stale, json!("Stale"));
    pins(&Freshness::Missing, json!("Missing"));
    // engine-output-contract §3.3 `availability_source`: `Override | Provider | Model | Missing`.
    pins(&AvailabilitySource::Override, json!("Override"));
    pins(&AvailabilitySource::Provider, json!("Provider"));
    pins(&AvailabilitySource::Model, json!("Model"));
    pins(&AvailabilitySource::Missing, json!("Missing"));
    // evaluation-and-leakage.md §4.1 `mode`.
    pins(&AsOfMode::Backtest, json!("Backtest"));
    pins(&AsOfMode::Production, json!("Production"));
    pins(&AsOfMode::Research, json!("Research"));
}

#[test]
fn terminal_state_is_the_variant_name() {
    // The serialized form is the `TerminalState` variant column of engine-output-contract §13,
    // not the SCREAMING_SNAKE spelling column of engine-spec §8.6.4 (`NO_NEW_DATA`). No document
    // states which column is the serialized form; this pins what P1-01 writes so that a change
    // is a reviewed contract change.
    let expected = [
        (TerminalState::NotRun, "NotRun"),
        (TerminalState::Running, "Running"),
        (TerminalState::Success, "Success"),
        (TerminalState::NoNewData, "NoNewData"),
        (TerminalState::PartialSuccess, "PartialSuccess"),
        (TerminalState::NetworkFailure, "NetworkFailure"),
        (TerminalState::Timeout, "Timeout"),
        (TerminalState::RateLimited, "RateLimited"),
        (
            TerminalState::AuthenticationFailure,
            "AuthenticationFailure",
        ),
        (TerminalState::InvalidResponse, "InvalidResponse"),
        (TerminalState::SchemaFailure, "SchemaFailure"),
        (
            TerminalState::DataValidationFailure,
            "DataValidationFailure",
        ),
        (TerminalState::DatabaseFailure, "DatabaseFailure"),
        (TerminalState::LearningFailure, "LearningFailure"),
    ];
    assert_eq!(expected.len(), TerminalState::ALL.len());
    for (state, text) in expected {
        pins(&state, json!(text));
    }
}

#[test]
fn lock_has_the_contract_keys() {
    // engine-output-contract §2 `lock`: `{ kind, at: UtcInstant }`; `UtcInstant` is whole Unix
    // seconds, UTC.
    pins(
        &Lock {
            kind: LockKind::Sunday,
            at: UtcInstant::from_unix_seconds(1_696_770_000),
        },
        json!({ "kind": "Sunday", "at": 1_696_770_000_i64 }),
    );
}

#[test]
fn player_and_team_keys_are_externally_tagged() {
    pins(
        &PlayerKey::Gsis("00-9900001".into()),
        json!({ "Gsis": "00-9900001" }),
    );
    pins(&PlayerKey::Synthetic(17), json!({ "Synthetic": 17 }));
    pins(&TeamKey::Nfl("KC".into()), json!({ "Nfl": "KC" }));
    pins(&TeamKey::Synthetic(3), json!({ "Synthetic": 3 }));
}

#[test]
fn open_label_enums_are_the_raw_label() {
    pins(&ParticipantPosition::Qb, json!("QB"));
    pins(&ParticipantPosition::Def, json!("DEF"));
    pins(&ParticipantPosition::Other("OLB".into()), json!("OLB"));
    pins(&TouchdownType::Receiving, json!("Receiving"));
    pins(
        &TouchdownType::Other("Defensive".into()),
        json!("Defensive"),
    );
}

#[test]
fn a_hand_built_other_with_a_known_label_reads_back_as_the_known_variant() {
    // A reader never produces `Other` with a known label. A hand-built one serializes to the
    // known label's text and therefore reads back as the known variant: equality after a round
    // trip holds only for values a reader produced.
    let text = serde_json::to_string(&ParticipantPosition::Other("QB".into())).unwrap();
    assert_eq!(text, "\"QB\"");
    let read: ParticipantPosition = serde_json::from_str(&text).unwrap();
    assert_eq!(read, ParticipantPosition::Qb);

    let text = serde_json::to_string(&TouchdownType::Other("Passing".into())).unwrap();
    assert_eq!(text, "\"Passing\"");
    let read: TouchdownType = serde_json::from_str(&text).unwrap();
    assert_eq!(read, TouchdownType::Passing);
}

#[test]
fn golden_json_text_of_small_records() {
    // Field names, field order and the `null` of an absent optional are all pinned by the text.
    let flags = RegimeFlags {
        intervention: true,
        observation_noise_inflation: false,
        scheme_reset: false,
        changepoint_z: None,
    };
    let golden = r#"{"intervention":true,"observation_noise_inflation":false,"scheme_reset":false,"changepoint_z":null}"#;
    assert_eq!(serde_json::to_string(&flags).unwrap(), golden);
    assert_eq!(serde_json::from_str::<RegimeFlags>(golden).unwrap(), flags);

    let dist = StatDistribution {
        mean: 1.5,
        sd: 0.5,
        q10: 0.75,
        q25: 1.0,
        q50: 1.5,
        q75: 2.0,
        q90: 2.25,
    };
    let golden = r#"{"mean":1.5,"sd":0.5,"q10":0.75,"q25":1.0,"q50":1.5,"q75":2.0,"q90":2.25}"#;
    assert_eq!(serde_json::to_string(&dist).unwrap(), golden);
    assert_eq!(
        serde_json::from_str::<StatDistribution>(golden).unwrap(),
        dist
    );
}
