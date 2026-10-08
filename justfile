set shell := ["bash", "-cu"]

default:
    just verify

# Bootstrap dev tools and the local development database.
# Run once per fresh environment. NOT a verify recipe: the recipes under
# `verify` are a frozen contract (docs/02-adr/001-repo-bootstrap-decisions.md D5).
# Versions are pinned: --locked pins a tool's dependencies, only --version pins the tool.
# Keep in step with toolchains/dev-tools.lock and .github/workflows/alpha-ci.yml.
# sqlx-cli takes the same feature narrowing CI uses -- the default set pulls the MySQL and
# Postgres drivers this project never uses.
bootstrap:
    cargo install cargo-nextest --locked --version 0.9.143
    cargo install sqlx-cli --locked --version 0.9.0 --no-default-features --features sqlite,rustls
    cargo install cargo-deny --locked --version 0.20.2
    cargo install cargo-audit --locked --version 0.22.2
    cargo install typos-cli --locked --version 1.49.0
    # Local SQLite development database for the compile-time query cache (ADR-002).
    # target/ must exist first: sqlite cannot create a file in a missing directory,
    # and a fresh checkout has no target/ until cargo runs.
    mkdir -p target
    sqlx database create
    sqlx migrate run

# Full verification suite (Linux smoke)
verify:
    just check-fmt
    just check-lint
    just check-sqlx
    just test-rust
    just test-doc
    just audit
    just check-guards
    just check-typos

check-fmt:
    cargo fmt --all -- --check

check-lint:
    cargo clippy --all-targets --all-features -- -D warnings

check-sqlx:
    cargo sqlx prepare --check --workspace -- --lib

test-rust:
    cargo nextest run --workspace

# nextest does not run doctests at all (ADR-007). Zero exist today; this keeps the gate
# from going blind the moment P1-01 adds a doc example.
test-doc:
    cargo test --workspace --doc

audit:
    cargo deny check
    cargo audit

# alpha-spec.md 8.11 lists secret scanning and traceability as part of the verification
# orchestration. Before ADR-007 these ran only in CI, so the canonical command provided
# neither. BASE_REF lets CI pass the PR base; locally it defaults to origin/main.
check-guards:
    ./tests/guards/run.sh
    ./scripts/check-migrations.sh
    ./scripts/check-secrets.sh
    ./scripts/check-traceability.sh
    ./scripts/check-authority-sync.sh
    ./scripts/check-verify-parity.sh
    ./scripts/check-env-contract.sh

check-typos:
    typos

# Database
db-prepare:
    cargo sqlx prepare -- --lib

db-migrate:
    sqlx migrate run

# Evidence manifest
evidence WP_ID:
    ./scripts/generate-evidence-manifest.sh {{WP_ID}}

# Benchmark and mutation tooling. Deliberately NOT in `bootstrap`: no verify recipe and no CI
# job uses either, so every CI run was compiling them from source for nothing.
bootstrap-bench:
    cargo install hyperfine --locked
    cargo install cargo-mutants --locked

# Experimental: mutation testing on statistical core
mutants:
    cargo mutants --package grid-models --output mutants.out

# Bench (placeholder)
bench:
    @echo "Add hyperfine benchmarks after P1-07"