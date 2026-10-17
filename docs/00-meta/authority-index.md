# Authority Index

Canonical map of what governs GRID Engine and where it lives. When documents conflict, the lower
priority number wins.

> **Replaced in P0-01.** The previous revision ranked `final-build-spec.md` and `alpha-spec.md` at
> levels 1 and 2. ADR-011 replaces both with `engine-spec.md` and archives them as superseded
> (DR-A1, DR-A2). The order below is adopted by P0-01. The owner ratified DR-A2 on 2026-10-08 in the
> decision interview and named it again in the P0-01 merge review on 2026-10-15 (PR #4 comment
> 6185953773), so the order is in force (`docs/02-adr/README.md`).

## Order of authority (engine-spec.md §1.5)

| Priority | Authority | Location |
|----------|-----------|----------|
| 1 | The specification, `engine-spec.md`, and its byte-identical mirror | `engine-spec.md` (repo root); `docs/00-meta/specs/engine-spec.md` |
| 2 | Accepted Architecture Decision Records | `docs/02-adr/` |
| 3 | Versioned contracts and schemas, model specifications, and provider manifests | `docs/03-contracts/`, `docs/05-model-specs/`, `docs/04-providers/` |
| 4 | The approved work-package file | `docs/01-work-packages/` |
| 5 | Tests and fixtures that implement the approved contracts, including committed reference-oracle fixtures and golden files | `crates/**/tests/`, `tests/`, `fixtures/` (from P1-01) |
| 6 | Existing source code, comments, and local conventions, including the `reference/python/` source | `crates/`, `scripts/`, `reference/python/` |

Two rules that are easy to get wrong:

- **Existing code is not authoritative merely because it already exists.**
- **Tests are not authoritative if they contradict a higher-level approved requirement.**

When you detect a conflict, or an absent decision that materially changes behavior, stop the work
package at a clean boundary and produce a decision request. Do not silently pick an interpretation.

**The reference oracle never overrides the specification** (engine-spec §1.5, §1.7):

- A disagreement between oracle behaviour and a higher authority is a decision request.
- An accepted divergence requires an ADR approved by the statistical owner and an entry in
  `reference/python/PARITY.md`.
- Oracle behaviour is never a reason to weaken a rule in the specification.
- An oracle golden that contradicts a model spec is regenerated through the correction ledger. It is
  never hand-edited, and never obeyed over the model spec.

`engine-spec.md` §1.5 requires every restatement of this order to match it: `CLAUDE.md`,
`docs/CLAUDE.md` and this file.

## Non-authoritative sets

Consulted for rationale and provenance only. None of them ranks in the order above.

| Set | Location | Notes |
|---|---|---|
| Superseded specifications | `docs/00-meta/specs/superseded/alpha-spec.md`, `docs/00-meta/specs/superseded/final-build-spec.md` | Verbatim. Old section citations resolve through engine-spec Appendix H |
| Archive | `docs/07-archive/` | Includes the verbatim cautious-nevermore copies (DR-A9; `docs/07-archive/README.md`) |
| Session logs and reviews | `docs/06-sessions/` | Records of what happened, never edited after the fact |
| Registers | `docs/00-meta/decision-register.md`, `docs/00-meta/known-issues.md`, `docs/00-meta/lessons-learned.md` | Track decisions and defects. A ratified decision changes requirements only when it is carried into `engine-spec.md`, an accepted ADR or a model spec |
| Dashboard and daily log | `docs/00-meta/dashboard.md`, `docs/00-meta/daily-log.md` | Status views |

## Path aliases

New documents use the actual numbered paths. A path or section written in a superseded
specification, an immutable ADR or an evidence record resolves through this table (engine-spec
§8.14). A citation of "alpha-spec §x" or "final-build-spec §y" resolves to its engine-spec section
through **engine-spec Appendix H**; immutable records are not rewritten.

| Name used in superseded or immutable documents | Actual path | Status |
|------------------------------------------------|-------------|--------|
| `alpha-spec.md`, `final-build-spec.md` (repo root) | `docs/00-meta/specs/superseded/alpha-spec.md`, `docs/00-meta/specs/superseded/final-build-spec.md` | superseded, non-authoritative |
| `docs/authority.md` | `docs/00-meta/authority-index.md` | this file |
| `docs/adr/` | `docs/02-adr/` | exists |
| `docs/work-packages/` | `docs/01-work-packages/` | exists |
| `docs/contracts/` | `docs/03-contracts/` | exists |
| `docs/providers/` | `docs/04-providers/` | exists |
| `docs/model-specs/` | `docs/05-model-specs/` | exists |
| `docs/sessions/` | `docs/06-sessions/` | exists |
| `docs/archive/` | `docs/07-archive/` | exists |
| `docs/templates/` | `docs/99-templates/` | exists |
| `docs/runbooks/`, `docs/model-cards/`, `docs/traceability/` | not yet created | deferred ([ADR-006](../02-adr/006-deferred-deliverables.md)); runbooks and model cards in P1-11; traceability is superseded in substance by `scripts/check-traceability.sh` |
| `docs/05-sessions/` | — | **invalid.** 05 is model specs; sessions are `docs/06-sessions/` |

## Canonical verification

```text
powershell -ExecutionPolicy Bypass -File scripts/verify.ps1 -Scope Full   # merge-authoritative (DR-A3)
just verify                                                               # Linux smoke
just bootstrap                                                            # once per fresh environment
```

- **Windows is merge-authoritative pending DR-A3.** `verify.ps1 -Scope Full` and the
  `windows-authoritative` CI job stay the merge gate. The old rationale ("the production target is
  Windows") no longer holds; Windows authority is kept only to avoid reducing coverage during the
  pivot. A follow-up ADR is due before P1-11 (engine-spec §8.19). Its direction was ratified on
  2026-10-08: Linux becomes merge- and release-authoritative, with Windows kept as a matrix job. Until
  that ADR is accepted, Windows stays the merge gate.
- **`just verify` is the Linux smoke check.** It does not replace the merge gate.
- **The frozen verify chain** is 8 recipes and 15 commands, identical in the `justfile`,
  `scripts/verify.sh` and `scripts/verify.ps1` (`scripts/check-verify-parity.sh`). Recipes are a
  frozen contract: make the repository satisfy them; a recipe changes only by ADR (ADR-001 D5).
- **The reference-oracle job is Linux-only and outside the frozen chain.** The `reference-oracle` job
  in `.github/workflows/alpha-ci.yml` checks `reference/python/` against `MANIFEST.tsv` and runs the
  oracle suite with threads pinned to 1, on the runner image's tool-cache CPython 3.11 (the job fails
  if that interpreter is absent) and with `OPENBLAS_CORETYPE=Haswell`. It is never wired into
  `verify.ps1`, `just verify` or `windows-authoritative`. The owner decided on 2026-10-09 that it
  becomes a required status check for every pull request (DR-D30). P0-01 has merged, and the owner now
  applies the setting in branch protection. The oracle's golden is frozen on that pinned platform (KI-NEW-Z78; DR-D31, ratified 2026-10-08; correction-ledger entry L0), `reference/python/tools/pytest_platform_pin.py` enforces it,
  and the job is expected green. Never loosen a tolerance to absorb a platform difference.

## Active work

- **Last merged work package:** P0-02 — Record the P0-01 merge review, the DR-A10 ruling and the
  ADR acceptances (`docs/01-work-packages/p0-02-record-p0-01-merge-rulings.md`), merged on 2026-10-15
  (`73a8e3b`, the merge of PR #5; `docs/06-sessions/2026-10-15-P0-02.md`). Before it, P0-01, the
  engine-only consolidation, merged on 2026-10-15 (`b819ae0`), with P1-00 (DR-A6).
- **In progress:** P1-01 — Domain types, engine API skeleton, oracle-fixture harness and crate
  restructure (`docs/01-work-packages/p1-01-engine-skeleton.md`), on branch
  `claude/pensive-fermat-fgaweh` from `73a8e3b`. It proposes ADR-014 (`grid-domain` takes `serde` as
  its only external dependency), which the owner accepts or rejects at its merge.
- **Next:** P1-02 (SQLite schema, migrations, durable jobs) and, in parallel with the P1-02 → P1-05
  chain, P1-06 (numerical primitives), once P1-01 has merged and its contracts are fixed.
- **Sequence:** engine-spec §9.5; index and readiness in `docs/01-work-packages/README.md`. A package
  is not Ready while a decision it depends on is unratified (`docs/00-meta/decision-register.md`).

## Human roles (engine-spec.md §1.4)

| Role | Authority |
|------|-----------|
| Product/Architecture owner | Requirements and architecture conflicts, ADR approval, scope; ratifies DR-A decisions; owns the oracle retirement review (DR-A12) |
| Statistical owner | Model equations, priors, evaluation design, calibration, promotion criteria; oracle correction-ledger entries (DR-B1), parity tolerances (DR-B3), the synthetic world (DR-B4), and every accepted engine/oracle divergence |
| Data/Licensing owner | Provider access, retention, attribution, benchmark import rights; the `reference/python/` licence (DR-A10) and fixture licensing (DR-A11) |
| Security/Release owner | Agent permissions, secrets, dependencies, signing, release artifacts, releases; CI-surface changes for the oracle job and the authoritative verification platform (DR-A3); with the Product/Architecture owner, whether the oracle job is a required check (DR-D30) |
| Merge reviewer | Final diff and evidence bundle |

All roles are currently held by the repository owner. The implementing agent is never the sole
reviewer, and may not merge, sign, publish, promote or deploy (engine-spec §1.3, Appendix D).
