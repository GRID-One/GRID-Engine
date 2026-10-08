---
work-package-id:
status: Draft            # Draft | Ready | In Progress | Review | Done | Blocked
risk-class: Medium       # Low | Medium | High | Release-critical
owner:
implementer:
reviewer:
phase: 1
---

<!--
Structure follows engine-spec.md Appendix B exactly, in order. This file is the canonical copy:
Appendix B summarizes it, and the two change together (engine-spec Appendix B).

Two documented additions, ratified in P1-00 (docs/02-adr/001-repo-bootstrap-decisions.md):
  1. YAML frontmatter. docs/00-meta/dashboard.md queries these fields through Dataview, so the keys
     are hyphenated to match that query. `phase` was `alpha-phase` until P0-01.
  2. "Security and licensing considerations". engine-spec §8.16's field list requires it.

Updated in P0-01 (ADR-011) to the engine structure:
  - a single engine-spec authority line;
  - "Observable outcome" replaces "User-visible outcome";
  - new "Owner decisions depended on" and "Oracle parity targets" sections (engine-spec §8.16);
  - screenshots removed from the evidence list.

Copy this file to docs/01-work-packages/p<phase>-<nn>-<slug>.md. The lowercase ID must appear in the
filename, because scripts/check-traceability.sh resolves a cited ID by filename glob. Fill every
section. A section that does not apply says "Not applicable", plus one line of justification.
-->

# <WORK_PACKAGE_ID> — <Title>

## Status
Draft | Ready | In Progress | Review | Done | Blocked

## Ownership and risk
- Owner:
- Implementer:
- Reviewer:
- Risk class: Low | Medium | High | Release-critical
- Required human approvals:
- Agent budget (engine-spec §8.18.1): max turns, wall-clock timeout, concurrent writers, cost ceiling

## Authority
- Engine spec (`engine-spec.md`): <sections>
- ADRs/contracts/model specs/provider manifests:

## Owner decisions depended on
DR IDs from `docs/00-meta/decision-register.md`, each with its current status.

**The package is not Ready while any of them is unratified** (engine-spec §8.16.1). A proposed
default is not a ratification.

## Objective
One measurable outcome.

## Observable outcome
What an operator or consumer can observe through the CLI, library API, a report or a file,
or "none" for infrastructure work.

## Preconditions
Merged packages, fixtures, decisions, and environment requirements.
A package is Ready only when dependencies are merged and passing.

## Scope
- Modules/files expected to change
- Contracts consumed
- Contracts changed

## Non-goals
Explicitly excluded work.

## Inputs and fixtures
Named, versioned, sanitized inputs. Parity fixtures are synthetic-only (DR-A11).

## Oracle parity targets
Give, for each target:
- the oracle module and function under `reference/python/` (the oracle counterpart);
- the gate;
- the tolerance class (engine-spec §7.12.5);
- the fixture IDs and the manifest hash;
- the known oracle defects (KI IDs), with their correction-ledger status.

Write "none" if there are no targets.

## Implementation constraints
Architecture, dependency, timing, determinism, security, and licensing rules.

## Security and licensing considerations
Secrets touched, permission changes, new hosts, provider license and retention terms,
redistribution limits. "None" is an acceptable answer; silence is not.

## Acceptance criteria
- [ ] Criterion with objective evidence
- [ ] Criterion with objective evidence

## Verification
```text
<targeted commands>
<canonical verify command>
```
Verification recipes are a frozen contract. Make the repository satisfy them; never edit a
recipe to make a failure disappear.

## Numerical/performance tolerances
Declared units, error bands, machine/dataset assumptions, or "not applicable."

## Migration and rollback
Forward path, upgrade fixtures, backup/rollback behavior, or "not applicable."
Migrations are append-only.

## Evidence required
Test output, benchmark results, hashes, generated files, oracle parity report, and review report.
Generate the manifest with `just evidence <WORK_PACKAGE_ID>`; never hand-write it.

## Stop/decision conditions
Questions the implementer must escalate rather than decide silently (engine-spec §8.17 stop conditions).

## Follow-up
Deferred work with risk statement.
