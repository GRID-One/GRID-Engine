//! The CLI integration test (P1-01, "Observable outcome"): runs the built `grid` binary and
//! asserts the four P1-01 behaviours: `--version`, the usage listing, the exit-code table, and
//! the usage exit for recognised-but-unavailable and unknown subcommands.

use std::process::{Command, Output};

use grid_domain::{TerminalState, EXIT_INTERNAL, EXIT_USAGE};

/// One expected usage line: the §8.2 or §8.3 CLI spelling, its library function, and the work
/// package engine-spec names as delivering it: P1-10 for every entry (§8.1 target table and
/// §8.2; the P1-01 orchestrator's ruling on the five Phase 2 outputs, whose content §10.2.6
/// assigns to P2-01, P2-02, P2-06 and P2-07 while the adapter stays P1-10's).
type Expected = (&'static str, &'static str, &'static str);

/// Every §8.2 CLI spelling the usage must list (engine-spec §8.2, "CLI (indicative)").
const COMMANDS: [Expected; 13] = [
    ("update", "trigger_daily_update", "P1-10"),
    ("project", "request_projection_run", "P1-10"),
    ("rebuild", "request_model_rebuild", "P1-10"),
    ("scoring add|set-default", "set_scoring_profile", "P1-10"),
    (
        "availability import",
        "import_availability_overrides",
        "P1-10",
    ),
    ("identity approve", "approve_identity_link", "P1-10"),
    ("identity reject", "reject_identity_link", "P1-10"),
    ("benchmark import", "import_benchmark_snapshot", "P1-10"),
    ("model promote", "promote_candidate", "P1-10"),
    ("model rollback", "rollback_to_snapshot", "P1-10"),
    ("export", "export_projections", "P1-10"),
    ("model configure", "set_model_config", "P1-10"),
    ("backtest", "request_evaluation_run", "P1-10"),
];

/// Every §8.3 CLI spelling the usage must list (engine-spec §8.3, "CLI (indicative)").
const QUERIES: [Expected; 14] = [
    ("query projections", "get_week_projections", "P1-10"),
    ("query player", "get_player_projection_detail", "P1-10"),
    ("query distribution", "get_projection_distribution", "P1-10"),
    ("report changes", "get_projection_change_log", "P1-10"),
    ("status --freshness", "get_data_freshness", "P1-10"),
    ("status", "get_ingestion_status", "P1-10"),
    ("status", "get_training_status", "P1-10"),
    ("identity queue", "get_identity_review_queue", "P1-10"),
    ("report scorecard", "get_model_scorecard", "P1-10"),
    ("report benchmark", "get_benchmark_results", "P1-10"),
    ("report data-quality", "get_data_quality_report", "P1-10"),
    ("query ratings", "get_player_ratings", "P1-10"),
    ("query bands", "get_confidence_bands", "P1-10"),
    ("model status", "get_model_status", "P1-10"),
];

/// The usage tag of one entry, as `args::availability_tag` renders it.
fn tag(delivered_by: &str) -> String {
    format!("available from {delivered_by}")
}

fn grid(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_grid"))
        .args(args)
        .output()
        .expect("the grid binary runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is UTF-8")
}

#[test]
fn version_prints_grid_and_the_package_version() {
    let output = grid(&["--version"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        stdout(&output),
        format!("grid {}\n", env!("CARGO_PKG_VERSION"))
    );
    assert!(stderr(&output).is_empty());
}

#[test]
fn help_lists_every_command_and_query_spelling() {
    let help = grid(&["--help"]);
    assert_eq!(help.status.code(), Some(0));
    let text = stdout(&help);
    for (spelling, library, delivered_by) in COMMANDS.iter().chain(QUERIES.iter()) {
        let expected_tag = format!("[{library}; {}]", tag(delivered_by));
        assert!(
            text.lines().any(|line| {
                line.trim_start().starts_with(spelling) && line.contains(&expected_tag)
            }),
            "usage lacks the line `{spelling} … {expected_tag}`:\n{text}"
        );
    }
    // Every command and query names its delivering package (P1-01 "Observable outcome").
    let named = text
        .lines()
        .filter(|line| line.contains("; available from "))
        .count();
    assert_eq!(
        named,
        13 + 14,
        "one tagged line per command and per query:\n{text}"
    );
    assert!(
        !text.contains("not available in this engine version]"),
        "no entry is left without a delivering package:\n{text}"
    );
    // One footer line keeps the engine-spec §10.2.6 nuance: the content behind some outputs
    // arrives with the package that owns it. The usage is plain ASCII (no section sign).
    assert!(text.is_ascii(), "{text}");
    let footers: Vec<&str> = text
        .lines()
        .filter(|line| line.contains("engine-spec 9.2.7 and 10.2.6"))
        .collect();
    assert_eq!(
        footers.len(),
        1,
        "one content-ownership footer line:\n{text}"
    );
    for package in ["P2-07", "P2-06", "P2-02", "P2-01"] {
        assert!(footers[0].contains(package), "{}", footers[0]);
    }
    assert!(
        footers[0].contains(&format!("exits {EXIT_USAGE} ")),
        "{}",
        footers[0]
    );
    assert!(text.contains("exit-codes"));
    // `help` and no arguments print the same listing and exit 0.
    for args in [vec!["help"], vec![]] {
        let other = grid(&args);
        assert_eq!(other.status.code(), Some(0), "{args:?}");
        assert_eq!(stdout(&other), text, "{args:?}");
    }
}

#[test]
fn exit_codes_prints_the_terminal_state_table() {
    let output = grid(&["exit-codes"]);
    assert_eq!(output.status.code(), Some(0));
    let text = stdout(&output);
    for state in TerminalState::ALL {
        let line = text
            .lines()
            .find(|line| line.split_whitespace().next() == Some(state.spec_name()))
            .unwrap_or_else(|| panic!("table lacks {}:\n{text}", state.spec_name()));
        let rendered = line.split_whitespace().nth(1).unwrap();
        match state.exit_code() {
            Some(code) => assert_eq!(rendered, code.to_string(), "{line}"),
            None => assert_eq!(rendered, "none", "{line}"),
        }
    }
    for (label, code) in [
        ("USAGE_ERROR", EXIT_USAGE),
        ("INTERNAL_ERROR", EXIT_INTERNAL),
    ] {
        let line = text
            .lines()
            .find(|line| line.split_whitespace().next() == Some(label))
            .unwrap_or_else(|| panic!("table lacks {label}:\n{text}"));
        assert_eq!(
            line.split_whitespace().nth(1),
            Some(code.to_string().as_str()),
            "{line}"
        );
    }
}

#[test]
fn a_recognised_but_unavailable_subcommand_exits_with_the_usage_code() {
    // (arguments, the spelling reported, the delivering package named).
    let cases: [(Vec<&str>, &str, &str); 13] = [
        (vec!["update"], "update", "P1-10"),
        (vec!["update", "--offline"], "update", "P1-10"),
        (
            vec![
                "project", "--season", "2023", "--week", "5", "--lock", "sun",
            ],
            "project",
            "P1-10",
        ),
        (
            vec!["scoring", "set-default", "ppr"],
            "scoring add|set-default",
            "P1-10",
        ),
        (vec!["model", "promote", "42"], "model promote", "P1-10"),
        (vec!["query", "projections"], "query projections", "P1-10"),
        (vec!["status", "--freshness"], "status --freshness", "P1-10"),
        // A bare `status` is reported under its own spelling, not the `--freshness` entry.
        (vec!["status"], "status", "P1-10"),
        (
            vec!["report", "data-quality"],
            "report data-quality",
            "P1-10",
        ),
        (vec!["report", "changes"], "report changes", "P1-10"),
        (vec!["identity", "queue"], "identity queue", "P1-10"),
        (
            vec!["availability", "import", "x.csv"],
            "availability import",
            "P1-10",
        ),
        (vec!["backtest"], "backtest", "P1-10"),
    ];
    for (args, spelling, delivered_by) in cases {
        let output = grid(&args);
        assert_eq!(output.status.code(), Some(EXIT_USAGE), "{args:?}");
        assert!(stdout(&output).is_empty(), "{args:?}");
        let err = stderr(&output);
        let expected = format!(
            "grid {spelling}: not available in this engine version: delivered by work package \
             {delivered_by}\n"
        );
        assert_eq!(err, expected, "{args:?}");
    }
}

#[test]
fn an_unknown_subcommand_exits_with_the_usage_code() {
    for args in [
        vec!["frobnicate"],
        vec!["model"],
        vec!["model", "frob"],
        vec!["query"],
        vec!["--bogus"],
    ] {
        let output = grid(&args);
        assert_eq!(output.status.code(), Some(EXIT_USAGE), "{args:?}");
        assert!(stdout(&output).is_empty(), "{args:?}");
        let err = stderr(&output);
        assert!(err.contains("unknown subcommand"), "{args:?}: {err}");
    }
}

/// An argument that is not valid UTF-8 is a usage error (exit 2), not a panic (exit 101, a code
/// engine-output-contract §13 does not define); control characters are echoed escaped.
#[cfg(unix)]
#[test]
fn non_utf8_and_control_character_arguments_are_usage_errors() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let output = Command::new(env!("CARGO_BIN_EXE_grid"))
        .arg(OsStr::from_bytes(b"\xff"))
        .output()
        .expect("the grid binary runs");
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("unknown subcommand"));

    let output = grid(&["\u{1b}[31mred"]);
    assert_eq!(output.status.code(), Some(2));
    let err = stderr(&output);
    assert!(
        !err.contains('\u{1b}'),
        "the escape byte is not echoed raw: {err:?}"
    );
    assert!(err.contains("\\u{1b}[31mred"));
}
