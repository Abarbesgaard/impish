//! `cargo impish check`: hold the Contract to its tests.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, anyhow};

use crate::contract::{covers, obligations, tag_lines, tagged_tests};

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
    for name in &names {
        let Ok(readme) = fs::read_to_string(dir.join(name).join("README.md")) else {
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
        let readme_file = format!("contract/{name}/README.md");
        for target in covers(&readme) {
            report.problems.push(match target {
                Ok((file, func)) if defines(root, file, func) => continue,
                Ok((file, func)) => {
                    format!("{readme_file}: covers `{file}::{func}`, which doesn't exist")
                }
                Err(line) => {
                    format!("{readme_file}: covers line `{line}` is not <file>::<function>")
                }
            });
        }
        let ids = match obligations(&readme, &readme_file) {
            Ok(ids) => ids,
            Err(problem) => {
                report.problems.push(problem);
                continue;
            }
        };
        report.obligations += ids.len();
        let mut proved = BTreeSet::new();
        let mut ignored: BTreeMap<u32, Vec<String>> = BTreeMap::new();
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
                        if test.ignored {
                            let tests = ignored.entry(id).or_default();
                            tests.push(format!("{name}::{}", test.name));
                        } else {
                            proved.insert(id);
                        }
                    }
                    _ => report.problems.push(format!(
                        "{name}::{} names `{tag}`, which contract/{name} doesn't declare",
                        test.name
                    )),
                }
            }
        }
        for id in ids.difference(&proved) {
            report.problems.push(match ignored.get(id) {
                Some(tests) => format!("{name}/A{id} has no test (ignored: {})", tests.join(", ")),
                None => format!("{name}/A{id} has no test"),
            });
        }
    }
    report.problems.extend(stray_tags(root, &names));
    Ok(report)
}

fn defines(root: &Path, file: &str, func: &str) -> bool {
    fs::read_to_string(root.join(file)).is_ok_and(|src| {
        src.contains(&format!("fn {func}(")) || src.contains(&format!("fn {func}<"))
    })
}

/// Tags in `src/` and `tests/` outside the primitives' test files.
fn stray_tags(root: &Path, names: &[String]) -> Vec<String> {
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    rust_files(&root.join("tests"), &mut files);
    files.sort();
    let homes: Vec<PathBuf> = names
        .iter()
        .map(|name| root.join("tests/contract").join(format!("{name}.rs")))
        .collect();
    let mut problems = Vec::new();
    for file in files.iter().filter(|file| !homes.contains(file)) {
        let src = fs::read_to_string(file).unwrap_or_default();
        let rel = file.strip_prefix(root).unwrap_or(file).display();
        for n in tag_lines(&src) {
            problems.push(format!(
                "{rel}:{n}: a Contract tag outside tests/contract/<primitive>.rs"
            ));
        }
    }
    problems
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}
