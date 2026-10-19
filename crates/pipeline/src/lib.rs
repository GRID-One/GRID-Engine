//! `grid-pipeline` — the orchestration crate: the engine façade of commands, queries and events.
//!
//! This crate is the `pipeline` crate of the DR-A8 target state (engine-spec §8.1 "Target end
//! state", ratified 2026-10-08; `docs/02-adr/011-engine-only-pivot.md` D11: an engine-only
//! repository whose public surface is a Rust library API, a headless CLI over it and versioned
//! outputs). It is the renamed `application` crate (P1-01). It owns:
//!
//! - the **library façade** of engine-spec §8.2–§8.4: typed command requests and the
//!   [`commands::EngineCommands`] trait (13 commands), typed queries, windowed response shapes and
//!   the [`queries::EngineQueries`] trait (14 queries), the [`events::EngineEvent`] enumeration
//!   (21 events) carrying identifiers only, the [`events::EventSubscriber`] trait and the
//!   in-process [`events::EventBus`];
//! - the **record serialization** of engine-output-contract §9–§10: the JSON record envelope that
//!   stamps the `grid.output` contract id and version, the reader that rejects an unknown MAJOR,
//!   and the export sidecar ([`serialization`]);
//! - from P1-02 onward, the orchestration of the §8.6 daily DAG and the durable-job execution of
//!   §8.10 behind the façade.
//!
//! # What P1-01 delivers and what it does not
//!
//! P1-01 delivers the façade as **types and traits only**. No §8.2 command and no §8.3 query has an
//! implementation in this crate: each is a trait method with no implementor, by design
//! (`docs/01-work-packages/p1-01-engine-skeleton.md`, "Observable outcome"). Nothing here computes
//! a projection, touches a database, reads a clock or opens a file. The implementors arrive with the
//! packages that own their stages (P1-02 onward); the CLI subcommands are P1-10's (engine-spec
//! §8.1 target table; §8.2 "completed in P1-10"), and the content of each Phase 2 output is the
//! package §10.2.6 names; `grid-cli` records the delivering package per subcommand.
//!
//! # Synchronous façade over an asynchronous interior
//!
//! Every façade trait method is a plain `fn`, not an `async fn`. The CLI is a one-shot process and
//! `grid-cli` MUST depend only on this crate and `grid-domain` (engine-spec §8.1 rule 4), so the
//! façade cannot require a consumer to own a runtime. From P1-02 on, the implementors of these
//! traits use Tokio internally for I/O and job coordination and a dedicated CPU pool
//! (`spawn_blocking` or Rayon) for mathematical work, never blocking the async runtime
//! (engine-spec §8.13); a façade method blocks its caller until the command is enqueued or the
//! query is answered, and that is the whole of its contract with the caller.
//!
//! # No business logic outside this crate's dependents
//!
//! Business logic lives behind this façade, in the crates it orchestrates; it never lives in the
//! CLI or in any presentation layer (engine-spec §1.1 item 2, §8.1 rule 2). The CLI parses
//! arguments, calls a façade method, renders the response and maps the outcome to an exit code.
//!
//! # Dependencies
//!
//! `grid-domain` (the DTOs), `serde` and `serde_json` (the record format, ADR-014 decision 3).
//! `crates/pipeline/tests/crate_boundaries.rs` enforces the workspace's dependency rules.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod commands;
pub mod events;
pub mod queries;
pub mod serialization;
