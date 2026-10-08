---
adr-id: 007
status: Accepted
date: 2026-08-20
deciders: Product/Architecture owner
supersedes:
superseded-by: "011 (in part, proposed): the chain listing naming test-ffi and its step count"
---

# ADR-007 — `just verify` covers the repository guards and doctests

## Status
Accepted 2026-08-20. **Second amendment to ADR-001 D5**, granted under the three conditions
ADR-005 established. Raised by adversarial review of PR #1 (findings M2 and M10).

## Context

`alpha-spec.md` §8.11 says the verification orchestration includes, among other things,
"license/dependency/security scans", **"secret scanning"**, **"traceability checks from changed
code to work package and acceptance criteria"**, and **"doctests"**.

Neither was in the canonical command.

**M2 — guards absent.** The six guard scripts existed only in the CI `guards` job. A developer
or agent running `just verify` — the command `CLAUDE.md`, the authority index and this work
package all name as canonical — got no secret scan and no traceability check. Combined with M1
(the fail-open defect, now fixed), the only place secret scanning genuinely ran was one CI job
on one code path.

**M10 — doctests absent.** `cargo nextest run --workspace` does not execute doctests at all;
`cargo test --doc` is a separate invocation. Zero doctests exist today, so nothing was missed —
but the gate would have gone silently blind the moment P1-01 added a doc example, which is
exactly the kind of latent hole a bootstrap package exists to close.

Neither could be fixed by changing the repository: the recipes simply did not invoke the checks.

## Decision

**Add two steps to the verify chain, in all three implementations.**

```
just check-fmt · check-lint · check-sqlx · test-rust · test-doc
              · test-ffi · audit · check-guards · check-typos
```

- `test-doc` → `cargo test --workspace --doc`
- `check-guards` → the guard behaviour suite (`tests/guards/run.sh`) followed by all six guard
  scripts

Applied identically to `justfile`, `scripts/verify.sh` and `scripts/verify.ps1`;
`check-verify-parity.sh` confirms the first two agree, now at 16 steps.

`check-guards` runs `tests/guards/run.sh` **first**. A guard that is broken should fail on its
own test suite before it renders a verdict on the repository — M1 was a guard reporting OK while
scanning nothing, and the suite is what makes that self-evident.

### Conditions from ADR-005, restated and met

1. **No repository change can satisfy the recipe as written** — the checks were not invoked at all.
2. **The amendment strengthens rather than weakens** — coverage goes from 7 checks to 9, adding
   two §8.11 items that were absent. Nothing is relaxed.
3. **Explicit owner approval, recorded** — obtained before the change was made.

## Consequences

**Easier.** The canonical command is now genuinely canonical: secret scanning, traceability and
doctests run wherever `just verify` runs, not only in CI. A developer cannot commit a secret and
see green locally.

**Harder.** `just verify` is slower and needs git history — `check-traceability` and
`check-secrets` diff against `origin/main`, so a shallow clone degrades them. Both fail closed
or report an explicit unverified state rather than passing silently. `check-env-contract` will
also fail `just verify` for anyone whose environment overrides `DATABASE_URL`, which is the
point but will surprise people.

**D5 is now amended twice.** That is a trend worth naming: the "frozen" contract has proven to
be frozen-except-when-the-owner-rules-otherwise. The three conditions are what keep that from
becoming meaningless, and both amendments have added coverage rather than removed it. A future
amendment that *reduces* coverage should be refused outright, not weighed against these.

## Compliance

- `check-verify-parity.sh` — the three implementations cannot drift; now order-sensitive.
- `tests/guards/run.sh` — 12 cases; runs first inside `check-guards`, and in the CI guards job.
- Any further recipe amendment needs its own ADR meeting the three conditions above.

## Alternatives considered

**Leave both as known limitations.** Keeps D5 inviolate at the cost of a canonical command that
omits three things §8.11 says it includes, for every package P1-01…P1-10. Rejected by the owner.

**Defer to P1-11** alongside `-Scope Changed` and CI caching. Reasonable for M10, which is
latent; wrong for M2, which is a live gap in a security control today. Rejected as a pair.

**Add the guards to CI only, more thoroughly.** Already the status quo, and the status quo is
what M2 objects to: one code path, one environment, no local coverage.
