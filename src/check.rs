//! `cargo impish check`: every obligation has a test.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use anyhow::Context;

mod stray;

use crate::contract::tagged_tests;
use impish_core::obligations;

struct Report {
    primitives: usize,
    obligations: usize,
    problems: Vec<String>,
}

pub fn run(root: &Path) -> anyhow::Result<ExitCode> {
    let report = check(root)?;
    if report.problems.is_empty() {
        println!(
            "{} primitives, {} obligations, all tested",
            report.primitives, report.obligations
        );
        return Ok(ExitCode::SUCCESS);
    }
    for problem in &report.problems {
        eprintln!("{problem}");
    }
    eprintln!("The Contract is not met.");
    Ok(ExitCode::FAILURE)
}

/// The text of `rel`, or None when there is no such file. A file that is
/// there but can't be read means check can't decide.
fn read(root: &Path, rel: &str) -> anyhow::Result<Option<String>> {
    let path = root.join(rel);
    if !path.exists() {
        return Ok(None);
    }
    fs::read_to_string(&path)
        .map(Some)
        .with_context(|| format!("read {rel}"))
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
    for name in &names {
        let readme_file = format!("contract/{name}/README.md");
        let Some(readme) = read(root, &readme_file)? else {
            report
                .problems
                .push(format!("contract/{name}: no README.md"));
            continue;
        };
        let tests_file = format!("tests/contract/{name}.rs");
        let Some(tests) = read(root, &tests_file)? else {
            report
                .problems
                .push(format!("contract/{name}: no {tests_file}"));
            continue;
        };
        let ids = match obligations(&readme, &readme_file) {
            Ok(ids) => ids,
            Err(problems) => {
                report.problems.extend(problems);
                continue;
            }
        };
        report.obligations += ids.values().filter(|o| !o.retired).count();
        let tagged = match tagged_tests(&tests) {
            Ok(tagged) => tagged,
            Err(e) => {
                let line = e.span().start().line;
                report
                    .problems
                    .push(format!("{tests_file}:{line}: isn't valid Rust: {e}"));
                continue;
            }
        };
        let mut proved = BTreeSet::new();
        let mut ignored: BTreeMap<u32, Vec<String>> = BTreeMap::new();
        for test in tagged {
            if test.tags.is_empty() {
                report
                    .problems
                    .push(format!("{name}::{} names no obligation", test.name));
            }
            for tag in test.tags {
                let id = tag
                    .strip_prefix(&format!("{name}/A"))
                    .and_then(|n| n.parse().ok());
                match id.and_then(|id| Some((id, ids.get(&id)?))) {
                    Some((_, o)) if o.retired => report.problems.push(format!(
                        "{name}::{} names `{tag}`, which is retired",
                        test.name
                    )),
                    Some((id, _)) if test.ignored => {
                        let tests = ignored.entry(id).or_default();
                        tests.push(format!("{name}::{}", test.name));
                    }
                    Some((id, _)) => {
                        proved.insert(id);
                    }
                    None => report.problems.push(format!(
                        "{name}::{} names `{tag}`, which contract/{name} doesn't declare",
                        test.name
                    )),
                }
            }
        }
        let active = ids.iter().filter(|(_, o)| !o.retired).map(|(id, _)| id);
        for id in active.filter(|id| !proved.contains(*id)) {
            report.problems.push(match ignored.get(id) {
                Some(tests) => format!("{name}/A{id} has no test (ignored: {})", tests.join(", ")),
                None => format!("{name}/A{id} has no test"),
            });
        }
    }
    report.problems.extend(stray::tags(root, &names));
    report.problems.extend(stray::tests(root, &names));
    Ok(report)
}
