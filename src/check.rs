//! `cargo impish check`: hold the Contract to its tests.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::{Command, ExitCode};

use anyhow::{Context, bail};
use serde::Deserialize;
use serde_json::{Map, Value, json};

mod exceptions;
mod scope;
mod stray;

use crate::code::{self, Resolved};
use crate::contract::{fingerprint, intent, surface, tagged_tests};
use crate::decisions;
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
    /// The ceilings in contract/scope.toml that could be counted.
    scope: Vec<scope::Ceiling>,
    /// Each exception's tag and the target it excuses.
    exceptions: BTreeMap<String, String>,
    /// What the covers lines resolve to.
    code: Resolved,
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
        let percent = percent(covered.len(), real.len());
        value["surface"] =
            json!({"covered": covered.len(), "total": real.len(), "percent": percent});
        value["uncovered"] = json!(
            real.iter()
                .filter(|l| !covered.contains(l))
                .collect::<Vec<_>>()
        );
    }
    if let Some((covered, total)) = report.code.share() {
        value["code"] =
            json!({"covered": covered, "total": total, "percent": percent(covered, total)});
    }
    if !report.exceptions.is_empty() {
        value["exceptions"] = json!(report.exceptions);
    }
    if !report.scope.is_empty() {
        let scope: Map<String, Value> = report
            .scope
            .iter()
            .map(|c| {
                (
                    c.name.clone(),
                    json!({"counts": c.counts, "ceiling": c.ceiling}),
                )
            })
            .collect();
        value["scope"] = Value::Object(scope);
    }
    value
}

/// `part` of `whole` as a percentage with one decimal; 0 of nothing is 0.
fn percent(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        0.0
    } else {
        (part as f64 * 1000.0 / whole as f64).round() / 10.0
    }
}

pub fn run(root: &Path, write: bool, json: bool, outside: bool) -> anyhow::Result<ExitCode> {
    let mut report = check(root)?;
    let path = root.join("contract/coverage.json");
    let mut published = coverage(&report);
    mutate_results(root, &report.code, &mut published, &mut report.problems);
    let unclaimed: Vec<String> = report
        .code
        .outside
        .iter()
        .filter(|_| outside)
        .map(|f| format!("{}::{}", f.file, f.name))
        .collect();
    let text = serde_json::to_string_pretty(&published)? + "\n";
    if write {
        fs::write(&path, text).context("write contract/coverage.json")?;
    } else if path.exists() && fs::read_to_string(&path)? != text {
        report.problems.push(
            "contract/coverage.json is out of date; run `cargo impish check --write`".to_string(),
        );
    }
    if json {
        let mut doc = json!({
            "version": 1,
            "met": report.problems.is_empty(),
            "obligations": report.obligations,
            "primitives": report.primitives,
            "problems": report.problems,
            "coverage": published,
        });
        if outside {
            doc["outside"] = json!(unclaimed);
        }
        println!("{doc}");
        return Ok(if report.problems.is_empty() {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        });
    }
    if report.problems.is_empty() {
        println!(
            "The Contract holds: {} obligations across {} primitives.",
            report.obligations, report.primitives
        );
        if let Some((covered, total)) = report.code.share() {
            let percent = percent(covered, total);
            println!("code: {covered} of {total} functions in the Contract ({percent}%)");
        }
        for c in &report.scope {
            println!("{}: {} of at most {}", c.name, c.count, c.ceiling);
        }
    }
    if !unclaimed.is_empty() {
        println!("Outside the Contract:");
        for f in &unclaimed {
            println!("  {f}");
        }
    }
    if report.problems.is_empty() {
        return Ok(ExitCode::SUCCESS);
    }
    for problem in &report.problems {
        eprintln!("{problem}");
    }
    eprintln!("The Contract is not met.");
    Ok(ExitCode::FAILURE)
}

/// Carries each primitive's mutate result, recorded by `mutate --write`, into
/// the published coverage, and names the ones whose fingerprint has changed.
fn mutate_results(root: &Path, code: &Resolved, published: &mut Value, problems: &mut Vec<String>) {
    let Some(recorded) = fs::read_to_string(root.join("contract/coverage.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
    else {
        return;
    };
    let Some(primitives) = published["primitives"].as_object_mut() else {
        return;
    };
    for (name, entry) in primitives {
        let result = &recorded["primitives"][name.as_str()]["mutate"];
        if result.is_null() {
            continue;
        }
        let now = fingerprint(name, code.files_of(name), |file| {
            fs::read_to_string(root.join(file)).unwrap_or_default()
        });
        if result["fingerprint"] != now.as_str() {
            problems.push(format!(
                "contract/coverage.json: the mutate result for {name} is stale; \
                 run `cargo impish mutate {name} --write`"
            ));
        }
        entry["mutate"] = result.clone();
    }
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
        scope: Vec::new(),
        exceptions: BTreeMap::new(),
        code: code::resolve(root, &code::readmes(root)),
    };
    let mut cited = Vec::new();
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
        let covers = report.code.problems.get(name).cloned().unwrap_or_default();
        report.problems.extend(covers);
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
        cited.extend(ids.values().map(|o| {
            let at = format!("{readme_file}:{}", o.record_line);
            (at, o.records.clone())
        }));
        let (excused, problems) = exceptions::excused(root, name, &ids);
        report.exceptions.extend(excused);
        report.problems.extend(problems);
        let active: Vec<_> = ids.values().filter(|o| o.needs_test()).collect();
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
                    Some(id) if ids.get(&id).is_some_and(|o| !o.needs_test()) => {
                        report.problems.push(format!(
                            "{name}::{} names `{tag}`, which is an exception",
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
        let active = ids.iter().filter(|(_, o)| o.needs_test()).map(|(id, _)| id);
        for id in active.filter(|id| !proved.contains(*id)) {
            report.problems.push(match ignored.get(id) {
                Some(tests) => format!("{name}/A{id} has no test (ignored: {})", tests.join(", ")),
                None => format!("{name}/A{id} has no test"),
            });
        }
    }
    report.problems.extend(stray::tags(root, &names));
    report.problems.extend(stray::tests(root, &names));
    let intents = std::iter::once("contract/intent.md".to_string()).chain(
        names
            .iter()
            .map(|name| format!("contract/{name}/intent.md")),
    );
    for file in intents {
        if let Ok(text) = fs::read_to_string(root.join(&file)) {
            report.problems.extend(intent(&file, &text));
        }
    }
    let scope = scope::scope(root);
    report.problems.extend(scope.problems);
    report.scope = scope.ceilings;
    cited.extend(scope.cited);
    let (records, problems) = decisions::read(root);
    report.problems.extend(problems);
    report
        .problems
        .extend(decisions::dangling(&records, &cited));
    report.real = real;
    Ok(report)
}

fn defines(root: &Path, file: &str, func: &str) -> bool {
    fs::read_to_string(root.join(file)).is_ok_and(|src| {
        src.contains(&format!("fn {func}(")) || src.contains(&format!("fn {func}<"))
    })
}
