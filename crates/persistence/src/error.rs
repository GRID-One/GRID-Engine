//! The typed failures of the store and their mapping to the engine's error vocabulary (engine-spec
//! §6.8 rule 4; owner ruling R6; work package P1-02 "Implementation constraints").
//!
//! Every failure of the store is a [`PersistenceError`]. [`PersistenceError::engine_error`] turns it
//! into the closed [`EngineError`] the rest of the engine reports, and
//! [`PersistenceError::terminal_state`] gives the terminal state that decides the exit code
//! (engine-output-contract §13, 1.1):
//!
//! | `PersistenceError` | `EngineErrorCode` | Terminal state |
//! |---|---|---|
//! | [`Database`](PersistenceError::Database), [`Io`](PersistenceError::Io), [`Busy`](PersistenceError::Busy) | `Internal` | `DatabaseFailure` (exit 30) |
//! | [`Schema`](PersistenceError::Schema): missing, not initialized, pending, newer | `UnsupportedVersion` | `DatabaseFailure` (exit 30) |
//! | [`Schema`](PersistenceError::Schema): checksum mismatch, dirty, unrecorded, out of order; [`StateCorrupt`](PersistenceError::StateCorrupt) | `StateCorrupt` | `DatabaseFailure` (exit 30) |
//! | [`Conflict`](PersistenceError::Conflict), [`IdOutOfRange`](PersistenceError::IdOutOfRange), [`Constraint`](PersistenceError::Constraint), [`InvalidInput`](PersistenceError::InvalidInput) | `Internal` | none (exit 70) |
//!
//! The last row holds programming errors: every value under a store constraint is validated in
//! Rust first, so a constraint fires only on a defect (P1-02 "Typed failures").
//!
//! **Messages.** No variant carries an absolute path, a host or the raw text of an `sqlx::Error`
//! (engine-output-contract §8 rule 5). Paths are relative to the data directory, SQLite failures
//! carry their numeric result code, and constraint failures carry the message of the constraint
//! this crate's own migrations define.

use std::fmt;
use std::io;

use grid_domain::{EngineError, EngineErrorCode, TerminalState};

/// The longest error message the store keeps, in characters (Unicode scalar values): the bound of
/// the `error_message` CHECKs of `0002_backbone.sql` (work package P1-02 "Typed failures").
pub const MAX_MESSAGE_CHARS: usize = 2000;

/// The SQLite primary result code of a constraint failure (`SQLITE_CONSTRAINT`).
const SQLITE_CONSTRAINT: i64 = 19;
/// The SQLite primary result codes of a busy or locked database (`SQLITE_BUSY`, `SQLITE_LOCKED`).
const SQLITE_BUSY: i64 = 5;
const SQLITE_LOCKED: i64 = 6;
/// The SQLite primary result codes of a damaged file (`SQLITE_CORRUPT`, `SQLITE_NOTADB`).
const SQLITE_CORRUPT: i64 = 11;
const SQLITE_NOTADB: i64 = 26;
/// How many characters of a constraint message a [`PersistenceError::Constraint`] keeps.
const CONSTRAINT_MESSAGE_CHARS: usize = 300;

/// Normalizes an error message before the store writes it (work package P1-02 "Typed failures").
///
/// An empty message becomes the code's name ([`EngineErrorCode::name`]), and a message longer
/// than [`MAX_MESSAGE_CHARS`] characters is cut at that many characters, on a character boundary.
/// The result therefore always satisfies the store's `length(error_message) BETWEEN 1 AND 2000`
/// CHECK, so a failure path can never make its own failure record unwritable.
pub fn normalize_message(code: EngineErrorCode, message: &str) -> String {
    if message.is_empty() {
        return code.name().to_owned();
    }
    truncate_chars(message, MAX_MESSAGE_CHARS).to_owned()
}

/// Returns `error` with its message normalized by [`normalize_message`].
pub fn normalize_error(error: &EngineError) -> EngineError {
    EngineError::new(error.code, normalize_message(error.code, &error.message))
}

/// The longest prefix of `text` with at most `max` characters.
fn truncate_chars(text: &str, max: usize) -> &str {
    match text.char_indices().nth(max) {
        Some((index, _)) => &text[..index],
        None => text,
    }
}

/// Why the store's schema is not usable as it is (ADR-015 D2; owner ruling R3). **Closed** enum
/// within this crate's API: the pipeline matches on it to classify a refusal.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SchemaProblem {
    /// There is no database file. `grid migrate` creates it. Maps to `UnsupportedVersion`.
    Missing {
        /// The database file name, without its directory.
        file: String,
    },
    /// The database exists but holds no schema and no migration record. Maps to
    /// `UnsupportedVersion`.
    NotInitialized,
    /// Migrations embedded in this build are not applied yet. Maps to `UnsupportedVersion`.
    Pending {
        /// The highest applied version; `None` when no migration is applied.
        applied: Option<i64>,
        /// The embedded versions not applied yet, ascending. Never empty.
        pending: Vec<i64>,
    },
    /// The database records a migration this build does not embed: the database is newer than
    /// the binary. Maps to `UnsupportedVersion`.
    Newer {
        /// The lowest applied version this build does not know.
        version: i64,
    },
    /// An applied migration's recorded checksum differs from the embedded one. Maps to
    /// `StateCorrupt`.
    ChecksumMismatch {
        /// The migration whose checksum differs.
        version: i64,
    },
    /// A migration is recorded as failed (`success = false`). Maps to `StateCorrupt`.
    Dirty {
        /// The lowest migration recorded as failed.
        version: i64,
    },
    /// The database holds schema objects but no migration record, so its schema is unknown. Maps
    /// to `StateCorrupt` (work package P1-02: a store whose schema cannot be classified is never
    /// migrated over).
    Unrecorded,
    /// An embedded migration is not applied although a later one is, so the record has a gap. Maps
    /// to `StateCorrupt`.
    OutOfOrder {
        /// The lowest embedded version that is missing below the highest applied one.
        version: i64,
    },
}

impl SchemaProblem {
    /// The engine error code of this problem (ADR-015 D2; owner ruling R6).
    pub const fn code(&self) -> EngineErrorCode {
        match self {
            SchemaProblem::Missing { .. }
            | SchemaProblem::NotInitialized
            | SchemaProblem::Pending { .. }
            | SchemaProblem::Newer { .. } => EngineErrorCode::UnsupportedVersion,
            SchemaProblem::ChecksumMismatch { .. }
            | SchemaProblem::Dirty { .. }
            | SchemaProblem::Unrecorded
            | SchemaProblem::OutOfOrder { .. } => EngineErrorCode::StateCorrupt,
        }
    }
}

impl fmt::Display for SchemaProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SchemaProblem::Missing { file } => {
                write!(f, "there is no database file `{file}`; run `grid migrate`")
            }
            SchemaProblem::NotInitialized => {
                f.write_str("the database has no schema; run `grid migrate`")
            }
            SchemaProblem::Pending { applied, pending } => {
                match applied {
                    Some(version) => write!(f, "the database is at schema version {version}")?,
                    None => f.write_str("the database has no applied migration")?,
                }
                write!(
                    f,
                    " and migrations {pending:?} are pending; run `grid migrate`"
                )
            }
            SchemaProblem::Newer { version } => write!(
                f,
                "the database records migration {version}, which this build does not embed (the \
                 database is newer than the binary)"
            ),
            SchemaProblem::ChecksumMismatch { version } => write!(
                f,
                "applied migration {version} differs from the one this build embeds (checksum \
                 mismatch)"
            ),
            SchemaProblem::Dirty { version } => {
                write!(
                    f,
                    "migration {version} is recorded as failed (dirty database)"
                )
            }
            SchemaProblem::Unrecorded => f.write_str(
                "the database holds schema objects but no migration record, so its schema is \
                 unknown",
            ),
            SchemaProblem::OutOfOrder { version } => write!(
                f,
                "migration {version} is not applied although a later migration is (the migration \
                 record has a gap)"
            ),
        }
    }
}

/// A failure of the durable store (owner ruling R6; work package P1-02 "Typed failures").
///
/// **Open** enum (`#[non_exhaustive]`): the store modules of later P1-02 stages and later packages
/// add variants, so a `match` outside this crate needs a wildcard arm. Classify a value with
/// [`PersistenceError::engine_error`] and [`PersistenceError::terminal_state`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PersistenceError {
    /// SQLite failed for a reason that is neither a constraint, a busy store nor a damaged file.
    /// `Internal`, terminal state `DatabaseFailure`.
    Database {
        /// What the store was doing, for example `opening the database`.
        context: &'static str,
        /// SQLite's extended result code, in decimal; `None` when the failure did not come from
        /// SQLite (a closed pool, a protocol error).
        sqlite_code: Option<String>,
    },
    /// File I/O failed. `Internal`, terminal state `DatabaseFailure`.
    Io {
        /// What the store was doing, for example `renaming an artifact into place`.
        context: &'static str,
        /// The operating system's error class.
        kind: io::ErrorKind,
        /// The path involved, relative to the data directory, with `/` separators; never
        /// absolute. `None` when no single path is involved.
        path: Option<String>,
    },
    /// The store is busy: SQLite stayed locked past the busy timeout, or a store lock is held by
    /// another process. `Internal`, terminal state `DatabaseFailure` (owner ruling R6).
    Busy {
        /// What the store was doing.
        context: &'static str,
    },
    /// The schema is not current (ADR-015 D2). The code depends on the problem
    /// ([`SchemaProblem::code`]); terminal state `DatabaseFailure`.
    Schema(SchemaProblem),
    /// Persisted state failed its integrity check: a missing, short or altered artifact, an
    /// unknown enum spelling, a negative id, a malformed stored value, a damaged database file.
    /// Never treated as empty state (engine-spec §8.11; DR-C15; KI-A8). `StateCorrupt`, terminal
    /// state `DatabaseFailure`.
    StateCorrupt {
        /// What is corrupt, naming the table, column or relative path.
        detail: String,
    },
    /// A compare-and-set write found the row in another state than expected. A programming error
    /// under the store locks. `Internal`, no terminal state (exit 70).
    Conflict {
        /// The write that lost.
        context: &'static str,
    },
    /// An id does not fit SQLite's signed 64-bit INTEGER. A programming error. `Internal`, no
    /// terminal state (exit 70).
    IdOutOfRange {
        /// The id's column or kind.
        what: &'static str,
        /// The value that does not fit.
        value: u64,
    },
    /// A CHECK, UNIQUE, FOREIGN KEY or trigger constraint of the store refused a write. Rust
    /// validates first, so this is a programming error. `Internal`, no terminal state (exit 70).
    Constraint {
        /// The write that was refused.
        context: &'static str,
        /// The constraint's message, as the store's own DDL words it (a trigger message starts
        /// with `grid: <table>:`), cut to 300 characters.
        detail: String,
    },
    /// A value failed validation before it reached the store. A programming error. `Internal`, no
    /// terminal state (exit 70).
    InvalidInput {
        /// The value's field or kind.
        what: &'static str,
        /// Why it is invalid.
        detail: String,
    },
}

impl PersistenceError {
    /// Builds a [`PersistenceError::StateCorrupt`].
    pub fn state_corrupt(detail: impl Into<String>) -> Self {
        PersistenceError::StateCorrupt {
            detail: detail.into(),
        }
    }

    /// Builds a [`PersistenceError::InvalidInput`].
    pub fn invalid_input(what: &'static str, detail: impl Into<String>) -> Self {
        PersistenceError::InvalidInput {
            what,
            detail: detail.into(),
        }
    }

    /// Builds a [`PersistenceError::Io`] from an I/O error, keeping only its kind (the message of
    /// an `io::Error` can name an absolute path).
    pub fn io(context: &'static str, error: &io::Error, path: Option<String>) -> Self {
        PersistenceError::Io {
            context,
            kind: error.kind(),
            path,
        }
    }

    /// Classifies an `sqlx::Error` without keeping its text (owner ruling R6).
    ///
    /// | `sqlx::Error` | Result |
    /// |---|---|
    /// | database error, `SQLITE_CONSTRAINT` | [`PersistenceError::Constraint`] with the constraint's message |
    /// | database error, `SQLITE_BUSY` or `SQLITE_LOCKED` | [`PersistenceError::Busy`] |
    /// | database error, `SQLITE_CORRUPT` or `SQLITE_NOTADB` | [`PersistenceError::StateCorrupt`] |
    /// | any other database error | [`PersistenceError::Database`] with the extended result code |
    /// | I/O | [`PersistenceError::Io`] with no path |
    /// | a stored value that does not decode | [`PersistenceError::StateCorrupt`] |
    /// | anything else | [`PersistenceError::Database`] with no code |
    pub fn from_sqlx(context: &'static str, error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::Database(database) => {
                let code_text = database.code().map(|code| code.into_owned());
                let primary = code_text
                    .as_deref()
                    .and_then(|code| code.parse::<i64>().ok())
                    .map(|code| code & 0xff);
                match primary {
                    Some(SQLITE_CONSTRAINT) => PersistenceError::Constraint {
                        context,
                        detail: truncate_chars(database.message(), CONSTRAINT_MESSAGE_CHARS)
                            .to_owned(),
                    },
                    Some(SQLITE_BUSY) | Some(SQLITE_LOCKED) => PersistenceError::Busy { context },
                    Some(SQLITE_CORRUPT) | Some(SQLITE_NOTADB) => PersistenceError::StateCorrupt {
                        detail: format!(
                            "the database file is damaged or not a database (SQLite code {}) \
                             while {context}",
                            code_text.as_deref().unwrap_or("unknown")
                        ),
                    },
                    _ => PersistenceError::Database {
                        context,
                        sqlite_code: code_text,
                    },
                }
            }
            sqlx::Error::Io(io_error) => PersistenceError::Io {
                context,
                kind: io_error.kind(),
                path: None,
            },
            sqlx::Error::ColumnDecode { index, .. } => PersistenceError::StateCorrupt {
                detail: format!("column {index} holds a value of the wrong type while {context}"),
            },
            sqlx::Error::Decode(_) => PersistenceError::StateCorrupt {
                detail: format!("a stored value does not decode while {context}"),
            },
            _ => PersistenceError::Database {
                context,
                sqlite_code: None,
            },
        }
    }

    /// The engine error code of this failure (owner ruling R6; the table in the module docs).
    pub const fn code(&self) -> EngineErrorCode {
        match self {
            PersistenceError::Database { .. }
            | PersistenceError::Io { .. }
            | PersistenceError::Busy { .. }
            | PersistenceError::Conflict { .. }
            | PersistenceError::IdOutOfRange { .. }
            | PersistenceError::Constraint { .. }
            | PersistenceError::InvalidInput { .. } => EngineErrorCode::Internal,
            PersistenceError::Schema(problem) => problem.code(),
            PersistenceError::StateCorrupt { .. } => EngineErrorCode::StateCorrupt,
        }
    }

    /// The typed engine error of this failure: [`PersistenceError::code`] and the `Display` text,
    /// normalized by [`normalize_message`] (owner ruling R6).
    pub fn engine_error(&self) -> EngineError {
        let code = self.code();
        EngineError::new(code, normalize_message(code, &self.to_string()))
    }

    /// The terminal state this failure exits with: `DatabaseFailure` (exit 30) for every failure
    /// of the store, and `None` (exit 70, `EXIT_INTERNAL`) for a programming error (owner ruling
    /// R6; engine-output-contract §13, 1.1).
    pub const fn terminal_state(&self) -> Option<TerminalState> {
        match self {
            PersistenceError::Database { .. }
            | PersistenceError::Io { .. }
            | PersistenceError::Busy { .. }
            | PersistenceError::Schema(_)
            | PersistenceError::StateCorrupt { .. } => Some(TerminalState::DatabaseFailure),
            PersistenceError::Conflict { .. }
            | PersistenceError::IdOutOfRange { .. }
            | PersistenceError::Constraint { .. }
            | PersistenceError::InvalidInput { .. } => None,
        }
    }
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PersistenceError::Database {
                context,
                sqlite_code,
            } => match sqlite_code {
                Some(code) => write!(
                    f,
                    "the database failed while {context} (SQLite code {code})"
                ),
                None => write!(f, "the database failed while {context}"),
            },
            PersistenceError::Io {
                context,
                kind,
                path,
            } => match path {
                Some(path) => write!(f, "store I/O failed while {context} at `{path}`: {kind}"),
                None => write!(f, "store I/O failed while {context}: {kind}"),
            },
            PersistenceError::Busy { context } => {
                write!(
                    f,
                    "the store is busy (locked by another writer) while {context}"
                )
            }
            PersistenceError::Schema(problem) => write!(f, "{problem}"),
            PersistenceError::StateCorrupt { detail } => {
                write!(f, "persisted state is corrupt: {detail}")
            }
            PersistenceError::Conflict { context } => write!(
                f,
                "compare-and-set lost: the row was not in the expected state while {context}"
            ),
            PersistenceError::IdOutOfRange { what, value } => write!(
                f,
                "{what} {value} does not fit a signed 64-bit SQLite INTEGER"
            ),
            PersistenceError::Constraint { context, detail } => {
                write!(f, "the store refused a write while {context}: {detail}")
            }
            PersistenceError::InvalidInput { what, detail } => {
                write!(f, "invalid {what}: {detail}")
            }
        }
    }
}

impl std::error::Error for PersistenceError {}

impl From<PersistenceError> for EngineError {
    fn from(error: PersistenceError) -> Self {
        error.engine_error()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_message_becomes_the_code_name() {
        assert_eq!(
            normalize_message(EngineErrorCode::NotConverged, ""),
            "NotConverged"
        );
        assert_eq!(normalize_message(EngineErrorCode::Internal, "x"), "x");
    }

    #[test]
    fn long_message_is_cut_on_a_character_boundary() {
        let long = "é".repeat(MAX_MESSAGE_CHARS + 5);
        let cut = normalize_message(EngineErrorCode::Internal, &long);
        assert_eq!(cut.chars().count(), MAX_MESSAGE_CHARS);
        assert!(long.starts_with(&cut));
        let exact = "a".repeat(MAX_MESSAGE_CHARS);
        assert_eq!(normalize_message(EngineErrorCode::Internal, &exact), exact);
    }

    #[test]
    fn mapping_follows_ruling_r6() {
        let cases: [(PersistenceError, EngineErrorCode, Option<TerminalState>); 12] = [
            (
                PersistenceError::Database {
                    context: "x",
                    sqlite_code: Some("1".into()),
                },
                EngineErrorCode::Internal,
                Some(TerminalState::DatabaseFailure),
            ),
            (
                PersistenceError::Io {
                    context: "x",
                    kind: io::ErrorKind::NotFound,
                    path: None,
                },
                EngineErrorCode::Internal,
                Some(TerminalState::DatabaseFailure),
            ),
            (
                PersistenceError::Busy { context: "x" },
                EngineErrorCode::Internal,
                Some(TerminalState::DatabaseFailure),
            ),
            (
                PersistenceError::Schema(SchemaProblem::NotInitialized),
                EngineErrorCode::UnsupportedVersion,
                Some(TerminalState::DatabaseFailure),
            ),
            (
                PersistenceError::Schema(SchemaProblem::Pending {
                    applied: Some(1),
                    pending: vec![2],
                }),
                EngineErrorCode::UnsupportedVersion,
                Some(TerminalState::DatabaseFailure),
            ),
            (
                PersistenceError::Schema(SchemaProblem::Newer { version: 9 }),
                EngineErrorCode::UnsupportedVersion,
                Some(TerminalState::DatabaseFailure),
            ),
            (
                PersistenceError::Schema(SchemaProblem::ChecksumMismatch { version: 2 }),
                EngineErrorCode::StateCorrupt,
                Some(TerminalState::DatabaseFailure),
            ),
            (
                PersistenceError::Schema(SchemaProblem::Dirty { version: 2 }),
                EngineErrorCode::StateCorrupt,
                Some(TerminalState::DatabaseFailure),
            ),
            (
                PersistenceError::state_corrupt("x"),
                EngineErrorCode::StateCorrupt,
                Some(TerminalState::DatabaseFailure),
            ),
            (
                PersistenceError::Conflict { context: "x" },
                EngineErrorCode::Internal,
                None,
            ),
            (
                PersistenceError::IdOutOfRange {
                    what: "job_id",
                    value: u64::MAX,
                },
                EngineErrorCode::Internal,
                None,
            ),
            (
                PersistenceError::Constraint {
                    context: "x",
                    detail: "grid: jobs: y".into(),
                },
                EngineErrorCode::Internal,
                None,
            ),
        ];
        for (error, code, state) in cases {
            assert_eq!(error.code(), code, "{error}");
            assert_eq!(error.engine_error().code, code, "{error}");
            assert_eq!(error.terminal_state(), state, "{error}");
            assert!(!error.engine_error().message.is_empty());
        }
    }
}
