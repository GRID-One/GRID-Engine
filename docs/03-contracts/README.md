# Contracts (`docs/03-contracts/`)

Versioned data and output contracts of the GRID engine. Each one fixes the field names, types,
units, null semantics, invariants and compatibility rules that independently written components
rely on.

## Authority

Contracts are **authority level 3** in the order of `engine-spec.md` §1.5, together with
`docs/05-model-specs/` and `docs/04-providers/`. This follows owner decision DR-A2 (adopted by
the P0-01 consolidation, subject to owner ratification).

- **Above them:** the engine spec and accepted ADRs.
- **Below them:** the approved work package, then tests and fixtures (including committed
  oracle fixtures and goldens), then code (including `reference/python/`).

Two consequences:

- **The Python oracle never overrides a contract.** Where `reference/python/` and a contract
  disagree, that is a decision request. An accepted divergence is a statistical-owner ADR plus an
  entry in `reference/python/PARITY.md`.
- **A contract describes, and does not obey, the code it was derived from.** The v0 (legacy)
  sections describe the oracle exactly, so its defects are recorded rather than adopted. The
  current version is what Rust implements.

Statements tagged *(proposed — DR-xx)* depend on an owner decision that is still open in
[`docs/00-meta/decision-register.md`](../00-meta/decision-register.md). They are defaults, not
settled requirements. Defects are cross-referenced to
[`docs/00-meta/known-issues.md`](../00-meta/known-issues.md) (`KI-…`).

## Index

| Contract | Id | Version | Status | Implemented by |
|---|---|---|---|---|
| [plays-contract.md](plays-contract.md): the engine input frames `plays`, `players`, `market`, `college`; the as-of and publication-lag metadata; the known v0 defects | `grid.plays` | v0 frozen (oracle, CN@`59bce1d`); v1 Draft (Rust) | Draft | P1-01 (`grid-domain` types), P1-03 (`grid-ingestion` nflverse adapter), P1-05 (as-of views), P1-12 |
| [engine-output-contract.md](engine-output-contract.md): projection records, stat vectors, distributions, scoring transform, explanations, change log, reports, exports | `grid.output` | 1 Draft | Draft; no producer | P1-01, P1-08, P1-10, P2-06, P2-07 |
| [parity-fixture-contract.md](parity-fixture-contract.md): how Rust ports prove parity with `reference/python/`. Covers fixture format and layout, manifest, stage-isolated injection, tolerance classes, legacy vs corrected oracle, regeneration, CI | `grid.parity-fixture` | 1 Draft | Draft; no fixture yet | P1-01 (harness), P1-06, P1-12 and every later port |

Related documents outside this directory:

- **Provider contracts** (raw source to canonical field): [`docs/04-providers/`](../04-providers/)
  - [nflverse](../04-providers/nflverse/README.md);
  - [nflverse access and licence](../04-providers/nflverse/access-and-license.md);
  - [CFBD](../04-providers/cfbd/README.md).
- **Model specs** (equations behind each stage): [`docs/05-model-specs/`](../05-model-specs/).

## Versioning convention

- **Identifier.** Every contract has a stable id (`grid.<name>`) and a version. Data carries the
  version it was produced under: frames, records, export sidecars and fixture manifests.
- **Version form.**
  - `grid.plays` uses whole numbers. **v0** is the frozen legacy description of the oracle; it
    changes only to correct an erratum. **v1** is the Rust target.
  - `grid.output` and `grid.parity-fixture` use `MAJOR.MINOR`.
- **MAJOR** (breaking). Removing or renaming a field; changing a unit, vocabulary, definition,
  default or invariant so that previously valid data is rejected or reinterpreted.
- **MINOR** (compatible). Adding an optional field, or an enum variant on an enum documented as
  open, whose absence has a defined meaning.
- **Readers** reject an unknown MAJOR with a typed error. They never "make the parser flexible"
  to absorb unknown semantics (superseded alpha-spec §4.6).
- **Derived artifacts.** Snapshots, accumulators, model states and fixtures record the contract
  version they were built from. After a MAJOR change they are rebuilt from retained raw data or
  regenerated from the oracle. They are never hand-migrated.
- **Process.** A contract change needs:
  - an ADR or an approved work-package decision;
  - the version line in the document updated;
  - fixtures regenerated under the reviewed-semantic-explanation rule
    ([parity-fixture-contract.md](parity-fixture-contract.md) §9);
  - synchronized tests in the same PR.

  Refactoring internal Rust types without changing a contract needs none of this.
- **Status values.** `Draft`, `Approved` or `Superseded`, recorded in each file's front matter.
  A contract becomes `Approved` when the work package that first implements it is approved by the
  owners named in its front matter.
