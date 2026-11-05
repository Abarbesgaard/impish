//! Contract tags and tests outside the primitives' test files.

use std::fs;
use std::path::{Path, PathBuf};

use crate::contract::{contract_attributes, test_attributes};

/// `file` relative to `root`, with `/` between its parts on every platform,
/// as the Contract names files.
fn relative(root: &Path, file: &Path) -> String {
    let parts: Vec<_> = file
        .strip_prefix(root)
        .unwrap_or(file)
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect();
    parts.join("/")
}

/// Tags in `src/` and `tests/` outside the primitives' test files.
pub fn tags(root: &Path, names: &[String]) -> Vec<String> {
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    rust_files(&root.join("tests"), &mut files);
    files.sort();
    let homes: Vec<PathBuf> = names
        .iter()
        .map(|name| root.join("tests/contract").join(format!("{name}.rs")))
        .collect();
    let mut problems = Vec::new();
    for file in files.iter().filter(|file| !homes.contains(file)) {
        let src = fs::read_to_string(file).unwrap_or_default();
        let rel = relative(root, file);
        for n in contract_attributes(&src) {
            problems.push(format!(
                "{rel}:{n}: an #[impish::contract] attribute outside tests/contract/<primitive>.rs"
            ));
        }
    }
    problems
}

/// Tests in `tests/contract/` outside the primitives' test files.
pub fn tests(root: &Path, names: &[String]) -> Vec<String> {
    let dir = root.join("tests/contract");
    let mut files = Vec::new();
    rust_files(&dir, &mut files);
    files.sort();
    let mut problems = Vec::new();
    for file in &files {
        let home = names
            .iter()
            .any(|name| *file == dir.join(format!("{name}.rs")));
        if home {
            continue;
        }
        let src = fs::read_to_string(file).unwrap_or_default();
        let rel = relative(root, file);
        for n in test_attributes(&src) {
            problems.push(format!(
                "{rel}:{n}: a test outside tests/contract/<primitive>.rs"
            ));
        }
    }
    problems
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}
