mod check;
mod code;
mod conditions;
mod contract;
mod decisions;
mod mutate;
mod regen;

use std::path::{Path, PathBuf};
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
        /// List every function in a src/ directory that no covers line claims.
        #[arg(long)]
        outside: bool,
    },
    /// Break the code a primitive covers and credit each kill to the
    /// obligations whose tests caught it; fail on an obligation that catches
    /// nothing. Needs cargo-mutants.
    Mutate {
        /// The primitive, a directory under contract/; every primitive that
        /// covers code when left out.
        primitive: Option<String>,
        /// Mutate only the covered lines changed since the merge base with
        /// this ref, and all of a primitive whose Contract or tests changed.
        #[arg(long, value_name = "REF", conflicts_with = "write")]
        since: Option<String>,
        /// With --since, mutate only the changed covered lines, even of a
        /// primitive whose Contract or tests changed: quicker, but the lines
        /// they didn't change aren't tried against the changed tests.
        #[arg(long, requires = "since")]
        only_lines: bool,
        /// Run every test, not only the Contract's: name each outside test
        /// that catches a mutant the Contract misses, and, mutating every
        /// primitive, fail on each whose catches the Contract all makes.
        #[arg(long)]
        whole_suite: bool,
        /// How many mutants to try at once, each in its own copy of the
        /// project; half the cores when left out.
        #[arg(short, long, value_name = "N", value_parser = clap::builder::RangedU64ValueParser::<usize>::new().range(1..))]
        jobs: Option<usize>,
        /// List the mutants the run would try, and which are declared, and
        /// build nothing.
        #[arg(long, conflicts_with = "write")]
        dry_run: bool,
        /// How many times the baseline's run time a mutant gets before it
        /// counts as timed out.
        #[arg(long, value_name = "X", default_value_t = 5.0, value_parser = positive)]
        timeout_coefficient: f64,
        /// Record in contract/coverage.json whether the primitive's mutate is
        /// met; `check` fails once what it rests on changes.
        #[arg(long)]
        write: bool,
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
    },
    /// Re-check the conditions of every decision record under contract/, and
    /// fail when one no longer holds.
    Conditions,
    /// Write a pack to rebuild a primitive from its Contract alone: the
    /// intents, obligations, decision records, contract tests and stubs of
    /// the covered functions, without their code.
    Regen {
        /// The primitive, a directory under contract/.
        primitive: String,
        /// Where to write the pack; target/impish/regen/<primitive> when left
        /// out. Only a previous pack there is replaced.
        #[arg(long, value_name = "DIR")]
        out: Option<PathBuf>,
    },
}

/// How a run reports: text for people, or one JSON document on stdout.
#[derive(Clone, Copy, PartialEq, ValueEnum)]
enum Format {
    Text,
    Json,
}

/// A finite number above zero.
fn positive(s: &str) -> Result<f64, String> {
    match s.parse::<f64>() {
        Ok(x) if x.is_finite() && x > 0.0 => Ok(x),
        _ => Err(format!("{s} is not a number above zero")),
    }
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
        Command::Check {
            write,
            format,
            outside,
        } => check::run(root(&cwd)?, write, format == Format::Json, outside),
        Command::Conditions => conditions::run(root(&cwd)?),
        Command::Regen { primitive, out } => {
            regen::run(root(&cwd)?, &primitive, out.map(|o| cwd.join(o)).as_deref())
        }
        Command::Mutate {
            primitive,
            since,
            only_lines,
            whole_suite,
            jobs,
            dry_run,
            timeout_coefficient,
            write,
            format,
        } => mutate::run(
            root(&cwd)?,
            primitive.as_deref(),
            since.as_deref().map(|r| (r, !only_lines)),
            whole_suite,
            write,
            format == Format::Json,
            mutate::Knobs {
                jobs: jobs.unwrap_or_else(|| {
                    std::thread::available_parallelism().map_or(1, |n| n.get() / 2)
                }),
                dry_run,
                coefficient: timeout_coefficient,
            },
        ),
    }
}
