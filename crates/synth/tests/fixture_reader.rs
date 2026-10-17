//! The DR-D29 reader against cases written into a temporary directory with `std` alone
//! (parity-fixture-contract §2.1, §4, §11; P1-01 acceptance criteria for `grid-synth`):
//! every dtype and the ragged encoding read back; byte-length, dtype and shape mismatches and a
//! missing required key rejected with the typed error; `provenance.data ≠ synthetic` rejected;
//! NaN preserved bit for bit.

mod common;

use std::io::ErrorKind;

use common::{
    base_manifest, entry, f64le, i32le, i64le, json_strings, push_input, push_output, u8bool,
    TempCase,
};
use grid_synth::fixture::{
    DType, FixtureCase, FixtureError, OracleLevel, ToleranceClass, MANIFEST_VERSION,
    PARITY_FIXTURE_CONTRACT_VERSION,
};
use serde_json::json;

/// A case holding one array of every dtype plus a ragged list.
fn every_dtype_case() -> TempCase {
    let case = TempCase::new("dtypes");
    let mut m = base_manifest("scoring", "every-dtype-legacy");
    push_input(&mut m, entry("x", "f64le", &[2, 3], None));
    case.write(
        "x.f64le",
        &f64le(&[1.5, -2.25, 0.0, -0.0, f64::INFINITY, 1e300]),
    );
    push_input(&mut m, entry("ids", "i64le", &[3], None));
    case.write("ids.i64le", &i64le(&[i64::MIN, -1, i64::MAX]));
    push_input(&mut m, entry("small", "i32le", &[2], None));
    case.write("small.i32le", &i32le(&[i32::MIN, 7]));
    push_input(&mut m, entry("flags", "u8", &[4], None));
    case.write("flags.u8", &u8bool(&[true, false, false, true]));
    push_input(&mut m, entry("labels", "json", &[3], None));
    case.write(
        "labels.json",
        &json_strings(&["QB".into(), "DEF".into(), "WR".into()]),
    );
    push_input(&mut m, entry("parts.offsets", "i64le", &[4], None));
    case.write("parts.offsets.i64le", &i64le(&[0, 2, 2, 5]));
    push_input(&mut m, entry("parts.values", "i64le", &[5], None));
    case.write("parts.values.i64le", &i64le(&[10, 10, 11, 12, 13]));
    push_output(&mut m, entry("y", "f64le", &[1], Some("A")));
    case.write("y.f64le", &f64le(&[42.0]));
    push_output(&mut m, entry("beta", "f64le", &[2], Some("A'")));
    case.write("beta.f64le", &f64le(&[1.0, 2.0]));
    case.write_manifest(&m);
    case
}

#[test]
fn reads_every_dtype_and_the_ragged_encoding() {
    let case = every_dtype_case();
    let fx = FixtureCase::open(case.path()).expect("valid case");
    let m = fx.manifest();
    assert_eq!(m.manifest_version, MANIFEST_VERSION);
    assert_eq!(
        m.contracts.get("parity_fixture").map(String::as_str),
        Some(PARITY_FIXTURE_CONTRACT_VERSION)
    );
    assert_eq!(m.oracle.level, OracleLevel::Legacy);
    assert_eq!(m.oracle.ledger_independent, Some(true));
    assert_eq!(
        m.oracle.import_patches.as_deref(),
        Some(["P1".to_owned(), "P2".to_owned()].as_slice())
    );
    assert_eq!(m.oracle.source_commit.as_deref(), Some("59bce1d"));
    assert_eq!(m.seeds.get("synth"), Some(&7));
    assert_eq!(m.provenance.profile.as_deref(), Some("synthetic-legacy"));
    // Not a `synthetic-world` case: the §5.1 configuration hash is absent, never substituted.
    assert_eq!(m.provenance.synth_config_sha256, None);
    assert_eq!(m.entries().count(), 9);
    assert_eq!(fx.entry("y").unwrap().class, Some(ToleranceClass::A));
    assert_eq!(
        fx.entry("beta").unwrap().class,
        Some(ToleranceClass::APrime)
    );
    assert_eq!(fx.entry("x").unwrap().class, None);
    assert_eq!(fx.entry("x").unwrap().dtype, DType::F64le);

    let x = fx.read_f64("x").expect("f64le");
    assert_eq!(x.shape, vec![2, 3]);
    assert_eq!(x.len(), 6);
    assert_eq!(x.data[..3], [1.5, -2.25, 0.0]);
    assert_eq!(
        x.data[3].to_bits(),
        (-0.0f64).to_bits(),
        "sign of zero kept"
    );
    assert_eq!(x.data[4], f64::INFINITY);
    assert_eq!(x.data[5], 1e300);

    let ids = fx.read_i64("ids").expect("i64le");
    assert_eq!(ids.data, vec![i64::MIN, -1, i64::MAX]);
    let small = fx.read_i32("small").expect("i32le");
    assert_eq!(small.data, vec![i32::MIN, 7]);
    let flags = fx.read_bool("flags").expect("u8");
    assert_eq!(flags.data, vec![true, false, false, true]);
    let labels = fx.read_strings("labels").expect("json");
    assert_eq!(labels.data, vec!["QB", "DEF", "WR"]);

    let parts = fx.read_ragged_i64("parts").expect("ragged");
    assert_eq!(parts.len(), 3);
    assert_eq!(parts.row(0), &[10, 10], "duplicates and order kept");
    assert_eq!(parts.row(1), &[] as &[i64]);
    assert_eq!(parts.row(2), &[11, 12, 13]);

    assert_eq!(fx.read_scalar_f64("y"), Ok(42.0));
    assert_eq!(
        fx.read_i64("nope"),
        Err(FixtureError::UnknownArray {
            name: "nope".into()
        })
    );
}

#[test]
fn nan_is_preserved_bit_for_bit() {
    let case = TempCase::new("nan");
    let mut m = base_manifest("value-model", "nan-legacy");
    push_input(&mut m, entry("v", "f64le", &[4], None));
    // NumPy's quiet NaN, a NaN with a payload, a negative NaN and a signalling NaN.
    let bits = [
        0x7ff8_0000_0000_0000u64,
        0x7ff8_0000_dead_beef,
        0xfff8_0000_0000_0001,
        0x7ff0_0000_0000_0001,
    ];
    let values: Vec<f64> = bits.iter().map(|b| f64::from_bits(*b)).collect();
    case.write("v.f64le", &f64le(&values));
    case.write_manifest(&m);
    let fx = FixtureCase::open(case.path()).expect("valid case");
    let v = fx.read_f64("v").expect("read");
    let read_bits: Vec<u64> = v.data.iter().map(|x| x.to_bits()).collect();
    assert_eq!(read_bits, bits);
    assert!(v.data.iter().all(|x| x.is_nan()));
}

#[test]
fn byte_length_dtype_and_shape_mismatches_are_typed() {
    let case = TempCase::new("mismatch");
    let mut m = base_manifest("scoring", "mismatch-legacy");
    push_input(&mut m, entry("short", "f64le", &[3], None));
    case.write("short.f64le", &f64le(&[1.0, 2.0])); // 16 bytes, manifest says 24
    push_input(&mut m, entry("ids", "i64le", &[2], None));
    case.write("ids.i64le", &i64le(&[1, 2]));
    push_input(&mut m, entry("pair", "f64le", &[2], None));
    case.write("pair.f64le", &f64le(&[1.0, 2.0]));
    push_input(&mut m, entry("labels", "json", &[2], None));
    case.write(
        "labels.json",
        &json_strings(&["a".into(), "b".into(), "c".into()]),
    );
    push_input(&mut m, entry("notstrings", "json", &[1], None));
    case.write("notstrings.json", b"[1]\n");
    push_input(&mut m, entry("flags", "u8", &[3], None));
    case.write("flags.u8", &[0, 2, 1]);
    push_input(&mut m, entry("bad.offsets", "i64le", &[3], None));
    case.write("bad.offsets.i64le", &i64le(&[0, 3, 2]));
    push_input(&mut m, entry("bad.values", "i64le", &[3], None));
    case.write("bad.values.i64le", &i64le(&[7, 8, 9]));
    push_input(&mut m, entry("absent", "f64le", &[1], None));
    case.write_manifest(&m);
    let fx = FixtureCase::open(case.path()).expect("manifest itself is valid");

    assert_eq!(
        fx.read_f64("short"),
        Err(FixtureError::LengthMismatch {
            name: "short".into(),
            expected_bytes: 24,
            found_bytes: 16
        })
    );
    assert_eq!(
        fx.read_f64("ids"),
        Err(FixtureError::DtypeMismatch {
            name: "ids".into(),
            expected: "f64le".into(),
            found: "i64le".into()
        })
    );
    assert_eq!(
        fx.read_strings("ids"),
        Err(FixtureError::DtypeMismatch {
            name: "ids".into(),
            expected: "json".into(),
            found: "i64le".into()
        })
    );
    assert_eq!(
        fx.read_scalar_f64("pair"),
        Err(FixtureError::ShapeMismatch {
            name: "pair".into(),
            declared: vec![2],
            expected: vec![1]
        })
    );
    assert_eq!(
        fx.read_f64("pair").unwrap().expect_shape("pair", &[1, 2]),
        Err(FixtureError::ShapeMismatch {
            name: "pair".into(),
            declared: vec![2],
            expected: vec![1, 2]
        })
    );
    assert_eq!(
        fx.read_strings("labels"),
        Err(FixtureError::ShapeMismatch {
            name: "labels".into(),
            declared: vec![2],
            expected: vec![3]
        })
    );
    assert!(matches!(
        fx.read_strings("notstrings"),
        Err(FixtureError::DtypeMismatch { name, .. }) if name == "notstrings"
    ));
    assert!(matches!(
        fx.read_bool("flags"),
        Err(FixtureError::DtypeMismatch { name, found, .. })
            if name == "flags" && found == "byte 2 at index 1"
    ));
    assert!(matches!(
        fx.read_ragged_i64("bad"),
        Err(FixtureError::BadOffsets { name, .. }) if name == "bad"
    ));
    assert!(matches!(
        fx.read_f64("absent"),
        Err(FixtureError::Io {
            kind: ErrorKind::NotFound,
            ..
        })
    ));
}

#[test]
fn missing_required_keys_and_wrong_provenance_are_typed() {
    for key in [
        "manifest_version",
        "component",
        "case",
        "description",
        "contracts",
        "generator",
        "oracle",
        "environment",
        "seeds",
        "provenance",
        "inputs",
        "outputs",
    ] {
        let case = TempCase::new("missing");
        let mut m = base_manifest("scoring", "missing-legacy");
        m.as_object_mut().unwrap().remove(key);
        case.write_manifest(&m);
        assert_eq!(
            FixtureCase::open(case.path()).map(|_| ()),
            Err(FixtureError::MissingKey { key: key.into() }),
            "{key}"
        );
    }
    // An output without its tolerance class.
    let case = TempCase::new("noclass");
    let mut m = base_manifest("scoring", "noclass-legacy");
    push_output(&mut m, entry("y", "f64le", &[1], None));
    case.write_manifest(&m);
    assert_eq!(
        FixtureCase::open(case.path()).map(|_| ()),
        Err(FixtureError::MissingKey {
            key: "outputs[0].class".into()
        })
    );
    // provenance.data other than synthetic (DR-A11).
    let case = TempCase::new("real");
    let mut m = base_manifest("scoring", "real-legacy");
    m["provenance"]["data"] = json!("nflverse");
    case.write_manifest(&m);
    assert_eq!(
        FixtureCase::open(case.path()).map(|_| ()),
        Err(FixtureError::NotSynthetic {
            found: "nflverse".into()
        })
    );
    // An unsupported manifest or contract version.
    let case = TempCase::new("version");
    let mut m = base_manifest("scoring", "version-legacy");
    m["contracts"]["parity_fixture"] = json!("2");
    case.write_manifest(&m);
    assert!(matches!(
        FixtureCase::open(case.path()),
        Err(FixtureError::UnsupportedContract { field, .. }) if field == "contracts.parity_fixture"
    ));
    // A manifest that is not JSON, and a directory with no manifest.
    let case = TempCase::new("notjson");
    case.write("manifest.json", b"{ nope");
    assert!(matches!(
        FixtureCase::open(case.path()),
        Err(FixtureError::Manifest { .. })
    ));
    let case = TempCase::new("empty");
    assert!(matches!(
        FixtureCase::open(case.path()),
        Err(FixtureError::Io {
            kind: ErrorKind::NotFound,
            ..
        })
    ));
}
