# Architecture Decision Records

**Rank.** Accepted ADRs are **authority level 2** (`engine-spec.md` §1.5, DR-A2): above contracts, model
specs, provider manifests and work packages, and **below** `engine-spec.md`.

- **Pending ratification.** That order is adopted by P0-01 (ADR-011) subject to owner ratification at
  merge. Until then, the superseded order of `alpha-spec.md` §1.5 applies, which puts ADRs at level 3,
  below both superseded specs.
- **Limits.** An ADR can resolve an ambiguity or record a deviation ratified by the owner. It can never
  override the specification. A decision that replaces the specification itself (ADR-011) gets its force
  from the owner's direct ruling, and the ADR is the record of that ruling (the ADR-001 D3 precedent).

## Numbering

`NNN-kebab-case-title.md`, zero-padded, allocated in order and never reused. An ADR is immutable once
Accepted. To change a decision, write a new ADR, set `superseded-by` on the old one and `supersedes` on
the new one.

Start from `docs/99-templates/template-adr.md`.

## Status

The `status` values are those of the template: `Proposed`, `Accepted`, `Superseded` and `Rejected`.

- **Who accepts.** Only the deciding owner role moves an ADR from Proposed to Accepted or Rejected. An
  implementing agent writes ADRs as Proposed and never marks one Accepted.
- **ADR-011 and ADR-012** are accepted, or overridden, in the owner's merge review of the P0-01 pull
  request, together with the DR-A decisions they carry (`docs/00-meta/decision-register.md`, "How to
  ratify or override").

## Partial supersession

When a new ADR replaces only part of an older one:

- **The older ADR stays `Accepted`.** Only its front-matter `superseded-by` changes, to name the new
  ADR with the qualifier `(in part)`.
- **The new ADR** names the older one in `supersedes` with the same qualifier. A "Supersession" section
  in the new ADR states exactly which parts are replaced and which still stand.
- **While the new ADR is `Proposed`,** the annotation also carries `proposed`, and it binds nothing. The
  older ADR's text governs until the new ADR is accepted.
- **On acceptance,** the `proposed` qualifier is dropped.
- **On rejection,** the annotation is removed.

Editing `superseded-by` is the only change permitted to an Accepted ADR. The body is never edited.

## Index

| ADR | Title | Status | Date | Superseded in part by |
|-----|-------|--------|------|-----------------------|
| [001](001-repo-bootstrap-decisions.md) | Repository bootstrap decisions | Accepted | 2026-08-20 | 011 (proposed) |
| [002](002-sqlx-offline-cache.md) | SQLx offline cache and database provisioning | Accepted | 2026-08-20 | — |
| [003](003-sqlx-0-9-upgrade.md) | Upgrade sqlx 0.8 to 0.9 | Accepted | 2026-08-20 | — |
| [004](004-line-ending-policy.md) | Line-ending policy | Accepted | 2026-08-20 | — |
| [005](005-check-sqlx-workspace-flag.md) | Amend check-sqlx with --workspace | Accepted | 2026-08-20 | — |
| [006](006-deferred-deliverables.md) | Deferred 8.7/9.2 deliverables | Accepted | 2026-08-20 | 011 (proposed) |
| [007](007-verify-covers-guards-and-doctests.md) | verify covers guards and doctests | Accepted | 2026-08-20 | 011 (proposed) |
| [008](008-verify-scope-guard.md) | Validate the scope argument in verify.sh | Accepted | 2026-08-20 | — |
| [009](009-verify-ps1-exit-codes.md) | verify.ps1 must propagate native exit codes | Accepted | 2026-08-20 | 011 (proposed) |
| [010](010-guard-the-shape-not-the-instance.md) | Fix the shape, not the instance: where a fail-closed guard belongs | Accepted | 2026-08-29 | — |
| [011](011-engine-only-pivot.md) | Engine-only pivot | **Proposed** | 2026-10-07 | — |
| [012](012-python-reference-oracle.md) | Python reference oracle | **Proposed** | 2026-10-07 | — |

**What ADR-011 supersedes in part** (its "Supersession" section is authoritative):

- **ADR-001:** the twelve-crate list including `ffi`; the authority rows that name the `alpha-spec.md`
  copies; the D5 recipe count; the template's reference to the superseded Appendix B; "P1-01…P1-11".
- **ADR-006:** the `app/` deliverables in item 6 and the `toolchains/flutter.version` sentence.
- **ADR-007:** the chain listing that names `test-ffi`.
- **ADR-009:** the 16-command count and the "production target is Windows" rationale.

**Rationales orphaned but decisions unchanged:** ADR-003's Windows-compatibility rationale and ADR-002's
mention of `crates/ffi/src/generated/`.

**Deferrals carried to the re-scoped P1-11:** those of ADR-004, ADR-007 and ADR-008.
