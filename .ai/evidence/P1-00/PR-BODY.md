# P1-00 — Agent-Ready Repository, Authority Index, CI, Verification Scripts, Templates

Implements [`docs/01-work-packages/p1-00-work-package.md`](docs/01-work-packages/p1-00-work-package.md).

## Outcome

The repository is self-describing: a fresh session can enter it, discover authority, and extend
it without prior chat context. `just verify` exits 0 across **9 recipes** — from this tree and
from a clean checkout with no `target/`.

**Non-goals honoured:** no ingestion, models, features, or simulation; no Flutter code beyond
`pubspec.yaml`; no provider fixtures; no branch-protection changes; no `cargo-deny` policy
finalization; no `rust-toolchain.toml` channel change.

## The starting state did not match the work package's preconditions

Read-only exploration found that **no `.rs`, `.dart`, or `.sql` file had ever existed in this
repository**. `crates/` was absent, so all 12 workspace members were dangling and every cargo
command failed at manifest-parse time. Four of five templates were 0 bytes; the fifth was
truncated and backslash-corrupted. `docs/CLAUDE.md` and the authority index both inverted
alpha-spec §1.5. `LICENSE` was GPL-3.0 while `Cargo.toml` declared `MIT OR Apache-2.0`.

## Obstacles escalated rather than worked around

Each became an ADR with an owner ruling on the record.

| ADR | Found |
|---|---|
| **002** | `check-sqlx` and `test-rust` cannot pass against an empty workspace (`cargo nextest` exits **4** on zero tests) → minimal SQLx toolchain moved P1-02 → P1-00 |
| **002** | Environment presets a stale `DATABASE_URL` from the predecessor `nfl-alpha` name; a committed `.env` cannot override it |
| **003** | `cargo audit` red on RUSTSEC-2023-0071 (`rsa`, no fix available) via `sqlx-mysql`; feature narrowing tested and ineffective → sqlx 0.8 → 0.9 |
| **004** | Every file CRLF, leaving `scripts/verify.sh` **unparsable by bash** — the §8.11 canonical Linux command could not run |
| **005** | `check-sqlx` unsatisfiable: sqlx-cli 0.9 refuses a virtual workspace root without `--workspace` |
| **006** | §8.7/§9.2 deliverables absent without a recorded deviation |
| **007** | §8.11's secret scanning, traceability and doctests absent from the canonical command |
| **008** | `scripts/verify.sh` accepted any scope and exited 0 having run nothing |
| **009** | `scripts/verify.ps1` — the §8.11 merge gate — discarded every command's exit code |
| **010** | Three of four blockers were second instances of a defect already fixed in the same file → where a fail-closed guard belongs |

## Architecture, contract, schema, model, FFI impact

- **FFI:** none. No FFI surface exists; `crates/ffi/src/generated/` is an empty placeholder. `grid-ffi` gains only `[features] flutter-bridge-tests = []`, named by the frozen `test-ffi` recipe.
- **Public Rust API:** two items, both in `grid-persistence` — `schema_meta_value()` and `applied_schema_version()`. (An earlier revision of this body named `SCHEMA_VERSION_KEY`, deleted in `a37e49f`.)
- **Schema:** one infrastructure migration, `0001_schema_meta.sql`. Not a domain table; all domain schema remains P1-02, which is pre-approved to retire it by forward migration (ADR-002).
- **Version impact: none.** Workspace stays `0.1.0-alpha.1` — nothing has been published.
- **Model/statistical semantics:** untouched.

## Verification recipes are frozen (ADR-001 D5) — full audit

`verify` runs **9 recipes**. Across the branch, **three command lines changed**, each under an
owner ruling recorded in its own ADR:

```diff
- cargo sqlx prepare --check -- --lib
+ cargo sqlx prepare --check --workspace -- --lib     # ADR-005
+ cargo test --workspace --doc                        # ADR-007
+ just check-guards                                   # ADR-007
```

ADR-008 (scope validation in `verify.sh`) and ADR-009 (exit-code propagation in `verify.ps1`)
**change no command at all** — both fix the harness *around* the checks. `check-verify-parity.sh`
confirms the justfile, `verify.sh` **and `verify.ps1`** agree at **16 steps**, order-sensitively
(three-way since round 3; it read two of the three before).

**D5 has now been amended four times.** All four *increased* coverage; none relaxed a check. Worth
naming the pattern rather than just the count: **two of the four — 008 and 009 — were defects in
the harness, not the checks.** An entry point that accepted any scope and ran nothing, and a merge
gate that threw away every exit code. D5 froze the check list and left the code deciding whether a
check counts unexamined; both fail-opens survived multiple green CI runs and two adversarial
reviews. An amendment that *reduces* coverage should still be refused outright.

**Round 3 needed no fifth amendment**, and that is the same pattern seen from the other side:
its blocker and three majors all landed in guard scripts, the guard suite, or CI — none of which
D5 freezes. D5 protects *what is checked*; every recurring defect has been in *whether the
checking code can see*.

## Migration and rollback

`migrations/0001_schema_meta.sql` creates one table. Forward path `sqlx migrate run`, tested
against a blank database. Rollback: revert the branch — the database lives in gitignored
`target/`. The directory is append-only from here, enforced by `check-migrations.sh`.

## Tests added and why

| Test | Why |
|------|-----|
| `migrations_apply_to_a_blank_database` | Proves the migration toolchain end-to-end (FBS §8.2: "use sqlx migrate from day one") |
| `absent_key_is_none_not_an_error` | A missing key is `Ok(None)`, never conflated with failure (§12.7 forbids silent-default conversion) |
| `schema_meta_round_trips` | The applied version is derived from the migrator, not a hand-written literal that would go stale at `0002` |

Three tests — the first in the repository, and the reason `cargo nextest run --workspace` can
pass at all. **`tests/guards/run.sh` carries 54 committed behaviour cases** covering all six
guard scripts and the `verify.sh` scope guard, run inside `just verify` (ADR-007).

That suite has now caught seven defects that reading did not — including the three most serious
in the branch, all found after the second review had finished. `check-secrets.sh` needed two
fixes: it first ignored untracked files, then — found by the first adversarial review — **failed
open** on an invalid git expression, reporting OK over a committed secret. Writing the parity
cases for this round exposed that two silently-empty extractions would have compared equal, and
that the script did not even fail cleanly when they did. And an over-broad ancestor match in the
traceability fix was caught by the suite on its first run.

## Data, licensing, security

- **License corrected** to `MIT OR Apache-2.0` (ADR-001 D4); texts copied verbatim from canonical sources, not reproduced from memory. See the licensing note under the review section below.
- **Security fix:** RUSTSEC-2023-0071 removed from the graph (ADR-003). Verified: `Cargo.lock` resolves 172 crates and contains zero `rsa` entries.
- **Build tooling is pinned** in `toolchains/dev-tools.lock` and in both installers; both CI jobs echo their resolved versions. `--locked` pins a tool's dependencies, only `--version` pins the tool.
- **`.claude/settings.json`** adds a least-privilege profile per §14.1 — **requires Security/Release owner approval.**
- **No secrets committed.** `.env` holds a credential-free relative SQLite path plus `SQLX_OFFLINE=true`, which is what lets a fresh clone compile against the committed cache. That property is now a CI regression gate on **both** platforms.
- **No new dependencies.** `sqlx` is a version bump of an existing declaration; no crate was added.
- CI uses only `actions/checkout` — no third-party actions in a security-boundary file. `cargo audit`'s advisory-database fetch is a recorded, accepted network dependency (ADR-002).

---

## Completion record (Appendix C)

```text
Work package:                    P1-00
Final commit:                    8d4320304f0c  (the commit the manifest attests to; the
                                 evidence commit carrying it follows and is not self-covered --
                                 a manifest cannot record its own SHA. Checked mechanically by
                                 scripts/check-evidence-claims.sh, not asserted.)
Model/harness identifier:        claude-opus-5 (project alias; provider id and harness version
                                 read from ai-toolchain.lock, per alpha-spec 1.3)
Environment:                     grid-alpha-opus5 (Linux 6.18.5; rustc 1.94.1; cargo 1.94.1)
Authority documents read:        final-build-spec.md 2,3,7,8,19,21,22,23
                                 alpha-spec.md 1.1-1.6, 4.6, 8.1, 8.7-8.12, 9.2, 9.4, 9.5,
                                   12.7-12.9, 14, 17.1, 17.3, Appendices B/C/D/E
                                 docs/CLAUDE.md; docs/00-meta/authority-index.md
                                 docs/01-work-packages/p1-00-work-package.md
                                 docs/adr/ (empty at authority-load time -- no ADRs existed;
                                   the ten in docs/02-adr/ were written by this package)
Contracts changed:               None. No FFI DTO, no provider schema, no model spec.
                                 New public API: grid-persistence::{schema_meta_value,
                                 applied_schema_version}. Workspace version unchanged
                                 (0.1.0-alpha.1).
Migrations changed:              migrations/0001_schema_meta.sql (added; append-only)
Dependencies changed:            sqlx 0.8 -> 0.9 (ADR-003, security). No crate added or removed.
                                 Dev tooling pinned in toolchains/dev-tools.lock (not a
                                 production dependency).
Targeted tests:                  cargo nextest run --workspace  -> 3 tests run, 3 passed, 0 skipped
                                 cargo test --workspace --doc   -> 12 crate targets, 0 doctests
                                 cargo test -p grid-ffi --features flutter-bridge-tests -> 0 tests, ok
                                 ./tests/guards/run.sh          -> 54 passed, 0 failed
Canonical verification command:  just verify
Verification exit status:        0, run against 8d4320304f0c -- the commit this record and the
                                 manifest both name.
Golden files changed:            N/A -- no golden files exist (P1-06)
Performance evidence:            N/A -- no performance target is affected (no code to measure)
Security/licensing review:       RUSTSEC-2023-0071 remediated (ADR-003); Cargo.lock resolves 172
                                 crates with zero rsa entries. 14.2's maintenance assessment is
                                 recorded there. 14.2's WINDOWS COMPATIBILITY CHECK IS
                                 ESTABLISHED: run 32415391750 on 6b7f3fd, where verify.ps1
                                 -Scope Full passed on windows-latest in the SAME job as the
                                 ADR-009 self-test proving that script can fail. The earlier
                                 citation (run 32329932430) stays withdrawn and is not the
                                 basis for this one. License is MIT OR Apache-2.0
                                 (ADR-001 D4), recorded by the owner on this PR.
                                 .claude/settings.json least-privilege profile AWAITS
                                 Security/Release owner approval.
Fresh-context reviewer:          DONE, THREE TIMES - independent adversarial agent, fresh
                                 context, each round reproducing findings by execution.
                                 Round 1 at 7787921 (PR #2): CONDITIONAL, 6 blockers, 10 major,
                                 9 minor. Round 2 at de3b36c (PR #3, comment on this PR):
                                 CONDITIONAL, 3 blockers, 9 major, 11 minor.
                                 Round 3 at 8e07b04: CONDITIONAL, 1 blocker, 3 major, 4 minor.
                                 Round 3 confirmed round 2's blockers and majors all closed or
                                 owner-recorded, and corrected its own round-2 C3 supporting
                                 fact. Its blocker was a THIRD instance of a shape already fixed
                                 twice in the same file; ADR-010 records what changed about the
                                 method, not just the code.
Reviewer findings resolved:      Round 1: 6/6 blockers, 9/10 major, 8/9 minor.
                                 Round 2: all 3 blockers closed (C3 by the owner's own record on
                                 this PR); 9/9 major closed or owner-recorded; 10/11 minor fixed
                                 or recorded, one deliberately not changed with reasoning.
                                 Round 3: 1/1 blocker, 3/3 major, 3/4 minor fixed; minor-4
                                 answered with reasoning rather than changed.
                                 SELF-FOUND after round 2, all fail-opens, all fixed:
                                   verify.ps1 discarded every exit code (ADR-009)
                                   check-secrets scanned zero files on Windows
                                   the ADR-009 fix was itself unproven (all-green run)
                                 Every finding acted on was reproduced first. The reviewers'
                                 questions are answered in the manifest under
                                 review.round_2_disposition and review.round_3_disposition.
Human approvals:                 The manifest's 8.12-mandated human_approvals.required /
                                 .obtained fields are populated, not empty.
                                 DECISION RULINGS OBTAINED - answers the owner gave to explicit
                                 questions naming the alternatives:
                                   D1 branch naming; D2 remediate skeleton inside P1-00;
                                   D3 numbered vault; D4 MIT OR Apache-2.0; D5 frozen recipes;
                                   D6 environment fix; D7 verify.sh scope guard (ADR-008);
                                   D8/D9 owner records the licence and scope deviations here;
                                   D10 verify.ps1 exit-code checks (ADR-009);
                                   D11 withdraw and re-earn the Windows evidence;
                                   ADR-003; ADR-004; ADR-005; ADR-007.
                                 OWNER-AUTHORED RECORD ON THIS PR - the permissive licence
                                 (2565acc), the SQLx bootstrap moving P1-02 -> P1-00, and the
                                 sqlx 0.8 -> 0.9 bump as a consequence. This closes C3, M6 and
                                 M7, which needed an artifact implementer prose cannot supply.
                                 ROLE-SCOPED SIGN-OFFS OBTAINED - none. Still required:
                                   Architecture owner  authority order, crate boundaries
                                   Security/Release    .claude/settings.json
                                   Merge reviewer      CI skeleton and the final diff
Known limitations:               (1) Role-scoped sign-offs: NONE obtained.
                                 (2) RESOLVED. verify.ps1 -Scope Full is ESTABLISHED as passing
                                     under a gate proven capable of failing -- run 32415391750,
                                     the ADR-009 self-test and the authoritative verification both
                                     green in one job on 6b7f3fd. Withheld through three earlier
                                     greens because the two halves sat in different runs of
                                     different commits. CI is authoritative on the true final
                                     commit (12.8).
                                 (3) THREE fail-opens were found by the implementer AFTER round 2,
                                     all invisible to CI's own conclusion and to both reviewers:
                                     the Windows gate discarded exit codes, the secret scanner
                                     examined zero files on Windows, and the fix for the first was
                                     itself unproven. All fixed and regression-tested. ADR-001 D5
                                     records the lesson: freezing a check list is not the same as
                                     trusting it.
                                 (4) RESOLVED in round 3 (M2). check-verify-parity.sh now reads
                                     all THREE frozen implementations. It still compares WHICH
                                     commands each runs, not whether each exit code is checked --
                                     which is why it could not have caught ADR-009's defect, and
                                     why the Assert-Ok coverage analysis is a separate control.
                                 (5) An evidence manifest cannot record its own commit SHA.
                                 (6) Round 1's M10 (Flutter/Dart analysis) and app/pubspec.lock
                                     are NOT done. pubspec.lock is BLOCKED not deferred: Flutter
                                     is not installed here; a hand-written lockfile would be a
                                     fabrication.
                                 (7) just verify is not offline -- cargo audit fetches the
                                     RustSec database. Accepted, recorded in ADR-002.
                                 (8) -Scope Changed still unimplemented in both verify scripts
                                     though 8.11 cites it as canonical. Deferred to P1-11.
                                 (9) justfile, verify.sh and verify.ps1 remain three duplicate
                                     implementations; unification barred by D5, drift guarded
                                     three-way by the parity check since round 3.
                                (10) Cargo.toml, ai-toolchain.lock, rust-toolchain.toml and
                                     justfile retain CRLF. ADR-004 counts the workarounds and
                                     sets a threshold for escalating out of P1-11.
                                (11) No fixtures. 9.5 lists them under P1-00; deferred to
                                     P1-03/P1-04 and recorded in ADR-001.
                                (12) CI caching deferred to P1-11; an uncached ~25-minute Windows
                                     job that any push cancels is hard to observe passing.
                                (13) The stale grid-alpha-opus5 environment is still uncorrected;
                                     .env supplies correct values where it does not override.
                                (14) check-evidence-claims.sh re-derives NUMBERS from artifacts.
                                     It cannot tell that a sentence has gone stale; two known
                                     limitations and two narrative lines were falsified by round
                                     3's own fixes while every numeric claim still matched.
                                     Prose remains a manual sweep. Stated rather than implied,
                                     because claiming otherwise would be the fail-open shape
                                     ADR-010 is about.
Evidence manifest hash:          sha256:0e9545c224cc43e0eb58e98e22d90dcbb99884a2924267863aa48b77d62c9826
                                 (.ai/evidence/P1-00/manifest.json)
```

## Independent adversarial review — round 3, at `8e07b04`

**CONDITIONAL: 1 blocker, 3 major, 4 minor.** Round 2's 3 blockers and 9 majors were re-checked
by execution and all confirmed closed or owner-recorded. The reviewer also corrected their own
round-2 C3 supporting fact after verifying the commit ordering. Full dispositions are in the
manifest under `review.round_3_disposition`.

**The finding that matters is about method, not code.** Three of the last four blockers were
second instances of a defect already fixed elsewhere in the same file:

> when a defect is found, the fix is being applied to the instance rather than swept for across
> the file, and no mechanism enforces the sweep.

That is right, and the blocker proves it — **and it was worse than the review found.** C1 was
reported as a diff-mode gap in `check-secrets.sh`. Reproduced here, **both** arms fail open: in
diff mode nothing checked the untracked scan, and in full-tree mode the round-2 call-site guard
runs several lines *before* the untracked scan, so it only ever counted the tracked pass.

```
diff mode,      scan_files broken            -> exit 0, ghp_ token untouched
full-tree mode, untracked scan broken alone  -> exit 0, same
```

So "give the diff arm the guard the full-tree arm has" was the wrong fix — a third patch to an
instance. **The guard moved inside `scan_files`,** which now counts what it was handed, what it
deliberately skipped, and what it opened, and refuses to return having opened nothing it cannot
account for. Every caller is covered, including ones not yet written. `docs/02-adr/010` records
the placement rule.

| | Finding | Disposition |
|---|---|---|
| **C1** | `check-secrets.sh` fails open over untracked files | Fixed in **both** arms; guard moved into the function, redundant call-site guard removed. An all-binary set still passes, so the guard discriminates rather than refusing whenever it opened nothing |
| **M1** | the `Assert-Ok` coverage analysis misses 6 of 7 command forms | Fixed. Rule inverted: a line is a command **unless** it is a recognised construct. Controls for all seven forms and twelve constructs |
| **M2** | `verify.ps1` parity-checked against nothing, while being the merge gate | Fixed. Three-way parity. Verified "latent, not drifted" independently first — 16 commands, same order, all three |
| **M3** | the self-test accepts any non-zero as proof, and covers `Assert-Ok` #1 of 16 | Fixed. Asserts on the specific message; a second run exercises the **last** assert and proves the other fifteen pass through on success |
| **min-1** | "All 65 non-trivial paths traced" — actually 66, and outside the claim pass | Fixed as a **mechanism**: `scripts/check-evidence-claims.sh` |
| **min-2** | no offline-compile step on `windows-authoritative` | Fixed |
| **min-3** | the column-0 coupling was a comment, not an assertion | Fixed, with a control |
| **min-4** | the manifest has never covered the head | Answered, deliberately not "fixed" — see below |

**No fifth D5 amendment.** Every fix landed in a guard script, the guard suite, or CI — none of
which D5 freezes. Parity still reports **16 steps**. That distinction is itself the pattern
ADR-001 D5 names: D5 protects *what is checked*; the recurring defects have all been in *whether
the checking code can see*.

### Two things I got wrong on the way, since they shaped the fixes

**My first reproduction of M1 was invalid.** Inserting the unguarded command directly after
`cargo fmt` displaced *that* command's `Assert-Ok`, so the analysis flagged `cargo fmt` and all
seven rows read CAUGHT — for the wrong reason. Inserting after a complete pair reproduces the
reviewer's table exactly.

**My first fix for M1 used `\b` as a word boundary.** In awk `\b` is a backspace, not a word
boundary (gawk spells it `\y`; this runner is mawk 1.3.4), so the construct list matched nothing
and `param(` and `if (...)` became false positives. Caught because the baseline moved from
`16 guarded, 0 unguarded` to `16 guarded, 3 unguarded`.

### What is proven where

`verify.ps1` cannot be executed here — there is no PowerShell on this runner — so **M3's fix is
proven by CI on `windows-latest`, not locally.** What was verified locally is that the two
expected substrings match the real `Assert-Ok` labels and the throw format string, and the guard
suite now asserts that coupling so a rename fails in milliseconds on Linux rather than 25
minutes into the Windows job.

### min-4 — answered, not fixed

Generating the manifest in CI against the pushed head would make CI the author of its own
evidence. The one-commit lag is structural, not a workflow defect. What *was* fixable is that
the lag went unmeasured: `check-evidence-claims.sh` verifies the content claims independently of
which commit they were written at, and deliberately **does not** assert `manifest.commit == HEAD`,
because that would encode a falsehood as a rule.

### What the claim checker still cannot see

Its first run caught a stale `Final commit` reference the numeric claims had passed straight
over, and a ninth claim now covers that. But four sentences in this record were falsified by
round 3's own fixes while every number stayed correct — two known limitations and two narrative
lines, all found by reading rather than by the tool. **Prose staleness is still a manual sweep**,
and limitation (14) says so rather than leaving a reader to assume the record is self-verifying.

## Independent adversarial review — round 2, at `de3b36c`

**CONDITIONAL: 3 blockers, 9 major, 11 minor.** Every finding acted on was reproduced against
the working tree first. Full dispositions, including answers to all eight reviewer questions,
are in the evidence manifest under `review.round_2_disposition`, per §8.12.

The through-line was that this branch's green meant less than it looked: gates that did not
gate, and a record that had drifted from the code.

| | Finding | Disposition |
|---|---|---|
| **C1** | `verify.sh Full` — the capitalization every authority doc prints — ran **zero checks and exited 0**. So did `verify.sh smoke` | Fixed. Scope normalized and validated; ADR-008, third D5 amendment, no command touched. 9 regression cases |
| **C2** | Both `CLAUDE.md` files said `SQLX_OFFLINE` unset, contradicting ADR-002, and claimed an enforcement the guard does not perform | Fixed in **three** files — `check-env-contract.sh` carried the same inversion in its own advice. The guard now resolves `SQLX_OFFLINE` the way `query!` does |
| **C3** | Relicensing is irreversible, out of scope, and rests only on implementer prose | **The review's supporting fact is wrong**; see below. The irreversibility objection stands and the owner records the ruling on this PR (D8) |
| **M1** | The §8.11 merge gate's green **excluded** append-only and traceability — both SKIPped at `fetch-depth: 1` and exited 0 | Fixed. `fetch-depth: 0` on both build jobs; both SKIP paths now **fail** under `$CI` while still skipping in a local shallow checkout |
| **M2** | Six checkably-false statements; the canonical run predated HEAD by three commits | Fixed. Every number in this body re-derived from a command run against the attested head — mechanically, since round 3 |
| **M5** | The offline-compile property — the entire reason `.sqlx` is committed — was **never exercised by CI** | Fixed. Offline `cargo check` gate added, proven to pass on a forced macro re-expansion and to fail with `.sqlx` absent. On both platforms since round 3 |
| **M4** | Tooling unpinned in a repo whose thesis is pinned reproducibility; `deny.toml`'s own claim depended on it | Fixed. `toolchains/dev-tools.lock`; both jobs echo resolved versions. Verified `cargo-deny 0.20.2` honours the explicit keys with no warning |
| **M8** | Parity and env-contract had no tests — and parity is load-bearing because D5 forbids unifying the two implementations | Fixed, and the worry was **understated**: two empty extractions compared equal, and the script did not even fail cleanly. Suite 12 → **42** cases in round 2, **54** after round 3 |
| **M9** | The permission profile had drifted from the justfile it enumerates | Fixed. Prefix forms that cannot fall behind |
| **M3** | The §8.12-mandated `human_approvals.required` said no approvals are required | Fixed. Both mandated fields populated |

### The licensing fact, corrected

The review states the `MIT OR Apache-2.0` line "arrived in the skeleton commit and is the value
a `cargo new`-style scaffold produces", concluding the conflict "was resolved in favour of the
scaffold-shaped value and against the human's explicit act."

**Both sides are the owner's own commits, and the permissive one is the later:**

| Commit | Author | Time | Act |
|---|---|---|---|
| `f71a241` | Ethan Nelson | 2026-08-19 16:05:08 | adds `LICENSE`, GPL-3.0 |
| `2565acc` | Ethan Nelson | 2026-08-19 16:35:13 | adds `Cargo.toml` declaring `license = "MIT OR Apache-2.0"` |

`git merge-base --is-ancestor f71a241 2565acc` confirms the order. `main` shipped a
self-contradiction between two commits by one person thirty minutes apart; D4 resolved it toward
the more recent. ADR-001 D4 now records this. **Round 3 verified this correction independently
and withdrew its own round-2 supporting fact.**

**The review's actual objection survives the correction and is not dismissed.** Relicensing is
the one change here that cannot be undone in the real world, §1.5 names licence uncertainty as a
stop condition, and a change of that class should rest on an artifact the Data/Licensing owner
authored. Per ruling D8, D4 stays on the branch and **the owner records the ruling on this PR.**

### Referred to the owner rather than fixed

**M6** (the work-package non-goal amended inside the PR it constrains) and **M7** (the sqlx
semver bump folded into a bootstrap PR against §14.2) are real and are not argued away. Both are
technically forced under D5 — `cargo nextest` exits 4 on an empty workspace and
`cargo sqlx prepare --check` cannot run against a bare virtual root, which the reviewer
independently confirmed — but necessity is not authorization, and ADR-001 already says a work
package must not be the artifact authorizing exceeding itself.

**The owner has now recorded both on this PR**, which closes them. What this branch could fix, it
did: ADR-003 carries §14.2's maintenance assessment, from evidence — 172 crates, zero `rsa`,
dependency footprint reduced, CLI now pinned.

**§14.2's Windows compatibility check was withdrawn, then re-established from a different run.**
An earlier revision of this body and of ADR-003 cited `verify.ps1 -Scope Full` PASSING on
`windows-latest` as that evidence. ADR-009 shows the script could not fail, so that citation
proved execution, not passing — and it was written by the same implementer who then found the
defect. It was re-recorded only once a run of the *fixed* script was green alongside a self-test
proving the gate can fail: run `32415391750` on `6b7f3fd`. ADR-003 keeps the withdrawal in place
rather than overwriting it with a clean assertion.

### Deliberately not changed

**minor 2** — `.cargo/config.toml [env]` is the more idiomatic committed way to set
`SQLX_OFFLINE` than a tracked `.env`, and the reviewer is right about that. It is also the exact
mechanism whose last change was a blocker, and swapping it inside the same PR that corrects the
documentation about it trades a real regression risk for a stylistic gain. Deferred to P1-11
with the reasoning recorded rather than churned now.

### What the reviewer checked and could not fault

Recorded so the next reviewer does not re-litigate it: the §1.5 authority-order correction,
stated identically in three places; crate boundaries verified per-crate from the manifests;
`.sqlx` cache integrity and the drift gate; the RUSTSEC remediation; the fail-closed rewrite of
`check-secrets.sh`; migration hygiene; and every CI claim verified against the GitHub API.

## Found after the review finished — three fail-opens, all mine

Neither reviewer could have found these. CI reported green.

They surfaced only because round 2's fixes — the `$CI` fail-closed change and 23 new guard
cases — made `windows-authoritative` print a real verdict for the first time. Run
`32376218798` then showed, twelve lines apart:

```
33 passed, 2 failed          <- tests/guards/run.sh, exited non-zero
[verify] All checks passed.  <- exit 0, job reported SUCCESS
```

**`scripts/verify.ps1` could not fail.** Sixteen native commands, zero `$LASTEXITCODE` checks.
`$ErrorActionPreference = "Stop"` governs PowerShell error records, not native exit codes, and
`#Requires -Version 7.2` pins the script to a version where
`$PSNativeCommandUseErrorActionPreference` is off by default. Every exit code was discarded —
`cargo fmt --check`, `clippy -D warnings`, `sqlx prepare --check`, `nextest`, `deny`, `audit`,
all seven guards, `typos`. **ADR-009**, fourth D5 amendment.

**`check-secrets.sh` examined zero files on Windows.** Both `git ls-files` loops opened nothing,
so the guard printed `OK no secret patterns found` over a committed `ghp_` token and a
credentialed connection string. The five cases expecting exit 0 all passed *vacuously*. The first
defect hid the second: the suite caught it and `verify.ps1` threw the verdict away.

**And the fix for the first was itself unproven** — see below. Three in total.

### What that cost, and what it took to get back

**Every `windows-authoritative` green before `a190e71` was withdrawn as evidence of passing** —
including the one this body and ADR-003 had cited as §14.2's Windows compatibility check. They
showed the steps *ran*. `verification_status` dropped to `passed_with_exceptions` and stayed
there for **8 commits and 5 CI runs**, through three greens that would each have been easy to
bank.

**It is re-earned as of run `32415391750`** on `6b7f3fd` — the first run where the ADR-009
self-test and the authoritative verification are green **in the same job on the same commit**.
Three earlier greens were declined because the two halves of the proof sat in different runs of
different commits, and combining them would have been the M2 defect wearing a different hat.

### And the fix itself had to be proven, not assumed

Run `32382377066` came back green with `38 passed, 0 failed` on Windows — the CR-strip fixed the
real mechanism. But **that green does not prove ADR-009 works**: every command passed, so no exit
code ever needed propagating, and an inert `Assert-Ok` produces identical output. ADR-002's own
argument — a gate that passes vacuously is a gate nobody has tested — applies to my fix too.

**Both are now closed, and visible in one CI run:**

| | Proves |
|---|---|
| **ADR-009 self-test** step on `windows-authoritative` | Runs the real `verify.ps1` with a stub `cargo` that exits 7 and requires the failure to name `[verify] FAILED: cargo fmt`. A second run stubs a *later* command so the last `Assert-Ok` is exercised too |
| **`verify.ps1` exit-code coverage** case in the guard suite | Every native command is immediately followed by an `Assert-Ok` (16/16), with a mutation control proving the check can fail. Nothing else could catch this: parity compares WHICH commands each implementation runs — three-way since round 3 — never whether their exit codes are handled |

The scanner's diff-mode branch also got the empty-scan guard its full-tree branch had, and both
verdicts now state what they measured: `7506 added line(s) from 82 changed file(s)` rather than a
bare `OK`. That number reading `0` is what a fail-open looks like.

**The pattern, recorded in ADR-001 D5.** Two of the four D5 amendments were defects in the
*harness around* the checks, not in the checks. Freezing a check list is not the same as
trusting it.

## Before merging

**Do not merge on the agent's authority** (§1.3, Appendix D). Outstanding:

- ✅ **Owner-authored records — DONE.** The owner recorded on this PR: the permissive licence
  per `2565acc`, the SQLx bootstrap moving P1-02 → P1-00, and the sqlx 0.8 → 0.9 bump as a
  consequence of that fold-in. That closes C3, M6 and M7 with the artifact two review rounds
  asked for and that no implementer-written ADR could substitute for.
- **Role-scoped sign-offs — none obtained.** Security/Release on `.claude/settings.json`;
  Architecture on the diff; Merge reviewer on the final diff.
- ✅ **`windows-authoritative` has been observed passing under a gate proven capable of failing
  — DONE.** Run `32415391750`, job `96575256117`: the **ADR-009 self-test** and **Authoritative
  verification** both green, same job, same commit. `verification_status` is back to `passed`,
  earned.
- ✅ **A third adversarial review — DONE**, at `8e07b04`: CONDITIONAL, 1 blocker, 3 major,
  4 minor, all closed above. Both questions I nominated turned out to have something in them:
  the `Assert-Ok` analysis missed six of seven command forms, and the diff-mode empty-scan guard
  could indeed be fooled. **A fourth round is worth having but is no longer load-bearing** — the
  trend across rounds is 6 → 3 → 1 blockers, and round 3 found no defect that CI now cannot see.
  The highest-value thing left to attack is `check-evidence-claims.sh` itself: it is new, it
  guards the record rather than the code, and nothing yet checks *it* for the blind-spot shape
  that ADR-010 is about.
- **Governance:** reconcile `docs/05-sessions/` vs `docs/06-sessions/` before PRs #2 and #3
  merge, and fix the reviewer brief that keeps sending reviewers to a path the authority index
  calls invalid.

Owner follow-ups: correct the `grid-alpha-opus5` environment per ADR-002, and configure branch
protection on `main` (an explicit P1-00 non-goal).

---
_Generated by [Claude Code](https://claude.ai/code)_
