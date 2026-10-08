---
adr-id: 005
status: Accepted
date: 2026-08-20
deciders: Product/Architecture owner
supersedes:
superseded-by:
---

# ADR-005 — Amend the frozen `check-sqlx` recipe with `--workspace`

## Status
Accepted 2026-08-20. This ADR **amends ADR-001 D5**, which froze the verification recipes.
It required explicit owner approval and got it, on the record, before the change was made.

## Context

ADR-001 D5 froze the seven `verify` recipes: where a recipe cannot pass, the repository
changes, not the recipe. That ruling held through four earlier obstacles — it forced the SQLx
pull-forward (ADR-002), and it was vindicated when `cargo nextest run --workspace` turned out
to exit 4 on an empty workspace.

`check-sqlx` is the case where it could not hold. The recipe reads:

```
cargo sqlx prepare --check -- --lib
```

Against sqlx-cli 0.9 at a **virtual** workspace root this fails unconditionally:

```
error: failed to get package in current working directory, pass `--workspace` if running from a workspace root
```

The failure is in cargo package discovery, before sqlx looks at a database, a query, or the
cache. **No change to the repository can reach it.** Every avenue was tested:

| Attempt | Result |
|---|---|
| Provide crates, migration, query, database, committed `.sqlx` | Still exit 1 — never gets that far |
| Add a root `[package]` to make the workspace non-virtual | Would not work *and* would be wrong: `prepare -- --lib` would then scan the **root** package's queries, not `grid-persistence`'s, producing an empty cache |
| `sqlx.toml` / env override to imply `--workspace` | No such setting; `--workspace` is CLI-only |
| Pin sqlx-cli back to 0.8 | Untested. Even if it accepted the form, it reintroduces exactly the library/CLI skew ADR-003 removed: a 0.8-written cache consumed by the 0.9 library |

With `--workspace` added, the recipe exits 0 — and the gate is real, not merely quiet.
Verified by mutation: editing the query so the committed cache goes stale produces
`error: prepare check failed: .sqlx is missing one or more queries`, exit 1; restoring it
returns exit 0.

## Decision

**Add `--workspace` to `check-sqlx` in all three implementations**, keeping them in parity:

```
cargo sqlx prepare --check --workspace -- --lib
```

Changed in `justfile`, `scripts/verify.sh`, and `scripts/verify.ps1`. No other recipe is
touched. `scripts/check-verify-parity.sh` confirms the three stay in agreement.

### Why this does not violate the spirit of D5

D5 exists to stop a check being **weakened** to make a failure disappear — the pattern
`alpha-spec.md` §12.7 and Appendix D prohibit. This is the opposite: the recipe went from
*always erroring on argument parsing* to *actually verifying query-cache freshness across the
workspace*. Before this change the gate could never have caught cache drift, because it never
reached the cache. Coverage increased.

D5's procedural requirement is also honoured. The rule was not reinterpreted by the
implementer to get unblocked; the recipe was left untouched, the analysis and every rejected
alternative were put to the owner, and the change was made only after approval.

## Consequences

**Easier.** `just verify` can pass. The SQLx offline cache is genuinely gated against drift
from the first work package rather than from whenever someone noticed.

**Harder.** D5 is no longer absolute — there is now a precedent for amending a frozen recipe.
The precedent is deliberately narrow: an amendment requires demonstrating that *no* repository
change can satisfy the recipe, that the amendment strengthens rather than weakens the check,
and explicit owner approval recorded in an ADR. Convenience is not a qualifying reason.

**Inherited.** P1-02 adds real domain queries; `check-sqlx` will exercise them from day one.

## Compliance

- `scripts/check-verify-parity.sh` — the three implementations cannot drift apart.
- The drift-detection mutation test above should be re-run whenever `check-sqlx` changes.
- Any future recipe amendment requires its own ADR meeting the three conditions named above.

## Alternatives considered

**Leave `check-sqlx` failing and ship P1-00 red.** Literal D5 compliance, but it fails the
package's central acceptance criterion and normalizes a red gate for every later package.

**Pin sqlx-cli to 0.8.** Untested, and even on success it reintroduces the version skew
ADR-003 removed. Trading a working security fix for one word in a recipe is a bad exchange.

**Drop the SQLx pull-forward.** Strictly worse: `check-sqlx` still errors on package discovery
with zero crates, and `test-rust` returns to exiting 4 on no tests — two red gates instead of
one, and P1-02 inherits an unproven toolchain.
