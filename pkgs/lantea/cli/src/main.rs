//! `lantea`: plain-language access to a Lantea OS system.
//!
//! Every command prints plain text by default, JSON with `--json`, and the
//! real commands it would run with `--explain`. Exit codes: 0 success,
//! 1 failed or refused, 2 usage error (clap's own exit code for bad usage).

use clap::{Parser, Subcommand};
use serde::Serialize;

#[derive(Parser)]
#[command(
    name = "lantea",
    // The CLI version, then the name of the edition this build belongs to.
    // LANTEA_EDITION_NAME comes from modules/core/discipline/edition.nix, set
    // by the Nix package and the development shell.
    version = concat!(
        env!("CARGO_PKG_VERSION"),
        " (",
        env!("LANTEA_EDITION_NAME"),
        ")"
    ),
    about = "Plain-language access to a Lantea OS system",
    arg_required_else_help = true
)]
struct Cli {
    /// Print machine-readable JSON instead of plain text.
    #[arg(long, global = true)]
    json: bool,

    /// Print the real commands instead of running them.
    #[arg(long, global = true)]
    explain: bool,

    /// Answer yes to confirmation questions.
    #[arg(long, global = true)]
    yes: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show the current condition of the system.
    Condition,
}

#[derive(Serialize)]
struct ConditionReport {
    state: &'static str,
}

fn condition(cli: &Cli) {
    if cli.explain {
        println!("No commands are run yet: the condition daemon arrives in Phase 3.");
    } else if cli.json {
        let report = ConditionReport { state: "unknown" };
        println!(
            "{}",
            serde_json::to_string(&report).expect("a fixed report serialises")
        );
    } else {
        println!("Condition is not yet known.");
    }
}

fn main() {
    let cli = Cli::parse();
    // --yes has nothing to confirm yet; it is accepted so scripts can pass it.
    let _ = cli.yes;
    match cli.command {
        Command::Condition => condition(&cli),
    }
}
