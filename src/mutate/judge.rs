//! What a mutate run found: the kill matrix, and why the Contract isn't met.

use std::collections::BTreeSet;

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
    pub(super) problems: Vec<String>,
}

/// Hollow obligations, blind tests, mutants no contract test catches, and
/// declarations that don't hold.
pub(super) fn judge(
    active: &[String],
    tests: &[&str],
    tried: &[(&Mutant, &Outcome)],
    declared: &[Declared],
    declared_file: &str,
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
        if killed == 0 {
            v.problems.push(format!(
                "{id} is hollow: its tests catch none of the mutants"
            ));
        }
        v.rows.push((id.clone(), killed, unique));
    }
    for test in tests {
        let catches = tried
            .iter()
            .any(|(_, o)| matches!(o, Outcome::Killed { tests, .. } if tests.contains(*test)));
        if !catches {
            v.problems
                .push(format!("{test} is blind: it catches none of the mutants"));
        }
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
            v.problems.push(format!("{at} matches no mutant"));
        } else if !by.is_empty() {
            let verb = if by.len() == 1 { "catches" } else { "catch" };
            let by = by.into_iter().collect::<Vec<_>>().join(", ");
            v.problems
                .push(format!("{at} is declared {}, but {by} {verb} it", d.kind));
        }
    }
    v
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
        "problems": v.problems,
    })
}
