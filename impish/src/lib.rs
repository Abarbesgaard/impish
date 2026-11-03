//! `#[impish::contract("<primitive>/A<n>", …)]` marks a contract test: the
//! obligations of the Contract it proves. It takes the place of `#[test]`,
//! and checks each tag against the crate's `contract/` as the test compiles.

use std::path::PathBuf;

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{LitStr, Token};

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
    // Reading each README into the build makes cargo check the tags again when one changes.
    quote! {
        #(#errors)*
        #(const _: &[u8] = include_bytes!(#readmes);)*
        #[test]
        #item
    }
    .into()
}

/// The README that declares `tag`'s obligation, or why it doesn't.
fn check(root: &std::path::Path, tag: &str) -> Result<String, String> {
    let (primitive, id) = tag
        .rsplit_once("/A")
        .and_then(|(p, n)| Some((p, n.parse::<u32>().ok()?)))
        .ok_or_else(|| format!("`{tag}` isn't <primitive>/A<n>"))?;
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
        Some(_) => Ok(path.display().to_string()),
    }
}
