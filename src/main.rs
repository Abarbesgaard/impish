mod check;
mod contract;

use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// The Contract gate for Rust, after Impatient Programming.
#[derive(Parser)]
#[command(name = "cargo impish", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Hold the Contract under contract/ to its tests.
    Check {
        /// Write contract/coverage.json, the Contract's published coverage.
        #[arg(long)]
        write: bool,
    },
}

fn main() -> anyhow::Result<ExitCode> {
    let mut args: Vec<String> = std::env::args().collect();
    // Cargo runs `cargo impish …` as `cargo-impish impish …`.
    if args.get(1).map(String::as_str) == Some("impish") {
        args.remove(1);
    }
    match Cli::parse_from(args).command {
        Command::Check { write } => check::run(&std::env::current_dir()?, write),
    }
}
