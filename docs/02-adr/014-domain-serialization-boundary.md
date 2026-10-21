---
adr-id: 014
status: Accepted
date: 2026-10-17
deciders: Product/Architecture owner
supersedes:
superseded-by:
---

# ADR-014 — `grid-domain` takes `serde` as its only external dependency

## Status

**Proposed** 2026-10-17 under work package P1-01 (`docs/01-work-packages/p1-01-engine-skeleton.md`).
**Accepted** 2026-10-19 by the Product/Architecture owner. The owner merged the P1-01 pull request
(PR #6, merge commit `8755552`) on 2026-10-19 without a review comment, then accepted this ADR the same
day in answer 1 of `docs/06-sessions/2026-10-19-P0-03.md` ("Accept (Recommended)"). This status was
transcribed at the owner's direction by work package P0-03; an implementing agent never marks an ADR
Accepted on its own judgment (`docs/02-adr/README.md`, "Status").

- **What it decides.** The question engine-spec §8.1 rule 5 assigns to P1-01: "`domain` currently
  declares 'no external deps'. If public DTOs need serialization derives, P1-01 decides by ADR whether
  `domain` takes the dependency or serialization lives in another crate."
- **Rank.** Once accepted it is authority level 2 (engine-spec §1.5), above the three contracts whose
  sentences it changes and below `engine-spec.md`, which delegated the decision.
- **While Proposed.** P1-01 implements it, as P0-01 implemented ADR-011 and ADR-012 while they were
  Proposed; the owner's ruling at merge either accepts it or sends the package back (P1-01, "Stop/decision
  conditions").

## Context

- `docs/CLAUDE.md` and engine-spec §8.1 give `domain` the boundary "IDs, types, errors, DTOs. No
  external deps." The three contracts repeat it: engine-output-contract §10 ("The domain crate has no
  external dependencies"), plays-contract §11 ("timestamps are a domain newtype and serialization lives
  outside the domain crate") and parity-fixture-contract §11 ("`grid-domain` may not depend on
  `serde_json`").
- engine-spec §5.5 rule 2 requires "one versioned schema module": the public output records are
  defined in `domain`, and file outputs have a machine-readable schema generated from those
  definitions. Rule 1 forbids "a hand-maintained parallel domain model", and engine-output-contract §1
  rule 1 repeats it.
- §5.5 rule 5 and engine-output-contract §11 require serialize/deserialize round trips of
  representative records in Rust tests, and §12.6 requires precision-preserving round trips through
  JSON and the parity-fixture formats.
- The orchestration crate writes JSON records and sidecars with `serde_json`
  (engine-output-contract §10), and the fixture reader parses manifests with `serde_json`
  (parity-fixture-contract §2). Both already assume `serde`-style serialization of domain types.
- `serde` (with `derive`) is already in `[workspace.dependencies]` and already resolved in `Cargo.lock`
  (`serde`, `serde_core`, `serde_derive`), because `sqlx` depends on it. Adding it to `grid-domain`'s
  manifest adds **no package** to the lockfile and nothing to the audited dependency graph.
- The dependency policy (engine-spec §14.2) requires justification and owner approval for a new
  production dependency, and prefers already-declared workspace dependencies.

## Decision

1. **`grid-domain` depends on `serde` with the `derive` feature, unconditionally, from
   `[workspace.dependencies]`, and on nothing else outside `std`.** Its boundary sentence becomes:
   "IDs, types, errors, DTOs. One external dependency: `serde` derives (ADR-014)."
2. **Public contract types in `grid-domain` derive `Serialize` and `Deserialize`** where the contract
   requires a serialized form: the output-contract records and enums, the plays-contract v1 frames and
   their metadata, the identifiers and versions they carry, `UtcInstant`, `AsOf` and its parts, the
   typed error codes and the status enumerations. Field names in the serialized form are the contract's
   field names; a rename is a contract change (engine-output-contract §9).
3. **Format crates stay outside `grid-domain`.** `serde_json` lives in `grid-pipeline` (records,
   sidecars) and `grid-synth` (fixture manifests). A CSV writer, Parquet or any hashing crate follows
   the §14.2 approval path in the package that first needs it. `grid-domain` never gains a second
   external dependency without a superseding ADR.
4. **Errors are hand-written.** `grid-domain` implements `Display` and `std::error::Error` by hand
   rather than taking `thiserror`, so that the dependency set stays at one and the error texts are
   visible in one place.
5. **No feature flag.** The `serde` derives are not behind a cargo feature. A feature matrix would have
   to be added to the verify chain for the dependency-free build to mean anything (engine-spec §8.19
   lists feature combinations as a partial gap), and no consumer of `grid-domain` wants a build without
   serialization.

## Consequences

**Easier.**
- One definition of every public record, serialized and deserialized by the same derives everywhere:
  the library API, the JSON sidecars, the parity harness and the P1-10 exports.
- The round-trip tests the contracts require can be written in the orchestration crate against the real
  types, with no mirror.
- No lockfile growth and no new licence or advisory surface (`serde` is `MIT OR Apache-2.0`, already
  gated by `cargo deny check` and `cargo audit`).

**Harder.**
- `grid-domain` is no longer dependency-free in the literal sense. The structural boundary test in
  `crates/pipeline/tests/` (P1-01) fails if any second external dependency appears, so the boundary is
  enforced rather than described.
- Serialized field names become part of the public contract, and the derives make a rename look like a
  one-line refactor. The output contract's versioning rules apply to every such rename
  (engine-output-contract §9), and the round-trip tests pin the representative records.
- A proc-macro dependency (`serde_derive`) is in the build of the lowest crate. It was already built
  for `sqlx-macros`, so build time does not change measurably; it is still one more thing the lowest
  crate trusts.

**Inherited.** Every crate that depends on `grid-domain` (all of them) inherits `serde`. That was
already true transitively through `sqlx` for `grid-persistence` and will be true for `grid-pipeline`
and `grid-synth` directly.

## Compliance

| Rule | Enforced or detected by |
|---|---|
| Decision 1 (one external dependency) | `crates/pipeline/tests/crate_boundaries.rs` (P1-01): `cargo metadata` shows `grid-domain`'s normal dependencies are exactly `{serde}`; the test fails otherwise. In review: `crates/domain/Cargo.toml` `[dependencies]` has one line |
| Decision 2 (derives on contract types) | The round-trip tests in `crates/pipeline/tests/` fail to compile if a contract type lacks the derives |
| Decision 3 (format crates outside `domain`) | The same structural test (no `serde_json` in `grid-domain`); `cargo deny check` and `cargo audit` on every new format crate |
| Decision 4 (no `thiserror` in `domain`) | The same structural test |
| Decision 5 (no feature flag) | `crates/domain/Cargo.toml` has no `[features]` table; `cargo clippy --all-features` therefore equals the default build for this crate |
| Contract sentences updated together | engine-output-contract §10, plays-contract §11, parity-fixture-contract §11 and `docs/CLAUDE.md` rule 4 are changed in P1-01 to cite this ADR |

## Alternatives considered

- **Serialization in `grid-pipeline` through mirror types.** Keeps `grid-domain` at zero dependencies.
  Rejected: it is exactly the "hand-maintained parallel domain model" engine-spec §5.5 rule 1 and
  engine-output-contract §1 rule 1 forbid, it doubles every record and every rename, and the mirror
  drifts from the type it mirrors with nothing but review to notice.
- **Serialization in `grid-pipeline` through `serde`'s remote derive (`#[serde(remote = …)]`).**
  Avoids a second struct but still requires a parallel definition of every field in the remote crate,
  needs every field public, and cannot express enum variant data without repeating it. The same drift
  risk as mirror types, with less readable code. Rejected.
- **Hand-written `Serialize`/`Deserialize` impls in `grid-pipeline` for every domain type.** No derive
  dependency in `domain`, but hundreds of lines of boilerplate per record group that must be kept in
  step with the type by hand, and the output contract's round-trip tests would then be testing the
  boilerplate rather than the type. Rejected.
- **`serde` behind an optional cargo feature of `grid-domain`.** Honours the literal "no external deps"
  in the default build. Rejected (Decision 5): nobody consumes the dependency-free build, and the
  feature would either go untested or force a feature matrix into the frozen verify chain, which is a
  D5 amendment nobody has asked for.
- **Also take `thiserror` in `grid-domain`.** Convenient, already in the lockfile. Rejected (Decision 4):
  it buys derive macros for `Display` on a few dozen variants at the price of a second exception to the
  boundary, and the error texts are contract-visible, so writing them by hand in one place is a
  feature, not a cost.
- **Also take `serde_json` in `grid-domain` so the fixture reader can live there.** Rejected
  (Decision 3): the reader is test-support code (parity-fixture-contract §11), the format crate belongs
  with the code that chooses the format, and `domain` would then carry a JSON parser that no engine
  type needs.
