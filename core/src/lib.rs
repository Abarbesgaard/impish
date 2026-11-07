//! Reading a primitive's README: the obligations it declares.

use std::collections::BTreeMap;

const KINDS: [&str; 5] = [
    "functional",
    "non-functional",
    "persisted-format",
    "ordering",
    "security",
];
const FIELDS: [&str; 4] = ["must", "why", "kind", "status"];

pub struct Obligation {
    /// A retired obligation keeps its number and needs no test.
    pub retired: bool,
}

struct Block<'a> {
    id: u32,
    line: usize,
    fields: BTreeMap<&'a str, &'a str>,
}

/// The obligations a primitive's README declares under `## Obligations`.
/// Every line there must belong to a block; `file` names the README in the
/// problems, which are sorted by line.
pub fn obligations(readme: &str, file: &str) -> Result<BTreeMap<u32, Obligation>, Vec<String>> {
    let mut found = BTreeMap::new();
    let mut problems = Vec::new();
    let mut in_section = false;
    let mut in_fence = false;
    let mut block = None;
    for (i, line) in readme.lines().enumerate() {
        let n = i + 1;
        let line = line.trim_end();
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if line.starts_with("## ") {
            finish(block.take(), &mut found, &mut problems);
            in_section = line == "## Obligations";
            continue;
        }
        if !in_section {
            if line.starts_with("### A") || line.starts_with("- **A") {
                problems.push((n, "an obligation outside ## Obligations".to_string()));
            }
            continue;
        }
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("### ") {
            finish(block.take(), &mut found, &mut problems);
            match heading(rest) {
                Some(id) => {
                    block = Some(Block {
                        id,
                        line: n,
                        fields: BTreeMap::new(),
                    })
                }
                None => problems.push((n, format!("malformed obligation heading `{line}`"))),
            }
            continue;
        }
        match (field(line), block.as_mut()) {
            (Some((name, _)), Some(_)) if !FIELDS.contains(&name) => {
                problems.push((n, format!("unknown field `{name}`")))
            }
            (Some((name, value)), Some(block)) => {
                if block.fields.insert(name, value).is_some() {
                    problems.push((n, format!("`{name}` is given twice")));
                }
            }
            _ => problems.push((n, format!("`{line}` is not part of an obligation block"))),
        }
    }
    finish(block, &mut found, &mut problems);
    if !problems.is_empty() {
        problems.sort_by_key(|(n, _)| *n);
        return Err(problems
            .into_iter()
            .map(|(n, problem)| format!("{file}:{n}: {problem}"))
            .collect());
    }
    if found.is_empty() {
        return Err(vec![format!("{file}: declares no obligations")]);
    }
    Ok(found)
}

/// The number in `A<n> — <title>`, the part of a heading after `### `.
fn heading(rest: &str) -> Option<u32> {
    let (num, title) = rest.strip_prefix('A')?.split_once(' ')?;
    title
        .strip_prefix("— ")
        .or_else(|| title.strip_prefix("- "))?;
    num.parse().ok()
}

/// The name and value of a `- **name:** value` line.
fn field(line: &str) -> Option<(&str, &str)> {
    line.strip_prefix("- **")?.split_once(":** ")
}

fn finish(
    block: Option<Block>,
    found: &mut BTreeMap<u32, Obligation>,
    problems: &mut Vec<(usize, String)>,
) {
    let Some(Block { id, line, fields }) = block else {
        return;
    };
    if !fields.contains_key("must") {
        problems.push((line, format!("A{id} has no `must`")));
    }
    if let Some(kind) = fields.get("kind")
        && !KINDS.contains(kind)
    {
        let kinds = KINDS.join(", ");
        problems.push((line, format!("A{id} has kind `{kind}`, not one of {kinds}")));
    }
    let retired = match fields.get("status") {
        Some(&"retired") => true,
        Some(&"active") | None => false,
        Some(status) => {
            problems.push((
                line,
                format!("A{id} has status `{status}`, not active or retired"),
            ));
            false
        }
    };
    if found.insert(id, Obligation { retired }).is_some() {
        problems.push((line, format!("A{id} is declared twice")));
    }
}
