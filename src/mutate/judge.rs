//! What a mutate run found: the kill matrix, and why the Contract isn't met.

use std::collections::BTreeSet;
use std::fmt::Write;

use serde_json::{Value, json};

use super::erase::{self, Key};
use super::{Mutant, Outcome};
use crate::contract::Declared;

#[derive(Default)]
pub(super) struct Verdict {
    pub(super) killed: usize,
    pub(super) lived: usize,
    pub(super) unreached: usize,
    pub(super) timed_out: usize,
    /// Mutants never tried, as their function is erased.
    pub(super) erased: usize,
    /// Uncaught mutants that mutants.md declares; not in the counts above.
    pub(super) declared: usize,
    pub(super) unviable: usize,
    /// Per active obligation: its id, killed and unique counts.
    pub(super) rows: Vec<(String, usize, usize)>,
    /// Per contract test: its name, killed and unique counts.
    pub(super) tests: Vec<(String, usize, usize)>,
    /// The tests with kills but no unique kill, each with the reason it is
    /// kept, if it gives one; only when every mutant was tried.
    pub(super) blind: Vec<(String, Option<String>)>,
    pub(super) problems: Vec<String>,
}

/// Hollow obligations, blind tests, mutants no contract test catches, and
/// declarations that don't hold.
pub(super) fn judge(
    active: &[String],
    tests: &[(&str, Option<&str>)],
    tried: &[(&Mutant, &Outcome)],
    declared: &[Declared],
    declared_file: &str,
    full: bool,
    erased: &BTreeSet<Key>,
) -> Verdict {
    let mut v = Verdict::default();
    for (m, outcome) in tried {
        let is_declared = || declared.iter().any(|d| d.name == key(m));
        *match outcome {
            Outcome::Killed { .. } => &mut v.killed,
            Outcome::Unviable => &mut v.unviable,
            _ if is_declared() => &mut v.declared,
            Outcome::Lived { .. } => &mut v.lived,
            Outcome::Unreached => &mut v.unreached,
            Outcome::TimedOut => &mut v.timed_out,
            Outcome::Erased => &mut v.erased,
        } += 1;
    }
    for id in active {
        let mut killed = 0;
        let mut unique = 0;
        for (_, outcome) in tried {
            if let Outcome::Killed { by, .. } = outcome
                && by.contains(id)
            {
                killed += 1;
                unique += usize::from(by.len() == 1);
            }
        }
        if full && killed == 0 {
            v.problems.push(format!(
                "{id} is hollow: its tests catch none of the mutants"
            ));
        }
        v.rows.push((id.clone(), killed, unique));
    }
    for (test, reason) in tests {
        let mut killed = 0;
        let mut unique = 0;
        for (_, outcome) in tried {
            if let Outcome::Killed { tests, .. } = outcome
                && tests.contains(*test)
            {
                killed += 1;
                unique += usize::from(tests.len() == 1);
            }
        }
        if full && killed == 0 {
            v.problems
                .push(format!("{test} is blind: it catches none of the mutants"));
        }
        if full && killed > 0 && unique == 0 {
            v.blind.push((test.to_string(), reason.map(str::to_string)));
        }
        v.tests.push((test.to_string(), killed, unique));
    }
    let mut reported = BTreeSet::new();
    for (mutant, outcome) in tried {
        let Some(what) = uncaught(outcome) else {
            continue;
        };
        if let Some(f) = erase::of(mutant).filter(|f| erased.contains(f)) {
            if reported.insert(f) {
                v.problems.push(erase::problem(mutant, declared_file));
            }
            continue;
        }
        let name = key(mutant);
        if !declared.iter().any(|d| d.name == name) {
            v.problems.push(format!(
                "{name} {what}: catch it in a contract test, or declare it in {declared_file}"
            ));
        }
    }
    for d in declared {
        let mut by = BTreeSet::new();
        let mut matched = false;
        for (_, outcome) in tried.iter().filter(|(m, _)| key(m) == d.name) {
            matched = true;
            if let Outcome::Killed { by: ids, .. } = outcome {
                by.extend(ids.iter().map(String::as_str));
            }
        }
        let at = format!("{declared_file}:{}: {}", d.line, d.name);
        if !matched {
            if full {
                v.problems.push(format!("{at} matches no mutant"));
            }
        } else if !by.is_empty() {
            let verb = if by.len() == 1 { "catches" } else { "catch" };
            let by = by.into_iter().collect::<Vec<_>>().join(", ");
            v.problems
                .push(format!("{at} is declared {}, but {by} {verb} it", d.kind));
        }
    }
    v
}

/// What an uncaught mutant is: `lived`, `unreached` or `timed out`.
pub(super) fn uncaught(outcome: &Outcome) -> Option<&'static str> {
    match outcome {
        Outcome::Lived { .. } => Some("lived"),
        Outcome::Unreached => Some("unreached"),
        Outcome::TimedOut => Some("timed out"),
        Outcome::Erased => Some("erased"),
        _ => None,
    }
}

/// The run's line for one mutant, and for one no test catches, how to clear
/// it, unless mutants.md declares it.
pub(super) fn line(
    m: &Mutant,
    outcome: &Outcome,
    declared: &[Declared],
    declared_file: &str,
    witnessed: &str,
) -> String {
    let what = match outcome {
        Outcome::Killed { by, .. } => {
            let by: Vec<&str> = by.iter().map(String::as_str).collect();
            return format!("  {}  killed by {}", m.name, by.join(", "));
        }
        Outcome::Unviable => return format!("  {}  unviable", m.name),
        Outcome::Lived { ran } if !ran.is_empty() => {
            let ran: Vec<&str> = ran.iter().map(String::as_str).collect();
            format!("LIVED, run by {}", ran.join(", "))
        }
        other => uncaught(other).unwrap_or_default().to_uppercase(),
    };
    let what = what.replace("TIMED OUT", "timed out");
    if let Some(d) = declared.iter().find(|d| d.name == key(m)) {
        return format!("  {}  {what}, declared {}{witnessed}", m.name, d.kind);
    }
    let clear = match outcome {
        Outcome::Lived { ran } if !ran.is_empty() => {
            lived_clear(&ran.iter().map(String::as_str).collect::<Vec<_>>())
        }
        Outcome::Unreached => format!(
            "no contract test runs this code: write the promise that needs it, \
             delete the code, or declare it unpromised in {declared_file}"
        ),
        Outcome::TimedOut => {
            format!("make the code finish under its tests, or declare it in {declared_file}")
        }
        _ => "make a contract test fail on it".to_string(),
    };
    format!("  {}  {what}{witnessed}\n    to clear: {clear}", m.name)
}

/// How to clear a mutant the `ran` tests ran past.
pub(super) fn lived_clear(ran: &[&str]) -> String {
    format!(
        "make {} fail on it, or write the obligation it breaks and a test for it",
        ran.join(" or ")
    )
}

/// The kill matrix, per obligation and per test, and the tests with no
/// unique kill.
pub(super) fn matrix(v: &Verdict, whole: bool) -> String {
    let mut out = String::new();
    for (heading, rows, unique) in [
        ("obligation", &v.rows, "no other obligation's tests"),
        ("test", &v.tests, "no other contract test"),
    ] {
        let width = rows
            .iter()
            .map(|(name, ..)| name.len())
            .fold(10, usize::max);
        let _ = writeln!(out, "\n  {heading:width$}  killed  unique");
        for (name, killed, only) in rows {
            let _ = writeln!(out, "  {name:width$}  {killed:>6}  {only:>6}");
        }
        let _ = writeln!(out, "  unique: caught by {unique}.");
        if !whole && heading == "obligation" {
            let _ = writeln!(
                out,
                "  Tests outside the Contract weren't run; they may catch it too."
            );
        }
    }
    let (kept, warned): (Vec<_>, Vec<_>) = v.blind.iter().partition(|(_, r)| r.is_some());
    if !warned.is_empty() {
        let _ = writeln!(
            out,
            "\nMutation-blind tests: another contract test catches every mutant they catch.\n\
             Remove them, or say why they stay with blind = \"<reason>\"."
        );
        for (name, _) in warned {
            let _ = writeln!(out, "  {name}");
        }
    }
    if !kept.is_empty() {
        let _ = writeln!(out, "\nKept with no unique kill:");
        for (name, reason) in kept {
            let _ = writeln!(out, "  {name}: {}", reason.as_deref().unwrap_or_default());
        }
    }
    out
}

/// The run as one JSON document, for `--format json`.
pub(super) fn json(
    primitive: &str,
    tried: &[(&Mutant, &Outcome)],
    declared: &[Declared],
    v: &Verdict,
) -> Value {
    let mutants: Vec<Value> = tried
        .iter()
        .map(|(m, outcome)| {
            let none = BTreeSet::new();
            let (name, by, tests, ran) = match outcome {
                Outcome::Killed { by, tests } => ("killed", by, tests, &none),
                Outcome::Lived { ran } => ("lived", &none, &none, ran),
                Outcome::Unreached => ("unreached", &none, &none, &none),
                Outcome::TimedOut => ("timed out", &none, &none, &none),
                Outcome::Unviable => ("unviable", &none, &none, &none),
                Outcome::Erased => ("erased", &none, &none, &none),
            };
            let declared = declared
                .iter()
                .find(|d| uncaught(outcome).is_some() && d.name == key(m))
                .map(|d| d.kind);
            json!({
                "name": m.name,
                "outcome": name,
                "caught_by": by,
                "tests": tests,
                "ran_by": ran,
                "declared": declared,
            })
        })
        .collect();
    let rows: Vec<Value> = v
        .rows
        .iter()
        .map(|(id, killed, unique)| json!({"id": id, "killed": killed, "unique": unique}))
        .collect();
    let tests: Vec<Value> = v
        .tests
        .iter()
        .map(|(name, killed, unique)| {
            let reason = v.blind.iter().find(|(b, _)| b == name).map(|(_, r)| r);
            json!({"name": name, "killed": killed, "unique": unique, "blind": reason.is_some(), "reason": reason.cloned().flatten()})
        })
        .collect();
    json!({
        "version": 1,
        "primitive": primitive,
        "met": v.problems.is_empty(),
        "mutants": mutants,
        "killed": v.killed,
        "lived": v.lived,
        "unreached": v.unreached,
        "timed_out": v.timed_out,
        "erased": v.erased,
        "declared": v.declared,
        "unviable": v.unviable,
        "obligations": rows,
        "tests": tests,
        "problems": v.problems,
    })
}

/// The mutant's name without line and column, as mutants.md declares it.
pub(super) fn key(m: &Mutant) -> String {
    let rest = m.name.strip_prefix(m.file.as_str());
    let rest = rest
        .and_then(|r| r.split_once(": "))
        .map_or(m.name.as_str(), |(_, r)| r);
    format!("{}: {rest}", m.file)
}
