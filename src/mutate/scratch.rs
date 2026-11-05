//! The copy of the project that mutate breaks, one mutant at a time, so the
//! real project is never touched.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, anyhow, bail};

use super::{Mutant, Outcome, Position, Span, cargo};

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
pub(super) struct Scratch {
    dir: PathBuf,
    target: PathBuf,
    /// Whether the tests outside the Contract run too.
    whole: bool,
}

/// The test executables a build makes: the contract target's, and with the
/// whole suite every other target's, by target name.
pub(super) struct Built {
    pub(super) contract: PathBuf,
    pub(super) outside: Vec<(String, PathBuf)>,
}

impl Scratch {
    pub(super) fn copy(root: &Path, whole: bool) -> anyhow::Result<Self> {
        let base = root.join("target/impish/mutate");
        let dir = base.join("project");
        if dir.exists() {
            fs::remove_dir_all(&dir).context("clear the scratch copy")?;
        }
        copy_dir(root, &dir, true).context("copy the project to mutate")?;
        let target = std::env::var_os("CARGO_TARGET_DIR")
            .map_or_else(|| root.join("target"), PathBuf::from)
            .join("impish-mutate");
        Ok(Self { dir, target, whole })
    }

    /// The test executables, or `None` when they don't build.
    pub(super) fn build(&self) -> anyhow::Result<Option<Built>> {
        let mut cmd = Command::new(cargo());
        cmd.args(["test", "--no-run", "--message-format=json"]);
        if self.whole {
            cmd.arg("--workspace");
        } else {
            cmd.args(["--test", "contract"]);
        }
        let out = cmd
            .current_dir(&self.dir)
            .env("CARGO_TARGET_DIR", &self.target)
            .stderr(Stdio::null())
            .output()
            .context("run `cargo test --no-run`")?;
        if !out.status.success() {
            return Ok(None);
        }
        let mut contract = None;
        let mut outside = Vec::new();
        for msg in String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        {
            let (Some(name), Some(exe)) = (
                msg["target"]["name"].as_str(),
                msg["executable"].as_str().map(PathBuf::from),
            ) else {
                continue;
            };
            if name == "contract" {
                contract = Some(exe);
            } else {
                outside.push((name.to_string(), exe));
            }
        }
        Ok(contract.map(|contract| Built { contract, outside }))
    }

    /// The tests outside the Contract that fail, as `<target>::<test>`, or
    /// `None` when one of their targets times out.
    pub(super) fn test_outside(
        &self,
        built: &Built,
        timeout: Option<Duration>,
    ) -> anyhow::Result<Option<BTreeSet<String>>> {
        let mut failed = BTreeSet::new();
        for (name, exe) in &built.outside {
            let Some(tests) = self.test(exe, None, timeout)? else {
                return Ok(None);
            };
            failed.extend(tests.into_iter().map(|t| format!("{name}::{t}")));
        }
        Ok(Some(failed))
    }

    /// The tests that fail, those under the `primitive` module when given,
    /// or `None` when they time out.
    pub(super) fn test(
        &self,
        exe: &Path,
        primitive: Option<&str>,
        timeout: Option<Duration>,
    ) -> anyhow::Result<Option<BTreeSet<String>>> {
        let mut cmd = Command::new(exe);
        if let Some(primitive) = primitive {
            cmd.arg(format!("{primitive}::"));
        }
        let mut child = cmd
            .current_dir(&self.dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("run the tests")?;
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
            .map_err(|_| anyhow!("read the tests' output"))??;
        Ok(Some(
            text.lines()
                .filter_map(|l| l.strip_prefix("test ")?.strip_suffix(" ... FAILED"))
                .map(String::from)
                .collect(),
        ))
    }

    /// The mutant's outcome against the contract tests, and with the whole
    /// suite the outside tests that catch it.
    pub(super) fn try_mutant(
        &self,
        mutant: &Mutant,
        primitive: &str,
        timeout: Duration,
        tags: &BTreeMap<String, Vec<String>>,
        active: &[String],
    ) -> anyhow::Result<(Outcome, BTreeSet<String>)> {
        let path = self.dir.join(&mutant.file);
        let original =
            fs::read_to_string(&path).with_context(|| format!("read {}", mutant.file))?;
        fs::write(
            &path,
            mutated(&original, &mutant.span, &mutant.replacement)?,
        )?;
        let mut outside = BTreeSet::new();
        let outcome = match self.build()? {
            None => Outcome::Unviable,
            Some(built) => match self.test(&built.contract, Some(primitive), Some(timeout))? {
                None => Outcome::TimedOut,
                Some(failed) => {
                    let by: BTreeSet<String> = failed
                        .iter()
                        .flat_map(|test| tags.get(test).into_iter().flatten())
                        .filter(|tag| active.contains(tag))
                        .cloned()
                        .collect();
                    if self.whole {
                        // A target that hangs names no test, so it proves nothing.
                        outside = self
                            .test_outside(&built, Some(timeout))?
                            .unwrap_or_default();
                    }
                    // Caught only by tests that name no active obligation,
                    // the Contract still lets the mutant through.
                    if by.is_empty() {
                        Outcome::Survived
                    } else {
                        Outcome::Killed { by, tests: failed }
                    }
                }
            },
        };
        fs::write(&path, original)?;
        Ok((outcome, outside))
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
