//! The mutants cargo-mutants makes of the code a primitive covers.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, bail};
use serde::Deserialize;

use crate::code::Function;

#[derive(Deserialize)]
pub(super) struct Mutant {
    pub(super) name: String,
    pub(super) file: String,
    pub(super) replacement: String,
    pub(super) span: Span,
    /// `FnValue` for a replacement of the whole body.
    #[serde(default)]
    pub(super) genre: String,
    /// The innermost function the mutant is in.
    pub(super) function: Option<Within>,
}

/// What became of a mutant.
pub(super) enum Outcome {
    /// Caught: the active obligations tagged on the tests that failed, and
    /// those tests.
    Killed {
        by: BTreeSet<String>,
        tests: BTreeSet<String>,
    },
    /// Uncaught, though these contract tests run its code.
    Lived {
        ran: BTreeSet<String>,
    },
    /// No contract test runs its code, so it was never built.
    Unreached,
    /// The mutant doesn't build, so it proves nothing.
    Unviable,
    TimedOut,
    /// Never tried: no contract test notices its function doing nothing.
    Erased,
}

#[derive(Deserialize)]
pub(super) struct Within {
    pub(super) function_name: String,
    pub(super) span: Span,
}

#[derive(Deserialize)]
pub(super) struct Span {
    pub(super) start: Position,
    pub(super) end: Position,
}

/// 1-based line, 1-based column in characters, as cargo-mutants writes them.
#[derive(Deserialize)]
pub(super) struct Position {
    pub(super) line: usize,
    pub(super) column: usize,
}

/// The mutants cargo-mutants makes of the functions the primitive owns, in
/// its order: those inside one of them, nested functions included.
pub(super) fn list(root: &Path, owned: &[Function]) -> anyhow::Result<Vec<Mutant>> {
    let files: BTreeSet<&str> = owned.iter().map(|f| f.file.as_str()).collect();
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
            owned.iter().any(|f| {
                m.file == f.file && f.first <= m.span.start.line && m.span.end.line <= f.last
            })
        })
        .collect())
}

pub(super) fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".into())
}

/// The directories under contract/ with a README, in order.
pub(super) fn primitives(root: &Path) -> anyhow::Result<Vec<String>> {
    let dir = root.join("contract");
    let mut names: Vec<String> = fs::read_dir(&dir)
        .with_context(|| format!("read {}", dir.display()))?
        .flatten()
        .filter(|entry| entry.path().join("README.md").is_file())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    Ok(names)
}
