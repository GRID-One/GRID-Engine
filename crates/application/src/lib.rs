//! `grid-application` — Commands, queries, events, app services. Orchestrates crates.
//!
//! This crate is the engine facade: the Rust library API of commands, queries and events
//! (`engine-spec.md` §8.2–§8.4) and the orchestration of the daily pipeline DAG (§8.6). "App
//! services" are the application services behind that library API. This repository has no user
//! interface (ADR-011); the headless CLI is a thin adapter over this API.
//!
//! P1-01 renames this crate to `pipeline` and adds the `grid-cli` binary over it (DR-A8, pending
//! owner ratification). Crate boundaries are defined in `docs/CLAUDE.md`; see
//! `docs/02-adr/001-repo-bootstrap-decisions.md` and `docs/02-adr/011-engine-only-pivot.md`.
