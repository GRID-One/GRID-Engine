-- Upgrade seed for migration 0001_schema_meta (ADR-015 D9; engine-spec §8.19 "upgrade fixture").
-- Synthetic, invented values only. Applied to a database at version 1; every row must survive
-- every later migration (crates/persistence/tests/migrations.rs).

INSERT INTO schema_meta (key, value) VALUES ('upgrade-seed', 'seed_0001');
