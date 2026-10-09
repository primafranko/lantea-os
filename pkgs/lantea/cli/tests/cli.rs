//! Exact output and exit codes of the `lantea` command.

use std::process::{Command, Output};

fn lantea(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lantea"))
        .args(args)
        .output()
        .expect("the lantea binary runs")
}

fn assert_run(args: &[&str], code: i32, stdout: &str) -> String {
    let out = lantea(args);
    let stderr = String::from_utf8(out.stderr).expect("stderr is UTF-8");
    assert_eq!(out.status.code(), Some(code), "lantea {args:?}: {stderr}");
    assert_eq!(
        String::from_utf8(out.stdout).expect("stdout is UTF-8"),
        stdout,
        "lantea {args:?}"
    );
    stderr
}

const UNKNOWN: &str = "Condition is not yet known.\n";
const UNKNOWN_JSON: &str = "{\"state\":\"unknown\"}\n";
const EXPLAIN: &str = "No commands are run yet: the condition daemon arrives in Phase 3.\n";

#[test]
fn version_names_the_edition() {
    assert_run(&["--version"], 0, "lantea 0.1.0 (Harbour)\n");
}

#[test]
fn bare_lantea_prints_usage_to_stderr_and_exits_2() {
    let stderr = assert_run(&[], 2, "");
    assert!(stderr.contains("Usage:"), "{stderr}");
}

#[test]
fn condition_plain() {
    assert_run(&["condition"], 0, UNKNOWN);
}

#[test]
fn condition_json_after_and_before_the_subcommand() {
    assert_run(&["condition", "--json"], 0, UNKNOWN_JSON);
    assert_run(&["--json", "condition"], 0, UNKNOWN_JSON);
}

#[test]
fn condition_explain_after_and_before_the_subcommand() {
    assert_run(&["condition", "--explain"], 0, EXPLAIN);
    assert_run(&["--explain", "condition"], 0, EXPLAIN);
}

#[test]
fn yes_is_a_global_flag() {
    assert_run(&["condition", "--yes"], 0, UNKNOWN);
    assert_run(&["--yes", "condition"], 0, UNKNOWN);
}

#[test]
fn unknown_subcommand_is_a_usage_error() {
    assert_run(&["no-such-command"], 2, "");
}
