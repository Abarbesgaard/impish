//! The crate's code: every function, by file and lines, and which primitive's
//! covers line claims it. A function claimed by several primitives goes to the
//! most specific reference, a method before a function or type, before a file,
//! before a tree, and on a tie to the primitive whose name sorts first.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use syn::spanned::Spanned;
use syn::{Attribute, ImplItem, Item, TraitItem, Type};

use crate::contract::{covers, escapes};

/// A function, method or trait method with a body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Function {
    pub file: String,
    /// `name`, or `Type::name` for a method.
    pub name: String,
    /// The type or trait whose impl it is in.
    pub ty: Option<String>,
    /// Its first and last line, attributes included; a nested function is
    /// part of the one it is in.
    pub first: usize,
    pub last: usize,
}

/// What every primitive's covers lines resolve to.
#[derive(Default)]
pub struct Resolved {
    /// Per primitive: the functions it owns.
    pub owned: BTreeMap<String, Vec<Function>>,
    /// Per primitive: its covers lines that are malformed, escape the crate
    /// or name nothing.
    pub problems: BTreeMap<String, Vec<String>>,
    /// The crate's functions, in a `src/` directory, that no line claims.
    pub outside: Vec<Function>,
    /// How many functions the crate has in its `src/` directories.
    pub total: usize,
}

impl Resolved {
    /// How many of the crate's functions some primitive owns, of how many;
    /// `None` when it has none.
    pub fn share(&self) -> Option<(usize, usize)> {
        (self.total > 0).then(|| (self.total - self.outside.len(), self.total))
    }

    /// The files holding the code `primitive` owns.
    pub fn files_of(&self, primitive: &str) -> Vec<String> {
        let mut files: Vec<String> = self
            .owned
            .get(primitive)
            .into_iter()
            .flatten()
            .map(|f| f.file.clone())
            .collect();
        files.dedup();
        files
    }
}

/// Each primitive under `contract/` that has a README, with its text, in
/// order of name.
pub fn readmes(root: &Path) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = fs::read_dir(root.join("contract"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let readme = fs::read_to_string(entry.path().join("README.md")).ok()?;
            Some((entry.file_name().to_string_lossy().into_owned(), readme))
        })
        .collect();
    out.sort();
    out
}

/// One covers line: `<dir>/...`, `<file>`, `<file>::<item>` or
/// `<file>::<Type>::<method>`.
enum Reference<'a> {
    Tree(&'a str),
    File(&'a str),
    Item(&'a str, &'a str),
    Method(&'a str, &'a str, &'a str),
}

impl<'a> Reference<'a> {
    fn parse(line: &'a str) -> Option<Self> {
        if let Some(dir) = line.strip_suffix("/...") {
            return (!dir.is_empty()).then_some(Reference::Tree(dir));
        }
        let parts: Vec<&str> = line.split("::").collect();
        if parts.iter().any(|p| p.is_empty()) {
            return None;
        }
        match parts[..] {
            [file] => Some(Reference::File(file)),
            [file, item] => Some(Reference::Item(file, item)),
            [file, ty, method] => Some(Reference::Method(file, ty, method)),
            _ => None,
        }
    }

    fn path(&self) -> &'a str {
        match self {
            Reference::Tree(p) | Reference::File(p) => p,
            Reference::Item(p, _) | Reference::Method(p, _, _) => p,
        }
    }

    /// Higher is more specific.
    fn rank(&self) -> u8 {
        match self {
            Reference::Tree(_) => 1,
            Reference::File(_) => 2,
            Reference::Item(..) => 3,
            Reference::Method(..) => 4,
        }
    }

    fn claims(&self, f: &Function) -> bool {
        let method = f.name.rsplit("::").next().unwrap_or_default();
        match *self {
            Reference::Tree(dir) => f.file.starts_with(&format!("{dir}/")),
            Reference::File(file) => f.file == file,
            Reference::Item(file, item) => {
                f.file == file && (method == item || f.ty.as_deref() == Some(item))
            }
            Reference::Method(file, ty, m) => {
                f.file == file && f.ty.as_deref() == Some(ty) && method == m
            }
        }
    }
}

/// Resolves the covers lines of each `(primitive, README)`.
pub fn resolve(root: &Path, readmes: &[(String, String)]) -> Resolved {
    let files = files(root);
    let mut parsed: BTreeMap<String, Vec<Function>> = BTreeMap::new();
    let mut parse = |file: &str| -> Vec<Function> {
        parsed
            .entry(file.to_string())
            .or_insert_with(|| {
                functions(
                    file,
                    &fs::read_to_string(root.join(file)).unwrap_or_default(),
                )
            })
            .clone()
    };
    let mut resolved = Resolved::default();
    let crate_fns: Vec<Function> = files
        .iter()
        .filter(|f| in_src(f))
        .flat_map(|f| parse(f))
        .collect();
    resolved.total = crate_fns.len();
    // (file, first line) -> (rank, primitive, function)
    let mut claims: BTreeMap<(String, usize), (u8, String, Function)> = BTreeMap::new();
    for (name, readme) in readmes {
        let readme_file = format!("contract/{name}/README.md");
        let problems = resolved.problems.entry(name.clone()).or_default();
        for line in covers(readme) {
            let Some(reference) = Reference::parse(line) else {
                problems.push(format!(
                    "{readme_file}: covers line `{line}` is not <file>, <dir>/..., \
                     <file>::<item> or <file>::<Type>::<method>"
                ));
                continue;
            };
            if escapes(reference.path()) {
                problems.push(format!(
                    "{readme_file}: covers `{line}`, which is outside the crate"
                ));
                continue;
            }
            let candidates: Vec<Function> = match reference {
                Reference::Tree(_) => files.iter().flat_map(|f| parse(f)).collect(),
                _ => parse(reference.path()),
            };
            let claimed: Vec<Function> = candidates
                .into_iter()
                .filter(|f| reference.claims(f))
                .collect();
            if claimed.is_empty() {
                problems.push(format!(
                    "{readme_file}: covers `{line}`, which doesn't exist"
                ));
            }
            for f in claimed {
                let key = (f.file.clone(), f.first);
                let rank = reference.rank();
                // `readmes` is sorted, so on a tie the first claim stands.
                if claims.get(&key).is_none_or(|(r, _, _)| rank > *r) {
                    claims.insert(key, (rank, name.clone(), f));
                }
            }
        }
    }
    for (_, name, f) in claims.values() {
        resolved
            .owned
            .entry(name.clone())
            .or_default()
            .push(f.clone());
    }
    resolved.outside = crate_fns
        .into_iter()
        .filter(|f| !claims.contains_key(&(f.file.clone(), f.first)))
        .collect();
    resolved
}

/// Whether `file` is Rust in a `src/` directory: the crate's own code.
fn in_src(file: &str) -> bool {
    file.ends_with(".rs") && file.split('/').rev().skip(1).any(|part| part == "src")
}

/// The functions in `src`, `file` being its path; none when it doesn't parse
/// or is generated. `#[cfg(test)]` code is left out.
pub fn functions(file: &str, src: &str) -> Vec<Function> {
    if !file.ends_with(".rs") || src.lines().take(5).any(|l| l.contains("@generated")) {
        return Vec::new();
    }
    let Ok(parsed) = syn::parse_file(src) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    items(file, &parsed.items, &mut out);
    out
}

fn items(file: &str, list: &[Item], out: &mut Vec<Function>) {
    let mut push = |name: String, ty: Option<String>, item: &dyn Spanned| {
        let span = item.span();
        out.push(Function {
            file: file.to_string(),
            name,
            ty,
            first: span.start().line,
            last: span.end().line,
        });
    };
    let mut nested = Vec::new();
    for item in list {
        match item {
            Item::Fn(f) if !test_only(&f.attrs) => push(f.sig.ident.to_string(), None, f),
            Item::Impl(i) if !test_only(&i.attrs) => {
                let Type::Path(path) = &*i.self_ty else {
                    continue;
                };
                let Some(ty) = path.path.segments.last().map(|s| s.ident.to_string()) else {
                    continue;
                };
                for item in &i.items {
                    if let ImplItem::Fn(f) = item
                        && !test_only(&f.attrs)
                    {
                        push(format!("{ty}::{}", f.sig.ident), Some(ty.clone()), f);
                    }
                }
            }
            Item::Trait(t) if !test_only(&t.attrs) => {
                let ty = t.ident.to_string();
                for item in &t.items {
                    if let TraitItem::Fn(f) = item
                        && f.default.is_some()
                    {
                        push(format!("{ty}::{}", f.sig.ident), Some(ty.clone()), f);
                    }
                }
            }
            Item::Mod(m) if !test_only(&m.attrs) => {
                if let Some((_, inner)) = &m.content {
                    nested.push(inner);
                }
            }
            _ => {}
        }
    }
    for inner in nested {
        items(file, inner, out);
    }
}

/// Whether the attributes hold `#[cfg(test)]`.
fn test_only(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|a| {
        a.path().is_ident("cfg")
            && a.meta
                .require_list()
                .is_ok_and(|l| l.tokens.to_string() == "test")
    })
}

/// Every file under `root`, relative and with `/` between its parts, leaving
/// out `target/` and hidden directories such as `.git/`.
pub fn files(root: &Path) -> Vec<String> {
    fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>) {
        for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let rel = format!("{prefix}{name}");
            match entry.file_type() {
                Ok(t) if t.is_dir() => {
                    if !name.starts_with('.') && rel != "target" {
                        walk(&entry.path(), &format!("{rel}/"), out);
                    }
                }
                Ok(_) => out.push(rel),
                Err(_) => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(root, "", &mut out);
    out.sort();
    out
}
