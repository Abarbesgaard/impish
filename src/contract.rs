//! Reading the Contract: obligations in a README, `#[impish::contract]`
//! attributes in Rust files.

use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Attribute, ItemFn, LitStr, Token};

/// The `(file, function)` pairs in a README's covers blocks, or the line that
/// isn't `<file>::<function>`. A README without the block covers nothing.
pub fn covers(readme: &str) -> Vec<Result<(&str, &str), &str>> {
    block(readme, "covers")
        .map(|line| line.split_once("::").ok_or(line))
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
