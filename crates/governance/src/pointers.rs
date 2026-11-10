//! Production-pointer plans for promotion and rollback (engine-spec §8.7.4, §8.8, §8.13; work
//! package P1-02 D14).
//!
//! A production pointer names the `PRODUCTION` version of one model family; a version is
//! `PRODUCTION` exactly when a pointer names it (§8.8). The functions here check what the caller
//! read and return the pointer moves to make. `grid-persistence` executes them in one
//! `BEGIN IMMEDIATE` transaction, together with the snapshot of every pointer and the state
//! changes of [`crate::model_state`], with compare-and-set writes (D14, §8.13). Nothing here
//! copies or deletes a version (§8.7.4: rollback "never copies or deletes versions").
//!
//! The failures fall in two classes ([`GovernanceError::code`]):
//!
//! - a request that a consistent store refuses (a candidate that is not `VALIDATING`, of another
//!   family, or already production) is a request error, `Internal`;
//! - a read that no consistent store can produce (a pointer naming a version that is not
//!   `PRODUCTION` or belongs to another family, a `PRODUCTION` version its family's pointer does
//!   not name, a snapshot naming a missing version, a non-`SUPERSEDED` target or a family with no
//!   pointer now) is persisted-state corruption, `StateCorrupt` (owner ruling R6, P1-02).

use std::collections::{BTreeMap, BTreeSet};

use grid_domain::{ModelFamily, ModelState, ModelVersionId};

use crate::error::GovernanceError;
use crate::model_state::{check_transition, TransitionCause};

/// One production-pointer move (work package P1-02 D14). Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PointerMove {
    /// The family whose pointer moves.
    pub family: ModelFamily,
    /// The version the pointer names before the move, which goes `PRODUCTION → SUPERSEDED`.
    /// `None` only for the first promotion of a family, which creates its pointer; a rollback
    /// move always has one ([`plan_rollback`]).
    pub from: Option<ModelVersionId>,
    /// The version the pointer names after the move. It goes `VALIDATING → PRODUCTION` in a
    /// promotion and `SUPERSEDED → PRODUCTION` in a rollback.
    pub to: ModelVersionId,
}

/// Plans the promotion of a validated candidate onto its family's production pointer
/// (engine-spec §8.8 "promote OR reject"; work package P1-02 D14).
///
/// `family` is the pointer to move. `candidate` is the candidate version as the caller read it:
/// its id, family and state. `current` is the version the pointer names now, read the same way,
/// or `None` when the family has never had a production version.
///
/// The checks run in this order, so the error is deterministic:
///
/// 1. the candidate belongs to `family`;
/// 2. the current version, if any, belongs to `family` and is `PRODUCTION` (§8.8: "a version is
///    `PRODUCTION` exactly when a production pointer names it");
/// 3. when the current version is the candidate, the two reads agree, and the request, which
///    would promote the version that is already production, is refused;
/// 4. a `PRODUCTION` candidate is named by the pointer (§8.8), which step 3 has already refused;
/// 5. the candidate is `VALIDATING`, so that `VALIDATING → PRODUCTION` by
///    [`TransitionCause::Promotion`] is permitted ([`check_transition`]).
///
/// # Errors
///
/// Request errors, which map to `Internal`:
///
/// - [`GovernanceError::FamilyMismatch`] when the candidate belongs to another family;
/// - [`GovernanceError::IllegalTransition`] `PRODUCTION → PRODUCTION` when the candidate is the
///   current production version, and `<state> → PRODUCTION` when the candidate is `TRAINING`,
///   `CANDIDATE`, `REJECTED` or `SUPERSEDED`.
///
/// Persisted-state errors, which map to `StateCorrupt`:
///
/// - [`GovernanceError::Inconsistent`] when the current version belongs to another family or is
///   not `PRODUCTION`, when the candidate and the current version are the same version read in
///   two states, or when the candidate is `PRODUCTION` but the pointer names another version or
///   no version.
pub fn plan_promotion(
    family: &ModelFamily,
    candidate: (ModelVersionId, ModelFamily, ModelState),
    current: Option<(ModelVersionId, ModelFamily, ModelState)>,
) -> Result<PointerMove, GovernanceError> {
    let (candidate_id, candidate_family, candidate_state) = candidate;
    if candidate_family != *family {
        return Err(GovernanceError::FamilyMismatch {
            version: candidate_id,
            version_family: candidate_family,
            pointer_family: family.clone(),
        });
    }
    let current_id = match current {
        Some((current_id, current_family, current_state)) => {
            check_pointer_target(family, current_id, Some((current_family, current_state)))?;
            if current_id == candidate_id {
                if candidate_state != current_state {
                    return Err(GovernanceError::Inconsistent(format!(
                        "version {candidate_id} of family {:?} was read as both {candidate_state} \
                         (the candidate) and {current_state} (the production version)",
                        family.0
                    )));
                }
                return Err(GovernanceError::IllegalTransition {
                    from: ModelState::Production,
                    to: ModelState::Production,
                    cause: TransitionCause::Promotion,
                });
            }
            check_transition(
                ModelState::Production,
                ModelState::Superseded,
                TransitionCause::Promotion,
            )?;
            Some(current_id)
        }
        None => None,
    };
    if candidate_state == ModelState::Production {
        return Err(GovernanceError::Inconsistent(format!(
            "version {candidate_id} of family {:?} is PRODUCTION, but the family's production \
             pointer names {}",
            family.0,
            current_id.map_or_else(|| "no version".to_owned(), |id| format!("version {id}"))
        )));
    }
    check_transition(
        candidate_state,
        ModelState::Production,
        TransitionCause::Promotion,
    )?;
    Ok(PointerMove {
        family: candidate_family,
        from: current_id,
        to: candidate_id,
    })
}

/// The pointer moves of a rollback to a snapshot (engine-spec §8.7.4 "Rollback"; work package
/// P1-02 D14). Never null.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RollbackPlan {
    /// The pointers that move, one per family whose current version differs from the recorded
    /// one, ordered by family. Empty when every recorded pointer already names its recorded
    /// version.
    pub moves: Vec<PointerMove>,
    /// The families that have a pointer now but none in the snapshot, ordered by family. Their
    /// pointers are left untouched and reported, never cleared: rollback never deletes (§8.7.4).
    pub untouched: Vec<ModelFamily>,
}

/// Plans `rollback_to_snapshot` (engine-spec §8.2 command 10, §8.7.4; work package P1-02 D14).
///
/// `current` is every production pointer now, as `(family, version)`; `recorded` is every pointer
/// the snapshot records. `version_of` returns the family and current state of a version, or
/// `None` when the store has no such version. Both lists may be in any order: the plan is ordered
/// by family.
///
/// - A family whose current version equals the recorded one does not move, and is not checked
///   further.
/// - A family whose current version differs moves to the recorded version, which must belong to
///   the family and be `SUPERSEDED` (`SUPERSEDED → PRODUCTION` by [`TransitionCause::Rollback`]);
///   its current version must belong to the family and be `PRODUCTION`, and goes
///   `PRODUCTION → SUPERSEDED`.
/// - A recorded family must have a pointer now. A snapshot records only pointers that existed
///   (§8.7.4), and nothing removes a pointer: a promotion creates or moves one and a rollback never
///   deletes (D14). A recorded family with no pointer is therefore persisted-state corruption,
///   and the rollback is refused rather than creating the pointer.
/// - A family with a current pointer and none in the snapshot is untouched and reported in
///   [`RollbackPlan::untouched`].
///
/// The plan is all or nothing: one refused family refuses the whole rollback, and nothing moves.
/// The checks run in a fixed order, so the error is deterministic: duplicates in either list
/// first, then each recorded family in family order (its pointer, then the recorded version, then
/// the current version).
///
/// # Errors
///
/// Every error here is persisted-state corruption and maps to `StateCorrupt`, because `recorded`
/// and `current` come from the store:
///
/// - [`GovernanceError::NotRestorable`] when a recorded version that would be restored is not
///   `SUPERSEDED` (for example `REJECTED`: "a rejected version never returns to `PRODUCTION`",
///   §8.8);
/// - [`GovernanceError::Inconsistent`] when a list names a family or a version twice, when a
///   recorded family has no pointer now, when a recorded version is not in the store or belongs to
///   another family, or when a pointer that would move names a version that is not in the store,
///   belongs to another family or is not `PRODUCTION`.
pub fn plan_rollback(
    current: &[(ModelFamily, ModelVersionId)],
    recorded: &[(ModelFamily, ModelVersionId)],
    version_of: impl Fn(ModelVersionId) -> Option<(ModelFamily, ModelState)>,
) -> Result<RollbackPlan, GovernanceError> {
    let current = by_family(current, "the production pointers")?;
    let recorded = by_family(recorded, "the snapshot")?;

    let mut moves = Vec::new();
    for (&family, &target) in &recorded {
        let Some(now) = current.get(family).copied() else {
            return Err(GovernanceError::Inconsistent(format!(
                "the snapshot records version {target} for family {:?}, which has no production \
                 pointer now",
                family.0
            )));
        };
        if now == target {
            continue;
        }
        match version_of(target) {
            Some((target_family, _)) if target_family != *family => {
                return Err(GovernanceError::Inconsistent(format!(
                    "the snapshot records version {target} for family {:?}, but that version \
                     belongs to family {:?}",
                    family.0, target_family.0
                )))
            }
            Some((_, ModelState::Superseded)) => {}
            Some((_, state)) => {
                return Err(GovernanceError::NotRestorable {
                    family: family.clone(),
                    version: target,
                    state,
                })
            }
            None => {
                return Err(GovernanceError::Inconsistent(format!(
                    "the snapshot records version {target} for family {:?}, which the store \
                     does not have",
                    family.0
                )))
            }
        }
        check_transition(
            ModelState::Superseded,
            ModelState::Production,
            TransitionCause::Rollback,
        )?;
        check_pointer_target(family, now, version_of(now))?;
        check_transition(
            ModelState::Production,
            ModelState::Superseded,
            TransitionCause::Rollback,
        )?;
        moves.push(PointerMove {
            family: family.clone(),
            from: Some(now),
            to: target,
        });
    }

    let untouched = current
        .keys()
        .filter(|family| !recorded.contains_key(*family))
        .map(|family| (*family).clone())
        .collect();
    Ok(RollbackPlan { moves, untouched })
}

/// Indexes `(family, version)` pairs by family, refusing a family or a version listed twice: a
/// pointer list and a snapshot hold one row per family, and a version belongs to one family.
fn by_family<'a>(
    pairs: &'a [(ModelFamily, ModelVersionId)],
    what: &str,
) -> Result<BTreeMap<&'a ModelFamily, ModelVersionId>, GovernanceError> {
    let mut by_family = BTreeMap::new();
    let mut versions = BTreeSet::new();
    for (family, version) in pairs {
        if by_family.insert(family, *version).is_some() {
            return Err(GovernanceError::Inconsistent(format!(
                "family {:?} appears twice in {what}",
                family.0
            )));
        }
        if !versions.insert(*version) {
            return Err(GovernanceError::Inconsistent(format!(
                "version {version} appears for more than one family in {what}"
            )));
        }
    }
    Ok(by_family)
}

/// Checks that the production pointer of `family`, which names `version`, names a `PRODUCTION`
/// version of that family (§8.8). `read` is the version's family and state, or `None` when the
/// store has no such version.
fn check_pointer_target(
    family: &ModelFamily,
    version: ModelVersionId,
    read: Option<(ModelFamily, ModelState)>,
) -> Result<(), GovernanceError> {
    match read {
        Some((version_family, _)) if version_family != *family => {
            Err(GovernanceError::Inconsistent(format!(
                "the production pointer of family {:?} names version {version}, which belongs \
                 to family {:?}",
                family.0, version_family.0
            )))
        }
        Some((_, ModelState::Production)) => Ok(()),
        state => Err(GovernanceError::Inconsistent(format!(
            "the production pointer of family {:?} names version {version}, which is {}, not \
             PRODUCTION",
            family.0,
            state.map_or("not in the store", |(_, state)| state.spec_name())
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use grid_domain::EngineErrorCode;

    use ModelState as S;

    fn fam(name: &str) -> ModelFamily {
        ModelFamily(name.to_owned())
    }

    fn v(id: u64) -> ModelVersionId {
        ModelVersionId(id)
    }

    fn inconsistent(result: Result<impl std::fmt::Debug, GovernanceError>) -> String {
        match result {
            Err(error @ GovernanceError::Inconsistent(_)) => {
                assert_eq!(error.code(), EngineErrorCode::StateCorrupt);
                error.to_string()
            }
            other => panic!("expected Inconsistent, got {other:?}"),
        }
    }

    #[test]
    fn promotion_moves_the_pointer_to_a_validating_candidate() {
        let kalman = fam("kalman");
        assert_eq!(
            plan_promotion(&kalman, (v(2), fam("kalman"), S::Validating), None),
            Ok(PointerMove {
                family: kalman.clone(),
                from: None,
                to: v(2),
            }),
            "first promotion of the family"
        );
        assert_eq!(
            plan_promotion(
                &kalman,
                (v(5), fam("kalman"), S::Validating),
                Some((v(2), fam("kalman"), S::Production))
            ),
            Ok(PointerMove {
                family: kalman,
                from: Some(v(2)),
                to: v(5),
            })
        );
    }

    #[test]
    fn promotion_refuses_a_candidate_that_is_not_validating() {
        // A consistent store refuses the request: Internal. A PRODUCTION candidate is not a
        // request error and is covered by the two tests below.
        let kalman = fam("kalman");
        for state in [S::Training, S::Candidate, S::Rejected, S::Superseded] {
            for current in [None, Some((v(1), kalman.clone(), S::Production))] {
                let result =
                    plan_promotion(&kalman, (v(9), kalman.clone(), state), current.clone());
                assert_eq!(
                    result,
                    Err(GovernanceError::IllegalTransition {
                        from: state,
                        to: S::Production,
                        cause: TransitionCause::Promotion,
                    }),
                    "{state}, current {current:?}"
                );
                assert_eq!(result.unwrap_err().code(), EngineErrorCode::Internal);
            }
        }
    }

    #[test]
    fn promoting_the_current_production_version_is_a_request_error() {
        // The store is consistent: the pointer names the candidate, which is PRODUCTION. The
        // request is redundant, which is a request error, not corruption.
        let kalman = fam("kalman");
        let result = plan_promotion(
            &kalman,
            (v(4), kalman.clone(), S::Production),
            Some((v(4), kalman.clone(), S::Production)),
        );
        assert_eq!(
            result,
            Err(GovernanceError::IllegalTransition {
                from: S::Production,
                to: S::Production,
                cause: TransitionCause::Promotion,
            })
        );
        assert_eq!(result.unwrap_err().code(), EngineErrorCode::Internal);
    }

    #[test]
    fn a_production_candidate_the_pointer_does_not_name_is_state_corrupt() {
        // §8.8: a version is PRODUCTION exactly when a pointer names it.
        let kalman = fam("kalman");
        let text = inconsistent(plan_promotion(
            &kalman,
            (v(9), kalman.clone(), S::Production),
            Some((v(1), kalman.clone(), S::Production)),
        ));
        assert!(
            text.ends_with("pointer names version 1"),
            "another version: {text}"
        );
        let text = inconsistent(plan_promotion(
            &kalman,
            (v(9), kalman.clone(), S::Production),
            None,
        ));
        assert!(
            text.ends_with("pointer names no version"),
            "no pointer: {text}"
        );
    }

    #[test]
    fn promotion_refuses_a_candidate_of_another_family() {
        let result = plan_promotion(
            &fam("kalman"),
            (v(9), fam("rapm"), S::Validating),
            Some((v(1), fam("kalman"), S::Production)),
        );
        assert_eq!(
            result,
            Err(GovernanceError::FamilyMismatch {
                version: v(9),
                version_family: fam("rapm"),
                pointer_family: fam("kalman"),
            })
        );
        assert_eq!(result.unwrap_err().code(), EngineErrorCode::Internal);
    }

    #[test]
    fn promotion_refuses_a_pointer_that_does_not_name_a_production_version_of_its_family() {
        let kalman = fam("kalman");
        for state in ModelState::ALL {
            if state == S::Production {
                continue;
            }
            let text = inconsistent(plan_promotion(
                &kalman,
                (v(9), kalman.clone(), S::Validating),
                Some((v(1), kalman.clone(), state)),
            ));
            assert!(text.contains(state.spec_name()), "{text}");
            assert!(text.contains("not PRODUCTION"), "{text}");
        }
        // The pointer names a version of another family.
        let text = inconsistent(plan_promotion(
            &kalman,
            (v(9), kalman.clone(), S::Validating),
            Some((v(1), fam("rapm"), S::Production)),
        ));
        assert!(text.ends_with("belongs to family \"rapm\""), "{text}");
        // The candidate and the pointer name one version, read in two states.
        let text = inconsistent(plan_promotion(
            &kalman,
            (v(9), kalman.clone(), S::Validating),
            Some((v(9), kalman.clone(), S::Production)),
        ));
        assert!(text.contains("read as both VALIDATING"), "{text}");
    }

    /// A store of versions, `(id, family, state)`, for the rollback tests.
    fn store(
        entries: &[(u64, &str, ModelState)],
    ) -> impl Fn(ModelVersionId) -> Option<(ModelFamily, ModelState)> {
        let map: BTreeMap<ModelVersionId, (ModelFamily, ModelState)> = entries
            .iter()
            .map(|(id, family, state)| (v(*id), (fam(family), *state)))
            .collect();
        move |id| map.get(&id).cloned()
    }

    #[test]
    fn rollback_moves_only_differing_families_and_reports_untouched_ones() {
        // Snapshot: a → 1, b → 2. Now: a → 1 (unchanged), b → 5 (moved since), d → 7 (created
        // since).
        let current = [(fam("d"), v(7)), (fam("b"), v(5)), (fam("a"), v(1))];
        let recorded = [(fam("a"), v(1)), (fam("b"), v(2))];
        let version_of = store(&[
            (1, "a", S::Production),
            (2, "b", S::Superseded),
            (5, "b", S::Production),
            (7, "d", S::Production),
        ]);
        let plan = plan_rollback(&current, &recorded, &version_of).unwrap();
        assert_eq!(
            plan,
            RollbackPlan {
                moves: vec![PointerMove {
                    family: fam("b"),
                    from: Some(v(5)),
                    to: v(2),
                }],
                untouched: vec![fam("d")],
            }
        );
        // The plan does not depend on the input order.
        let mut current_rev = current.to_vec();
        current_rev.reverse();
        let mut recorded_rev = recorded.to_vec();
        recorded_rev.reverse();
        assert_eq!(
            plan_rollback(&current_rev, &recorded_rev, &version_of),
            Ok(plan)
        );
    }

    #[test]
    fn rollback_moves_every_differing_family_in_family_order() {
        let current = [(fam("b"), v(20)), (fam("a"), v(10))];
        let recorded = [(fam("b"), v(2)), (fam("a"), v(1))];
        let version_of = store(&[
            (1, "a", S::Superseded),
            (2, "b", S::Superseded),
            (10, "a", S::Production),
            (20, "b", S::Production),
        ]);
        let plan = plan_rollback(&current, &recorded, &version_of).unwrap();
        assert_eq!(
            plan.moves,
            vec![
                PointerMove {
                    family: fam("a"),
                    from: Some(v(10)),
                    to: v(1),
                },
                PointerMove {
                    family: fam("b"),
                    from: Some(v(20)),
                    to: v(2),
                },
            ]
        );
        assert!(plan.untouched.is_empty());
    }

    #[test]
    fn rollback_to_the_current_pointers_moves_nothing() {
        let pointers = [(fam("a"), v(1)), (fam("b"), v(2))];
        let version_of = store(&[(1, "a", S::Production), (2, "b", S::Production)]);
        let plan = plan_rollback(&pointers, &pointers, &version_of).unwrap();
        assert_eq!(
            plan,
            RollbackPlan {
                moves: vec![],
                untouched: vec![],
            }
        );
        // An empty snapshot moves nothing and leaves every pointer untouched.
        let plan = plan_rollback(&pointers, &[], &version_of).unwrap();
        assert!(plan.moves.is_empty());
        assert_eq!(plan.untouched, vec![fam("a"), fam("b")]);
    }

    #[test]
    fn rollback_refuses_a_target_that_is_not_superseded_for_the_whole_plan() {
        let current = [(fam("a"), v(10)), (fam("b"), v(20))];
        for state in ModelState::ALL {
            if state == S::Superseded {
                continue;
            }
            // Family a would restore cleanly; family b's target is not restorable.
            let recorded = [(fam("a"), v(1)), (fam("b"), v(2))];
            let version_of = store(&[
                (1, "a", S::Superseded),
                (2, "b", state),
                (10, "a", S::Production),
                (20, "b", S::Production),
            ]);
            let result = plan_rollback(&current, &recorded, &version_of);
            assert_eq!(
                result,
                Err(GovernanceError::NotRestorable {
                    family: fam("b"),
                    version: v(2),
                    state,
                }),
                "{state}"
            );
            // No consistent store records a version that is now neither SUPERSEDED nor named by
            // its family's pointer, so the refusal is persisted-state corruption.
            assert_eq!(result.unwrap_err().code(), EngineErrorCode::StateCorrupt);
        }
    }

    #[test]
    fn rollback_refuses_a_recorded_family_with_no_pointer_now() {
        // Snapshot: a → 1, c → 3; now only a → 10. Nothing removes a pointer, so family c's
        // missing pointer is corruption, and the rollback is refused, family a included.
        let version_of = store(&[
            (1, "a", S::Superseded),
            (3, "c", S::Superseded),
            (10, "a", S::Production),
        ]);
        let text = inconsistent(plan_rollback(
            &[(fam("a"), v(10))],
            &[(fam("a"), v(1)), (fam("c"), v(3))],
            &version_of,
        ));
        assert!(
            text.ends_with(
                "the snapshot records version 3 for family \"c\", which has no production \
                 pointer now"
            ),
            "{text}"
        );
    }

    #[test]
    fn rollback_refuses_inconsistent_inputs() {
        let ok = store(&[(1, "a", S::Superseded), (10, "a", S::Production)]);
        // A recorded version the store does not have.
        let text = inconsistent(plan_rollback(
            &[(fam("a"), v(10))],
            &[(fam("a"), v(99))],
            &ok,
        ));
        assert!(text.contains("does not have"), "{text}");
        // A recorded version of another family.
        let other = store(&[(1, "z", S::Superseded), (10, "a", S::Production)]);
        let text = inconsistent(plan_rollback(
            &[(fam("a"), v(10))],
            &[(fam("a"), v(1))],
            &other,
        ));
        assert!(
            text.ends_with("but that version belongs to family \"z\""),
            "{text}"
        );
        // The pointer that would move does not name a PRODUCTION version of its family.
        for now in [
            Some((fam("a"), S::Superseded)),
            Some((fam("a"), S::Rejected)),
            None,
        ] {
            let version_of = move |id: ModelVersionId| {
                if id == v(1) {
                    Some((fam("a"), S::Superseded))
                } else {
                    now.clone()
                }
            };
            let text = inconsistent(plan_rollback(
                &[(fam("a"), v(10))],
                &[(fam("a"), v(1))],
                version_of,
            ));
            assert!(text.contains("not PRODUCTION"), "{text}");
        }
        let text = inconsistent(plan_rollback(
            &[(fam("a"), v(10))],
            &[(fam("a"), v(1))],
            store(&[(1, "a", S::Superseded), (10, "z", S::Production)]),
        ));
        assert!(
            text.ends_with("names version 10, which belongs to family \"z\""),
            "{text}"
        );
        // A family listed twice, in either list.
        let text = inconsistent(plan_rollback(
            &[(fam("a"), v(10)), (fam("a"), v(11))],
            &[],
            &ok,
        ));
        assert!(
            text.ends_with("family \"a\" appears twice in the production pointers"),
            "{text}"
        );
        let text = inconsistent(plan_rollback(
            &[],
            &[(fam("a"), v(1)), (fam("a"), v(2))],
            &ok,
        ));
        assert!(
            text.ends_with("family \"a\" appears twice in the snapshot"),
            "{text}"
        );
        // A version listed for two families, in either list.
        let text = inconsistent(plan_rollback(
            &[(fam("a"), v(10))],
            &[(fam("a"), v(1)), (fam("b"), v(1))],
            &ok,
        ));
        assert!(
            text.ends_with("version 1 appears for more than one family in the snapshot"),
            "{text}"
        );
        let text = inconsistent(plan_rollback(
            &[(fam("a"), v(10)), (fam("b"), v(10))],
            &[],
            &ok,
        ));
        assert!(
            text.ends_with(
                "version 10 appears for more than one family in the production pointers"
            ),
            "{text}"
        );
    }
}
