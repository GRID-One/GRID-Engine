//! Instants (engine-spec §4.5).
//!
//! Every required timestamp of §4.5 (source publication, retrieval, feature computation,
//! projection, benchmark, kickoff and the lock itself) is a [`UtcInstant`]. The engine never
//! reads a clock in this crate; `grid-domain` holds values only.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A point in time as **whole seconds since the Unix epoch, UTC**.
///
/// Units: seconds. Negative values are instants before 1970 and are representable but never
/// produced by the engine. Never null where a timestamp is required; a record that lacks a
/// required timestamp is a typed error (`ContractError::MissingTimestamp`), never an epoch default.
/// Ordered chronologically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct UtcInstant(pub i64);

impl UtcInstant {
    /// Builds an instant from whole seconds since the Unix epoch.
    pub const fn from_unix_seconds(seconds: i64) -> Self {
        Self(seconds)
    }

    /// Whole seconds since the Unix epoch.
    pub const fn unix_seconds(self) -> i64 {
        self.0
    }
}

impl fmt::Display for UtcInstant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}s", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instants_order_chronologically() {
        let earlier = UtcInstant::from_unix_seconds(1_700_000_000);
        let later = UtcInstant::from_unix_seconds(1_700_000_001);
        assert!(earlier < later);
        assert_eq!(later.unix_seconds(), 1_700_000_001);
        assert_eq!(earlier.to_string(), "1700000000s");
    }
}
