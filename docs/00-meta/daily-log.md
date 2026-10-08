# Daily log

Terse running record. Per-session detail belongs in a session log
(`docs/99-templates/template-session-log.md`).

## 2026-08-20 — P1-00

Branch `wp/P1-00-repo-bootstrap`.

Authority load found the repository could not support its own work package: no `.rs`, `.dart`
or `.sql` file had ever existed, `crates/` was absent so every cargo command failed at
manifest-parse time, four of five templates were 0 bytes, and the authority index and
`docs/CLAUDE.md` both inverted alpha-spec 1.5.

Owner rulings: `wp/` branch naming; remediate the skeleton inside P1-00; keep the numbered
vault; `MIT OR Apache-2.0`; **verify recipes are frozen — rework the spec, never the recipe**;
environment defects fixed in the environment, not the repo.

Four things found during implementation that the plan had not anticipated:

- `cargo nextest run --workspace` exits 4 on an empty workspace, so the SQLx pull-forward is
  load-bearing rather than tidy — its two tests are what let `test-rust` pass.
- The environment presets a stale `DATABASE_URL` from a predecessor project name, and a
  committed `.env` cannot override it. → ADR-002 contract + `check-env-contract.sh`.
- `cargo audit` failed on RUSTSEC-2023-0071 via `sqlx 0.8 -> sqlx-mysql -> rsa`, no fix
  available. Feature narrowing does not help; sqlx 0.9 removes it. → ADR-003.
- Every committed file is CRLF, leaving `scripts/verify.sh` unparsable by bash — the 8.11
  canonical Linux command could not run at all. → ADR-004.

## 2026-10-01 to 2026-10-08 — P0-01 engine-only consolidation

Branch `claude/grid-engine-consolidation-e7kmh9` (assigned by the harness; carries PR #1's P1-00
commits). Owner request of 2026-10-01: consolidate GRID-One/GRID-Engine and
Seismic-Fate/cautious-nevermore under GRID-Engine and make the repository engine-only. Owner choice:
Rust target, Python reference oracle.

Done (all in review, nothing merged):

- Consolidation inventory: eight reports and a completeness critic, imported verbatim into
  `docs/06-sessions/2026-10-01-consolidation-inventory/`. P1-00 review rounds 1–4 imported into
  `docs/06-sessions/`, ending the invalid `docs/05-sessions/` path.
- ADR-011 (engine-only pivot) and ADR-012 (Python reference oracle), both Proposed. `engine-spec.md`
  drafted in parts to replace both superseded specs, which move verbatim to
  `docs/00-meta/specs/superseded/`.
- Registers: decision register (DR-A1 to DR-A12, DR-B1 to DR-B6, DR-C1 to DR-C15, DR-D1 to DR-D30),
  known-issues backlog, lessons learned. Three contracts, nflverse and CFBD provider documents, eight
  draft model specs and their index.
- `reference/python/`: 105 cautious-nevermore files at `59bce1d` (103 verbatim, patches P1 and P2),
  `MANIFEST.tsv`, `PARITY.md`. Its own suite passed on Linux: 446 tests, threads pinned to 1.
- `docs/07-archive/cautious-nevermore/`: deleted engine documents copied verbatim with a manifest.
- Repository: `app/`, `crates/ffi/`, the Flutter pin, the FRB dependency and `test-ffi` removed (chain
  9 recipes / 16 commands → 8 / 15); traceability accepts `P[0-9]-[0-9]{2}`;
  `check-evidence-claims.sh` generalized; R4-1 and R4-2 guard fixes; guard suite 54 → 78 cases;
  `reference-oracle` CI job (Linux, outside the verify chain); `.gitattributes` keeps the verbatim
  trees byte-exact; entry docs rewritten for the engine.

Found on the way:

- The synthetic generator draws every play's defenders from the offense's own team (KI-NEW-Y0). Team,
  Layer-3, defender and matchup recoveries were never measured against a correct truth.
- DR-B3's Class C is unattainable for V(s): re-seeding the oracle's own booster reaches corr(dV)
  0.989–0.996. Replacement criteria C-V and C-L1 are proposed (DR-D27).
- Tier-0 floors hold only for the canonical seed; ensemble gates proposed (DR-D26).
- The historical real-data results rest on biased labels and iid intervals; non-citable.

Open before merge:

- `engine-spec.md` assembled and committed with its byte-identical mirror (`check-authority-sync`);
  `typos` hits in verbatim records under `docs/`, to be allowlisted, never edited.
- P0-01 evidence generated on the final commit; `ai-toolchain.lock` provenance for P0-01 unrecorded.
- Windows chain and the runner's `python3` unproven until the first CI runs. `chacha20 0.10.1` yanked
  while `cargo deny check` passes.
- Fresh-context review.

Owner decisions pending:

- At merge: ratify or override DR-A1, DR-A2, DR-A4 to DR-A9, DR-A11, DR-A12; accept or override
  ADR-011 and ADR-012.
- DR-A10: the `reference/python/` licence ruling, recorded on the PR before merge.
- DR-A3: follow-up ADR on the authoritative platform before P1-11.
- Every B, C and D decision stays proposed or open; each gates the packages it blocks.
- Tag `oracle-legacy-59bce1d`; close PRs #2 and #3 unmerged; a `python3 -m pytest` permission rule
  (Security/Release).
