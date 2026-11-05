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
use serde_json::{Value, json};

mod judge;
mod scratch;

use judge::{Verdict, judge};
use scratch::Scratch;

use crate::contract::{Declared, covers, declarations, fingerprint, tagged_tests};
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

pub fn run(root: &Path, primitive: &str, write: bool, json: bool) -> anyhow::Result<ExitCode> {
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
        let problems = unreasoned
            .into_iter()
            .map(|(line, kind)| {
                format!(
                    "{declared_file}:{line}: an {kind} block with no ## heading giving its reason"
                )
            })
            .collect();
        let verdict = Verdict {
            problems,
            ..Verdict::default()
        };
        if write {
            record(root, primitive, &readme, false)?;
        }
        return Ok(finish(primitive, &[], &declared, &verdict, json));
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
    if !json {
        println!(
            "{primitive}: {} mutants of {} functions, {} tests",
            mutants.len(),
            targets.len(),
            tags.len()
        );
    }

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
        if !json {
            println!("  {}  {verdict}", mutant.name);
        }
        outcomes.push(outcome);
    }
    let tried: Vec<_> = mutants.iter().zip(&outcomes).collect();
    let tests: Vec<&str> = tags.keys().map(String::as_str).collect();
    let verdict = judge(&active, &tests, &tried, &declared, &declared_file);
    if write {
        record(root, primitive, &readme, verdict.problems.is_empty())?;
    }
    Ok(finish(primitive, &tried, &declared, &verdict, json))
}

/// Records in contract/coverage.json whether the primitive's mutate is met,
/// with the fingerprint `check` holds it to.
fn record(root: &Path, primitive: &str, readme: &str, met: bool) -> anyhow::Result<()> {
    let path = root.join("contract/coverage.json");
    let mut doc: Value = if path.exists() {
        let text = fs::read_to_string(&path).context("read contract/coverage.json")?;
        serde_json::from_str(&text).context("read contract/coverage.json")?
    } else {
        json!({})
    };
    let fingerprint = fingerprint(primitive, readme, |file| {
        fs::read_to_string(root.join(file)).unwrap_or_default()
    });
    doc["primitives"][primitive]["mutate"] = json!({"met": met, "fingerprint": fingerprint});
    fs::write(&path, serde_json::to_string_pretty(&doc)? + "\n")
        .context("write contract/coverage.json")
}

/// The mutant's name without line and column, as mutants.md declares it.
fn key(m: &Mutant) -> String {
    let rest = m.name.strip_prefix(m.file.as_str());
    let rest = rest
        .and_then(|r| r.split_once(": "))
        .map_or(m.name.as_str(), |(_, r)| r);
    format!("{}: {rest}", m.file)
}

/// Reports the verdict, as JSON or as the kill matrix and the problems.
fn finish(
    primitive: &str,
    tried: &[(&Mutant, &Outcome)],
    declared: &[Declared],
    v: &Verdict,
    json: bool,
) -> ExitCode {
    let code = if v.problems.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    };
    if json {
        println!("{}", judge::json(primitive, tried, declared, v));
        return code;
    }
    if !tried.is_empty() {
        println!(
            "\n{} killed, {} survived, {} timed out, {} unviable",
            v.killed, v.survived, v.timed_out, v.unviable
        );
        let width = v
            .rows
            .iter()
            .map(|(id, ..)| id.len())
            .max()
            .unwrap_or(0)
            .max(10);
        println!("\n  {:width$}  killed  unique", "obligation");
        for (id, killed, unique) in &v.rows {
            println!("  {id:width$}  {killed:>6}  {unique:>6}");
        }
        println!("  unique: caught by no other obligation's tests.");
        println!("  Tests outside the Contract weren't run; they may catch it too.");
    }
    if v.problems.is_empty() {
        println!("\nEvery mutant of {primitive} is caught or declared.");
        return code;
    }
    for problem in &v.problems {
        eprintln!("{problem}");
    }
    eprintln!("The Contract is not met.");
    code
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
