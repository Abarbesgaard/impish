//! What changed since a git ref, for `mutate --since`: the files, and the
//! lines in each, against the merge base, including uncommitted work.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

use anyhow::{Context, bail};

pub(super) struct Changes {
    pub(super) reference: String,
    /// Whether a changed Contract or contract test mutates all of a primitive.
    contract: bool,
    files: BTreeSet<String>,
    /// Per file: the changed line ranges, first and last, 1-based.
    lines: BTreeMap<String, Vec<(usize, usize)>>,
}

impl Changes {
    pub(super) fn since(root: &Path, reference: &str, contract: bool) -> anyhow::Result<Self> {
        let base = git(root, &["merge-base", reference, "HEAD"])?;
        let diff = git(
            root,
            &[
                "diff",
                "-U0",
                "--no-color",
                "--no-ext-diff",
                "--relative",
                base.trim(),
            ],
        )?;
        let mut changes = Changes {
            reference: reference.to_string(),
            contract,
            files: BTreeSet::new(),
            lines: BTreeMap::new(),
        };
        let mut file = None;
        for line in diff.lines() {
            if let Some(path) = line.strip_prefix("--- a/") {
                changes.files.insert(path.to_string());
            } else if let Some(path) = line.strip_prefix("+++ b/") {
                changes.files.insert(path.to_string());
                file = Some(path.to_string());
            } else if line.starts_with("+++ ") {
                file = None;
            } else if let (Some(file), Some(range)) = (&file, hunk(line)) {
                changes.lines.entry(file.clone()).or_default().push(range);
            }
        }
        for path in git(root, &["ls-files", "--others", "--exclude-standard"])?.lines() {
            changes.files.insert(path.to_string());
            changes
                .lines
                .insert(path.to_string(), vec![(1, usize::MAX)]);
        }
        Ok(changes)
    }

    /// Whether the primitive's Contract or its contract tests changed, and
    /// that calls for mutating all of it.
    pub(super) fn touch_contract(&self, primitive: &str) -> bool {
        if !self.contract {
            return false;
        }
        let dir = format!("contract/{primitive}/");
        let tests = format!("tests/contract/{primitive}.rs");
        self.files
            .iter()
            .any(|f| f.starts_with(&dir) || *f == tests)
    }

    /// Whether any line from `first` to `last` of `file` changed.
    pub(super) fn touch(&self, file: &str, first: usize, last: usize) -> bool {
        self.lines
            .get(file)
            .is_some_and(|ranges| ranges.iter().any(|&(a, b)| a <= last && b >= first))
    }
}

/// The new side of a `@@ -a,b +c,d @@` header: lines c to c+d-1, or the
/// line after which lines were removed.
fn hunk(line: &str) -> Option<(usize, usize)> {
    let new = line
        .strip_prefix("@@ ")?
        .split(' ')
        .nth(1)?
        .strip_prefix('+')?;
    let (start, count) = match new.split_once(',') {
        Some((start, count)) => (start.parse().ok()?, count.parse().ok()?),
        None => (new.parse().ok()?, 1),
    };
    Some((start, start + count.max(1) - 1))
}

fn git(root: &Path, args: &[&str]) -> anyhow::Result<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .context("run git")?;
    if !out.status.success() {
        bail!(
            "`git {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}
