//! A function no contract test misses: when every replacement of its whole
//! body goes uncaught, the run reports the function once and doesn't try
//! its other mutants.

use std::collections::{BTreeMap, BTreeSet};

use super::judge::{self, key, lived_clear, uncaught};
use super::suite::witnessed;
use super::{Mutant, Outcome};
use crate::contract::Declared;

/// A function, by its file and first line.
pub(super) type Key = (String, usize);

/// The function the mutant is in, if cargo-mutants names one.
pub(super) fn of(m: &Mutant) -> Option<Key> {
    m.function
        .as_ref()
        .map(|f| (m.file.clone(), f.span.start.line))
}

fn body(m: &Mutant) -> bool {
    m.genre == "FnValue"
}

/// The mutants a pass tries, in order: the first only body replacements,
/// the second every mutant not yet decided.
pub(super) fn pass(mutants: &[Mutant], outcomes: &[Option<Outcome>], first: bool) -> Vec<usize> {
    (0..mutants.len())
        .filter(|&i| outcomes[i].is_none() && (!first || body(&mutants[i])))
        .collect()
}

/// The functions whose every viable body replacement went uncaught and
/// undeclared, and that have other mutants to leave out; those mutants
/// become `Erased`.
pub(super) fn erase(
    mutants: &[Mutant],
    outcomes: &mut [Option<Outcome>],
    declared: &[Declared],
) -> BTreeSet<Key> {
    // Per function: every body replacement uncaught, one is, other mutants.
    let mut fns: BTreeMap<Key, (bool, bool, bool)> = BTreeMap::new();
    for (m, outcome) in mutants.iter().zip(outcomes.iter()) {
        let Some(k) = of(m) else { continue };
        let f = fns.entry(k).or_insert((true, false, false));
        if !body(m) {
            f.2 = true;
            continue;
        }
        match outcome {
            Some(Outcome::Unviable) => {}
            Some(o) if uncaught(o).is_some() && !declared.iter().any(|d| d.name == key(m)) => {
                f.1 = true;
            }
            _ => f.0 = false,
        }
    }
    let erased: BTreeSet<Key> = fns
        .into_iter()
        .filter(|(_, (all, any, others))| *all && *any && *others)
        .map(|(k, _)| k)
        .collect();
    for (m, outcome) in mutants.iter().zip(outcomes) {
        if outcome.is_none() && of(m).is_some_and(|f| erased.contains(&f)) {
            *outcome = Some(Outcome::Erased);
        }
    }
    erased
}

/// The run's lines, in cargo-mutants' order, for the mutants decided since
/// the last call, up to the first still waiting: an erased function once,
/// in place of its mutants.
pub(super) fn ready(
    (next, seen): &mut (usize, BTreeSet<Key>),
    (mutants, outcomes, caught): (&[Mutant], &[Option<Outcome>], &[BTreeSet<String>]),
    erased: &BTreeSet<Key>,
    declared: &[Declared],
    declared_file: &str,
) -> String {
    let mut out = String::new();
    while let Some(Some(outcome)) = outcomes.get(*next) {
        let m = &mutants[*next];
        let line = match of(m).filter(|f| erased.contains(f)) {
            Some(f) if seen.insert(f.clone()) => line(m, mutants, outcomes, declared_file),
            Some(_) => String::new(),
            None => {
                let witnessed = witnessed(outcome, &caught[*next]);
                judge::line(m, outcome, declared, declared_file, &witnessed)
            }
        };
        if !line.is_empty() {
            out.push_str(&line);
            out.push('\n');
        }
        *next += 1;
    }
    out
}

/// The run's line for an erased function, and how to clear it.
pub(super) fn line(
    m: &Mutant,
    mutants: &[Mutant],
    outcomes: &[Option<Outcome>],
    declared_file: &str,
) -> String {
    let mut ran = BTreeSet::new();
    let mut unreached = true;
    for (other, outcome) in mutants.iter().zip(outcomes) {
        if body(other) && of(other) == of(m) {
            if let Some(Outcome::Lived { ran: by }) = outcome {
                ran.extend(by.iter().map(String::as_str));
            }
            unreached &= matches!(outcome, Some(Outcome::Unreached | Outcome::Unviable));
        }
    }
    let f = name(m);
    let what = format!("ERASED: no contract test notices {f} doing nothing");
    let (what, clear) = if !ran.is_empty() {
        let ran: Vec<&str> = ran.into_iter().collect();
        (
            format!("{what}, run by {}", ran.join(", ")),
            lived_clear(&ran),
        )
    } else if unreached {
        (
            format!("{what}, and none runs it"),
            format!(
                "write the promise that needs it, delete it, \
                 or declare its mutants unpromised in {declared_file}"
            ),
        )
    } else {
        (what, "make a contract test fail on it".to_string())
    };
    format!("  {}: {f}  {what}\n    to clear: {clear}", m.file)
}

/// Why an erased function fails the run.
pub(super) fn problem(m: &Mutant, declared_file: &str) -> String {
    format!(
        "{}: {} erased: no contract test notices it doing nothing; \
         catch it in a contract test, or declare its mutants in {declared_file}",
        m.file,
        name(m)
    )
}

fn name(m: &Mutant) -> &str {
    m.function.as_ref().map_or("", |f| f.function_name.as_str())
}
