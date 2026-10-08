---
adr-id: 011
status: Proposed
date: 2026-10-07
deciders: Product/Architecture owner (DR-A1, DR-A2, DR-A4, DR-A5, DR-A6, DR-A8, DR-A9); Security/Release owner (DR-A7 and the guard changes); Statistical owner (DR-A2, jointly with ADR-012)
supersedes: "001 (in part), 006 (in part), 007 (in part), 009 (in part); see Supersession"
superseded-by:
---

# ADR-011 — Engine-only pivot

## Status

**Proposed** 2026-10-07 under work package P0-01 (`docs/01-work-packages/p0-01-engine-consolidation.md`).
It is not accepted. The owner accepts or overrides it at the merge of the P0-01 pull request, in the
review or comment that ratifies the DR-A decisions by ID (`docs/00-meta/decision-register.md`, "How to
ratify or override", step 3). A merge alone does not ratify a DR that the comment does not name.

**Where this decision gets its force.** It replaces the two documents that outranked every ADR
(`alpha-spec.md` §1.5, superseded). An ADR cannot do that on its own rank. ADR-001 D3 set the precedent:
a deviation from a specification is ratified by the owner directly. The force here comes from the
owner's request of 2026-10-01, the owner's choice of option, and ratification at merge. This ADR is the
record of all three.

## Context

### The request

The owner wrote on 2026-10-01: "consolidate the specs and any useful work that's actually been done in
these repos [GRID-One/GRID-Engine and Seismic-Fate/cautious-nevermore] under the GRID-One/GRID-Engine
repo; pivot the whole repo to dedicate it to just building the engine."

Three options were put to the owner:

| Option | Chosen |
|---|---|
| Rust target plus Python reference. The Rust workspace is trimmed to engine crates. The working Python engine comes in under `reference/python/` as the executable oracle, and its synthetic-recovery gates and golden masters become parity targets. The specs are rewritten to engine-only scope. | **Yes** |
| A Python-only engine | No |
| Rust only, with the Python engine folded into documentation | No |

### What was true before

**GRID-Engine** at `3823478`, the head of PR #1 (P1-00 bootstrap):

- 49 commits ahead of `origin/main` (`48ee320`). Approved after adversarial review round 4 with
  0 blockers, 1 major (R4-1) and 1 minor (R4-2). Not merged.
- Authority 1 and 2 were `final-build-spec.md` and `alpha-spec.md`. Both specified a Windows desktop
  product:
  - a native Flutter Windows UI (final-build-spec §1, its first non-negotiable; alpha-spec §1.1);
  - `flutter_rust_bridge` v2 as the only UI/core boundary;
  - installer packaging;
  - P1-10 as the Flutter board and P1-11 as installer and clean-machine acceptance.
- The repository carried that scope:
  - `app/pubspec.yaml`;
  - the `crates/ffi/` crate (`grid-ffi`);
  - `toolchains/flutter.version`;
  - the `flutter_rust_bridge = "=2.5.0"` workspace dependency;
  - the `test-ffi` and `serve-ui` recipes.
- `test-ffi` executed **0 tests** (0 unit tests, 0 doctests; measured at `3823478`).
- The workspace had 12 crates, and `Cargo.lock` held 172 packages. The verify chain had 9 recipes and
  16 commands.

**cautious-nevermore** (`Seismic-Fate/cautious-nevermore`) at `59bce1d`:

- A fantasy-football dashboard: a FastAPI backend, a React frontend, ESPN and Sleeper league adapters,
  an LLM chart agent, and trade and draft tooling.
- It is built on a working Python GRID engine (`backend/grid`, `projection`, `validation`, `scoring`),
  validated against planted synthetic truth.
- Of its 236 tracked files, 105 form the import-closed engine closure (446 tests). The other 131 are the
  application.

**The inventory** is in `docs/06-sessions/2026-10-01-consolidation-inventory/`: eight reports and a
completeness critic. The critic de-duplicated 33 owner decisions (critic §3: A-1 to A-12, B-1 to B-6,
C-1 to C-15). It also found that the synthetic generator draws defenders from the offense's own team
(KI-NEW-Y0).

**Why a pivot needs the specs rewritten.** The spec set was inconsistent with an engine-only repository
until both specs were replaced: the first non-negotiable of the level-1 spec is the Flutter UI.

## Decision

### D1 — Engine-only scope

This repository builds the GRID statistical projection engine and nothing else:

- a Rust library API (commands, queries and events);
- a headless CLI that is a thin adapter over that API;
- versioned file and database outputs over SQLite.

The repository contains no user interface, installer or FFI layer (`engine-spec.md` §1.1 item 2). The
engine does not change shape for a consumer. Consumers adapt to the versioned output contract
(`engine-spec.md` §1.6, §5.5; `docs/03-contracts/engine-output-contract.md`).

### D2 — Rust is the target; Python is the oracle

The Rust engine is the product.

- The cautious-nevermore Python engine is imported under `reference/python/` as an executable reference
  oracle. ADR-012 governs it.
- No engine crate, binary, build script or release artifact embeds, spawns, links or reads Python
  (`engine-spec.md` §1.1 item 8).

### D3 — One consolidated specification (DR-A1)

- **Authority 1.** `engine-spec.md` at the root, with a byte-identical mirror at
  `docs/00-meta/specs/engine-spec.md`, replaces both `alpha-spec.md` and `final-build-spec.md`.
- **The originals are archived verbatim** at `docs/00-meta/specs/superseded/`. Each archived file is
  byte-identical to its copy at `3823478`.
  - The old mirror `docs/00-meta/specs/alpha-spec.md` was byte-identical to the root copy, so one
    archived copy serves both.
  - Archived copies have no mirror, and nothing compares them.
- **Old citations resolve through the crosswalk.** Immutable records cite "alpha-spec §x" or
  "final-build-spec §y": ADRs, evidence and guard comments. They resolve through `engine-spec.md`
  Appendix H and are never rewritten.

### D4 — Authority order (DR-A2)

1. `engine-spec.md` and its byte-identical mirror
2. accepted ADRs in `docs/02-adr/`
3. contracts, model specs and provider manifests (`docs/03-contracts/`, `docs/05-model-specs/`,
   `docs/04-providers/`)
4. the approved work package in `docs/01-work-packages/`
5. tests and fixtures, including committed reference-oracle fixtures and golden files
6. existing code and local conventions, including the `reference/python/` source

Rules for using the order:

- **Restatements must match.** `CLAUDE.md`, `docs/CLAUDE.md` and `docs/00-meta/authority-index.md`
  restate this order exactly (`engine-spec.md` §1.5).
- **Non-authoritative history.** The superseded specs, `docs/07-archive/` and `docs/06-sessions/` are
  consulted for rationale only.
- **Registers are not authorities.** The registers in `docs/00-meta/` bind only through a document that
  is: the spec, an accepted ADR or a model spec.
- **The oracle.** Where the oracle sits in the order is decided in ADR-012.

### D5 — The AI-governance apparatus is kept (DR-A4)

The following carry over unchanged in substance (`engine-spec.md` §1.3–§1.6, §8.15–§8.20, Appendices B–E):

- the agent boundary, roles and authority;
- the work-package contract and the frozen verification recipes (ADR-001 D5);
- evidence manifests and fresh-context review;
- the rule that the agent never merges.

### D6 — The application is removed

| Removed | How |
|---|---|
| `app/` (`app/pubspec.yaml`) | deleted |
| `crates/ffi/` (`Cargo.toml`, `src/lib.rs`, `src/generated/.gitkeep`) | deleted. The `"crates/ffi"` member was dropped from `Cargo.toml` |
| `toolchains/flutter.version` | deleted |
| `flutter_rust_bridge = "=2.5.0"` and its comment | dropped from `[workspace.dependencies]` |
| `grid-ffi` entry in `Cargo.lock` | removed with `cargo metadata --offline`, 172 → 171 packages. `cargo update` was not run, and no other package version changed (4 deletions, 0 additions) |
| `test-ffi` | removed **in lockstep** from all three implementations: the `verify` recipe line and the recipe in `justfile`, the 3-line step in `scripts/verify.sh`, and the 4-line step with its `Assert-Ok` in `scripts/verify.ps1` |
| `serve-ui` | recipe removed from `justfile`. It was not in the verify chain |

Other effects:

- **The verify chain** goes from 9 recipes and 16 commands to **8 recipes and 15 commands**. The other
  15 commands are byte-identical, in the same order, in all three implementations.
  `scripts/check-verify-parity.sh` reports 15 steps, `tests/guards/run.sh` reports "15 guarded,
  0 unguarded", and `grep -c 'Assert-Ok "' scripts/verify.ps1` is 15.
- **`scripts/bootstrap-repo.sh`** no longer creates `crates/ffi/src/generated`, `app/lib` or `app/test`,
  and it creates `docs/07-archive`.
- **`.github/workflows/alpha-ci.yml`** had no Flutter steps. Only a comment count changes (16 → 15).
- **Untouched:** `deny.toml`, `migrations/`, `.sqlx/`, `.env`, `rust-toolchain.toml` and
  `toolchains/dev-tools.lock`. None of them referenced FFI or Flutter.

#### Why removing `test-ffi` is de-scoping, not weakening

ADR-005 set three conditions for amending a frozen recipe.

1. **"No repository change can satisfy the recipe."** Met. Once the owner removes `grid-ffi`,
   `cargo test -p grid-ffi --features flutter-bridge-tests` cannot pass. The only way to keep it
   passing would be to keep a crate the owner has taken out of scope.
2. **"The amendment strengthens rather than weakens."** Not met as worded, and this ADR does not
   pretend otherwise. Removing a step does not strengthen anything. Nothing is weakened either:
   - the step executed 0 tests;
   - the code it covered is deleted along with it;
   - no check over any surviving code is touched.

   Measured in tests executed, coverage goes from 0 to 0. ADR-007 and ADR-009 refuse "an amendment
   that reduces coverage". This amendment reduces none.
3. **Explicit owner approval, recorded.** Required. The authority for this amendment is the owner's
   scope decision, not the ADR-005 precedent, and the owner ratifies it at merge.

This makes it the **fifth amendment to ADR-001 D5, and the first that removes a step.**

ADR-001 rejected a different alternative: "Relax `check-sqlx`/`test-ffi` to no-op". That would have kept
the crate and silenced its check. This change removes the crate and its check together, so no remaining
code loses a gate.

### D7 — Guard changes (DR-A5, DR-A7)

None of these is a D5 amendment. D5 freezes the verify recipes. Guards, the guard suite and CI are not
frozen (ADR-010). The facts below were measured by the infrastructure change.

| Guard | Change | Why | Class |
|---|---|---|---|
| `scripts/check-authority-sync.sh` | `CANONICAL=engine-spec.md`, `MIRROR=docs/00-meta/specs/engine-spec.md`. The header explains the retarget | The live spec is still mirrored and still byte-compared, so this is a retarget, not a removal. The superseded specs are single archived copies with nothing to sync | retarget |
| `scripts/check-traceability.sh` | The ID regex `P1-[0-9]{2}` becomes `P[0-9]-[0-9]{2}`, and the resolution `case` `P1-*)` becomes `P[0-9]-*)` | Without it, `P0-01` and every `P2-NN` package could not be cited (DR-A5). Widening the syntax changes which documents a commit may cite, not what citing one buys. `covered()` and the per-path semantics are byte-unchanged | strengthen |
| `scripts/check-evidence-claims.sh` | Selects the evidence record instead of hard-coding `WP="P1-00"`. Fails if the change set alters a `.ai/evidence/<WP>/` record that already exists in the base. The `(N/16)` literal becomes "`Assert-Ok` (N/M)", with N and M both re-derived. A failing traceability run is now a named FAIL claim (it used to exit silently under `set -e`). JSON is read with `jq`, else `python3`, else `python`, else FAIL | It could only ever check P1-00. Without the immutability rule, selection could pick a historical record rewritten to be true of a later tree. The P1-00 record now checks **10** claims (the floor rises 8 → 9) | strengthen |
| `scripts/generate-evidence-manifest.sh` | Rejects an ID outside `^P[0-9]-[0-9]{2}$`. Fills `contracts` and `model_specs` from `docs/03-contracts/*.md` and `docs/05-model-specs/*.md`, README excluded, instead of a hard-coded `[]` | The ID becomes a path. P0-01 is the first package with contracts and model specs | strengthen |
| `scripts/check-secrets.sh` | **R4-1:** dies when `listed − excluded − skipped − opened > 0`, and names up to 5 unopened paths. A tracked file deleted in the worktree is read from its index blob | Before the fix, a partial unresolved set passed silently. Reading the index blob answers the open DR-A7 sub-question: such a file is scanned, not skipped | strengthen |
| `scripts/check-verify-parity.sh` and `ps1_unguarded()` in `tests/guards/run.sh` | **R4-2:** the `Write-Host` and `Assert-Ok` skips are anchored to the start of the line. A verify step that joins commands with `;` or `\|\|` outside quotes, in `justfile` or `verify.ps1`, is refused. `verify.sh` is exempt because it runs under `set -e` | Both copies certified evasions such as `cargo sbom generate; Write-Host "done"` as clean. The fix is applied to both copies (ADR-010) | strengthen |
| `tests/guards/run.sh` | **54 → 78 cases**: authority-sync +1, traceability +4, evidence-claims +7, R4-1 +4, R4-2 (ps1 analysis) +4, R4-2 (parity) +4. The authority-sync fixture is renamed to `engine-spec.md` | Each new positive case **fails** when its script is swapped back to its `3823478` version, and every control passes under both versions | strengthen |
| `.gitignore` | The Flutter block is replaced by Python caches (`__pycache__/`, `*.py[cod]`, `.pytest_cache/`, `.venv/`, `venv/`) and `/reference/python/data/` | Untracked oracle caches would otherwise enter the traceability and secrets change sets (critic X-8). Neither an oracle source file nor `tests/grid/golden/snapshot.npz` is ignored | de-scope and strengthen |
| `_typos.toml` | Allowlist entries with a justification comment per group: `vor`, `mis`, `yhat`, `ot_yl`, `ot_dn`, `ot_yds`, `fo_s`, `fo_w`, `pn` (oracle tree); `GAM`, `Tung` (engine-spec terms). Exact-path exclusions for the 22 verbatim copies (nine inventory reports, thirteen archived cautious-nevermore documents), whose spellings can never be corrected in place | Allowlist, never an exclusion of `reference/python/` (critic X-6); verbatim copies excluded file by file, never by directory, and maintained files in those trees stay checked | allowlist + scoped exclusion |
| `.github/workflows/alpha-ci.yml` | Adds the `reference-oracle` job (ADR-012). Comments on the evidence-claims step and on the self-test count. The header's Windows-authority rationale is restated per D8 | `windows-authoritative` and `linux-smoke` are otherwise byte-unchanged. The evidence-claims command is unchanged, because the script now selects the record | strengthen |

**Residuals, recorded rather than fixed:**

- **R4-1 side effect.** Any listed path that is not a regular file now fails `check-secrets`, and the
  message names it. One example is an untracked nested git repository listed as `dir/`. Whether that
  should count as a deliberate skip is for the reviewer to decide.
- **R4-2 gaps.** Pipes (the justfile's `bash -cu` has no `pipefail`) and a command on the same line as a
  construct keyword are not caught. A future `for (;;)` in `verify.ps1` would be refused (fail-closed).
- **Rename gap (pre-existing).** In a committed range, traceability sees only a renamed file's
  destination.
- **Windows portability.** The new evidence fixtures in `run.sh` run inside `verify.ps1` on
  `windows-authoritative`. They need `jq`, `python3` or `python`, plus `sha256sum`. Only the first real
  Windows run can confirm that.

### D8 — Windows stays merge-authoritative (DR-A3)

`scripts/verify.ps1 -Scope Full` and the `windows-authoritative` job stay the merge gate. **The reason
changes, and the decision does not.**

- The old rationale was "the production target is Windows". The engine has no Windows application, so
  that rationale no longer holds.
- Windows authority is retained only because removing it would reduce coverage without its own ADR
  (ADR-007, ADR-009).
- A follow-up ADR (Product/Architecture owner and Security/Release owner) should land before P1-11. It
  may make Linux authoritative, with Windows as a matrix job.
- Either way, the Python oracle job is Linux-only (ADR-012).

### D9 — Work-package numbering (DR-A5)

- **Form.** IDs keep the form `P<phase>-<NN>`. **P0-01** is this consolidation.
- **Phase 1.** P1-01 to P1-09 keep their numbers and are re-sequenced for the engine. Two are re-scoped
  and one is new:
  - **P1-10**, was the Flutter UI, becomes the engine CLI, reports and exports;
  - **P1-11**, was installer and clean machine, becomes recovery, the release artifact (the `grid` CLI
    binary and library crates), clean-checkout reproduction and end-to-end acceptance evidence;
  - **P1-12** is new: the GRID component port.
- **Phase 2.** P2-00 to P2-08 are kept, re-scoped for the engine. **P2-09**, an optional oracle
  retirement review, is new (ADR-012).
- **No suffixed IDs.** A split takes the next free number, because traceability resolves IDs by
  filename glob.
- **The index** is `docs/01-work-packages/README.md`, from `engine-spec.md` §9.5 and §10.5.
- **Earlier deferrals carry to the re-scoped P1-11** unless superseded below. These are:
  - `-Scope Changed` (ADR-001 D5, ADR-008);
  - repository-wide line-ending normalization (ADR-004);
  - CI caching (ADR-007);
  - runbooks and model cards (ADR-006 item 4);
  - `toolchains/native-dependencies.lock`, now only if a native booster is adopted (DR-C8, proposed).

### D10 — PR #1 merge semantics (DR-A6, adapted)

The critic's default was to merge PR #1 first. This branch instead contains PR #1's 49 commits as a
fast-forward, verified three ways:

- `3823478` is the base of the consolidation work;
- `git rev-list --count origin/main..3823478` is 49;
- `8d43203`, the commit P1-00's manifest attests, is an ancestor.

What follows from that:

- **Merge with a merge commit, never a squash.** A merge commit keeps `8d43203` reachable. A squash
  would orphan it.
- **PR #1** is then closed as included.
- **PRs #2 and #3** are closed unmerged. Their review records (rounds 1–4) are imported verbatim into
  `docs/06-sessions/`, and merging either would recreate the invalid `docs/05-sessions/` path.

Closing the PRs and merging are owner actions.

### D11 — Crate plan (DR-A8)

- **P0-01 drops only `ffi`.** The workspace has 11 crates. The `application` and `governance` doc
  comments are reworded with no code change (`crates/application/src/lib.rs`,
  `crates/governance/src/lib.rs`, `docs/CLAUDE.md`).
- **P1-01 restructures.** It renames `application` → `pipeline`, adds a `grid-cli` binary crate, and adds
  a `synth` crate (`engine-spec.md` §8.1).
- **`persistence` and SQLite stay.** `check-sqlx` and `test-rust` depend on them.

### D12 — Archive (DR-A9)

`docs/07-archive/cautious-nevermore/` is registered in `docs/00-meta/authority-index.md` as
**non-authoritative history**. It holds:

- the cautious-nevermore engine documents deleted by CN PR #92;
- the whiteboard before it was cleared;
- the Phase-4 SDD reports T1 and T5–T8;
- `HISTORY.md`;
- `real-data-results.md`, which is historical and never a parity target.

Copies are verbatim. Known errors in them are recorded in `MANIFEST.md` and never fixed in place.

## Supersession

Each superseded part takes effect when this ADR is accepted. Until then, the annotation in the older
ADR's front matter reads "proposed" (`docs/02-adr/README.md`, "Partial supersession").

| ADR | Superseded in part | Not superseded |
|---|---|---|
| ADR-001 | (a) "Crate boundaries": twelve crates including `ffi` becomes eleven (D6, D11). (b) Compliance rows "Authority order" and "Authority integrity": the order is D4's, and `check-authority-sync.sh` compares the two `engine-spec.md` copies, not the `alpha-spec.md` copies. (c) D5's recipe count, "nine since ADR-007", becomes eight; this is the fifth amendment (D6). (d) "The work-package template is not byte-exact to Appendix B": the reference is now `engine-spec.md` Appendix B. The two documented additions stay, and the frontmatter key `alpha-phase` becomes `phase`. (e) "Inherited. P1-01…P1-11" now reads per D9 | D1, D2, D3 (extended by D12 with the `docs/07-archive/` slot), D4, the D5 rule itself, D6, the choice of `just` (its Windows-target rationale is replaced by D8), the work-package amendment precedent, and the fixtures deferral (now read together with ADR-012 on fixture policy) |
| ADR-006 | Item 6 for `app/lib/main.dart` and `app/pubspec.lock`, the sentence "`toolchains/flutter.version` is committed now", and the matching rows of its "Absent" table. All are withdrawn: there is no app in scope | Items 1–5. `toolchains/native-dependencies.lock` stays with P1-11, only if a native booster is adopted |
| ADR-007 | The chain listing in its Decision, which names `test-ffi` and "16 steps". It is now 8 recipes and 15 commands | Its decision to add `test-doc` and `check-guards` |
| ADR-009 | "Sixteen call sites" and the Compliance count "(16 today)" become 15: `cargo test -p grid-ffi` and its `Assert-Ok` leave with D6. The Context rationale "the production target is Windows" is replaced by D8 | Its decision of one `Assert-Ok` per native command, with each step named, and its compliance rule |

**Not superseded:**

- **ADR-002.** Its mention of `crates/ffi/src/generated/` as a never-hand-edit path becomes historical.
- **ADR-003.** Its Windows-compatibility rationale ("Windows is the production target") is orphaned.
  The decision stands, and D8 keeps the Windows gate.
- **ADR-004.** Its P1-11 deferral carries (D9).
- **ADR-005.** Its three conditions are applied in D6.
- **ADR-008.** Its `-Scope Changed` deferral carries (D9).
- **ADR-010.** The R4-1 and R4-2 fixes apply it.

## Consequences

**Easier.**

- One authority document describes the product actually being built.
- `P0-01` and `P2-NN` can be cited.
- The evidence checker works for any package.
- Two known guard holes (R4-1, R4-2) are closed with mutation-checked cases.
- The working Python engine is preserved as executable evidence instead of prose (ADR-012).

**Harder.**

- **The D5 chain shrinks for the first time.** The argument in D6 must hold up under review. A future
  removal will cite this one as precedent, and it should be read narrowly: a step leaves only with the
  code it covered.
- **Citation churn.** More than 100 citations of superseded sections in immutable records now need
  Appendix H to resolve.
- **Windows authority stays without its original rationale** until the follow-up ADR.
- **Review load.** One pull request carries about 260 non-trivial paths, most of them new documents
  and the verbatim oracle tree.
- **Most decisions are only proposed.** Every B, C and D decision is proposed or open, so most
  Phase 1 packages are not Ready until the owner rules (`engine-spec.md` §8.16.1).
- **Ratification can change the PR.** The A-decisions are implemented before ratification. If the owner
  overrides one, the PR changes before merge.

**Inherited.** Every later package inherits the engine-only scope, the six-level order, the `P<phase>-NN`
scheme, the 8-recipe chain and the 11-crate workspace. An error here propagates to all of them.

## Compliance

| Decision | Enforced or detected by |
|---|---|
| D3 | `scripts/check-authority-sync.sh`: `engine-spec.md` must equal its mirror, and a missing mirror fails closed |
| D4 | Review. The three restatements must match `engine-spec.md` §1.5. No guard compares them |
| D6 | `scripts/check-verify-parity.sh` (15 steps, three-way); `tests/guards/run.sh`; the ADR-009 self-test in CI; and in review: `test ! -e crates/ffi && test ! -e app && test ! -e toolchains/flutter.version`, `! grep -q flutter_rust_bridge Cargo.toml Cargo.lock` |
| D7 | `tests/guards/run.sh` (78 cases, each new case mutation-checked); `scripts/check-evidence-claims.sh` in the `guards` job |
| D8 | `.github/workflows/alpha-ci.yml` `windows-authoritative`, unchanged |
| D9 | `scripts/check-traceability.sh` (accepts `P[0-9]-[0-9]{2}`); `generate-evidence-manifest.sh` and `check-evidence-claims.sh` reject other IDs |
| D10 | Merge settings, an owner action: merge commit, never squash. Afterwards, `git merge-base --is-ancestor 8d43203 main` |
| D2 (no Python in the engine) | Review: `grep -rn -i 'python\|pyo3' crates/ Cargo.toml` must be empty, and `grep -c pyo3 Cargo.lock` must be 0. No guard asserts this yet; a structural guard is a candidate for P1-01 |

## Alternatives considered

- **A Python-only engine.** Offered to the owner and not chosen. It contradicts the Rust-core and
  no-Python-runtime constraints carried from both specs. It would also discard P1-00's Rust workspace,
  SQLx toolchain and verification gates.
- **Rust only, with the Python engine folded into documentation.** Offered and not chosen. Prose cannot
  re-run a recovery gate, regenerate a fixture, or demonstrate a defect. Parity would then rest on
  descriptions of behaviour rather than on behaviour.
- **Keep the `alpha-spec.md` and `final-build-spec.md` filenames with rewritten content** (DR-A1
  option 1). It needs no guard churn. Rejected because the 100-plus immutable "§x" citations would
  silently point at renumbered sections. A new name makes every stale citation visibly stale.
- **Keep `alpha-spec.md` as the only spec and demote `final-build-spec.md`** (DR-A1 option 3).
  Rejected: alpha-spec is itself app-scoped throughout (§1.1, §8.7, §9.5), so it would need the same
  rewrite under a name that implies continuity.
- **Drop or restructure more crates now** (DR-A8 options 2 and 3).
  - The full restructure (`application` → `pipeline`, plus `grid-cli` and `synth`) would add crate-count
    churn to the evidence claims and mix structural work into a documentation and scope PR. It is
    deferred to P1-01.
  - Dropping `persistence` and SQLite would remove `check-sqlx` coverage. That is a coverage-reducing
    change needing its own ADR, and nobody proposes it.
- **Keep the FFI and Flutter scaffolding dormant.** Rejected: dead scaffolding contradicts D1, and a
  check that runs 0 tests is a gate that passes vacuously (ADR-002).
- **Make Linux authoritative in this PR.** Rejected: it reduces coverage without its own ADR (DR-A3).
- **Merge PR #1 separately first** (the critic's DR-A6 default). Adapted rather than rejected. The
  consolidation was built on `3823478`, so carrying PR #1 as a fast-forward and merging with a merge
  commit gives the same reachability guarantee. A squash is refused under either option.
- **A new ID scheme such as `E1-01`** (DR-A5 option 2). It needs the same guard change and breaks
  number stability for the ADR-006 and authority-index citations of P1-01 to P1-11.

## Paths affected

`scripts/check-traceability.sh` requires every non-trivial changed path to be named in a document the
commit messages cite. Paths are named exactly, or by a directory prefix in backticks. Any path not
listed here is outside P0-01. The same list appears in the P0-01 work package.

**Added**

- `engine-spec.md`
- `docs/00-meta/specs/engine-spec.md`
- `docs/00-meta/specs/superseded/`:
  - `docs/00-meta/specs/superseded/alpha-spec.md`
  - `docs/00-meta/specs/superseded/final-build-spec.md`
  - `docs/00-meta/specs/superseded/README.md`
- `README.md`
- `docs/00-meta/decision-register.md`
- `docs/00-meta/known-issues.md`
- `docs/00-meta/lessons-learned.md`
- `docs/01-work-packages/README.md`
- `docs/01-work-packages/p0-01-engine-consolidation.md`
- `docs/02-adr/011-engine-only-pivot.md`
- `docs/02-adr/012-python-reference-oracle.md`
- `docs/03-contracts/`:
  - `docs/03-contracts/README.md`
  - `docs/03-contracts/engine-output-contract.md`
  - `docs/03-contracts/parity-fixture-contract.md`
  - `docs/03-contracts/plays-contract.md`
- `docs/04-providers/nflverse/access-and-license.md`
- `docs/05-model-specs/`:
  - `docs/05-model-specs/README.md`
  - `docs/05-model-specs/cross-league-priors.md`
  - `docs/05-model-specs/evaluation-and-leakage.md`
  - `docs/05-model-specs/layer1-credit.md`
  - `docs/05-model-specs/projection-stack.md`
  - `docs/05-model-specs/rapm-attribution.md`
  - `docs/05-model-specs/state-space-kalman.md`
  - `docs/05-model-specs/synthetic-world.md`
  - `docs/05-model-specs/value-model.md`
- `docs/06-sessions/2026-10-01-consolidation-inventory/`
- `docs/06-sessions/review-P1-00-adversarial-round1.md`
- `docs/06-sessions/review-P1-00-adversarial-round2.md`
- `docs/06-sessions/review-P1-00-adversarial-round3.md`
- `docs/06-sessions/review-P1-00-adversarial-round4.md`
- `docs/06-sessions/review-P0-01-adversarial-round1.md`
- `docs/07-archive/`
- `reference/python/`
- `.ai/evidence/P0-01/`

**Changed**

- `CLAUDE.md`
- `docs/CLAUDE.md`
- `docs/00-meta/authority-index.md`
- `docs/00-meta/dashboard.md`
- `docs/00-meta/daily-log.md`
- `docs/02-adr/README.md`
- `docs/02-adr/001-repo-bootstrap-decisions.md` (front matter `superseded-by` only)
- `docs/02-adr/006-deferred-deliverables.md` (front matter `superseded-by` only)
- `docs/02-adr/007-verify-covers-guards-and-doctests.md` (front matter `superseded-by` only)
- `docs/02-adr/009-verify-ps1-exit-codes.md` (front matter `superseded-by` only)
- `docs/04-providers/cfbd/README.md`
- `docs/04-providers/nflverse/README.md`
- `docs/06-sessions/README.md`
- `docs/99-templates/template-model-spec.md`
- `docs/99-templates/template-work-package.md`
- `Cargo.toml`
- `Cargo.lock`
- `ai-toolchain.lock`
- `crates/application/src/lib.rs`
- `crates/governance/src/lib.rs`
- `justfile`
- `scripts/verify.sh`
- `scripts/verify.ps1`
- `scripts/bootstrap-repo.sh`
- `scripts/check-authority-sync.sh`
- `scripts/check-traceability.sh`
- `scripts/check-evidence-claims.sh`
- `scripts/check-secrets.sh`
- `scripts/check-verify-parity.sh`
- `scripts/generate-evidence-manifest.sh`
- `tests/guards/run.sh`
- `_typos.toml`
- `.gitignore`
- `.gitattributes`
- `.github/workflows/alpha-ci.yml`

**Deleted or moved**

- `app/` (`app/pubspec.yaml`)
- `crates/ffi/` (`crates/ffi/Cargo.toml`, `crates/ffi/src/lib.rs`, `crates/ffi/src/generated/.gitkeep`)
- `toolchains/flutter.version`
- `alpha-spec.md`, moved to `docs/00-meta/specs/superseded/alpha-spec.md`
- `final-build-spec.md`, moved to `docs/00-meta/specs/superseded/final-build-spec.md`
- `docs/00-meta/specs/alpha-spec.md`, the old mirror, deleted (byte-identical to the archived copy)

**Checked and unchanged:** `deny.toml`, `migrations/`, `.sqlx/`, `.env`, `rust-toolchain.toml`,
`toolchains/dev-tools.lock`, `.ai/evidence/P1-00/` (immutable) and `.claude/`.
