---
adr-id: 016
status: Proposed
date: 2026-11-06
deciders: Statistical owner; Product/Architecture owner
supersedes:
superseded-by:
---

# ADR-016 — Typed failure replaces the oracle's silent-failure paths (DR-B6)

## Status

**Proposed** on 2026-11-06 under work package P1-02
(`docs/01-work-packages/p1-02-sqlite-backbone-and-durable-jobs.md`).

- **What it records.** The policy of DR-B6, which the Statistical and Product/Architecture owners
  ratified in part on 2026-10-08 in the decision interview
  (`docs/06-sessions/2026-10-09-P0-01-decision-interview.md`).
- **Why it is needed.**
  - engine-spec §1.7 requires one: "The ADR that adopts these typed failures MUST cite
    cautious-nevermore PR #53 audit item C3 (the `lstsq` fallback) and PR #56 (the re-initialization on
    corrupt state), the earlier decisions it reverses."
  - ADR-012 says "a new ADR is to record it".
  - No package was assigned to write it. P1-02 is the first package that implements one of the
    reversals: corrupt persisted state becomes `StateCorrupt` instead of a fresh start.
- **What it does not decide.** The open part of DR-B6, the numeric ill-conditioning threshold. It
  stays open until a model spec proposes it, before P1-06's solver failure semantics are final.
- **If the owners prefer** P1-06 to write this record, they reject it here and P1-06 carries the same
  text.

## Context

The oracle (`reference/python/`, `legacy-59bce1d + L0`) degrades silently on several paths
(engine-spec §1.7; `reference/python/PARITY.md` §(e) rows 1–7):

1. an `lstsq` fallback when a ridge system's condition number exceeds 1e10 (`layers.py:355-358`;
   KI-NEW-A6);
2. re-initialization on corrupt or unknown-width state (`statespace.py:116-118`; KI-A8,
   KI-NEW-W2);
3. skip-on-failure in `weekly_update.run`, and an unconditional "ok" health (KI-G6, KI-NEW-W5);
4. a V(s) refit on every run (KI-NEW-W4);
5. non-atomic saves (KI-G9, KI-V10);
6. accumulators saved before the solve (KI-NEW-Z14);
7. a missing defender rating read as 0.0 (KI-NEW-Z4).

Two of these were deliberate decisions in cautious-nevermore:
- **PR #53, audit item C3** ("Conditioning check + `lstsq` fallback in ridge solver"; critic G-3);
- **PR #56**, which described "corrupt → reinit" as intended design.

engine-spec §6.8 rule 4 and Appendix D forbid converting malformed data or a failed stage into a
healthy default.

## Decision

1. **Typed failure, everywhere.** For every oracle silent-failure path, the engine raises a typed
   failure (`grid_domain::EngineErrorCode`, or a typed job or terminal state), or it implements the
   behaviour the specification requires. It never reproduces the silent path.
2. **This reverses two recorded decisions:**
   - **cautious-nevermore PR #53 audit item C3**, the `lstsq` fallback. An ill-conditioned or
     non-convergent solve is a typed failure, and the candidate is not promotable (§6.4.3).
   - **cautious-nevermore PR #56**, re-initialization on corrupt state. Corrupt, width-mismatched,
     missing or hash-mismatched persisted state is `StateCorrupt`, never an empty or fresh state
     (§8.11).
3. **The oracle is left unchanged.** Each divergence is listed in `reference/python/PARITY.md` §(e).
4. **Parity fixtures exercise the healthy path only** (§7.12.6). A Rust test asserts the typed failure
   for each row of the §7.12.6 table, in the package that implements that row.
5. **The ill-conditioning threshold stays open** (DR-B6). A model spec proposes it before P1-06's solver
   failure semantics are final. Until then no package gates on a numeric threshold.

## Consequences

**Easier.**
- A failure is visible and has a code. A scheduler alerts on it.
- A crash or a corrupt file can no longer silently restart a model from scratch.

**Harder.**
- Some oracle outputs are not parity targets: the paths of §(e).
- An operator must act (rebuild from retained raw data, or restore a backup) where the oracle would
  have carried on.

**Inherited.**

| Package | Implements |
|---|---|
| P1-02 | `StateCorrupt` for missing, truncated or altered registered artifacts and for broken store invariants; the typed job states of §8.10, in which a failed job is never skipped; the atomic stage commit (row 6's primitive); atomic artifact writes (row 5) |
| P1-06 | The solver failure semantics (row 1) |
| P1-12 | The Kalman state schema and width checks (row 2's model-specific part), the frozen V(s) (row 4) and the uncovered-defender rule (row 7) |
| P1-03 | The fetch and validation paths of row 3 |

## Compliance

- **`PARITY.md` §(e)** lists the seven rows, with a Rust-status note naming the tests that assert each
  row as packages land. P1-02 adds the notes for rows 2 (generic `StateCorrupt` only), 3, 5 and 6.
- **The §7.12.6 table** has one Rust test per row (§12.6), named in the implementing work package.
- **Fresh-context review** (`reference-parity-reviewer`, §12.9 items 11–15) checks that no ported
  component reproduces a §(e) path.

## Alternatives considered

1. **A faithful port of the failure paths** (DR-B6 option 1). The owners rejected it on 2026-10-08.
   It contradicts §6.8 rule 4 and Appendix D, and it would make a crash indistinguishable from a
   fresh start.
2. **Leave the record to P1-06.** P1-06 owns the threshold, but P1-02 implements the first
   reversal, PR #56's. Recording the policy before the first implementation keeps the
   citation that §1.7 requires next to the code that first needs it. The owners may still choose
   this alternative by rejecting this ADR.
