//! Contract tests for Rust: a test that names the promise it proves, checked
//! as it compiles.
//!
//! A *Contract* is the list of promises a piece of code makes, written down in
//! `contract/<primitive>/README.md` as numbered *obligations*. A contract test
//! names the obligations it proves with [`#[impish::contract]`](macro@contract):
//!
//! ```rust,ignore
//! #[impish::contract("slugify/A1")]
//! fn lowercases_letters_and_digits() {
//!     assert_eq!(slug::slugify("Rust 2024"), "rust-2024");
//! }
//! ```
//!
//! The attribute reads `contract/slugify/README.md` while the test compiles. If
//! `A1` isn't declared there, or is retired, the test doesn't compile, so a
//! typo or a broken promise can't slip into a green build.
//!
//! This crate is the attribute only. The [`cargo-impish`] command checks that
//! every obligation has a test (`cargo impish check`) and that the tests catch
//! real bugs (`cargo impish mutate`). The [README] explains the method.
//!
//! # Getting started
//!
//! Add the crate as a dev-dependency, and the command once per machine:
//!
//! ```sh
//! cargo add --dev impish
//! cargo install cargo-impish
//! ```
//!
//! Write the Contract for a primitive, here `slugify`, in
//! `contract/slugify/README.md`:
//!
//! ````markdown
//! # slugify
//!
//! Turns a title into a URL slug.
//!
//! ## Obligations
//!
//! ### A1 — Letters and digits are kept, lowercased
//! - **must:** keep ASCII letters and digits, lowercased
//! - **kind:** functional
//! - **status:** active
//!
//! ```covers
//! src/lib.rs::slugify
//! ```
//! ````
//!
//! Prove it in `tests/contract/slugify.rs`, and list the primitive in
//! `tests/contract/main.rs` (`mod slugify;`), which Cargo builds as the
//! `contract` test target:
//!
//! ```rust,ignore
//! #[impish::contract("slugify/A1")]
//! fn lowercases_letters_and_digits() {
//!     assert_eq!(slug::slugify("Rust 2024"), "rust-2024");
//! }
//! ```
//!
//! Then `cargo test` runs it, and `cargo impish check` confirms every
//! obligation is proved:
//!
//! ```text
//! The Contract holds: 1 obligations across 1 primitives.
//! ```
//!
//! # Existing tests
//!
//! The attribute doesn't replace a test attribute that is already there. Add
//! it on the line below (or above) `#[test]`, `#[tokio::test]` or any
//! attribute whose path ends in `test`, and the test still runs once:
//!
//! ```rust,ignore
//! #[tokio::test]
//! #[impish::contract("fetch/A2")]
//! async fn retries_after_a_timeout() {
//!     // ...
//! }
//! ```
//!
//! On its own, the attribute makes the function a `#[test]`.
//!
//! # What is checked as the test compiles
//!
//! For each tag, the attribute reports a compile error on that tag when:
//!
//! | Error | Meaning and fix |
//! | --- | --- |
//! | ``name the obligations this test proves: #[impish::contract("<primitive>/A<n>")]`` | The attribute has no tags. Name at least one. Tags are string literals. |
//! | `` `x` isn't <primitive>/A<n> `` | A tag is not a primitive and an obligation number, such as `"slugify/A1"`. |
//! | `` `x`: the primitive must be letters, digits, `-` and `_` `` | A tag tried to name a path outside `contract/`. Nothing is read. |
//! | `` `x`: contract/p/README.md can't be read `` | The primitive has no Contract yet. Create its README, or fix the name. |
//! | ``…isn't a valid Contract; `cargo impish check` names its problems`` | The README doesn't parse. Run `cargo impish check` for the line. |
//! | `` `x`: contract/p/README.md doesn't declare A9 `` | No such obligation. Add it to the Contract first, or fix the number. |
//! | `` `x`: contract/p/README.md declares A2 retired `` | The promise was withdrawn. Delete the test, or name the obligation that replaced it. |
//!
//! The README is part of the test's build, so editing it checks the tags again
//! on the next `cargo test`.
//!
//! # Where it belongs
//!
//! Use the attribute only in `tests/contract/<primitive>.rs`, the tests of
//! that primitive's Contract; `cargo impish check` reports it anywhere else.
//! Tags are resolved against `contract/` in the crate being tested
//! (`CARGO_MANIFEST_DIR`). Several tags on one test are fine:
//! `#[impish::contract("cli/A1", "cli/A2")]`.
//!
//! The Contract format, every `check` and `mutate` message, and how to adopt
//! impish in an existing crate are in the [README].
//!
//! [`cargo-impish`]: https://crates.io/crates/cargo-impish
//! [README]: https://github.com/Abarbesgaard/impish#readme

use std::path::PathBuf;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{Attribute, LitStr, Token};

/// Marks a contract test with the obligations it proves.
///
/// ```text
/// #[impish::contract("<primitive>/A<n>", …)]
/// ```
///
/// Each argument is a string literal naming one obligation: the primitive (a
/// directory under `contract/`) and the obligation's number in its README.
/// Every tag is checked against `contract/<primitive>/README.md` as the test
/// compiles. A tag that is malformed, unknown or retired is a compile error on
/// that tag; the [crate documentation](crate#what-is-checked-as-the-test-compiles)
/// lists each error and its fix.
///
/// The function becomes a `#[test]`, unless it already has a test attribute
/// (`#[test]`, `#[tokio::test]` or any path ending in `test`), above or below
/// this one, which it keeps:
///
/// ```rust,ignore
/// #[impish::contract("parse/A1", "parse/A3")]
/// fn rejects_an_empty_line() {
///     assert!(parse("").is_err());
/// }
///
/// #[tokio::test]
/// #[impish::contract("fetch/A2")]
/// async fn retries_after_a_timeout() {
///     // ...
/// }
/// ```
///
/// `#[ignore]` works as usual, but an ignored test proves nothing:
/// `cargo impish check` reports its obligations as unproved.
#[proc_macro_attribute]
pub fn contract(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = proc_macro2::TokenStream::from(item);
    let names = match Punctuated::<LitStr, Token![,]>::parse_terminated.parse(attr) {
        Ok(names) if !names.is_empty() => names,
        Ok(_) => {
            return syn::Error::new(
                Span::call_site(),
                "name the obligations this test proves: #[impish::contract(\"<primitive>/A<n>\")]",
            )
            .into_compile_error()
            .into();
        }
        Err(e) => return e.into_compile_error().into(),
    };
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let mut readmes = Vec::new();
    let mut errors = Vec::new();
    for name in &names {
        match check(&root, &name.value()) {
            Ok(readme) => readmes.push(readme),
            Err(problem) => errors.push(syn::Error::new(name.span(), problem).into_compile_error()),
        }
    }
    let test = (!is_test(&item)).then(|| quote!(#[test]));
    // Reading each README into the build makes cargo check the tags again when one changes.
    quote! {
        #(#errors)*
        #(const _: &[u8] = include_bytes!(#readmes);)*
        #test
        #item
    }
    .into()
}

/// Whether `item` already carries a test attribute: `#[test]`, or one whose
/// path ends in `test`. When one sits above this attribute, rustc expands it
/// first and passes the bare function on, so the source file is asked too.
fn is_test(item: &proc_macro2::TokenStream) -> bool {
    let Ok(item) = syn::parse2::<syn::ItemFn>(item.clone()) else {
        return false;
    };
    has_test(&item.attrs) || tested_in_source(&item)
}

fn has_test(attrs: &[Attribute]) -> bool {
    attrs
        .iter()
        .any(|a| a.path().segments.last().is_some_and(|s| s.ident == "test"))
}

/// Whether the file this attribute is written in has `item`, the same
/// signature and body, with a test attribute.
fn tested_in_source(item: &syn::ItemFn) -> bool {
    let Some(file) = proc_macro::Span::call_site().local_file() else {
        return false;
    };
    let Ok(text) = std::fs::read_to_string(file) else {
        return false;
    };
    let Ok(file) = syn::parse_file(&text) else {
        return false;
    };
    let same = |f: &syn::ItemFn| {
        let tokens = |f: &syn::ItemFn| {
            let (sig, block) = (&f.sig, &f.block);
            quote!(#sig #block).to_string()
        };
        tokens(f) == tokens(item)
    };
    fn any(items: &[syn::Item], same: &dyn Fn(&syn::ItemFn) -> bool) -> bool {
        items.iter().any(|item| match item {
            syn::Item::Fn(f) => same(f) && has_test(&f.attrs),
            syn::Item::Mod(m) => m
                .content
                .as_ref()
                .is_some_and(|(_, items)| any(items, same)),
            _ => false,
        })
    }
    any(&file.items, &same)
}

/// The README that declares `tag`'s obligation, or why it doesn't.
fn check(root: &std::path::Path, tag: &str) -> Result<String, String> {
    let (primitive, id) = tag
        .rsplit_once("/A")
        .and_then(|(p, n)| Some((p, n.parse::<u32>().ok()?)))
        .ok_or_else(|| format!("`{tag}` isn't <primitive>/A<n>"))?;
    let plain = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
    if primitive.is_empty() || !primitive.chars().all(plain) {
        return Err(format!(
            "`{tag}`: the primitive must be letters, digits, `-` and `_`"
        ));
    }
    let file = format!("contract/{primitive}/README.md");
    let path = root.join(&file);
    let text =
        std::fs::read_to_string(&path).map_err(|_| format!("`{tag}`: {file} can't be read"))?;
    let declared = impish_core::obligations(&text, &file).map_err(|_| {
        format!("`{tag}`: {file} isn't a valid Contract; `cargo impish check` names its problems")
    })?;
    match declared.get(&id) {
        None => Err(format!("`{tag}`: {file} doesn't declare A{id}")),
        Some(o) if o.retired => Err(format!("`{tag}`: {file} declares A{id} retired")),
        Some(o) if !o.needs_test() => Err(format!(
            "`{tag}`: {file} declares A{id} an exception, which no test proves"
        )),
        Some(_) => Ok(path.display().to_string()),
    }
}
