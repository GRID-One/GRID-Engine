//! Model-state transitions (engine-spec §8.8 "States"; work package P1-02 D13).
//!
//! Every model version has exactly one [`ModelState`]. Engine-spec §8.8 lists the permitted
//! transitions and says "no other transition is permitted"; [`MODEL_TRANSITIONS`] is that list,
//! and the `model_versions` trigger of migration `0002_backbone.sql` is its backstop in the store
//! (D13).
//!
//! Each transition also names the [`TransitionCause`] that may perform it, and the causes encode
//! only what §8.8 and D14 state:
//!
//! - a version enters or leaves `PRODUCTION` only inside a pointer move (§8.8: "a version is
//!   `PRODUCTION` exactly when a production pointer names it, and its state changes in the same
//!   write transaction that moves the pointer"; §8.13);
//! - `SUPERSEDED → PRODUCTION` happens "only through rollback_to_snapshot" (§8.8), and
//!   `VALIDATING → PRODUCTION` is the "promote" step of the §8.8 flow;
//! - `PRODUCTION → SUPERSEDED` happens "when a later version is promoted for the same family, or
//!   when rollback_to_snapshot moves the family's production pointer" (§8.8);
//! - a pointer move changes no other state (D14 lists the state changes of a promotion; §8.7.4 and
//!   D14 those of a rollback).
//!
//! §8.8 names no cause for the transitions that move no pointer, so the policy does not
//! distinguish who performs them: they all take [`TransitionCause::StateOnly`].

use std::fmt;

use grid_domain::ModelState;

use crate::error::GovernanceError;

/// What performs a model-state transition: a state change on its own, or one of the two pointer
/// moves. **Closed** enum: [`check_transition`] matches on it.
///
/// The cause is policy, not a stored value: the caller states it, and [`check_transition`]
/// refuses a permitted pair requested by the wrong cause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TransitionCause {
    /// A state change that moves no production pointer: the outcome of training
    /// (`TRAINING → CANDIDATE`, `TRAINING → REJECTED`), the start of validation
    /// (`CANDIDATE → VALIDATING`) and the rejection of a candidate (`VALIDATING → REJECTED`).
    /// Engine-spec §8.8 names no cause for these transitions, so whichever stage reaches the
    /// outcome performs them with this cause.
    StateOnly,
    /// The promotion pointer transaction: the candidate becomes production
    /// (`VALIDATING → PRODUCTION`) and the version it replaces is superseded
    /// (`PRODUCTION → SUPERSEDED`; engine-spec §8.8; work package P1-02 D14).
    Promotion,
    /// `rollback_to_snapshot` (engine-spec §8.2 command 10, §8.7.4): the recorded version is
    /// restored (`SUPERSEDED → PRODUCTION`) and the version it replaces is superseded
    /// (`PRODUCTION → SUPERSEDED`; work package P1-02 D14).
    Rollback,
}

impl TransitionCause {
    /// Every cause, in declaration order.
    pub const ALL: [TransitionCause; 3] = [
        TransitionCause::StateOnly,
        TransitionCause::Promotion,
        TransitionCause::Rollback,
    ];

    /// The cause's stable name, equal to its variant name.
    pub const fn name(self) -> &'static str {
        match self {
            TransitionCause::StateOnly => "StateOnly",
            TransitionCause::Promotion => "Promotion",
            TransitionCause::Rollback => "Rollback",
        }
    }

    /// Whether the cause is a production-pointer move: [`TransitionCause::Promotion`] or
    /// [`TransitionCause::Rollback`].
    pub const fn moves_a_pointer(self) -> bool {
        match self {
            TransitionCause::StateOnly => false,
            TransitionCause::Promotion | TransitionCause::Rollback => true,
        }
    }
}

impl fmt::Display for TransitionCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// One permitted model-state transition and the causes allowed to perform it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModelTransition {
    /// The state the version leaves.
    pub from: ModelState,
    /// The state the version enters.
    pub to: ModelState,
    /// The causes that may perform it; never empty.
    pub causes: &'static [TransitionCause],
}

/// The seven transitions engine-spec §8.8 permits, in the order it writes them, each with the
/// causes allowed to perform it. No other transition is permitted, and no state moves to itself.
///
/// | From | To | Causes | §8.8 |
/// |---|---|---|---|
/// | `TRAINING` | `CANDIDATE` | `StateOnly` | `TRAINING → CANDIDATE` |
/// | `CANDIDATE` | `VALIDATING` | `StateOnly` | `CANDIDATE → VALIDATING` |
/// | `VALIDATING` | `PRODUCTION` | `Promotion` | `VALIDATING → { PRODUCTION \| … }`, the "promote" step |
/// | `VALIDATING` | `REJECTED` | `StateOnly` | `VALIDATING → { … \| REJECTED }`, the "reject" step |
/// | `TRAINING` | `REJECTED` | `StateOnly` | "training fails or is aborted, or its artifact cannot be written" |
/// | `PRODUCTION` | `SUPERSEDED` | `Promotion`, `Rollback` | "when a later version is promoted … or when rollback_to_snapshot moves the family's production pointer" |
/// | `SUPERSEDED` | `PRODUCTION` | `Rollback` | "only through rollback_to_snapshot" |
pub const MODEL_TRANSITIONS: [ModelTransition; 7] = [
    ModelTransition {
        from: ModelState::Training,
        to: ModelState::Candidate,
        causes: &[TransitionCause::StateOnly],
    },
    ModelTransition {
        from: ModelState::Candidate,
        to: ModelState::Validating,
        causes: &[TransitionCause::StateOnly],
    },
    ModelTransition {
        from: ModelState::Validating,
        to: ModelState::Production,
        causes: &[TransitionCause::Promotion],
    },
    ModelTransition {
        from: ModelState::Validating,
        to: ModelState::Rejected,
        causes: &[TransitionCause::StateOnly],
    },
    ModelTransition {
        from: ModelState::Training,
        to: ModelState::Rejected,
        causes: &[TransitionCause::StateOnly],
    },
    ModelTransition {
        from: ModelState::Production,
        to: ModelState::Superseded,
        causes: &[TransitionCause::Promotion, TransitionCause::Rollback],
    },
    ModelTransition {
        from: ModelState::Superseded,
        to: ModelState::Production,
        causes: &[TransitionCause::Rollback],
    },
];

/// The entry of [`MODEL_TRANSITIONS`] for `from → to`, or `None` when §8.8 does not permit it.
fn transition(from: ModelState, to: ModelState) -> Option<&'static ModelTransition> {
    MODEL_TRANSITIONS
        .iter()
        .find(|entry| entry.from == from && entry.to == to)
}

/// Whether engine-spec §8.8 permits `from → to` for some cause. This is the pair rule the store's
/// trigger enforces, which cannot see the cause (work package P1-02 D13).
pub fn model_transition_allowed(from: ModelState, to: ModelState) -> bool {
    transition(from, to).is_some()
}

/// Checks that `cause` may move a version from `from` to `to` (engine-spec §8.8).
///
/// # Errors
///
/// [`GovernanceError::IllegalTransition`] when §8.8 does not permit the pair, or permits it only
/// for another cause: for example `SUPERSEDED → PRODUCTION` requested by `Promotion`, or
/// `VALIDATING → PRODUCTION` requested by `StateOnly`, which would make a version `PRODUCTION`
/// without moving a pointer.
pub fn check_transition(
    from: ModelState,
    to: ModelState,
    cause: TransitionCause,
) -> Result<(), GovernanceError> {
    match transition(from, to) {
        Some(entry) if entry.causes.contains(&cause) => Ok(()),
        _ => Err(GovernanceError::IllegalTransition { from, to, cause }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    use ModelState as S;
    use TransitionCause as C;

    /// The §8.8 list, written out independently of [`MODEL_TRANSITIONS`].
    fn expected() -> Vec<(ModelState, ModelState, Vec<TransitionCause>)> {
        vec![
            (S::Training, S::Candidate, vec![C::StateOnly]),
            (S::Candidate, S::Validating, vec![C::StateOnly]),
            (S::Validating, S::Production, vec![C::Promotion]),
            (S::Validating, S::Rejected, vec![C::StateOnly]),
            (S::Training, S::Rejected, vec![C::StateOnly]),
            (
                S::Production,
                S::Superseded,
                vec![C::Promotion, C::Rollback],
            ),
            (S::Superseded, S::Production, vec![C::Rollback]),
        ]
    }

    #[test]
    fn the_table_is_exactly_the_spec_list() {
        let table: Vec<(ModelState, ModelState, Vec<TransitionCause>)> = MODEL_TRANSITIONS
            .iter()
            .map(|t| (t.from, t.to, t.causes.to_vec()))
            .collect();
        assert_eq!(table, expected());
        let pairs: BTreeSet<(&str, &str)> = MODEL_TRANSITIONS
            .iter()
            .map(|t| (t.from.spec_name(), t.to.spec_name()))
            .collect();
        assert_eq!(pairs.len(), 7, "no pair is listed twice");
        for t in MODEL_TRANSITIONS {
            assert!(!t.causes.is_empty());
        }
    }

    #[test]
    fn every_pair_and_cause_not_listed_is_refused() {
        let expected = expected();
        let mut checked = 0;
        for from in ModelState::ALL {
            for to in ModelState::ALL {
                let listed = expected.iter().find(|(f, t, _)| *f == from && *t == to);
                assert_eq!(
                    model_transition_allowed(from, to),
                    listed.is_some(),
                    "{from} -> {to}"
                );
                for cause in TransitionCause::ALL {
                    let permitted = listed.is_some_and(|(_, _, causes)| causes.contains(&cause));
                    let result = check_transition(from, to, cause);
                    if permitted {
                        assert_eq!(result, Ok(()), "{from} -> {to} by {cause}");
                    } else {
                        assert_eq!(
                            result,
                            Err(GovernanceError::IllegalTransition { from, to, cause }),
                            "{from} -> {to} by {cause}"
                        );
                    }
                }
                checked += 1;
            }
        }
        assert_eq!(checked, 36);
    }

    #[test]
    fn production_is_entered_and_left_exactly_by_a_pointer_move() {
        // §8.8: a version is PRODUCTION exactly when a pointer names it, and its state changes in
        // the transaction that moves the pointer. So a transition that touches PRODUCTION is
        // performed only by a pointer move, and a pointer move performs no other transition.
        for t in MODEL_TRANSITIONS {
            let touches_production = t.from == S::Production || t.to == S::Production;
            for cause in t.causes {
                assert_eq!(
                    cause.moves_a_pointer(),
                    touches_production,
                    "{} -> {} by {cause}",
                    t.from,
                    t.to
                );
            }
        }
        assert!(!C::StateOnly.moves_a_pointer());
        assert!(C::Promotion.moves_a_pointer());
        assert!(C::Rollback.moves_a_pointer());
    }

    #[test]
    fn named_rules_of_the_spec() {
        // SUPERSEDED → PRODUCTION only through rollback_to_snapshot.
        assert!(check_transition(S::Superseded, S::Production, C::Rollback).is_ok());
        assert!(check_transition(S::Superseded, S::Production, C::Promotion).is_err());
        assert!(check_transition(S::Superseded, S::Production, C::StateOnly).is_err());
        // VALIDATING → PRODUCTION only by promotion, never without a pointer move.
        assert!(check_transition(S::Validating, S::Production, C::Promotion).is_ok());
        assert!(check_transition(S::Validating, S::Production, C::Rollback).is_err());
        assert!(check_transition(S::Validating, S::Production, C::StateOnly).is_err());
        // PRODUCTION → SUPERSEDED by promotion or rollback, and by nothing else.
        assert!(check_transition(S::Production, S::Superseded, C::Promotion).is_ok());
        assert!(check_transition(S::Production, S::Superseded, C::Rollback).is_ok());
        assert!(check_transition(S::Production, S::Superseded, C::StateOnly).is_err());
        // The transitions that move no pointer are open to any stage that reaches the outcome.
        for (from, to) in [
            (S::Training, S::Candidate),
            (S::Candidate, S::Validating),
            (S::Validating, S::Rejected),
            (S::Training, S::Rejected),
        ] {
            assert!(
                check_transition(from, to, C::StateOnly).is_ok(),
                "{from} -> {to}"
            );
        }
        // A rejected version never returns to PRODUCTION, and REJECTED has no way out at all.
        for to in ModelState::ALL {
            assert!(!model_transition_allowed(S::Rejected, to), "{to}");
        }
        // No state moves to itself.
        for state in ModelState::ALL {
            assert!(!model_transition_allowed(state, state), "{state}");
        }
        assert_eq!(C::Rollback.to_string(), "Rollback");
        assert_eq!(C::StateOnly.to_string(), "StateOnly");
        assert_eq!(C::ALL.len(), 3);
    }
}
