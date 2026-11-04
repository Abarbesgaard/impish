//! `cargo impish mutate <primitive>`: break the code a primitive covers, one
//! mutant at a time, and credit each kill to the obligations tagged on the
//! contract tests that caught it. cargo-mutants supplies the mutants.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::{Command, ExitCode};
use std::time::{Duration, Instant};

use anyhow::{Context, anyhow, bail};
use serde::Deserialize;

mod scratch;

use scratch::Scratch;

use crate::contract::{Declared, covers, declarations, tagged_tests};
use impish_core::obligations;

#[derive(Deserialize)]
struct Mutant {
    name: String,
    file: String,
    function: Option<Function>,
    replacement: String,
    span: Span,
}

#[derive(Deserialize)]
struct Function {
    function_name: String,
}

#[derive(Deserialize)]
struct Span {
    start: Position,
    end: Position,
}

/// 1-based line, 1-based column in characters, as cargo-mutants writes them.
#[derive(Deserialize)]
struct Position {
    line: usize,
    column: usize,
}

enum Outcome {
    /// Caught: the active obligations tagged on the tests that failed, and
    /// those tests.
    Killed {
        by: BTreeSet<String>,
        tests: BTreeSet<String>,
    },
    Survived,
    /// The mutant doesn't build, so it proves nothing.
    Unviable,
    TimedOut,
}

pub fn run(root: &Path, primitive: &str) -> anyhow::Result<ExitCode> {
    let readme_file = format!("contract/{primitive}/README.md");
    let readme =
        fs::read_to_string(root.join(&readme_file)).with_context(|| format!("no {readme_file}"))?;
    let ids =
        obligations(&readme, &readme_file).map_err(|problems| anyhow!(problems.join("\n")))?;
    let active: Vec<String> = ids
        .iter()
        .filter(|(_, o)| !o.retired)
        .map(|(id, _)| format!("{primitive}/A{id}"))
        .collect();
    let targets = covers(&readme)
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|line| anyhow!("{readme_file}: covers line `{line}` is not <file>::<function>"))?;
    if targets.is_empty() {
        bail!("{readme_file} covers no code, so there is nothing to mutate");
    }
    let declared_file = format!("contract/{primitive}/mutants.md");
    let declared_text = match fs::read_to_string(root.join(&declared_file)) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e).with_context(|| format!("read {declared_file}")),
    };
    let (declared, unreasoned) = declarations(&declared_text);
    if !unreasoned.is_empty() {
        for (line, kind) in unreasoned {
            eprintln!(
                "{declared_file}:{line}: an {kind} block with no ## heading giving its reason"
            );
        }
        eprintln!("The Contract is not met.");
        return Ok(ExitCode::FAILURE);
    }
    let tests_file = format!("tests/contract/{primitive}.rs");
    let tests =
        fs::read_to_string(root.join(&tests_file)).with_context(|| format!("no {tests_file}"))?;
    let tags: BTreeMap<String, Vec<String>> = tagged_tests(&tests)
        .with_context(|| format!("{tests_file} isn't valid Rust"))?
        .into_iter()
        .map(|t| (format!("{primitive}::{}", t.name), t.tags))
        .collect();

    let mutants = list(root, &targets)?;
    if mutants.is_empty() {
        bail!("cargo-mutants finds no mutants in the code {readme_file} covers");
    }
    let scratch = Scratch::copy(root)?;
    let exe = scratch
        .build()?
        .ok_or_else(|| anyhow!("the contract tests don't build"))?;
    let started = Instant::now();
    let failed = match scratch.test(&exe, primitive, None)? {
        Some(failed) => failed,
        None => bail!("the contract tests time out before any mutation"),
    };
    if !failed.is_empty() {
        let failed: Vec<_> = failed.into_iter().collect();
        bail!(
            "the baseline is red: {} fail before any mutation",
            failed.join(", ")
        );
    }
    let timeout = started.elapsed() * 5 + Duration::from_secs(10);
    println!(
        "{primitive}: {} mutants of {} functions, {} tests",
        mutants.len(),
        targets.len(),
        tags.len()
    );

    let mut outcomes = Vec::new();
    for mutant in &mutants {
        let outcome = scratch.try_mutant(mutant, primitive, timeout, &tags, &active)?;
        let mut verdict = match &outcome {
            Outcome::Killed { by, .. } => {
                format!(
                    "killed by {}",
                    by.iter().cloned().collect::<Vec<_>>().join(", ")
                )
            }
            Outcome::Survived => "SURVIVED".to_string(),
            Outcome::Unviable => "unviable".to_string(),
            Outcome::TimedOut => "timed out".to_string(),
        };
        if matches!(outcome, Outcome::Survived | Outcome::TimedOut)
            && let Some(d) = declared.iter().find(|d| d.name == key(mutant))
        {
            verdict = format!("{verdict}, declared {}", d.kind);
        }
        println!("  {}  {verdict}", mutant.name);
        outcomes.push(outcome);
    }
    let tried: Vec<_> = mutants.iter().zip(&outcomes).collect();
    let tests: Vec<&str> = tags.keys().map(String::as_str).collect();
    Ok(report(
        primitive,
        &active,
        &tests,
        &tried,
        &declared,
        &declared_file,
    ))
}

/// The mutant's name without line and column, as mutants.md declares it.
fn key(m: &Mutant) -> String {
    let rest = m.name.strip_prefix(m.file.as_str());
    let rest = rest
        .and_then(|r| r.split_once(": "))
        .map_or(m.name.as_str(), |(_, r)| r);
    format!("{}: {rest}", m.file)
}

/// The kill matrix, then why the Contract isn't met: hollow obligations,
/// blind tests, mutants no contract test catches, and declarations that
/// don't hold.
fn report(
    primitive: &str,
    active: &[String],
    tests: &[&str],
    tried: &[(&Mutant, &Outcome)],
    declared: &[Declared],
    declared_file: &str,
) -> ExitCode {
    let outcomes: Vec<&Outcome> = tried.iter().map(|(_, o)| *o).collect();
    let count = |f: fn(&Outcome) -> bool| outcomes.iter().filter(|o| f(o)).count();
    println!(
        "\n{} killed, {} survived, {} timed out, {} unviable",
        count(|o| matches!(o, Outcome::Killed { .. })),
        count(|o| matches!(o, Outcome::Survived)),
        count(|o| matches!(o, Outcome::TimedOut)),
        count(|o| matches!(o, Outcome::Unviable)),
    );
    let width = active.iter().map(String::len).max().unwrap_or(0).max(10);
    println!("\n  {:width$}  killed  unique", "obligation");
    let mut problems = Vec::new();
    for id in active {
        let mut killed = 0;
        let mut unique = 0;
        for by in outcomes.iter().filter_map(|o| match o {
            Outcome::Killed { by, .. } if by.contains(id) => Some(by),
            _ => None,
        }) {
            killed += 1;
            unique += usize::from(by.len() == 1);
        }
        println!("  {id:width$}  {killed:>6}  {unique:>6}");
        if killed == 0 {
            problems.push(format!(
                "{id} is hollow: its tests catch none of the mutants"
            ));
        }
    }
    println!("  unique: caught by no other obligation's tests.");
    println!("  Tests outside the Contract weren't run; they may catch it too.");
    for test in tests {
        let catches = outcomes
            .iter()
            .any(|o| matches!(o, Outcome::Killed { tests, .. } if tests.contains(*test)));
        if !catches {
            problems.push(format!("{test} is blind: it catches none of the mutants"));
        }
    }
    for (mutant, outcome) in tried {
        let what = match outcome {
            Outcome::Survived => "survived",
            Outcome::TimedOut => "timed out",
            _ => continue,
        };
        let name = key(mutant);
        if !declared.iter().any(|d| d.name == name) {
            problems.push(format!(
                "{name} {what}: catch it in a contract test, or declare it in {declared_file}"
            ));
        }
    }
    for d in declared {
        let mut by = BTreeSet::new();
        let mut matched = false;
        for (_, outcome) in tried.iter().filter(|(m, _)| key(m) == d.name) {
            matched = true;
            if let Outcome::Killed { by: ids, .. } = outcome {
                by.extend(ids.iter().map(String::as_str));
            }
        }
        let at = format!("{declared_file}:{}: {}", d.line, d.name);
        if !matched {
            problems.push(format!("{at} matches no mutant"));
        } else if !by.is_empty() {
            let verb = if by.len() == 1 { "catches" } else { "catch" };
            let by = by.into_iter().collect::<Vec<_>>().join(", ");
            problems.push(format!("{at} is declared {}, but {by} {verb} it", d.kind));
        }
    }
    if problems.is_empty() {
        println!("\nEvery mutant of {primitive} is caught or declared.");
        return ExitCode::SUCCESS;
    }
    for problem in problems {
        eprintln!("{problem}");
    }
    eprintln!("The Contract is not met.");
    ExitCode::FAILURE
}

/// The mutants cargo-mutants makes of the covered functions, in its order.
fn list(root: &Path, targets: &[(&str, &str)]) -> anyhow::Result<Vec<Mutant>> {
    let files: BTreeSet<&str> = targets.iter().map(|(file, _)| *file).collect();
    let mut cmd = Command::new(cargo());
    cmd.args(["mutants", "--list", "--json", "--workspace"])
        .current_dir(root);
    for file in &files {
        cmd.args(["--file", file]);
    }
    let out = cmd
        .output()
        .context("run `cargo mutants`; is cargo-mutants installed?")?;
    if !out.status.success() {
        bail!(
            "`cargo mutants --list` failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let all: Vec<Mutant> =
        serde_json::from_slice(&out.stdout).context("read `cargo mutants --list --json`")?;
    Ok(all
        .into_iter()
        .filter(|m| {
            let name = m.function.as_ref().map_or("", |f| f.function_name.as_str());
            targets.iter().any(|(file, func)| {
                m.file == *file && (name == *func || name.ends_with(&format!("::{func}")))
            })
        })
        .collect())
}

fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".into())
}
