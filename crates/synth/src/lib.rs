//! `grid-synth` — the synthetic world: the DR-D29 parity-fixture reader, the planted truth and
//! the legacy v0 world loader.
//!
//! # Boundary
//!
//! A **numerical crate**: synchronous, with no Tokio or SQLx dependency (engine-spec §8.1 rule 3;
//! `docs/CLAUDE.md` "Crate Boundaries"). Its only dependencies are `grid-domain` and `serde_json`,
//! the latter being the first use of an already-declared workspace dependency for the fixture
//! manifest (`docs/03-contracts/parity-fixture-contract.md` §2, §11; DR-D29). The arrays are
//! decoded with `std` alone. **`grid-models` MUST NOT depend on this crate** (engine-spec §8.1
//! rule 3; `docs/05-model-specs/synthetic-world.md` §10.1); the structural test in
//! `crates/pipeline/tests/` checks normal and build dependencies. Whether a numerical crate's parity
//! tests may take it as a dev-dependency is an open owner item recorded by P1-01. The
//! crate set itself is ADR-011 D11 (`docs/02-adr/011-engine-only-pivot.md`, DR-A8).
//!
//! Nothing here invokes Python, reads a network or a clock, or coerces malformed data to a default:
//! every failure is a typed error (engine-spec §1.1 item 8, §6.8 rule 4; Appendix D).
//!
//! # What this crate contains (P1-01)
//!
//! | Module | Content | Governing text |
//! |---|---|---|
//! | [`fixture`] | the manifest and array reader of the DR-D29 format, typed and ragged views, and the exact, A, A′ and B tolerance helpers | parity-fixture-contract §2–§4, §6, §11, §12 |
//! | [`truth`] | [`truth::SyntheticTruth`], the planted truth bundle that estimator code cannot reach | plays-contract §6.1, D-13; synthetic-world.md §4.10 |
//! | [`legacy_v0`] | the columnar v0 world, `from_legacy_v0` to the v1 frames, `to_v0_columns` back, and `load_world` from a `synthetic-world` case | plays-contract §2.1, §4, §6, §11; synthetic-world.md §5.3 (a), §10.3 S-1 |
//!
//! # What this crate does not contain yet
//!
//! The Rust-native generator (`synth::generator`, the §4 model with an injectable draw source), the
//! draw-tape replay (`synth::tape`) and the profiles (`synth::profiles`) are **P1-12's**
//! (synthetic-world.md §10.1); the committed `synthetic-world` fixtures are P1-05's. This package
//! delivers the loader and proves it on hand-built frames only (P1-01 "Oracle parity targets").

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod fixture;
pub mod legacy_v0;
pub mod truth;
