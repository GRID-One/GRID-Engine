//! Conversions between domain values and their stored SQL form (work package P1-02 D5).
//!
//! - **Ids** are `u64` in the domain and `INTEGER` (signed 64-bit) in SQLite. Writing checks the
//!   range: a value above `i64::MAX` is [`PersistenceError::IdOutOfRange`], a programming error.
//!   Reading checks the sign: a negative stored id is [`PersistenceError::StateCorrupt`], never
//!   wrapped or clamped.
//! - **Instants** are [`UtcInstant`] whole seconds, stored unchanged.
//! - **Booleans** are `INTEGER` 0 or 1; any other stored value is `StateCorrupt`.
//! - **Enum text** is the domain spelling ([`StoredEnum::stored_name`]). Unknown stored text is
//!   `StateCorrupt`, never a default variant (DR-B6; Appendix D item 8).
//! - **u64 values beyond `i64`** (seeds) are canonical decimal text ([`u64_to_text`],
//!   [`u64_from_text`]).

use grid_domain::{
    EngineErrorCode, JobStatus, LockKind, ModelState, StageId, TerminalState, UtcInstant,
};

use crate::error::PersistenceError;

/// How many characters of an unknown stored value a `StateCorrupt` message shows.
const SHOWN_CHARS: usize = 64;

/// Converts an id for binding: `u64 → i64` (work package P1-02 D5).
///
/// # Errors
///
/// [`PersistenceError::IdOutOfRange`] when `value > i64::MAX`.
pub fn id_to_sql(what: &'static str, value: u64) -> Result<i64, PersistenceError> {
    i64::try_from(value).map_err(|_| PersistenceError::IdOutOfRange { what, value })
}

/// Converts a stored id: `i64 → u64` (work package P1-02 D5).
///
/// # Errors
///
/// [`PersistenceError::StateCorrupt`] when the stored value is negative.
pub fn id_from_sql(what: &'static str, value: i64) -> Result<u64, PersistenceError> {
    u64::try_from(value).map_err(|_| {
        PersistenceError::state_corrupt(format!("{what} holds the negative value {value}"))
    })
}

/// Converts a stored `u32` quantity (a feature-schema or scoring-profile version, a draw count).
///
/// # Errors
///
/// [`PersistenceError::StateCorrupt`] when the stored value is negative or above `u32::MAX`.
pub fn u32_from_sql(what: &'static str, value: i64) -> Result<u32, PersistenceError> {
    u32::try_from(value).map_err(|_| {
        PersistenceError::state_corrupt(format!("{what} holds {value}, outside 0..=4294967295"))
    })
}

/// Converts a boolean for binding: `false → 0`, `true → 1`.
pub const fn bool_to_sql(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

/// Converts a stored boolean.
///
/// # Errors
///
/// [`PersistenceError::StateCorrupt`] for any value other than 0 or 1.
pub fn bool_from_sql(what: &'static str, value: i64) -> Result<bool, PersistenceError> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        other => Err(PersistenceError::state_corrupt(format!(
            "{what} holds {other}, not a boolean 0 or 1"
        ))),
    }
}

/// Converts an instant for binding: whole seconds since the Unix epoch (engine-spec §4.5).
pub const fn instant_to_sql(instant: UtcInstant) -> i64 {
    instant.unix_seconds()
}

/// Converts a stored instant.
pub const fn instant_from_sql(seconds: i64) -> UtcInstant {
    UtcInstant::from_unix_seconds(seconds)
}

/// Writes a `u64` as canonical decimal text, the stored form of a seed (`0002_backbone.sql`:
/// digits only, no leading zero, at most `18446744073709551615`).
pub fn u64_to_text(value: u64) -> String {
    value.to_string()
}

/// Reads a `u64` from canonical decimal text.
///
/// # Errors
///
/// [`PersistenceError::StateCorrupt`] when the text is empty, has a sign, a leading zero (other
/// than `0` itself) or a non-digit, or exceeds `u64::MAX`.
pub fn u64_from_text(what: &'static str, text: &str) -> Result<u64, PersistenceError> {
    let canonical = !text.is_empty()
        && text.bytes().all(|b| b.is_ascii_digit())
        && (text == "0" || !text.starts_with('0'));
    let parsed = if canonical {
        text.parse::<u64>().ok()
    } else {
        None
    };
    parsed.ok_or_else(|| {
        PersistenceError::state_corrupt(format!(
            "{what} holds {}, not a canonical decimal u64",
            shown(text)
        ))
    })
}

/// A domain enum stored as text in a `CHECK … IN` column (work package P1-02 D5).
///
/// The stored spelling is the domain's own: `spec_name()` for [`StageId`], [`JobStatus`],
/// [`TerminalState`] and [`ModelState`], [`EngineErrorCode::name`] for error codes, and the
/// variant name for [`LockKind`]. [`StoredEnum::ALL`] lists every value, so the `CHECK` lists of
/// `0002_backbone.sql` can be tested against it (`tests/schema_rules.rs`).
pub trait StoredEnum: Sized + Copy + PartialEq + 'static {
    /// What the value is, for error messages, for example `job status`.
    const WHAT: &'static str;
    /// Every value, in the domain's declaration order.
    const ALL: &'static [Self];

    /// The stored spelling.
    fn stored_name(self) -> &'static str;

    /// Reads a stored spelling.
    ///
    /// # Errors
    ///
    /// [`PersistenceError::StateCorrupt`] when `text` is not exactly the stored spelling of one
    /// value: matching is exact and case-sensitive, and nothing falls back to a default.
    fn from_stored(text: &str) -> Result<Self, PersistenceError> {
        Self::ALL
            .iter()
            .copied()
            .find(|value| value.stored_name() == text)
            .ok_or_else(|| {
                PersistenceError::state_corrupt(format!(
                    "unknown {} {} in the store",
                    Self::WHAT,
                    shown(text)
                ))
            })
    }
}

impl StoredEnum for StageId {
    const WHAT: &'static str = "job type";
    const ALL: &'static [Self] = &StageId::ALL;

    fn stored_name(self) -> &'static str {
        self.spec_name()
    }
}

impl StoredEnum for JobStatus {
    const WHAT: &'static str = "job status";
    const ALL: &'static [Self] = &JobStatus::ALL;

    fn stored_name(self) -> &'static str {
        self.spec_name()
    }
}

impl StoredEnum for TerminalState {
    const WHAT: &'static str = "terminal state";
    const ALL: &'static [Self] = &TerminalState::ALL;

    fn stored_name(self) -> &'static str {
        self.spec_name()
    }
}

impl StoredEnum for ModelState {
    const WHAT: &'static str = "model state";
    const ALL: &'static [Self] = &ModelState::ALL;

    fn stored_name(self) -> &'static str {
        self.spec_name()
    }
}

impl StoredEnum for EngineErrorCode {
    const WHAT: &'static str = "error code";
    const ALL: &'static [Self] = &EngineErrorCode::ALL;

    fn stored_name(self) -> &'static str {
        self.name()
    }
}

/// Every [`LockKind`], in declaration order (`grid-domain` declares no list of its own).
const LOCK_KINDS: [LockKind; 3] = [LockKind::Thursday, LockKind::Sunday, LockKind::Operational];

impl StoredEnum for LockKind {
    const WHAT: &'static str = "lock kind";
    const ALL: &'static [Self] = &LOCK_KINDS;

    fn stored_name(self) -> &'static str {
        // An exhaustive match, so a new domain variant fails to compile here rather than being
        // stored under a missing spelling.
        match self {
            LockKind::Thursday => "Thursday",
            LockKind::Sunday => "Sunday",
            LockKind::Operational => "Operational",
        }
    }
}

/// Stored text, escaped, quoted and cut to [`SHOWN_CHARS`] characters for a message.
fn shown(text: &str) -> String {
    let cut: String = text.chars().take(SHOWN_CHARS).collect();
    let suffix = if text.chars().count() > SHOWN_CHARS {
        "…"
    } else {
        ""
    };
    format!("{:?}{suffix}", cut)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u64_text_is_canonical() {
        assert_eq!(u64_from_text("seed", "0").ok(), Some(0));
        assert_eq!(
            u64_from_text("seed", &u64_to_text(u64::MAX)).ok(),
            Some(u64::MAX)
        );
        for bad in ["", "-1", "+1", "01", "1.0", " 1", "18446744073709551616"] {
            assert!(
                matches!(
                    u64_from_text("seed", bad),
                    Err(PersistenceError::StateCorrupt { .. })
                ),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn lock_kinds_cover_the_domain_display() {
        for kind in LOCK_KINDS {
            assert_eq!(kind.stored_name(), kind.to_string());
        }
    }
}
