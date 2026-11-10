//! The typed failures of the governance policy (engine-spec §6.8 rule 4; DR-B6) and their mapping
//! onto the engine's closed error vocabulary (engine-output-contract §13, 1.1; owner ruling R6,
//! P1-02).
//!
//! `Display` and [`std::error::Error`] are written by hand, as in `grid-domain` (ADR-014
//! decision 4), so the crate keeps `grid-domain` as its only dependency (work package P1-02 D1).

use std::fmt;

use grid_domain::{EngineError, EngineErrorCode, ModelFamily, ModelState, ModelVersionId};

use crate::model_state::TransitionCause;

/// A governance policy refused a request, or found that the state its caller read violates an
/// invariant the policy relies on.
///
/// Every message names what failed and carries no path, host or credential
/// (engine-output-contract §8 rule 5). Text that came from the store, such as a family name or a
/// setting key, is shown escaped and quoted.
///
/// **Open** enum (`#[non_exhaustive]`): the packages that add the promotion gate and the
/// threshold registry (P2-07) add variants, so a `match` outside this crate needs a wildcard arm.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum GovernanceError {
    /// A model-state transition that engine-spec §8.8 does not permit, or permits only for
    /// another cause ([`crate::model_state::check_transition`]). A request error; maps to
    /// [`EngineErrorCode::Internal`].
    IllegalTransition {
        /// The state the version is in.
        from: ModelState,
        /// The state requested.
        to: ModelState,
        /// The cause that requested it.
        cause: TransitionCause,
    },
    /// A rollback target that the snapshot records is not `SUPERSEDED`, so it cannot be restored
    /// (engine-spec §8.7.4, §8.8: "a rejected version never returns to `PRODUCTION`"). The whole
    /// rollback is refused; nothing moves ([`crate::pointers::plan_rollback`]).
    ///
    /// In a consistent store this cannot happen: a snapshot records pointer values (§8.7.4), a
    /// pointer names only a `PRODUCTION` version (§8.8), and the only way out of `PRODUCTION` is
    /// `SUPERSEDED`, so a recorded version is now either `SUPERSEDED` or still named by its
    /// family's pointer, which the rollback skips. The cause is persisted state, so it maps to
    /// [`EngineErrorCode::StateCorrupt`] (owner ruling R6, P1-02: "StateCorrupt for integrity
    /// failures").
    NotRestorable {
        /// The family whose pointer the rollback would move.
        family: ModelFamily,
        /// The version the snapshot records for it.
        version: ModelVersionId,
        /// The state that version is in now.
        state: ModelState,
    },
    /// A promotion names a candidate of another family than the pointer it would move
    /// ([`crate::pointers::plan_promotion`]). A request error; maps to
    /// [`EngineErrorCode::Internal`].
    FamilyMismatch {
        /// The candidate version.
        version: ModelVersionId,
        /// The family the candidate belongs to.
        version_family: ModelFamily,
        /// The family whose pointer the promotion would move.
        pointer_family: ModelFamily,
    },
    /// An input of the idempotency key is malformed (engine-spec §8.10 "Idempotency"; work
    /// package P1-02 D6): the configuration hash is not 64 lowercase hexadecimal characters. The
    /// payload says what is wrong. Maps to [`EngineErrorCode::Internal`].
    InvalidKey(String),
    /// An engine setting the job policy reads is missing, unknown inside a validated namespace,
    /// not an unsigned decimal integer, or out of bounds (work package P1-02 D8). The payload
    /// names the key. Persisted state failed its check, so it maps to
    /// [`EngineErrorCode::StateCorrupt`] and is never replaced by a default (DR-B6).
    InvalidSetting(String),
    /// The persisted state the caller read violates an invariant of engine-spec §8.7.4 or §8.8: a
    /// production pointer naming a version that is not `PRODUCTION` or belongs to another family,
    /// a `PRODUCTION` version that its family's pointer does not name, a snapshot naming a
    /// version the store does not have or a family that has no pointer now, a family listed
    /// twice. The payload says which. Maps to [`EngineErrorCode::StateCorrupt`].
    Inconsistent(String),
}

impl GovernanceError {
    /// The closed engine error code of this failure: [`EngineErrorCode::StateCorrupt`] for
    /// [`GovernanceError::NotRestorable`], [`GovernanceError::InvalidSetting`] and
    /// [`GovernanceError::Inconsistent`], whose cause is persisted state;
    /// [`EngineErrorCode::Internal`] for every other variant, whose cause is the request (owner
    /// ruling R6, P1-02: a programming error carries no terminal state and exits
    /// `EXIT_INTERNAL`).
    pub const fn code(&self) -> EngineErrorCode {
        match self {
            GovernanceError::NotRestorable { .. }
            | GovernanceError::InvalidSetting(_)
            | GovernanceError::Inconsistent(_) => EngineErrorCode::StateCorrupt,
            GovernanceError::IllegalTransition { .. }
            | GovernanceError::FamilyMismatch { .. }
            | GovernanceError::InvalidKey(_) => EngineErrorCode::Internal,
        }
    }
}

impl fmt::Display for GovernanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GovernanceError::IllegalTransition { from, to, cause } => write!(
                f,
                "model-state transition {from} -> {to} is not permitted for cause {cause}"
            ),
            GovernanceError::NotRestorable {
                family,
                version,
                state,
            } => write!(
                f,
                "rollback refused: the snapshot records version {version} for family {:?}, which \
                 is {state}, not SUPERSEDED",
                family.0
            ),
            GovernanceError::FamilyMismatch {
                version,
                version_family,
                pointer_family,
            } => write!(
                f,
                "promotion refused: version {version} belongs to family {:?}, not {:?}",
                version_family.0, pointer_family.0
            ),
            GovernanceError::InvalidKey(detail) => write!(f, "invalid idempotency key: {detail}"),
            GovernanceError::InvalidSetting(detail) => {
                write!(f, "invalid engine setting: {detail}")
            }
            GovernanceError::Inconsistent(detail) => {
                write!(f, "inconsistent governance state: {detail}")
            }
        }
    }
}

impl std::error::Error for GovernanceError {}

impl From<GovernanceError> for EngineError {
    /// Carries the failure as an [`EngineError`] with [`GovernanceError::code`] and the
    /// `Display` text as its message.
    fn from(error: GovernanceError) -> Self {
        EngineError::new(error.code(), error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn every_variant() -> Vec<GovernanceError> {
        vec![
            GovernanceError::IllegalTransition {
                from: ModelState::Rejected,
                to: ModelState::Production,
                cause: TransitionCause::Promotion,
            },
            GovernanceError::NotRestorable {
                family: ModelFamily("kalman".into()),
                version: ModelVersionId(7),
                state: ModelState::Rejected,
            },
            GovernanceError::FamilyMismatch {
                version: ModelVersionId(9),
                version_family: ModelFamily("rapm".into()),
                pointer_family: ModelFamily("kalman".into()),
            },
            GovernanceError::InvalidKey("config hash is 3 bytes".into()),
            GovernanceError::InvalidSetting("`jobs.lease_secs` is missing".into()),
            GovernanceError::Inconsistent("family \"rapm\" is listed twice".into()),
        ]
    }

    #[test]
    fn engine_error_codes_follow_the_cause() {
        let expected = [
            EngineErrorCode::Internal,
            EngineErrorCode::StateCorrupt,
            EngineErrorCode::Internal,
            EngineErrorCode::Internal,
            EngineErrorCode::StateCorrupt,
            EngineErrorCode::StateCorrupt,
        ];
        let all = every_variant();
        assert_eq!(all.len(), expected.len());
        for (error, code) in all.into_iter().zip(expected) {
            assert_eq!(error.code(), code, "{error:?}");
            let text = error.to_string();
            let engine: EngineError = error.into();
            assert_eq!(engine.code, code);
            assert_eq!(engine.message, text);
            assert!(!engine.message.is_empty());
        }
    }

    #[test]
    fn messages_name_what_failed() {
        let shown: Vec<String> = every_variant().iter().map(|e| e.to_string()).collect();
        assert_eq!(
            shown[0],
            "model-state transition REJECTED -> PRODUCTION is not permitted for cause Promotion"
        );
        assert_eq!(
            shown[1],
            "rollback refused: the snapshot records version 7 for family \"kalman\", which is \
             REJECTED, not SUPERSEDED"
        );
        assert_eq!(
            shown[2],
            "promotion refused: version 9 belongs to family \"rapm\", not \"kalman\""
        );
        assert_eq!(shown[3], "invalid idempotency key: config hash is 3 bytes");
        assert_eq!(
            shown[4],
            "invalid engine setting: `jobs.lease_secs` is missing"
        );
        assert_eq!(
            shown[5],
            "inconsistent governance state: family \"rapm\" is listed twice"
        );
        let boxed: Box<dyn std::error::Error> = Box::new(every_variant().remove(0));
        assert!(boxed.to_string().starts_with("model-state transition"));
    }

    #[test]
    fn store_text_is_escaped_in_messages() {
        let error = GovernanceError::FamilyMismatch {
            version: ModelVersionId(1),
            version_family: ModelFamily("a\nb".into()),
            pointer_family: ModelFamily("c".into()),
        };
        let text = error.to_string();
        assert!(!text.contains('\n'), "{text}");
        assert!(text.contains("\"a\\nb\""), "{text}");
    }
}
