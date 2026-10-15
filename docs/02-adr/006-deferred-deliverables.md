---
adr-id: 006
status: Accepted
date: 2026-08-20
deciders: Product/Architecture owner (ratification pending)
supersedes:
superseded-by: "011 (in part, proposed): item 6 app/ deliverables and the toolchains/flutter.version sentence"
---

# ADR-006 — §8.7 and §9.2 deliverables deferred out of P1-00

## Status
Accepted 2026-08-20 as a **recorded deviation awaiting architecture-owner ratification.**
Raised by adversarial review of PR #1 (finding M7).

## Context

`alpha-spec.md` §8.7 sketches a repository tree and §9.2 lists the Phase 1 "AI implementation
foundation". Several named items are absent from P1-00, and — unlike the fixture deferral,
which ADR-001 records explicitly — they were disposed of by a single row in the authority
index's alias table reading "not yet created — P1-11".

That is not good enough. §9.2 is level-2 authority; a work package at level 5 cannot silently
drop items from it. The reviewer's objection is procedural and correct: the fixtures deferral
was handled well and set the standard these should have met.

**Absent:**

| Item | Named in | Status |
|---|---|---|
| `.claude/agents/`, `.claude/skills/` | §8.7 tree; §9.2 "explorer, implementer, numerical-review, data-contract, security-review, and adversarial-review agent definitions" | deferred |
| `schemas/`, `fixtures/`, `tests/`, `benches/` | §8.7 tree | `tests/` now exists (guard suite); rest deferred |
| `docs/runbooks/`, `docs/model-cards/`, `docs/traceability/` | §8.7 tree; §9.2 names a traceability matrix explicitly | deferred |
| `app/lib/main.dart` | §8.7; permitted as a placeholder by P1-00's own non-goals | deferred |
| `toolchains/native-dependencies.lock` | §8.7 | deferred to P1-11 with XGBoost vendoring |
| `app/pubspec.lock` | §8.7, §8.11 | blocked — Flutter is not installed in the build environment |

## Decision

**Defer them, explicitly and with reasons, rather than by omission.** Each is assigned a
package and a rationale.

1. **Agent definitions** (`.claude/agents/`, `.claude/skills/`) → **P1-01**. §9.2 qualifies
   these with "when supported" (§8.10). More importantly, an agent definition written before
   any domain code exists would encode guesses about a codebase that does not exist yet. The
   adversarial-review role has now been exercised once *without* a committed definition, which
   produced a better artifact than a speculative definition would have. P1-01 writes them from
   that experience.
2. **`schemas/`, `fixtures/`** → **P1-03/P1-04**, alongside the provider contracts that give
   them meaning. Consistent with the fixtures deferral already recorded in ADR-001.
3. **`benches/`** → **P1-07**, the first package with numerical code worth measuring. `just bench`
   already says so.
4. **`docs/runbooks/`, `docs/model-cards/`** → **P1-11**. Both describe operating and shipping a
   system that does not yet run.
5. **`docs/traceability/`** → **superseded in substance.** §9.2 asks for a traceability matrix;
   `scripts/check-traceability.sh` now enforces per-file traceability executably, which is
   stronger than a matrix document that drifts. If the owner wants the document as well, it is
   P1-11.
6. **`app/lib/main.dart`, `app/pubspec.lock`, `toolchains/native-dependencies.lock`** → **P1-10/P1-11.**
   `pubspec.lock` is **blocked, not deferred**: Flutter is not installed in this environment and
   a hand-written lockfile would be a fabrication. `toolchains/flutter.version` is committed now
   so the version intent is at least recorded.

## Consequences

**Easier.** Every §8.7/§9.2 absence now has a named owner package and a reason, so a fresh
session can tell "deliberately deferred" from "forgotten" — which is the whole point of P1-00.

**Harder.** P1-01 inherits work that §9.2 arguably wanted first. The agent definitions in
particular are a real gap: the specialized roles §8.10 describes are how §9.2 intends quality
to be maintained, and until they exist that depends on whoever is driving.

**Risk accepted.** If the architecture owner disagrees with any line, the deferral is reversible
— none of these items is depended upon by code already merged.

## Compliance

- This ADR is the record. `docs/00-meta/authority-index.md`'s alias table now points here
  instead of asserting "P1-11" on its own authority.
- Each deferred item names a package; those packages' Preconditions should cite this ADR.

## Alternatives considered

**Build them all in P1-00.** Honest to §9.2, but agent definitions and schemas written before
the domain exists would be speculative, and P1-00 is already substantially over its stated
scope. Rejected.

**Leave the alias-table row.** What the reviewer objected to. A table cell is not a decision
record and carries no rationale, no owner, and no reversal path. Rejected.
