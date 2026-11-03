use crate::harness::{Project, readme, test_fn};

const NOT_MET: &str = "The Contract is not met.";

fn calc(ids: &[&str], tests: &str) -> Project {
    let p = Project::new();
    p.write("contract/calc/README.md", &readme(ids))
        .write("tests/contract/calc.rs", tests);
    p
}

// Contract: check/A1
#[test]
fn a_held_contract_passes_and_says_what_it_checked() {
    let tests = test_fn("adds", &["calc/A1, calc/A2"]) + &test_fn("subtracts", &["calc/A3"]);
    let p = calc(&["A1", "A2", "A3"], &tests);
    let sum = [
        "// Contract: sum/A1",
        "/// Sums.",
        "#[cfg(unix)]",
        "#[test]",
        "fn sums() {}",
    ];
    p.write("contract/README.md", "# The Contract\n")
        .write("contract/sum/README.md", &readme(&["A1"]))
        .write("tests/contract/sum.rs", &sum.join("\n"));
    let out = p.check();
    assert_eq!(out.code, Some(0), "{}", out.stderr);
    assert_eq!(
        out.stdout.trim(),
        "The Contract holds: 4 obligations across 2 primitives."
    );
}

// Contract: check/A2
#[test]
fn an_unproved_obligation_fails_by_name() {
    let out = calc(&["A1", "A2"], &test_fn("adds", &["calc/A1"])).check();
    assert_eq!(out.code, Some(1));
    assert_eq!(out.stderr, format!("calc/A2 has no test\n{NOT_MET}\n"));
}

// Contract: check/A3
#[test]
fn an_untagged_test_fails_by_name() {
    let tests = test_fn("adds", &["calc/A1"]) + &test_fn("subtracts", &[]);
    let out = calc(&["A1"], &tests).check();
    assert_eq!(out.code, Some(1));
    assert_eq!(
        out.stderr,
        format!("calc::subtracts names no obligation\n{NOT_MET}\n")
    );
}

// Contract: check/A4
#[test]
fn a_tag_the_primitive_doesnt_declare_fails_by_name() {
    let tests = test_fn("adds", &["calc/A1", "calc/A9, sum/A1", "calc/one"]);
    let out = calc(&["A1"], &tests).check();
    assert_eq!(out.code, Some(1));
    for tag in ["calc/A9", "sum/A1", "calc/one"] {
        let problem = format!("calc::adds names `{tag}`, which contract/calc doesn't declare");
        assert!(out.reports(&problem), "{problem} not in:\n{}", out.stderr);
    }
}

// Contract: check/A5
#[test]
fn a_bad_obligation_list_fails_with_file_and_line() {
    let file = "contract/calc/README.md";
    let tests = test_fn("adds", &["calc/A1"]);
    for (text, problem) in [
        (
            "# calc\n\n- **A1** Adds.\n- **A1**\n",
            format!("{file}:4: malformed obligation `- **A1**`"),
        ),
        (
            "# calc\n- **Ax** Adds.\n",
            format!("{file}:2: malformed obligation `- **Ax** Adds.`"),
        ),
        (
            "# calc\n- **A1** Adds.\n\n- **A1** Again.\n",
            format!("{file}:4: A1 is declared twice"),
        ),
        ("# calc\n", format!("{file}: declares no obligations")),
    ] {
        let p = calc(&[], &tests);
        p.write(file, text);
        let out = p.check();
        assert_eq!(out.code, Some(1));
        assert_eq!(out.stderr, format!("{problem}\n{NOT_MET}\n"));
    }
}

// Contract: check/A6
#[test]
fn a_primitive_missing_a_file_fails_naming_it() {
    let p = Project::new();
    p.write("contract/calc/README.md", &readme(&["A1"]))
        .write("contract/sum/notes.md", "")
        .write("tests/contract/sum.rs", &test_fn("sums", &["sum/A1"]));
    let out = p.check();
    assert_eq!(out.code, Some(1));
    assert_eq!(
        out.stderr,
        format!(
            "contract/calc: no tests/contract/calc.rs\ncontract/sum: no README.md\n{NOT_MET}\n"
        )
    );
}

// Contract: check/A7
#[test]
fn check_finds_the_contract_above_the_current_directory() {
    let p = calc(&["A1"], &test_fn("adds", &["calc/A1"]));
    let out = p.check_in("tests/contract");
    assert_eq!(out.code, Some(0), "{}", out.stderr);
}

// Contract: check/A7
#[test]
fn check_without_a_contract_fails() {
    let out = Project::new().check();
    assert_eq!(out.code, Some(1));
    assert!(out.stderr.contains("no contract/"), "{}", out.stderr);
}

// Contract: check/A8
#[test]
fn every_problem_is_reported() {
    let p = calc(
        &["A1", "A2"],
        &(test_fn("adds", &["calc/A7"]) + &test_fn("subtracts", &[])),
    );
    p.write("contract/sum/README.md", "# sum\n");
    p.write("tests/contract/sum.rs", "");
    let out = p.check();
    assert_eq!(out.code, Some(1));
    assert_eq!(
        out.stderr,
        format!(
            "calc::adds names `calc/A7`, which contract/calc doesn't declare\n\
             calc::subtracts names no obligation\n\
             calc/A1 has no test\n\
             calc/A2 has no test\n\
             contract/sum/README.md: declares no obligations\n\
             {NOT_MET}\n"
        )
    );
}

fn with_covers(lines: &str) -> Project {
    let p = calc(&[], &test_fn("adds", &["calc/A1"]));
    let covers = format!("{}\n```covers\n{lines}```\n", readme(&["A1"]));
    p.write("contract/calc/README.md", &covers).write(
        "src/calc.rs",
        "pub fn add(a: u8) -> u8 { a }\npub fn add_one() {}\npub fn parse<T>() {}\n",
    );
    p
}

// Contract: check/A9
#[test]
fn covers_naming_defined_functions_hold() {
    let out = with_covers("src/calc.rs::add\n\nsrc/calc.rs::parse\n").check();
    assert_eq!(out.code, Some(0), "{}", out.stderr);
}

// Contract: check/A9
#[test]
fn covers_naming_missing_functions_fail() {
    let out =
        with_covers("src/calc.rs::sub\nsrc/calc.rs::add_\nsrc/gone.rs::add\nsrc/calc.rs\n").check();
    assert_eq!(out.code, Some(1));
    let file = "contract/calc/README.md";
    assert_eq!(
        out.stderr,
        format!(
            "{file}: covers `src/calc.rs::sub`, which doesn't exist\n\
             {file}: covers `src/calc.rs::add_`, which doesn't exist\n\
             {file}: covers `src/gone.rs::add`, which doesn't exist\n\
             {file}: covers line `src/calc.rs` is not <file>::<function>\n\
             {NOT_MET}\n"
        )
    );
}

// Contract: check/A10
#[test]
fn ignored_tests_prove_nothing() {
    let tests = [
        "// Contract: calc/A1",
        "#[test]",
        "#[ignore]",
        "fn adds() {}",
        "",
        "#[ignore = \"slow\"]",
        "// Contract: calc/A1, calc/A2",
        "#[test]",
        "fn subtracts() {}",
        "",
        "// Contract: calc/A3",
        "#[test]",
        "fn multiplies() {}",
    ];
    let out = calc(&["A1", "A2", "A3"], &tests.join("\n")).check();
    assert_eq!(out.code, Some(1));
    assert_eq!(
        out.stderr,
        format!(
            "calc/A1 has no test (ignored: calc::adds, calc::subtracts)\n\
             calc/A2 has no test (ignored: calc::subtracts)\n\
             {NOT_MET}\n"
        )
    );
}

// Contract: check/A11
#[test]
fn a_tag_outside_a_primitive_test_file_fails() {
    let p = calc(&["A1"], &test_fn("adds", &["calc/A1"]));
    p.write(
        "tests/contract/main.rs",
        "mod calc;\n// Contract: calc/A1\n",
    )
    .write("tests/unit/calc.rs", &test_fn("unit", &["calc/A1"]))
    .write("src/lib.rs", "fn f() {\n    // Contract: calc/A1\n}\n")
    .write("src/notes.txt", "// Contract: calc/A1\n");
    let out = p.check();
    assert_eq!(out.code, Some(1));
    let outside = "a Contract tag outside tests/contract/<primitive>.rs";
    assert_eq!(
        out.stderr,
        format!(
            "src/lib.rs:2: {outside}\n\
             tests/contract/main.rs:2: {outside}\n\
             tests/unit/calc.rs:1: {outside}\n\
             {NOT_MET}\n"
        )
    );
}

fn with_surface(lines: &str) -> Project {
    let p = calc(&[], &test_fn("adds", &["calc/A1"]));
    let surface = format!("{}\n```surface\n{lines}```\n", readme(&["A1"]));
    p.write("contract/calc/README.md", &surface);
    p
}

const LISTS_ADD_AND_FIND: &str =
    "[surface]\ncommand = [\"sh\", \"-c\", \"printf 'cli add\\\\n\\\\n  mcp find  \\\\n'\"]\n";

// Contract: check/A12
#[test]
fn surface_lines_must_be_kind_and_name() {
    let out = with_surface("cli add\n\nmcp find\n").check();
    assert_eq!(out.code, Some(0), "{}", out.stderr);
    let out = with_surface("cli\ncli two words\ncli add\n").check();
    assert_eq!(out.code, Some(1));
    let file = "contract/calc/README.md";
    assert_eq!(
        out.stderr,
        format!(
            "{file}: surface line `cli` is not <kind> <name>\n\
             {file}: surface line `cli two words` is not <kind> <name>\n\
             {NOT_MET}\n"
        )
    );
}

// Contract: check/A13
#[test]
fn surface_lines_the_command_prints_hold() {
    let p = with_surface("cli add\nmcp find\n");
    p.write("impish.toml", LISTS_ADD_AND_FIND);
    let out = p.check();
    assert_eq!(out.code, Some(0), "{}", out.stderr);
}

// Contract: check/A13
#[test]
fn a_surface_line_the_command_doesnt_print_fails() {
    let p = with_surface("cli add\nmcp add\ncli nope\n");
    p.write("impish.toml", LISTS_ADD_AND_FIND);
    let out = p.check();
    assert_eq!(out.code, Some(1));
    let file = "contract/calc/README.md";
    assert_eq!(
        out.stderr,
        format!(
            "{file}: surface `mcp add` is not in the surface command's output\n\
             {file}: surface `cli nope` is not in the surface command's output\n\
             {NOT_MET}\n"
        )
    );
}

// Contract: check/A13
#[test]
fn a_failing_surface_command_fails_check() {
    let p = with_surface("cli add\n");
    p.write(
        "impish.toml",
        "[surface]\ncommand = [\"sh\", \"-c\", \"echo broken >&2; exit 3\"]\n",
    );
    let out = p.check();
    assert_eq!(out.code, Some(1));
    assert!(out.stderr.contains("surface command"), "{}", out.stderr);
    assert!(out.stderr.contains("broken"), "{}", out.stderr);
}
