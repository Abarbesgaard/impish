use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
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
        self.run_in(sub, &["check"])
    }

    pub fn run(&self, args: &[&str]) -> Checked {
        self.run_in(".", args)
    }

    pub fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.0.path().join(rel)).unwrap()
    }

    fn run_in(&self, sub: &str, args: &[&str]) -> Checked {
        let dir = self.0.path().join(sub);
        fs::create_dir_all(&dir).unwrap();
        let out = impish_in(&dir, args);
        Checked {
            code: out.status.code(),
            stdout: String::from_utf8(out.stdout).unwrap(),
            stderr: String::from_utf8(out.stderr).unwrap(),
        }
    }
}

/// A contract test named `name` that proves `tags`, each of which may list
/// several obligations separated by commas; with no tags, a plain `#[test]`.
pub fn test_fn(name: &str, tags: &[&str]) -> String {
    let tags: Vec<String> = tags
        .iter()
        .flat_map(|t| t.split(", "))
        .map(|t| format!("{t:?}"))
        .collect();
    let attr = match tags.is_empty() {
        true => "#[test]".to_string(),
        false => format!("#[impish::contract({})]", tags.join(", ")),
    };
    format!("{attr}\nfn {name}() {{}}\n\n")
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

/// A throwaway crate whose tests use the `impish` attribute, built against
/// this workspace's `impish` and `Cargo.lock` into a shared target directory.
pub struct Crate(TempDir);

pub struct Tested {
    pub status: std::process::ExitStatus,
    stdout: String,
    stderr: String,
}

impl Tested {
    pub fn text(&self) -> String {
        format!("{}\n{}", self.stdout, self.stderr)
    }
}

impl Crate {
    pub fn with_tests(src: &str) -> Self {
        let dir = TempDir::new().expect("temp dir");
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        // Fixtures share a target directory, so each needs its own name.
        let mut hasher = DefaultHasher::new();
        src.hash(&mut hasher);
        let manifest = format!(
            "[package]\nname = \"fixture{:x}\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n\
             [dev-dependencies]\nimpish = {{ path = {:?} }}\n",
            hasher.finish(),
            root.join("impish")
        );
        fs::write(dir.path().join("Cargo.toml"), manifest).unwrap();
        fs::copy(root.join("Cargo.lock"), dir.path().join("Cargo.lock")).unwrap();
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/lib.rs"), "").unwrap();
        fs::create_dir_all(dir.path().join("tests")).unwrap();
        fs::write(dir.path().join("tests/fixture.rs"), src).unwrap();
        Self(dir)
    }

    pub fn write(&self, rel: &str, text: &str) -> &Self {
        let path = self.0.path().join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
        self
    }

    pub fn test(&self) -> Tested {
        let out = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
            .args(["test", "--test", "fixture"])
            .current_dir(self.0.path())
            .env(
                "CARGO_TARGET_DIR",
                concat!(env!("CARGO_TARGET_TMPDIR"), "/fixtures"),
            )
            .env_remove("RUSTFLAGS")
            .output()
            .expect("run cargo test");
        Tested {
            status: out.status,
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        }
    }
}

/// A throwaway crate with a `calc` primitive, for `cargo impish mutate`:
/// `src/lib.rs` defines `add` and `double`, which the README covers, and
/// `tests/contract/calc.rs` holds `tests`. Builds share a target directory.
pub fn calc_crate(tests: &str) -> Project {
    let p = Project::new();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut hasher = DefaultHasher::new();
    tests.hash(&mut hasher);
    let manifest = format!(
        "[package]\nname = \"calc{:x}\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n\
         [lib]\nname = \"calc\"\n\n[dev-dependencies]\nimpish = {{ path = {:?} }}\n",
        hasher.finish(),
        root.join("impish")
    );
    let readme = readme(&["A1", "A2"]) + "```covers\nsrc/lib.rs::add\nsrc/lib.rs::double\n```\n";
    p.write("Cargo.toml", &manifest)
        .write(
            "Cargo.lock",
            &fs::read_to_string(root.join("Cargo.lock")).unwrap(),
        )
        .write(
            "src/lib.rs",
            "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n\n\
             pub fn double(a: i32) -> i32 {\n    a * 2\n}\n",
        )
        .write("contract/calc/README.md", &readme)
        .write("tests/contract/main.rs", "mod calc;\n")
        .write("tests/contract/calc.rs", tests);
    p
}

impl Project {
    pub fn mutate(&self, primitive: &str) -> Checked {
        let out = Command::new(env!("CARGO_BIN_EXE_cargo-impish"))
            .args(["mutate", primitive])
            .current_dir(self.0.path())
            .env(
                "CARGO_TARGET_DIR",
                concat!(env!("CARGO_TARGET_TMPDIR"), "/fixtures"),
            )
            .env_remove("RUSTFLAGS")
            .output()
            .expect("run cargo-impish mutate");
        Checked {
            code: out.status.code(),
            stdout: String::from_utf8(out.stdout).unwrap(),
            stderr: String::from_utf8(out.stderr).unwrap(),
        }
    }
}
