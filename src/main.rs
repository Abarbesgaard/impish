mod check;
mod contract;
mod mutate;

use std::path::Path;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

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
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
    },
    /// Break the code a primitive covers and credit each kill to the
    /// obligations whose tests caught it; fail on an obligation that catches
    /// nothing. Needs cargo-mutants.
    Mutate {
        /// The primitive, a directory under contract/.
        primitive: String,
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
    },
}

/// How a run reports: text for people, or one JSON document on stdout.
#[derive(Clone, Copy, PartialEq, ValueEnum)]
enum Format {
    Text,
    Json,
}

/// The nearest directory, from `cwd` up, that has `contract/`.
fn root(cwd: &Path) -> anyhow::Result<&Path> {
    cwd.ancestors()
        .find(|dir| dir.join("contract").is_dir())
        .ok_or_else(|| anyhow::anyhow!("no contract/ in {} or above it", cwd.display()))
}

/// Exit 0 when the Contract holds, 1 when it doesn't, and 2 when impish
/// couldn't tell.
fn main() -> ExitCode {
    run().unwrap_or_else(|e| {
        eprintln!("Error: {e:?}");
        ExitCode::from(2)
    })
}

fn run() -> anyhow::Result<ExitCode> {
    let mut args: Vec<String> = std::env::args().collect();
    // Cargo runs `cargo impish …` as `cargo-impish impish …`.
    if args.get(1).map(String::as_str) == Some("impish") {
        args.remove(1);
    }
    let cwd = std::env::current_dir()?;
    match Cli::parse_from(args).command {
        Command::Check { write, format } => check::run(root(&cwd)?, write, format == Format::Json),
        Command::Mutate { primitive, format } => {
            mutate::run(root(&cwd)?, &primitive, format == Format::Json)
        }
    }
}
