use crate::harness::{Project, calc, test_fn};

fn project(scope: &str) -> Project {
    let p = calc(&["A1"], &test_fn("adds", &["calc/A1"]));
    p.write("contract/scope.toml", scope);
    p
}

fn ceiling(name: &str, counts: &str, glob: &str, ceiling: usize) -> String {
    format!(
        "[[ceiling]]\nname = \"{name}\"\ncounts = \"{counts}\"\nglob = \"{glob}\"\n\
         ceiling = {ceiling}\nwhy = \"small\"\n"
    )
}

#[impish::contract("scope/A1")]
fn malformed_ceilings_fail_naming_the_file_and_line() {
    let p = project(&format!(
        "{}{}{}",
        "[[ceiling]]\nname = 1\ncounts = \"words\"\nglob = \"src/[\"\nceiling = 0\n\
         colour = \"red\"\n",
        "[[ceiling]]\nname = \"n\"\ncounts = \"files\"\nglob = \"**/*.md\"\nceiling = 3\n\
         why = \"w\"\nrecord = \"D9\"\n",
        "[[ceiling]]\nname = \"m\"\ncounts = \"files\"\nglob = \"nothing/**\"\n\
         ceiling = \"many\"\nwhy = 2\n"
    ));
    let out = p.check();
    assert_eq!(out.code, Some(1), "{}", out.stderr);
    for problem in [
        "contract/scope.toml:1: unknown field `colour`",
        "contract/scope.toml:1: no `why`",
        "contract/scope.toml:1: `name` must be a string",
        "contract/scope.toml:1: `ceiling` must be a whole number above 0",
        "contract/scope.toml:1: counts `words`, not one of files, lines, functions",
        "contract/scope.toml:14: `why` must be a string",
        "contract/scope.toml:14: `ceiling` must be a whole number above 0",
        "contract/scope.toml:14: glob `nothing/**` matches no file",
        "contract/scope.toml:7: record `D9` names no decision record",
    ] {
        assert!(out.reports(problem), "{problem}\n{}", out.stderr);
    }
    assert!(
        out.stderr
            .lines()
            .any(|l| l.starts_with("contract/scope.toml:1: glob doesn't parse")),
        "{}",
        out.stderr
    );
    let p = project("ceiling = 1\nsize = 2\n");
    let out = p.check();
    assert!(
        out.reports("contract/scope.toml: unknown field `size`"),
        "{}",
        out.stderr
    );
    assert!(
        out.reports("contract/scope.toml: `ceiling` must be a list of [[ceiling]] tables"),
        "{}",
        out.stderr
    );
    let out = project("ceiling = [1]\n").check();
    assert!(out.reports("contract/scope.toml:1: a ceiling must be a table"));
    let out = project("[[ceiling]\n").check();
    assert!(
        out.stderr
            .starts_with("contract/scope.toml:1: isn't valid TOML"),
        "{}",
        out.stderr
    );
}

#[impish::contract("scope/A2")]
fn a_count_over_its_ceiling_fails_by_name() {
    let p = project(&format!(
        "{}{}{}{}{}",
        ceiling("docs", "files", "contract/**/*.md", 1),
        ceiling("code", "lines", "src/*.rs", 4),
        ceiling("fns", "functions", "src/**", 3),
        ceiling("held", "files", "src/*.rs", 3),
        ceiling("hidden", "files", "**/x.rs", 1),
    ));
    let code = "pub fn a() {\n\n    fn inner() {}\n}\n\nstruct S;\n\
                impl S {\n    fn m(&self) {}\n}\ntrait T {\n    fn t();\n}\n";
    p.write("contract/README.md", "# The Contract\n")
        .write("src/lib.rs", code)
        .write("src/notes.txt", "fn not_rust() {}\n")
        .write("src/broken.rs", "fn {\n")
        .write("target/x.rs", "fn hidden() {}\n")
        .write(".git/x.rs", "fn hidden() {}\n")
        .write("src/x.rs", "");
    let out = p.check();
    assert_eq!(out.code, Some(1), "{}", out.stderr);
    for problem in [
        "contract/scope.toml:1: `docs` counts 2 files, over its ceiling of 1",
        "contract/scope.toml:7: `code` counts 11 lines, over its ceiling of 4",
        "contract/scope.toml:13: `fns` counts 4 functions, over its ceiling of 3",
    ] {
        assert!(out.reports(problem), "{problem}\n{}", out.stderr);
    }
    for held in ["`held`", "`hidden`"] {
        assert!(!out.stderr.contains(held), "{}", out.stderr);
    }
}

#[impish::contract("scope/A3")]
fn every_count_is_reported() {
    let p = project(&format!(
        "{}{}",
        ceiling("docs", "files", "contract/**/*.md", 1),
        ceiling("tests", "lines", "tests/contract/*.rs", 99),
    ));
    let out = p.check();
    assert_eq!(out.code, Some(0), "{}", out.stderr);
    assert_eq!(
        out.stdout,
        "The Contract holds: 1 obligations across 1 primitives.\n\
         docs: 1 of at most 1\ntests: 2 of at most 99\n"
    );
    assert_eq!(p.run(&["check", "--write"]).code, Some(0));
    let json: serde_json::Value = serde_json::from_str(&p.read("contract/coverage.json")).unwrap();
    assert_eq!(
        json["scope"],
        serde_json::json!({
            "docs": {"counts": "files", "ceiling": 1},
            "tests": {"counts": "lines", "ceiling": 99},
        })
    );
    let p = calc(&["A1"], &test_fn("adds", &["calc/A1"]));
    let out = p.check();
    assert_eq!(
        out.stdout,
        "The Contract holds: 1 obligations across 1 primitives.\n"
    );
    assert_eq!(p.run(&["check", "--write"]).code, Some(0));
    assert!(!p.read("contract/coverage.json").contains("scope"));
}
