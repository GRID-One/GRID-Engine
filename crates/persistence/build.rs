//! Build script of `grid-persistence` (ADR-015 D1).
//!
//! `sqlx::migrate!("../../migrations")` embeds the migration files at compile time. Cargo does
//! not know about that input, so without this line a new or changed migration would not rebuild
//! the crate and the binary would carry a stale migration list. The line tells cargo to re-run
//! the build, and so re-embed the migrations, whenever the directory changes.

fn main() {
    println!("cargo:rerun-if-changed=../../migrations");
}
