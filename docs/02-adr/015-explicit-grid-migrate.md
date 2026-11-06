---
adr-id: 015
status: Proposed
date: 2026-11-06
deciders: Product/Architecture owner
supersedes:
superseded-by:
---

# ADR-015 — Migrations run only through an explicit `grid migrate`

## Status

**Proposed** on 2026-11-06 under work package P1-02
(`docs/01-work-packages/p1-02-sqlite-backbone-and-durable-jobs.md`).

- **What it decides.** It settles the question engine-spec §8.11 hands to P1-02: "P1-02 decides by ADR
  whether migrations run automatically at start-up or through an explicit subcommand. A destructive
  migration is always a human gate (§1.4)."
- **Where the choice came from.** The owner made it in owner ruling R3 (2026-10-21), recorded in the
  work package and in `docs/06-sessions/2026-11-06-P1-02.md`: "Explicit `grid migrate`". This ADR
  records that ruling and the mechanism around it.
- **While Proposed.** P1-02 implements it, as P0-01 implemented ADR-011 and ADR-012 and P1-01
  implemented ADR-014 while they were Proposed. The Product/Architecture owner accepts or rejects it
  at the merge.

## Context

- **The engine is a one-shot CLI under an external scheduler** (engine-spec §8.6.1, §8.10).
  - A scheduled `grid update` runs unattended.
  - A binary upgrade lands between two runs.
- **Migrations are append-only** (`scripts/check-migrations.sh`; root `CLAUDE.md`).
  - A table's name is fixed when its migration lands (§8.5).
  - A destructive migration needs explicit human approval (§1.4: "schemas with destructive
    potential").
- **The schema-version mechanism was left open.** `migrations/0001_schema_meta.sql` created
  `schema_meta`, and ADR-002 left the mechanism to P1-02: "Whether `schema_meta` becomes the real
  schema-version mechanism … is P1-02's call."
- **The oracle migrates in code on every start.** `reference/python/backend/db/connection.py:24-74`
  does `CREATE TABLE IF NOT EXISTS`, swallows `ALTER TABLE` errors and rebuilds a table by
  rename–copy–drop. That is an automatic, destructive migration with no record. It is not ported.
- **sqlx checksums every migration** and records it in `_sqlx_migrations`. A checkout whose line
  endings differ changes the checksum. sqlx's own guidance is to force LF through `.gitattributes`.
- **The store lock** (owner ruling R5, P1-02):
  - every invocation takes a shared store lock;
  - `grid migrate` takes it exclusively;
  - `grid update` also holds an exclusive update lock.
- **Refusals exit 30** (owner ruling R6; engine-output-contract §13, version 1.1).

## Decision

1. **D1. Embedded migrations.**
   - `grid-persistence` embeds `migrations/` with `sqlx::migrate!("../../migrations")`.
   - A `build.rs` re-runs the build when the directory changes.
   - The binary therefore knows exactly which migrations it was built with.
2. **D2. Every start-up classifies the store and refuses unless it is current.**
   - `Engine::open` compares the database's `_sqlx_migrations` rows with the embedded list, version by
     version and checksum by checksum, without writing anything. Only *current* proceeds.
   - Every refusal exits 30 (`DATABASE_FAILURE`), with this `EngineErrorCode`:

     | Store state | `EngineErrorCode` |
     |---|---|
     | No database | `UnsupportedVersion`, "run `grid migrate`" |
     | A pending migration | `UnsupportedVersion`, "run `grid migrate`" |
     | A version unknown to this build (database newer than binary) | `UnsupportedVersion` |
     | A checksum mismatch | `StateCorrupt` |
     | A migration recorded as failed (dirty) | `StateCorrupt` |

3. **D3. `grid migrate` is the only applier.** `Engine::migrate`, and the `grid migrate` subcommand
   that calls it, runs these steps:
   1. take the store lock exclusively (R5);
   2. create the data directory and the database file if they are absent;
   3. refuse a newer, mismatched or dirty database without writing to its schema;
   4. take a consistent pre-migration copy with `VACUUM INTO`, under
      `data/backups/grid-v<applied>-<unix seconds>.db`, when the database already has applied
      migrations;
   5. apply each pending migration in its own transaction;
   6. check `PRAGMA foreign_key_check` and `PRAGMA quick_check`;
   7. log each applied version, to `tracing` and to standard output.

   It is idempotent: on a current database it applies nothing and exits 0.
4. **D4. No other invocation changes the schema.**
   - A scheduled `grid update` never migrates.
   - After a binary upgrade it refuses with exit 30 until an operator runs `grid migrate`.
5. **D5. A destructive statement is a human gate, enforced by a test.**
   - A persistence test scans every migration, comments stripped. It fails on `DROP` of a table,
     index, view or trigger; `DELETE FROM`; `ALTER TABLE … DROP` or `… RENAME`; and `UPDATE … SET`.
   - The only escape is a migration named in the test's approval list. That list is empty, and
     only an owner-approved change may add to it.
6. **D6. The version mechanism.**
   - The mechanism is `_sqlx_migrations`, with its versions and checksums.
   - `schema_meta` is kept. Retiring it would be a destructive migration that buys nothing.
7. **D7. Migrations are LF on every platform.** `.gitattributes` carries
   `migrations/*.sql text eol=lf`, and a persistence test asserts that no migration contains `\r`.
8. **D8. Forward-only.**
   - There are no down migrations.
   - Rollback means the previous binary together with the D3 pre-migration copy, or a backup of
     `data/`.
   - This is how P1-02 reads the engine-spec §12.9 item 7 phrase "reversible where required". The
     owner confirms that reading when ruling on this ADR.
9. **D9. Every migration ships an upgrade seed.**
   - The seed is `crates/persistence/tests/fixtures/upgrade/seed_<NNNN>.sql`.
   - A test migrates from every earlier version with the seeds applied, and checks that every
     seeded row survives (the engine-spec §8.19 "upgrade fixture").

## Consequences

**Easier.**
- An unattended run can never change the schema.
- A failed migration is seen by the operator who started it, not by the scheduler at 3 a.m.
- Every applied version is logged, and a pre-migration copy exists.
- Checksums are identical on Linux and Windows checkouts.

**Harder.**
- A binary upgrade needs an operator step (`grid migrate`). Until then every invocation refuses with
  exit 30.
- The pre-migration copy costs disk space, about the size of the database per migration run.
  P1-11's retention runbook covers it.

**Inherited.**
- Every later package that adds tables:
  - adds a forward migration and its upgrade seed;
  - regenerates `.sqlx/` with `cargo sqlx prepare --workspace -- --lib`;
  - must not introduce a destructive statement without the owner's approval.
- P1-10 completes the argument grammar of `grid migrate`, including a configurable data directory.
  P1-02's `grid migrate` takes no arguments, and any further argument exits 2.
- P1-11's runbooks state the upgrade procedure.

## Compliance

| Rule | Enforced by |
|---|---|
| D1 embedded migrations, current-only start-up (D2) | `crates/pipeline/tests/engine_lifecycle.rs`: open refuses a missing, pending, newer, mismatched or dirty store and leaves it unchanged |
| D3 the applier, idempotence, pre-migration copy | `engine_lifecycle.rs` (`migrate_*` tests); `crates/grid-cli/tests/cli.rs` (`grid migrate` twice exits 0; with an extra argument exits 2) |
| D3 exclusive lock | `engine_lifecycle.rs`: `migrate` refuses (exit 30) while another invocation holds the shared lock |
| D4 | `engine_lifecycle.rs::open_never_migrates` |
| D5 | `crates/persistence/tests/migrations.rs::no_unapproved_destructive_statement` |
| D6 | `migrations.rs::blank_database_reaches_head` (the `schema_meta` row survives) |
| D7 | `.gitattributes`; `migrations.rs::migration_files_are_lf_only` |
| D9 | `migrations.rs::upgrade_from_every_prior_version_preserves_seeded_rows`, `every_migration_has_an_upgrade_seed` |
| Append-only | `scripts/check-migrations.sh` (unchanged) |

## Alternatives considered

1. **Automatic migration at start-up.** Lifecycle step 2 would apply pending migrations before any
   subcommand.
   - It is convenient: no operator step after an upgrade.
   - The owner rejected it (R3). A destructive migration would have only PR review as its human gate.
     A scheduled run would change the schema unattended.
2. **Migrate inside `grid update` only.** This is the same objection as 1, limited to one command.
   It would also make `update` the one command that both mutates the schema and runs long jobs.
3. **Leave migration to `sqlx migrate run` (sqlx-cli).**
   - A production install would need a dev tool.
   - It bypasses the store lock, so it could change the schema under a running engine.
   - It takes no pre-migration copy.
   - `just bootstrap` keeps using it for the development database only (ADR-002).
4. **Keep `schema_meta` as the version mechanism.** It would duplicate `_sqlx_migrations` without its
   checksums. Retiring it instead would be a destructive migration (D5).
5. **Down migrations.** sqlx supports reversible migrations. A down migration of an append-only,
   immutable store is itself destructive, and it would be tested less than the forward path.
   Rollback by copy (D8) is simpler and does not depend on hand-written inverse SQL.
