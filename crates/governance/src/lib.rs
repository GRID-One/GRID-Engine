//! `grid-governance` — Model promotion, rollback, snapshot, job queue.
//!
//! The policy half of the engine's durable state, written as pure, synchronous functions over the
//! `grid-domain` types. Engine-spec §8.1 gives this crate "model states and the promotion gate
//! (§8.8), the pre-registered threshold registry, production pointers and snapshot records
//! (§8.7.4), and the durable job queue (§8.10)". Work package P1-02 D1 fixes the split: this
//! crate decides, `grid-persistence` executes compare-and-set writes in SQL and holds no business
//! logic, and `grid-pipeline` reads the store, asks this crate for a plan and has persistence
//! execute it inside one write transaction.
//!
//! # What this crate contains (P1-02)
//!
//! | Module | Content | Governing text |
//! |---|---|---|
//! | [`jobs`] | the job-status transition table, readiness over upstream keys, the retry, backoff and lease policy read from the engine settings, the stale-`RUNNING` decision, the idempotency key, global serialization | engine-spec §8.10, §8.11, §8.13; work package P1-02 D6–D9 |
//! | [`model_state`] | the model-state transition table of §8.8 with the cause each transition requires | engine-spec §8.8; work package P1-02 D13 |
//! | [`pointers`] | the promotion and rollback plans for the production pointers | engine-spec §8.7.4, §8.8, §8.13; work package P1-02 D14 |
//! | [`error`] | [`GovernanceError`] and its mapping to [`grid_domain::EngineError`] | engine-spec §6.8 rule 4; owner ruling R6 (P1-02) |
//!
//! # What this crate deliberately does not contain
//!
//! - **No I/O.** No file, database, clock, process or network access, no `tokio`, no `sqlx`
//!   (engine-spec §8.1 rule 3; work package P1-02 D1). Instants and durations are values the
//!   caller passes in.
//! - **No defaults for persisted policy.** The job settings are read from the store's
//!   `engine_settings` rows, which migration `0002_backbone.sql` seeds (work package P1-02 D8). A
//!   missing or malformed setting is a typed error, never a value supplied here (DR-B6;
//!   Appendix D item 8), so [`jobs::JobPolicy`] has no `Default`.
//! - **No promotion gate, threshold registry or snapshot command yet.** P1-02 delivers the
//!   primitives; the §8.8 gate, the §8.2 promote and rollback commands and automatic rollback
//!   belong to P2-07 (DR-C5). The reference oracle's verdict is rendered as evidence, never acted
//!   on.
//!
//! Crate boundaries are defined in `docs/CLAUDE.md`; see
//! `docs/02-adr/001-repo-bootstrap-decisions.md` and `docs/02-adr/011-engine-only-pivot.md`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod error;
pub mod jobs;
pub mod model_state;
pub mod pointers;

pub use error::GovernanceError;
