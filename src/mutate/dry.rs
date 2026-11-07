//! `--dry-run`: the mutants a run would try, and which are declared, without
//! building anything.

use serde_json::{Value, json};

use super::judge::key;
use super::list::Mutant;
use crate::contract::Declared;

/// How mutants.md declares the mutant, if it does.
fn kind<'a>(m: &Mutant, declared: &[Declared<'a>]) -> Option<&'a str> {
    declared.iter().find(|d| d.name == key(m)).map(|d| d.kind)
}

/// Each mutant the run would try on a line, with its declaration, and the
/// count.
pub(super) fn text(
    primitive: &str,
    functions: usize,
    mutants: &[Mutant],
    declared: &[Declared],
) -> String {
    let mut out = format!(
        "{primitive}: {} mutants of {functions} functions, a dry run\n",
        mutants.len()
    );
    let mut n = 0;
    for m in mutants {
        let declared = kind(m, declared).map(|k| {
            n += 1;
            format!("  declared {k}")
        });
        out.push_str(&format!("  {}{}\n", m.name, declared.unwrap_or_default()));
    }
    out.push_str(&format!(
        "\n{} mutants, {n} declared; nothing built or tested\n\n",
        mutants.len()
    ));
    out
}

/// The dry run as one JSON document, for `--format json`.
pub(super) fn json(primitive: &str, mutants: &[Mutant], declared: &[Declared]) -> Value {
    let listed: Vec<Value> = mutants
        .iter()
        .map(|m| json!({"name": m.name, "declared": kind(m, declared)}))
        .collect();
    json!({
        "version": 1,
        "primitive": primitive,
        "dry_run": true,
        "met": true,
        "mutants": listed,
        "count": mutants.len(),
    })
}
