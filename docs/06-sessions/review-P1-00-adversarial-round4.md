---
reviewer: fresh-context-adversarial
round: 4 (verification pass)
date: 2026-08-30
work_package: P1-00
branch: wp/P1-00-repo-bootstrap
head_reviewed: 38234782280a6f4287b6e3ba15e7f7545f997eb4
base: origin/main
---

# P1-00 — Round 4: verification of the round-3 fixes

Scope is narrower than round 3: did the round-3 findings actually get fixed, did anything
break, and do the fixes hold under the same adversarial pressure that found the originals.
Every claim below was re-tested by execution against `8e07b04..3823478` (7 commits, +911/−119).

## Verdict

**APPROVED with findings** — 0 blockers. 1 major, 1 minor, both residual rather than new
regressions. The remaining gate is the three human sign-offs, which is not mine to clear.

**All four round-3 findings and all four minors are closed.** I re-ran the original
reproductions rather than reading the dispositions.

| Round 3 | Verified at `3823478` |
|---|---|
| **C1** diff-mode empty-scan fail-open | **Closed.** Re-ran the exact reproduction: broke `scan_files` path resolution the way the Windows job broke it, with a real `ghp_` token in an untracked file. Now dies with a full accounting (`1 path(s) listed, ZERO opened, only 0 accounted for as deliberate skips`). Stronger than the fix I asked for — it accounts for every listed path rather than checking a single counter |
| **M1** six blind spots in the `Assert-Ok` coverage analysis | **Closed.** Re-ran my seven-form mutation table: **7/7 CAUGHT** (was 1/7). The rewrite drops the `[A-Za-z]`-anchored rule for a catch-all and bounds the keyword list with `([^[:alnum:]_-]|$)`. They also found and fixed something I did not: `\b` is a backspace in awk, not a word boundary, and Ubuntu runners use mawk which has neither |
| **M2** `verify.ps1` not parity-checked | **Closed.** Three-way parity, and it names *which pair* drifted rather than reporting a bare disagreement |
| **M3** self-test accepts any non-zero | **Closed, and past what I asked.** Now asserts the message (`[verify] FAILED: cargo fmt`), and adds a second run stubbing the **last** command so `Assert-Ok` #16 is exercised too — I had only asked for the message assertion |
| **min-1** stale traceability count | **Closed, and generalised.** `scripts/check-evidence-claims.sh` re-derives 8 claims from the artifacts, with a floor (`checked < 8` fails) so a phrasing change cannot silently drop claims. Live count now reads 68; actual at HEAD is 68. The surviving "65" is a quotation inside the round-3 disposition record, which is correct |
| **min-2** Windows lacked the offline-compile gate | **Closed.** Both build jobs now carry it |
| **min-3** column-0 coupling | **Closed** via the parity rewrite |
| **min-4** manifest never covers the head | **Addressed honestly.** `check-evidence-claims.sh` states outright that it deliberately does *not* assert manifest-commit == HEAD, with the reason. Claim 8 instead pins the body's "Final commit" to the commit the manifest attests to — which is the drift that actually caused harm |

**CI on the real head.** Run `33270930510` on `3823478`: all three jobs success. Windows carries
the self-test, the offline-compile gate and the authoritative verification. The guards job now
runs three-way parity and the evidence-claims check as named steps.

**Two things they caught that I did not**, both worth recording because they are the same class
I have been reporting: an `awk -v` value processes escape sequences, so
`-v ins='.\tools\x.exe'` silently delivered a mangled string — the relative-path case was
passing while testing something else. They added a verbatim-insertion assertion so a fixture
that does not land is a failure rather than a pass. That is the "passing for the wrong reason"
discipline applied to their own test harness.

---

## Findings

### Major

**R4-1. The path-accounting guard catches total failure, not partial.**

- **Location:** `scripts/check-secrets.sh`, `scan_files()`, the `accounted` guard
- **Condition is `listed > 0 && opened == 0 && accounted < listed`.** It fires only when
  **nothing at all** was opened. The unresolvable branch is even commented
  `# UNRESOLVABLE -- not accounted for` and then not acted on.
- **Reproduced.** Two clean untracked files, no secret anywhere so only the guard can produce a
  non-zero exit; path resolution broken for one of the two:

  ```
  control  -> check-secrets: OK ... plus 2 whole file(s)   exit=0
  partial  -> check-secrets: OK ... plus 1 whole file(s)   exit=0   <- one path silently unread
  ```

- **Why it matters.** ADR-010 is titled *guard the shape, not the instance*. The instance —
  CR on **every** path, the Windows signature — is closed. The shape is "a path the scanner was
  handed and could not reach", and a subset failing is the same shape: a filename the loop
  mishandles, a file removed between `git ls-files --others` and the read, a permission error.
  The printed count is the only signal and nothing reads it.
- **Fix:** the values are already in hand. `unresolved=$((listed - excluded - skipped - opened))`,
  `die` when `unresolved > 0`. Add a partial-failure guard case, with a no-secret fixture so the
  guard is the only thing that can fail it.

### Minor

**R4-2. Three residual evasions in the rewritten coverage analysis; one certifies rather than misses.**

Tested against the new `ps1_unguarded()`:

| form | result |
|---|---|
| `cargo sbom generate; Write-Host "done"` | MISSED — `/Write-Host/ { next }` skips the whole line |
| `typos --config Assert-Ok.toml` | MISSED — `/Assert-Ok/ { next }` skips the whole line |
| `cargo a; cargo b` followed by one `Assert-Ok` | **reported as `17 guarded, 0 unguarded`** |

The third is the one worth acting on. `$LASTEXITCODE` after `a; b` reflects only `b`, so the
first command's failure is invisible — and the analysis does not merely fail to see it, it
**certifies it as guarded**. Low realism today: `verify.ps1` is one command per line and any
addition passes review. But the first two rules skip on a *substring anywhere in the line*,
which is the same over-broad-skip shape that caused the original M1.

**Fix:** anchor the `Write-Host` and `Assert-Ok` skips to the start of the line, and either
reject `;`-joined native commands or require an `Assert-Ok` per segment.

## Questions answered from round 3

1. **Was the diff-mode `scan_files` call considered when the full-tree guard was added?**
   Answered in ADR-010 and in the code comments: no — the fix was applied to the instance. The
   response is a documented rule rather than a denial, which is the right shape.
2. **Three-way parity or another mechanism?** Three-way, implemented.
3. **Is the manifest-vs-head framing hiding a fixable problem?** Partly. They kept the
   structural limitation and fixed the part that actually caused harm (body vs manifest
   disagreeing on the attested commit). Reasonable.

## Remaining before merge — none of it mine

- Architecture owner: authority order, crate boundaries, the P1-02 → P1-00 rework
- Security/Release owner: `.claude/settings.json`
- Merge reviewer: the final diff
- Owner: move the two review files out of `docs/05-sessions/` and fix the reviewer brief that
  names that path
