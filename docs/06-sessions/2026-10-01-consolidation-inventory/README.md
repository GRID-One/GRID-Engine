# Consolidation inventory — 2026-10-01

Working evidence behind work package P0-01
([`docs/01-work-packages/p0-01-engine-consolidation.md`](../../01-work-packages/p0-01-engine-consolidation.md)):
the engine-only pivot of GRID-Engine and the import of the cautious-nevermore Python engine as
a reference oracle ([ADR-011](../../02-adr/011-engine-only-pivot.md),
[ADR-012](../../02-adr/012-python-reference-oracle.md)).

**These reports are history, not authority.** They are not in the order of authority
(`docs/00-meta/authority-index.md`). Where a report disagrees with the consolidated
`engine-spec.md`, an accepted ADR, a contract or a model spec, the authority document wins.
The reports record what was found and recommended on 2026-10-01, before any of it was acted on.

## What the inventory was

On 2026-10-01 a Claude Code session orchestrated fresh-context AI inventory agents to survey
the material being consolidated. Each agent started with no prior conversation context and
worked **read-only** against two repositories:

- **GRID-Engine** @ `3823478`, branch `claude/grid-engine-consolidation-e7kmh9`, which was then
  the tip of PR #1 (P1-00 bootstrap), plus the unmerged review branches of PRs #2 and #3;
- **cautious-nevermore** (CN) @ `59bce1d`, the repository holding the working Python GRID
  engine.

Eight agents each wrote one report. A ninth, the completeness critic, read all eight in full
and wrote `critic.md`. Analysis, repro scripts and scratch clones lived in the session's
scratchpad. Nothing in either repository was modified by the inventory.

The nine reports below are copied **verbatim** — byte-identical to what the agents wrote,
checked by sha256 at copy time. This README is the only file in the directory written
afterwards. Do not edit the reports: a correction belongs in the authority document that
supersedes them, in [`docs/00-meta/known-issues.md`](../../00-meta/known-issues.md), or in
[`docs/00-meta/decision-register.md`](../../00-meta/decision-register.md).

## Reports

The short name is the abbreviation `critic.md` uses to cite each report.

| File | Short name | One line | Lines | sha256 |
|---|---|---|---|---|
| [`alpha-spec.md`](alpha-spec.md) | AS | Section-by-section classification of the 2186-line `alpha-spec.md` (ENGINE / VALIDATION / DATA / PLATFORM / PROCESS / APP-ONLY / MIXED): the binding requirements, the wording each needs for an engine-only repo, a proposed re-sequencing, and the inbound references that break on a rename | 1604 | `8080c15e2c18eeefabc8a6f10f1efe17a722dbf1822193c24022acda43b7a5ff` |
| [`final-build-spec.md`](final-build-spec.md) | FB | KEEP / KEEP-R / REPLACE / DROP disposition of every section of the 818-line `final-build-spec.md`, its internal defects, its overlap with alpha-spec, the parity regime against the Python oracle, and a proposed outline for the consolidated spec | 977 | `d0ff82d078d6b533f8e6a7f727f09275ea1f09da204996d36fb588b7458573fb` |
| [`bootstrap-infra.md`](bootstrap-infra.md) | BI | The P1-00 bootstrap infrastructure (all 89 tracked files) under the pivot: guard-by-guard analysis, a simulated minimal pivot in a scratch clone, the PR #2/#3 review records and their still-open findings, ADR validity, and a sequencing plan | 758 | `6890c499410f257b3dc992c4f68c7c7777c79ae7d6c8474c841f5d0fce43022b` |
| [`python-closure.md`](python-closure.md) | PC | The AST-derived import closure of the CN engine for `reference/python/`: 105 upstream files (103 byte-identical, one required and one optional patch) plus 5 support files, proven 446-passed in a clean venv with app and network imports blocked | 514 | `be5cc70311da8660bfd0d90fe93fd3aa0026cf0629b8baf3f189da875fdc728a` |
| [`cn-docs.md`](cn-docs.md) | CD | Engine-relevant extraction from CN's documentation, including the docs deleted by CN PR #92 (read from history): engine and validation design, the corrected Phase-2c verdict record, decisions, licensing, traps, and doc-vs-code discrepancies | 387 | `1291bdab979890188e49b46fb0ce7df9683603bf134528490ef591b5de0f83eb` |
| [`cn-issues.md`](cn-issues.md) | CI | CN's engine known-issues backlog, each status re-verified against the code at `59bce1d`, re-graded for severity, oracle contamination and Rust-port consequence. Source of the `KI-` IDs | 320 | `103bbec2274f2de4f115d08d7b6c25a0a93e4490501762886b836862bf5dcda5` |
| [`reconcile-code-first.md`](reconcile-code-first.md) | CF | Code-first reconciliation: what each Python component actually estimates and how it is validated, the component-to-spec map, gaps, extras the spec must add, conflicts, Rust crate homes, parity targets and a constants appendix | 881 | `082f8914f1376dbeeba0d92949cb22393d83dcf8dfd33ccc2ce5abba5d2e7248` |
| [`reconcile-spec-first.md`](reconcile-spec-first.md) | SF | Independent spec-first second opinion: requirement-by-requirement SAT / PARTIAL / ABSENT / CONTRA reconciliation, engine-semantic contradictions the specs do not enumerate (including the synthetic defender bug, its §4 C1), a unified architecture and a parity strategy | 983 | `5da81ec0ea40faff72073b11fca86d0217704b045d5ab8c144870ab588f22ccf` |
| [`critic.md`](critic.md) | — | Completeness critic over the other eight: gaps G-1..G-9, 21 inter-report contradictions adjudicated (X-1..X-21), 33 de-duplicated owner decisions with recommended defaults (A-1..A-12, B-1..B-6, C-1..C-15) plus 14 obvious-default items, and a checklist for the consolidation PR | 320 | `a6814364c09e098cf3a6fa15261737b43dc55c6a628a7db2896f38631ae18446` |

Two of the files share a name with a superseded specification. `alpha-spec.md` and
`final-build-spec.md` in this directory are inventory reports **about** those specifications,
not copies of them. The specifications themselves are at
[`docs/00-meta/specs/superseded/`](../../00-meta/specs/superseded/).

## Caveats — read before citing anything here

1. **Absolute scratch paths are dead.** Every `/tmp/claude-0/.../scratchpad/...` path, and every
   `scratch-*/` path relative to it (`scratch-cnissues/`, `scratch-python/`, `scratch-critic/`,
   `scratch-specfirst/` and the rest), refers to the inventory session's ephemeral scratch storage.
   That storage no longer exists. The paths are kept because the reports are verbatim; none of
   them resolves.
2. **The repro scripts moved.** The reports' repro scripts were carried into
   [`reference/python/tools/investigations/`](../../../reference/python/tools/investigations/).
   Look there, not under `scratch-*/`.
3. **Numbers measured on the legacy synthetic generator are invalid as parity targets.** CN's
   `backend/grid/synth.py:193` draws each play's defenders from the offense's own team
   ([`KI-NEW-Y0`](../../00-meta/known-issues.md); `critic.md` G-1, X-1). Unless a report says
   it used a corrected generator, its synthetic numbers come from that one. The exceptions are
   SF's and the critic's fixed-generator runs, and CI's planted-defense test, which has its own
   generator. Everything else is legacy-generator output: the team-strength, Layer-3,
   DEF-recovery and matchup-grade figures, the "[+1,−1] fixes G1" evidence, and every
   parity-number table (PC §8, CF Appendix A.2, FB §5 Class D, CD §3.6). Cite those numbers as
   legacy-generator history only, never as a target for the Rust port.
4. **`critic.md` adjudicates contradictions.** The reports disagree in 21 places. Where two
   reports conflict, follow `critic.md` §2.
5. **Owner decisions live in the decision register.** `critic.md` §3's IDs appear in
   [`docs/00-meta/decision-register.md`](../../00-meta/decision-register.md) prefixed `DR-`
   (`A-1` becomes `DR-A1`). The structural A-decisions are adopted by this consolidation subject
   to owner ratification. The B- and C-decisions are proposed defaults that wait on the owner;
   a report that states one as settled is stating a recommendation.
6. **Defects live in the known-issues register.** `cn-issues.md` IDs appear in
   [`docs/00-meta/known-issues.md`](../../00-meta/known-issues.md) prefixed `KI-` (`G1` becomes
   `KI-G1`, `NEW-A2` becomes `KI-NEW-A2`). The register also holds two defects the critic found:
   `KI-NEW-Y0` (the synthetic defender bug above) and `KI-NEW-D1` (`coaching_changes_2025.json`
   data quality, `critic.md` G-4).
7. **The reports describe the repository as it was before the consolidation.** "PR #1 is not
   merged", "the remote branch still points at `48ee320`", `alpha-spec.md` and
   `final-build-spec.md` at the repo root, and the `docs/00-meta/specs/alpha-spec.md` mirror are
   all 2026-10-01 pre-consolidation state. The specifications have since moved verbatim to
   `docs/00-meta/specs/superseded/` and the old mirror was removed, so the reports' spec line
   numbers (`AS:75`, `L390` and the like) resolve in the superseded copies. CN paths refer to
   CN @ `59bce1d`. Under `reference/python/` the same files carry that prefix, and two of them
   are patched (see `reference/python/MANIFEST.tsv`).

## Checks at import

- **Verbatim.** All nine reports matched the sha256 of the agents' originals after copying.
- **Secret patterns.** Every line of every report was run through the deny and warn tiers of
  `scripts/secret-patterns.txt`, which is the pattern loop of `scripts/check-secrets.sh`. Result:
  0 deny-tier and 0 warn-tier matches.
- **Spelling.** `typos` 1.49.0 (the `toolchains/dev-tools.lock` pin), run with the repository's
  `_typos.toml` as it stood at import, flags 113 tokens in eight of the nine reports. Only
  `final-build-spec.md` is clean. None of the 113 is a misspelling. Each falls into one of four
  groups (described here in words, so that this README stays clean under the same check):
  - the three-letter abbreviation for Value Over Replacement, the oracle's scoring module: 65 hits;
  - short oracle variable names (an "other situations" bucket, a first-origin season and week,
    a loop variable, a fitted value) quoted in the reports' proposed `typos` allowlists;
  - prose abbreviations: overtime, threshold, opportunities, POSIX basic regular expression, and
    the plural of the PNG file extension;
  - the hyphenated prefix meaning "wrongly", in four compounds.

  Run `typos` on this directory to list them with line numbers. They have to be handled in the
  repository's `typos` configuration. Editing the reports to silence them is not an option.
