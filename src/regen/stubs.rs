//! Stubs of covered functions: each one's doc comments, attributes and
//! signature, with a `todo!()` body, so a pack shows what to rebuild and
//! none of how it was built.

use proc_macro2::LineColumn;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{ImplItemFn, ItemFn, ItemImpl};

/// The stubs of the functions in `src` named in `names`, or `None` when
/// `src` doesn't parse or defines none of them.
pub fn stubs(src: &str, names: &[&str]) -> Option<String> {
    let file = syn::parse_file(src).ok()?;
    let mut lines = vec![0];
    lines.extend(src.match_indices('\n').map(|(i, _)| i + 1));
    let mut found = Found {
        src,
        names,
        lines,
        header: None,
        out: Vec::new(),
    };
    found.visit_file(&file);
    (!found.out.is_empty()).then(|| found.out.join("\n"))
}

struct Found<'a> {
    src: &'a str,
    names: &'a [&'a str],
    /// The byte offset each line starts at.
    lines: Vec<usize>,
    /// The `impl … ` the functions being visited are in.
    header: Option<String>,
    out: Vec<String>,
}

impl Found<'_> {
    fn offset(&self, at: LineColumn) -> usize {
        let start = self.lines[at.line - 1];
        self.src[start..]
            .char_indices()
            .nth(at.column)
            .map_or(self.src.len(), |(i, _)| start + i)
    }

    /// The text from `item`'s first attribute up to the `{` at `open`.
    fn head(&self, item: &impl Spanned, open: LineColumn) -> String {
        let from = self.offset(item.span().start());
        self.src[from..self.offset(open)].trim_end().to_string()
    }
}

impl<'ast> Visit<'ast> for Found<'_> {
    fn visit_item_fn(&mut self, f: &'ast ItemFn) {
        // Never into the body: a nested function is implementation.
        if self.names.contains(&f.sig.ident.to_string().as_str()) {
            let sig = self.head(f, f.block.brace_token.span.open().start());
            self.out.push(format!("{sig} {{\n    todo!()\n}}\n"));
        }
    }

    fn visit_item_impl(&mut self, i: &'ast ItemImpl) {
        let header = self.head(i, i.brace_token.span.open().start());
        let outer = self.header.replace(header);
        visit::visit_item_impl(self, i);
        self.header = outer;
    }

    fn visit_impl_item_fn(&mut self, f: &'ast ImplItemFn) {
        if self.names.contains(&f.sig.ident.to_string().as_str())
            && let Some(header) = &self.header
        {
            let sig = self.head(f, f.block.brace_token.span.open().start());
            self.out.push(format!(
                "{header} {{\n    {sig} {{\n        todo!()\n    }}\n}}\n"
            ));
        }
    }
}
