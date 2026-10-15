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
- Registers: decision register (DR-A1 to DR-A12, DR-B1 to DR-B6, DR-C1 to DR-C15, DR-D1 to DR-D31),
  known-issues backlog, lessons learned. Three contracts, nflverse and CFBD provider documents, eight
  draft model specs and their index.
- `reference/python/`: 105 cautious-nevermore files at `59bce1d` (102 verbatim; patches P1, P2 and
  correction-ledger entry L0), `MANIFEST.tsv`, `PARITY.md`. Its own suite passes on its pinned platform
  (CPython 3.11, `OPENBLAS_CORETYPE=Haswell`): 446 tests, threads pinned to 1.
- `docs/07-archive/cautious-nevermore/`: deleted engine documents copied verbatim with a manifest.
- Repository: `app/`, `crates/ffi/`, the Flutter pin, the FRB dependency and `test-ffi` removed (chain
  9 recipes / 16 commands → 8 / 15); traceability accepts `P[0-9]-[0-9]{2}`;
  `check-evidence-claims.sh` generalized; R4-1 and R4-2 guard fixes; guard suite 54 → 78 cases;
  `reference-oracle` CI job (Linux, outside the verify chain); `.gitattributes` keeps the verbatim
  trees byte-exact; entry docs rewritten for the engine.
- `engine-spec.md` committed with its byte-identical mirror (`check-authority-sync`). `typos` excludes
  the 22 verbatim records under `docs/` by exact path, never by directory, and they are never edited.

Found on the way:

- The synthetic generator draws every play's defenders from the offense's own team (KI-NEW-Y0). Team,
  Layer-3, defender and matchup recoveries were never measured against a correct truth.
- DR-B3's Class C is unattainable for V(s): re-seeding the oracle's own booster reaches corr(dV)
  0.989–0.996. Replacement criteria C-V and C-L1 are proposed (DR-D27).
- Tier-0 floors hold only for the canonical seed; ensemble gates proposed (DR-D26).
- The historical real-data results rest on biased labels and iid intervals; non-citable.
- PR #4's first `reference-oracle` run failed 2 of 446 tests, both Layer C golden-master tests: the
  imported golden reproduces at rtol 1e-5 only on CPython 3.11 with OpenBLAS AVX-512 kernels
  (KI-NEW-Z78). The owner ratified DR-D31 option 1 on 2026-10-08: the job runs the tool-cache CPython
  3.11, every oracle run pins `OPENBLAS_CORETYPE=Haswell` (enforced by
  `reference/python/tools/pytest_platform_pin.py`), and correction-ledger entry L0 regenerated the
  Layer C golden once under that platform. No tolerance is loosened and no test is skipped.

Open before merge:

- P0-01 evidence generated on the final commit; `ai-toolchain.lock` provenance for P0-01 unrecorded.
- The Windows chain is unproven until its CI run on the final commit. `chacha20 0.10.1` yanked while
  `cargo deny check` passes.
- Fresh-context review.

Owner decisions pending:

- At merge: ratify or override DR-A1, DR-A2, DR-A4 to DR-A9, DR-A11, DR-A12; accept or override
  ADR-011 and ADR-012.
- DR-A10: the `reference/python/` licence ruling, recorded on the PR before merge.
- DR-A3: follow-up ADR on the authoritative platform before P1-11.
- Every B, C and D decision stays proposed or open; each gates the packages it blocks.
- Tag `oracle-legacy-59bce1d`; close PRs #2 and #3 unmerged; a `python3 -m pytest` permission rule
  (Security/Release).

## 2026-10-08 to 2026-10-09 — P0-01 owner decision interview

The owner asked to be interviewed "to close any decisions that I can make right now". One question
at a time, through the harness's question tool; the record, with every answer verbatim, is
`docs/06-sessions/2026-10-09-P0-01-decision-interview.md`.

- Asked: DR-A1 to DR-A12; every B, C and D entry except DR-D9 and DR-D10 (both need real-data
  measurements first), DR-D27 and DR-D31; ADR-011/012 acceptance; and the owner information request.
  DR-D27 was not asked separately: the DR-B3 answer settled its decidable part (C-V and its two numeric
  limits). DR-D31 was already ratified (2026-10-08).
- Ratified as the A-decisions stand: DR-A1, A2, A4 to A9, A11, A12 (to be named again in the merge
  comment). DR-A3: direction ratified (Linux authoritative, Windows additional); the follow-up ADR is
  still owed before P1-11. DR-A10: option 1 chosen; the owner posts the ruling on the PR. ADR-011 and
  ADR-012 are to be accepted in the merge comment (the owner's answer).
- Overrides of a recommended default: DR-B3 (adopt the C-V/C-L1, Class B and seed-ensemble
  amendments), DR-C6 (the Kalman restarts from the prior in production too, via DR-D17), DR-C12
  (official labels reconciled against the play-by-play rebuild), DR-C13 (adds `goal_line`,
  `goal_line_wr`, `third_and_long`, `fourth_down`), DR-D26 (the low quantile is gated too), DR-D30
  (always required).
- Still open in full: DR-D2, DR-D3 and DR-D13 (interim rules confirmed, directions recorded), DR-D9 and
  DR-D10. Every "in part" entry's register line names what stays open.
- The register's Status lines, the summary table and the documents that tagged these decisions
  "(proposed — DR-xx)" were updated to match.

## 2026-10-14 — P0-01 owner decisions on DR-D14, DR-D17 and DR-D19

The owner asked the agent to "settle d14 and d17 before the PR". Eleven questions, through the
harness's question tool and in chat; the record, with every answer verbatim, is
`docs/06-sessions/2026-10-14-P0-01.md`. No register entry had a recommended default for these parts;
every recommendation was the agent's.

- DR-D14, decided in part (the uncovered-defender policy in both DR-B6 roles): a defender with no
  defensive play among the fold's training plays, or with a `no_data` coefficient, takes his RAPM
  prior mean m_p, a declared, counted substitute; a defender missing from the fold's ratings stays
  `CreditError::UncoveredDefender`. No real-data Layer-1 run that used a substitute is promoted
  until a pre-registered recovery check of the substitute against planted truth on synthetic-world
  requirement R-7 passes. The ratified training rows stand (answer 4 superseded answer 3); the
  DR-C7 context-model comparison also compares them with nested out-of-fold rows, on weekly-credit
  recovery and out-of-sample forecasting, with statistics and a margin pre-registered first.
- DR-D17, decided in part: the windowed prior for a player with NFL history is the recency-weighted
  pooled mean. For a player with a feeder prior, that prior is the base of his windowed prior in
  every season; every other player's base is the population prior. Answer 9 confirmed three
  details found in review as part of the decision: the window evidence (participation-free
  prior-builder records), the per-season observation variance v_s = r_c/N_s + ω_c·Σ_w (n_{s,w}/N_s)²,
  and the population prior N(m, σ²_c) per (position class, role), estimated on the window (its
  covariates stay open, DR-C9).
- DR-D32, new and decided: two priors, strict window. Each season-boundary prior is built twice from
  the same data version, on S−3..S−1 for the preseason and week-1 forecast and on S−2..S−1 for the
  state the filter restarts from. Two canaries prove the window; answer 11 confirmed that both hold
  the model version's learned constants fixed (κ_c, λ_c and the filter's `SSParams`), which are
  calibrated by rolling origin, generally on seasons before the window, and are not window evidence.
  S−3's weight after week 1 and the three- against two-season comparison stay open as P1-12
  measurements.
- Still open, before P1-12 tunes or gates anything or implements the windowed-prior builder: the
  R-7 check's rates, offsets, statistics, margins, pass rule and gate kind, and the training-row
  comparison's statistics and margin (DR-D14, DR-C7); the tuning criterion of κ_c and λ_c (the base
  scale and the recency weight), the window-hyperparameter estimator (DR-D17, DR-C5), and the
  minimum class size and the degenerate-pool thresholds (DR-D17). None of them blocks the start of
  P1-12. The comparison's worlds and seed ensemble also stay open.
- Review also found that the live context model `g′_S` trains on a stored `δ^team` that carries team
  data from before the window (KI-NEW-Z79). It needs a decision request.
- Review also found that a stored production track cannot supply the DR-D19 targets that each
  DR-D32 build needs from its own seasons. Such a track restarted from a prior built around the
  player's own college prior and used inputs fitted before the build's window, so for a
  feeder-linked track neither canary could pass. Answer 10 (DR-D19, with DR-D32) chose a
  season-level target with no seed. Each build computes its targets from its own window only, and a
  stored production track never supplies one. Every option offered computed each build's targets
  from that build's own window, and "Keep open" said so explicitly (the record, answer 10); with
  DR-D32's second canary (answer 6) and DR-D19's "the prior never trains on itself", that part does
  not wait on ADR-013.
- Under answer 10, in each build, a feeder-linked, non-ambiguous track's target is the mean of its
  prior-builder residuals over its role events in its first window season with regular-season role
  events. Its known variance comes from the build's own r_c and ω_c. No filter run and no prior of
  any kind enter the target. Per equivalency class, a Fay–Herriot-type measurement-error regression
  on `feeder_sv` fits the translation. In DR-C9's proposed engine form, the equivalency's residual
  variance is σ̂²_u, net of the known target noise, and the §6.5 widening applies to it.
- Answer 10 reverses DR-D19's 2026-10-09 decision in part: the estimand moves from end-of-season
  smoothed talent to the season level. The Kalman credit scale, no separate scale map and "the
  prior never trains on itself" are kept. Register rule 6 requires a reversing ADR that cites
  DR-D19, and the option as offered named that route, so ADR-013 is written as Proposed. It binds
  once the owner accepts it; until then DR-D19's 2026-10-09 text stands. Once it binds, both DR-D32
  canaries can hold for feeder-linked tracks. P1-12 implements the fit only after the owner's ruling
  on ADR-013 and once the estimator, including its weighting (DR-C5), the equivalency class and the
  minimum shared players per class and build (DR-D19, DR-C9) are set. Also still open:
  whether σ²_prior also carries the line's parameter uncertainty (DR-C9), and `P0_form`,
  `P0_scheme` and P0 for window-restarted tracks (DR-D19).
- At the owner's direction (answers 8 and 11), the register and the documents that implement these
  answers were updated. The owner confirms the record in the P0-01 merge comment and accepts or
  rejects ADR-013 there; on rejection, the feeder-equivalency target goes to a new decision request.

## 2026-10-15 — P0-01 owner merge review, DR-A10 ruling and merge

The owner posted the merge review as PR #4 comment 6185953773 (06:27:27Z) and merged PR #4 with a
merge commit, `b819ae0` (06:27:42Z; first parent `48ee320`, second parent `1ee42f7`, the PR head,
whose tree it equals). The record, with the owner's three answers verbatim, is
`docs/06-sessions/2026-10-15-P0-02.md`.

- Merge review (owner, all roles): ADR-011 and ADR-012 accepted; DR-A1, DR-A2, DR-A4 to DR-A9,
  DR-A11 and DR-A12 named again (ratified 2026-10-08), so the `engine-spec.md` §1.5 order is in
  force; DR-A3 restated: Windows stays merge-authoritative until the follow-up ADR, whose direction
  (Linux authoritative, Windows additional) is ratified. Both decision records confirmed as
  transcribed into the register. ADR-013 accepted (DR-D19: the season level, with no seed), so it
  governs the parts of DR-D19's 2026-10-09 text it reverses.
- DR-A10: the ruling in the same comment said "Apache-2.0 terms". Asked, the owner answered
  `MIT OR Apache-2.0` (answer 1), option 1 as chosen on 2026-10-08, and edited the comment at 12:57Z
  to read "`MIT or Apache-2.0` terms". The imported `reference/python/` code, and later corrections
  to it under `PARITY.md`, is under `MIT OR Apache-2.0`. Real third-party data stays excluded
  (DR-A11), and so does `backend/db/data/coaching_changes_2025.json` as data (KI-NEW-D1).
- PR #1's attested commit `8d43203` is an ancestor of `main` (DR-A6). GitHub records PR #1 as
  merged, its head `3823478` being in `main`; PRs #2 and #3 were closed unmerged.
- P0-01 is Done. The follow-up (answer 2) is one PR from `main`, run as work package P0-02 (answer 3;
  `docs/01-work-packages/p0-02-record-p0-01-merge-rulings.md`). It cites the corrected ruling and, at
  the owner's direction, sets ADR-011, ADR-012 and ADR-013 to Accepted; the owner reviews and merges
  it.
- Now that P0-01 has merged, the owner applies the branch-protection setting that makes
  `reference-oracle` a required status check (DR-D30). Still open: DR-A3's follow-up ADR before
  P1-11; the KI-NEW-Z79 decision request, still to raise; the P1-12 pre-registrations (DR-D14,
  DR-D17, DR-D19 with DR-C5 and DR-C9); every open B, C and D part; `ai-toolchain.lock` provenance
  for P0-01, unrecorded.
