#!/bin/bash
set -euo pipefail

# Scope is normalized and validated before anything runs. An unrecognized value used to fall
# straight through to the final "All checks passed." line, so `verify.sh Full` -- the exact
# capitalization every authority document prints -- ran zero checks and exited 0.
# verify.ps1 never had this hole: [ValidateSet("Full","Changed")] rejects bad input for it.
# Approved as the third D5 amendment; see docs/02-adr/008-verify-scope-guard.md.
#
# Written at column 0 deliberately: check-verify-parity.sh extracts commands with
# `grep -E '^[[:space:]]+[a-zA-Z._/]'`, so an indented case arm would be read as a
# verification step and break parity with the justfile.
SCOPE="$(printf '%s' "${1:-full}" | tr '[:upper:]' '[:lower:]')"
case "$SCOPE" in
full|changed) ;;
*) echo "[verify] unknown scope: '${1}' (expected Full or Changed)" >&2; exit 2 ;;
esac
echo "[verify] Starting verification scope: $SCOPE"

if [[ "$SCOPE" == "full" || "$SCOPE" == "changed" ]]; then
    echo "[verify] Formatting..."
    cargo fmt --all -- --check

    echo "[verify] Linting..."
    cargo clippy --all-targets --all-features -- -D warnings

    echo "[verify] SQLx offline cache..."
    cargo sqlx prepare --check --workspace -- --lib

    echo "[verify] Rust tests..."
    cargo nextest run --workspace

    echo "[verify] Doctests..."
    cargo test --workspace --doc

    echo "[verify] Audit..."
    cargo deny check
    cargo audit

    echo "[verify] Repository guards..."
    ./tests/guards/run.sh
    ./scripts/check-migrations.sh
    ./scripts/check-secrets.sh
    ./scripts/check-traceability.sh
    ./scripts/check-authority-sync.sh
    ./scripts/check-verify-parity.sh
    ./scripts/check-env-contract.sh

    echo "[verify] Typos..."
    typos
fi

echo "[verify] All checks passed."