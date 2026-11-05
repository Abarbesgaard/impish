//! The mutants cargo-mutants makes of the code a primitive covers.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, bail};

use super::Mutant;

/// The mutants cargo-mutants makes of the covered functions, in its order.
pub(super) fn list(root: &Path, targets: &[(&str, &str)]) -> anyhow::Result<Vec<Mutant>> {
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

pub(super) fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".into())
}
