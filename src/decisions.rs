//! Decision records: `contract/decisions.toml` and
//! `contract/<primitive>/decisions.toml`, read and checked for shape and
//! references. `check` reports their problems; `conditions` re-checks them.

mod evaluate;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use toml::{Table, Value};

pub use evaluate::{Verdict, evaluate};

const REQUIRED: [&str; 5] = ["id", "date", "decision", "driver", "approved_by"];
const FIELDS: [&str; 11] = [
    "id",
    "date",
    "decision",
    "driver",
    "approved_by",
    "alternatives",
    "consequences",
    "depends_on",
    "superseded_by",
    "conditions",
    "expires",
];
const CONDITION_FIELDS: [&str; 6] = ["text", "dep", "version", "env", "file", "rustc"];

pub struct Record {
    pub id: String,
    /// Where it is declared, as `<file>:<line>`.
    pub at: String,
    pub depends_on: Vec<String>,
    pub superseded_by: Option<String>,
    pub conditions: Vec<Condition>,
}

pub struct Condition {
    pub text: String,
    /// What re-checks it; `None` for a condition only a person can judge.
    pub check: Option<Check>,
}

pub enum Check {
    Dep {
        name: String,
        version: Option<String>,
    },
    Env(String),
    File(String),
    Rustc(String),
}

/// Every record under `contract/`, the project's first, then each
/// primitive's, and a problem for each file or record that is malformed.
pub fn read(root: &Path) -> (Vec<Record>, Vec<String>) {
    let mut files = vec!["contract/decisions.toml".to_string()];
    let mut primitives: Vec<String> = fs::read_dir(root.join("contract"))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    primitives.sort();
    files.extend(
        primitives
            .iter()
            .map(|p| format!("contract/{p}/decisions.toml")),
    );
    let mut records = Vec::new();
    let mut problems = Vec::new();
    for file in files {
        let Ok(text) = fs::read_to_string(root.join(&file)) else {
            continue;
        };
        parse(&file, &text, &mut records, &mut problems);
    }
    let mut first: BTreeMap<&str, &str> = BTreeMap::new();
    for record in &records {
        if let Some(at) = first.insert(&record.id, &record.at) {
            problems.push(format!(
                "{}: `{}` is declared twice (first at {at})",
                record.at, record.id
            ));
            first.insert(&record.id, at);
        }
    }
    (records, problems)
}

/// Each `record:`, `depends_on` or `superseded_by` that names no record.
/// `cited` holds an obligation's place and the records it names.
pub fn dangling(records: &[Record], cited: &[(String, Vec<String>)]) -> Vec<String> {
    let known: Vec<&str> = records.iter().map(|r| r.id.as_str()).collect();
    let mut problems = Vec::new();
    let mut each = |at: &str, field: &str, id: &str| {
        if !known.contains(&id) {
            problems.push(format!("{at}: {field} `{id}` names no decision record"));
        }
    };
    for (at, ids) in cited {
        ids.iter().for_each(|id| each(at, "record", id));
    }
    for r in records {
        r.depends_on
            .iter()
            .for_each(|id| each(&r.at, "depends_on", id));
        if let Some(id) = &r.superseded_by {
            each(&r.at, "superseded_by", id);
        }
    }
    problems
}

fn parse(file: &str, text: &str, records: &mut Vec<Record>, problems: &mut Vec<String>) {
    let table: Table = match toml::from_str(text) {
        Ok(table) => table,
        Err(e) => {
            let line = e
                .span()
                .map_or(1, |span| text[..span.start].lines().count().max(1));
            problems.push(format!("{file}:{line}: isn't valid TOML: {}", e.message()));
            return;
        }
    };
    // The line of each `[[record]]`, so a problem names the record it is in.
    let headers: Vec<usize> = text
        .lines()
        .enumerate()
        .filter(|(_, l)| l.trim() == "[[record]]")
        .map(|(i, _)| i + 1)
        .collect();
    for key in table.keys().filter(|k| *k != "record") {
        problems.push(format!("{file}: unknown field `{key}`"));
    }
    let Some(list) = table.get("record") else {
        return;
    };
    let Some(list) = list.as_array() else {
        problems.push(format!(
            "{file}: `record` must be a list of [[record]] tables"
        ));
        return;
    };
    for (i, value) in list.iter().enumerate() {
        let at = format!("{file}:{}", headers.get(i).copied().unwrap_or(1));
        match value.as_table() {
            Some(fields) => records.extend(record(&at, fields, problems)),
            None => problems.push(format!("{at}: a record must be a table")),
        }
    }
}

fn record(at: &str, fields: &Table, problems: &mut Vec<String>) -> Option<Record> {
    for key in fields.keys().filter(|k| !FIELDS.contains(&k.as_str())) {
        problems.push(format!("{at}: unknown field `{key}`"));
    }
    for name in REQUIRED.into_iter().filter(|n| !fields.contains_key(*n)) {
        problems.push(format!("{at}: no `{name}`"));
    }
    for (name, value) in fields {
        let fits = match name.as_str() {
            "date" | "expires" => is_date(value),
            "alternatives" | "consequences" | "depends_on" => strings(value).is_some(),
            "conditions" => value.is_array(),
            _ => value.is_str(),
        };
        if FIELDS.contains(&name.as_str()) && !fits {
            let want = match name.as_str() {
                "date" | "expires" => "a date",
                "alternatives" | "consequences" | "depends_on" => "a list of strings",
                "conditions" => "a list of [[record.conditions]] tables",
                _ => "a string",
            };
            problems.push(format!("{at}: `{name}` must be {want}"));
        }
    }
    let conditions = fields
        .get("conditions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|c| condition(at, c, problems))
        .collect();
    // A malformed record still counts as declared, so references to it
    // don't add problems of their own.
    let text = |name: &str| fields.get(name).and_then(Value::as_str).map(String::from);
    Some(Record {
        id: text("id")?,
        at: at.to_string(),
        depends_on: fields
            .get("depends_on")
            .and_then(strings)
            .unwrap_or_default(),
        superseded_by: text("superseded_by"),
        conditions,
    })
}

fn condition(at: &str, value: &Value, problems: &mut Vec<String>) -> Option<Condition> {
    let Some(fields) = value.as_table() else {
        problems.push(format!("{at}: a condition must be a table"));
        return None;
    };
    for key in fields
        .keys()
        .filter(|k| !CONDITION_FIELDS.contains(&k.as_str()))
    {
        problems.push(format!("{at}: unknown condition field `{key}`"));
    }
    for (name, _) in fields.iter().filter(|(_, v)| !v.is_str()) {
        problems.push(format!("{at}: condition `{name}` must be a string"));
    }
    let get = |name: &str| fields.get(name).and_then(Value::as_str).map(String::from);
    let Some(text) = get("text") else {
        problems.push(format!("{at}: a condition has no `text`"));
        return None;
    };
    let kinds: Vec<&str> = ["dep", "env", "file", "rustc"]
        .into_iter()
        .filter(|k| fields.contains_key(*k))
        .collect();
    if kinds.len() > 1 {
        problems.push(format!(
            "{at}: condition `{text}` has more than one check: {}",
            kinds.join(", ")
        ));
    }
    if fields.contains_key("version") && !fields.contains_key("dep") {
        problems.push(format!(
            "{at}: condition `{text}` has a `version` but no `dep`"
        ));
    }
    let check = match kinds.first().copied() {
        Some("dep") => get("dep").map(|name| Check::Dep {
            name,
            version: get("version"),
        }),
        Some("env") => get("env").map(Check::Env),
        Some("file") => get("file").map(Check::File),
        Some("rustc") => get("rustc").map(Check::Rustc),
        _ => None,
    };
    Some(Condition { text, check })
}

fn is_date(value: &Value) -> bool {
    value
        .as_datetime()
        .is_some_and(|d| d.date.is_some() && d.time.is_none())
}

fn strings(value: &Value) -> Option<Vec<String>> {
    value
        .as_array()?
        .iter()
        .map(|v| v.as_str().map(String::from))
        .collect()
}
