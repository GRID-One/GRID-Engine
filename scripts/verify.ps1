#Requires -Version 7.2
param(
    [ValidateSet("Full", "Changed")]
    [string]$Scope = "Full"
)

$ErrorActionPreference = "Stop"

# ErrorActionPreference governs PowerShell errors. It does NOT govern the exit code of a
# NATIVE command, and #Requires -Version 7.2 pins this script to a version where
# $PSNativeCommandUseErrorActionPreference is an off-by-default experimental feature. Without
# an explicit check every command below could fail and this script would still reach
# "[verify] All checks passed." and exit 0.
#
# That is not hypothetical. alpha-ci run 32376218798 on 8bc1eb1: tests/guards/run.sh printed
# "33 passed, 2 failed" and exited non-zero, this script ran six more guards, reported success,
# and windows-authoritative -- the 8.11 MERGE GATE -- went green. Every Windows green on this
# branch before this fix proves the steps RAN, not that they PASSED.
#
# Approved as the fourth ADR-001 D5 amendment; see docs/02-adr/009-verify-ps1-exit-codes.md.
# No check command is changed -- this is exit-code handling around them.
function Assert-Ok([string]$Step) {
    if ($LASTEXITCODE -ne 0) {
        throw "[verify] FAILED: $Step (exit $LASTEXITCODE)"
    }
}

Write-Host "[verify] Starting verification scope: $Scope"

if ($Scope -eq "Full" -or $Scope -eq "Changed") {
    Write-Host "[verify] Formatting..."
    cargo fmt --all -- --check
    Assert-Ok "cargo fmt --all -- --check"

    Write-Host "[verify] Linting..."
    cargo clippy --all-targets --all-features -- -D warnings
    Assert-Ok "cargo clippy"

    Write-Host "[verify] SQLx offline cache..."
    cargo sqlx prepare --check --workspace -- --lib
    Assert-Ok "cargo sqlx prepare --check"

    Write-Host "[verify] Rust tests..."
    cargo nextest run --workspace
    Assert-Ok "cargo nextest run"

    Write-Host "[verify] Doctests..."
    cargo test --workspace --doc
    Assert-Ok "cargo test --doc"

    Write-Host "[verify] Audit..."
    cargo deny check
    Assert-Ok "cargo deny check"
    cargo audit
    Assert-Ok "cargo audit"

    Write-Host "[verify] Repository guards..."
    bash ./tests/guards/run.sh
    Assert-Ok "tests/guards/run.sh"
    bash ./scripts/check-migrations.sh
    Assert-Ok "check-migrations.sh"
    bash ./scripts/check-secrets.sh
    Assert-Ok "check-secrets.sh"
    bash ./scripts/check-traceability.sh
    Assert-Ok "check-traceability.sh"
    bash ./scripts/check-authority-sync.sh
    Assert-Ok "check-authority-sync.sh"
    bash ./scripts/check-verify-parity.sh
    Assert-Ok "check-verify-parity.sh"
    bash ./scripts/check-env-contract.sh
    Assert-Ok "check-env-contract.sh"

    Write-Host "[verify] Typos..."
    typos
    Assert-Ok "typos"
}

Write-Host "[verify] All checks passed."