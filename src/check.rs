//! `cargo impish check`: every obligation has a test.

use std::collections::BTreeMap;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use anyhow::Context;

mod github;
mod stray;

use crate::contract::tagged_tests;
use impish_core::{Obligation, obligations};

pub struct Report {
    pub primitives: Vec<Primitive>,
    pub problems: Vec<Problem>,
}

pub struct Primitive {
    pub name: String,
    pub obligations: Vec<Row>,
}

/// An obligation, and the tests that prove it.
pub struct Row {
    pub id: u32,
    pub title: String,
    pub retired: bool,
    pub tests: Vec<String>,
}

pub struct Problem {
    /// The primitive it belongs to, if any.
    pub primitive: Option<String>,
    /// The file and line it names, if any.
    pub at: Option<(String, usize)>,
    pub text: String,
    /// One line on how to fix it.
    pub fix: &'static str,
}

impl Report {
    pub fn obligations(&self) -> usize {
        self.primitives.iter().map(Primitive::active).sum()
    }
}

impl Primitive {
    pub fn active(&self) -> usize {
        self.obligations.iter().filter(|o| !o.retired).count()
    }

    pub fn tested(&self) -> usize {
        let proved = |o: &&Row| !o.retired && !o.tests.is_empty();
        self.obligations.iter().filter(proved).count()
    }
}

pub fn run(root: &Path) -> anyhow::Result<ExitCode> {
    let report = check(root)?;
    let met = report.problems.is_empty();
    if met {
        println!(
            "{} primitives, {} obligations, all tested",
            report.primitives.len(),
            report.obligations()
        );
    } else {
        for problem in &report.problems {
            eprintln!("{}", problem.text);
        }
        eprintln!("The Contract is not met.");
    }
    let github = github::Github::from_env(root);
    if env::var("GITHUB_ACTIONS").as_deref() == Ok("true") {
        for annotation in github.annotations(&report) {
            println!("{annotation}");
        }
    }
    if let Some(file) = env::var_os("GITHUB_STEP_SUMMARY") {
        OpenOptions::new()
            .append(true)
            .create(true)
            .open(&file)
            .and_then(|mut f| f.write_all(github.summary(&report).as_bytes()))
            .with_context(|| format!("append the job summary to {}", file.display()))?;
    }
    Ok(if met {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
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

/// The file and line at the start of a `<file>:<line>: …` problem.
fn located(text: &str) -> Option<(String, usize)> {
    let (at, _) = text.split_once(": ")?;
    let (file, line) = at.rsplit_once(':')?;
    Some((file.to_string(), line.parse().ok()?))
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
        primitives: Vec::new(),
        problems: Vec::new(),
    };
    for name in &names {
        let (primitive, problems) = primitive(root, name)?;
        report.primitives.push(primitive);
        report.problems.extend(problems);
    }
    let stray = stray::tags(root, &names)
        .into_iter()
        .map(|text| (text, "Move the test into its primitive's test file."));
    let tests = stray::tests(root, &names).into_iter().map(|text| {
        (
            text,
            "Give it a Contract, or move it out of tests/contract/.",
        )
    });
    for (text, fix) in stray.chain(tests) {
        report.problems.push(Problem {
            primitive: None,
            at: located(&text),
            text,
            fix,
        });
    }
    Ok(report)
}

/// A primitive's obligations, each with the tests that prove it, and the
/// problems found on the way.
fn primitive(root: &Path, name: &str) -> anyhow::Result<(Primitive, Vec<Problem>)> {
    let mut problems = Vec::new();
    let mut problem = |at, text, fix| {
        problems.push(Problem {
            primitive: Some(name.to_string()),
            at,
            text,
            fix,
        })
    };
    let mut primitive = Primitive {
        name: name.to_string(),
        obligations: Vec::new(),
    };
    let readme_file = format!("contract/{name}/README.md");
    let tests_file = format!("tests/contract/{name}.rs");
    let Some(readme) = read(root, &readme_file)? else {
        let fix = "Write down what the primitive promises in its README.";
        problem(None, format!("contract/{name}: no README.md"), fix);
        return Ok((primitive, problems));
    };
    let Some(tests) = read(root, &tests_file)? else {
        let fix = "Add the test file that proves its obligations.";
        problem(None, format!("contract/{name}: no {tests_file}"), fix);
        return Ok((primitive, problems));
    };
    let ids = match obligations(&readme, &readme_file) {
        Ok(ids) => ids,
        Err(found) => {
            for text in found {
                let fix = "Fix the obligation list; contract/README.md has the format.";
                problem(located(&text), text, fix);
            }
            return Ok((primitive, problems));
        }
    };
    let tagged = match tagged_tests(&tests) {
        Ok(tagged) => tagged,
        Err(e) => {
            let line = e.span().start().line;
            let text = format!("{tests_file}:{line}: isn't valid Rust: {e}");
            problem(
                Some((tests_file, line)),
                text,
                "Make the test file compile.",
            );
            primitive.obligations = ids
                .into_iter()
                .map(|(id, o)| row(id, o, Vec::new()))
                .collect();
            return Ok((primitive, problems));
        }
    };
    let mut proved: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    let mut ignored: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    for test in tagged {
        let at = Some((tests_file.clone(), test.line));
        let full = format!("{name}::{}", test.name);
        if test.tags.is_empty() {
            let fix = "Tag it with the obligations it proves, or move it out of tests/contract/.";
            problem(at.clone(), format!("{full} names no obligation"), fix);
        }
        for tag in test.tags {
            let id = tag
                .strip_prefix(&format!("{name}/A"))
                .and_then(|n| n.parse().ok());
            match id.and_then(|id| Some((id, ids.get(&id)?))) {
                Some((_, o)) if o.retired => problem(
                    at.clone(),
                    format!("{full} names `{tag}`, which is retired"),
                    "Delete the test, or name the obligation that replaced it.",
                ),
                Some((id, _)) if test.ignored => ignored.entry(id).or_default().push(full.clone()),
                Some((id, _)) => proved.entry(id).or_default().push(full.clone()),
                None => problem(
                    at.clone(),
                    format!("{full} names `{tag}`, which contract/{name} doesn't declare"),
                    "Fix the tag, or add the obligation to the Contract first.",
                ),
            }
        }
    }
    for (id, o) in ids {
        let tests = proved.remove(&id).unwrap_or_default();
        if !o.retired && tests.is_empty() {
            let at = Some((readme_file.clone(), o.line));
            let fix = "Write a test that proves it, or retire the obligation.";
            let text = match ignored.get(&id) {
                Some(t) => format!("{name}/A{id} has no test (ignored: {})", t.join(", ")),
                None => format!("{name}/A{id} has no test"),
            };
            problem(at, text, fix);
        }
        primitive.obligations.push(row(id, o, tests));
    }
    Ok((primitive, problems))
}

fn row(id: u32, o: Obligation, tests: Vec<String>) -> Row {
    Row {
        id,
        title: o.title,
        retired: o.retired,
        tests,
    }
}
