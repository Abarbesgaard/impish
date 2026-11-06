//! Reading the Contract: obligations in a README, `#[impish::contract]`
//! attributes in Rust files.

use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Attribute, ItemFn, LitStr, Token};

/// The lines of a README's covers blocks. A README without the block covers
/// nothing.
pub fn covers(readme: &str) -> Vec<&str> {
    block(readme, "covers").collect()
}

/// Whether a covers file could name something outside the crate.
pub fn escapes(file: &str) -> bool {
    use std::path::Component::{ParentDir, Prefix, RootDir};
    std::path::Path::new(file)
        .components()
        .any(|c| matches!(c, ParentDir | RootDir | Prefix(_)))
}

/// A stable hash of what a primitive's mutate result rests on: its README,
/// its mutants.md, its contract tests and `covered`, the files holding the
/// code it owns. `read` returns a file's text, or "" when there is none.
pub fn fingerprint(
    primitive: &str,
    covered: impl IntoIterator<Item = String>,
    read: impl Fn(&str) -> String,
) -> String {
    let mut files: std::collections::BTreeSet<String> = covered.into_iter().collect();
    files.extend([
        format!("contract/{primitive}/README.md"),
        format!("contract/{primitive}/mutants.md"),
        format!("tests/contract/{primitive}.rs"),
    ]);
    // FNV-1a, 64-bit: the same on every platform and Rust version.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for file in &files {
        let text = read(file);
        for byte in file.bytes().chain([0]).chain(text.bytes()).chain([0]) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    }
    format!("{hash:016x}")
}

/// The sections an intent.md lacks, each a problem naming `file`.
pub fn intent(file: &str, text: &str) -> Vec<String> {
    ["## Purpose", "## Public surface"]
        .into_iter()
        .filter(|heading| !text.lines().any(|l| l.trim_end() == *heading))
        .map(|heading| format!("{file}: no `{heading}` section"))
        .collect()
}

/// The `<kind> <name>` lines in a README's surface blocks, or the line that
/// isn't in that form. A README without the block has no surface.
pub fn surface(readme: &str) -> Vec<Result<&str, &str>> {
    block(readme, "surface")
        .map(|line| match line.split_once(' ') {
            Some((_, name)) if !name.contains(' ') => Ok(line),
            _ => Err(line),
        })
        .collect()
}

/// The non-blank, trimmed lines of every ```` ```kind ```` block in a README.
fn block<'a>(readme: &'a str, kind: &str) -> impl Iterator<Item = &'a str> {
    let open = format!("```{kind}\n");
    let blocks: Vec<&str> = readme
        .split(open.as_str())
        .skip(1)
        .map(|rest| rest.split_once("```").map_or("", |(block, _)| block))
        .collect();
    blocks
        .into_iter()
        .flat_map(str::lines)
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

/// The 1-based lines of the test attributes (`#[test]`, or any path ending in
/// `test`) on functions in a Rust file, or none if it isn't valid Rust.
pub fn test_attributes(src: &str) -> Vec<usize> {
    struct Lines(Vec<usize>);
    impl Visit<'_> for Lines {
        fn visit_item_fn(&mut self, f: &ItemFn) {
            for attr in &f.attrs {
                if attr
                    .path()
                    .segments
                    .last()
                    .is_some_and(|s| s.ident == "test")
                {
                    self.0.push(attr.span().start().line);
                }
            }
            visit::visit_item_fn(self, f);
        }
    }
    let mut lines = Lines(Vec::new());
    if let Ok(file) = syn::parse_file(src) {
        lines.visit_file(&file);
    }
    lines.0
}

/// A mutant `contract/<primitive>/mutants.md` declares, by its cargo-mutants
/// name without line and column.
pub struct Declared<'a> {
    /// 1-based line in mutants.md.
    pub line: usize,
    /// `equivalent` or `unpromised`.
    pub kind: &'a str,
    pub name: &'a str,
}

/// The mutants a mutants.md declares, and the line and kind of each
/// declaration block with no `##` heading above it to give its reason.
pub fn declarations(text: &str) -> (Vec<Declared<'_>>, Vec<(usize, &str)>) {
    let (mut declared, mut unreasoned) = (Vec::new(), Vec::new());
    let mut reasoned = false;
    // The open fence: `Some(kind)` in a declaration block, `Some("")` in any other.
    let mut fence: Option<&str> = None;
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        match fence {
            Some(_) if line.starts_with("```") => fence = None,
            Some("") => {}
            Some(kind) if !line.is_empty() => declared.push(Declared {
                line: i + 1,
                kind,
                name: line,
            }),
            Some(_) => {}
            None => match line.strip_prefix("```") {
                Some(kind @ ("equivalent" | "unpromised")) => {
                    if !reasoned {
                        unreasoned.push((i + 1, kind));
                    }
                    fence = Some(kind);
                }
                Some(_) => fence = Some(""),
                None if line.starts_with('#') => reasoned = line.starts_with("##"),
                None => {}
            },
        }
    }
    (declared, unreasoned)
}
