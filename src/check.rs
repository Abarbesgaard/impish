//! `cargo impish check`: hold the Contract to its tests.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context, anyhow};

use crate::contract::{obligations, tagged_tests};

struct Report {
    primitives: usize,
    obligations: usize,
    problems: Vec<String>,
}

pub fn run(cwd: &Path) -> anyhow::Result<ExitCode> {
    let root = cwd
        .ancestors()
        .find(|dir| dir.join("contract").is_dir())
        .ok_or_else(|| anyhow!("no contract/ in {} or above it", cwd.display()))?;
    let report = check(root)?;
    if report.problems.is_empty() {
        println!(
            "The Contract holds: {} obligations across {} primitives.",
            report.obligations, report.primitives
        );
        return Ok(ExitCode::SUCCESS);
    }
    for problem in &report.problems {
        eprintln!("{problem}");
    }
    eprintln!("The Contract is not met.");
    Ok(ExitCode::FAILURE)
}

fn check(root: &Path) -> anyhow::Result<Report> {
    let dir = root.join("contract");
    let mut names: Vec<String> = fs::read_dir(&dir)
        .with_context(|| format!("read {}", dir.display()))?
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    let mut report = Report {
        primitives: names.len(),
        obligations: 0,
        problems: Vec::new(),
    };
    for name in names {
        let Ok(readme) = fs::read_to_string(dir.join(&name).join("README.md")) else {
            report
                .problems
                .push(format!("contract/{name}: no README.md"));
            continue;
        };
        let tests_file = format!("tests/contract/{name}.rs");
        let Ok(tests) = fs::read_to_string(root.join(&tests_file)) else {
            report
                .problems
                .push(format!("contract/{name}: no {tests_file}"));
            continue;
        };
        let ids = match obligations(&readme, &format!("contract/{name}/README.md")) {
            Ok(ids) => ids,
            Err(problem) => {
                report.problems.push(problem);
                continue;
            }
        };
        report.obligations += ids.len();
        let mut proved = BTreeSet::new();
        for test in tagged_tests(&tests) {
            if test.tags.is_empty() {
                report
                    .problems
                    .push(format!("{name}::{} names no obligation", test.name));
            }
            for tag in test.tags {
                match tag
                    .strip_prefix(&format!("{name}/A"))
                    .and_then(|n| n.parse().ok())
                {
                    Some(id) if ids.contains(&id) => {
                        proved.insert(id);
                    }
                    _ => report.problems.push(format!(
                        "{name}::{} names `{tag}`, which contract/{name} doesn't declare",
                        test.name
                    )),
                }
            }
        }
        for id in ids.difference(&proved) {
            report.problems.push(format!("{name}/A{id} has no test"));
        }
    }
    Ok(report)
}
