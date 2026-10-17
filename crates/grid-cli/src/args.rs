//! Argument handling for the `grid` binary: hand-rolled over `std::env::args`, no parser crate
//! (one needs approval in the package that first requires it, P1-10; engine-spec §14.2).
//!
//! This module recognises the §8.2 command spellings and the §8.3 query spellings of
//! `engine-spec.md` (the indicative spellings; `docs/03-contracts/engine-output-contract.md`
//! freezes them in P1-10), the `--version`, `--help` and `exit-codes` flags, and nothing else. It
//! holds no business logic: it classifies the argument list and renders two tables whose content
//! comes from `grid-domain` and `grid-pipeline`.

use std::fmt::Write as _;

use grid_domain::{TerminalState, EXIT_INTERNAL, EXIT_USAGE};

/// One entry of the CLI surface: a subcommand spelling, the §8.2/§8.3 library function it
/// adapts, a one-line summary, and the work package that delivers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Surface {
    /// The indicative CLI spelling (engine-spec §8.2, §8.3). Tokens separated by spaces; a token
    /// of the form `a|b` is a set of alternatives; a token starting with `--` is a flag that must
    /// appear somewhere in the arguments for the spelling to match, so that a bare `status` is
    /// never read as `status --freshness`.
    pub spelling: &'static str,
    /// The library function name, from `grid_pipeline::commands::COMMAND_NAMES` or
    /// `grid_pipeline::queries::QUERY_NAMES`.
    pub library: &'static str,
    /// A one-line summary for the usage listing.
    pub summary: &'static str,
    /// The work package that delivers this subcommand, as engine-spec names it: `P1-10`, for
    /// every entry. The §8.1 target-state table assigns the `grid-cli` crate "P1-01 (skeleton);
    /// P1-10 (subcommands, reports, export)", §8.2 says the spellings are "completed in P1-10",
    /// and the P1-01 work package's observable outcome reads "P1-10 for all of them".
    ///
    /// The §10.2.6 table (with §5.6 "Delivery") assigns each Phase 2 *output* to the package
    /// that owns its content: availability review P2-02, projection change log P2-07, model
    /// scorecard and benchmark results P2-06, data-quality report P2-01. The first draft of this
    /// table inferred from that an owner per subcommand for the five entries that adapt those
    /// outputs (`availability import`, `report changes`, `report scorecard`, `report benchmark`,
    /// `report data-quality`) and raised it as a decision request. The P1-01 orchestrator ruled
    /// (`docs/01-work-packages/p1-01-engine-skeleton.md`, Amendments row 10): §8.1 is the section
    /// that assigns the crate's work and §8.2 the section that assigns the spellings, so they
    /// outrank the inference from §10.2.6, which assigns content, not the adapter; both are
    /// engine-spec, so the ruling is a reading of one authority, not a conflict between two. The
    /// CLI adapter for every §8.2 command and §8.3 query is P1-10's, and this field is `P1-10`
    /// uniformly. The content nuance survives as the one footer line of [`usage`]. Never empty.
    pub delivered_by: &'static str,
}

/// The §8.2 commands, in table order.
pub const COMMANDS: [Surface; 13] = [
    Surface {
        spelling: "update",
        library: "trigger_daily_update",
        summary: "one-shot daily update of the pipeline [--offline]",
        delivered_by: "P1-10",
    },
    Surface {
        spelling: "project",
        library: "request_projection_run",
        summary: "produce a prediction version --season S --week W --lock {thu,sun,operational}",
        delivered_by: "P1-10",
    },
    Surface {
        spelling: "rebuild",
        library: "request_model_rebuild",
        summary: "rebuild a model family into a new candidate version <model-family>",
        delivered_by: "P1-10",
    },
    Surface {
        spelling: "scoring add|set-default",
        library: "set_scoring_profile",
        summary: "create a versioned affine scoring profile <profile>",
        delivered_by: "P1-10",
    },
    Surface {
        spelling: "availability import",
        library: "import_availability_overrides",
        summary: "import timestamped availability overrides <file>",
        delivered_by: "P1-10",
    },
    Surface {
        spelling: "identity approve",
        library: "approve_identity_link",
        summary: "approve an identity review item <review-id> <player-id>",
        delivered_by: "P1-10",
    },
    Surface {
        spelling: "identity reject",
        library: "reject_identity_link",
        summary: "reject an identity review item <review-id>",
        delivered_by: "P1-10",
    },
    Surface {
        spelling: "benchmark import",
        library: "import_benchmark_snapshot",
        summary: "store a benchmark snapshot for evaluation only <provider> <file> --observed-at T",
        delivered_by: "P1-10",
    },
    Surface {
        spelling: "model promote",
        library: "promote_candidate",
        summary: "promote a validated candidate (human gate) <model-version>",
        delivered_by: "P1-10",
    },
    Surface {
        spelling: "model rollback",
        library: "rollback_to_snapshot",
        summary: "move production pointers back to a snapshot <snapshot-id>",
        delivered_by: "P1-10",
    },
    Surface {
        spelling: "export",
        library: "export_projections",
        summary: "export projections and quantiles with a JSON sidecar",
        delivered_by: "P1-10",
    },
    Surface {
        spelling: "model configure",
        library: "set_model_config",
        summary: "create a model configuration and rebuild <model-family> <config-file>",
        delivered_by: "P1-10",
    },
    Surface {
        spelling: "backtest",
        library: "request_evaluation_run",
        summary: "run a rolling-origin evaluation",
        delivered_by: "P1-10",
    },
];

/// The §8.3 queries, in table order.
pub const QUERIES: [Surface; 14] = [
    Surface {
        spelling: "query projections",
        library: "get_week_projections",
        summary: "the weekly projection table",
        delivered_by: "P1-10",
    },
    Surface {
        spelling: "query player",
        library: "get_player_projection_detail",
        summary: "one player-week in detail",
        delivered_by: "P1-10",
    },
    Surface {
        spelling: "query distribution",
        library: "get_projection_distribution",
        summary: "the fantasy-point distribution outputs",
        delivered_by: "P1-10",
    },
    Surface {
        spelling: "report changes",
        library: "get_projection_change_log",
        summary: "prior versus current projection with change attribution",
        delivered_by: "P1-10",
    },
    Surface {
        spelling: "status --freshness",
        library: "get_data_freshness",
        summary: "per-source freshness against provider thresholds",
        delivered_by: "P1-10",
    },
    Surface {
        spelling: "status",
        library: "get_ingestion_status",
        summary: "last successful update, terminal state, per-source results",
        delivered_by: "P1-10",
    },
    Surface {
        spelling: "status",
        library: "get_training_status",
        summary: "current training status and running jobs",
        delivered_by: "P1-10",
    },
    Surface {
        spelling: "identity queue",
        library: "get_identity_review_queue",
        summary: "open identity review items",
        delivered_by: "P1-10",
    },
    Surface {
        spelling: "report scorecard",
        library: "get_model_scorecard",
        summary: "PB-MAE, MAE, RMSE, rank accuracy, Brier and coverage by week and position",
        delivered_by: "P1-10",
    },
    Surface {
        spelling: "report benchmark",
        library: "get_benchmark_results",
        summary: "provider comparison with confidence intervals (no raw competitor rows)",
        delivered_by: "P1-10",
    },
    Surface {
        spelling: "report data-quality",
        library: "get_data_quality_report",
        summary: "stale sources, unresolved identities, quarantined rows, missing context",
        delivered_by: "P1-10",
    },
    Surface {
        spelling: "query ratings",
        library: "get_player_ratings",
        summary: "the GRID signal: RAPM, Layer-1' credit, latent states, net strength, matchup",
        delivered_by: "P1-10",
    },
    Surface {
        spelling: "query bands",
        library: "get_confidence_bands",
        summary: "band data from the one-step predictive variance",
        delivered_by: "P1-10",
    },
    Surface {
        spelling: "model status",
        library: "get_model_status",
        summary: "model versions per family, states, production pointers, last validation",
        delivered_by: "P1-10",
    },
];

/// What an argument list asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Invocation {
    /// `--version` / `-V`: print `grid <version>`.
    Version,
    /// `--help` / `-h` / `help` / no arguments: print the usage listing.
    Help,
    /// `exit-codes`: print the terminal-state exit-code table.
    ExitCodes,
    /// A recognised §8.2 or §8.3 spelling that this engine version does not deliver.
    Unavailable {
        /// The surface entry matched.
        surface: Surface,
    },
    /// A token sequence that matches nothing.
    Unknown {
        /// The arguments that failed to match, joined by spaces.
        tokens: String,
    },
}

/// Classifies the arguments after the program name.
pub fn parse<I>(args: I) -> Invocation
where
    I: IntoIterator<Item = String>,
{
    let args: Vec<String> = args.into_iter().collect();
    let Some(first) = args.first() else {
        return Invocation::Help;
    };
    match first.as_str() {
        "--version" | "-V" => return Invocation::Version,
        "--help" | "-h" | "help" => return Invocation::Help,
        "exit-codes" => return Invocation::ExitCodes,
        _ => {}
    }
    let matched = COMMANDS
        .iter()
        .chain(QUERIES.iter())
        .find(|surface| spelling_matches(surface.spelling, &args));
    match matched {
        Some(surface) => Invocation::Unavailable { surface: *surface },
        None => Invocation::Unknown {
            tokens: args.join(" "),
        },
    }
}

/// Whether `args` begins with every non-flag token of `spelling`, each alternative set (`a|b`)
/// matching either alternative, **and** contains every flag token of `spelling`. The flag rule
/// is what keeps `status` from matching the earlier table entry `status --freshness`: the
/// entries are tried in table order, and an entry with a flag the arguments lack does not match.
fn spelling_matches(spelling: &str, args: &[String]) -> bool {
    let (flags, words): (Vec<&str>, Vec<&str>) = spelling
        .split_whitespace()
        .partition(|t| t.starts_with("--"));
    words.len() <= args.len()
        && words
            .iter()
            .zip(args)
            .all(|(token, arg)| token.split('|').any(|alt| alt == arg))
        && flags.iter().all(|flag| args.iter().any(|arg| arg == flag))
}

/// The `--version` line.
pub fn version_line() -> String {
    format!("grid {}", env!("CARGO_PKG_VERSION"))
}

/// The usage listing: every §8.2 command spelling and every §8.3 query spelling, each tagged
/// with the work package that delivers it (P1-10 for all of them: engine-spec §8.1, §8.2; see
/// [`Surface::delivered_by`]), and one footer line recording that the content behind the Phase 2
/// outputs arrives with the package that owns it (§9.2.7, §10.2.6). The output is plain ASCII.
pub fn usage() -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{}", version_line());
    let _ = writeln!(
        out,
        "The GRID engine: a headless CLI over the grid-pipeline library API."
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "Usage: grid <subcommand> [arguments]");
    let _ = writeln!(out, "       grid --version | --help | exit-codes");
    let _ = writeln!(out);
    let width = COMMANDS
        .iter()
        .chain(QUERIES.iter())
        .map(|s| s.spelling.len())
        .max()
        .unwrap_or(0);
    let _ = writeln!(out, "Commands (engine-spec 8.2):");
    for surface in &COMMANDS {
        push_surface(&mut out, surface, width);
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "Queries (engine-spec 8.3):");
    for surface in &QUERIES {
        push_surface(&mut out, surface, width);
    }
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "Other: exit-codes (the terminal-state exit-code table), --version, --help"
    );
    let _ = writeln!(
        out,
        "Every subcommand above is P1-10's and exits {EXIT_USAGE} with a message on stderr \
         until P1-10 delivers it; the content behind some outputs arrives with the package that \
         owns it (change log and audit report P2-07; benchmark results P2-06; live availability \
         adapters P2-02; the Phase 2 data-quality additions P2-01; engine-spec 9.2.7 and 10.2.6)."
    );
    out
}

fn push_surface(out: &mut String, surface: &Surface, width: usize) {
    let _ = writeln!(
        out,
        "  {:<width$}  {}  [{}; {}]",
        surface.spelling,
        surface.summary,
        surface.library,
        availability_tag(surface),
        width = width
    );
}

/// The availability tag of a surface entry in the usage listing.
fn availability_tag(surface: &Surface) -> String {
    format!("available from {}", surface.delivered_by)
}

/// The terminal-state exit-code table of engine-output-contract §13, rendered by iterating the
/// domain enum; no code is written here.
pub fn exit_code_table() -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "Terminal state exit codes (engine-spec 8.6.4; engine-output-contract 13)"
    );
    let width = TerminalState::ALL
        .iter()
        .map(|s| s.spec_name().len())
        .max()
        .unwrap_or(0);
    for state in TerminalState::ALL {
        match state.exit_code() {
            Some(code) => {
                let _ = writeln!(
                    out,
                    "  {:<width$}  {code}",
                    state.spec_name(),
                    width = width
                );
            }
            None => {
                let _ = writeln!(
                    out,
                    "  {:<width$}  none (not terminal)",
                    state.spec_name(),
                    width = width
                );
            }
        }
    }
    let _ = writeln!(
        out,
        "  {:<width$}  {EXIT_USAGE}",
        "USAGE_ERROR",
        width = width
    );
    let _ = writeln!(
        out,
        "  {:<width$}  {EXIT_INTERNAL}",
        "INTERNAL_ERROR",
        width = width
    );
    out
}

/// The message for a recognised-but-unavailable subcommand: it names the delivering work
/// package ([`Surface::delivered_by`]: P1-10 for every entry).
pub fn unavailable_message(surface: &Surface) -> String {
    format!(
        "grid {}: not available in this engine version: delivered by work package {}",
        surface.spelling, surface.delivered_by
    )
}

/// The message for an unknown subcommand.
pub fn unknown_message(tokens: &str) -> String {
    format!("grid: unknown subcommand `{tokens}` (run `grid --help` for the subcommand list)")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn surface_pairs_every_library_function_in_table_order() {
        let commands: Vec<&str> = COMMANDS.iter().map(|s| s.library).collect();
        assert_eq!(commands, grid_pipeline::commands::COMMAND_NAMES);
        let queries: Vec<&str> = QUERIES.iter().map(|s| s.library).collect();
        assert_eq!(queries, grid_pipeline::queries::QUERY_NAMES);
    }

    #[test]
    fn flags_and_bare_invocations() {
        assert_eq!(parse(args(&[])), Invocation::Help);
        assert_eq!(parse(args(&["--help"])), Invocation::Help);
        assert_eq!(parse(args(&["help"])), Invocation::Help);
        assert_eq!(parse(args(&["--version"])), Invocation::Version);
        assert_eq!(parse(args(&["exit-codes"])), Invocation::ExitCodes);
    }

    fn unavailable(args: &[&str]) -> Surface {
        match parse(self::tests::args(args)) {
            Invocation::Unavailable { surface } => surface,
            other => panic!("{args:?} parsed as {other:?}"),
        }
    }

    #[test]
    fn recognised_spellings_are_unavailable_and_others_unknown() {
        assert_eq!(unavailable(&["update", "--offline"]).spelling, "update");
        assert_eq!(
            unavailable(&["scoring", "set-default", "ppr"]).spelling,
            "scoring add|set-default"
        );
        assert_eq!(
            unavailable(&["status", "--freshness"]).spelling,
            "status --freshness"
        );
        // A bare `status` is query 6, not the `--freshness` entry that precedes it in the table.
        let status = unavailable(&["status"]);
        assert_eq!(status.spelling, "status");
        assert_eq!(status.library, "get_ingestion_status");
        assert_eq!(unavailable(&["model", "status"]).spelling, "model status");
        assert_eq!(
            parse(args(&["model"])),
            Invocation::Unknown {
                tokens: "model".into()
            }
        );
        assert_eq!(
            parse(args(&["model", "frob"])),
            Invocation::Unknown {
                tokens: "model frob".into()
            }
        );
        assert_eq!(
            parse(args(&["--bogus"])),
            Invocation::Unknown {
                tokens: "--bogus".into()
            }
        );
    }

    #[test]
    fn tables_render_from_the_domain_enum() {
        let table = exit_code_table();
        for state in TerminalState::ALL {
            assert!(table.contains(state.spec_name()), "{}", state.spec_name());
        }
        let learning = table
            .lines()
            .find(|line| line.split_whitespace().next() == Some("LEARNING_FAILURE"))
            .expect("LEARNING_FAILURE row");
        assert_eq!(learning.split_whitespace().nth(1), Some("40"));
        assert!(table.contains("none (not terminal)"));
        let usage = usage();
        for surface in COMMANDS.iter().chain(QUERIES.iter()) {
            assert!(usage.contains(surface.spelling), "{}", surface.spelling);
            assert!(usage.contains(surface.library), "{}", surface.library);
        }
    }

    #[test]
    fn delivery_is_p1_10_for_every_entry() {
        // engine-spec §8.1 target table and §8.2: the CLI adapter for every §8.2 command and
        // §8.3 query is P1-10's (the P1-01 orchestrator's ruling; see `Surface::delivered_by`).
        for surface in COMMANDS.iter().chain(QUERIES.iter()) {
            assert_eq!(surface.delivered_by, "P1-10", "{}", surface.library);
            assert_eq!(
                unavailable_message(surface),
                format!(
                    "grid {}: not available in this engine version: delivered by work package \
                     P1-10",
                    surface.spelling
                )
            );
        }
        assert_eq!(COMMANDS.len() + QUERIES.len(), 13 + 14);
    }

    #[test]
    fn usage_footer_keeps_the_content_ownership_nuance_in_plain_ascii() {
        // engine-spec §10.2.6 (with §9.2.7): the content behind the Phase 2 outputs arrives with
        // the package that owns it. One footer line carries it, and the usage is plain ASCII
        // with no section sign.
        let usage = usage();
        assert!(usage.is_ascii(), "{usage}");
        let footer = usage
            .lines()
            .find(|line| line.contains("engine-spec 9.2.7 and 10.2.6"))
            .unwrap_or_else(|| panic!("usage lacks the content-ownership footer:\n{usage}"));
        for package in ["P2-07", "P2-06", "P2-02", "P2-01"] {
            assert!(footer.contains(package), "{footer}");
        }
        assert!(footer.contains(&format!("exits {EXIT_USAGE} ")), "{footer}");
        assert_eq!(
            usage.lines().filter(|line| line.contains("P2-0")).count(),
            1,
            "exactly one footer line names a Phase 2 package:\n{usage}"
        );
        assert_eq!(
            usage
                .lines()
                .filter(|line| line.contains("available from P1-10"))
                .count(),
            13 + 14
        );
    }
}
