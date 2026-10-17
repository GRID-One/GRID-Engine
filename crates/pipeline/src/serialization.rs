//! JSON record serialization under the `grid.output` contract (engine-output-contract §8–§10;
//! engine-spec §5.5 rules 2 and 5, §12.6; ADR-014 decision 3).
//!
//! Every record, report and export sidecar the engine writes as JSON is wrapped in a
//! [`RecordEnvelope`] that carries the contract id and version. The reader, [`from_json`], rejects
//! an unknown MAJOR with `EngineErrorCode::UnsupportedVersion` (engine-output-contract §9 "A
//! consumer-facing reader MUST reject an unknown MAJOR") and tolerates any MINOR, because records
//! never carry `#[serde(deny_unknown_fields)]` and an additive field is ignored.
//!
//! Precision: `serde_json` writes an `f64` in its shortest round-tripping form, and the reader
//! parses it back bit-exactly **only because this crate enables `serde_json`'s `float_roundtrip`
//! feature** (`crates/pipeline/Cargo.toml`). Without that feature `serde_json`'s parser is
//! best-effort on 16- and 17-digit significands and lands one ULP away, which engine-spec §12.6
//! ("precision-preserving serialization round trips … JSON exports preserve bits") forbids. The
//! tripwire is `tests/output_round_trip.rs::json_round_trip_preserves_computed_f64_bit_for_bit`,
//! which fails if the feature is ever dropped; the unit test below pins one such value at module
//! level too.
//!
//! Non-finite values: `serde_json` writes a NaN or an infinity as `null`. On read, a `null` in an
//! `f64` field is a `ContractViolation`, but a `null` in an `Option<f64>` field is a silent
//! `None`: the value would change from non-finite (a §3.9 invariant violation,
//! engine-output-contract §3.9 "no NaN or infinity anywhere") into a declared-missing value, which
//! is the prohibited conversion of malformed data to a healthy default (engine-spec Appendix D;
//! KI-P1). Every contract record has such fields (`PriorBlock`, `GridSignalBlock::form`,
//! `RegimeFlags::changepoint_z`, the `ExplanationRecord` adjustments, …). So the **writer refuses**:
//! [`to_json`] walks the record before serializing it and fails with `EngineErrorCode::NonFinite`,
//! naming the path of the offending field, whenever any `f32` or `f64` in it is NaN or infinite.
//! A producer's own `validate()` is the first line (engine-spec §5.5 "It is not emitted"); this
//! writer is the last, so that no path, validated or not, can write a non-finite value as `null`.

use std::fmt;

use grid_domain::{
    ContractId, ContractVersion, EngineError, EngineErrorCode, PredictionVersionId,
    ScoringProfileRef, GRID_OUTPUT_CONTRACT_ID, GRID_OUTPUT_CONTRACT_VERSION,
};
use serde::de::DeserializeOwned;
use serde::ser::{
    SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant,
};
use serde::{Deserialize, Serialize, Serializer};

/// The JSON envelope of one record (engine-output-contract §9 "Every record, report and export
/// sidecar carries it"). Never null.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordEnvelope<T> {
    /// The contract id, `grid.output`.
    pub contract: ContractId,
    /// The contract version the record conforms to, `MAJOR.MINOR`.
    pub version: ContractVersion,
    /// The record.
    pub record: T,
}

/// The header of an envelope, read before the record so that the version check happens first.
#[derive(Deserialize)]
struct EnvelopeHeader {
    contract: ContractId,
    version: ContractVersion,
}

/// Serializes `record` inside a [`RecordEnvelope`] stamped with the `grid.output` contract id and
/// the version this crate implements ([`GRID_OUTPUT_CONTRACT_VERSION`]).
///
/// - A NaN or infinite `f32` or `f64` anywhere in the record is `EngineErrorCode::NonFinite`, with
///   the field's path (`record.explanation.grid.form`, `record.items.3.weekly_credit`, …) in the
///   message; nothing is written (engine-output-contract §3.9; see the module documentation).
/// - `EngineErrorCode::Internal` only when `serde_json` cannot serialize the value (a map with a
///   non-string key, for example); no contract record has such a shape.
pub fn to_json<T: Serialize>(record: &T) -> Result<String, EngineError> {
    let mut path = vec!["record".to_owned()];
    record
        .serialize(FiniteCheck { path: &mut path })
        .map_err(EngineError::from)?;
    let envelope = RecordEnvelope {
        contract: ContractId(GRID_OUTPUT_CONTRACT_ID.to_owned()),
        version: GRID_OUTPUT_CONTRACT_VERSION,
        record,
    };
    serde_json::to_string(&envelope).map_err(|e| {
        EngineError::new(
            EngineErrorCode::Internal,
            format!("record serialization failed: {e}"),
        )
    })
}

/// Reads a [`RecordEnvelope`] from JSON, checking the contract id and version before the record.
///
/// - A contract id other than `grid.output` is `EngineErrorCode::ContractViolation`.
/// - A MAJOR other than the one this crate implements is `EngineErrorCode::UnsupportedVersion`
///   (engine-output-contract §9). Any MINOR is accepted: an additive field the reader does not
///   know is ignored, and a field the reader knows but the writer omitted is a
///   `ContractViolation` from the record's own deserialization.
/// - Malformed JSON or a record that does not deserialize is `EngineErrorCode::ContractViolation`.
pub fn from_json<T: DeserializeOwned>(json: &str) -> Result<RecordEnvelope<T>, EngineError> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|e| {
        EngineError::new(
            EngineErrorCode::ContractViolation,
            format!("record is not valid JSON: {e}"),
        )
    })?;
    let header: EnvelopeHeader = serde_json::from_value(value.clone()).map_err(|e| {
        EngineError::new(
            EngineErrorCode::ContractViolation,
            format!("record envelope lacks a contract id or version: {e}"),
        )
    })?;
    if header.contract.0 != GRID_OUTPUT_CONTRACT_ID {
        return Err(EngineError::new(
            EngineErrorCode::ContractViolation,
            format!(
                "record contract is `{}`, this reader reads `{GRID_OUTPUT_CONTRACT_ID}`",
                header.contract
            ),
        ));
    }
    if header.version.major != GRID_OUTPUT_CONTRACT_VERSION.major {
        return Err(EngineError::new(
            EngineErrorCode::UnsupportedVersion,
            format!(
                "record contract version {} has MAJOR {}; this reader implements MAJOR {}",
                header.version, header.version.major, GRID_OUTPUT_CONTRACT_VERSION.major
            ),
        ));
    }
    serde_json::from_value(value).map_err(|e| {
        EngineError::new(
            EngineErrorCode::ContractViolation,
            format!("record does not conform to the contract: {e}"),
        )
    })
}

// ---------------------------------------------------------------------------------------------
// The non-finite guard: a serializer that writes nothing and fails on the first NaN or infinity
// ---------------------------------------------------------------------------------------------

/// The failure of the non-finite walk.
#[derive(Debug)]
enum FiniteCheckError {
    /// A NaN or infinite float at `path` (dot-separated field names and sequence indices, from
    /// `record`).
    NonFinite { path: String, value: f64 },
    /// A `Serialize` implementation raised its own error (none of the contract types does).
    Custom(String),
}

impl fmt::Display for FiniteCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FiniteCheckError::NonFinite { path, value } => {
                write!(
                    f,
                    "non-finite value {value} at `{path}`; the record is not written"
                )
            }
            FiniteCheckError::Custom(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for FiniteCheckError {}

impl serde::ser::Error for FiniteCheckError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        FiniteCheckError::Custom(msg.to_string())
    }
}

impl From<FiniteCheckError> for EngineError {
    fn from(error: FiniteCheckError) -> Self {
        let code = match error {
            FiniteCheckError::NonFinite { .. } => EngineErrorCode::NonFinite,
            FiniteCheckError::Custom(_) => EngineErrorCode::Internal,
        };
        EngineError::new(code, error.to_string())
    }
}

/// A `serde` serializer that produces no output and fails on the first non-finite float, carrying
/// the path of the value being visited so the error names the field.
struct FiniteCheck<'a> {
    path: &'a mut Vec<String>,
}

impl<'a> FiniteCheck<'a> {
    fn check(self, value: f64) -> Result<(), FiniteCheckError> {
        if value.is_finite() {
            Ok(())
        } else {
            Err(FiniteCheckError::NonFinite {
                path: self.path.join("."),
                value,
            })
        }
    }

    fn descend<T: ?Sized + Serialize>(
        self,
        segment: String,
        value: &T,
    ) -> Result<(), FiniteCheckError> {
        self.path.push(segment);
        let result = value.serialize(FiniteCheck { path: self.path });
        self.path.pop();
        result
    }

    fn compound(self) -> FiniteCheckCompound<'a> {
        FiniteCheckCompound {
            path: self.path,
            index: 0,
        }
    }
}

impl<'a> Serializer for FiniteCheck<'a> {
    type Ok = ();
    type Error = FiniteCheckError;
    type SerializeSeq = FiniteCheckCompound<'a>;
    type SerializeTuple = FiniteCheckCompound<'a>;
    type SerializeTupleStruct = FiniteCheckCompound<'a>;
    type SerializeTupleVariant = FiniteCheckCompound<'a>;
    type SerializeMap = FiniteCheckCompound<'a>;
    type SerializeStruct = FiniteCheckCompound<'a>;
    type SerializeStructVariant = FiniteCheckCompound<'a>;

    fn serialize_bool(self, _: bool) -> Result<(), FiniteCheckError> {
        Ok(())
    }
    fn serialize_i8(self, _: i8) -> Result<(), FiniteCheckError> {
        Ok(())
    }
    fn serialize_i16(self, _: i16) -> Result<(), FiniteCheckError> {
        Ok(())
    }
    fn serialize_i32(self, _: i32) -> Result<(), FiniteCheckError> {
        Ok(())
    }
    fn serialize_i64(self, _: i64) -> Result<(), FiniteCheckError> {
        Ok(())
    }
    fn serialize_u8(self, _: u8) -> Result<(), FiniteCheckError> {
        Ok(())
    }
    fn serialize_u16(self, _: u16) -> Result<(), FiniteCheckError> {
        Ok(())
    }
    fn serialize_u32(self, _: u32) -> Result<(), FiniteCheckError> {
        Ok(())
    }
    fn serialize_u64(self, _: u64) -> Result<(), FiniteCheckError> {
        Ok(())
    }
    fn serialize_f32(self, v: f32) -> Result<(), FiniteCheckError> {
        self.check(f64::from(v))
    }
    fn serialize_f64(self, v: f64) -> Result<(), FiniteCheckError> {
        self.check(v)
    }
    fn serialize_char(self, _: char) -> Result<(), FiniteCheckError> {
        Ok(())
    }
    fn serialize_str(self, _: &str) -> Result<(), FiniteCheckError> {
        Ok(())
    }
    fn serialize_bytes(self, _: &[u8]) -> Result<(), FiniteCheckError> {
        Ok(())
    }
    fn serialize_none(self) -> Result<(), FiniteCheckError> {
        Ok(())
    }
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<(), FiniteCheckError> {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<(), FiniteCheckError> {
        Ok(())
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<(), FiniteCheckError> {
        Ok(())
    }
    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
    ) -> Result<(), FiniteCheckError> {
        Ok(())
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<(), FiniteCheckError> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<(), FiniteCheckError> {
        self.descend(variant.to_owned(), value)
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, FiniteCheckError> {
        Ok(self.compound())
    }
    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, FiniteCheckError> {
        Ok(self.compound())
    }
    fn serialize_tuple_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleStruct, FiniteCheckError> {
        Ok(self.compound())
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant, FiniteCheckError> {
        self.path.push(variant.to_owned());
        Ok(FiniteCheckCompound {
            path: self.path,
            index: 0,
        })
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, FiniteCheckError> {
        Ok(self.compound())
    }
    fn serialize_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStruct, FiniteCheckError> {
        Ok(self.compound())
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant, FiniteCheckError> {
        self.path.push(variant.to_owned());
        Ok(FiniteCheckCompound {
            path: self.path,
            index: 0,
        })
    }
}

/// The compound-value state of [`FiniteCheck`]: the path so far and the index of the next
/// element. A tuple or struct variant pushed its variant name onto the path when it began and
/// pops it in `end`, so that a path reads `record.event.DataFetchCompleted.duration_ms`.
struct FiniteCheckCompound<'a> {
    path: &'a mut Vec<String>,
    index: usize,
}

impl FiniteCheckCompound<'_> {
    fn element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), FiniteCheckError> {
        let segment = self.index.to_string();
        self.index += 1;
        FiniteCheck { path: self.path }.descend(segment, value)
    }

    fn named<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), FiniteCheckError> {
        FiniteCheck { path: self.path }.descend(key.to_owned(), value)
    }

    fn end_variant(self) -> Result<(), FiniteCheckError> {
        self.path.pop();
        Ok(())
    }
}

impl SerializeSeq for FiniteCheckCompound<'_> {
    type Ok = ();
    type Error = FiniteCheckError;
    fn serialize_element<T: ?Sized + Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), FiniteCheckError> {
        self.element(value)
    }
    fn end(self) -> Result<(), FiniteCheckError> {
        Ok(())
    }
}

impl SerializeTuple for FiniteCheckCompound<'_> {
    type Ok = ();
    type Error = FiniteCheckError;
    fn serialize_element<T: ?Sized + Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), FiniteCheckError> {
        self.element(value)
    }
    fn end(self) -> Result<(), FiniteCheckError> {
        Ok(())
    }
}

impl SerializeTupleStruct for FiniteCheckCompound<'_> {
    type Ok = ();
    type Error = FiniteCheckError;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), FiniteCheckError> {
        self.element(value)
    }
    fn end(self) -> Result<(), FiniteCheckError> {
        Ok(())
    }
}

impl SerializeTupleVariant for FiniteCheckCompound<'_> {
    type Ok = ();
    type Error = FiniteCheckError;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), FiniteCheckError> {
        self.element(value)
    }
    fn end(self) -> Result<(), FiniteCheckError> {
        self.end_variant()
    }
}

impl SerializeMap for FiniteCheckCompound<'_> {
    type Ok = ();
    type Error = FiniteCheckError;
    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), FiniteCheckError> {
        FiniteCheck { path: self.path }.descend(format!("key[{}]", self.index), key)
    }
    fn serialize_value<T: ?Sized + Serialize>(
        &mut self,
        value: &T,
    ) -> Result<(), FiniteCheckError> {
        self.element(value)
    }
    fn end(self) -> Result<(), FiniteCheckError> {
        Ok(())
    }
}

impl SerializeStruct for FiniteCheckCompound<'_> {
    type Ok = ();
    type Error = FiniteCheckError;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), FiniteCheckError> {
        self.named(key, value)
    }
    fn end(self) -> Result<(), FiniteCheckError> {
        Ok(())
    }
}

impl SerializeStructVariant for FiniteCheckCompound<'_> {
    type Ok = ();
    type Error = FiniteCheckError;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), FiniteCheckError> {
        self.named(key, value)
    }
    fn end(self) -> Result<(), FiniteCheckError> {
        self.end_variant()
    }
}

/// One column of an exported data file (engine-output-contract §8 rule 3 "the column schema").
/// Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ColumnSpec {
    /// The column name, fixed per contract version.
    pub name: String,
    /// The column's unit, as the contract states it for the field (`probability`,
    /// `fantasy points (profile)`, `dV`, `identifier`, `seconds since the Unix epoch`, …).
    /// Open vocabulary; never empty.
    pub unit: String,
    /// Whether the column may be empty. An empty cell is a missing value with a declared reason
    /// in the contract, never `0` (KI-P1).
    pub nullable: bool,
}

/// The JSON sidecar every export carries (engine-output-contract §8 rule 3). Written through
/// [`to_json`], so the envelope carries the contract id; this record carries the version the data
/// file conforms to. Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExportSidecar {
    /// The contract version of the data file.
    pub contract: ContractVersion,
    /// The column schema of the data file, in file order.
    pub columns: Vec<ColumnSpec>,
    /// The prediction version exported.
    pub prediction_version_id: PredictionVersionId,
    /// The scoring profile of the exported fantasy points.
    pub scoring_profile: ScoringProfileRef,
    /// The number of data rows, excluding the header.
    pub row_count: u64,
    /// The sha256 of the data file, as 64 lowercase hex characters; never empty
    /// (engine-output-contract §8 rule 3 "row count and the sha256 of the data file"). Required
    /// unconditionally: a sidecar without it is not a sidecar the contract admits, so the field
    /// is not nullable even though no producer exists before P1-10.
    pub data_sha256: String,
    /// The attribution strings of every upstream source whose data fed the run
    /// (engine-output-contract §8 rule 6; engine-spec §4.8).
    pub attribution: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_stamps_contract_and_version() {
        let json = to_json(&42u8).unwrap();
        assert_eq!(
            json,
            "{\"contract\":\"grid.output\",\"version\":{\"major\":1,\"minor\":0},\"record\":42}"
        );
        let back: RecordEnvelope<u8> = from_json(&json).unwrap();
        assert_eq!(back.record, 42);
        assert_eq!(back.version, GRID_OUTPUT_CONTRACT_VERSION);
    }

    #[test]
    fn reader_rejects_unknown_major_and_foreign_contract() {
        let major_2 =
            "{\"contract\":\"grid.output\",\"version\":{\"major\":2,\"minor\":0},\"record\":1}";
        let err = from_json::<u8>(major_2).unwrap_err();
        assert_eq!(err.code, EngineErrorCode::UnsupportedVersion);
        let foreign =
            "{\"contract\":\"grid.plays\",\"version\":{\"major\":1,\"minor\":0},\"record\":1}";
        let err = from_json::<u8>(foreign).unwrap_err();
        assert_eq!(err.code, EngineErrorCode::ContractViolation);
        let err = from_json::<u8>("not json").unwrap_err();
        assert_eq!(err.code, EngineErrorCode::ContractViolation);
        let err = from_json::<u8>("{\"record\":1}").unwrap_err();
        assert_eq!(err.code, EngineErrorCode::ContractViolation);
    }

    #[test]
    fn reader_tolerates_any_minor() {
        let minor_7 =
            "{\"contract\":\"grid.output\",\"version\":{\"major\":1,\"minor\":7},\"record\":1}";
        let back: RecordEnvelope<u8> = from_json(minor_7).unwrap();
        assert_eq!(back.version, ContractVersion::new(1, 7));
    }

    #[test]
    fn finite_f64_values_survive_bit_exactly() {
        // The last value is `43.199999999999996`, a 17-significant-digit shortest form that
        // only a correctly rounded parser (`float_roundtrip`) reads back to the same bits.
        let values = [
            0.1,
            1e-300,
            1.0 / 3.0,
            -2.5e300,
            123_456_789.000_001,
            f64::MIN_POSITIVE,
            36.0 * 1.2,
        ];
        let json = to_json(&values).unwrap();
        let back: RecordEnvelope<[f64; 7]> = from_json(&json).unwrap();
        for (a, b) in values.iter().zip(back.record.iter()) {
            assert_eq!(a.to_bits(), b.to_bits());
        }
    }

    /// A record shaped like the contract's: an optional float, a required float, a nested
    /// sequence of structs, a tuple and an enum with a struct variant.
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Inner {
        weight: f32,
        z: Option<f64>,
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    enum Variant {
        Unit,
        Newtype(f64),
        Tuple(f64, f64),
        Struct { x: f64 },
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Record {
        a: Option<f64>,
        b: f64,
        inner: Vec<Inner>,
        pair: (u8, f64),
        variant: Variant,
    }

    fn finite_record() -> Record {
        Record {
            a: Some(0.5),
            b: 1.5,
            inner: vec![
                Inner {
                    weight: 0.25,
                    z: None,
                },
                Inner {
                    weight: 0.125,
                    z: Some(2.0),
                },
            ],
            pair: (3, 4.0),
            variant: Variant::Struct { x: 0.0625 },
        }
    }

    #[test]
    fn writer_refuses_a_non_finite_value_and_names_its_path() {
        let json = to_json(&finite_record()).unwrap();
        let back: RecordEnvelope<Record> = from_json(&json).unwrap();
        assert_eq!(back.record, finite_record());

        // `Some(NaN)` in an `Option<f64>`: without the guard this would be written as `null`
        // and read back as `None` without an error.
        let mut record = finite_record();
        record.a = Some(f64::NAN);
        let err = to_json(&record).unwrap_err();
        assert_eq!(err.code, EngineErrorCode::NonFinite);
        assert!(err.message.contains("`record.a`"), "{}", err.message);

        let mut record = finite_record();
        record.b = f64::INFINITY;
        let err = to_json(&record).unwrap_err();
        assert_eq!(err.code, EngineErrorCode::NonFinite);
        assert!(err.message.contains("`record.b`"), "{}", err.message);

        let mut record = finite_record();
        record.inner[1].z = Some(f64::NEG_INFINITY);
        let err = to_json(&record).unwrap_err();
        assert_eq!(err.code, EngineErrorCode::NonFinite);
        assert!(
            err.message.contains("`record.inner.1.z`"),
            "{}",
            err.message
        );

        let mut record = finite_record();
        record.inner[0].weight = f32::NAN;
        let err = to_json(&record).unwrap_err();
        assert_eq!(err.code, EngineErrorCode::NonFinite);
        assert!(
            err.message.contains("`record.inner.0.weight`"),
            "{}",
            err.message
        );

        let mut record = finite_record();
        record.pair.1 = f64::NAN;
        let err = to_json(&record).unwrap_err();
        assert!(err.message.contains("`record.pair.1`"), "{}", err.message);

        for (variant, path) in [
            (Variant::Newtype(f64::NAN), "`record.variant.Newtype`"),
            (Variant::Tuple(1.0, f64::NAN), "`record.variant.Tuple.1`"),
            (
                Variant::Struct { x: f64::INFINITY },
                "`record.variant.Struct.x`",
            ),
        ] {
            let mut record = finite_record();
            record.variant = variant;
            let err = to_json(&record).unwrap_err();
            assert_eq!(err.code, EngineErrorCode::NonFinite);
            assert!(err.message.contains(path), "{}", err.message);
        }
        // A unit variant and a finite variant pass, and the path stack unwinds (the second
        // call sees a clean `record.…` prefix).
        let mut record = finite_record();
        record.variant = Variant::Unit;
        to_json(&record).unwrap();
        record.variant = Variant::Tuple(1.0, 2.0);
        to_json(&record).unwrap();
    }
}
