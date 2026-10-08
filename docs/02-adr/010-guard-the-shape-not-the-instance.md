---
adr-id: 010
status: Accepted
date: 2026-08-29
deciders: Product/Architecture owner
supersedes:
superseded-by:
---

# ADR-010 — Fix the shape, not the instance: where a fail-closed guard belongs

## Status

Accepted 2026-08-29. **This ADR does not amend ADR-001 D5.** It changes no verification recipe
and adds no check to `just verify`; it records a practice and the placement rule that follows
from it. Recorded here rather than in a commit message because the whole premise of P1-00 is
that a fresh session can discover the repository's reasoning without prior chat context.

## Context

Three of the last four blocker-class defects in P1-00 were **second instances of a defect
already fixed elsewhere in the same file.** The third adversarial review named the pattern
directly, and it is worth quoting because it is a statement about method, not about care:

> when a defect is found, the fix is being applied to the instance rather than swept for across
> the file, and no mechanism enforces the sweep.

The sequence, in `scripts/check-secrets.sh` alone:

| Round | Defect | Fix applied to |
|---|---|---|
| 1 | `git diff <tree>...HEAD` failed and `\|\| true` swallowed it; OK printed over a committed token | that expression |
| 2 | `scan_files` opened zero of the paths handed to it on Windows (CR on every path) | the **full-tree** call site |
| 3 | the same `scan_files`, called for **untracked** files, still unguarded | — |

Round 3's instance was worse than the review found. The finding described it as a diff-mode
gap. Reproduced here, it fails open in **both** arms: in diff mode nothing checked the untracked
scan at all, and in full-tree mode the round-2 call-site guard runs *several lines before* the
untracked scan, so it only ever counted the tracked pass. One defect, two arms, and the guard
written to prevent it sat in between them.

The same shape appears outside that file. `tests/guards/run.sh`'s `Assert-Ok` coverage analysis
asked "does this line look like a command?" and so could not see six of the seven ways a native
command can be written. `check-verify-parity.sh` compared two of the three frozen
implementations. In each case the check was real, tested, and narrower than the thing it was
believed to cover.

## Decision

**A fail-closed guard belongs at the narrowest point that every path must pass through — not at
the call site where the defect was observed.**

Three rules follow, and they are what a later session should apply:

1. **Guard the function, not the caller.** `scan_files` now counts what it was handed, what it
   deliberately skipped, and what it opened, and refuses to return having opened nothing it
   cannot account for. Every caller is covered, including callers not yet written. The
   redundant call-site guard was removed rather than left alongside it: two guards for one
   property is how they drift apart.

2. **Prefer a rule that fails safe on the unknown.** The `Assert-Ok` analysis was inverted from
   "a line is a command if it looks like one" to "a line is a command unless it is a recognised
   construct". A new invocation form is then caught by default instead of needing the pattern
   widened to admit it first. Whitelists of the familiar are how blind spots are built.

3. **A test must prove the check can SEE, not merely that it can FAIL.** A mutation control that
   removes one `Assert-Ok` proves the analysis is not inert. It says nothing about the six forms
   the analysis never examined — and it passed throughout. Coverage controls now enumerate every
   form, and each fixture is paired with a control asserting the *unmodified* code behaves
   differently on the same input, so a case cannot pass for the wrong reason.

**When closing a finding, grep the file for the same shape before closing it, and add the
regression case against the shape rather than the instance.** Where the sweep cannot be
mechanised, say so explicitly rather than asserting it was done.

## Consequences

- `scripts/check-secrets.sh` fails closed for every corpus, present and future.
- `scripts/check-verify-parity.sh` reads all three frozen implementations. `verify.ps1` is the
  §8.11 merge-authoritative gate; a step missing only from it would have made that gate run one
  check fewer than the smoke gate, silently. It was not drifted when found — the guard is what
  keeps "not drifted" from being luck.
- `scripts/check-evidence-claims.sh` makes the claim-verification pass mechanical. It had been a
  routine run from memory over nine facts, so a tenth ("All 65 non-trivial paths traced",
  actually 66) sat outside it for three rounds. A pass you must remember to extend is not a
  control.
- The guard suite grew 42 → 54 cases, most of them controls rather than new assertions.
- **Cost, stated plainly:** every one of these guards is slower and wordier than the check it
  replaced, and two of them (the coverage control, the claim checker) exist only to test other
  tests. That is the right trade here because every defect this package has shipped was a
  verification harness reporting success over work it had not done — but it is a real cost and
  should not be copied reflexively into code that is not itself a gate.

## Alternatives considered

- **Fix the untracked call site and move on.** Cheapest, and what rounds 1 and 2 did. Rejected:
  it is the method that produced three repeat blockers, and it leaves the next call site
  unguarded by construction.
- **Add a lint that every `scan_files`-like call is followed by a check.** Rejected as
  over-engineering for a two-call function; moving the guard inside makes the lint unnecessary.
- **Put `check-evidence-claims.sh` inside `just verify`.** Rejected on a genuine circularity:
  the manifest records verify's own output, so requiring a current manifest before verify can
  pass means neither ever converges. It runs in CI's guards job instead, where the commit under
  test is already pushed. This is also why it needs no D5 amendment — it adds no verify step.

## Compliance

- No verification recipe changed; `check-verify-parity.sh` reports 16 steps across all three
  implementations, unchanged.
- Each finding was reproduced before being fixed, and each fix was mutation-controlled: with the
  new guard disabled, the three empty-scan cases go red; with the old coverage analysis
  restored, six of the seven form cases go red.
- `.\scripts\verify.ps1 -Scope Full` remains authoritative for merge (§8.11).
