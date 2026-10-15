---
adr-id:
status: Proposed         # Proposed | Accepted | Superseded | Rejected
date:
deciders:
supersedes:
superseded-by:
---

# ADR-<NNN> — <Short decision title>

## Status
Proposed | Accepted | Superseded | Rejected — with date and the deciding role
(alpha-spec.md 1.4). An ADR ranks above contracts and work packages but **below**
`final-build-spec.md` and `alpha-spec.md`; it can never override either.

## Context
The forces in play: the requirement, the constraint, the conflict, or the ambiguity that
made a decision necessary. State what was true before. Cite authority sections.

## Decision
What was decided, in the active voice. One decision per ADR.

## Consequences
What becomes easier, what becomes harder, and what future work inherits.
Include the negative consequences — an ADR listing only benefits is not finished.

## Compliance
How conformance is enforced and detected: the script, test, CI job, or review step that
fails when this decision is violated. A decision with no enforcement is a preference.

## Alternatives considered
Each rejected option and the specific reason it lost. "We didn't think of it" is not a
reason; if an obvious option was not evaluated, say so.
