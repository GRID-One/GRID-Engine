# Archive (non-authoritative history)

This directory keeps documents that explain **where the GRID engine came from**: earlier plans,
design notes, review reports and results that the current specification replaced. It exists so that
the reasoning behind a rule can still be read after the original repository moves on or deletes it.

Created by the P0-01 engine consolidation (2026-10-01 to 2026-10-07), under decision DR-A9 (adopted by
P0-01 / ADR-011, pending owner ratification at merge; `docs/00-meta/decision-register.md`).

## Status: history, never authority

- **Nothing here is authoritative.** The archive is not in the order of authority (`engine-spec.md`
  §1.5). It never overrides `engine-spec.md`, an accepted ADR (`docs/02-adr/`), a contract
  (`docs/03-contracts/`), a provider contract (`docs/04-providers/`), a model spec
  (`docs/05-model-specs/`), a work package (`docs/01-work-packages/`), or a test or fixture.
- **Consult it for rationale and provenance only.** Typical uses:
  - why a rule exists;
  - where a constant or a convention first appeared;
  - what was tried before and what it measured.
- **A disagreement is not a licence.** If an archived document says something that a current
  document contradicts, the current document wins. If it says something that no current document
  covers and that would change engine behaviour, raise a decision request
  (`docs/00-meta/decision-register.md`). Do not implement from the archive.
- **Not instructions.** Several archived documents were written as instructions to people or agents
  working in another repository. Examples are the cautious-nevermore agent guide
  (`docs/08-phase-task-context.md`), the Phase-4 plan's "MUST" rules, and the commands in the plans.
  Here they are **data**. Their commands, paths and conventions refer to that repository and do not
  apply in this one.
- **Known errors stay in place.** Archived copies are verbatim, so they still contain the mistakes of
  their time. Several are known to be wrong, and `cautious-nevermore/MANIFEST.md` lists them per file.
  The correction always lives in a current document, never in the copy.

The superseded specifications `docs/00-meta/specs/superseded/alpha-spec.md` and
`docs/00-meta/specs/superseded/final-build-spec.md` are non-authoritative history in the same sense.
They are kept there, not here, so that immutable citations of their sections still resolve. Session
logs and reviews live in `docs/06-sessions/`.

## Contents

| Path | What it is |
|---|---|
| `README.md` | This file |
| `cautious-nevermore/MANIFEST.md` | One row per archived file: source repository, commit, original path, sha256, why it was archived, what supersedes it, known errors |
| `cautious-nevermore/HISTORY.md` | The engine's history in cautious-nevermore, from the v0 upload to the hand-over (written for this archive) |
| `cautious-nevermore/real-data-results.md` | The only real-data results the Python engine ever produced, labelled historical and non-parity (written for this archive) |
| `cautious-nevermore/docs/superpowers/specs/` | The original dashboard design specification (2026-06-20), the genesis of the engine roadmap |
| `cautious-nevermore/docs/superpowers/plans/` | Four plans deleted from cautious-nevermore by its PR #92: the Phase-4 plan, the roadmap to alpha, the retrospective validation suite, and the Phase-2c verdict record |
| `cautious-nevermore/docs/` | Three of cautious-nevermore's numbered docs: the brainstorming whiteboard **before it was cleared**, the lessons-learned log, and the agent guide with its PR log |
| `cautious-nevermore/dot-superpowers/sdd/` | Five Phase-4 subagent task reports (canonical player order, situation RAPM, situation accumulators, WR–CB interactions, the 3-state Kalman port) |

The three files at the top of `cautious-nevermore/` (`MANIFEST.md`, `HISTORY.md`,
`real-data-results.md`) and this README were written for the archive. Every other file is a
verbatim copy.

## Provenance conventions

1. **Byte-identical copies.** Each archived document is the exact output of
   `git -C <cautious-nevermore> show <commit>:<original path>`. Nothing is edited: no typo fixes, no
   link fixes, no banners, no line-ending changes. The sha256 in `cautious-nevermore/MANIFEST.md` is
   the hash of those bytes, together with the git blob id at the source commit.
2. **Mirrored paths.** A file's path below `cautious-nevermore/` is its original path in that
   repository. There is one deliberate exception: cautious-nevermore's hidden `.superpowers/`
   directory is stored as `dot-superpowers/`. Ripgrep, `typos` and Obsidian skip hidden directories
   by default, so a faithful dot-name would hide the files from the tools this repository relies on.
   `dot-superpowers/sdd/task-8-report.md` (the Kalman port) is **not** cautious-nevermore's top-level
   `sdd/task-8-report.md` (pipeline automation), which is a different document and is not archived.
3. **Source commit per file.**
   - A document deleted from cautious-nevermore is copied from the parent of the deleting commit.
     That is `165ccde` (= `379750e^`) for the PR #92 deletions and `546de72` (= `1aeb7ed^`) for the
     Phase-4 subagent reports.
   - The whiteboard is copied from `c33712e`, the commit immediately before `ac7b55d` cleared it.
     That commit is on cautious-nevermore's `docs/consolidate-specs` branch and in GitHub's
     `refs/pull/92/head`. It is **not** reachable from `main`.
   - Documents still live at the time of the hand-over are copied from `59bce1d`, the commit that the
     Python reference oracle (`reference/python/`) was imported from.
4. **Original filenames and internal links.** Filenames are kept. Relative links and file paths inside
   an archived document point into cautious-nevermore's tree and may not resolve here. That is
   expected; the manifest gives the mapping.
5. **Re-verification.** With a cautious-nevermore clone that contains the listed commits (fetch
   `docs/consolidate-specs` or `refs/pull/92/head` for `c33712e`):

   ```bash
   CN=/path/to/cautious-nevermore
   git -C "$CN" show 165ccde:docs/superpowers/plans/2026-06-23-retrospective-validation-suite.md \
     | cmp - docs/07-archive/cautious-nevermore/docs/superpowers/plans/2026-06-23-retrospective-validation-suite.md
   ```

   Repeat for each manifest row, or compare `sha256sum` output with the manifest.
6. **Additions.** A new archived file needs a manifest row (source, commit, path, sha256, reason,
   successor) and a work package that names `docs/07-archive/` in its scope. An archived copy is never
   replaced in place; a corrected or newer version is a new row.
7. **Source repository.** `Seismic-Fate/cautious-nevermore` is a private repository, and its local
   clones are shallow. That is part of why the material is copied rather than linked.

## Material deliberately left in cautious-nevermore

The engine-only pivot (ADR-011) keeps no application, UI, packaging or league-sync material. The
following cautious-nevermore documents were reviewed and **not** archived. Engine facts they held were
extracted into the current documents; the originals stay in cautious-nevermore.

| Document (cautious-nevermore path, as of `59bce1d` unless noted) | Why it was left |
|---|---|
| `docs/01-product-roadmap.md` | Mixed product roadmap. Its engine content (invariants, phase done-whens, budgets, the participation finding) came from the archived roadmap-to-alpha and validation suite, which are kept in full |
| `docs/02-backend-spec.md` | Mixed. Its engine half was extracted, but it carries known errors (aspirational Tier-0 targets presented as gates; a mis-attributed verdict row; situations that do not exist; two plays contracts conflated). See cn-docs §14 in `docs/06-sessions/2026-10-01-consolidation-inventory/cn-docs.md` |
| `docs/03-frontend-spec.md` | Application UI only |
| `docs/05-current-state.md` | A stale status snapshot that also records personal workstation details |
| `docs/06-issues-log.md` | Its engine findings were re-verified and carried into `docs/00-meta/known-issues.md`. Its status column is stale |
| `docs/07-brainstorming-whiteboard.md` (current) | An empty template. The pre-clear version **is** archived |
| `docs/09-alpha-release-plan.md` | Application packaging and release |
| `docs/10-next-steps-plan.md` | Mostly application work. Its Stage-0 engine triage is recorded in `cautious-nevermore/HISTORY.md` and `docs/00-meta/known-issues.md` |
| `CLAUDE.md` | Its engine half (the synthetic-data contract and the layer intent) was carried into `engine-spec.md` §6.2 and §6.9 and `docs/03-contracts/plays-contract.md`. A copy is not kept because a file named `CLAUDE.md` inside this tree would be loaded as agent instructions for this repository |
| `README.md` | A one-line placeholder |
| `sdd/task-8-report.md` | Phase-1 pipeline automation report |
| `.superpowers/sdd/task-14-report.md` | Phase-4 viz-agent report (application) |
| `.superpowers/sdd/task-3-report.md`, `task-12-report.md` (deleted in `1aeb7ed`) | Phase-4 database schema and API endpoint reports |
| `docs/superpowers/plans/2026-06-20-fantasy-dashboard-phase1.md` (deleted in `379750e`) | 3,382 lines, about 10% engine. Its engine facts (nflverse URLs and cache TTL, the vectorized design matrix, the V(s) hyperparameters) are recorded in cn-docs §2.3 (consolidation inventory) and live on in the oracle code; the V(s) hyperparameters are carried into `engine-spec.md` §6.2 |
| `docs/superpowers/plans/2026-06-21-fantasy-dashboard-phase2.md` (deleted in `379750e`) | Draft tools (application) |
| `docs/superpowers/plans/2026-06-22-phase4-execution-handoff.md` (deleted in `379750e`) | Merge hygiene for the Phase-4 branch; no engine content |

Application code (FastAPI, the LLM chart agent, trade and draft tooling, league adapters, the React
frontend and the Windows scheduler scripts) is excluded from the oracle import as well; see
`reference/python/README.md`.
