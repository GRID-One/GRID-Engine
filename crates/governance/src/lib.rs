//! `grid-governance` — Model promotion, rollback, snapshot, job queue.
//!
//! This crate owns model states and the candidate promotion gate (`engine-spec.md` §8.8), the
//! pre-registered threshold registry, production pointers and snapshot records (§8.7.4), and the
//! durable job queue (§8.10). The reference oracle's verdict is rendered as evidence, never acted
//! on.
//!
//! Crate boundaries are defined in `docs/CLAUDE.md`; see
//! `docs/02-adr/001-repo-bootstrap-decisions.md` and `docs/02-adr/011-engine-only-pivot.md`.
