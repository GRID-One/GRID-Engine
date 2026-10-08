---
adr-id: 003
status: Accepted
date: 2026-08-20
deciders: Product/Architecture owner, Security/Release owner
supersedes:
superseded-by:
---

# ADR-003 — Upgrade sqlx 0.8 → 0.9

## Status
Accepted 2026-08-20. Approved explicitly by the owner during P1-00 after the advisory below
was surfaced and escalated rather than worked around.

## Context

The frozen `audit` verify recipe runs `cargo deny check` followed by `cargo audit`.
`cargo audit` failed:

```
RUSTSEC-2023-0071 — Marvin Attack: potential key recovery through timing sidechannels
crate: rsa 0.9.10   severity: 5.9 (medium)   Solution: No fixed upgrade is available!
```

Dependency path: `sqlx 0.8` → `sqlx-macros` → `sqlx-macros-core` → `sqlx-mysql` → `rsa`.

GRID-Alpha uses SQLite exclusively (`final-build-spec.md` §8). The MySQL driver is never
instantiated, so the vulnerable code is unreachable at runtime — but it is present in
`Cargo.lock`, and `cargo audit` reads the lockfile.

Two remedies were tested empirically rather than assumed:

| Attempt | Result |
|---|---|
| `default-features = false` with only the sqlite/migrate/macros features on sqlx 0.8 | **Ineffective.** `sqlx-macros-core` 0.8 depends on all three drivers unconditionally so the `query!` macro can serve any backend. `rsa` remains. Dropping `macros` is not an option — `final-build-spec.md` §8.2 mandates compile-time-checked queries. |
| Upgrade to sqlx 0.9, feature set unchanged | **Effective.** `rsa` leaves the graph entirely; `cargo audit` exits 0. |

A second, independent defect pointed the same way: `just bootstrap` installs `sqlx-cli`
unpinned, which resolves to 0.9.0, while the workspace pinned the library at 0.8. The CLI
generates the `.sqlx` cache that the library consumes, so a major-version skew between them is
a latent cache-format hazard for ADR-002. The upgrade closes both problems with one change.

## Decision

**Pin `sqlx = "0.9"` in `[workspace.dependencies]`, feature set unchanged.**

```toml
sqlx = { version = "0.9", features = ["runtime-tokio", "sqlite", "migrate", "chrono"] }
```

This is deliberately the *minimal* change. Feature narrowing was not adopted: it does not fix
the advisory, and dropping sqlx's default `json` feature could pre-empt P1-02's raw-data
retention design (`final-build-spec.md` §8.3).

### Relationship to §14.2 — corrected after review

`alpha-spec.md` §14.2 states: *"Major version upgrades are separate work packages and cannot be
hidden inside feature changes."*

An earlier revision of this ADR argued that "the intent of §14.2 — no silent upgrades — is
satisfied" by disclosure. **That reasoning is withdrawn.** Adversarial review of PR #1 (finding
M4) correctly objected: disclosure is not the rule, separation is, and an ADR reinterpreting a
level-2 requirement whenever compliance is costly is exactly what `docs/02-adr/README.md`
forbids when it says an ADR "can never override either specification". Left standing, that
reasoning would license every later package to argue its way around any spec clause.

What actually authorizes this change is narrower and does not depend on reinterpretation:

**This is an owner deviation from §14.2, not a satisfaction of it.** §1.5 makes resolving
conflicts and absent decisions the Product/Architecture owner's role. The conflict — a security
advisory with no fixed upgrade available, reachable only through a dependency this package must
introduce to satisfy a frozen verification recipe — was put to the owner with the advisory, both
tested remedies and their measured results, and the alternatives including a separate P1-00b
package. The owner chose to keep it here.

So §14.2 is **deviated from, on the owner's authority, with the deviation recorded** — the same
mechanism ADR-001 D3 uses for §8.7 and ADR-006 uses for the §8.7/§9.2 deferrals. It is not the
ADR that overrides the spec; it is the owner, and the ADR is only the record.

The distinction matters for what P1-01…P1-11 inherit: a deviation needs an owner ruling on a
specific question, not a well-written rationale.

## Consequences

**Easier.** `cargo audit` passes for a real reason. Library and CLI versions agree, removing a
cache-format failure mode before any query depends on it.

**Harder.** sqlx 0.9 is semver-breaking relative to 0.8. Nothing in the repository consumed
sqlx before P1-00, so the migration cost here was zero — but any 0.8-era snippet or example
found in external documentation may not apply. P1-02 should read 0.9 docs, not 0.8.

**Risk accepted.** Verified after the change: `cargo audit` exit 0, `cargo deny check` exit 0,
and both `grid-persistence` tests pass against a freshly migrated database.

## §14.2 dependency checklist

`alpha-spec.md` §14.2 (line 1927) requires a maintenance assessment and a Windows compatibility
check for any dependency change. An earlier revision of this ADR recorded neither; the second
adversarial review was right that "the advisory is real" does not discharge the checklist. Both
are recorded here, from evidence rather than assertion.

**Maintenance assessment — sqlx 0.9.**
- Same maintainers and repository (`launchbadge/sqlx`) as the 0.8 line this project already
  depended on; not a fork, not a re-homing.
- 0.9 is the current major line, released ahead of this work package. GRID-Alpha is not moving
  to a bleeding edge — it is moving off a line whose transitive `rsa` dependency has an
  advisory with **no fixed version available**, which is the condition that forces a decision.
- Dependency footprint went **down**, not up: with `--no-default-features --features
  sqlite,rustls` the MySQL and Postgres drivers, and `rsa` with them, are absent from
  `Cargo.lock`. Verified — the lockfile resolves 172 crates and contains no `rsa` entry.
- Surface used by this project is one `query!` macro and one `query` call in a single crate,
  so the exposure to any 0.9 API change is minimal and fully covered by `check-sqlx`.
- The CLI is now pinned to `sqlx-cli 0.9.0` in `toolchains/dev-tools.lock`, closing the
  version-skew hazard this ADR was itself created by (an unpinned CLI resolving 0.9 against a
  0.8 library).

**Windows compatibility check — ESTABLISHED (run `32415391750`).** Required because Windows is
the production target (`final-build-spec.md` §3.2). `scripts/verify.ps1 -Scope Full` passed on
`windows-latest` in the same job where the **ADR-009 self-test** proved the script is capable of
failing — the full recipe chain against sqlx 0.9, including `cargo sqlx prepare --check
--workspace`, `cargo nextest`, `cargo deny` and `cargo audit`.

That pairing is the point. The history below is kept rather than deleted, because the reason this
item was withdrawn is more useful to a later reader than a clean assertion would be.

An earlier revision of this section cited `windows-latest` run `32329932430`,
`scripts/verify.ps1 -Scope Full`, PASSED in 32m12s, as empirical proof. **That citation is
withdrawn.** ADR-009 establishes that `verify.ps1` discarded the exit code of every command it
ran, so those greens show the recipe chain *executed* on Windows against sqlx 0.9 — they do not
show it *passed*. Citing a gate that could not fail as compatibility evidence is exactly the
"evidence over assertion" failure this project's §1.6 exists to prevent, and it was written into
this ADR by the same implementer who then found the defect. Recorded plainly rather than quietly
repaired.

What still holds, on its own evidence: `sqlx-sqlite` uses the bundled SQLite C library and
`rustls` rather than a system OpenSSL, so no Windows-specific native dependency is introduced —
that is a property of the dependency graph (`Cargo.lock`, verified: 172 crates, zero `rsa`), not
of a CI run.

Re-recorded from run `32415391750` on `6b7f3fd`, per owner ruling D11: withdraw and re-earn. It
took four rounds and eight commits to get back what a single unexamined green had claimed.

**Not assessed:** long-term maintenance trajectory beyond the current release, which no
snapshot can establish. `cargo audit` in the frozen `audit` recipe is the standing regression
gate for whatever the trajectory turns out to be.

## Compliance

- `cargo audit` in the frozen `audit` recipe — regression gate for this advisory.
- `cargo deny check` — license and advisory policy (`deny.toml`).
- `just bootstrap` installs `sqlx-cli`; ADR-002's environment contract keeps the toolchain
  and library aligned.

## Alternatives considered

**Suppress via `.cargo/audit.toml`.** Keeps 0.8 and ignores the advisory on the argument that
`rsa` is unreachable. Rejected: `alpha-spec.md` §12.7 and Appendix D prohibit weakening a
security control to complete a package, and it leaves the CLI/library skew unfixed.

**Feature narrowing on 0.8.** Tested and ineffective; see the table above.

**A separate P1-00b upgrade package.** Strictly literal §14.2 compliance. Rejected by the
owner: it costs an extra review cycle and blocks P1-00 and everything downstream, for a change
whose full justification and verification already fit in this ADR.

**Ship P1-00 with `cargo audit` red.** Rejected — a foundational package that normalizes a red
gate teaches every later package that red is acceptable.
