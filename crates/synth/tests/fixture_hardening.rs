//! Security hardening of the DR-D29 reader (parity-fixture-contract §2.1, §3, §8, §11): every
//! read stays inside the case directory, sizes are checked with checked arithmetic and before
//! any byte is read, and no `FixtureCase` carries a non-synthetic manifest (DR-A11).
//!
//! Each test here failed against the reader before the hardening: an overflowing shape parsed
//! (and wrapped to an empty array in release builds), a drive-prefixed or space-bearing file name
//! parsed, a symbolic link was followed out of the case directory, an oversized file was read in
//! full before its size was checked, and `with_manifest` accepted `provenance.data = nflverse`.

mod common;

use std::fs;

use common::{base_manifest, entry, f64le, json_strings, push_input, push_output, TempCase};
use grid_synth::fixture::{
    FixtureCase, FixtureError, Manifest, MAX_MANIFEST_BYTES, MAX_STRING_ARRAY_BYTES,
};
use serde_json::json;

#[test]
fn an_overflowing_shape_is_refused_when_the_manifest_is_parsed() {
    let case = TempCase::new("overflow");
    let mut m = base_manifest("scoring", "overflow-legacy");
    // 2^32 × 2^32 wraps to 0 in a release build's unchecked product.
    push_input(
        &mut m,
        entry("y", "f64le", &[4_294_967_296, 4_294_967_296], None),
    );
    case.write("y.f64le", b"");
    case.write_manifest(&m);
    assert!(matches!(
        FixtureCase::open(case.path()),
        Err(FixtureError::Manifest { detail })
            if detail.contains("array `y`") && detail.contains("overflows")
    ));
}

#[test]
fn file_names_that_could_leave_the_case_directory_are_refused() {
    for file in [
        "C:x.f64le",
        "a/b.f64le",
        "..",
        "x y.f64le",
        "a\\b.f64le",
        "/x.f64le",
    ] {
        let case = TempCase::new("names");
        let mut m = base_manifest("scoring", "names-legacy");
        let mut e = entry("x", "f64le", &[1], None);
        e["file"] = json!(file);
        push_input(&mut m, e);
        case.write_manifest(&m);
        assert!(
            matches!(
                FixtureCase::open(case.path()),
                Err(FixtureError::Manifest { ref detail }) if detail.contains("plain file name")
            ),
            "{file}"
        );
    }
}

#[cfg(unix)]
#[test]
fn symbolic_links_are_never_followed() {
    use std::os::unix::fs::symlink;

    let outside = TempCase::new("outside");
    outside.write("secret.f64le", &f64le(&[1.0]));
    outside.write("secret.json", &json_strings(&["QB".into()]));
    let mut m = base_manifest("scoring", "link-legacy");
    push_input(&mut m, entry("y", "f64le", &[1], None));
    push_input(&mut m, entry("s", "json", &[1], None));
    outside.write_manifest(&m);

    let case = TempCase::new("link");
    case.write_manifest(&m);
    symlink(
        outside.path().join("secret.f64le"),
        case.path().join("y.f64le"),
    )
    .expect("create symlink");
    symlink(
        outside.path().join("secret.json"),
        case.path().join("s.json"),
    )
    .expect("create symlink");
    let fx = FixtureCase::open(case.path()).expect("the manifest itself is a regular file");
    let link = case.path().join("y.f64le");
    assert_eq!(
        fx.read_f64("y"),
        Err(FixtureError::NotRegularFile {
            path: link,
            found: "a symbolic link".into()
        })
    );
    assert!(matches!(
        fx.read_strings("s"),
        Err(FixtureError::NotRegularFile { found, .. }) if found == "a symbolic link"
    ));

    // A linked manifest is refused too.
    let linked = TempCase::new("linked-manifest");
    symlink(
        outside.path().join("manifest.json"),
        linked.path().join("manifest.json"),
    )
    .expect("create symlink");
    assert!(matches!(
        FixtureCase::open(linked.path()),
        Err(FixtureError::NotRegularFile { found, .. }) if found == "a symbolic link"
    ));
}

#[test]
fn a_directory_in_place_of_an_array_file_is_refused() {
    let case = TempCase::new("dir");
    let mut m = base_manifest("scoring", "dir-legacy");
    push_input(&mut m, entry("y", "f64le", &[1], None));
    case.write_manifest(&m);
    fs::create_dir(case.path().join("y.f64le")).expect("create dir");
    let fx = FixtureCase::open(case.path()).expect("valid manifest");
    assert!(matches!(
        fx.read_f64("y"),
        Err(FixtureError::NotRegularFile { found, .. }) if found == "a directory"
    ));
}

#[test]
fn a_binary_file_longer_than_declared_is_refused_by_its_length() {
    let case = TempCase::new("long");
    let mut m = base_manifest("scoring", "long-legacy");
    push_output(&mut m, entry("y", "f64le", &[2], Some("A")));
    case.write("y.f64le", &f64le(&[1.0, 2.0, 3.0]));
    case.write_manifest(&m);
    let fx = FixtureCase::open(case.path()).expect("valid manifest");
    assert_eq!(
        fx.read_f64("y"),
        Err(FixtureError::LengthMismatch {
            name: "y".into(),
            expected_bytes: 16,
            found_bytes: 24
        })
    );
}

/// A sparse file of 1 TiB: refused from its metadata length alone. The reader before the
/// hardening tried to read it in full (`fs::read` failed with `OutOfMemory`).
#[cfg(target_os = "linux")]
#[test]
fn a_huge_binary_file_is_refused_without_reading_it() {
    let case = TempCase::new("huge");
    let mut m = base_manifest("scoring", "huge-legacy");
    push_input(&mut m, entry("y", "f64le", &[1], None));
    case.write_manifest(&m);
    let file = fs::File::create(case.path().join("y.f64le")).expect("create");
    file.set_len(1 << 40).expect("sparse 1 TiB");
    drop(file);
    let fx = FixtureCase::open(case.path()).expect("valid manifest");
    assert_eq!(
        fx.read_f64("y"),
        Err(FixtureError::LengthMismatch {
            name: "y".into(),
            expected_bytes: 8,
            found_bytes: 1 << 40
        })
    );
}

#[test]
fn text_files_above_the_cap_are_refused() {
    // A string array one byte above its cap (the tail is a sparse run of zero bytes).
    let case = TempCase::new("bigstrings");
    let mut m = base_manifest("scoring", "bigstrings-legacy");
    push_input(&mut m, entry("s", "json", &[1], None));
    case.write_manifest(&m);
    case.write("s.json", &json_strings(&["QB".into()]));
    let path = case.path().join("s.json");
    fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .and_then(|f| f.set_len(MAX_STRING_ARRAY_BYTES + 1))
        .expect("extend");
    let fx = FixtureCase::open(case.path()).expect("valid manifest");
    assert_eq!(
        fx.read_strings("s"),
        Err(FixtureError::FileTooLarge {
            path,
            limit_bytes: MAX_STRING_ARRAY_BYTES,
            found_bytes: MAX_STRING_ARRAY_BYTES + 1
        })
    );

    // A manifest one byte above its cap.
    let case = TempCase::new("bigmanifest");
    case.write_manifest(&base_manifest("scoring", "bigmanifest-legacy"));
    let path = case.path().join("manifest.json");
    fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .and_then(|f| f.set_len(MAX_MANIFEST_BYTES + 1))
        .expect("extend");
    assert_eq!(
        FixtureCase::open(case.path()),
        Err(FixtureError::FileTooLarge {
            path,
            limit_bytes: MAX_MANIFEST_BYTES,
            found_bytes: MAX_MANIFEST_BYTES + 1
        })
    );
}

#[test]
fn with_manifest_refuses_a_manifest_that_fails_validation() {
    let case = TempCase::new("with-manifest");
    let mut m = base_manifest("scoring", "with-manifest-legacy");
    push_input(&mut m, entry("y", "f64le", &[1], None));
    case.write("y.f64le", &f64le(&[4.5]));
    let parsed = Manifest::from_value(&m).expect("valid");

    let fx = FixtureCase::with_manifest(case.path(), parsed.clone()).expect("valid manifest");
    assert_eq!(fx.read_scalar_f64("y"), Ok(4.5));

    let mut real = parsed.clone();
    real.provenance.data = "nflverse".into();
    assert_eq!(
        FixtureCase::with_manifest(case.path(), real),
        Err(FixtureError::NotSynthetic {
            found: "nflverse".into()
        })
    );

    let mut escaping = parsed.clone();
    escaping.inputs[0].file = "../y.f64le".into();
    assert!(matches!(
        FixtureCase::with_manifest(case.path(), escaping),
        Err(FixtureError::Manifest { detail }) if detail.contains("plain file name")
    ));

    let mut wrapping = parsed;
    wrapping.inputs[0].shape = vec![usize::MAX, 2];
    assert!(matches!(
        FixtureCase::with_manifest(case.path(), wrapping),
        Err(FixtureError::Manifest { detail }) if detail.contains("overflows")
    ));
}
