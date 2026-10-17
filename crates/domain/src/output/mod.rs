//! The `grid.output` contract records (`docs/03-contracts/engine-output-contract.md`; engine-spec
//! §5.5, §5.6, §6.7).
//!
//! Every record the engine publishes is defined here: the player-week projection and its groups
//! ([`projection`]), the explanation record and GRID signal block ([`explanation`]), the team-game
//! environment ([`environment`]), the projection change log ([`change`]) and the report skeletons
//! ([`reports`]). File outputs have a machine-readable schema generated from these definitions
//! (engine-spec §5.5 rule 2; the artifact accompanies the first file output, P1-10).
//!
//! Field rules (engine-spec §5.5): units are explicit; probabilities and shares lie in `[0, 1]`;
//! fantasy points are per profile; GRID quantities are in `dV` (expected drive points per play)
//! and are never mixed with fantasy points in one field; a missing value is `Option::None` with a
//! declared reason, never `0` (KI-P1); filtered, smoothed and one-step predictive estimates are
//! separately named and never aliased; a record that violates a §3.9 invariant is a typed failure
//! and is not emitted. No `#[serde(deny_unknown_fields)]`: readers tolerate MINOR additions
//! (engine-output-contract §9).

pub mod change;
pub mod environment;
pub mod explanation;
pub mod projection;
pub mod reports;

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ids::{ContractId, ContractVersion};

pub use change::*;
pub use environment::*;
pub use explanation::*;
pub use projection::*;
pub use reports::*;

/// The contract id every record, report and export sidecar carries: `grid.output`
/// (engine-output-contract §9).
pub const GRID_OUTPUT_CONTRACT_ID: &str = "grid.output";

/// The contract version this crate implements: `grid.output` 1.0 (engine-output-contract §9; the
/// version line fixed by P1-01). A consumer-facing reader MUST reject an unknown MAJOR.
pub const GRID_OUTPUT_CONTRACT_VERSION: ContractVersion = ContractVersion::new(1, 0);

/// The contract id as a typed [`ContractId`].
pub fn grid_output_contract_id() -> ContractId {
    ContractId(GRID_OUTPUT_CONTRACT_ID.to_owned())
}

/// The projection horizon (engine-output-contract §3.1; DR-C4). Weekly is the primary contract;
/// rest-of-season and preseason are derived products (sums of weekly draws) reported as
/// diagnostics. A typed field, never a `week = 0` sentinel (KI-A4). **Closed** enum. Never null.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Horizon {
    /// One game week.
    Week,
    /// The remaining weeks of the season, summed.
    RestOfSeason,
    /// The whole season, projected before week 1.
    Preseason,
}

impl fmt::Display for Horizon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Horizon::Week => "Week",
            Horizon::RestOfSeason => "RestOfSeason",
            Horizon::Preseason => "Preseason",
        })
    }
}

/// The projected positions, the primary accuracy population (engine-output-contract §2). K and
/// DST, if added in Phase 2, are separate record types. Serialized as `QB`, `RB`, `WR`, `TE`.
/// **Closed** enum. Never null.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Position {
    /// Quarterback.
    Qb,
    /// Running back.
    Rb,
    /// Wide receiver.
    Wr,
    /// Tight end.
    Te,
}

impl Position {
    /// Every position, in the order of engine-output-contract §3.2.
    pub const ALL: [Position; 4] = [Position::Qb, Position::Rb, Position::Wr, Position::Te];

    /// The serialized label.
    pub const fn label(self) -> &'static str {
        match self {
            Position::Qb => "QB",
            Position::Rb => "RB",
            Position::Wr => "WR",
            Position::Te => "TE",
        }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_identity() {
        assert_eq!(grid_output_contract_id().0, "grid.output");
        assert_eq!(GRID_OUTPUT_CONTRACT_VERSION, ContractVersion::new(1, 0));
        assert_eq!(Position::Qb.to_string(), "QB");
        assert_eq!(Horizon::RestOfSeason.to_string(), "RestOfSeason");
    }
}
