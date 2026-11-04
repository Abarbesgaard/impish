//! impish's contract tests are made by `#[impish::contract]` itself, so a
//! broken attribute could delete its own proofs and leave `cargo test` green.
//! This plain test, outside the contract target, checks that every marked
//! function is still a test.

use std::process::Command;

use syn::visit::{self, Visit};

#[test]
fn the_attribute_made_every_contract_test() {
    struct Marked(usize);
    impl Visit<'_> for Marked {
        fn visit_item_fn(&mut self, f: &syn::ItemFn) {
            let contract = |a: &syn::Attribute| {
                let path: Vec<_> = a
                    .path()
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect();
                path == ["impish", "contract"]
            };
            self.0 += usize::from(f.attrs.iter().any(contract));
            visit::visit_item_fn(self, f);
        }
    }
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/contract");
    let mut marked = Marked(0);
    for entry in std::fs::read_dir(dir).unwrap() {
        let src = std::fs::read_to_string(entry.unwrap().path()).unwrap();
        marked.visit_file(&syn::parse_file(&src).unwrap());
    }

    let out = Command::new(env!("CARGO"))
        .args([
            "test", "--test", "contract", "--", "--list", "--format", "terse",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let listed = stdout.lines().filter(|l| l.ends_with(": test")).count();

    assert!(marked.0 > 0);
    assert_eq!(listed, marked.0, "contract tests listed:\n{stdout}");
}
