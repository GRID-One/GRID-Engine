---
adr-id: 008
status: Accepted
date: 2026-08-20
deciders: Product/Architecture owner
supersedes:
superseded-by:
---

# ADR-008 — Validate the scope argument in `scripts/verify.sh`

## Status
Accepted 2026-08-20. This ADR **amends ADR-001 D5** for the third time. It required explicit
owner approval and got it, on the record, before the change was made.

## Context

`scripts/verify.sh` took its scope from `SCOPE="${1:-full}"` and ran its checks inside
`if [[ "$SCOPE" == "full" || "$SCOPE" == "changed" ]]`. Any other value skipped the entire
body and fell through to the final line:

```
$ ./scripts/verify.sh Full   ; echo $?     # 0 -- and zero checks ran
[verify] Starting verification scope: Full
[verify] All checks passed.
$ ./scripts/verify.sh smoke  ; echo $?     # 0 -- likewise
```

`Full` is not a hypothetical typo. It is the capitalization **every authority document in this
repository prints**, because the Windows form takes it that way:

| Document | Line |
|---|---|
| `CLAUDE.md` | 25 |
| `docs/CLAUDE.md` | 48 |
| `docs/00-meta/authority-index.md` | 55 |

A reader who copies the documented capitalization onto the bash entry point gets a green
"All checks passed." over nothing at all. This is the same fail-open class as the `check-secrets.sh`
defect the first adversarial review found, in the one script whose entire job is to be the gate.

`scripts/verify.ps1` never had the hole. Its `param([ValidateSet("Full","Changed")]…)` rejects
anything else at binding time, case-insensitively, before the script body runs. The two
implementations were not equivalent, and `check-verify-parity.sh` could not see it: parity
compares the *command lists*, which were and remain identical.

### Why no change outside the frozen file can fix it

ADR-005's first condition asks whether the repository can be changed instead of the recipe.
Here the defect is **located in the frozen file itself** — in its argument dispatcher, above the
checks — so by construction nothing outside it reaches the fault. The alternatives were weighed:

| Attempt | Result |
|---|---|
| Change the docs to print lowercase `verify.sh full` | Removes one instance of the trap and leaves the mechanism. `verify.sh smoke`, `verify.sh --help`, and any future typo still exit 0 having verified nothing |
| Add a guard-suite case asserting the current behaviour | Codifies the defect as intended behaviour |
| Wrap `verify.sh` in a validating caller | The wrapper is not what §8.11, `CLAUDE.md` and CI invoke; the unguarded path stays reachable and documented |
| Leave it, record as a limitation | A silent-pass verification entry point is not a limitation, it is the failure Appendix D #1 names |

## Decision

**Normalize and validate the scope argument before anything else runs.** Replaces the single
line `SCOPE="${1:-full}"`:

```bash
SCOPE="$(printf '%s' "${1:-full}" | tr '[:upper:]' '[:lower:]')"
case "$SCOPE" in
full|changed) ;;
*) echo "[verify] unknown scope: '${1}' (expected Full or Changed)" >&2; exit 2 ;;
esac
```

Exit 2 distinguishes "you asked for something that does not exist" from a check failure (1).
Case-insensitive acceptance makes the bash and PowerShell entry points behave alike, so the
capitalization the documents print works on both.

**No check body is touched.** All 16 verification steps stay byte-identical, and
`check-verify-parity.sh` still reports `justfile and scripts/verify.sh agree on 16 verification
step(s)`. `scripts/verify.ps1` needs no change — `[ValidateSet]` already does this.

The block is written at **column 0** deliberately. `check-verify-parity.sh` extracts commands
with `grep -E '^[[:space:]]+[a-zA-Z._/]'`; an indented `full|changed) ;;` arm would be read as a
verification step and break parity against the justfile. A comment in the script records this so
a later reindent does not silently break the guard that guards the guards.

Verified across the full argument space, with `cargo` stubbed so the guard is observed in
isolation:

```
scope='Full'     exit=99 | [verify] Starting verification scope: full
scope='FULL'     exit=99 | [verify] Starting verification scope: full
scope='full'     exit=99 | [verify] Starting verification scope: full
scope='Changed'  exit=99 | [verify] Starting verification scope: changed
scope='changed'  exit=99 | [verify] Starting verification scope: changed
scope='smoke'    exit=2  | [verify] unknown scope: 'smoke' (expected Full or Changed)
scope=''         exit=99 | [verify] Starting verification scope: full
scope='--help'   exit=2  | [verify] unknown scope: '--help' (expected Full or Changed)
scope='full '    exit=2  | [verify] unknown scope: 'full ' (expected Full or Changed)
```

(99 is the stub's exit code, proving control reached the first real check.) Regression cases are
committed in `tests/guards/run.sh`, so `just verify` fails if the guard is ever removed.

## Consequences

**Easier.** The documented invocation works. A mistyped scope is a loud error instead of a
false green.

**Harder.** D5 has been amended four times: ADR-005 (`--workspace`), ADR-007 (`test-doc`,
`check-guards`), this one, and ADR-009 (exit-code propagation in `verify.ps1` — found because
this ADR's guard suite finally made the Windows job print a real verdict). ADR-007 named the trend and the standard it implies, and that
standard holds here — **all three amendments increased coverage; not one relaxed a check.** An
amendment that *reduces* coverage should still be refused outright rather than weighed against
these three. This one is additionally the narrowest of the three: it is the only one that leaves
every verification command byte-identical.

**Inherited.** `-Scope Changed` remains an alias for Full in both scripts, deferred to P1-11.
That deferral is now safer, because the argument that reaches the alias is validated.

## Compliance

- `tests/guards/run.sh` — accepted and rejected scopes are regression-tested, and the suite runs
  inside `just verify` (ADR-007).
- `scripts/check-verify-parity.sh` — still 16 steps; run it after any edit to the guard block.
- The three ADR-005 conditions apply unchanged to any future amendment.

## Alternatives considered

**Lowercase the documented commands instead.** Cheapest, and it violates no frozen file. Rejected:
it fixes one symptom of a fail-open dispatcher and leaves the dispatcher. §8.11 names
`verify.sh` as canonical; a canonical gate that can pass without running is worse than a
documentation inconsistency.

**Mirror PowerShell exactly and reject `Full` in bash too (case-sensitive).** Consistent in one
sense and hostile in another: PowerShell's `ValidateSet` is itself case-insensitive, so
case-sensitive bash would still diverge from it, in the direction that rejects what the docs print.

**Implement `-Scope Changed` properly at the same time.** Correct eventually and out of scope
here: it is a behaviour change to a frozen file for a feature P1-11 owns, and it would enlarge a
blocker fix into a design change.
