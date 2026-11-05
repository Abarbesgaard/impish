use crate::harness::{Project, calc, readme, test_fn};

const LIB: &str = "\
/// Adds the two.
#[inline]
pub fn add(a: i32, b: i32) -> i32 {
    fn helper() -> i32 { 7 }
    a + b + helper() - 7
}

pub struct Twice;

impl Twice {
    pub fn twice(&self, x: i32) -> i32 {
        x * 2
    }
}

fn secret() -> i32 {
    42
}
";

/// calc, covering `add` and `twice` in src/lib.rs.
fn project() -> Project {
    let p = calc(&["A1"], &test_fn("adds", &["calc/A1"]));
    p.write(
        "contract/calc/README.md",
        &(readme(&["A1"]) + "```covers\nsrc/lib.rs::add\nsrc/lib.rs::twice\n```\n"),
    )
    .write("src/lib.rs", LIB);
    p
}

fn pack(p: &Project) -> Vec<String> {
    let mut files: Vec<String> = p
        .files()
        .keys()
        .filter_map(|f| {
            f.strip_prefix(p.path().join("target/impish/regen/calc"))
                .ok()
        })
        .map(|f| f.to_string_lossy().replace('\\', "/"))
        .collect();
    files.sort();
    files
}

#[impish::contract("regen/A1")]
fn an_intent_needs_its_purpose_and_public_surface() {
    let p = project();
    p.write("contract/intent.md", "# calc\n\n## Purpose\n\nAdds.\n")
        .write("contract/calc/intent.md", "## Public surface\n\n`add`\n");
    let out = p.check();
    assert_eq!(out.code, Some(1), "{}", out.stderr);
    for problem in [
        "contract/intent.md: no `## Public surface` section",
        "contract/calc/intent.md: no `## Purpose` section",
    ] {
        assert!(out.reports(problem), "no `{problem}` in\n{}", out.stderr);
    }
    let whole = "## Purpose\n\nAdds.\n\n## Public surface\n\n`add`\n";
    p.write("contract/intent.md", whole)
        .write("contract/calc/intent.md", whole);
    let out = p.check();
    assert_eq!(out.code, Some(0), "{}", out.stderr);
}

#[impish::contract("regen/A2")]
fn regen_writes_a_pack() {
    let p = project();
    p.write("contract/intent.md", "system intent")
        .write("contract/calc/intent.md", "calc intent")
        .write("contract/decisions.toml", "# project records")
        .write("contract/calc/decisions.toml", "# calc records");
    let out = p.run(&["regen", "calc"]);
    assert_eq!(out.code, Some(0), "{}", out.stderr);
    assert_eq!(
        pack(&p),
        [
            ".impish-pack",
            "FORMAT.md",
            "PROMPT.md",
            "contract/calc/README.md",
            "contract/calc/decisions.toml",
            "contract/calc/intent.md",
            "contract/decisions.toml",
            "contract/intent.md",
            "stubs/src/lib.rs",
            "tests/contract/calc.rs",
        ]
    );
    let read = |f: &str| p.read(&format!("target/impish/regen/calc/{f}"));
    assert_eq!(read("contract/calc/intent.md"), "calc intent");
    assert_eq!(read("contract/decisions.toml"), "# project records");
    assert_eq!(
        read("tests/contract/calc.rs"),
        test_fn("adds", &["calc/A1"])
    );
    assert!(read("FORMAT.md").contains("## Obligations"));
    let prompt = read("PROMPT.md");
    assert!(
        prompt.contains("Rebuild the `calc` primitive from this pack alone"),
        "{prompt}"
    );
    assert_eq!(
        read("stubs/src/lib.rs"),
        "/// Adds the two.\n#[inline]\npub fn add(a: i32, b: i32) -> i32 {\n    todo!()\n}\n\n\
         impl Twice {\n    pub fn twice(&self, x: i32) -> i32 {\n        todo!()\n    }\n}\n"
    );

    let out = p.path().join("elsewhere");
    let moved = p.run(&["regen", "calc", "--out", out.to_str().unwrap()]);
    assert_eq!(moved.code, Some(0), "{}", moved.stderr);
    assert!(out.join("stubs/src/lib.rs").exists());
}

#[impish::contract("regen/A3")]
fn a_pack_holds_no_implementation() {
    let p = project();
    assert_eq!(p.run(&["regen", "calc"]).code, Some(0));
    let files = p.files();
    let packed: Vec<_> = files
        .iter()
        .filter(|(f, _)| f.starts_with(p.path().join("target/impish/regen/calc")))
        .collect();
    assert!(!packed.is_empty());
    for (file, bytes) in packed {
        let text = String::from_utf8_lossy(bytes);
        for leak in [
            "a + b",
            "helper",
            "x * 2",
            "secret",
            "42",
            "pub struct Twice",
        ] {
            assert!(
                !text.contains(leak),
                "{} holds `{leak}`:\n{text}",
                file.display()
            );
        }
    }
}

#[impish::contract("regen/A4")]
fn regen_only_deletes_what_it_wrote() {
    let p = project();
    p.write("contract/intent.md", "## Purpose\n## Public surface\n");
    assert_eq!(p.run(&["regen", "calc"]).code, Some(0));
    p.write("contract/intent.md", "")
        .write("contract/calc/README.md", &readme(&["A1"]));
    std::fs::remove_file(p.path().join("contract/intent.md")).unwrap();
    let again = p.run(&["regen", "calc"]);
    assert_eq!(again.code, Some(0), "{}", again.stderr);
    let files = pack(&p);
    assert!(
        !files.contains(&"contract/intent.md".to_string()),
        "{files:?}"
    );
    assert!(
        !files.contains(&"stubs/src/lib.rs".to_string()),
        "{files:?}"
    );
    let marker = p.read("target/impish/regen/calc/.impish-pack");
    assert!(marker.lines().any(|l| l == "PROMPT.md"), "{marker}");
    assert!(!marker.contains("stubs/"), "{marker}");

    p.write("target/impish/regen/calc/notes.txt", "mine");
    let stray = p.run(&["regen", "calc"]);
    assert_eq!(stray.code, Some(2), "{}", stray.stderr);
    assert!(stray.stderr.contains("notes.txt"), "{}", stray.stderr);
    assert!(p.path().join("target/impish/regen/calc/PROMPT.md").exists());

    p.write("mine/keep.rs", "fn keep() {}");
    let unmarked = p.run(&["regen", "calc", "--out", "mine"]);
    assert_eq!(unmarked.code, Some(2), "{}", unmarked.stderr);
    assert!(
        unmarked.stderr.contains(".impish-pack"),
        "{}",
        unmarked.stderr
    );
    assert_eq!(p.read("mine/keep.rs"), "fn keep() {}");
    assert!(!p.path().join("mine/PROMPT.md").exists());
}

#[impish::contract("regen/A5")]
fn an_unknown_primitive_is_undecided() {
    let p = project();
    // Each of these but "nope" would reach a README if taken as a path.
    p.write("README.md", "# the project\n");
    for name in ["nope", "../calc", "", "calc/.", "..", "../contract/calc"] {
        let out = p.run(&["regen", name]);
        assert_eq!(out.code, Some(2), "{name}: {}", out.stderr);
    }
    assert!(!p.path().join("target").exists());
}
