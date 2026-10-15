# Superseded specifications

This folder holds the two specifications that governed the repository before the engine-only
pivot. Both are **superseded** and **non-authoritative**. The governing specification is
[`engine-spec.md`](../../../../engine-spec.md) at the repository root, with a byte-identical mirror
at [`docs/00-meta/specs/engine-spec.md`](../engine-spec.md).

## Status

| File | Status | Superseded by | sha256 |
|---|---|---|---|
| [`alpha-spec.md`](alpha-spec.md) | Superseded 2026-10-01; archived verbatim; non-authoritative | `engine-spec.md`, under ADR-011 | `b51535ffe5586972dac662c6d0d180aa3dcb9929d422d3d195c3db51fca74cd7` |
| [`final-build-spec.md`](final-build-spec.md) | Superseded 2026-10-01; archived verbatim; non-authoritative | `engine-spec.md`, under ADR-011 | `44b33f8615f03234db1c227531ebf50624922c566db7b703895daa883d9eb507` |

- **Superseded.** `engine-spec.md` (Draft v1.0, 2026-10-01) supersedes both documents for all
  purposes. The decision is ADR-011, "Engine-only pivot"
  ([`docs/02-adr/011-engine-only-pivot.md`](../../../02-adr/011-engine-only-pivot.md)). ADR-011 is
  proposed and takes effect on owner ratification at merge of P0-01.
- **Archived verbatim.** Each file is byte-for-byte the document as it stood at the repository root.
  Neither changed after it was added on 2026-08-19 (commit `48ee320`). P0-01 moved them here without
  edits. The sha256 values above identify the archived bytes; recompute them with
  `sha256sum docs/00-meta/specs/superseded/*.md`.
- **Non-authoritative.** They are non-authoritative history, consulted for rationale only
  (engine-spec §1.5). They impose no requirement, and a conflict between them and `engine-spec.md`
  is not a conflict: the engine spec governs. Do not edit them. A correction belongs in
  `engine-spec.md`.

## Why they are kept

Immutable and historical records cite these documents by section: accepted ADRs, evidence manifests
under `.ai/evidence/`, session records under `docs/06-sessions/`, and comments in guard scripts. For
example, `scripts/check-migrations.sh` cites "alpha-spec.md Appendix D #4" and
"final-build-spec.md 8.2". Those records are not rewritten, so the text they cite must stay available
at a stable path. The files are kept as single copies, with no mirror, because they are not
authoritative.

## Resolving a citation

Resolve any citation of "alpha-spec §x" or "final-build-spec §y" through **engine-spec Appendix H**,
"Crosswalk from the Superseded Specifications":

- **H.1** gives the procedure. It covers citation forms without the section sign
  ("alpha-spec.md 12.8") and item numbers ("Appendix D #4"), and it lists the renumbered blocks.
- **H.2** maps every heading of `alpha-spec.md` to its engine-spec section and disposition.
- **H.3** does the same for `final-build-spec.md`.
- **H.4** lists the individual items that were dropped or materially converted, with the reason.

The engine-spec section named there governs. Read the original here only to understand what a
citing record meant when it was written.

## What each original described

### `alpha-spec.md`

"NFL Weekly Fantasy Projection App — Two-Phase Alpha Specification", dated August 12, 2026 (2,186
lines). It specified a **native Windows desktop application** that produced week-by-week NFL fantasy
projections for the core offensive positions. The application generated stat distributions first and
translated them into Standard, Half-PPR, PPR and user-defined scoring. Its architecture was a native
Flutter Windows UI over a Rust core, joined by `flutter_rust_bridge` v2, with SQLite through SQLx.
nflverse was the primary data foundation, supplemented by CollegeFootballData (CFBD) for rookies and
low-evidence players. The work ran in two phases. Alpha Phase 1, "Projection Core and Historical
Proof", ended in a minimal native projection UI. Alpha Phase 2, "Live Weekly Intelligence and
Competitive Proof", added daily live operation and the evidence for a market-leading accuracy claim.
The document also set out a market-benchmark validation protocol and the governance for AI-assisted
implementation: work packages, review roles, verification and evidence. It named
`final-build-spec.md` as its production-architecture authority.

### `final-build-spec.md`

"Native Windows Sports Analytics Application — Final Build Specification", undated (818 lines). It
was the production architecture for a **native Windows desktop analytical application**: a native
Flutter desktop UI with no WebView, embedded browser or Electron layer, and a Rust computation and
data engine. It covered the system architecture and deployment as a single installable Windows
application (MSIX/installer packaging), and the Flutter frontend and its state synchronization over
`flutter_rust_bridge`. It also covered visualization, the Tokio/Rayon concurrency model and SQLite
persistence through SQLx. Further sections covered once-daily ingestion and failure handling, and a
statistical engine with ridge regression, RAPM, Kalman filtering, RTS smoothing, empirical-Bayes
shrinkage, affine mapping and gradient boosting. The rest specified incremental learning, model
promotion and versioning, a durable job queue, crash recovery, observability, security, testing,
performance, dependencies, the application lifecycle, an implementation order and V1 non-goals. Its
data domains were written for basketball (lineups, stints, possessions). The engine spec replaces
them for the NFL (engine-spec §8.5).

Neither document describes the current repository. This repository builds only the GRID engine: a
Rust library and a headless CLI over SQLite, with no user interface, installer or FFI layer
(engine-spec header; ADR-011).
