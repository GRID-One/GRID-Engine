# P1-01 — Domain types, engine API skeleton, oracle-fixture harness and crate restructure

## Outcome

The workspace now has the DR-A8 crate set (13 members: `application` → `pipeline`, new `grid-cli` and
`synth`), and every later Phase 1 package gets its foundations: the canonical IDs and version IDs, the
`AsOf` type with its publication-lag axis, the typed error and status vocabulary with the exit-code
table, the `grid.plays` v1 types and core validators, the `grid.output` 1.0 record types with JSON
round trips, the §8.2–§8.4 command, query and event surface as types and traits, the DR-D29
parity-fixture reader and tolerance helpers, `SyntheticTruth` and the `from_legacy_v0` loader, a
structural test of the crate boundaries, and the §8.18 agent definitions. Nothing computes a
projection: no command or query has an implementor, by design of this skeleton package. Work package:
`docs/01-work-packages/p1-01-engine-skeleton.md`.

- **ADR-014 (Proposed) decides engine-spec §8.1 rule 5:** `grid-domain` takes `serde` derives as its
  only external dependency; format crates stay in `grid-pipeline` and `grid-synth`; errors are
  hand-written; no feature flag. Implemented while Proposed, as ADR-011/012/013 were; **you accept or
  reject it in this review.** Rejection sends the package back (mirror types would contradict
  engine-output-contract §1 rule 1).
- **No new dependency.** `serde` and `serde_json` were already in `[workspace.dependencies]` and in
  `Cargo.lock`; the lockfile changes only by the rename plus `grid-cli` and `grid-synth`
  (171 → 173 packages). `grid-pipeline` enables `serde_json`'s `float_roundtrip` feature so a parsed
  `f64` keeps its bits (engine-spec §12.6). Intra-workspace crates get versioned
  `[workspace.dependencies]` entries because `cargo deny`'s `wildcards = "deny"` treats a version-less
  path dependency as a wildcard; the policy is unchanged.
- **Contract names fixed** (plays-contract §11, engine-output-contract §10) and the **exit-code table**
  added (engine-output-contract §13: Success 0; NoNewData 10; PartialSuccess 11; NetworkFailure 20;
  Timeout 21; RateLimited 22; AuthenticationFailure 23; InvalidResponse 24; SchemaFailure 25;
  DataValidationFailure 26; DatabaseFailure 30; LearningFailure 40; usage 2; internal 70).
- **Parity target S-1 is prepared, not asserted.** No oracle-exported fixture exists (engine-spec
  §7.12.3); the loader is proven on hand-built toy worlds and P1-05 adds the one-line assertion over the
  committed canonical world. §9.5's parity column and §7.12.3 read differently on when that fixture
  exists; this package follows §7.12.3, §8.1 and §9.2.1 item 7 and records the reading for you.

## Decision requests and amendments for your ruling (work package, "Amendments")

Each row of the work package's dated "Amendments" section is proposed under the authority order and
binds only once you rule at the merge. Four are decision requests:

- **Row 11, Security/Release owner:** reviewer agents are read-only by instruction, not containment.
  Their tool lists carry no Edit or Write, but they keep Bash, and `.claude/settings.json` auto-allows
  some writing commands for every agent. Accept the residual risk, remove Bash from the reviewers, or
  add deny/ask rules in `.claude/settings.json` (a §14.1 change this package does not make).
- **Row 20, Product/Architecture owner:** the `EngineCommands` and `EngineQueries` methods are
  synchronous `fn`, because the CLI may depend only on the orchestration crate and `domain`;
  implementors run Tokio behind the façade from P1-02. If you prefer `async`, P1-02 changes the traits
  before the first implementor exists.
- **Row 21, Statistical owner:** the tolerance helpers use the ∞-norm for Class A′, as engine-spec
  §7.12.5 writes it; `rapm-attribution.md` §10.3 P-3 pre-registers the 2-norm. Amend the model spec,
  or the spec to name both, before P1-12 writes its first A′ test.
- **Row 22, Product/Architecture owner:** engine-spec §8.1 gives the CLI subcommands to P1-10 while
  §10.2.6 gives five surfaces to their Phase 2 content owners (P2-01, P2-02, P2-06, P2-07). The usage
  shows both and you decide.

The other rows are proposed contract additions on Drafts (rows 1 to 4, 7, 9, 15 to 19, 23), status
wording (6, 13), a withdrawn qualifier turned open item (5: may a numerical crate's parity tests take
`grid-synth` as a dev-dependency?), recorded facts (8, 12, 14, 24) and the superseded row 10. Open
items carried into engine-output-contract §12: the serialized spelling of the status enums and of
`PlayerKey` and `TeamKey`, the research marker per row or per run for query 10, the key of a
derived-horizon record, and a null form for `oldest_source_timestamp`.

## Impact

- **Architecture:** the DR-A8 crate set; ADR-014 (Proposed). **Contracts:** `grid.plays` v1 §11 names
  fixed; `grid.output` 1.0 §10 names fixed, §13 added, §4.2 `regime_flags` aligned to engine-spec §5.5,
  §3.9 states sd ≥ 0, §2 `lock.at`; `grid.parity-fixture` §5.1 (the `synthetic-world` array names and
  the `provenance.synth_config_sha256` key) and §11 (reader location). All three stay Draft; your
  approval of this package makes them Approved. **Schema/migrations:** none. **Models:** none.
  **Engine API:** the skeleton surface in `grid-pipeline` (traits with no implementor).
- **Oracle:** nothing under `reference/python/` changes; no fixture or golden is created or regenerated;
  no parity claim is made.
- **Harness:** `.claude/agents/` (seven roles) and `.claude/skills/work-package/`;
  `.claude/settings.json` unchanged (Security/Release owner review requested).
- **Known issues:** none introduced. The v1 canonical types resolve KI-NEW-Z49 (producer dtype drift)
  for every Rust consumer. The `from_legacy_v0` loader carries three oracle defects as-is on purpose,
  labelled `synthetic-legacy`: KI-NEW-Z38 (duplicate participation ids, kept as a multiset), KI-NEW-Z39
  (`yardline_100` up to 104) and KI-NEW-Y0 (defenders drawn from the offense; ledger entry 1 is
  proposed, not applied). KI-A4, KI-A8, KI-P1, KI-V8, KI-NEW-C1, KI-NEW-Z61 and KI-NEW-Z71 are
  deliberately not reproduced.

## Non-goals, tests and rollback

- **Non-goals:** no §8.2 command or §8.3 query is implemented; no persistence, ingestion, identity,
  feature, numerical or GRID-component code; no oracle exporter, committed parity fixture or fixture
  guard; no CLI subcommand, report or export; no change to the frozen verify chain, the permission
  profile, CI, the toolchain pin, migrations or the SQLx cache; no new dependency.
- **Tests added and why:** `grid-domain` unit tests pin the IDs, the strict `published_at < lock.at`
  rule, the error and exit-code tables and every output invariant. `grid-pipeline` tests round-trip
  every `grid.output` record through JSON bit-exactly (`output_round_trip.rs`), pin the serialized
  spellings that are still open items (`serialized_spelling.rs`) and enforce the crate boundaries
  structurally (`crate_boundaries.rs`). `grid-synth` tests cover the fixture reader, its hardening
  against hostile manifests (`fixture_hardening.rs`), the tolerance classes and the v0 loader's
  column, id-order, NaN-bit and sentinel round trips. `grid-cli` tests pin `--version`, `--help`,
  `exit-codes` and the usage exit code 2.
- **Rollback:** revert this package's merge commit. It removes the three crates' content and restores
  `crates/application/`; no migration ran and no persisted state depends on anything added here.

## Verification (all run on the final content commit 6eff77d27bcc)

- `just verify` exits 0. It runs **8 recipes**: nextest ran 179 tests (179 passed, 0 skipped) and 7 doctests passed.
- `tests/guards/run.sh` carries 78 committed behaviour cases.
- `pwsh scripts/verify.ps1 -Scope Full` exits 0 under PowerShell 7.4.6 on Linux. That is not
  the Windows merge-authoritative run: the `windows-authoritative` CI job on this PR is.
- `justfile`, `verify.sh` and `verify.ps1` agree at **15 steps**. `verify.ps1` guards every native
  command with `Assert-Ok` (15/15).
- `Cargo.lock` resolves 173 crates (171 before, plus `grid-cli` and `grid-synth`). A fresh clone compiles
  offline (`env -u DATABASE_URL SQLX_OFFLINE=true cargo check --workspace`).
- Traceability: all 66 non-trivial paths traced against `origin/main` (`73a8e3b`).
- `engine-spec.md` and its mirror are byte-identical.
- Reference oracle: `verify_manifest.py` OK (102 verbatim, 3 patched); no oracle file changed. The
  `reference-oracle` CI job is the Linux evidence on this PR.
- No placeholder in `crates/` (`git grep -n -E 'TODO|FIXME|unimplemented!|todo!' -- crates` prints
  nothing); `.claude/settings.json`, `justfile`, `scripts/verify.*`, `.github/`, `migrations/`, `.sqlx/`
  and `reference/python/` are byte-for-byte unchanged since `73a8e3b`.
- Review: a security-dependency review (no blocker; 3 major and 6 minor findings) and an adversarial PR review (CONDITIONAL; 2 blockers, 11 major and 12 minor findings), both fresh-context and read-only at `07ecd6a`. Every finding was fixed or raised as an Amendments row; the commit-trailer finding is a known limitation.

## Completion record (Appendix C)

```text
Work package:                    P1-01 (docs/01-work-packages/p1-01-engine-skeleton.md)
Final commit:                    6eff77d27bcc  (the commit the manifest attests to; the evidence
                                 commit carrying it follows and is not self-covered)
Model/harness identifier:        unrecorded (see Known limitations); harness Claude Code on the web
Environment:                     Linux 6.18.44-fc-v139 x86_64 (Claude Code on the web container); rustc 1.99.1
Authority documents read:        engine-spec.md §1.1, §1.5, §1.7, §2.2, §4.4.1, §4.5, §5.1-§5.5, §6.7,
                                 §6.8, §7.2, §7.12, §8.1-§8.4, §8.6.4, §8.9, §8.10, §8.14, §8.16-§8.20,
                                 §9.2.1, §9.5, §12.6, §14.1, §14.2, Appendices B-D; ADR-001, 006, 010,
                                 011, 012; the three contracts; evaluation-and-leakage.md §4.1,
                                 synthetic-world.md §5.3, §10; the decision register and known issues
Contracts changed:               grid.plays v1 §11 (names); grid.output 1.0 §2, §3.9, §4.2, §10, §13;
                                 grid.parity-fixture §5.1, §11 (all Draft)
Migrations changed:              none
Dependencies changed:            none new; serde (grid-domain, grid-pipeline) and serde_json
                                 (grid-pipeline with float_roundtrip, grid-synth) first used;
                                 Cargo.lock +2 members
Targeted tests:                  nextest per crate in the canonical run: grid-domain 62, grid-synth 51, grid-pipeline 47,
                                 grid-cli 16, grid-persistence 3; doctests grid-domain 3, grid-synth 4
Canonical verification command:  just verify (Linux smoke); verify.ps1 -Scope Full is merge-authoritative
Verification exit status:        0 (just verify); 0 (verify.ps1 under pwsh on Linux)
Golden files changed and approval: none
Performance evidence:            n/a (no engine computation)
Security/licensing review:       check-secrets OK; no new host, secret or dependency; agent definitions
                                 grant no permission (Security/Release owner review requested)
Fresh-context reviewer:          independent read-only agents, none the implementer: a verifier per crate
                                 during implementation, then a security-dependency review and an
                                 adversarial PR review of the whole diff at 07ecd6a
Reviewer findings resolved:      security F1-F8 fixed (F9, commit trailers, recorded); adversarial B1, B2,
                                 M1-M11 and m1-m12 applied or raised as Amendments rows 13 to 24
Human approvals:                 none yet: ADR-014 and this package at the merge (Product/Architecture
                                 owner); agent definitions (Security/Release owner)
Known limitations:               see below
Evidence manifest hash:          sha256:86ed7273027bd6b313947d2f82528f9e34eedd5559e1f3f3726ec82ac237af6a
Oracle parity result:            no parity asserted; S-1 prepared (loader + hand-built round trip)
Reference-oracle job result:     the CI job on this PR
Owner decisions depended on:     DR-A2, DR-A8, DR-A11, DR-B1, DR-B2, DR-B3 (as overridden), DR-C4,
                                 DR-D7, DR-D8 (interim rule), DR-D29: settled (register, 2026-10-17).
                                 DR-D10 is open and touched only by the MarketUnits enum, which fixes
                                 no spread-to-EP mapping; it gates P1-07 and P1-12, not this package
```

## Known limitations

- **`ai-toolchain.lock` provenance is "unrecorded"**, as in P0-01 and P0-02: the harness policy forbids
  writing model identifiers into repository artifacts (deviates from engine-spec §8.20).
- **The branch is harness-assigned** (`claude/pensive-fermat-fgaweh`), not `wp/P1-01-…`.
- **Windows is proven only by this PR's CI.**
- **Agent-only review.** Your merge is the first human review.
- **The CI `guards` job was red on the intermediate pushes** for one reason only: until this evidence
  record landed, `check-evidence-claims.sh` re-derived P0-02's record against this tree.
- **`cargo test -p grid-cli --doc` exits 101** ("no library targets"): a binary-only crate has no doctest
  target; the canonical `cargo test --workspace --doc` is unaffected.
- **Open items recorded for later packages:** whether a numerical crate's parity tests may take
  `grid-synth` as a dev-dependency (engine-spec §8.1 rule 3 says `models` MUST NOT depend on `synth`,
  unqualified; P1-06 or P1-12 raises it); the §7 Status report row of `grid.output` versus the per-source
  results engine-spec §8.3 query 6 requires (P1-10 completes the contract); the §9.5/§7.12.3 wording on
  fixture timing (a spec erratum).

## Before merging

1. Rule on ADR-014 (accept or reject) and on the Amendments rows of the work package.
2. Review `.claude/agents/` and `.claude/skills/` as the Security/Release owner.
3. Merge with a merge commit.
4. Still yours after that, unchanged by this PR: make `reference-oracle (Linux only)` a required status
   check on `main` (DR-D30); the DR-A3 follow-up ADR before P1-11; the KI-NEW-Z79 decision request; the
   P1-12 pre-registrations.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01B716jmkXYqHUXNNYPmygvk
