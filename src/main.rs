mod check;
mod contract;
mod mutate;

use std::path::Path;
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
    /// Break the code a primitive covers and credit each kill to the
    /// obligations whose tests caught it; fail on an obligation that catches
    /// nothing. Needs cargo-mutants.
    Mutate {
        /// The primitive, a directory under contract/.
        primitive: String,
    },
}

/// The nearest directory, from `cwd` up, that has `contract/`.
fn root(cwd: &Path) -> anyhow::Result<&Path> {
    cwd.ancestors()
        .find(|dir| dir.join("contract").is_dir())
        .ok_or_else(|| anyhow::anyhow!("no contract/ in {} or above it", cwd.display()))
}

fn main() -> anyhow::Result<ExitCode> {
    let mut args: Vec<String> = std::env::args().collect();
    // Cargo runs `cargo impish …` as `cargo-impish impish …`.
    if args.get(1).map(String::as_str) == Some("impish") {
        args.remove(1);
    }
    let cwd = std::env::current_dir()?;
    match Cli::parse_from(args).command {
        Command::Check { write } => check::run(root(&cwd)?, write),
        Command::Mutate { primitive } => mutate::run(root(&cwd)?, &primitive),
    }
}
