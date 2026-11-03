//! `#[impish::contract("<primitive>/A<n>", …)]` marks a contract test: the
//! obligations of the Contract it proves. It takes the place of `#[test]`.

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{LitStr, Token};

#[proc_macro_attribute]
pub fn contract(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = proc_macro2::TokenStream::from(item);
    let names = Punctuated::<LitStr, Token![,]>::parse_terminated.parse(attr);
    match names {
        Ok(names) if !names.is_empty() => quote! { #[test] #item },
        Ok(_) => syn::Error::new(
            Span::call_site(),
            "name the obligations this test proves: #[impish::contract(\"<primitive>/A<n>\")]",
        )
        .into_compile_error(),
        Err(e) => e.into_compile_error(),
    }
    .into()
}
