//! `grid-domain` — IDs, types, errors, DTOs. One external dependency: `serde` derives (ADR-014).
//!
//! The lowest crate of the GRID engine workspace. Its boundary is fixed by `docs/CLAUDE.md`
//! ("Crate Boundaries", rule 4, which follows ADR-014, accepted 2026-10-19), `docs/02-adr/001-repo-bootstrap-decisions.md`
//! (the crate set and the frozen verify chain), `docs/02-adr/011-engine-only-pivot.md` (engine
//! only: a library API, a headless CLI and versioned outputs, no UI) and
//! `docs/02-adr/014-domain-serialization-boundary.md` (`serde` with `derive` is the only external
//! dependency; `Display` and `std::error::Error` are written by hand; format crates such as
//! `serde_json` live in `grid-pipeline` and `grid-synth`).
//!
//! # What this crate contains
//!
//! | Module | Content | Governing text |
//! |---|---|---|
//! | [`ids`] | canonical keys (`PlayerKey`, `TeamKey`, `GameId`, `PlayKey`, `DriveKey`), `Season`, `Week`, `Slot` and every version identifier | engine-spec §4.4.1, §8.9 |
//! | [`time`] | `UtcInstant`, whole seconds since the Unix epoch | engine-spec §4.5 |
//! | [`asof`] | the `AsOf` information model with its publication-lag axis | engine-spec §4.5; `docs/05-model-specs/evaluation-and-leakage.md` §4.1 |
//! | [`error`] | the typed error vocabulary: `EngineErrorCode`, `EngineError`, `ContractError`, `LeakageError`, `OutputInvariantError` | engine-spec §6.8 rule 4, §8.10; plays-contract §11; engine-output-contract §3.9 |
//! | [`status`] | `TerminalState` with its exit codes, `JobStatus`, `StageId`, `RunOutcome`, `TrainingStatus` | engine-spec §8.4, §8.6.4, §8.10; engine-output-contract §13 |
//! | [`plays`] | the `grid.plays` v1 frames and their validator | `docs/03-contracts/plays-contract.md` §7, §9, §11 |
//! | [`output`] | the `grid.output` 1.0 records, reports and the §3.9 invariant checks | `docs/03-contracts/engine-output-contract.md` |
//!
//! # What this crate deliberately does not contain
//!
//! - **No computation.** Nothing here fits, solves, simulates, scores or projects. A validator
//!   checks a stated invariant and returns a typed error; it never coerces, clamps or defaults
//!   (engine-spec §6.8 rules 4 and 12; Appendix D "converting malformed data to a healthy
//!   default").
//! - **No I/O.** No file, database, network or clock access. `UtcInstant` is a value; the engine
//!   reads clocks in the orchestration crate. Serialization *formats* (`serde_json`, CSV) live
//!   outside this crate (ADR-014 decision 3); this crate only derives `Serialize` and
//!   `Deserialize`.
//! - **No Python**, no binding crate, no build script (engine-spec §1.1 item 8, §8.1 rule 1).
//! - **No oracle defect.** `Week` cannot be zero (KI-A4), a missing value is `Option::None` with a
//!   documented reason and never `0` or `NaN` (KI-P1), slots are `(season, week)` tuples and never
//!   a bare week (KI-NEW-Z61, KI-NEW-W1), market quotes carry a publication time and no default
//!   (KI-NEW-Z71), identity of keys is structural and never text-order-sensitive (KI-NEW-C1).
//!
//! Every public item is documented; every contract field states its units, nullability and, for
//! an enum, whether it is open or closed (engine-output-contract §1 rule 3).
//!
//! The root re-exports every public item of every module, so `grid_domain::PlayerKey`,
//! `grid_domain::asof::AsOf` and `grid_domain::output::PlayerWeekProjection` all resolve.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod asof;
pub mod error;
pub mod ids;
pub mod output;
pub mod plays;
pub mod status;
pub mod time;

pub use asof::*;
pub use error::*;
pub use ids::*;
pub use output::*;
pub use plays::*;
pub use status::*;
pub use time::*;
