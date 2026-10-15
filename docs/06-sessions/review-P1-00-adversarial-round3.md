---
reviewer: fresh-context-adversarial
round: 3
date: 2026-08-20
work_package: P1-00
branch: wp/P1-00-repo-bootstrap
head_reviewed: 8e07b04ba59b42ec900db826cf12a9ffb46844d8
base: origin/main
review_branch: claude/adversarial-review-p1-00-l32emp
---

# Adversarial Review: P1-00 — Round 3

> **Path note.** Rounds 1 and 2 were written to `docs/05-sessions/` because the review brief
> named it. The authority index calls that path invalid, `docs/06-sessions/README.md` now
> records the collision as needing an owner decision, and no brief specified a path for this
> round. So this file follows the repository's own authority rather than repeating the
> mistake a third time. The two earlier files still need moving; that is an owner action.
>
> **Head note.** The request named `6b7f3fd`. The branch head is **`8e07b04`**, two commits
> later. Reviewing a named-but-stale head is the M2 defect wearing a different hat, so this
> review is against `8e07b04`. The two intervening commits touch only `.ai/evidence/*` and
> ADR-003.

## Summary

- **Verdict: CONDITIONAL** — 1 blocker, 3 major, 4 minor.
- **Confidence: High** on the blocker and majors — each was reproduced by execution, not
  reasoned about. Stated explicitly where I could not verify something.
- **Time spent:** ~50 minutes.
- Round 2's 3 blockers and 9 majors: **all closed or owner-recorded.** Verified, not accepted
  on assertion — see *Round 2 dispositions* below.

**The round-2 diagnosis still holds, one layer down.** Round 2 said the prose drifts behind
the code. Round 3's finding is narrower and worse: **the fixes drift behind each other.** The
blocker below is the *same defect*, in the *same file*, in the *other arm of the same `if`* as
the one fixed in this round — and the guard suite grew by 30 cases without covering it. The
implementer named this pattern themselves ("the same defect class sitting untouched next to its
own fix") and then shipped another instance of it in the commit that says so.

That is not a criticism of care. The care is unusually high. It is a statement about method:
when a defect is found, the fix is being applied to the instance rather than swept for across
the file, and no mechanism enforces the sweep.

## Answers to the four questions asked

The implementer asked where to attack. Three of the four had something in them.

### Q1 — "Does the `Assert-Ok` coverage analysis have a blind spot?"

**Yes. Six.** I ran the analysis from `tests/guards/run.sh` against `verify.ps1` with an
unguarded native command inserted in seven different PowerShell forms. Baseline reproduces
their `16 guarded, 0 unguarded`. Results:

| Inserted unguarded command | Detected? |
|---|---|
| `    sbom-tool generate` (plain, indented) | **CAUGHT** |
| `    & $sbom generate` (call operator) | MISSED |
| `    .\tools\sbom.exe` (relative path) | MISSED |
| `    ifconfig` (name begins `if`) | MISSED |
| `    paramgen --check` (name begins `param`) | MISSED |
| `    throwaway-tool` (name begins `throw`) | MISSED |
| `sbom-tool generate` at column 0 | MISSED |

Two causes:

1. **`/^[[:space:]]+[A-Za-z]/` requires indentation and an alphabetic first character.**
   `& …` and `.\…` — the two idiomatic PowerShell forms for invoking a path or a variable, and
   `.\…` is the exact form the workflow itself uses to call `verify.ps1` — begin with `&` and
   `.`, match no rule, and are never examined. So does anything at column 0.
2. **The construct-skip alternation is unanchored:**
   `/^[[:space:]]*(if|throw|else|\}|function|param|\[|\$)/` matches any command whose *name
   begins with* those letters. `ifconfig` is skipped as if it were an `if` statement.

Severity is bounded by realism — nobody is adding `ifconfig` — but `& $tool` and `.\tool.exe`
are ordinary, and column 0 is one careless newline away. See M1.

### Q2 — "Can either empty-scan guard be fooled?"

**Yes — the diff-mode one, which is the arm CI takes.** This is the blocker. Reproduced end to
end; see C1.

### Q3 — "Could the self-test pass while `Assert-Ok` is inert?"

**Not in the way you asked, but the assertion is weaker than it looks.** I could not construct
the case you feared: with `Assert-Ok` removed the stubbed `cargo` fails 8 times, the last
native command (`typos`) succeeds, `$LASTEXITCODE` is 0, `pwsh -File` exits 0, and the
self-test correctly throws "the fix is inert". That path is sound.

The weakness is elsewhere. `if ($rc -eq 0) { throw }` accepts **any** non-zero as proof, without
checking the failure came from `Assert-Ok`. A renamed script, a `pwsh` startup error, a PATH
mishap, `bash` missing on the runner — each exits non-zero and reads as "correctly refused to
pass". And the self-test exercises `Assert-Ok` **#1 of 16**: `cargo fmt` is the first command,
so it throws immediately and the other fifteen are never reached. See M3.

### Q4 — "Anything asserted that isn't mechanically checked."

One found, and it is small: the manifest says the traceability guard covered **65** non-trivial
paths. The true count at `6b7f3fd` is **66**, and CI printed 66 in the same run the manifest
cites. See min-1.

I also checked the `check-authority-sync` byte count, where the manifest says 94813 and the
Windows CI log says 96999 — **this one is fine.** The file is 94813 bytes in git; the Windows
working copy is CRLF-expanded by 2186 lines. Recording it so nobody re-opens it.

---

## Findings

### Critical (must fix before merge)

**C1. `check-secrets.sh` still fails open in diff mode — the arm CI takes — over untracked files.**

- **Location:** `scripts/check-secrets.sh`, the `if git rev-parse …` true-branch (diff mode)
- **Authority violated:** alpha-spec §8.11 (secret scanning); §12.7 / Appendix D (a security
  control may not be weakened or report an unrun check as passing).

**The defect.** Round 2's Windows fail-open was `scan_files` opening zero of the files it was
handed. The fix added an empty-scan guard — **to the full-tree branch only.** In diff mode
`scan_files` is still called, for untracked files, and **nothing checks `scanned`**. The
diff-mode guard compares `hunk_lines` against `changed_n`; it never asks whether the untracked
scan opened anything.

**Reproduced.** A throwaway repo, diff mode against a base, one changed file, and one
*untracked* file holding a real deny-tier `ghp_` token:

```
# control — scanner intact
check-secrets: FAIL pattern matched: gh[pousr]_[A-Za-z0-9]{36,}
    198:+token = "ghp_EEEE…"
exit=1                                          <- correct

# scan_files broken the same way the Windows job broke it (CR appended to each path)
check-secrets: OK no secret patterns found (diff vs base; 1 added line(s) from 1 changed file(s), plus 0 whole file(s))
exit=0                                          <- the secret is still sitting there
```

The verdict line even *prints* `plus 0 whole file(s)` and nothing acts on it.

**Why the guard suite missed it.** The suite grew 12 → 42 cases and has one case for each arm:
`zero files scanned over a non-empty tree fails closed` (full-tree) and `changed files with
zero added lines collected fails closed` (diff, hunk path). Neither exercises
*untracked-scan-inside-diff-mode*. The `scan_files` call in diff mode has no case at all.

**Why it matters despite CI looking clean.** In a clean CI checkout there are no untracked
files, so `scanned=0` is legitimate and the printed verdict is byte-identical to the broken
case — there is no way to tell them apart from the log. The gap bites (a) any local
`just verify`, which is where an agent works and where new files are untracked by definition,
and (b) the Windows merge gate, where the platform-specific path breakage that caused the
original fail-open lives.

**Fix required:** give the diff arm the guard the full-tree arm has — count the untracked
paths listed, and `die` if any were listed and none scanned. Add the guard-suite case, with a
control asserting the unmodified scanner passes on the same fixture (the pattern used elsewhere
in this suite, correctly).

---

### Major

**M1. The `Assert-Ok` coverage check misses six command forms — including the two idiomatic PowerShell ones.**

- **Location:** `tests/guards/run.sh`, `ps1_unguarded()`
- **Evidence:** the mutation table under Q1 above; all seven runs reproduced locally.
- **Risk:** this analysis is the *only* thing standing between a future `verify.ps1` edit and
  a reopened merge-gate fail-open — the self-test covers command #1 only (M3). A check added as
  `& $tool` or `.\tools\x.exe`, or pasted at column 0, is unguarded *and* invisible. The
  mutation control proves the analysis can fail; it does not prove it can see.
- **Fix:** match on "an indented line that is not a recognised PowerShell construct" rather
  than on `[A-Za-z]`; anchor the construct alternation with a word boundary
  (`(if|throw|else|function|param)\b`); and either scan column 0 too or assert that no native
  command appears outside the `if` block.

---

**M2. `verify.ps1`'s command list is not parity-checked against the other two implementations.**

- **Location:** `scripts/check-verify-parity.sh` (reads `justfile` and `scripts/verify.sh` only)
- **Issue:** D5 freezes three implementations and forbids unifying them. Parity is enforced
  between two of the three. ADR-009 notes in passing that "check-verify-parity … never [reads]
  the PowerShell implementation", but treats that as context for the Assert-Ok gap rather than
  as a gap of its own. A step added to `justfile` + `verify.sh` and forgotten in `verify.ps1`
  passes every guard in the repository, and the **merge-authoritative** gate silently runs one
  check fewer than the smoke gate.
- **Status today: not drifted.** I extracted all three lists and compared — 16 commands, in the
  same order, identical. This is latent, not live.
- **Fix:** extend the parity guard to three-way, or add a case asserting the `verify.ps1`
  command list equals `verify.sh`'s.

---

**M3. The ADR-009 self-test accepts any non-zero exit as proof, and exercises 1 of 16 asserts.**

- **Location:** `.github/workflows/alpha-ci.yml`, step *verify.ps1 propagates failures*
- **Issue (a):** the assertion is `$rc -ne 0`. It does not check *why*. Any non-zero — script
  renamed or moved, `pwsh` startup failure, `bash` unavailable for the guard lines, a PATH
  mishap that makes the stub itself unrunnable — reads as "verify.ps1 correctly refused to
  pass". The implementer applied exactly this reasoning to their own empty-scan fixture ("with
  a secret, the case would exit 1 whether or not the guard exists, and would pass for the wrong
  reason"); the same standard has not been applied here.
- **Issue (b):** `cargo fmt` is `verify.ps1`'s first command, so the stub throws on
  `Assert-Ok` #1 and the remaining fifteen are never executed by the self-test. Their coverage
  rests entirely on M1's text analysis.
- **What I could not fault:** the case they feared. With `Assert-Ok` inert the last native
  command (`typos`) succeeds, `$LASTEXITCODE` is 0, and the self-test correctly catches it.
- **Fix:** assert on the message, not just the code — require stdout/stderr to contain
  `[verify] FAILED: cargo fmt`. Optionally stub a *later* command in a second cheap run so more
  than one `Assert-Ok` is exercised.

---

### Minor

1. **The manifest's traceability count is wrong.** It records "All 65 non-trivial paths traced";
   the true count at `6b7f3fd` is **66**, and the CI run the manifest itself cites printed 66.
   Recomputed independently using the guard's own `is_trivial` rules. Small, but it is the M2
   class, in the artifact whose accuracy this round was largely about, and it sits outside the
   nine facts the claim-verification pass covers — which is precisely the weakness the
   implementer nominated in Q4.

2. **`windows-authoritative` has no offline-compile step.** `linux-smoke` gained
   *Fresh clone compiles offline* as M5's regression gate. The Windows job — the merge gate —
   does not have it. The property is platform-sensitive (path handling is what broke
   `scan_files` there), so the one job that most needs it is the one that lacks it.

3. **`verify.sh`'s scope guard must stay at column 0 or `check-verify-parity` breaks.** The
   coupling is documented in a comment, which is the right thing to have done, but it means a
   routine re-indent of a `case` arm in one file silently changes another file's verdict. Worth
   an assertion rather than a comment.

4. **The manifest attests to `6b7f3fd` while the head is `8e07b04`.** Inherent to the ordering
   and honestly recorded, and the two intervening commits are evidence-only — but the head has
   now moved past the attested commit in three consecutive rounds. If the manifest is always
   two commits stale in practice, the structural note may be understating a workflow problem
   rather than describing an unavoidable one.

---

## Round 2 dispositions — verified, not accepted

I re-checked the claims rather than reading the disposition table.

| Round 2 finding | Verified at `8e07b04` |
|---|---|
| **C1** `verify.sh` scope fail-open | **Closed.** Case-normalised, unknown scopes exit 2 with a named reason. 9 guard cases; CI log shows all 9 green on Windows |
| **C2** both `CLAUDE.md` contradict ADR-002 | **Closed**, and in three files — they found a third instance in `check-env-contract.sh`'s own remediation advice that I missed. CI prints `SQLX_OFFLINE=true (from .env)` |
| **C3** relicensing without sign-off | **Closed by the owner.** And my supporting fact was wrong — see the correction below |
| **M1** merge gate excluded two guards | **Closed.** `fetch-depth: 0` on both build jobs; SKIP paths exit 1 under `$CI`, with guard cases both ways. Windows log shows both guards printing real verdicts |
| **M2** false statements in the record | **Largely closed.** One number still wrong (min-1) |
| **M3** `human_approvals.required: []` | **Closed.** Three required sign-offs now listed |
| **M4** unpinned tooling | **Closed.** `toolchains/dev-tools.lock`, `--version` on every install, versions echoed in both jobs. Their answer to my Q7 — cargo-deny 0.20.2 honours `unmaintained`/`unsound` with no warning — is now backed by a CI step I can read |
| **M5** offline compile never exercised | **Closed for Linux** (see min-2) |
| **M6/M7** scope and §14.2 deviations | **Owner-recorded.** The mechanism I asked for |
| **M8** parity + env-contract untested | **Closed, and they went further than the finding.** The silently-empty-extraction case I raised as hypothetical turned out to also die at exit 1 with an *empty message*, indistinguishable from real drift. Both now diagnosed explicitly |
| **M9** permission profile drift | **Closed.** Prefix forms |
| minor 2 (`.cargo/config.toml`) | **Declined with reasoning.** Fair: swapping the mechanism whose last change was a blocker, inside the PR correcting the documentation about it, trades real risk for idiom |

**The Windows evidence (owner ruling D11).** The implementer withdrew every prior
`windows-authoritative` green as evidence of *passing* — including one already written into
ADR-003 as §14.2's compatibility check — and refused to combine a happy-path run with a
throw-path run from a different commit. I verified the discharge against the API rather than
the manifest: run **32415391750**, job **96575256117**, `windows-latest`, `head_sha 6b7f3fd`,
conclusion `success`, with step 5 *verify.ps1 propagates failures (ADR-009 self-test)* SUCCESS
and step 7 *Authoritative verification* SUCCESS **in the same run**. The log shows
`42 passed, 0 failed`, `16 guarded, 0 unguarded`, and all six guards printing real verdicts.
D11 is genuinely discharged. Withdrawing evidence that had already been cited, rather than
annotating it, is the strongest single act in this branch.

## Correction to my own round-2 C3

I wrote that `MIT OR Apache-2.0` "is the scaffold-shaped value" and that the conflict "was
resolved against the human's explicit act." **That supporting fact was wrong**, and I verified
the correction rather than accepting it:

```
f71a241  2026-08-19 16:05:08  Ethan Nelson  Add GNU GPL v3 license
2565acc  2026-08-19 16:35:13  Ethan Nelson  initial GRID-Alpha repository skeleton   <- adds license = "MIT OR Apache-2.0"
git merge-base --is-ancestor f71a241 2565acc  ->  true
```

Both are the owner's own commits thirty minutes apart and the permissive declaration is the
**later** one. `main` shipped a self-contradiction; there was no "explicit act" to resolve
against. I should have checked the ordering before characterising provenance. The objection
itself — relicensing is irreversible and needs an owner-authored artifact — did not depend on
that fact and has been satisfied.

## Questions for Implementer

1. **C1:** was the diff-mode `scan_files` call considered when the full-tree empty-scan guard
   was added, or did the sweep stop at the instance that had failed? The answer matters more
   than the fix — it is the third time this shape has appeared.
2. **M2:** is three-way parity worth enforcing now, or is the `verify.ps1` list better checked
   by a different mechanism given D5 freezes all three?
3. **min-4:** across three rounds the manifest has never covered the head. Is the "a manifest
   cannot record its own commit" framing hiding a fixable ordering problem — e.g. generating it
   in CI against the pushed head rather than locally before the push?

## Follow-up Work Identified

- Sweep, don't patch: for each fail-open found in P1-00, grep the file for the same shape
  before closing it. Three of the last four blockers were second instances.
- **P1-01:** the `.claude/agents/` definitions deferred by ADR-006. Every finding in rounds 2
  and 3 is a fail-open in a verification harness; a `security-review` agent definition is the
  thing §8.10 intends to carry that knowledge forward.
- **Owner, pre-merge:** move both earlier review files out of `docs/05-sessions/`, and fix the
  reviewer brief that names it. Recorded in `docs/06-sessions/README.md`; neither review branch
  can do it alone.
- **Owner, pre-merge:** Architecture, Security/Release and Merge-reviewer sign-offs remain
  outstanding, as the implementer states. The agent does not merge.
