//! Reading the Contract: obligations in a README, `#[impish::contract]`
//! attributes in Rust files.

use std::collections::BTreeMap;

use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Attribute, ItemFn, LitStr, Token};

const KINDS: [&str; 5] = [
    "functional",
    "non-functional",
    "persisted-format",
    "ordering",
    "security",
];
const FIELDS: [&str; 4] = ["must", "why", "kind", "status"];

pub struct Obligation {
    pub kind: String,
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
    for name in ["must", "kind", "status"] {
        if !fields.contains_key(name) {
            problems.push((line, format!("A{id} has no `{name}`")));
        }
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
    let kind = fields.get("kind").unwrap_or(&"").to_string();
    if found.insert(id, Obligation { kind, retired }).is_some() {
        problems.push((line, format!("A{id} is declared twice")));
    }
}

/// The `(file, function)` pairs in a README's covers block, or the line that
/// isn't `<file>::<function>`. A README without the block covers nothing.
pub fn covers(readme: &str) -> Vec<Result<(&str, &str), &str>> {
    block(readme, "covers")
        .map(|line| line.split_once("::").ok_or(line))
        .collect()
}

/// The `<kind> <name>` lines in a README's surface block, or the line that
/// isn't in that form. A README without the block has no surface.
pub fn surface(readme: &str) -> Vec<Result<&str, &str>> {
    block(readme, "surface")
        .map(|line| match line.split_once(' ') {
            Some((_, name)) if !name.contains(' ') => Ok(line),
            _ => Err(line),
        })
        .collect()
}

/// The non-blank, trimmed lines of a README's first ```` ```kind ```` block.
fn block<'a>(readme: &'a str, kind: &str) -> impl Iterator<Item = &'a str> {
    readme
        .split_once(&format!("```{kind}\n"))
        .and_then(|(_, rest)| rest.split_once("```"))
        .map_or("", |(block, _)| block)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
}

pub struct TaggedTest {
    pub name: String,
    pub tags: Vec<String>,
    pub ignored: bool,
}

fn is(attr: &Attribute, path: &[&str]) -> bool {
    attr.path()
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .eq(path.iter().copied())
}

fn is_contract(attr: &Attribute) -> bool {
    is(attr, &["impish", "contract"])
}

/// Every test in a contract test file: each function marked `#[test]` or
/// `#[impish::contract(…)]`, with the obligations the attribute names.
pub fn tagged_tests(src: &str) -> syn::Result<Vec<TaggedTest>> {
    struct Tests(Vec<TaggedTest>);
    impl Visit<'_> for Tests {
        fn visit_item_fn(&mut self, f: &ItemFn) {
            let marked = f.attrs.iter().any(|a| is_contract(a) || is(a, &["test"]));
            if marked {
                let tags = f.attrs.iter().filter(|a| is_contract(a)).flat_map(|a| {
                    a.parse_args_with(Punctuated::<LitStr, Token![,]>::parse_terminated)
                        .map(|names| names.iter().map(LitStr::value).collect::<Vec<_>>())
                        .unwrap_or_default()
                });
                self.0.push(TaggedTest {
                    name: f.sig.ident.to_string(),
                    tags: tags.collect(),
                    ignored: f.attrs.iter().any(|a| is(a, &["ignore"])),
                });
            }
            visit::visit_item_fn(self, f);
        }
    }
    let mut tests = Tests(Vec::new());
    tests.visit_file(&syn::parse_file(src)?);
    Ok(tests.0)
}

/// The 1-based lines of the `#[impish::contract(…)]` attributes in a Rust
/// file, or none if it isn't valid Rust.
pub fn contract_attributes(src: &str) -> Vec<usize> {
    struct Lines(Vec<usize>);
    impl Visit<'_> for Lines {
        fn visit_attribute(&mut self, attr: &Attribute) {
            if is_contract(attr) {
                self.0.push(attr.span().start().line);
            }
        }
    }
    let mut lines = Lines(Vec::new());
    if let Ok(file) = syn::parse_file(src) {
        lines.visit_file(&file);
    }
    lines.0
}
