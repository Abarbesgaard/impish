//! `cargo impish mutate <primitive>`: break the code a primitive covers, one
//! mutant at a time, and credit each kill to the obligations tagged on the
//! contract tests that caught it. cargo-mutants supplies the mutants.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, anyhow, bail};
use serde::Deserialize;

use crate::contract::{covers, tagged_tests};
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
    /// Caught; the active obligations tagged on the tests that failed.
    Killed(BTreeSet<String>),
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
        let verdict = match &outcome {
            Outcome::Killed(by) => {
                format!(
                    "killed by {}",
                    by.iter().cloned().collect::<Vec<_>>().join(", ")
                )
            }
            Outcome::Survived => "SURVIVED".to_string(),
            Outcome::Unviable => "unviable".to_string(),
            Outcome::TimedOut => "timed out".to_string(),
        };
        println!("  {}  {verdict}", mutant.name);
        outcomes.push(outcome);
    }
    Ok(report(primitive, &active, &outcomes))
}

/// The kill matrix, then the hollow obligations: those whose tests catch none
/// of the mutants.
fn report(primitive: &str, active: &[String], outcomes: &[Outcome]) -> ExitCode {
    let count = |f: fn(&Outcome) -> bool| outcomes.iter().filter(|o| f(o)).count();
    println!(
        "\n{} killed, {} survived, {} timed out, {} unviable",
        count(|o| matches!(o, Outcome::Killed(_))),
        count(|o| matches!(o, Outcome::Survived)),
        count(|o| matches!(o, Outcome::TimedOut)),
        count(|o| matches!(o, Outcome::Unviable)),
    );
    let width = active.iter().map(String::len).max().unwrap_or(0).max(10);
    println!("\n  {:width$}  killed  unique", "obligation");
    let mut hollow = Vec::new();
    for id in active {
        let mut killed = 0;
        let mut unique = 0;
        for by in outcomes.iter().filter_map(|o| match o {
            Outcome::Killed(by) if by.contains(id) => Some(by),
            _ => None,
        }) {
            killed += 1;
            unique += usize::from(by.len() == 1);
        }
        println!("  {id:width$}  {killed:>6}  {unique:>6}");
        if killed == 0 {
            hollow.push(id);
        }
    }
    println!("  unique: caught by no other obligation's tests.");
    println!("  Tests outside the Contract weren't run; they may catch it too.");
    if hollow.is_empty() {
        println!("\nEvery obligation of {primitive} catches a mutant.");
        return ExitCode::SUCCESS;
    }
    for id in hollow {
        eprintln!("{id} is hollow: its tests catch none of the mutants");
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

/// `src` with the span replaced.
fn mutated(src: &str, span: &Span, replacement: &str) -> anyhow::Result<String> {
    let offset = |p: &Position| -> Option<usize> {
        let start: usize = src
            .split_inclusive('\n')
            .take(p.line - 1)
            .map(str::len)
            .sum();
        let line = src[start..].split_inclusive('\n').next().unwrap_or("");
        let column = line
            .char_indices()
            .map(|(i, _)| i)
            .chain([line.len()])
            .nth(p.column - 1)?;
        Some(start + column)
    };
    let (Some(start), Some(end)) = (offset(&span.start), offset(&span.end)) else {
        bail!("a mutant's span is outside its file");
    };
    Ok(format!("{}{replacement}{}", &src[..start], &src[end..]))
}

/// A copy of the project to mutate, so the real one is never touched. Builds
/// share one target directory, `impish-mutate` under `CARGO_TARGET_DIR` or
/// the project's `target/`, so each mutant only rebuilds what it changed.
struct Scratch {
    dir: PathBuf,
    target: PathBuf,
}

impl Scratch {
    fn copy(root: &Path) -> anyhow::Result<Self> {
        let base = root.join("target/impish/mutate");
        let dir = base.join("project");
        if dir.exists() {
            fs::remove_dir_all(&dir).context("clear the scratch copy")?;
        }
        copy_dir(root, &dir, true).context("copy the project to mutate")?;
        let target = std::env::var_os("CARGO_TARGET_DIR")
            .map_or_else(|| root.join("target"), PathBuf::from)
            .join("impish-mutate");
        Ok(Self { dir, target })
    }

    /// The contract test executable, or `None` when it doesn't build.
    fn build(&self) -> anyhow::Result<Option<PathBuf>> {
        let out = Command::new(cargo())
            .args([
                "test",
                "--test",
                "contract",
                "--no-run",
                "--message-format=json",
            ])
            .current_dir(&self.dir)
            .env("CARGO_TARGET_DIR", &self.target)
            .stderr(Stdio::null())
            .output()
            .context("run `cargo test --no-run`")?;
        if !out.status.success() {
            return Ok(None);
        }
        Ok(String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .filter(|msg| msg["target"]["name"] == "contract")
            .filter_map(|msg| msg["executable"].as_str().map(PathBuf::from))
            .next_back())
    }

    /// The primitive's tests that fail, or `None` when they time out.
    fn test(
        &self,
        exe: &Path,
        primitive: &str,
        timeout: Option<Duration>,
    ) -> anyhow::Result<Option<BTreeSet<String>>> {
        let mut child = Command::new(exe)
            .arg(format!("{primitive}::"))
            .current_dir(&self.dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("run the contract tests")?;
        let mut stdout = child.stdout.take().expect("piped stdout");
        let reader = thread::spawn(move || {
            let mut text = String::new();
            stdout.read_to_string(&mut text).map(|_| text)
        });
        let started = Instant::now();
        while child.try_wait()?.is_none() {
            if timeout.is_some_and(|t| started.elapsed() > t) {
                child.kill()?;
                child.wait()?;
                return Ok(None);
            }
            thread::sleep(Duration::from_millis(20));
        }
        let text = reader
            .join()
            .map_err(|_| anyhow!("read the contract tests' output"))??;
        Ok(Some(
            text.lines()
                .filter_map(|l| l.strip_prefix("test ")?.strip_suffix(" ... FAILED"))
                .map(String::from)
                .collect(),
        ))
    }

    fn try_mutant(
        &self,
        mutant: &Mutant,
        primitive: &str,
        timeout: Duration,
        tags: &BTreeMap<String, Vec<String>>,
        active: &[String],
    ) -> anyhow::Result<Outcome> {
        let path = self.dir.join(&mutant.file);
        let original =
            fs::read_to_string(&path).with_context(|| format!("read {}", mutant.file))?;
        fs::write(
            &path,
            mutated(&original, &mutant.span, &mutant.replacement)?,
        )?;
        let outcome = match self.build()? {
            None => Outcome::Unviable,
            Some(exe) => match self.test(&exe, primitive, Some(timeout))? {
                None => Outcome::TimedOut,
                Some(failed) => {
                    let by: BTreeSet<String> = failed
                        .iter()
                        .flat_map(|test| tags.get(test).into_iter().flatten())
                        .filter(|tag| active.contains(tag))
                        .cloned()
                        .collect();
                    // Caught only by tests that name no active obligation,
                    // the Contract still lets the mutant through.
                    if by.is_empty() {
                        Outcome::Survived
                    } else {
                        Outcome::Killed(by)
                    }
                }
            },
        };
        fs::write(&path, original)?;
        Ok(outcome)
    }
}

/// Copies `from` to `to`, leaving out `.git` and, at the top, `target`.
fn copy_dir(from: &Path, to: &Path, top: bool) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == ".git" || (top && name == "target") {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            copy_dir(&path, &to.join(&name), false)?;
        } else {
            // Written afresh, not `fs::copy`, which keeps the modification
            // time on macOS: Cargo would take the copy for older than the
            // last run's mutant build, and reuse that build.
            fs::write(to.join(&name), fs::read(&path)?)?;
            fs::set_permissions(to.join(&name), entry.metadata()?.permissions())?;
        }
    }
    Ok(())
}
