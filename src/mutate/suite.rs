//! What the tests outside the Contract add under `--whole-suite`: witnesses
//! of missing obligations, and duplicates of the Contract.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::Outcome;

/// Each outside test that caught a mutant, and whether any of them was one
/// the Contract misses.
pub(super) type Outside = BTreeMap<String, bool>;

/// Counts the outside tests that caught one mutant.
pub(super) fn tally(outside: &mut Outside, outcome: &Outcome, caught: &BTreeSet<String>) {
    for test in caught {
        let witnessed = outside.entry(test.clone()).or_default();
        *witnessed = *witnessed || matches!(outcome, Outcome::Lived { .. } | Outcome::Unreached);
    }
}

/// Adds one primitive's tally to the run's.
pub(super) fn merge(into: &mut Outside, from: Outside) {
    for (test, witnessed) in from {
        let into = into.entry(test).or_default();
        *into = *into || witnessed;
    }
}

/// The outside tests whose every catch the Contract catches too.
pub(super) fn duplicates(outside: &Outside) -> Vec<String> {
    outside
        .iter()
        .filter(|(_, witnessed)| !**witnessed)
        .map(|(test, _)| test.clone())
        .collect()
}

/// The problem a duplicate is.
pub(super) fn duplicate(test: &str) -> String {
    format!(
        "{test}: every mutant it catches, the Contract catches too, so it duplicates the Contract"
    )
}

/// How the run's line for a mutant names the outside tests that witness it.
pub(super) fn witnessed(outcome: &Outcome, caught: &BTreeSet<String>) -> String {
    if !matches!(outcome, Outcome::Lived { .. } | Outcome::Unreached) || caught.is_empty() {
        return String::new();
    }
    let tests: Vec<&str> = caught.iter().map(String::as_str).collect();
    format!(", witnessed by {}", tests.join(", "))
}

/// Adds to each mutant in the JSON document the outside tests that caught it.
pub(super) fn annotate(doc: &mut Value, caught: &[BTreeSet<String>]) {
    if let Some(mutants) = doc["mutants"].as_array_mut() {
        for (mutant, tests) in mutants.iter_mut().zip(caught) {
            mutant["outside"] = tests.iter().cloned().collect();
        }
    }
}
