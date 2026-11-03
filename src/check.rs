//! `cargo impish check`: hold the Contract to its tests.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use anyhow::{Context, anyhow, bail};
use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::contract::{contract_attributes, covers, surface, tagged_tests};
use impish_core::obligations;

/// `impish.toml`, at the project root; every key is optional.
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    #[serde(default)]
    surface: SurfaceConfig,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct SurfaceConfig {
    /// Prints the real surface, one `<kind> <name>` per line.
    command: Option<Vec<String>>,
}

fn config(root: &Path) -> anyhow::Result<Config> {
    let path = root.join("impish.toml");
    if !path.exists() {
        return Ok(Config::default());
    }
    let text = fs::read_to_string(&path).context("read impish.toml")?;
    toml::from_str(&text).context("parse impish.toml")
}

/// What the configured surface command prints, one entry per non-blank line.
fn real_surface(root: &Path, command: &[String]) -> anyhow::Result<BTreeSet<String>> {
    let (program, args) = command
        .split_first()
        .context("the surface command in impish.toml is empty")?;
    let shown = command.join(" ");
    let out = Command::new(program)
        .args(args)
        .current_dir(root)
        .output()
        .with_context(|| format!("run the surface command `{shown}`"))?;
    if !out.status.success() {
        bail!(
            "the surface command `{shown}` failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(String::from)
        .collect())
}

struct Report {
    primitives: usize,
    obligations: usize,
    problems: Vec<String>,
    /// The real surface, when `impish.toml` names a command that prints it.
    real: Option<BTreeSet<String>>,
    /// Active obligations per kind.
    kinds: BTreeMap<String, usize>,
    /// Per primitive: its active obligations and its surface lines.
    covered: BTreeMap<String, (usize, Vec<String>)>,
}

/// What `contract/coverage.json` holds.
fn coverage(report: &Report) -> Value {
    let primitives: Map<String, Value> = report
        .covered
        .iter()
        .map(|(name, (n, surface))| (name.clone(), json!({"obligations": n, "surface": surface})))
        .collect();
    let mut value = json!({
        "obligations": report.obligations,
        "kinds": report.kinds,
        "primitives": primitives,
    });
    if let Some(real) = &report.real {
        let covered: BTreeSet<&String> = report.covered.values().flat_map(|(_, s)| s).collect();
        let percent = if real.is_empty() {
            0.0
        } else {
            (covered.len() as f64 * 1000.0 / real.len() as f64).round() / 10.0
        };
        value["surface"] =
            json!({"covered": covered.len(), "total": real.len(), "percent": percent});
        value["uncovered"] = json!(
            real.iter()
                .filter(|l| !covered.contains(l))
                .collect::<Vec<_>>()
        );
    }
    value
}

pub fn run(cwd: &Path, write: bool) -> anyhow::Result<ExitCode> {
    let root = cwd
        .ancestors()
        .find(|dir| dir.join("contract").is_dir())
        .ok_or_else(|| anyhow!("no contract/ in {} or above it", cwd.display()))?;
    let mut report = check(root)?;
    let path = root.join("contract/coverage.json");
    let text = serde_json::to_string_pretty(&coverage(&report))? + "\n";
    if write {
        fs::write(&path, text).context("write contract/coverage.json")?;
    } else if path.exists() && fs::read_to_string(&path)? != text {
        report.problems.push(
            "contract/coverage.json is out of date; run `cargo impish check --write`".to_string(),
        );
    }
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
    let real = match config(root)?.surface.command {
        Some(command) => Some(real_surface(root, &command)?),
        None => None,
    };
    let mut report = Report {
        primitives: names.len(),
        obligations: 0,
        problems: Vec::new(),
        real: None,
        kinds: BTreeMap::new(),
        covered: BTreeMap::new(),
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
        let mut lines = Vec::new();
        for line in surface(&readme) {
            report.problems.push(match line {
                Ok(line) if real.as_ref().is_none_or(|real| real.contains(line)) => {
                    lines.push(line.to_string());
                    continue;
                }
                Ok(line) => {
                    format!(
                        "{readme_file}: surface `{line}` is not in the surface command's output"
                    )
                }
                Err(line) => format!("{readme_file}: surface line `{line}` is not <kind> <name>"),
            });
        }
        let ids = match obligations(&readme, &readme_file) {
            Ok(ids) => ids,
            Err(problems) => {
                report.problems.extend(problems);
                continue;
            }
        };
        let active: Vec<_> = ids.values().filter(|o| !o.retired).collect();
        for o in &active {
            *report.kinds.entry(o.kind.clone()).or_default() += 1;
        }
        report.obligations += active.len();
        report.covered.insert(name.clone(), (active.len(), lines));
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
                match tag
                    .strip_prefix(&format!("{name}/A"))
                    .and_then(|n| n.parse().ok())
                {
                    Some(id) if ids.get(&id).is_some_and(|o| o.retired) => {
                        report.problems.push(format!(
                            "{name}::{} names `{tag}`, which is retired",
                            test.name
                        ));
                    }
                    Some(id) if ids.contains_key(&id) => {
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
        let active = ids.iter().filter(|(_, o)| !o.retired).map(|(id, _)| id);
        for id in active.filter(|id| !proved.contains(*id)) {
            report.problems.push(match ignored.get(id) {
                Some(tests) => format!("{name}/A{id} has no test (ignored: {})", tests.join(", ")),
                None => format!("{name}/A{id} has no test"),
            });
        }
    }
    report.problems.extend(stray_tags(root, &names));
    report.real = real;
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
        for n in contract_attributes(&src) {
            problems.push(format!(
                "{rel}:{n}: an #[impish::contract] attribute outside tests/contract/<primitive>.rs"
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
