//! Shared helpers for the `grid-synth` integration tests: a temporary case directory, the §2.1
//! byte encoders (std only), a manifest builder carrying every required key of
//! parity-fixture-contract §4, and a hand-built toy v0 world.
//!
//! The toy world derives from no third-party data (DR-A11): it is written by hand in the style of
//! the oracle's own unit tests, and reproduces the legacy world's known defects on purpose so the
//! loader is proven to keep them: a duplicated participant (D-8), `yardline_100 = 104` (D-9) and
//! defenders drawn from the offense's roster (D-10, KI-NEW-Y0).

#![allow(dead_code)] // each integration test binary uses the subset of helpers it needs

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use grid_synth::fixture::Ragged;
use grid_synth::legacy_v0::{
    LegacyV0College, LegacyV0Players, LegacyV0Plays, LegacyV0Provenance, LegacyV0Truth,
    LegacyV0World,
};
use serde_json::{json, Map, Value};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// The placeholder every unverified array hash of these hand-built cases carries (the reader
/// carries `sha256` fields and never verifies them: DR-D29 puts hashing in a shell guard).
pub const PLACEHOLDER_SHA256: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// The `SynthConfig` field values the toy case declares under `params.synth_config`
/// (parity-fixture-contract §5.1): the fields the canonical world fixes (`seed`, `synth.py:46`;
/// `load_synthetic()` overrides `yards_noise_sd=3.2` and `drives_per_team_per_game=12`,
/// `data_adapters.py:32`). A declared configuration: the toy world is written by hand, not
/// generated from it.
pub fn toy_synth_config() -> Value {
    json!({ "drives_per_team_per_game": 12, "seed": 7, "yards_noise_sd": 3.2 })
}

/// The §5.1 hash of [`toy_synth_config`]: the sha256 of its canonical JSON serialization
/// `{"drives_per_team_per_game":12,"seed":7,"yards_noise_sd":3.2}` (sorted keys, no whitespace,
/// ASCII), computed once in a shell (`printf '%s' '<that text>' | sha256sum`) and embedded, so the
/// toy case obeys the rule it exercises. The reader carries it without recomputing it (DR-D29);
/// no hashing crate enters the test. The toy world states it as its `config_hash`, because
/// `from_legacy_v0` refuses a world that states none.
pub const TOY_SYNTH_CONFIG_SHA256: &str =
    "5d1041fc882d94997b248bc3fd32b9249f09c1b4665c503d4a2c30effffd9cbf";

/// A unique directory under `std::env::temp_dir()`, removed on drop.
pub struct TempCase {
    dir: PathBuf,
}

impl TempCase {
    pub fn new(label: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "grid-synth-{label}-{}-{nanos}-{n}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("create temp case dir");
        Self { dir }
    }

    pub fn path(&self) -> &Path {
        &self.dir
    }

    pub fn write(&self, file: &str, bytes: &[u8]) {
        fs::write(self.dir.join(file), bytes).expect("write case file");
    }

    pub fn write_manifest(&self, manifest: &Value) {
        let text = serde_json::to_string_pretty(manifest).expect("serialize manifest") + "\n";
        self.write("manifest.json", text.as_bytes());
    }

    /// Reads the written manifest back, applies `edit` and writes it again.
    pub fn edit_manifest(&self, edit: impl FnOnce(&mut Value)) {
        let path = self.dir.join("manifest.json");
        let mut manifest: Value =
            serde_json::from_str(&fs::read_to_string(path).expect("read manifest"))
                .expect("manifest is JSON");
        edit(&mut manifest);
        self.write_manifest(&manifest);
    }
}

impl Drop for TempCase {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

// --- §2.1 encoders, std only -----------------------------------------------------------------

pub fn f64le(values: &[f64]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

pub fn i64le(values: &[i64]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

pub fn i32le(values: &[i32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

pub fn u8bool(values: &[bool]) -> Vec<u8> {
    values.iter().map(|&b| u8::from(b)).collect()
}

/// A JSON array of strings: UTF-8, LF, trailing newline (the strings here are ASCII).
pub fn json_strings(values: &[String]) -> Vec<u8> {
    let mut text = serde_json::to_string(values).expect("serialize strings");
    text.push('\n');
    text.into_bytes()
}

// --- manifest builder --------------------------------------------------------------------------

/// A manifest with every required key of parity-fixture-contract §4 and empty `inputs` and
/// `outputs`; the environment block mirrors the contract's example.
pub fn base_manifest(component: &str, case: &str) -> Value {
    json!({
        "case": case,
        "component": component,
        "contracts": { "parity_fixture": "1", "plays": "grid.plays/0" },
        "description": format!("hand-built toy case `{case}` for the grid-synth tests (no third-party data)"),
        "environment": {
            "blas": { "architecture": "Haswell", "internal_api": "openblas",
                      "openblas_coretype": "Haswell", "version": "0.3.30" },
            "packages": { "numpy": "2.4.6", "pandas": "3.0.6", "scipy": "1.17.1" },
            "platform": "linux-x86_64",
            "python": "3.11.15",
            "python_implementation": "CPython",
            "threads": { "MKL_NUM_THREADS": "1", "OMP_NUM_THREADS": "1",
                         "OPENBLAS_NUM_THREADS": "1", "threadpool_limits": 1 }
        },
        "generator": { "entry": "hand-built", "script": "crates/synth/tests/common/mod.rs",
                       "script_sha256": "0000000000000000000000000000000000000000000000000000000000000000" },
        "inputs": [],
        "inputs_from": [],
        "manifest_version": 1,
        "oracle": { "corrections_applied": [], "import_patches": ["P1", "P2"],
                    "ledger_independent": true, "level": "legacy",
                    "manifest_tsv_sha256": "0000000000000000000000000000000000000000000000000000000000000000",
                    "source": "cautious-nevermore", "source_commit": "59bce1d" },
        "outputs": [],
        "params": {},
        "provenance": { "data": "synthetic", "profile": "synthetic-legacy", "synth_config": "CANONICAL_SYNTH" },
        "seeds": { "college": 106, "synth": 7 }
    })
}

/// One array entry; `class` is set on outputs.
pub fn entry(name: &str, dtype: &str, shape: &[usize], class: Option<&str>) -> Value {
    let mut m = Map::new();
    m.insert("dtype".into(), json!(dtype));
    m.insert("file".into(), json!(format!("{name}.{dtype}")));
    m.insert("name".into(), json!(name));
    m.insert(
        "sha256".into(),
        json!("0000000000000000000000000000000000000000000000000000000000000000"),
    );
    m.insert("shape".into(), json!(shape));
    if let Some(c) = class {
        m.insert("class".into(), json!(c));
    }
    Value::Object(m)
}

pub fn push_input(manifest: &mut Value, e: Value) {
    manifest["inputs"].as_array_mut().unwrap().push(e);
}

pub fn push_output(manifest: &mut Value, e: Value) {
    manifest["outputs"].as_array_mut().unwrap().push(e);
}

// --- the toy v0 world ----------------------------------------------------------------------------

/// Two teams (0 and 1) of 12 players each: QB, RB, WR, WR, TE and seven DEF.
/// Three drives over two weeks. Reproduces the legacy defects on purpose (see the module doc).
pub fn toy_world() -> LegacyV0World {
    let nan = f64::NAN;
    let plays = LegacyV0Plays {
        play_id: vec![0, 1, 2, 3, 4, 5],
        drive_id: vec![1, 1, 1, 2, 2, 3],
        week: vec![1, 1, 1, 1, 1, 2],
        off_team: vec![0, 0, 0, 1, 1, 0],
        def_team: vec![1, 1, 1, 0, 0, 1],
        down: vec![1, 1, 2, 1, 2, 1],
        ydstogo: vec![10, 10, 7, 10, 18, 10],
        yardline_100: vec![75, 63, 60, 96, 104, 5],
        // Row 2 gains nothing. The oracle writes a zero gain as `0.0`, never `-0.0`: its
        // `float(np.clip(round(yards), -8, 60))` (synth.py:215) rounds through a Python `int`
        // under NumPy 2.x. `from_legacy_v0` refuses a `-0.0` (tested in legacy_v0_round_trip.rs).
        yards: vec![12.0, 3.0, 0.0, -8.0, 2.0, 5.0],
        points: vec![0.0, 0.0, 3.0, 0.0, 0.0, 7.0],
        terminal: vec![false, false, true, false, true, true],
        terminal_value: vec![nan, nan, 3.0, nan, 0.0, 7.0],
        n_down: vec![1, 2, -1, 2, -1, -1],
        n_ydstogo: vec![10, 7, -1, 18, -1, -1],
        n_yardline_100: vec![63, 60, -1, 104, -1, -1],
        drive_points: vec![3.0, 3.0, 3.0, 0.0, 0.0, 7.0],
        // Offense of team 0 is 0..=4, of team 1 is 12..=16. Play 0 lists WR 2 twice (D-8).
        off_players: Ragged::from_rows(vec![
            vec![0, 1, 2, 2, 4],
            vec![0, 1, 2, 3, 4],
            vec![0, 1, 3, 2, 4],
            vec![12, 13, 14, 15, 16],
            vec![12, 13, 15, 14, 16],
            vec![0, 1, 2, 3, 4],
        ]),
        // Defenders come from the *offense's* roster, as the legacy generator draws them
        // (KI-NEW-Y0): team 0's DEF are 5..=11, team 1's are 17..=23.
        def_players: Ragged::from_rows(vec![
            vec![5, 6, 7, 8, 9, 10, 11],
            vec![11, 10, 9, 8, 7, 6, 5],
            vec![5, 7, 9, 11, 6, 8, 10],
            vec![17, 18, 19, 20, 21, 22, 23],
            vec![23, 22, 21, 20, 19, 18, 17],
            vec![5, 6, 7, 8, 9, 10, 11],
        ]),
    };
    let mut players = LegacyV0Players {
        player_id: Vec::new(),
        team: Vec::new(),
        position: Vec::new(),
        is_starter: Vec::new(),
        ability: Vec::new(),
    };
    for team in 0..2i64 {
        let base = team * 12;
        let positions = [
            "QB", "RB", "WR", "WR", "TE", "DEF", "DEF", "DEF", "DEF", "DEF", "DEF", "DEF",
        ];
        for (k, position) in positions.iter().enumerate() {
            let id = base + k as i64;
            players.player_id.push(id);
            players.team.push(team);
            players.position.push((*position).to_owned());
            players.is_starter.push(id != 5 && id != 17);
            players.ability.push(id as f64 * 0.01 - 0.1);
        }
    }
    let truth = LegacyV0Truth {
        team_strength_team: vec![0, 1],
        team_strength: vec![0.05, 0.10],
        focus_qb: 0,
        focus_tau: vec![0.075, nan, 0.1],
        league_factor: 0.62,
    };
    let college = LegacyV0College {
        player_id: vec![0, 12, 2],
        position: vec!["QB".into(), "QB".into(), "WR".into()],
        feeder_sv: vec![0.1, -0.05, 0.02],
        feeder_snaps: vec![300, 450, 899],
        is_rookie: vec![false, true, false],
    };
    LegacyV0World {
        plays,
        players,
        truth,
        college,
        provenance: LegacyV0Provenance {
            config_hash: Some(TOY_SYNTH_CONFIG_SHA256.into()),
            seeds: vec![("college".into(), 106), ("synth".into(), 7)],
        },
    }
}

/// Writes `world` as a `synthetic-world` case into `case`, with the array names
/// `legacy_v0::load_world` fixes, every array under `outputs` with class `exact`
/// (parity-fixture-contract §5, first row), every required manifest key, the declared
/// configuration under `params.synth_config` and, when the world states a hash, that hash under
/// `provenance.synth_config_sha256` (§5.1); a world that states none writes no such key.
pub fn write_world_case(case: &TempCase, world: &LegacyV0World) {
    let mut m = base_manifest("synthetic-world", "toy-legacy");
    let mut out = |name: &str, dtype: &str, shape: Vec<usize>, bytes: Vec<u8>| {
        push_output(&mut m, entry(name, dtype, &shape, Some("exact")));
        case.write(&format!("{name}.{dtype}"), &bytes);
    };
    let p = &world.plays;
    let n = p.play_id.len();
    for (name, col) in [
        ("plays.play_id", &p.play_id),
        ("plays.drive_id", &p.drive_id),
        ("plays.week", &p.week),
        ("plays.off_team", &p.off_team),
        ("plays.def_team", &p.def_team),
        ("plays.down", &p.down),
        ("plays.ydstogo", &p.ydstogo),
        ("plays.yardline_100", &p.yardline_100),
        ("plays.n_down", &p.n_down),
        ("plays.n_ydstogo", &p.n_ydstogo),
        ("plays.n_yardline_100", &p.n_yardline_100),
    ] {
        out(name, "i64le", vec![n], i64le(col));
    }
    for (name, col) in [
        ("plays.yards", &p.yards),
        ("plays.points", &p.points),
        ("plays.terminal_value", &p.terminal_value),
        ("plays.drive_points", &p.drive_points),
    ] {
        out(name, "f64le", vec![n], f64le(col));
    }
    out("plays.terminal", "u8", vec![n], u8bool(&p.terminal));
    for (name, ragged) in [
        ("plays.off_players", &p.off_players),
        ("plays.def_players", &p.def_players),
    ] {
        out(
            &format!("{name}.offsets"),
            "i64le",
            vec![ragged.offsets.len()],
            i64le(&ragged.offsets),
        );
        out(
            &format!("{name}.values"),
            "i64le",
            vec![ragged.values.len()],
            i64le(&ragged.values),
        );
    }
    let q = &world.players;
    let np = q.player_id.len();
    out("players.player_id", "i64le", vec![np], i64le(&q.player_id));
    out("players.team", "i64le", vec![np], i64le(&q.team));
    out(
        "players.position",
        "json",
        vec![np],
        json_strings(&q.position),
    );
    out("players.is_starter", "u8", vec![np], u8bool(&q.is_starter));
    out("players.ability", "f64le", vec![np], f64le(&q.ability));
    let t = &world.truth;
    out(
        "gt.team_strength.team",
        "i64le",
        vec![t.team_strength_team.len()],
        i64le(&t.team_strength_team),
    );
    out(
        "gt.team_strength",
        "f64le",
        vec![t.team_strength.len()],
        f64le(&t.team_strength),
    );
    out("gt.focus_qb", "i64le", vec![1], i64le(&[t.focus_qb]));
    out(
        "gt.focus_tau",
        "f64le",
        vec![t.focus_tau.len()],
        f64le(&t.focus_tau),
    );
    out(
        "gt.league_factor",
        "f64le",
        vec![1],
        f64le(&[t.league_factor]),
    );
    let c = &world.college;
    let nc = c.player_id.len();
    out("college.player_id", "i64le", vec![nc], i64le(&c.player_id));
    out(
        "college.position",
        "json",
        vec![nc],
        json_strings(&c.position),
    );
    out("college.feeder_sv", "f64le", vec![nc], f64le(&c.feeder_sv));
    out(
        "college.feeder_snaps",
        "i64le",
        vec![nc],
        i64le(&c.feeder_snaps),
    );
    out("college.is_rookie", "u8", vec![nc], u8bool(&c.is_rookie));
    m["params"]["synth_config"] = toy_synth_config();
    if let Some(hash) = &world.provenance.config_hash {
        m["provenance"]["synth_config_sha256"] = json!(hash);
    }
    case.write_manifest(&m);
}
