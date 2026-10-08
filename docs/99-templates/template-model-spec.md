---
model-spec-id:
status: Draft            # Draft | Approved | Superseded
statistical-owner:
version:
supersedes:
---

<!--
Fields follow engine-spec.md §6.8, the list of 17 contents that every production model spec
carries. Items 1-11 are carried from the superseded alpha-spec §6.6; items 12-17 are the GRID
additions. The headings below are the minimum. A spec may use its own numbered structure (the
house format of docs/05-model-specs/) if a "Template conformance" table maps every heading below
to the section that covers it. A heading that does not apply says "Not applicable" plus one line
of justification, or for Synthetic recovery, why no planted quantity exists.

Updated in P0-01 (ADR-011, ADR-012): the §6.8 fields that the P1-00 template lacked were added
(split rules, complexity, train/serve parity, statistical intent, oracle counterpart, parity,
known oracle defects, open decisions, synthetic recovery). The existing headings are unchanged and
keep their relative order.
-->

# Model spec — <component name>

Contract before implementation (engine-spec §1.6). This document is authority level 3
(engine-spec §1.5) and exists **before** the code that implements it. The statistical owner approves
the equations, and the implementing agent may not redefine them (engine-spec §6.8 rules 1 and 8).

## Statistical intent
(§6.8 item 12.) What the component may estimate, and what it treats as nuisance (engine-spec §6.2).
The Rust module documentation restates this in the same terms (§6.8 rule 9). Changing it is a
model-spec change.

## Target
(§6.8 item 1.) The quantity predicted, its units, and its support (e.g. non-negative real, count,
probability). Name the exact stat-vector component or scoring output.

## Inputs
(§6.8 item 2.) Each feature: source, units, null semantics, as-of rule (engine-spec §4.5, including
the publication-lag axis), and the feature-schema version it belongs to. Every input must be
point-in-time correct, with nothing knowable only after lock. "No data" is an explicit missing value
with a declared reason, never 0.0 (§6.8 rule 12).

## Equations
(§6.8 item 3.) The estimator in full, with every symbol defined. Include:
- the objective;
- any regularization and its penalty term;
- the update rule for online and incremental components.

Prose is not a substitute.

## Priors
(§6.8 item 4.) Prior families, hyperparameters, and where they come from.
- For empirical Bayes, state what is pooled over and how the shrinkage weight is derived.
- For NCAA priors, state the translation and the low-evidence eligibility rule (engine-spec §2.5,
  §6.5).

## Constraints
(§6.8 item 4.) Invariants that must hold. Examples:
- non-negativity;
- sum-to-team-total allocation;
- monotonicity;
- probability normalization;
- covariance positive-semi-definiteness.

Also state the transformations and the permitted parameter ranges. State what happens when a
constraint is violated. Never silently clamp.

## Training and validation split rules
(§6.8 item 5.) How training, validation and test data are separated in time and by player or game.
State the rolling-origin design, the warm-up window, and the three-season window rule
(engine-spec §2.4). Hyperparameters are never selected on live benchmark test weeks or competitor
projections (§6.8 rule 6).

## Seed policy
(§6.8 item 6.) Every stochastic path is seeded and reproducible. State:
- the seed source;
- how it is versioned;
- how it enters the prediction version.

Identical inputs must yield identical outputs.

## Tolerances
(§6.8 item 7.) Numerical error bands for golden tests, with units and the machine and dataset
assumptions they hold under. State absolute vs. relative and the comparison method.

Also state the **failure conditions**. Each of these is an explicit typed failure (§6.8 rule 4):
- NaN and infinity;
- a singular system;
- non-convergence;
- an invalid probability;
- share overflow;
- an impossible stat.

## Computational complexity
(§6.8 item 8.) Expected time and memory for the declared dimensions: players, plays, seasons, draws
and features. Name the dominant term and the size at which it is measured (engine-spec §13).

## Reference examples
(§6.8 item 9.) Worked numeric examples an implementer can verify against by hand or fixture, with
expected outputs to the stated tolerance. These become the golden tests. Goldens are never
regenerated merely to make a failure disappear (§6.8 rule 3).

## Training/serving parity
(§6.8 item 10.) How the quantities used in fitting match the quantities used at prediction time:
- the same features;
- the same as-of slice;
- the same scale;
- the same version.

Where the component runs both in batch and incrementally, state the two-path equivalence it must
satisfy (engine-spec §12.3).

## Explanation fields
(§6.8 item 11.) What the component contributes to the explanation payload of the engine output
contract (engine-spec §5.5, §6.7): driver names, units, sign conventions, and how contributions are
attributed. Filtered, smoothed and predictive estimates are distinct named outputs (§6.8 rule 15).

## Oracle counterpart
(§6.8 item 13.) The module and function under `reference/python/` that this component corresponds
to, at the imported revision, or "none". Name any part of the oracle that is deliberately **not**
ported (ADR-012 §1).

## Parity
(§6.8 item 14.) Declared before the parity test is written (§6.8 rule 10):
- **Tolerance class and tolerance.** A, A′, B, C or D, with its numeric criterion (engine-spec
  §7.12.5).
- **Fixtures.** The fixture IDs that carry the target, and their manifest
  (`docs/03-contracts/parity-fixture-contract.md`).
- **Oracle status targeted.** Legacy or corrected, and which correction-ledger entries must run
  first (engine-spec §7.12.2).
- **Injection.** Which upstream outputs are injected, if the comparison is stage-isolated.

## Known oracle defects
(§6.8 item 15.) Every known issue that touches the component, by ID (`docs/00-meta/known-issues.md`),
each with the required engine behaviour. List the deliberate divergences recorded in
`reference/python/PARITY.md` as well. A known defect is never required behaviour (§6.8 rule 11).

## Open parameters and decisions
(§6.8 item 16.) Each open parameter or decision, with its DR ID (`docs/00-meta/decision-register.md`,
engine-spec Appendix F) and its current status. An open item that changes behaviour keeps the
dependent work package from being Ready (engine-spec §8.16.1).

## Synthetic recovery
(§6.8 item 17.) The planted quantity this component must recover on the synthetic world, its gate
and its threshold (engine-spec §6.9, §7.13), and the generator version it is measured on. If the
synthetic world does not plant the estimand, say so, and make no recovery claim (§6.8 rule 13).

## Validation and promotion
Backtest design, the metric this component is judged on, and the threshold that gates promotion.
Thresholds are versioned before results are seen (engine-spec §12.7).

## Known limitations
Where the model is expected to be weak, and what would falsify it.
