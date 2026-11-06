//! The mutants cargo-mutants makes of the code a primitive covers.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, bail};

use super::Mutant;
use crate::code::Function;

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
