//! `grid` — the headless engine CLI, a thin adapter over `grid-pipeline` (engine-spec §8.1 rules
//! 2 and 4; DR-A8; ADR-011).
//!
//! The binary parses arguments ([`args`]), prints contract data that `grid-domain` and
//! `grid-pipeline` define, and maps every outcome to a process exit code through one function
//! ([`exit_code`]). It holds no business logic and depends on nothing but the orchestration crate
//! and `grid-domain`.
//!
//! P1-01 delivers the skeleton: `--version` (crate version plus the compile-time git SHA, or
//! `(git sha unrecorded)`; engine-spec §8.9), `--help`, `exit-codes`, and the recognition of
//! every §8.2 and §8.3 spelling, each of which exits with the usage code until the package that
//! delivers it does: P1-10 for every entry (engine-spec §8.1 target table and §8.2). For the five
//! entries that adapt a Phase 2 output, §10.2.6 names a different package for the content; that
//! conflict is an open owner decision, and the usage and the unavailable message show both
//! packages with the label "decision request" (the `content_by` field in [`args`]). The usage
//! footer records that the content behind the Phase 2 outputs arrives with the package that owns
//! it (§9.2.7, §10.2.6). No subcommand computes anything.

#![forbid(unsafe_code)]

mod args;

use std::io::{self, Write};

use grid_domain::{EXIT_INTERNAL, EXIT_USAGE};

use crate::args::Invocation;

/// The outcome of one invocation, before it becomes an exit code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    /// The request was served.
    Success,
    /// An unknown or not-yet-available subcommand (engine-output-contract §13 usage error).
    UsageError,
    /// The process could not write its own output.
    Internal,
}

/// The one place an outcome becomes a process exit code. A daily-update terminal state will map
/// here through `TerminalState::exit_code` once `grid update` exists (P1-10).
fn exit_code(outcome: Outcome) -> i32 {
    match outcome {
        Outcome::Success => 0,
        Outcome::UsageError => EXIT_USAGE,
        Outcome::Internal => EXIT_INTERNAL,
    }
}

/// Serves one argument list, writing to the given streams.
fn run<I, O, E>(arguments: I, out: &mut O, err: &mut E) -> Outcome
where
    I: IntoIterator<Item = String>,
    O: Write,
    E: Write,
{
    let result: io::Result<Outcome> = match args::parse(arguments) {
        Invocation::Version => writeln!(out, "{}", args::version_line()).map(|()| Outcome::Success),
        Invocation::Help => write!(out, "{}", args::usage()).map(|()| Outcome::Success),
        Invocation::ExitCodes => {
            write!(out, "{}", args::exit_code_table()).map(|()| Outcome::Success)
        }
        Invocation::Unavailable { surface } => {
            writeln!(err, "{}", args::unavailable_message(&surface)).map(|()| Outcome::UsageError)
        }
        Invocation::Unknown { tokens } => {
            writeln!(err, "{}", args::unknown_message(&tokens)).map(|()| Outcome::UsageError)
        }
    };
    match result.and_then(|outcome| out.flush().map(|()| outcome)) {
        Ok(outcome) => outcome,
        Err(_) => Outcome::Internal,
    }
}

fn main() {
    let stdout = io::stdout();
    let stderr = io::stderr();
    // `args_os`, not `args`: `std::env::args` panics (exit 101, a code engine-output-contract §13
    // does not define) on an argument that is not valid UTF-8. A lossy conversion turns such an
    // argument into one that matches no subcommand, so it is a usage error (exit 2).
    let outcome = run(
        std::env::args_os()
            .skip(1)
            .map(|arg| arg.to_string_lossy().into_owned()),
        &mut stdout.lock(),
        &mut stderr.lock(),
    );
    std::process::exit(exit_code(outcome));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn invoke(list: &[&str]) -> (Outcome, String, String) {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let outcome = run(list.iter().map(|s| (*s).to_owned()), &mut out, &mut err);
        (
            outcome,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    #[test]
    fn outcomes_map_to_the_contract_codes() {
        assert_eq!(exit_code(Outcome::Success), 0);
        assert_eq!(exit_code(Outcome::UsageError), 2);
        assert_eq!(exit_code(Outcome::Internal), 70);
    }

    #[test]
    fn version_help_and_errors_go_to_the_right_stream() {
        let (outcome, out, err) = invoke(&["--version"]);
        assert_eq!(outcome, Outcome::Success);
        assert_eq!(out, format!("{}\n", args::version_line()));
        let version = env!("CARGO_PKG_VERSION");
        let recorded = out
            .strip_prefix(&format!("grid {version} ("))
            .and_then(|rest| rest.strip_suffix(")\n"))
            .unwrap_or_else(|| panic!("unexpected version line {out:?}"));
        assert!(
            recorded == "git sha unrecorded" || (!recorded.is_empty() && !recorded.contains(' ')),
            "{out:?}"
        );
        assert!(err.is_empty());

        let (outcome, out, _) = invoke(&[]);
        assert_eq!(outcome, Outcome::Success);
        assert!(out.contains("query bands"));

        let (outcome, out, err) = invoke(&["update"]);
        assert_eq!(outcome, Outcome::UsageError);
        assert!(out.is_empty());
        assert!(err.contains("not available in this engine version"));

        let (outcome, _, err) = invoke(&["frobnicate"]);
        assert_eq!(outcome, Outcome::UsageError);
        assert!(err.contains("unknown subcommand"));
    }

    #[test]
    fn a_closed_output_stream_is_an_internal_error() {
        struct Closed;
        impl Write for Closed {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::Error::from(io::ErrorKind::BrokenPipe))
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let outcome = run(vec!["--version".to_owned()], &mut Closed, &mut Vec::new());
        assert_eq!(outcome, Outcome::Internal);
    }
}
