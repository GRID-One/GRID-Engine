---
adr-id: 001
status: Accepted
date: 2026-08-20
deciders: Product/Architecture owner (D1-D3, D5, D6); Data/Licensing owner (D4)
supersedes:
superseded-by: "011 (in part, proposed): crate list incl. ffi; alpha-spec authority rows; D5 recipe count; Appendix B reference; P1-01…P1-11"
---

# ADR-001 — Repository bootstrap decisions

## Status
Accepted 2026-08-20 by the Product/Architecture owner, during P1-00.

## Context

P1-00 makes the repository self-describing so a fresh session can enter it, discover authority,
and extend it correctly with no prior chat context (`alpha-spec.md` §8.7, §9.2).

The starting state did not support that. Read-only exploration established:

- No `.rs`, `.dart`, or `.sql` file had **ever** existed in the repository, across all commits.
  `crates/` was absent, so all 12 workspace members in `Cargo.toml` were dangling and every
  cargo invocation failed at manifest-parse time.
- Four of five templates were 0 bytes; the fifth was truncated mid-code-fence and corrupted
  with literal backslash escapes from a Markdown export. `docs/00-meta/authority-index.md`
  carried the same corruption.
- `docs/CLAUDE.md` and the authority index both listed the alpha spec **above**
  `final-build-spec.md`, inverting `alpha-spec.md` §1.5.
- `LICENSE` was GNU GPL v3 while `Cargo.toml` declared `MIT OR Apache-2.0`.
- `just verify` could not pass for five independent pre-existing reasons.

Several of these contradict P1-00's own stated preconditions, which assert the files already
exist. The decisions below were taken to resolve that gap.

## Decision

### D1 — Branch naming
Work proceeds on `wp/P1-00-repo-bootstrap`, per the `wp/P1-XX-description` convention in
`docs/CLAUDE.md`. The cloud harness assigns a differently-named branch by default; the owner
explicitly authorized the convention-conforming name.

### D2 — Unmet preconditions are remediated inside P1-00
The missing crate skeleton, `.gitignore`, `.claude/settings.json`, `deny.toml`, `_typos.toml`
and `migrations/` are created here rather than deferred to a separate remediation package.
These were asserted as preconditions, so supplying them is remediation, not scope creep.

### D3 — Numbered Obsidian vault, not the flat `docs/` tree
`alpha-spec.md` §8.7 sketches a flat tree (`docs/adr/`, `docs/contracts/`, …). The repository
uses a numbered vault (`docs/00-meta/`, `docs/01-work-packages/`, `docs/02-adr/`,
`docs/04-providers/`, `docs/99-templates/`). §8.7's tree is treated as **illustrative of
required content, not normative on paths**. `docs/00-meta/authority-index.md` carries a
path-alias table so every name used in the specs resolves.

This is a deviation from a level-2 document, so it is ratified by the product owner directly —
an ADR alone could not authorize it (§1.5).

### D4 — `MIT OR Apache-2.0` is the project license
**Authority: Data/Licensing owner** (`alpha-spec.md` §1.4), not Product/Architecture. An earlier
revision of this ADR attributed it to the wrong role; corrected after adversarial review.

`LICENSE` (GPL-3.0) and `Cargo.toml` (`MIT OR Apache-2.0`) contradicted each other. The owner
ruled `Cargo.toml` correct. Replaced with the conventional Rust pair `LICENSE-MIT` +
`LICENSE-APACHE`, texts copied verbatim from canonical sources rather than reproduced from memory.

**Both sides of the contradiction were the owner's own commits, and the permissive one is the
later.** Established from `origin/main` after the second adversarial review, which had read the
permissive declaration as a `cargo new`-style scaffold artifact that beat a deliberate human act:

| Commit | Author | Time | Act |
|---|---|---|---|
| `f71a241` | Ethan Nelson | 2026-08-19 16:05:08 | adds `LICENSE`, GPL-3.0 |
| `2565acc` | Ethan Nelson | 2026-08-19 16:35:13 | adds `Cargo.toml` declaring `license = "MIT OR Apache-2.0"` (line 22) |

`git merge-base --is-ancestor f71a241 2565acc` confirms the order. `main` shipped a
self-contradiction between two commits by the same person thirty minutes apart; D4 resolved it
toward the more recent of the two. It is not a scaffold value displacing a human decision — it is
one owner decision superseding another, ratified by the owner when the conflict was put to them.

**The scope deviation is still real.** P1-00 lists neither licensing nor `LICENSE` in its Scope or
acceptance criteria. It was surfaced during the authority load as a contradiction that
`cargo deny` would gate on, put to the owner as an explicit choice, and made only on their ruling
— but it is recorded here rather than glossed.

**Relicensing is the one change in this branch that is not cleanly reversible in the real world.**
Once published permissively, third parties may rely on it, and re-imposing copyleft afterwards
needs every copyright holder's consent. §1.5 names licence uncertainty as an explicit stop
condition. That property does not depend on who was right about the intent, so the second review's
objection stands on its own even with the fact above corrected: a change of this class should rest
on an artifact the Data/Licensing owner authored, not on implementer prose in an ADR. The owner's
ruling is therefore recorded by the owner on PR #1. A reviewer who still considers licensing
outside P1-00 can revert D4 alone: it touches only `LICENSE*` and no other decision depends on it.

### D5 — Verification recipes are a frozen contract
The `justfile` recipes under `verify` — seven when this was ruled, nine since ADR-007 — and the
check bodies in `scripts/verify.sh` and `scripts/verify.ps1`, are **never edited to make a
failure disappear**. Where a recipe cannot pass, the repository changes. Where it cannot be made
to pass, work stops and escalates.

**Amended four times, each on an owner ruling recorded in its own ADR:** ADR-005 (`--workspace`
on `check-sqlx`), ADR-007 (`test-doc` and `check-guards` added to the chain), ADR-008 (scope
validation in `verify.sh`), ADR-009 (exit-code propagation in `verify.ps1`). All four *increased*
coverage; none relaxed a check. An amendment that reduces coverage should be refused outright
rather than weighed against these.

**A pattern worth naming.** Two of the four amendments — 008 and 009 — were defects in the
*harness around* the checks, not in the checks: an entry point that accepted any scope and ran
nothing, and a merge gate that discarded every exit code. D5 froze the list of checks and left
the code deciding whether a check counts unexamined, and both fail-opens survived multiple green
CI runs and two adversarial reviews. Freezing a check list is not the same as trusting it.

**Still four amendments after round 3.** The third adversarial review returned a blocker and
three majors, and none of them needed a fifth: every fix landed in a guard script, the guard
suite, or CI — none of which D5 freezes. That is the distinction the pattern above predicts.
D5 protects *what is checked*; the recurring defects have all been in *whether the checking
code can see*. ADR-010 records the placement rule that came out of round 3.

Two consequences accepted deliberately:

- `just verify` does **not** delegate to `scripts/verify.sh`, so the two implementations remain
  duplicated. `scripts/check-verify-parity.sh` asserts they stay in lockstep instead.
- `-Scope Changed` remains unimplemented in both scripts even though §8.11 cites it as
  canonical. Deferred to P1-11; recorded as a known limitation.

This ruling is what forced ADR-002, and it was vindicated during implementation:
`cargo nextest run --workspace` exits **4** ("no tests to run") on an empty workspace, so the
frozen `test-rust` recipe could not have passed on documentation alone either.

### D6 — Environment defects are fixed in the environment
The cloud environment presets stale values from a predecessor project name (`nfl-alpha`):
a `DATABASE_URL` pointing at a nonexistent path, `SQLX_OFFLINE=true`, a typo'd `PROJECT_ROOT`
(`guthub`), and a malformed `sqlx=warn,` variable. These are corrected in the environment
definition, **not** worked around in-repo. `scripts/check-env-contract.sh` asserts the contract
so the next session gets a named error instead of SQLite error code 14.

### Crate boundaries
Twelve crates, matching `Cargo.toml` members and the boundary list in `docs/CLAUDE.md`:
`domain`, `persistence`, `ingestion`, `identity`, `features`, `models`, `simulation`,
`scoring`, `evaluation`, `governance`, `application`, `ffi`. Each `lib.rs` states its boundary
in a doc comment so the code and the authority document cannot drift apart silently.

`alpha-spec.md` §8.1 nests these under `core/`; the flat `crates/` layout matches `Cargo.toml`
and is the Rust-conventional form. §8.7 permits the split to evolve provided ownership
boundaries stay clear.

### `just` over Make
`just` is already the committed interface (`justfile`, `ai-toolchain.lock`, both verify
scripts). It is cross-platform without a POSIX shell layer, which matters because Windows is
the production target and `verify.ps1` is authoritative for merge. Recipes are plain commands
rather than build rules, so Make's dependency graph buys nothing here.

### On amending a work package mid-implementation

Adversarial review (finding M5) objected that P1-00's non-goals were rewritten inside the same
PR, by the implementer, to permit work those non-goals forbade. The objection is well taken and
is recorded here rather than argued away.

The mechanism used was: stop at the boundary, put the conflict to the owner as an explicit
question naming the alternatives, and amend only on their ruling — which is what §8.9 step 4
and §1.5's "produce a decision request" prescribe. The amendment note in the work package is
dated and states what changed and why.

What is genuinely uncomfortable is that the implementer both raised the question and wrote the
amendment, and that a reader encountering the work package later sees the amended text as
though it were the original scope. Two mitigations are now in place: the amendment note is a
separate dated section rather than an edit to the original text, and this ADR records the
mechanism so the precedent is visible rather than tacit.

**A work package should not be the artifact that authorizes exceeding itself.** For P1-01
onward, a scope change of this size should produce a decision request the owner records
themselves, or a new work package — not an amendment note written by the implementer.

### Scope amendments to P1-00
Recorded in the work package as a dated amendment note:

| Original | Amended to |
|---|---|
| Non-goal: "No SQLx migrations beyond the empty `migrations/` directory" | "One infrastructure-only `schema_meta` migration. All **domain** schema remains P1-02." |
| Follow-up: "ADR-002 … to be resolved in P1-02" | "ADR-002 resolved in P1-00." |

P1-02's substantive scope is untouched — only the toolchain bootstrap moved.

### The work-package template is not byte-exact to Appendix B

The P1-00 acceptance criterion says the template "follows the exact Appendix B structure", and
`docs/99-templates/template-work-package.md` adds two things Appendix B does not contain: YAML
frontmatter, and a `## Security and licensing considerations` section. Both are deliberate and
both are traceable to a higher authority than the criterion:

- **Frontmatter** — `docs/00-meta/dashboard.md` runs a Dataview query over hyphenated keys
  (`work-package-id`, `status`, `risk-class`, …). Without them the dashboard cannot execute.
- **Security and licensing** — `alpha-spec.md` §8.8's field list names "security/licensing
  considerations" as a required work-package field; Appendix B's headings omit it. That is an
  inconsistency inside the specification, not a choice this project gets to duck: §1.5 puts
  the spec body and its appendix at the same level, and the safe reading of a conflict between
  two same-level requirements is to satisfy both.

Every Appendix B heading is present, verbatim and in order; the additions are additive.
Recorded here so a reviewer does not have to rediscover the discrepancy and re-litigate it
(second adversarial review, minor 10).

### Fixtures remain deferred
`alpha-spec.md` §9.5 lists "fixtures" in P1-00 and §9.2 wants them before dependent
implementation begins; the work package defers all fixtures to P1-03/P1-04. The spec outranks
the work package, so the deferral is recorded here explicitly rather than accepted silently:
§9.2's requirement is satisfied by P1-03 and P1-04 carrying their own fixtures **before** their
adapters are implemented, per the §4.6 rule that a provider contract and fixtures exist before
an adapter package is Ready.

## Consequences

**Easier.** A fresh session finds one authority order, stated identically in three places.
Verification is a contract rather than a negotiation. The SQLx toolchain is proven before
anything depends on it. Guard scripts make the invariants executable instead of advisory.

**Harder.** P1-00's diff is substantially larger than a documentation package. The numbered
vault means every spec reference to `docs/adr/` requires the alias table — a permanent small
tax, and a real risk if someone reads §8.7 without reading the index.

**Inherited.** P1-01…P1-11 inherit the authority order, crate boundaries, permission profile,
frozen-recipe rule, and SQLx pattern. An error here propagates to all of them.

## Compliance

| Decision | Enforced by |
|---|---|
| D1 | Branch name; PR template |
| D3 | Path-alias table; `scripts/check-traceability.sh` accepts `docs/02-adr/` |
| D4 | `cargo deny check` license policy in `deny.toml` |
| D5 | `scripts/check-verify-parity.sh`; `git diff` on `justfile`/`verify.*` in review |
| D6 | `scripts/check-env-contract.sh` |
| Authority order | Stated identically in `CLAUDE.md`, `docs/CLAUDE.md`, and the index |
| Authority integrity | `scripts/check-authority-sync.sh` — the two `alpha-spec.md` copies cannot fork |

## Alternatives considered

**Migrate to the flat §8.7 tree.** Literal spec compliance, but it breaks
`docs/00-meta/dashboard.md`'s Dataview query and contradicts the numbered paths the P1-00
acceptance criteria themselves specify. Rejected by the owner in favour of the alias table.

**A separate P1-00a remediation package for the skeleton gaps.** Strictly honours P1-00's own
stop condition and keeps this diff small, at the cost of two PRs and a blocked P1-01.
Rejected: the gaps were asserted as preconditions, so fixing them is remediation.

**Ship documentation only and report `just verify` as red.** Honest, but a foundational
package that ships a knowingly-red acceptance criterion normalizes red for every later
package. Rejected.

**Relax `check-sqlx`/`test-ffi` to no-op until P1-01/P1-02.** Rejected by the owner as
weakening the verification contract — the origin of D5.

**Keep GPL-3.0 and change `Cargo.toml`.** Viable, but GPL copyleft complicates redistributing
the Windows installer and interacts awkwardly with the vendored XGBoost artifacts
`final-build-spec.md` §3.2 anticipates. The owner ruled `MIT OR Apache-2.0` correct.
