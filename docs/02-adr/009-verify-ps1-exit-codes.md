---
adr-id: 009
status: Accepted
date: 2026-08-20
deciders: Product/Architecture owner
supersedes:
superseded-by: "011 (in part, proposed): the 16-command count and the Windows-production-target rationale"
---

# ADR-009 — `scripts/verify.ps1` must propagate native command exit codes

## Status
Accepted 2026-08-20. This ADR **amends ADR-001 D5** for the fourth time. It required explicit
owner approval and got it, on the record, before the change was made.

## Context

`scripts/verify.ps1` is the **merge-authoritative** verification command under `alpha-spec.md`
§8.11: the production target is Windows, and no Linux run substitutes for it. It could not fail.

The script set `$ErrorActionPreference = "Stop"` and ran sixteen native commands with no exit-code
check between them (`grep -c LASTEXITCODE scripts/verify.ps1` → **0**). `$ErrorActionPreference`
governs PowerShell's own error records; it does not govern the exit status of a native executable.
PowerShell 7.3 added `$PSNativeCommandUseErrorActionPreference` to bridge that gap, but this script
carries `#Requires -Version 7.2`, where the feature is experimental and **off by default**. So a
non-zero exit from any command was discarded and execution continued to
`Write-Host "[verify] All checks passed."` and exit 0.

**This is not a hypothetical.** alpha-ci run `32376218798` on `8bc1eb1`, job `96448140619`:

```
33 passed, 2 failed          <- tests/guards/run.sh, exit non-zero
...six more guards ran...
[verify] All checks passed.  <- exit 0; windows-authoritative reported SUCCESS
```

The two failures were themselves a fail-open: `check-secrets.sh` examined zero files on Windows
and printed `OK no secret patterns found` over a committed `ghp_` token. So the merge gate went
green while the secret scanner was blind **and** while the suite that detected it was reporting
failure. Two independent fail-opens, stacked, neither visible from the job's conclusion.

The commands that could have failed silently are every check the gate exists to run:
`cargo fmt --all -- --check`, `cargo clippy -- -D warnings`,
`cargo sqlx prepare --check --workspace`, `cargo nextest run`, `cargo test --doc`,
`cargo test -p grid-ffi`, `cargo deny check`, `cargo audit`, all seven guards, and `typos`.

### Why no change outside the frozen file can fix it

ADR-005's first condition asks whether the repository can change instead of the recipe. The fault
is in the frozen file's own control flow — the absence of a check between commands — so nothing
outside it reaches the fault. Alternatives weighed:

| Attempt | Result |
|---|---|
| Set `$PSNativeCommandUseErrorActionPreference = $true` (one line) | Smallest diff, and a **silent no-op** on the 7.2 the script pins: the very failure mode being fixed. Rejected on those grounds by the owner |
| Have CI check the exit code instead | The workflow already does; GitHub reported the step successful because the *script* exited 0. The lie originates in the script |
| Make each guard louder | Does not help: the guard suite already exited non-zero and printed "2 failed". Nothing was listening |
| Drop `#Requires -Version 7.2` to 7.4 and rely on the default | Trades an explicit check for a version-dependent default, and silently changes the supported floor. A verification gate should not depend on which PowerShell the runner happens to ship |

## Decision

**Add an explicit exit-code assertion after every native command**, via one helper defined before
the first check:

```powershell
function Assert-Ok([string]$Step) {
    if ($LASTEXITCODE -ne 0) {
        throw "[verify] FAILED: $Step (exit $LASTEXITCODE)"
    }
}
```

Sixteen call sites, one per command, each naming its step so a failure says which check failed
rather than only that something did. Works on every PowerShell version, which the one-line
preference variable does not.

**No check command is changed.** The sixteen commands are byte-identical;
`check-verify-parity.sh` still reports `justfile and scripts/verify.sh agree on 16 verification
step(s)` (it compares the justfile against `verify.sh` and never reads `verify.ps1`, so parity was
never able to catch this either — worth knowing).

`*.ps1` is `-text` in `.gitattributes` (ADR-004), so the file's CRLF endings are preserved by the
edit; verified after patching.

## Consequences

**Easier.** The §8.11 merge gate can fail. For the first time, a green `windows-authoritative`
means the checks passed rather than that they ran.

**Harder — and this is the real cost.** Every `windows-authoritative` green recorded before this
change is **withdrawn as evidence of passing**. That includes run `32329932430`, which ADR-003
cited as the §14.2 Windows-compatibility check, and the entries in
`.ai/evidence/P1-00/manifest.json` and `PR-BODY.md`. They are re-recorded as NOT ESTABLISHED with
the reason, and will be re-earned only from a run of the fixed script. `verification_status`
drops to `passed_with_exceptions` until then. The owner ruled explicitly for withdraw-and-re-earn
over annotating in place.

**D5 is now amended four times** — ADR-005, ADR-007, ADR-008, and this. All four increased
coverage; none relaxed a check. Two of the four (008, 009) were defects in the *harness around*
the checks rather than in the checks themselves, which is worth naming as a pattern: this project
froze the check list and left the code that decides whether a check counts unexamined. An
amendment that reduces coverage should still be refused outright.

**Inherited.** P1-01 onward gets a Windows gate that behaves like a gate.

## Demonstrated

The fix is verified in both directions, which matters because an all-green run cannot
distinguish a working `Assert-Ok` from an inert one — both produce identical output when nothing
fails. That is the same "gate that passes vacuously" argument ADR-002 makes, applied here to the
fix rather than to the gate.

**Happy path** — run `32382377066` on `a190e71`, `windows-authoritative`: every command passed,
`verify.ps1` exited 0, and the guard suite reported `38 passed, 0 failed` on Windows.

**Throw path** — run `32403569748` on `c2fc053`, the `ADR-009 self-test` step, which runs the real
script with a stub `cargo` that exits 7:

```
[verify] Starting verification scope: Full
[verify] Formatting...
D:\a\GRID-Alpha\GRID-Alpha>exit /b 7
Exception: ...\scripts\verify.ps1:24
  24 |    throw "[verify] FAILED: $Step (exit $LASTEXITCODE)"
     |    [verify] FAILED: cargo fmt --all -- --check (exit 7)
verify.ps1 correctly refused to pass with a failing command: exit 1
```

`Assert-Ok` fired, named the exact failing command, and the script exited **1**. This is the
first time in this branch that the §8.11 merge gate has been observed refusing to pass.

**The step around it was wrong on that first run, and the script was not.** `pwsh -File` left
`$LASTEXITCODE` at 1 — correctly, since the child was asked to fail — and GitHub's `pwsh`
wrapper exits a step with whatever `$LASTEXITCODE` holds at the end, so the step printed its
success message and then reported failure. Fixed by ending the step with an explicit `exit 0`.
Recorded because the distinction is the whole point: a verification step that deliberately runs
a command it expects to fail must state its own verdict rather than inherit one.

## Compliance

- `grep -c 'Assert-Ok "' scripts/verify.ps1` must equal the number of native commands (16 today).
  A command added without an assertion re-opens the hole.
- `scripts/check-verify-parity.sh` cannot see `verify.ps1`; extending it to cover the PowerShell
  implementation is recorded as a P1-11 follow-up.
- `file scripts/verify.ps1` must still report CRLF after any edit (ADR-004).
- The three ADR-005 conditions apply unchanged to any future amendment.

## Alternatives considered

**Rewrite `verify.ps1` to call `bash scripts/verify.sh`.** Removes the duplication and the whole
class of defect at once. Rejected: D5 forbids unifying the two implementations, `verify.ps1` is
the Windows-native gate by design (`final-build-spec.md` §3.2), and routing the merge gate through
Git Bash would make Windows verification depend on a POSIX emulation layer.

**Leave it and rely on the guards job.** The dedicated `guards` job does run the guard suite
properly. Rejected: it does not run `cargo fmt`, `clippy`, `nextest`, `deny`, `audit` or `typos`,
and §8.11 names `verify.ps1 -Scope Full` as the merge gate. A second job passing is not the named
gate working.
