use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

pub fn impish_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-impish"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("run cargo-impish")
}

pub fn impish(args: &[&str]) -> Output {
    impish_in(Path::new("."), args)
}

/// A throwaway project to run `cargo impish check` in.
pub struct Project(TempDir);

pub struct Checked {
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl Checked {
    pub fn reports(&self, problem: &str) -> bool {
        self.stderr.lines().any(|l| l == problem)
    }
}

impl Project {
    pub fn new() -> Self {
        Self(TempDir::new().expect("temp dir"))
    }

    pub fn write(&self, rel: &str, text: &str) -> &Self {
        let path = self.0.path().join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
        self
    }

    pub fn check(&self) -> Checked {
        self.check_in(".")
    }

    pub fn check_in(&self, sub: &str) -> Checked {
        let dir = self.0.path().join(sub);
        fs::create_dir_all(&dir).unwrap();
        let out = impish_in(&dir, &["check"]);
        Checked {
            code: out.status.code(),
            stdout: String::from_utf8(out.stdout).unwrap(),
            stderr: String::from_utf8(out.stderr).unwrap(),
        }
    }
}

/// A `#[test]` named `name`, tagged with each of `tags` on its own line.
pub fn test_fn(name: &str, tags: &[&str]) -> String {
    let tags: String = tags.iter().map(|t| format!("// Contract: {t}\n")).collect();
    format!("{tags}{}\nfn {name}() {{}}\n\n", "#[test]")
}

/// A README declaring each of `ids` as an obligation.
pub fn readme(ids: &[&str]) -> String {
    let lines: String = ids
        .iter()
        .map(|id| {
            format!("### {id} — A promise\n- **must:** hold\n- **kind:** functional\n- **status:** active\n\n")
        })
        .collect();
    format!("# A primitive\n\n## Obligations\n\n{lines}")
}
