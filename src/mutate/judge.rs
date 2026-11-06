//! What a mutate run found: the kill matrix, and why the Contract isn't met.

use std::collections::BTreeSet;
use std::fmt::Write;

use serde_json::{Value, json};

use super::{Mutant, Outcome, key};
use crate::contract::Declared;

#[derive(Default)]
pub(super) struct Verdict {
    pub(super) killed: usize,
    pub(super) survived: usize,
    pub(super) timed_out: usize,
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
) -> Verdict {
    let mut v = Verdict::default();
    for (_, outcome) in tried {
        *match outcome {
            Outcome::Killed { .. } => &mut v.killed,
            Outcome::Survived => &mut v.survived,
            Outcome::TimedOut => &mut v.timed_out,
            Outcome::Unviable => &mut v.unviable,
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
    for (mutant, outcome) in tried {
        let what = match outcome {
            Outcome::Survived => "survived",
            Outcome::TimedOut => "timed out",
            _ => continue,
        };
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

/// The kill matrix, per obligation and per test, and the tests with no
/// unique kill.
pub(super) fn matrix(v: &Verdict, whole: bool) -> String {
    let mut out = String::new();
    for (heading, rows, unique) in [
        ("obligation", &v.rows, "no other obligation's tests"),
        ("test", &v.tests, "no other contract test"),
    ] {
        let width = rows.iter().map(|(name, ..)| name.len()).fold(10, usize::max);
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
            let (name, by, tests) = match outcome {
                Outcome::Killed { by, tests } => ("killed", by.clone(), tests.clone()),
                Outcome::Survived => ("survived", BTreeSet::new(), BTreeSet::new()),
                Outcome::TimedOut => ("timed out", BTreeSet::new(), BTreeSet::new()),
                Outcome::Unviable => ("unviable", BTreeSet::new(), BTreeSet::new()),
            };
            let declared = declared
                .iter()
                .find(|d| {
                    matches!(outcome, Outcome::Survived | Outcome::TimedOut) && d.name == key(m)
                })
                .map(|d| d.kind);
            json!({
                "name": m.name,
                "outcome": name,
                "caught_by": by,
                "tests": tests,
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
        "survived": v.survived,
        "timed_out": v.timed_out,
        "unviable": v.unviable,
        "obligations": rows,
        "tests": tests,
        "problems": v.problems,
    })
}
