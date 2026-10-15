---
adr-id: 004
status: Accepted
date: 2026-08-20
deciders: Product/Architecture owner
supersedes:
superseded-by:
---

# ADR-004 — Line-ending policy

## Status
Accepted 2026-08-20. Approved by the owner during P1-00 after the defect below was found and
escalated, because remediation touches a file frozen by ADR-001 D5.

## Context

Every file in the initial upload carries CRLF line endings — the `Add files via upload`
commit came from the GitHub web UI on Windows, and no `.gitattributes` existed to normalize
anything.

For most files this is cosmetic. For shell scripts it is fatal:

```
$ bash -n scripts/verify.sh
scripts/verify.sh: line 32: syntax error: unexpected end of file
$ bash scripts/verify.sh
scripts/verify.sh: line 2: set: pipefail: invalid option name
```

`scripts/verify.sh` is named by `alpha-spec.md` §8.11 as a canonical verification command and
documented in `docs/CLAUDE.md`. **It could not execute at all.** It was also mode `0644`, so
`./scripts/verify.sh` failed with "Permission denied" before bash ever saw the CRLF — two
independent defects masking each other. The P1-00 acceptance criteria require `test -x` on the
repository scripts, which the mode bit alone would have failed.

`just verify` was unaffected: `just` tolerates CRLF, which is why the breakage went unnoticed —
the working path hid the broken one.

Remediation collides with ADR-001 D5, which freezes `scripts/verify.sh`. The conflict is
resolved by reading D5 for its purpose: it protects the verification **contract** — which
checks run, with which flags — from being weakened to make a failure disappear. Line
terminators are not part of that contract, and a file that cannot be parsed enforces nothing.
Verified: stripping CR alone makes the file parse, and the resulting command list is
byte-identical to the committed one.

## Decision

**Shell scripts are LF. PowerShell is CRLF. A committed `.gitattributes` enforces both.**

1. `scripts/verify.sh`, `scripts/generate-evidence-manifest.sh`, and `scripts/bootstrap-repo.sh`
   converted CRLF → LF and set to mode `0755`.
2. `scripts/verify.ps1` stays CRLF — native and correct for PowerShell on the Windows target.
3. `.gitattributes` pins `*.sh` to `eol=lf` and `*.ps1` to `eol=crlf`, and marks `.sqlx/**`
   and `Cargo.lock` as generated so reviewers see them collapsed and do not hand-edit them.
4. **The policy is deliberately narrow.** A blanket `* text=auto` was drafted first and
   withdrawn: git immediately warned it would renormalize `Cargo.toml`, `docs/CLAUDE.md`, and
   the `justfile` on their next edit. Measured, that turned a **one-line** sqlx bump into a
   78-line whole-file rewrite and the three-block `docs/CLAUDE.md` edit into 174 lines. That
   destroys the reviewability ADR-001 D5 depends on — a reviewer cannot confirm the seven
   frozen verify recipes are untouched if every line of the `justfile` shows as changed.
   Repo-wide normalization is deferred to P1-11, where it can be a standalone diff that
   changes nothing but line endings.

## Consequences

**Easier.** The §8.11 canonical Linux command works. `test -x` passes. A Windows contributor
can no longer silently reintroduce the breakage, because `.gitattributes` normalizes on commit.

**Harder.** `git diff` against `origin/main` shows `scripts/verify.sh` as 30 changed lines even
though no command changed — reviewers must know to read it as a whitespace-only diff. The
proof is recorded here and in the PR body: `diff <(git show origin/main:scripts/verify.sh |
tr -d '\r') scripts/verify.sh` is empty.

**Residual.** `Cargo.toml`, `ai-toolchain.lock`, `rust-toolchain.toml`, and the `justfile`
retain CRLF in their committed bytes and are **not** covered by the policy. They parse
correctly, so there is no defect to fix, and excluding them keeps every diff in this package
minimal and reviewable. A mixed-line-ending repository is untidy; it is the lesser cost until
P1-11 normalizes it in isolation.

## Workaround count — track this

The narrow scope means CRLF survives on `Cargo.toml`, `justfile`, `ai-toolchain.lock` and
`rust-toolchain.toml`, and every tool that parses them has to cope. The deferral to P1-11 is
still right — a blanket `text=auto` would rewrite every file and destroy the review trail for
this branch — but the cost is not zero and is worth counting rather than asserting to be small
(second adversarial review, minor 11):

| Workaround | Location |
|---|---|
| `tr -d '\r'` before comparing justfile recipes | `scripts/check-verify-parity.sh` |
| `tr -d '\r'` on the extracted `verify` chain targets | `scripts/check-verify-parity.sh` |
| CRLF-preserving edits to CRLF documents | every tool that rewrites `docs/CLAUDE.md` et al. |

Three today, in two files. **If this table reaches five, or a workaround appears in Rust or
Dart source rather than in tooling, normalization stops being a P1-11 nicety and becomes its
own work package.** The next contributor to add a row should say so on the PR.

## Compliance

- `.gitattributes` — enforced by git on checkout and commit.
- `bash -n` on every shell script, run in CI before `just verify`.
- The `test -x scripts/*.sh` assertions in the P1-00 verification block.

## Alternatives considered

**`chmod +x` only, leave CRLF.** Satisfies `test -x` while leaving the script unrunnable —
a green check over a broken command, which is precisely the failure mode `alpha-spec.md` §12.7
exists to prevent. Rejected.

**Leave both defects and report as blocked.** Honest, but it strands a documented canonical
command in a package whose entire purpose is a working verification baseline. Rejected by the
owner.

**Normalize every file repo-wide, including the `justfile`.** Cleanest end state, and it was
the first draft of this ADR. Rejected on measurement: it rewrites a frozen file that has no
defect and inflates the P1-00 diff by ~250 lines of pure whitespace, burying the changes a
reviewer actually needs to see. Deferred to P1-11 as a standalone normalization commit.
