-- P1-00 / ADR-002: infrastructure-only bootstrap table.
--
-- Exists so the SQLx toolchain (migrations, compile-time-checked queries, committed offline
-- cache) is proven end to end from the first work package, per final-build-spec.md 8.2:
-- "Use `sqlx migrate` from day one."
--
-- Deliberately NOT a domain table. Games, teams, players, drives, plays, raw_data, jobs,
-- model_versions and every other domain schema group remain P1-02 (alpha-spec.md 8.5, 9.5).
--
-- NO SCHEMA VERSION IS SEEDED HERE. An earlier revision inserted schema_version = '0001',
-- which duplicated state SQLx already maintains in _sqlx_migrations and could silently go
-- stale the moment 0002 landed -- nothing would have updated the literal, and the test
-- asserting it would have kept passing on a wrong value. Adversarial review of PR #1 caught
-- this. The applied-migration version is derived from the migrator, never hand-written.
--
-- Whether a richer schema-version mechanism is needed at all is P1-02's decision.
--
-- Migrations are append-only from here; enforced by scripts/check-migrations.sh.

CREATE TABLE schema_meta (
    key   TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);
