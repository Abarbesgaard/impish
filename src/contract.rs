//! Reading contract test files: the tests in them, and where the
//! `#[impish::contract]` attributes are.

use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Attribute, Expr, ExprLit, ItemFn, Lit, Token};

pub struct TaggedTest {
    pub name: String,
    pub tags: Vec<String>,
    pub ignored: bool,
}

/// The string in `expr`, when it is a string literal.
fn string(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Str(s), ..
        }) => Some(s.value()),
        _ => None,
    }
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
                let mut test = TaggedTest {
                    name: f.sig.ident.to_string(),
                    tags: Vec::new(),
                    ignored: f.attrs.iter().any(|a| is(a, &["ignore"])),
                };
                let args = f.attrs.iter().filter(|a| is_contract(a)).flat_map(|a| {
                    a.parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated)
                        .unwrap_or_default()
                });
                test.tags.extend(args.filter_map(|a| string(&a)));
                self.0.push(test);
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
