//! The DR-D29 parity-fixture reader and the tolerance helpers
//! (`docs/03-contracts/parity-fixture-contract.md` §2 format, §2.1 encodings, §3 layout, §4
//! manifest, §6 tolerance classes, §11 Rust harness, §12 versioning).
//!
//! A case is a directory holding `manifest.json` and one file per array. The manifest is read
//! with `serde_json` (the first use of an already-declared workspace dependency); every array is
//! decoded with `std` alone (`f64::from_le_bytes` and friends), so NaN payloads survive bit for
//! bit. `sha256` fields are carried but **not verified here**: DR-D29 puts hash verification in the
//! shell guard of parity-fixture-contract §10, and an in-Rust check would need a hashing crate.
//!
//! Every failure is a typed [`FixtureError`]. Nothing is coerced: a short file, an undeclared
//! dtype, a shape that does not fit or a manifest with `provenance.data != "synthetic"` (DR-A11)
//! is rejected, never repaired.
//!
//! The tolerance helpers encode the pre-registered classes of §6 that have an element-wise
//! criterion: exact, A, A′ and B. Classes C-V, C-L1 and D are stage-specific and live with the
//! stages that need them (`value-model.md` §10.3, `layer1-credit.md` §10.3, `evaluation::recovery`).

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde_json::{Map, Value};

/// The parity-fixture contract version this reader implements (`contracts.parity_fixture`,
/// parity-fixture-contract §12).
pub const PARITY_FIXTURE_CONTRACT_VERSION: &str = "1";

/// The manifest schema version this reader implements (`manifest_version`, §12).
pub const MANIFEST_VERSION: u64 = 1;

/// The manifest file name inside a case directory (§3).
pub const MANIFEST_FILE: &str = "manifest.json";

/// Class A: `max_i |r_i − p_i| ≤ 1e-12` (§6, DR-B3).
pub const CLASS_A_ABS_TOL: f64 = 1e-12;

/// Class A′: `‖r − p‖∞ / ‖p‖∞ ≤ 1e-9` (§6, DR-B3).
pub const CLASS_A_PRIME_REL_TOL: f64 = 1e-9;

/// Class B, third part: the CG solution agrees with the dense solve to `1e-9` relative
/// (§6; `rapm-attribution.md` §10.3).
pub const CLASS_B_DENSE_REL_TOL: f64 = 1e-9;

/// A typed failure of the fixture reader. **Closed** enum; every variant names what failed.
///
/// `Io` carries the `std::io::ErrorKind` and message rather than the `io::Error` itself so that the
/// error stays `Clone + PartialEq` for tests; the path is the file that could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FixtureError {
    /// A file could not be read.
    Io {
        /// The file.
        path: PathBuf,
        /// The I/O error class.
        kind: io::ErrorKind,
        /// The I/O error text.
        message: String,
    },
    /// The manifest is not valid JSON, or a value has the wrong JSON type or an undeclared
    /// vocabulary entry (an unknown dtype, class or oracle level, a file name with a path
    /// separator, a dtype suffix that does not match the file name).
    Manifest {
        /// What is wrong, naming the key.
        detail: String,
    },
    /// A required manifest key is absent (§4 "Required keys"; `class` on every output).
    MissingKey {
        /// The key path, for example `provenance.data` or `outputs[2].class`.
        key: String,
    },
    /// No input or output of the manifest has this name.
    UnknownArray {
        /// The requested array name.
        name: String,
    },
    /// The array's declared dtype is not the one requested, or its bytes do not decode as the
    /// declared dtype (a byte other than 0 or 1 in a `u8` boolean array; a `.json` file that is
    /// not an array of strings).
    DtypeMismatch {
        /// The array name.
        name: String,
        /// What was requested or required.
        expected: String,
        /// What the manifest declares or the file holds.
        found: String,
    },
    /// The file's byte length is not `product(shape) × itemsize` (§11).
    LengthMismatch {
        /// The array name.
        name: String,
        /// `product(shape) × itemsize` from the manifest.
        expected_bytes: u64,
        /// The file's actual length.
        found_bytes: u64,
    },
    /// The declared shape is not the one the caller requires, or a `.json` string array does not
    /// hold `product(shape)` elements.
    ShapeMismatch {
        /// The array name.
        name: String,
        /// The shape the manifest declares.
        declared: Vec<usize>,
        /// The shape required (`[1]` for a scalar; `[n]` for a string array of `n` elements).
        expected: Vec<usize>,
    },
    /// A ragged `offsets` array is not a non-decreasing sequence starting at 0 and ending at the
    /// length of `values` (§2.1).
    BadOffsets {
        /// The ragged array name.
        name: String,
        /// What is wrong.
        detail: String,
    },
    /// `provenance.data` is not `synthetic` (§8; DR-A11).
    NotSynthetic {
        /// The value found.
        found: String,
    },
    /// `manifest_version` or `contracts.parity_fixture` is not the version this reader implements
    /// (§12). The parser is never "made flexible".
    UnsupportedContract {
        /// `manifest_version` or `contracts.parity_fixture`.
        field: String,
        /// The value found.
        found: String,
        /// The value this reader implements.
        supported: String,
    },
}

impl fmt::Display for FixtureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FixtureError::Io {
                path,
                kind,
                message,
            } => write!(f, "cannot read `{}` ({kind:?}): {message}", path.display()),
            FixtureError::Manifest { detail } => write!(f, "invalid manifest: {detail}"),
            FixtureError::MissingKey { key } => write!(f, "manifest key `{key}` is missing"),
            FixtureError::UnknownArray { name } => {
                write!(f, "the manifest lists no array named `{name}`")
            }
            FixtureError::DtypeMismatch {
                name,
                expected,
                found,
            } => write!(
                f,
                "array `{name}`: expected dtype {expected}, found {found}"
            ),
            FixtureError::LengthMismatch {
                name,
                expected_bytes,
                found_bytes,
            } => write!(
                f,
                "array `{name}`: the manifest implies {expected_bytes} bytes, the file holds \
                 {found_bytes}"
            ),
            FixtureError::ShapeMismatch {
                name,
                declared,
                expected,
            } => write!(
                f,
                "array `{name}`: declared shape {declared:?}, expected {expected:?}"
            ),
            FixtureError::BadOffsets { name, detail } => {
                write!(f, "ragged array `{name}`: bad offsets: {detail}")
            }
            FixtureError::NotSynthetic { found } => write!(
                f,
                "provenance.data is `{found}`; parity fixtures must be synthetic (DR-A11)"
            ),
            FixtureError::UnsupportedContract {
                field,
                found,
                supported,
            } => write!(
                f,
                "`{field}` is `{found}`; this reader implements `{supported}`"
            ),
        }
    }
}

impl std::error::Error for FixtureError {}

/// An array element type of the fixture format (§2.1). **Closed** enum: the five encodings the
/// contract defines. Parsed from and displayed as the manifest spelling (`f64le`, `i64le`,
/// `i32le`, `u8`, `json`), which is also the file-name suffix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DType {
    /// IEEE-754 binary64, little-endian, C order, no header. NaN kept bit for bit.
    F64le,
    /// 64-bit two's complement, little-endian.
    I64le,
    /// 32-bit two's complement, little-endian; only when the manifest declares it.
    I32le,
    /// One byte per element, 0 or 1: a boolean array (or a validity mask).
    U8,
    /// A JSON array of strings: UTF-8, ASCII-escaped, LF, trailing newline.
    Json,
}

impl DType {
    /// Every dtype, in declaration order.
    pub const ALL: [DType; 5] = [
        DType::F64le,
        DType::I64le,
        DType::I32le,
        DType::U8,
        DType::Json,
    ];

    /// The manifest spelling, equal to the file-name suffix.
    pub const fn as_str(self) -> &'static str {
        match self {
            DType::F64le => "f64le",
            DType::I64le => "i64le",
            DType::I32le => "i32le",
            DType::U8 => "u8",
            DType::Json => "json",
        }
    }

    /// Bytes per element for the binary encodings; `None` for `json`, whose byte length is not a
    /// function of the shape.
    pub const fn itemsize(self) -> Option<usize> {
        match self {
            DType::F64le | DType::I64le => Some(8),
            DType::I32le => Some(4),
            DType::U8 => Some(1),
            DType::Json => None,
        }
    }
}

impl FromStr for DType {
    type Err = FixtureError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        DType::ALL
            .iter()
            .copied()
            .find(|d| d.as_str() == s)
            .ok_or_else(|| FixtureError::Manifest {
                detail: format!("dtype `{s}` is not one of f64le, i64le, i32le, u8, json"),
            })
    }
}

impl fmt::Display for DType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A tolerance class of parity-fixture-contract §6 (DR-B3), as every output declares in `class`.
/// **Closed** enum. Spellings: `exact`, `A`, `A'` (also accepted as `A′`, U+2032), `B`, `C-V`,
/// `C-L1`, `D`. Only `Exact`, `A`, `APrime` and `B` have helpers in this module; `CV`, `CL1` and
/// `D` are stage-specific criteria.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToleranceClass {
    /// Bit-exact or value-exact equality: integers, booleans, ids, categoricals, sentinels.
    Exact,
    /// Element-wise `max |r − p| ≤ 1e-12`.
    A,
    /// `‖r − p‖∞ / ‖p‖∞ ≤ 1e-9` (dense linear solves).
    APrime,
    /// The three-part iterative-CG criterion (converged, residual ≤ `tol_cg`, dense agreement).
    B,
    /// The V(s) booster criterion (DR-D27; `value-model.md` §10.3).
    CV,
    /// The Layer-1 context-model criterion (`layer1-credit.md` §10.3).
    CL1,
    /// End-to-end recovery over a seed ensemble (DR-D26; floors in engine-spec §7.13).
    D,
}

impl ToleranceClass {
    /// Every class, in declaration order.
    pub const ALL: [ToleranceClass; 7] = [
        ToleranceClass::Exact,
        ToleranceClass::A,
        ToleranceClass::APrime,
        ToleranceClass::B,
        ToleranceClass::CV,
        ToleranceClass::CL1,
        ToleranceClass::D,
    ];

    /// The canonical manifest spelling (ASCII; `A'` for A′).
    pub const fn as_str(self) -> &'static str {
        match self {
            ToleranceClass::Exact => "exact",
            ToleranceClass::A => "A",
            ToleranceClass::APrime => "A'",
            ToleranceClass::B => "B",
            ToleranceClass::CV => "C-V",
            ToleranceClass::CL1 => "C-L1",
            ToleranceClass::D => "D",
        }
    }

    /// Whether this module has a helper for the class.
    pub const fn has_helper(self) -> bool {
        matches!(
            self,
            ToleranceClass::Exact | ToleranceClass::A | ToleranceClass::APrime | ToleranceClass::B
        )
    }
}

impl FromStr for ToleranceClass {
    type Err = FixtureError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "A\u{2032}" {
            return Ok(ToleranceClass::APrime);
        }
        ToleranceClass::ALL
            .iter()
            .copied()
            .find(|c| c.as_str() == s)
            .ok_or_else(|| FixtureError::Manifest {
                detail: format!(
                    "tolerance class `{s}` is not one of exact, A, A', B, C-V, C-L1, D"
                ),
            })
    }
}

impl fmt::Display for ToleranceClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One array of a case: an entry of the manifest's `inputs` or `outputs` (§4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrayEntry {
    /// The array name consumers read it by (`y`, `plays.play_id`, `plays.off_players.offsets`).
    pub name: String,
    /// The file name inside the case directory: a plain name, no path separator, carrying the
    /// dtype as its suffix (`y.f64le`).
    pub file: String,
    /// The element encoding.
    pub dtype: DType,
    /// The C-order shape; `[]` or `[1]` for a scalar. The binary file holds `product(shape)`
    /// elements.
    pub shape: Vec<usize>,
    /// The file's sha256, lowercase hex. Carried, not verified here (DR-D29: the shell guard of
    /// §10 verifies it).
    pub sha256: String,
    /// The tolerance class. Required on every output (§4); `None` on an input.
    pub class: Option<ToleranceClass>,
}

impl ArrayEntry {
    /// `product(shape)`: the element count (1 for shape `[]`).
    pub fn element_count(&self) -> usize {
        self.shape.iter().product()
    }
}

/// The oracle correction level a case was exported at (§7; DR-B1). **Closed** enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OracleLevel {
    /// The as-imported oracle (`59bce1d` plus import patches P1, P2 and ledger entry L0).
    Legacy,
    /// The oracle after the statistical-owner-approved correction-ledger entries listed in
    /// `corrections_applied`.
    Corrected,
}

impl OracleLevel {
    /// The manifest spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            OracleLevel::Legacy => "legacy",
            OracleLevel::Corrected => "corrected",
        }
    }
}

impl FromStr for OracleLevel {
    type Err = FixtureError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "legacy" => Ok(OracleLevel::Legacy),
            "corrected" => Ok(OracleLevel::Corrected),
            other => Err(FixtureError::Manifest {
                detail: format!("oracle.level `{other}` is not `legacy` or `corrected`"),
            }),
        }
    }
}

impl fmt::Display for OracleLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The manifest's `oracle` block (§4, §7): which oracle the case represents.
///
/// `level` and `corrections_applied` are required. Every other field is `Option`: `None` means
/// the exporter did not write the key (or wrote `null`), and is never replaced by a value, so a
/// consumer can tell "not stated" from "stated" (KI-P1 is never reproduced).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OracleInfo {
    /// `legacy` or `corrected`.
    pub level: OracleLevel,
    /// The correction-ledger entries applied (empty at the legacy level).
    pub corrections_applied: Vec<String>,
    /// Whether no ledger entry changes the stage's semantics for the injected inputs (§7 rule 4).
    /// `None` when the manifest does not say. §7 rule 4 admits a legacy case as parity evidence
    /// for a primitive only on `Some(true)`; a consumer treats `None` as not independent.
    pub ledger_independent: Option<bool>,
    /// The oracle source repository name; `None` when not recorded.
    pub source: Option<String>,
    /// The oracle source commit; `None` when not recorded.
    pub source_commit: Option<String>,
    /// The import patches applied; `None` when not recorded (an exporter that applied none
    /// writes an empty list, which is `Some(vec![])`).
    pub import_patches: Option<Vec<String>>,
    /// The sha256 of `reference/python/MANIFEST.tsv`; `None` when not recorded.
    pub manifest_tsv_sha256: Option<String>,
}

/// The manifest's `provenance` block (§4, §8). `data` has already been checked to be `synthetic`
/// when a [`Manifest`] exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    /// Always `synthetic` on a parsed manifest (DR-A11).
    pub data: String,
    /// The validation profile of the world the case derives from (`synthetic-legacy`, …), when
    /// recorded.
    pub profile: Option<String>,
    /// The name of the generator configuration (`CANONICAL_SYNTH`), when recorded. A label,
    /// distinct from [`Provenance::synth_config_sha256`].
    pub synth_config: Option<String>,
    /// The content hash of the generator configuration (§5.1): the sha256 of the canonical JSON
    /// serialization of `params.synth_config`, as 64 lowercase hexadecimal characters. Required
    /// on a `synthetic-world` case and absent (`None`) elsewhere. Carried as the manifest states
    /// it: this reader neither verifies nor recomputes it (DR-D29 puts hash verification in the
    /// shell guard of §10); `legacy_v0::load_world` checks its form before carrying it into the
    /// producer's `config_hash`.
    pub synth_config_sha256: Option<String>,
}

/// A parsed, validated `manifest.json` (§4).
///
/// Blocks whose shape the contract leaves to the exporter (`generator`, `environment`,
/// `inputs_from`, `params`) are carried as raw JSON values; the rest is typed. Construction fails
/// with a typed [`FixtureError`] when a required key is missing, a value has the wrong type, the
/// contract or manifest version is not the one implemented, or `provenance.data` is not
/// `synthetic`.
#[derive(Debug, Clone, PartialEq)]
pub struct Manifest {
    /// The manifest schema version; always [`MANIFEST_VERSION`] on a parsed manifest.
    pub manifest_version: u64,
    /// The owning model-spec name (`synthetic-world`, `rapm-attribution`, …; §3).
    pub component: String,
    /// The kebab-case case name ending in `-legacy` or `-corrected` (§3).
    pub case: String,
    /// A one-paragraph description; carries the §7 rule 4 justification for a legacy primitive.
    pub description: String,
    /// `contracts`: at least `parity_fixture` (always `"1"` here); `plays` when the case carries
    /// plays-contract frames.
    pub contracts: BTreeMap<String, String>,
    /// `generator`: the exporter script, entry point and script hash, as written.
    pub generator: Value,
    /// `oracle`: the oracle level and corrections.
    pub oracle: OracleInfo,
    /// `environment`: interpreter, packages, BLAS kernel and thread pins, as written.
    pub environment: Value,
    /// `seeds`: every named seed, sorted by name.
    pub seeds: BTreeMap<String, u64>,
    /// `provenance`: synthetic-only provenance.
    pub provenance: Provenance,
    /// The input arrays.
    pub inputs: Vec<ArrayEntry>,
    /// The output arrays; each carries its tolerance class.
    pub outputs: Vec<ArrayEntry>,
    /// `inputs_from`: references to shared inputs in other cases, as written. Carried, not
    /// resolved: resolution arrives with the first case that uses it (§3).
    pub inputs_from: Vec<Value>,
    /// `params`: the decimal-literal parameters of the case, as written; `None` when absent.
    pub params: Option<Value>,
}

impl Manifest {
    /// Parses and validates a manifest from its JSON text.
    pub fn from_json_str(text: &str) -> Result<Self, FixtureError> {
        let value: Value = serde_json::from_str(text).map_err(|e| FixtureError::Manifest {
            detail: format!("not valid JSON: {e}"),
        })?;
        Self::from_value(&value)
    }

    /// Validates a manifest already parsed as JSON.
    pub fn from_value(value: &Value) -> Result<Self, FixtureError> {
        let root = object(value, "manifest")?;

        let manifest_version = as_u64(require(root, "manifest_version", "")?, "manifest_version")?;
        if manifest_version != MANIFEST_VERSION {
            return Err(FixtureError::UnsupportedContract {
                field: "manifest_version".to_owned(),
                found: manifest_version.to_string(),
                supported: MANIFEST_VERSION.to_string(),
            });
        }
        let component = as_string(require(root, "component", "")?, "component")?;
        let case = as_string(require(root, "case", "")?, "case")?;
        let description = as_string(require(root, "description", "")?, "description")?;

        let contracts_obj = object(require(root, "contracts", "")?, "contracts")?;
        let mut contracts = BTreeMap::new();
        for (key, v) in contracts_obj {
            contracts.insert(key.clone(), as_string(v, &format!("contracts.{key}"))?);
        }
        let parity_version =
            contracts
                .get("parity_fixture")
                .ok_or_else(|| FixtureError::MissingKey {
                    key: "contracts.parity_fixture".to_owned(),
                })?;
        if parity_version != PARITY_FIXTURE_CONTRACT_VERSION {
            return Err(FixtureError::UnsupportedContract {
                field: "contracts.parity_fixture".to_owned(),
                found: parity_version.clone(),
                supported: PARITY_FIXTURE_CONTRACT_VERSION.to_owned(),
            });
        }

        let generator = require(root, "generator", "")?.clone();
        let oracle = parse_oracle(require(root, "oracle", "")?)?;
        let environment = require(root, "environment", "")?.clone();

        let seeds_obj = object(require(root, "seeds", "")?, "seeds")?;
        let mut seeds = BTreeMap::new();
        for (key, v) in seeds_obj {
            seeds.insert(key.clone(), as_u64(v, &format!("seeds.{key}"))?);
        }

        let provenance = parse_provenance(require(root, "provenance", "")?)?;
        if provenance.data != "synthetic" {
            return Err(FixtureError::NotSynthetic {
                found: provenance.data,
            });
        }

        let inputs = parse_entries(require(root, "inputs", "")?, "inputs", false)?;
        let outputs = parse_entries(require(root, "outputs", "")?, "outputs", true)?;
        let mut names: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for entry in inputs.iter().chain(outputs.iter()) {
            if !names.insert(entry.name.as_str()) {
                return Err(FixtureError::Manifest {
                    detail: format!("array name `{}` is listed twice", entry.name),
                });
            }
        }

        let inputs_from = match root.get("inputs_from") {
            Some(v) => as_array(v, "inputs_from")?.clone(),
            None => Vec::new(),
        };
        let params = root.get("params").cloned();

        Ok(Manifest {
            manifest_version,
            component,
            case,
            description,
            contracts,
            generator,
            oracle,
            environment,
            seeds,
            provenance,
            inputs,
            outputs,
            inputs_from,
            params,
        })
    }

    /// Every input and output entry, inputs first, in manifest order.
    pub fn entries(&self) -> impl Iterator<Item = &ArrayEntry> {
        self.inputs.iter().chain(self.outputs.iter())
    }

    /// The entry named `name`, or [`FixtureError::UnknownArray`].
    pub fn entry(&self, name: &str) -> Result<&ArrayEntry, FixtureError> {
        self.entries()
            .find(|e| e.name == name)
            .ok_or_else(|| FixtureError::UnknownArray {
                name: name.to_owned(),
            })
    }
}

fn object<'a>(value: &'a Value, path: &str) -> Result<&'a Map<String, Value>, FixtureError> {
    value.as_object().ok_or_else(|| FixtureError::Manifest {
        detail: format!("`{path}` must be a JSON object"),
    })
}

fn require<'a>(
    obj: &'a Map<String, Value>,
    key: &str,
    path: &str,
) -> Result<&'a Value, FixtureError> {
    obj.get(key).ok_or_else(|| FixtureError::MissingKey {
        key: if path.is_empty() {
            key.to_owned()
        } else {
            format!("{path}.{key}")
        },
    })
}

fn as_string(value: &Value, path: &str) -> Result<String, FixtureError> {
    value
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| FixtureError::Manifest {
            detail: format!("`{path}` must be a string"),
        })
}

fn as_u64(value: &Value, path: &str) -> Result<u64, FixtureError> {
    value.as_u64().ok_or_else(|| FixtureError::Manifest {
        detail: format!("`{path}` must be a non-negative integer"),
    })
}

fn as_bool(value: &Value, path: &str) -> Result<bool, FixtureError> {
    value.as_bool().ok_or_else(|| FixtureError::Manifest {
        detail: format!("`{path}` must be a boolean"),
    })
}

fn as_array<'a>(value: &'a Value, path: &str) -> Result<&'a Vec<Value>, FixtureError> {
    value.as_array().ok_or_else(|| FixtureError::Manifest {
        detail: format!("`{path}` must be a JSON array"),
    })
}

fn as_strings(value: &Value, path: &str) -> Result<Vec<String>, FixtureError> {
    as_array(value, path)?
        .iter()
        .enumerate()
        .map(|(i, v)| as_string(v, &format!("{path}[{i}]")))
        .collect()
}

fn optional_string(
    obj: &Map<String, Value>,
    key: &str,
    path: &str,
) -> Result<Option<String>, FixtureError> {
    match obj.get(key) {
        Some(Value::Null) | None => Ok(None),
        Some(v) => as_string(v, &format!("{path}.{key}")).map(Some),
    }
}

fn parse_oracle(value: &Value) -> Result<OracleInfo, FixtureError> {
    let obj = object(value, "oracle")?;
    let level = as_string(require(obj, "level", "oracle")?, "oracle.level")?.parse()?;
    let corrections_applied = as_strings(
        require(obj, "corrections_applied", "oracle")?,
        "oracle.corrections_applied",
    )?;
    let ledger_independent = match obj.get("ledger_independent") {
        Some(Value::Null) | None => None,
        Some(v) => Some(as_bool(v, "oracle.ledger_independent")?),
    };
    let import_patches = match obj.get("import_patches") {
        Some(Value::Null) | None => None,
        Some(v) => Some(as_strings(v, "oracle.import_patches")?),
    };
    Ok(OracleInfo {
        level,
        corrections_applied,
        ledger_independent,
        source: optional_string(obj, "source", "oracle")?,
        source_commit: optional_string(obj, "source_commit", "oracle")?,
        import_patches,
        manifest_tsv_sha256: optional_string(obj, "manifest_tsv_sha256", "oracle")?,
    })
}

fn parse_provenance(value: &Value) -> Result<Provenance, FixtureError> {
    let obj = object(value, "provenance")?;
    Ok(Provenance {
        data: as_string(require(obj, "data", "provenance")?, "provenance.data")?,
        profile: optional_string(obj, "profile", "provenance")?,
        synth_config: optional_string(obj, "synth_config", "provenance")?,
        synth_config_sha256: optional_string(obj, "synth_config_sha256", "provenance")?,
    })
}

fn parse_entries(
    value: &Value,
    path: &str,
    class_required: bool,
) -> Result<Vec<ArrayEntry>, FixtureError> {
    as_array(value, path)?
        .iter()
        .enumerate()
        .map(|(i, v)| parse_entry(v, &format!("{path}[{i}]"), class_required))
        .collect()
}

fn parse_entry(
    value: &Value,
    path: &str,
    class_required: bool,
) -> Result<ArrayEntry, FixtureError> {
    let obj = object(value, path)?;
    let name = as_string(require(obj, "name", path)?, &format!("{path}.name"))?;
    let file = as_string(require(obj, "file", path)?, &format!("{path}.file"))?;
    let dtype: DType =
        as_string(require(obj, "dtype", path)?, &format!("{path}.dtype"))?.parse()?;
    let shape = as_array(require(obj, "shape", path)?, &format!("{path}.shape"))?
        .iter()
        .enumerate()
        .map(|(j, d)| {
            as_u64(d, &format!("{path}.shape[{j}]")).and_then(|n| {
                usize::try_from(n).map_err(|_| FixtureError::Manifest {
                    detail: format!("`{path}.shape[{j}]` does not fit this platform's usize"),
                })
            })
        })
        .collect::<Result<Vec<usize>, FixtureError>>()?;
    let sha256 = as_string(require(obj, "sha256", path)?, &format!("{path}.sha256"))?;
    let class = match obj.get("class") {
        Some(v) => Some(as_string(v, &format!("{path}.class"))?.parse()?),
        None if class_required => {
            return Err(FixtureError::MissingKey {
                key: format!("{path}.class"),
            })
        }
        None => None,
    };
    check_file_name(&file, dtype, path)?;
    Ok(ArrayEntry {
        name,
        file,
        dtype,
        shape,
        sha256,
        class,
    })
}

/// A file name is a plain name inside the case directory (no separator, no `..`) and carries the
/// dtype as its suffix (§2.1 `<name>.f64le`).
fn check_file_name(file: &str, dtype: DType, path: &str) -> Result<(), FixtureError> {
    if file.is_empty() || file == "." || file == ".." || file.contains('/') || file.contains('\\') {
        return Err(FixtureError::Manifest {
            detail: format!("`{path}.file` (`{file}`) must be a plain file name inside the case"),
        });
    }
    let suffix = format!(".{}", dtype.as_str());
    if !file.ends_with(&suffix) || file.len() == suffix.len() {
        return Err(FixtureError::Manifest {
            detail: format!(
                "`{path}.file` (`{file}`) must end in `{suffix}`, the suffix of dtype {dtype}"
            ),
        });
    }
    Ok(())
}

/// A dense array read from a case: the manifest's C-order `shape` and the decoded elements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Array<T> {
    /// The declared shape; `product(shape) == data.len()`.
    pub shape: Vec<usize>,
    /// The elements, in C order.
    pub data: Vec<T>,
}

impl<T> Array<T> {
    /// The element count.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Whether the array has no element.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Checks the declared shape against `expected`; [`FixtureError::ShapeMismatch`] names `name`.
    pub fn expect_shape(&self, name: &str, expected: &[usize]) -> Result<&Self, FixtureError> {
        if self.shape == expected {
            Ok(self)
        } else {
            Err(FixtureError::ShapeMismatch {
                name: name.to_owned(),
                declared: self.shape.clone(),
                expected: expected.to_vec(),
            })
        }
    }

    /// The single element of a scalar array (shape `[]` or `[1]`, §2.1), or
    /// [`FixtureError::ShapeMismatch`] with expected `[1]`.
    pub fn scalar(&self, name: &str) -> Result<&T, FixtureError> {
        match (self.shape.as_slice(), self.data.as_slice()) {
            ([], [x]) | ([1], [x]) => Ok(x),
            _ => Err(FixtureError::ShapeMismatch {
                name: name.to_owned(),
                declared: self.shape.clone(),
                expected: vec![1],
            }),
        }
    }
}

/// A ragged list in the offsets + values encoding (§2.1): row `i` is
/// `values[offsets[i]..offsets[i + 1]]`. `offsets` has `n + 1` entries, starts at 0 and is
/// non-decreasing. **Order and duplicates are preserved**: legacy synthetic participation is a
/// multiset (plays-contract D-8).
///
/// [`Ragged::new`] validates the offsets; the fields are public so that a frame can be rebuilt
/// column-wise, and a value built directly from the fields is checked with [`Ragged::validate`]
/// before its rows are read. [`Ragged::row`] indexes like a slice does: it panics on an
/// out-of-range row or on offsets that were not validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ragged<T> {
    /// `n + 1` offsets into `values`, from 0 to `values.len()`.
    pub offsets: Vec<i64>,
    /// The concatenated rows.
    pub values: Vec<T>,
}

impl<T> Ragged<T> {
    /// Builds a ragged list, rejecting offsets that are empty, do not start at 0, decrease, are
    /// negative, or do not end at `values.len()` with [`FixtureError::BadOffsets`] naming `name`
    /// (the checks of [`Ragged::validate`]).
    ///
    /// ```
    /// use grid_synth::fixture::{FixtureError, Ragged};
    /// let parts = Ragged::new("off_players", vec![0, 2, 2, 3], vec![7, 7, 9]).unwrap();
    /// assert_eq!(parts.len(), 3);
    /// assert_eq!(parts.row(0), &[7, 7]); // duplicates kept (plays-contract D-8)
    /// assert_eq!(parts.row(1), &[] as &[i32]);
    /// assert!(matches!(
    ///     Ragged::new("x", vec![0, 5], vec![1]),
    ///     Err(FixtureError::BadOffsets { .. })
    /// ));
    /// ```
    pub fn new(name: &str, offsets: Vec<i64>, values: Vec<T>) -> Result<Self, FixtureError> {
        let ragged = Self { offsets, values };
        ragged.validate(name)?;
        Ok(ragged)
    }

    /// Checks the §2.1 offset invariants on a list built from its public fields: `offsets` is
    /// non-empty, starts at 0, is non-decreasing (hence never negative) and ends at
    /// `values.len()`. Fails with [`FixtureError::BadOffsets`] naming `name`. After `Ok`, every
    /// [`Ragged::row`] in `0..len()` is in bounds.
    pub fn validate(&self, name: &str) -> Result<(), FixtureError> {
        let bad = |detail: String| FixtureError::BadOffsets {
            name: name.to_owned(),
            detail,
        };
        let Some(&first) = self.offsets.first() else {
            return Err(bad("offsets is empty; it needs n + 1 entries".to_owned()));
        };
        if first != 0 {
            return Err(bad(format!("offsets[0] is {first}, not 0")));
        }
        for (i, pair) in self.offsets.windows(2).enumerate() {
            if pair[1] < pair[0] {
                return Err(bad(format!(
                    "offsets[{}] = {} is below offsets[{i}] = {}",
                    i + 1,
                    pair[1],
                    pair[0]
                )));
            }
        }
        let last = *self.offsets.last().expect("offsets is non-empty");
        let expected = i64::try_from(self.values.len()).map_err(|_| {
            bad(format!(
                "values has {} elements, beyond i64",
                self.values.len()
            ))
        })?;
        if last != expected {
            return Err(bad(format!(
                "the last offset is {last} but values has {expected} elements"
            )));
        }
        Ok(())
    }

    /// Builds a ragged list from rows, concatenating them in order.
    pub fn from_rows<I>(rows: I) -> Self
    where
        I: IntoIterator<Item = Vec<T>>,
    {
        let mut offsets = vec![0i64];
        let mut values = Vec::new();
        for row in rows {
            values.extend(row);
            offsets.push(values.len() as i64);
        }
        Self { offsets, values }
    }

    /// The row count `n`.
    pub fn len(&self) -> usize {
        self.offsets.len().saturating_sub(1)
    }

    /// Whether there is no row.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Row `i` as a slice of `values`. Panics when `i >= len()`, like slice indexing.
    pub fn row(&self, i: usize) -> &[T] {
        let start = usize::try_from(self.offsets[i]).expect("offsets validated non-negative");
        let end = usize::try_from(self.offsets[i + 1]).expect("offsets validated non-negative");
        &self.values[start..end]
    }

    /// The rows, in order.
    pub fn rows(&self) -> impl Iterator<Item = &[T]> + '_ {
        (0..self.len()).map(move |i| self.row(i))
    }
}

/// An opened case directory: its validated manifest plus typed readers for its arrays.
#[derive(Debug, Clone, PartialEq)]
pub struct FixtureCase {
    dir: PathBuf,
    manifest: Manifest,
}

impl FixtureCase {
    /// Opens `<dir>/manifest.json` and validates it. Reads no array yet.
    pub fn open(dir: impl AsRef<Path>) -> Result<Self, FixtureError> {
        let dir = dir.as_ref().to_path_buf();
        let manifest_path = dir.join(MANIFEST_FILE);
        let text = fs::read_to_string(&manifest_path).map_err(|e| io_error(&manifest_path, &e))?;
        let manifest = Manifest::from_json_str(&text)?;
        Ok(Self { dir, manifest })
    }

    /// Builds a case over an already-parsed manifest and a directory; the arrays are read from
    /// `dir` on demand.
    pub fn with_manifest(dir: impl AsRef<Path>, manifest: Manifest) -> Self {
        Self {
            dir: dir.as_ref().to_path_buf(),
            manifest,
        }
    }

    /// The case directory.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The validated manifest.
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    /// The manifest entry named `name`.
    pub fn entry(&self, name: &str) -> Result<&ArrayEntry, FixtureError> {
        self.manifest.entry(name)
    }

    /// Reads an `f64le` array. NaN is preserved bit for bit.
    pub fn read_f64(&self, name: &str) -> Result<Array<f64>, FixtureError> {
        let (entry, bytes) = self.read_binary(name, DType::F64le)?;
        Ok(Array {
            shape: entry.shape.clone(),
            data: decode(&bytes, f64::from_le_bytes),
        })
    }

    /// Reads an `i64le` array.
    pub fn read_i64(&self, name: &str) -> Result<Array<i64>, FixtureError> {
        let (entry, bytes) = self.read_binary(name, DType::I64le)?;
        Ok(Array {
            shape: entry.shape.clone(),
            data: decode(&bytes, i64::from_le_bytes),
        })
    }

    /// Reads an `i32le` array.
    pub fn read_i32(&self, name: &str) -> Result<Array<i32>, FixtureError> {
        let (entry, bytes) = self.read_binary(name, DType::I32le)?;
        Ok(Array {
            shape: entry.shape.clone(),
            data: decode(&bytes, i32::from_le_bytes),
        })
    }

    /// Reads a `u8` array as booleans. A byte other than 0 or 1 is
    /// [`FixtureError::DtypeMismatch`] (§2.1: "values 0 or 1 only").
    pub fn read_bool(&self, name: &str) -> Result<Array<bool>, FixtureError> {
        let (entry, bytes) = self.read_binary(name, DType::U8)?;
        let mut data = Vec::with_capacity(bytes.len());
        for (i, byte) in bytes.iter().enumerate() {
            match byte {
                0 => data.push(false),
                1 => data.push(true),
                other => {
                    return Err(FixtureError::DtypeMismatch {
                        name: name.to_owned(),
                        expected: "u8 boolean (0 or 1)".to_owned(),
                        found: format!("byte {other} at index {i}"),
                    })
                }
            }
        }
        Ok(Array {
            shape: entry.shape.clone(),
            data,
        })
    }

    /// Reads a `json` string array. The element count must equal `product(shape)`
    /// ([`FixtureError::ShapeMismatch`] otherwise); a file that is not a JSON array of strings is
    /// [`FixtureError::DtypeMismatch`].
    pub fn read_strings(&self, name: &str) -> Result<Array<String>, FixtureError> {
        let entry = self.entry(name)?;
        if entry.dtype != DType::Json {
            return Err(FixtureError::DtypeMismatch {
                name: name.to_owned(),
                expected: DType::Json.to_string(),
                found: entry.dtype.to_string(),
            });
        }
        let path = self.dir.join(&entry.file);
        let bytes = fs::read(&path).map_err(|e| io_error(&path, &e))?;
        let data: Vec<String> =
            serde_json::from_slice(&bytes).map_err(|e| FixtureError::DtypeMismatch {
                name: name.to_owned(),
                expected: "json array of strings".to_owned(),
                found: format!("not a JSON array of strings (`{}`): {e}", entry.file),
            })?;
        if data.len() != entry.element_count() {
            return Err(FixtureError::ShapeMismatch {
                name: name.to_owned(),
                declared: entry.shape.clone(),
                expected: vec![data.len()],
            });
        }
        Ok(Array {
            shape: entry.shape.clone(),
            data,
        })
    }

    /// Reads a scalar `f64le` (shape `[]` or `[1]`).
    pub fn read_scalar_f64(&self, name: &str) -> Result<f64, FixtureError> {
        self.read_f64(name)?.scalar(name).copied()
    }

    /// Reads a scalar `i64le` (shape `[]` or `[1]`).
    pub fn read_scalar_i64(&self, name: &str) -> Result<i64, FixtureError> {
        self.read_i64(name)?.scalar(name).copied()
    }

    /// Reads the ragged list `name` from its `<name>.offsets` (`i64le`) and `<name>.values`
    /// (`i64le`) entries (§2.1).
    pub fn read_ragged_i64(&self, name: &str) -> Result<Ragged<i64>, FixtureError> {
        let offsets = self.read_i64(&format!("{name}.offsets"))?.data;
        let values = self.read_i64(&format!("{name}.values"))?.data;
        Ragged::new(name, offsets, values)
    }

    /// Reads the ragged list `name` whose values are `f64le`.
    pub fn read_ragged_f64(&self, name: &str) -> Result<Ragged<f64>, FixtureError> {
        let offsets = self.read_i64(&format!("{name}.offsets"))?.data;
        let values = self.read_f64(&format!("{name}.values"))?.data;
        Ragged::new(name, offsets, values)
    }

    /// Looks the entry up, checks the dtype, reads the file and checks
    /// `byte length = product(shape) × itemsize`.
    fn read_binary(
        &self,
        name: &str,
        dtype: DType,
    ) -> Result<(&ArrayEntry, Vec<u8>), FixtureError> {
        let entry = self.entry(name)?;
        if entry.dtype != dtype {
            return Err(FixtureError::DtypeMismatch {
                name: name.to_owned(),
                expected: dtype.to_string(),
                found: entry.dtype.to_string(),
            });
        }
        let itemsize = dtype.itemsize().expect("binary dtypes have an itemsize");
        let expected_bytes = entry
            .element_count()
            .checked_mul(itemsize)
            .and_then(|n| u64::try_from(n).ok())
            .ok_or_else(|| FixtureError::Manifest {
                detail: format!("array `{name}`: product(shape) × itemsize overflows"),
            })?;
        let path = self.dir.join(&entry.file);
        let bytes = fs::read(&path).map_err(|e| io_error(&path, &e))?;
        let found_bytes = bytes.len() as u64;
        if found_bytes != expected_bytes {
            return Err(FixtureError::LengthMismatch {
                name: name.to_owned(),
                expected_bytes,
                found_bytes,
            });
        }
        Ok((entry, bytes))
    }
}

fn io_error(path: &Path, error: &io::Error) -> FixtureError {
    FixtureError::Io {
        path: path.to_path_buf(),
        kind: error.kind(),
        message: error.to_string(),
    }
}

fn decode<T, const N: usize>(bytes: &[u8], from_le: fn([u8; N]) -> T) -> Vec<T> {
    // The byte length was checked to be a multiple of `N` by the caller; the remainder is empty.
    let (chunks, _remainder) = bytes.as_chunks::<N>();
    chunks.iter().map(|chunk| from_le(*chunk)).collect()
}

/// `true` when `observed` is above `limit` **or the two are incomparable** (a NaN observed value
/// or bound): every criterion fails closed.
fn exceeds(observed: f64, limit: f64) -> bool {
    !matches!(
        observed.partial_cmp(&limit),
        Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)
    )
}

/// The first failing element of a tolerance comparison (parity-fixture-contract §11: "a tolerance
/// helper per class reports the failing index and both values").
///
/// `observed` and `limit` are the criterion's number and bound where the criterion has one:
/// `|r − p|` and `1e-12` for Class A, the ∞-norm ratio and `1e-9` for Class A′, the relative
/// residual and `tol_cg` for Class B's second part, `|r − p|` and `0` for a value or bitwise
/// mismatch between finite numbers, the length difference and `0` for a length mismatch
/// (`index = min(len)`, both lengths as the values). Where no such number exists they are
/// `None`, never a sentinel (KI-P1): an exact mismatch on a non-numeric type and a NaN-position
/// mismatch have no `observed`; Class B's convergence part has neither. Class B's diagnostic
/// parts (convergence, residual) report `index = 0`.
#[derive(Debug, Clone, PartialEq)]
pub struct Mismatch {
    /// The element index.
    pub index: usize,
    /// The Rust value, in `Debug` form.
    pub rust: String,
    /// The oracle value, in `Debug` form.
    pub oracle: String,
    /// The criterion's observed number; `None` when the criterion has none (see the type doc).
    pub observed: Option<f64>,
    /// The criterion's bound; `None` when the criterion has none (a boolean part).
    pub limit: Option<f64>,
}

impl fmt::Display for Mismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn number(v: Option<f64>) -> String {
            v.map_or_else(|| "n/a".to_owned(), |x| format!("{x:e}"))
        }
        write!(
            f,
            "mismatch at index {}: rust {} vs oracle {} (observed {}, limit {})",
            self.index,
            self.rust,
            self.oracle,
            number(self.observed),
            number(self.limit)
        )
    }
}

impl std::error::Error for Mismatch {}

/// The summary of a passing comparison.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClassReport {
    /// Elements compared (the common length; NaN pairs included).
    pub n: usize,
    /// `max |r − p|` over the non-NaN pairs; `0` for an exact comparison.
    pub max_abs_error: f64,
    /// Class A: `max |r − p| / |p|` over pairs with `p ≠ 0`; Class A′ and B: `‖r − p‖∞ / ‖p‖∞`;
    /// `0` for an exact comparison or when no element qualifies.
    pub max_rel_error: f64,
}

fn length_mismatch(rust_len: usize, oracle_len: usize) -> Mismatch {
    Mismatch {
        index: rust_len.min(oracle_len),
        rust: format!("{rust_len} elements"),
        oracle: format!("{oracle_len} elements"),
        observed: Some(rust_len.abs_diff(oracle_len) as f64),
        limit: Some(0.0),
    }
}

/// `|r − p|` when both are finite numbers or equal infinities; `None` when either is NaN (there
/// is no observed number to report, KI-P1).
fn abs_difference(r: f64, p: f64) -> Option<f64> {
    if r.is_nan() || p.is_nan() {
        None
    } else if r == p {
        Some(0.0)
    } else {
        Some((r - p).abs())
    }
}

/// Exact comparison (§6 "in every class": integers, booleans, ids, categoricals, sentinels,
/// empty-versus-missing). For floats use [`compare_exact_f64`], [`compare_bitwise`] or a class
/// helper: `PartialEq` on `f64` makes NaN unequal to itself.
pub fn compare_exact<T: PartialEq + fmt::Debug>(
    rust: &[T],
    oracle: &[T],
) -> Result<ClassReport, Mismatch> {
    if rust.len() != oracle.len() {
        return Err(length_mismatch(rust.len(), oracle.len()));
    }
    for (index, (r, p)) in rust.iter().zip(oracle).enumerate() {
        if r != p {
            return Err(Mismatch {
                index,
                rust: format!("{r:?}"),
                oracle: format!("{p:?}"),
                observed: None,
                limit: Some(0.0),
            });
        }
    }
    Ok(ClassReport {
        n: rust.len(),
        max_abs_error: 0.0,
        max_rel_error: 0.0,
    })
}

/// Value-exact comparison of float arrays: NaN positions must coincide and every non-NaN pair
/// must be `==`, so `-0.0 == 0.0` and a NaN payload is not compared. This is the weakest
/// reading of the exact class as §6 states it for floats ("NaN positions must coincide; v0
/// sentinels must match exactly"). It is **not** the S-1 round-trip criterion:
/// `crate::legacy_v0::LegacyV0World::first_difference` compares every float column with
/// [`compare_bitwise`]. Use this helper only for a quantity whose case declares that the sign of
/// zero or a NaN payload is not carried.
///
/// ```
/// use grid_synth::fixture::compare_exact_f64;
/// assert!(compare_exact_f64(&[0.0, f64::NAN, 3.0], &[-0.0, f64::NAN, 3.0]).is_ok());
/// let m = compare_exact_f64(&[0.0, 1.0], &[0.0, f64::NAN]).unwrap_err();
/// assert_eq!((m.index, m.observed), (1, None));
/// ```
pub fn compare_exact_f64(rust: &[f64], oracle: &[f64]) -> Result<ClassReport, Mismatch> {
    if rust.len() != oracle.len() {
        return Err(length_mismatch(rust.len(), oracle.len()));
    }
    for (index, (&r, &p)) in rust.iter().zip(oracle).enumerate() {
        let same = if r.is_nan() || p.is_nan() {
            r.is_nan() && p.is_nan()
        } else {
            r == p
        };
        if !same {
            return Err(Mismatch {
                index,
                rust: format!("{r:?}"),
                oracle: format!("{p:?}"),
                observed: abs_difference(r, p),
                limit: Some(0.0),
            });
        }
    }
    Ok(ClassReport {
        n: rust.len(),
        max_abs_error: 0.0,
        max_rel_error: 0.0,
    })
}

/// Bit-for-bit comparison of float arrays (`to_bits`): NaN position **and payload** and the sign
/// of zero must survive. The S-1 round-trip criterion of synthetic-world.md §10.3 (class
/// "exact": "every column, id order, NaN position and v0 sentinel round-trips") as
/// `crate::legacy_v0::LegacyV0World::first_difference` applies it to every float column, and
/// the check for any column that passes through a stage untouched, where any bit change is a
/// defect. [`compare_exact_f64`] is the weaker value-exact reading.
pub fn compare_bitwise(rust: &[f64], oracle: &[f64]) -> Result<ClassReport, Mismatch> {
    if rust.len() != oracle.len() {
        return Err(length_mismatch(rust.len(), oracle.len()));
    }
    for (index, (&r, &p)) in rust.iter().zip(oracle).enumerate() {
        if r.to_bits() != p.to_bits() {
            return Err(Mismatch {
                index,
                rust: format!("{r:?} (0x{:016x})", r.to_bits()),
                oracle: format!("{p:?} (0x{:016x})", p.to_bits()),
                observed: abs_difference(r, p),
                limit: Some(0.0),
            });
        }
    }
    Ok(ClassReport {
        n: rust.len(),
        max_abs_error: 0.0,
        max_rel_error: 0.0,
    })
}

/// Class A: `max_i |r_i − p_i| ≤ 1e-12` (§6). NaN positions must coincide; a NaN pair is skipped.
/// Equal infinities pass. Returns the first failing index.
///
/// ```
/// use grid_synth::fixture::compare_class_a;
/// let oracle = [1.0, f64::NAN, 3.0];
/// assert!(compare_class_a(&[1.0 + 1e-13, f64::NAN, 3.0], &oracle).is_ok());
/// let mismatch = compare_class_a(&[1.0, f64::NAN, 3.0 + 1e-6], &oracle).unwrap_err();
/// assert_eq!(mismatch.index, 2);
/// assert_eq!(mismatch.limit, Some(1e-12));
/// ```
pub fn compare_class_a(rust: &[f64], oracle: &[f64]) -> Result<ClassReport, Mismatch> {
    if rust.len() != oracle.len() {
        return Err(length_mismatch(rust.len(), oracle.len()));
    }
    let mut max_abs_error = 0.0f64;
    let mut max_rel_error = 0.0f64;
    for (index, (&r, &p)) in rust.iter().zip(oracle).enumerate() {
        if r.is_nan() || p.is_nan() {
            if r.is_nan() && p.is_nan() {
                continue;
            }
            return Err(Mismatch {
                index,
                rust: format!("{r:?}"),
                oracle: format!("{p:?}"),
                observed: None,
                limit: Some(CLASS_A_ABS_TOL),
            });
        }
        if r == p {
            continue;
        }
        let d = (r - p).abs();
        if exceeds(d, CLASS_A_ABS_TOL) {
            return Err(Mismatch {
                index,
                rust: format!("{r:?}"),
                oracle: format!("{p:?}"),
                observed: Some(d),
                limit: Some(CLASS_A_ABS_TOL),
            });
        }
        max_abs_error = max_abs_error.max(d);
        if p != 0.0 {
            max_rel_error = max_rel_error.max(d / p.abs());
        }
    }
    Ok(ClassReport {
        n: rust.len(),
        max_abs_error,
        max_rel_error,
    })
}

/// Class A′ with an explicit bound: `‖r − p‖∞ / ‖p‖∞ ≤ tol` over the non-NaN pairs; NaN positions
/// must coincide. `‖p‖∞ = 0` is the declared degenerate case: then `‖r‖∞` must be 0.
fn compare_relative_inf_norm(
    rust: &[f64],
    oracle: &[f64],
    tol: f64,
) -> Result<ClassReport, Mismatch> {
    if rust.len() != oracle.len() {
        return Err(length_mismatch(rust.len(), oracle.len()));
    }
    let mut num = 0.0f64;
    let mut num_index = 0usize;
    let mut den = 0.0f64;
    let mut rust_norm = 0.0f64;
    let mut rust_norm_index = 0usize;
    for (index, (&r, &p)) in rust.iter().zip(oracle).enumerate() {
        if r.is_nan() || p.is_nan() {
            if r.is_nan() && p.is_nan() {
                continue;
            }
            return Err(Mismatch {
                index,
                rust: format!("{r:?}"),
                oracle: format!("{p:?}"),
                observed: None,
                limit: Some(tol),
            });
        }
        let d = if r == p { 0.0 } else { (r - p).abs() };
        if d > num || d.is_nan() {
            num = d;
            num_index = index;
        }
        den = den.max(p.abs());
        if r.abs() > rust_norm {
            rust_norm = r.abs();
            rust_norm_index = index;
        }
    }
    if den == 0.0 {
        if rust_norm == 0.0 {
            return Ok(ClassReport {
                n: rust.len(),
                max_abs_error: 0.0,
                max_rel_error: 0.0,
            });
        }
        return Err(Mismatch {
            index: rust_norm_index,
            rust: format!("{:?}", rust[rust_norm_index]),
            oracle: format!("{:?}", oracle[rust_norm_index]),
            observed: Some(rust_norm),
            limit: Some(0.0),
        });
    }
    let ratio = num / den;
    if exceeds(ratio, tol) {
        return Err(Mismatch {
            index: num_index,
            rust: format!("{:?}", rust[num_index]),
            oracle: format!("{:?}", oracle[num_index]),
            observed: Some(ratio),
            limit: Some(tol),
        });
    }
    Ok(ClassReport {
        n: rust.len(),
        max_abs_error: num,
        max_rel_error: ratio,
    })
}

/// Class A′: `‖r − p‖∞ / ‖p‖∞ ≤ 1e-9` (§6, dense linear solves). NaN positions must coincide.
/// `‖p‖∞ = 0` is the declared degenerate case: then `‖r‖∞` must be 0. The failing index is the
/// position of the largest `|r − p|`.
pub fn compare_class_a_prime(rust: &[f64], oracle: &[f64]) -> Result<ClassReport, Mismatch> {
    compare_relative_inf_norm(rust, oracle, CLASS_A_PRIME_REL_TOL)
}

/// The inputs of a Class B comparison (§6; `rapm-attribution.md` §10.3).
#[derive(Debug, Clone, PartialEq)]
pub struct ClassBInputs<'a> {
    /// The Rust solver's convergence flag; must be `true`.
    pub converged: bool,
    /// The Rust solution's relative residual `‖A β − b‖ / ‖b‖` on the **oracle's** system.
    pub relative_residual: f64,
    /// The CG tolerance recorded in the case (`params.tol_cg`).
    pub tol_cg: f64,
    /// The Rust CG solution.
    pub beta_cg: &'a [f64],
    /// The dense reference solution.
    pub beta_dense: &'a [f64],
}

/// Class B, all three parts: `converged`, `relative_residual ≤ tol_cg`, and
/// `‖β_cg − β_dense‖∞ / ‖β_dense‖∞ ≤ 1e-9`. The report is the third part's; the first two
/// report `index = 0` on failure, with the flag or the residual as the values. The convergence
/// part is boolean: its mismatch carries no `observed` and no `limit`.
pub fn compare_class_b(inputs: &ClassBInputs<'_>) -> Result<ClassReport, Mismatch> {
    if !inputs.converged {
        return Err(Mismatch {
            index: 0,
            rust: "converged = false".to_owned(),
            oracle: "converged = true".to_owned(),
            observed: None,
            limit: None,
        });
    }
    if exceeds(inputs.relative_residual, inputs.tol_cg) {
        return Err(Mismatch {
            index: 0,
            rust: format!("relative residual {:?}", inputs.relative_residual),
            oracle: format!("tol_cg {:?}", inputs.tol_cg),
            observed: Some(inputs.relative_residual),
            limit: Some(inputs.tol_cg),
        });
    }
    compare_relative_inf_norm(inputs.beta_cg, inputs.beta_dense, CLASS_B_DENSE_REL_TOL)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_manifest() -> Value {
        serde_json::json!({
            "case": "toy-legacy",
            "component": "scoring",
            "contracts": { "parity_fixture": "1" },
            "description": "unit-test manifest",
            "environment": {},
            "generator": {},
            "inputs": [],
            "manifest_version": 1,
            "oracle": { "corrections_applied": [], "level": "legacy" },
            "outputs": [],
            "provenance": { "data": "synthetic" },
            "seeds": { "synth": 7 }
        })
    }

    #[test]
    fn dtype_and_class_spellings_round_trip() {
        for d in DType::ALL {
            assert_eq!(d.as_str().parse::<DType>(), Ok(d));
        }
        assert!(matches!(
            "f32le".parse::<DType>(),
            Err(FixtureError::Manifest { .. })
        ));
        assert_eq!(DType::F64le.itemsize(), Some(8));
        assert_eq!(DType::I32le.itemsize(), Some(4));
        assert_eq!(DType::U8.itemsize(), Some(1));
        assert_eq!(DType::Json.itemsize(), None);
        for c in ToleranceClass::ALL {
            assert_eq!(c.as_str().parse::<ToleranceClass>(), Ok(c));
        }
        assert_eq!(
            "A\u{2032}".parse::<ToleranceClass>(),
            Ok(ToleranceClass::APrime)
        );
        assert!("C".parse::<ToleranceClass>().is_err());
        assert!(ToleranceClass::B.has_helper());
        assert!(!ToleranceClass::D.has_helper());
        assert_eq!(
            "corrected".parse::<OracleLevel>(),
            Ok(OracleLevel::Corrected)
        );
        assert!("fixed".parse::<OracleLevel>().is_err());
    }

    #[test]
    fn minimal_manifest_parses() {
        let m = Manifest::from_value(&minimal_manifest()).expect("valid");
        assert_eq!(m.component, "scoring");
        assert_eq!(m.oracle.level, OracleLevel::Legacy);
        // Absent keys are `None`, never a substituted value (KI-P1).
        assert_eq!(m.oracle.ledger_independent, None);
        assert_eq!(m.oracle.import_patches, None);
        assert_eq!(m.oracle.source, None);
        let mut v = minimal_manifest();
        v["oracle"]["ledger_independent"] = serde_json::json!(false);
        v["oracle"]["import_patches"] = serde_json::json!([]);
        let m2 = Manifest::from_value(&v).expect("valid");
        assert_eq!(m2.oracle.ledger_independent, Some(false));
        assert_eq!(m2.oracle.import_patches, Some(Vec::new()));
        // The §5.1 configuration hash is carried as written and is absent elsewhere; the label
        // beside it is a separate key.
        assert_eq!(m.provenance.synth_config, None);
        assert_eq!(m.provenance.synth_config_sha256, None);
        let mut v = minimal_manifest();
        v["provenance"]["synth_config"] = serde_json::json!("CANONICAL_SYNTH");
        v["provenance"]["synth_config_sha256"] = serde_json::json!("ab".repeat(32));
        let m3 = Manifest::from_value(&v).expect("valid");
        assert_eq!(
            m3.provenance.synth_config.as_deref(),
            Some("CANONICAL_SYNTH")
        );
        assert_eq!(
            m3.provenance.synth_config_sha256.as_deref(),
            Some("ab".repeat(32).as_str())
        );
        v["provenance"]["synth_config_sha256"] = serde_json::json!(7);
        assert!(matches!(
            Manifest::from_value(&v),
            Err(FixtureError::Manifest { detail }) if detail.contains("provenance.synth_config_sha256")
        ));
        assert_eq!(m.seeds.get("synth"), Some(&7));
        assert!(m.inputs_from.is_empty());
        assert!(m.params.is_none());
        assert_eq!(
            m.entry("nothing"),
            Err(FixtureError::UnknownArray {
                name: "nothing".into()
            })
        );
    }

    #[test]
    fn every_required_key_is_required() {
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
            let mut v = minimal_manifest();
            v.as_object_mut().unwrap().remove(key);
            assert_eq!(
                Manifest::from_value(&v),
                Err(FixtureError::MissingKey { key: key.into() }),
                "{key}"
            );
        }
        let mut v = minimal_manifest();
        v["contracts"] = serde_json::json!({ "plays": "grid.plays/0" });
        assert_eq!(
            Manifest::from_value(&v),
            Err(FixtureError::MissingKey {
                key: "contracts.parity_fixture".into()
            })
        );
        let mut v = minimal_manifest();
        v["oracle"] = serde_json::json!({ "level": "legacy" });
        assert_eq!(
            Manifest::from_value(&v),
            Err(FixtureError::MissingKey {
                key: "oracle.corrections_applied".into()
            })
        );
        let mut v = minimal_manifest();
        v["outputs"] = serde_json::json!([
            { "dtype": "f64le", "file": "y.f64le", "name": "y", "sha256": "0", "shape": [2] }
        ]);
        assert_eq!(
            Manifest::from_value(&v),
            Err(FixtureError::MissingKey {
                key: "outputs[0].class".into()
            })
        );
    }

    #[test]
    fn versions_and_provenance_are_enforced() {
        let mut v = minimal_manifest();
        v["manifest_version"] = serde_json::json!(2);
        assert!(matches!(
            Manifest::from_value(&v),
            Err(FixtureError::UnsupportedContract { field, .. }) if field == "manifest_version"
        ));
        let mut v = minimal_manifest();
        v["contracts"]["parity_fixture"] = serde_json::json!("2");
        assert!(matches!(
            Manifest::from_value(&v),
            Err(FixtureError::UnsupportedContract { field, found, .. })
                if field == "contracts.parity_fixture" && found == "2"
        ));
        let mut v = minimal_manifest();
        v["provenance"]["data"] = serde_json::json!("nflverse");
        assert_eq!(
            Manifest::from_value(&v),
            Err(FixtureError::NotSynthetic {
                found: "nflverse".into()
            })
        );
    }

    #[test]
    fn malformed_values_are_manifest_errors() {
        let mut v = minimal_manifest();
        v["seeds"]["synth"] = serde_json::json!(-1);
        assert!(matches!(
            Manifest::from_value(&v),
            Err(FixtureError::Manifest { detail }) if detail.contains("seeds.synth")
        ));
        let mut v = minimal_manifest();
        v["inputs"] = serde_json::json!([
            { "dtype": "f64le", "file": "../y.f64le", "name": "y", "sha256": "0", "shape": [2] }
        ]);
        assert!(matches!(
            Manifest::from_value(&v),
            Err(FixtureError::Manifest { detail }) if detail.contains("plain file name")
        ));
        let mut v = minimal_manifest();
        v["inputs"] = serde_json::json!([
            { "dtype": "f64le", "file": "y.i64le", "name": "y", "sha256": "0", "shape": [2] }
        ]);
        assert!(matches!(
            Manifest::from_value(&v),
            Err(FixtureError::Manifest { detail }) if detail.contains(".f64le")
        ));
        let mut v = minimal_manifest();
        v["inputs"] = serde_json::json!([
            { "dtype": "f64le", "file": "y.f64le", "name": "y", "sha256": "0", "shape": [2] },
            { "dtype": "i64le", "file": "y.i64le", "name": "y", "sha256": "0", "shape": [2] }
        ]);
        assert!(matches!(
            Manifest::from_value(&v),
            Err(FixtureError::Manifest { detail }) if detail.contains("listed twice")
        ));
        assert!(matches!(
            Manifest::from_json_str("{ not json"),
            Err(FixtureError::Manifest { .. })
        ));
    }

    #[test]
    fn ragged_offsets_are_validated() {
        let ok = Ragged::new("r", vec![0, 2, 2, 5], vec![1, 2, 3, 4, 5]).expect("valid");
        assert_eq!(ok.len(), 3);
        assert_eq!(ok.row(0), &[1, 2]);
        assert_eq!(ok.row(1), &[] as &[i32]);
        assert_eq!(ok.row(2), &[3, 4, 5]);
        assert_eq!(ok.rows().count(), 3);
        let rebuilt = Ragged::from_rows(ok.rows().map(<[i32]>::to_vec));
        assert_eq!(rebuilt, ok);
        assert_eq!(rebuilt.validate("r"), Ok(()));
        // A list built from its public fields is checked by `validate`, the same checks `new`
        // runs, so a caller never reaches the `row` panic on unvalidated offsets.
        let direct = Ragged {
            offsets: vec![0, 99, 2],
            values: vec![1, 2],
        };
        assert!(matches!(
            direct.validate("direct"),
            Err(FixtureError::BadOffsets { ref name, ref detail })
                if name == "direct" && detail.contains("below")
        ));
        for (offsets, values, what) in [
            (vec![], vec![1], "empty"),
            (vec![1, 1], vec![], "not 0"),
            (vec![0, 2, 1], vec![1, 2], "below"),
            (vec![0, 1], vec![1, 2], "last offset"),
            (vec![0, -1], vec![], "below"),
        ] {
            assert!(
                matches!(
                    Ragged::new("r", offsets, values),
                    Err(FixtureError::BadOffsets { ref name, ref detail })
                        if name == "r" && detail.contains(what)
                ),
                "{what}"
            );
        }
    }

    #[test]
    fn array_shape_helpers() {
        let a = Array {
            shape: vec![1],
            data: vec![4.0],
        };
        assert_eq!(a.scalar("a"), Ok(&4.0));
        let b = Array {
            shape: vec![],
            data: vec![4.0],
        };
        assert_eq!(b.scalar("b"), Ok(&4.0));
        let c = Array {
            shape: vec![2],
            data: vec![4.0, 5.0],
        };
        assert_eq!(
            c.scalar("c"),
            Err(FixtureError::ShapeMismatch {
                name: "c".into(),
                declared: vec![2],
                expected: vec![1]
            })
        );
        assert!(c.expect_shape("c", &[2]).is_ok());
        assert!(c.expect_shape("c", &[1, 2]).is_err());
        assert_eq!(c.len(), 2);
        assert!(!c.is_empty());
    }

    #[test]
    fn exact_and_bitwise_helpers() {
        assert_eq!(compare_exact(&[1i64, 2, 3], &[1, 2, 3]).map(|r| r.n), Ok(3));
        let err = compare_exact(&[true, false], &[true, true]).expect_err("differs");
        assert_eq!(err.index, 1);
        assert_eq!((err.rust.as_str(), err.oracle.as_str()), ("false", "true"));
        let err = compare_exact(&["a", "b"], &["a"]).expect_err("length");
        assert_eq!(err.index, 1);
        assert_eq!(err.observed, Some(1.0));
        // A non-numeric exact mismatch has no observed number (KI-P1: no NaN marker).
        assert_eq!(err.limit, Some(0.0));
        let err = compare_exact(&["a"], &["b"]).expect_err("differs");
        assert_eq!((err.observed, err.limit), (None, Some(0.0)));
        // NaN payloads and signed zeros are distinguished bitwise.
        let payload = f64::from_bits(0x7ff8_0000_dead_beef);
        assert!(compare_bitwise(&[payload, -0.0], &[payload, -0.0]).is_ok());
        let err = compare_bitwise(&[payload], &[f64::NAN]).expect_err("payload differs");
        assert_eq!(err.index, 0);
        assert_eq!(err.observed, None, "no |r − p| exists between two NaNs");
        let err = compare_bitwise(&[0.0], &[-0.0]).expect_err("sign of zero differs");
        assert_eq!(err.index, 0);
        assert_eq!(err.observed, Some(0.0));
        assert!(err.rust.contains("0x0000000000000000"));
        // The value-exact check is the §6 exact class for floats: NaN positions coincide,
        // non-NaN values are ==, so payload and sign of zero are not compared.
        assert!(compare_exact_f64(&[payload, -0.0, 2.5], &[f64::NAN, 0.0, 2.5]).is_ok());
        let err = compare_exact_f64(&[1.0, 2.0], &[1.0, 2.5]).expect_err("value differs");
        assert_eq!(
            (err.index, err.observed, err.limit),
            (1, Some(0.5), Some(0.0))
        );
        let err = compare_exact_f64(&[f64::NAN, 2.0], &[1.0, 2.0]).expect_err("NaN position");
        assert_eq!((err.index, err.observed), (0, None));
        let err = compare_exact_f64(&[1.0], &[1.0, 2.0]).expect_err("length");
        assert_eq!(err.index, 1);
    }

    #[test]
    fn class_a_helper() {
        let p = [1.0, f64::NAN, 3.0, f64::INFINITY];
        let r = [1.0 + 4e-13, f64::NAN, 3.0 - 9e-13, f64::INFINITY];
        let report = compare_class_a(&r, &p).expect("within 1e-12");
        assert_eq!(report.n, 4);
        assert!(report.max_abs_error <= 1e-12 && report.max_abs_error > 0.0);
        assert!(report.max_rel_error > 0.0);
        let err = compare_class_a(&[1.0, 2.0, 3.0 + 2e-12], &[1.0, 2.0, 3.0]).expect_err("2e-12");
        assert_eq!(err.index, 2);
        assert!(err.observed.is_some_and(|d| d > 1e-12) && err.limit == Some(CLASS_A_ABS_TOL));
        let err = compare_class_a(&[1.0, 2.0], &[1.0, f64::NAN]).expect_err("NaN position");
        assert_eq!(err.index, 1);
        assert_eq!(err.observed, None);
        let err = compare_class_a(&[1.0], &[1.0, 2.0]).expect_err("length");
        assert_eq!(err.index, 1);
    }

    #[test]
    fn class_a_prime_helper() {
        let p = [1000.0, -2000.0, 0.5];
        let r = [1000.0 + 1e-6, -2000.0, 0.5];
        let report = compare_class_a_prime(&r, &p).expect("1e-6 / 2000 = 5e-10 ≤ 1e-9");
        assert_eq!(report.n, 3);
        // 1000.0 + 1e-6 rounds at ulp(1000) ≈ 1.1e-13, so the ratio is 5e-10 to within ~1e-16.
        assert!(
            (report.max_rel_error - 5e-10).abs() < 1e-16,
            "{}",
            report.max_rel_error
        );
        let err = compare_class_a_prime(&[1000.0, -2000.0 + 3e-6, 0.5], &p).expect_err("1.5e-9");
        assert_eq!(err.index, 1);
        assert!(err.observed.is_some_and(|d| d > 1e-9) && err.limit == Some(CLASS_A_PRIME_REL_TOL));
        // Degenerate ‖p‖∞ = 0: r must be exactly zero.
        assert!(compare_class_a_prime(&[0.0, 0.0], &[0.0, 0.0]).is_ok());
        let err = compare_class_a_prime(&[0.0, 1e-300], &[0.0, 0.0]).expect_err("degenerate");
        assert_eq!(err.index, 1);
        assert_eq!(err.limit, Some(0.0));
        let err = compare_class_a_prime(&[f64::NAN, 1.0], &[1.0, 1.0]).expect_err("NaN position");
        assert_eq!(err.index, 0);
        assert_eq!(err.observed, None);
    }

    #[test]
    fn class_b_helper() {
        let dense = [2.0, -1.0, 0.5];
        let cg = [2.0 + 1e-10, -1.0, 0.5];
        let ok = ClassBInputs {
            converged: true,
            relative_residual: 1e-9,
            tol_cg: 1e-8,
            beta_cg: &cg,
            beta_dense: &dense,
        };
        let report = compare_class_b(&ok).expect("all three parts");
        assert_eq!(report.n, 3);
        let err = compare_class_b(&ClassBInputs {
            converged: false,
            ..ok.clone()
        })
        .expect_err("not converged");
        assert_eq!(err.index, 0);
        assert!(err.rust.contains("converged = false"));
        // A boolean part has no observed number and no bound (KI-P1: no 0 / 1 stand-in).
        assert_eq!((err.observed, err.limit), (None, None));
        let err = compare_class_b(&ClassBInputs {
            relative_residual: 2e-8,
            ..ok.clone()
        })
        .expect_err("residual above tol_cg");
        assert_eq!(
            (err.index, err.observed, err.limit),
            (0, Some(2e-8), Some(1e-8))
        );
        let far = [2.0 + 1e-8, -1.0, 0.5];
        let err = compare_class_b(&ClassBInputs {
            beta_cg: &far,
            ..ok.clone()
        })
        .expect_err("dense disagreement");
        assert_eq!(err.index, 0);
        assert!(err.observed.is_some_and(|d| d > 1e-9) && err.limit == Some(CLASS_B_DENSE_REL_TOL));
        // A NaN tol_cg fails closed.
        assert!(compare_class_b(&ClassBInputs {
            tol_cg: f64::NAN,
            ..ok
        })
        .is_err());
    }

    #[test]
    fn errors_display() {
        let shown = FixtureError::LengthMismatch {
            name: "y".into(),
            expected_bytes: 16,
            found_bytes: 8,
        }
        .to_string();
        assert!(shown.contains("16 bytes") && shown.contains("holds 8"));
        let shown = Mismatch {
            index: 3,
            rust: "1.0".into(),
            oracle: "2.0".into(),
            observed: Some(1.0),
            limit: Some(1e-12),
        }
        .to_string();
        assert!(shown.starts_with("mismatch at index 3"));
        assert!(shown.contains("observed 1e0") && shown.contains("limit 1e-12"));
        let shown = Mismatch {
            index: 0,
            rust: "converged = false".into(),
            oracle: "converged = true".into(),
            observed: None,
            limit: None,
        }
        .to_string();
        assert!(shown.contains("observed n/a, limit n/a"));
        let boxed: Box<dyn std::error::Error> = Box::new(FixtureError::NotSynthetic {
            found: "real".into(),
        });
        assert!(boxed.to_string().contains("DR-A11"));
    }
}
