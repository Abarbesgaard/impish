//! `cargo impish mutate <primitive>`: break the code a primitive covers, one
//! mutant at a time, and credit each kill to the obligations tagged on the
//! contract tests that caught it. cargo-mutants supplies the mutants.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use anyhow::{Context, anyhow, bail};
use serde::Deserialize;
use serde_json::{Map, Value, json};

mod judge;
mod list;
mod pool;
mod record;
mod scratch;
mod since;
mod suite;

use judge::{Verdict, judge};
use list::{cargo, list};
use record::record;
use scratch::Scratch;
use since::Changes;
use suite::Outside;

use crate::code;
use crate::contract::{Declared, declarations, tagged_tests};
use impish_core::obligations;

#[derive(Deserialize)]
struct Mutant {
    name: String,
    file: String,
    replacement: String,
    span: Span,
}

#[derive(Deserialize)]
struct Span {
    start: Position,
    end: Position,
}

/// 1-based line, 1-based column in characters, as cargo-mutants writes them.
#[derive(Deserialize)]
struct Position {
    line: usize,
    column: usize,
}

enum Outcome {
    /// Caught: the active obligations tagged on the tests that failed, and
    /// those tests.
    Killed {
        by: BTreeSet<String>,
        tests: BTreeSet<String>,
    },
    Survived,
    /// The mutant doesn't build, so it proves nothing.
    Unviable,
    TimedOut,
}

/// Mutates the primitive, or every primitive that covers code; with a
/// `since` ref, only the mutants on lines changed since its merge base (see
/// `Changes::touch_contract` for the flag); with
/// the `whole` suite, the tests outside the Contract too; up to `jobs`
/// mutants at once.
pub fn run(
    root: &Path,
    primitive: Option<&str>,
    since: Option<(&str, bool)>,
    whole: bool,
    write: bool,
    json: bool,
    jobs: usize,
) -> anyhow::Result<ExitCode> {
    let names = match primitive {
        Some(name) => vec![name.to_string()],
        None => primitives(root)?,
    };
    let changes = since.map(|(r, c)| Changes::since(root, r, c)).transpose()?;
    let mut met = true;
    let mut docs = Map::new();
    let mut outside = Outside::new();
    let opts = Opts {
        every: primitive.is_none(),
        changes: changes.as_ref(),
        whole,
        write,
        json,
        jobs: jobs.max(1),
    };
    for name in &names {
        let ran = mutate(root, name, opts)?;
        if let Some(ran) = ran {
            met &= ran.problems.is_empty();
            docs.insert(name.clone(), ran.json);
            suite::merge(&mut outside, ran.outside);
        }
    }
    // An outside test may guard another primitive's code, or lines a
    // `--since` run skips, so only a run of everything finds a duplicate.
    let duplicates = if whole && primitive.is_none() && since.is_none() {
        suite::duplicates(&outside)
    } else {
        if whole && !json {
            println!(
                "Duplicates outside the Contract need a run of every primitive, with no --since.\n"
            );
        }
        Vec::new()
    };
    met &= duplicates.is_empty();
    if json {
        match primitive {
            Some(name) => println!("{}", docs[name]),
            None => println!(
                "{}",
                json!({"version": 1, "met": met, "primitives": docs, "duplicates": duplicates})
            ),
        }
    } else if !met {
        for test in &duplicates {
            eprintln!("{}", suite::duplicate(test));
        }
        eprintln!("The Contract is not met.");
    }
    Ok(if met {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// The directories under contract/ with a README, in order.
fn primitives(root: &Path) -> anyhow::Result<Vec<String>> {
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

/// How each primitive is mutated.
#[derive(Clone, Copy)]
struct Opts<'a> {
    every: bool,
    changes: Option<&'a Changes>,
    whole: bool,
    write: bool,
    json: bool,
    jobs: usize,
}

/// What one primitive's run found.
struct Ran {
    problems: Vec<String>,
    json: Value,
    outside: Outside,
}

/// Mutates one primitive and reports it; `None` when `every` primitive is
/// being mutated and this one covers no code.
fn mutate(root: &Path, primitive: &str, opts: Opts) -> anyhow::Result<Option<Ran>> {
    let Opts {
        every,
        changes,
        whole,
        write,
        json,
        jobs,
    } = opts;
    let readme_file = format!("contract/{primitive}/README.md");
    let readme =
        fs::read_to_string(root.join(&readme_file)).with_context(|| format!("no {readme_file}"))?;
    let ids =
        obligations(&readme, &readme_file).map_err(|problems| anyhow!(problems.join("\n")))?;
    let active: Vec<String> = ids
        .iter()
        .filter(|(_, o)| o.needs_test())
        .map(|(id, _)| format!("{primitive}/A{id}"))
        .collect();
    let code = code::resolve(root, &code::readmes(root));
    if let Some(problems) = code.problems.get(primitive).filter(|p| !p.is_empty()) {
        bail!(problems.join("\n"));
    }
    let owned = code.owned.get(primitive).cloned().unwrap_or_default();
    if owned.is_empty() {
        if every {
            if !json {
                println!("{primitive}: covers no code, so there is nothing to mutate\n");
            }
            return Ok(None);
        }
        bail!("{readme_file} covers no code, so there is nothing to mutate");
    }
    let declared_file = format!("contract/{primitive}/mutants.md");
    let declared_text = match fs::read_to_string(root.join(&declared_file)) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e).with_context(|| format!("read {declared_file}")),
    };
    let (declared, unreasoned) = declarations(&declared_text);
    if !unreasoned.is_empty() {
        let problems = unreasoned
            .into_iter()
            .map(|(line, kind)| {
                format!(
                    "{declared_file}:{line}: an {kind} block with no ## heading giving its reason"
                )
            })
            .collect();
        let verdict = Verdict {
            problems,
            ..Verdict::default()
        };
        if write {
            record(root, primitive, code.files_of(primitive), false)?;
        }
        return Ok(Some(finish(
            primitive,
            &[],
            &declared,
            verdict,
            whole,
            json,
        )));
    }
    let tests_file = format!("tests/contract/{primitive}.rs");
    let tests =
        fs::read_to_string(root.join(&tests_file)).with_context(|| format!("no {tests_file}"))?;
    let tagged = tagged_tests(&tests).with_context(|| format!("{tests_file} isn't valid Rust"))?;
    let reasons: BTreeMap<String, Option<String>> = tagged
        .iter()
        .map(|t| (format!("{primitive}::{}", t.name), t.blind.clone()))
        .collect();
    let tags: BTreeMap<String, Vec<String>> = tagged
        .into_iter()
        .map(|t| (format!("{primitive}::{}", t.name), t.tags))
        .collect();

    let mut mutants = list(root, &owned)?;
    if mutants.is_empty() {
        bail!("cargo-mutants finds no mutants in the code {readme_file} covers");
    }
    // A run that sees only some mutants can't call an obligation hollow, a
    // test blind, or a declaration unmatched.
    let full = changes.is_none_or(|c| c.touch_contract(primitive));
    if let Some(changes) = changes.filter(|_| !full) {
        mutants.retain(|m| changes.touch(&m.file, m.span.start.line, m.span.end.line));
        if mutants.is_empty() {
            if !json {
                println!(
                    "{primitive}: no covered line changed since {}\n",
                    changes.reference
                );
            }
            return Ok(Some(finish(
                primitive,
                &[],
                &declared,
                Verdict::default(),
                whole,
                json,
            )));
        }
    }
    let scratches = (0..jobs.min(mutants.len()))
        .map(|worker| Scratch::copy(root, whole, worker))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let scratch = &scratches[0];
    let built = scratch
        .build()?
        .ok_or_else(|| anyhow!("the tests don't build"))?;
    let started = Instant::now();
    let mut failed = match scratch.test(&built.contract, Some(primitive), None)? {
        Some(failed) => failed,
        None => bail!("the contract tests time out before any mutation"),
    };
    if whole {
        failed.extend(scratch.test_outside(&built, None)?.unwrap_or_default());
    }
    if !failed.is_empty() {
        let failed: Vec<_> = failed.into_iter().collect();
        bail!(
            "the baseline is red: {} fail before any mutation",
            failed.join(", ")
        );
    }
    // Mutants tried at once share the CPU, so each runs slower than this.
    let timeout = started.elapsed() * 5 * scratches.len() as u32 + Duration::from_secs(10);
    if !json {
        println!(
            "{primitive}: {} mutants of {} functions, {} tests",
            mutants.len(),
            owned.len(),
            tags.len()
        );
    }

    let mut outcomes = Vec::new();
    let mut caught = Vec::new();
    let mut outside = Outside::new();
    let work = |scratch: &Scratch, i: usize| {
        scratch.try_mutant(&mutants[i], primitive, timeout, &tags, &active)
    };
    let mut i = 0;
    pool::each(&scratches, mutants.len(), work, |(outcome, by_outside)| {
        let mutant = &mutants[i];
        i += 1;
        let mut verdict = match &outcome {
            Outcome::Killed { by, .. } => {
                format!(
                    "killed by {}",
                    by.iter().cloned().collect::<Vec<_>>().join(", ")
                )
            }
            Outcome::Survived => "SURVIVED".to_string(),
            Outcome::Unviable => "unviable".to_string(),
            Outcome::TimedOut => "timed out".to_string(),
        };
        if matches!(outcome, Outcome::Survived | Outcome::TimedOut)
            && let Some(d) = declared.iter().find(|d| d.name == key(mutant))
        {
            verdict = format!("{verdict}, declared {}", d.kind);
        }
        if !json {
            let witnessed = suite::witnessed(&outcome, &by_outside);
            println!("  {}  {verdict}{witnessed}", mutant.name);
        }
        suite::tally(&mut outside, &outcome, &by_outside);
        caught.push(by_outside);
        outcomes.push(outcome);
    })?;
    let tried: Vec<_> = mutants.iter().zip(&outcomes).collect();
    let tests: Vec<(&str, Option<&str>)> = reasons
        .iter()
        .map(|(name, reason)| (name.as_str(), reason.as_deref()))
        .collect();
    let verdict = judge(&active, &tests, &tried, &declared, &declared_file, full);
    if write {
        record(
            root,
            primitive,
            code.files_of(primitive),
            verdict.problems.is_empty(),
        )?;
    }
    let mut ran = finish(primitive, &tried, &declared, verdict, whole, json);
    if whole {
        suite::annotate(&mut ran.json, &caught);
    }
    ran.outside = outside;
    Ok(Some(ran))
}

/// The mutant's name without line and column, as mutants.md declares it.
fn key(m: &Mutant) -> String {
    let rest = m.name.strip_prefix(m.file.as_str());
    let rest = rest
        .and_then(|r| r.split_once(": "))
        .map_or(m.name.as_str(), |(_, r)| r);
    format!("{}: {rest}", m.file)
}

/// Reports one primitive's verdict as text, unless the run reports JSON.
fn finish(
    primitive: &str,
    tried: &[(&Mutant, &Outcome)],
    declared: &[Declared],
    v: Verdict,
    whole: bool,
    json: bool,
) -> Ran {
    let doc = judge::json(primitive, tried, declared, &v);
    if json {
        return Ran {
            problems: v.problems,
            json: doc,
            outside: Outside::new(),
        };
    }
    if !tried.is_empty() {
        println!(
            "\n{} killed, {} survived, {} timed out, {} unviable",
            v.killed, v.survived, v.timed_out, v.unviable
        );
        print!("{}", judge::matrix(&v, whole));
    }
    if v.problems.is_empty() && !tried.is_empty() {
        println!("\nEvery mutant of {primitive} is caught or declared.");
    }
    for problem in &v.problems {
        eprintln!("{problem}");
    }
    Ran {
        problems: v.problems,
        json: doc,
        outside: Outside::new(),
    }
}
