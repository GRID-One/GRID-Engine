//! The CLI integration test (P1-01, "Observable outcome"): runs the built `grid` binary and
//! asserts the four P1-01 behaviours: `--version`, the usage listing, the exit-code table, and
//! the usage exit for recognised-but-unavailable and unknown subcommands.

use std::process::{Command, Output};

use grid_domain::{TerminalState, EXIT_INTERNAL, EXIT_USAGE};

/// One expected usage line: the §8.2 or §8.3 CLI spelling, its library function, the work
/// package that delivers it (P1-10 for every entry: §8.1 target table and §8.2), and, for the
/// five Phase 2 outputs, the package §10.2.6 names for their content (P2-01, P2-02, P2-06,
/// P2-07). Which of the two owns those five subcommands is an open owner decision, so the usage
/// shows both with the label "decision request".
type Expected = (
    &'static str,
    &'static str,
    &'static str,
    Option<&'static str>,
);

/// Every §8.2 CLI spelling the usage must list (engine-spec §8.2, "CLI (indicative)").
const COMMANDS: [Expected; 13] = [
    ("update", "trigger_daily_update", "P1-10", None),
    ("project", "request_projection_run", "P1-10", None),
    ("rebuild", "request_model_rebuild", "P1-10", None),
    (
        "scoring add|set-default",
        "set_scoring_profile",
        "P1-10",
        None,
    ),
    (
        "availability import",
        "import_availability_overrides",
        "P1-10",
        Some("P2-02"),
    ),
    ("identity approve", "approve_identity_link", "P1-10", None),
    ("identity reject", "reject_identity_link", "P1-10", None),
    (
        "benchmark import",
        "import_benchmark_snapshot",
        "P1-10",
        None,
    ),
    ("model promote", "promote_candidate", "P1-10", None),
    ("model rollback", "rollback_to_snapshot", "P1-10", None),
    ("export", "export_projections", "P1-10", None),
    ("model configure", "set_model_config", "P1-10", None),
    ("backtest", "request_evaluation_run", "P1-10", None),
];

/// Every §8.3 CLI spelling the usage must list (engine-spec §8.3, "CLI (indicative)").
const QUERIES: [Expected; 14] = [
    ("query projections", "get_week_projections", "P1-10", None),
    (
        "query player",
        "get_player_projection_detail",
        "P1-10",
        None,
    ),
    (
        "query distribution",
        "get_projection_distribution",
        "P1-10",
        None,
    ),
    (
        "report changes",
        "get_projection_change_log",
        "P1-10",
        Some("P2-07"),
    ),
    ("status --freshness", "get_data_freshness", "P1-10", None),
    ("status", "get_ingestion_status", "P1-10", None),
    ("status", "get_training_status", "P1-10", None),
    ("identity queue", "get_identity_review_queue", "P1-10", None),
    (
        "report scorecard",
        "get_model_scorecard",
        "P1-10",
        Some("P2-06"),
    ),
    (
        "report benchmark",
        "get_benchmark_results",
        "P1-10",
        Some("P2-06"),
    ),
    (
        "report data-quality",
        "get_data_quality_report",
        "P1-10",
        Some("P2-01"),
    ),
    ("query ratings", "get_player_ratings", "P1-10", None),
    ("query bands", "get_confidence_bands", "P1-10", None),
    ("model status", "get_model_status", "P1-10", None),
];

/// The usage tag of one entry, as `args::availability_tag` renders it.
fn tag(delivered_by: &str, content_by: Option<&str>) -> String {
    match content_by {
        None => format!("available from {delivered_by}"),
        Some(content) => {
            format!("available from {delivered_by}; content {content} (decision request)")
        }
    }
}

/// The unavailable message of one entry, as `args::unavailable_message` renders it.
fn unavailable(spelling: &str, delivered_by: &str, content_by: Option<&str>) -> String {
    let mut message = format!(
        "grid {spelling}: not available in this engine version: delivered by work package \
         {delivered_by}"
    );
    if let Some(content) = content_by {
        message.push_str(&format!(
            "; content by work package {content} (decision request)"
        ));
    }
    message.push('\n');
    message
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
fn version_prints_grid_the_package_version_and_the_git_sha() {
    // engine-spec §8.9 build version: crate version plus git SHA. The SHA comes from the
    // compile-time `GRID_GIT_SHA`, which the release build (P1-11) sets; a build without it says
    // so. Both forms are accepted here.
    let output = grid(&["--version"]);
    assert_eq!(output.status.code(), Some(0));
    let out = stdout(&output);
    let recorded = out
        .strip_prefix(&format!("grid {} (", env!("CARGO_PKG_VERSION")))
        .and_then(|rest| rest.strip_suffix(")\n"))
        .unwrap_or_else(|| panic!("unexpected version line {out:?}"));
    assert!(
        recorded == "git sha unrecorded" || (!recorded.is_empty() && !recorded.contains(' ')),
        "{out:?}"
    );
    assert!(stderr(&output).is_empty());
}

#[test]
fn help_lists_every_command_and_query_spelling() {
    let help = grid(&["--help"]);
    assert_eq!(help.status.code(), Some(0));
    let text = stdout(&help);
    for (spelling, library, delivered_by, content_by) in COMMANDS.iter().chain(QUERIES.iter()) {
        let expected_tag = format!("[{library}; {}]", tag(delivered_by, *content_by));
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
    // The five Phase 2 outputs name their content package with the decision-request label.
    assert_eq!(
        text.lines()
            .filter(|line| line.contains(" (decision request)]"))
            .count(),
        5,
        "five entries carry the decision-request label:\n{text}"
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
    // (arguments, the spelling reported, the delivering package named, the content package).
    let cases: [(Vec<&str>, &str, &str, Option<&str>); 15] = [
        (vec!["update"], "update", "P1-10", None),
        (vec!["update", "--offline"], "update", "P1-10", None),
        (
            vec![
                "project", "--season", "2023", "--week", "5", "--lock", "sun",
            ],
            "project",
            "P1-10",
            None,
        ),
        (
            vec!["scoring", "set-default", "ppr"],
            "scoring add|set-default",
            "P1-10",
            None,
        ),
        (
            vec!["model", "promote", "42"],
            "model promote",
            "P1-10",
            None,
        ),
        (
            vec!["query", "projections"],
            "query projections",
            "P1-10",
            None,
        ),
        (
            vec!["status", "--freshness"],
            "status --freshness",
            "P1-10",
            None,
        ),
        // A bare `status` is reported under its own spelling, not the `--freshness` entry.
        (vec!["status"], "status", "P1-10", None),
        (
            vec!["report", "data-quality"],
            "report data-quality",
            "P1-10",
            Some("P2-01"),
        ),
        (
            vec!["report", "changes"],
            "report changes",
            "P1-10",
            Some("P2-07"),
        ),
        (
            vec!["report", "scorecard"],
            "report scorecard",
            "P1-10",
            Some("P2-06"),
        ),
        (
            vec!["report", "benchmark"],
            "report benchmark",
            "P1-10",
            Some("P2-06"),
        ),
        (vec!["identity", "queue"], "identity queue", "P1-10", None),
        (
            vec!["availability", "import", "x.csv"],
            "availability import",
            "P1-10",
            Some("P2-02"),
        ),
        (vec!["backtest"], "backtest", "P1-10", None),
    ];
    for (args, spelling, delivered_by, content_by) in cases {
        let output = grid(&args);
        assert_eq!(output.status.code(), Some(EXIT_USAGE), "{args:?}");
        assert!(stdout(&output).is_empty(), "{args:?}");
        let err = stderr(&output);
        assert_eq!(
            err,
            unavailable(spelling, delivered_by, content_by),
            "{args:?}"
        );
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
