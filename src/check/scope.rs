//! Scope ceilings: `contract/scope.toml` caps what the crate can grow to,
//! counted in files, non-blank lines or functions, so a machine says when it
//! has outgrown what one person can read (rule 17).

use std::fs;
use std::path::Path;

use globset::GlobBuilder;

use crate::code::files;
use syn::visit::{self, Visit};
use toml::{Table, Value};

const FILE: &str = "contract/scope.toml";
const REQUIRED: [&str; 5] = ["name", "counts", "glob", "ceiling", "why"];
const FIELDS: [&str; 6] = ["name", "counts", "glob", "ceiling", "why", "record"];
const COUNTS: [&str; 3] = ["files", "lines", "functions"];

/// A ceiling that could be counted, held or not.
pub struct Ceiling {
    pub name: String,
    pub counts: String,
    pub count: usize,
    pub ceiling: usize,
}

#[derive(Default)]
pub struct Scope {
    pub ceilings: Vec<Ceiling>,
    pub problems: Vec<String>,
    /// Each ceiling's place and the decision record it names.
    pub cited: Vec<(String, Vec<String>)>,
}

pub fn scope(root: &Path) -> Scope {
    let mut scope = Scope::default();
    let Ok(text) = fs::read_to_string(root.join(FILE)) else {
        return scope;
    };
    let table: Table = match toml::from_str(&text) {
        Ok(table) => table,
        Err(e) => {
            let line = e
                .span()
                .map_or(1, |span| text[..span.start].lines().count().max(1));
            let why = e.message();
            scope
                .problems
                .push(format!("{FILE}:{line}: isn't valid TOML: {why}"));
            return scope;
        }
    };
    let headers: Vec<usize> = text
        .lines()
        .enumerate()
        .filter(|(_, l)| l.trim() == "[[ceiling]]")
        .map(|(i, _)| i + 1)
        .collect();
    for key in table.keys().filter(|k| *k != "ceiling") {
        scope
            .problems
            .push(format!("{FILE}: unknown field `{key}`"));
    }
    let list = table.get("ceiling").map(|v| v.as_array().ok_or(v));
    let Some(Ok(list)) = list else {
        if let Some(Err(_)) = list {
            let want = "a list of [[ceiling]] tables";
            scope
                .problems
                .push(format!("{FILE}: `ceiling` must be {want}"));
        }
        return scope;
    };
    let files = files(root);
    for (i, value) in list.iter().enumerate() {
        let at = format!("{FILE}:{}", headers.get(i).copied().unwrap_or(1));
        match value.as_table() {
            Some(fields) => ceiling(root, &files, &at, fields, &mut scope),
            None => scope
                .problems
                .push(format!("{at}: a ceiling must be a table")),
        }
    }
    scope
}

fn ceiling(root: &Path, files: &[String], at: &str, fields: &Table, scope: &mut Scope) {
    let problems = &mut scope.problems;
    for key in fields.keys().filter(|k| !FIELDS.contains(&k.as_str())) {
        problems.push(format!("{at}: unknown field `{key}`"));
    }
    for name in REQUIRED.into_iter().filter(|n| !fields.contains_key(*n)) {
        problems.push(format!("{at}: no `{name}`"));
    }
    let text = |name: &str| fields.get(name).and_then(Value::as_str);
    for name in ["name", "counts", "glob", "why", "record"] {
        if fields.contains_key(name) && text(name).is_none() {
            problems.push(format!("{at}: `{name}` must be a string"));
        }
    }
    let limit = fields
        .get("ceiling")
        .map(|v| v.as_integer().filter(|n| *n > 0));
    if let Some(None) = limit {
        problems.push(format!("{at}: `ceiling` must be a whole number above 0"));
    }
    if let Some(counts) = text("counts").filter(|c| !COUNTS.contains(c)) {
        let kinds = COUNTS.join(", ");
        problems.push(format!("{at}: counts `{counts}`, not one of {kinds}"));
    }
    if let Some(record) = text("record") {
        scope.cited.push((at.to_string(), vec![record.to_string()]));
    }
    let matcher = text("glob").map(|g| GlobBuilder::new(g).literal_separator(true).build());
    let matched: Vec<&String> = match matcher {
        Some(Ok(glob)) => {
            let glob = glob.compile_matcher();
            let matched: Vec<_> = files.iter().filter(|f| glob.is_match(f)).collect();
            if matched.is_empty() {
                let glob = text("glob").unwrap_or_default();
                problems.push(format!("{at}: glob `{glob}` matches no file"));
            }
            matched
        }
        Some(Err(e)) => {
            problems.push(format!("{at}: glob doesn't parse: {e}"));
            Vec::new()
        }
        None => Vec::new(),
    };
    let (Some(name), Some(counts), Some(Some(limit))) = (text("name"), text("counts"), limit)
    else {
        return;
    };
    let read = |file: &&String| fs::read_to_string(root.join(file)).unwrap_or_default();
    let count = match counts {
        "files" => matched.len(),
        "lines" => matched
            .iter()
            .map(|f| read(f).lines().filter(|l| !l.trim().is_empty()).count())
            .sum(),
        _ => matched
            .iter()
            .filter(|f| f.ends_with(".rs"))
            .map(|f| functions(&read(f)))
            .sum(),
    };
    let ceiling = usize::try_from(limit).unwrap_or(usize::MAX);
    if count > ceiling {
        problems.push(format!(
            "{at}: `{name}` counts {count} {counts}, over its ceiling of {ceiling}"
        ));
    }
    scope.ceilings.push(Ceiling {
        name: name.to_string(),
        counts: counts.to_string(),
        count,
        ceiling,
    });
}

/// The fn items in `src`, methods and nested ones included; 0 when it
/// doesn't parse.
fn functions(src: &str) -> usize {
    #[derive(Default)]
    struct Count(usize);
    impl<'ast> Visit<'ast> for Count {
        fn visit_signature(&mut self, sig: &'ast syn::Signature) {
            self.0 += 1;
            visit::visit_signature(self, sig);
        }
    }
    let mut count = Count::default();
    if let Ok(file) = syn::parse_file(src) {
        count.visit_file(&file);
    }
    count.0
}
