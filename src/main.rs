mod check;
mod contract;

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
    /// Check that every obligation under contract/ has a test.
    Check,
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
        Command::Check => check::run(root(&cwd)?),
    }
}
