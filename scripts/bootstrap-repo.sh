#!/usr/bin/env bash
# Create any missing repository skeleton directories.
#
# IDEMPOTENT: never overwrites an existing file. The previous version wrote crate manifests
# and lib.rs stubs unconditionally, and created the FLAT docs tree from alpha-spec.md 8.7 —
# which would have clobbered the numbered Obsidian vault this repository actually uses
# (docs/02-adr/001-repo-bootstrap-decisions.md D3).
#
# This is a scaffolding helper, not part of verification. `just bootstrap` installs tooling
# and creates the development database; this script only fills in missing directories.
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

created=0
mk() { [[ -d "$1" ]] || { mkdir -p "$1"; echo "  + $1"; created=$((created+1)); }; }
keep() { mk "$(dirname "$1")"; [[ -e "$1" ]] || { : > "$1"; echo "  + $1"; created=$((created+1)); }; }

CRATES=(domain persistence ingestion identity features models simulation scoring
        evaluation governance application)

echo "[bootstrap-repo] crates"
for c in "${CRATES[@]}"; do mk "crates/$c/src"; done
mk crates/application/src/bin

# data/ is gitignored (raw and derived provider data is never committed), so a fresh clone
# and a bootstrapped tree differ by this one empty directory. That is intended: ingestion in
# P1-03/P1-04 expects the mount point to exist, and creating it here is cheaper than every
# later script having to. Second adversarial review, minor 8.
echo "[bootstrap-repo] data and migrations"
mk migrations
mk data

echo "[bootstrap-repo] docs (numbered vault — see ADR-001 D3, NOT the flat 8.7 tree)"
mk docs/00-meta/specs
mk docs/01-work-packages
mk docs/02-adr
mk docs/03-contracts        # first populated by P0-01 (engine consolidation)
mk docs/04-providers/nflverse
mk docs/04-providers/cfbd
mk docs/05-model-specs      # first populated by P0-01 (engine consolidation)
mk docs/06-sessions
mk docs/07-archive          # non-authoritative history (P0-01)
mk docs/99-templates

# No app/ and no crates/ffi/: the engine-only pivot (ADR-011) removed the Flutter app and the
# flutter_rust_bridge crate. reference/python/ is committed source (the Python reference
# oracle, ADR-012), not scaffolding, so it is not created here either.
echo "[bootstrap-repo] evidence"
mk .ai/evidence

if [[ "$created" -eq 0 ]]; then
    echo "[bootstrap-repo] nothing to do — skeleton already complete"
else
    echo "[bootstrap-repo] created $created path(s)"
fi

cat <<'NEXT'

Crate manifests and lib.rs files are NOT generated here — they are committed source, not
scaffolding. Next steps on a fresh environment:
  just bootstrap    install dev tooling and create the local development database
  just verify       Linux smoke
NEXT
