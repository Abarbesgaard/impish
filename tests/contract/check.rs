use crate::harness::{NOT_MET, Project, calc, readme, test_fn};

#[impish::contract("check/A1")]
fn a_held_contract_passes_and_says_what_it_checked() {
    let tests = test_fn("adds", &["calc/A1, calc/A2"]) + &test_fn("subtracts", &["calc/A3"]);
    let p = calc(&["A1", "A2", "A3"], &tests);
    let sum = [
        "mod nested {",
        "    #[impish::contract(\"sum/A1\")]",
        "    /// Sums.",
        "    #[cfg(unix)]",
        "    fn sums() {}",
        "}",
    ];
    p.write("contract/README.md", "# The Contract\n")
        .write(
            "contract/sum/README.md",
            &readme(&["A1"]).replace(" — ", " - "),
        )
        .write("tests/contract/sum.rs", &sum.join("\n"));
    let out = p.check();
    assert_eq!(out.code, Some(0), "{}", out.stderr);
    assert_eq!(out.stdout.trim(), "2 primitives, 4 obligations, all tested");
}

#[impish::contract("check/A2")]
fn an_unproved_obligation_fails_by_name() {
    let out = calc(&["A1", "A2"], &test_fn("adds", &["calc/A1"])).check();
    assert_eq!(out.code, Some(1));
    assert_eq!(out.stderr, format!("calc/A2 has no test\n{NOT_MET}\n"));
}

#[impish::contract("check/A3")]
fn an_untagged_test_fails_by_name() {
    let tests = test_fn("adds", &["calc/A1"])
        + &test_fn("subtracts", &[])
        + "// Contract: calc/A1\n#[test]\nfn divides() {}\n"
        + "#[allow(dead_code)]\nfn helper() {}\n";
    let out = calc(&["A1"], &tests).check();
    assert_eq!(out.code, Some(1));
    assert_eq!(
        out.stderr,
        format!(
            "calc::subtracts names no obligation\n\
             calc::divides names no obligation\n\
             {NOT_MET}\n"
        )
    );
}

#[impish::contract("check/A4")]
fn a_tag_the_primitive_doesnt_declare_fails_by_name() {
    let tests = test_fn("adds", &["calc/A1", "calc/A9, sum/A1", "calc/one"]);
    let out = calc(&["A1"], &tests).check();
    assert_eq!(out.code, Some(1));
    for tag in ["calc/A9", "sum/A1", "calc/one"] {
        let problem = format!("calc::adds names `{tag}`, which contract/calc doesn't declare");
        assert!(out.reports(&problem), "{problem} not in:\n{}", out.stderr);
    }
}

#[impish::contract("check/A5")]
fn a_malformed_obligation_list_fails_with_file_and_line() {
    let text = [
        "# calc",
        "",
        "## Obligations",
        "",
        "Some prose.",
        "### A1",
        "### Ax - Adds",
        "### A2 — Adds",
        "- **must:** add",
        "- **must:** add",
        "- **scope:** x",
        "- **kind:** fast",
        "- **status:** done",
        "- **why:**",
        "### A2 - Again",
        "- **kind:** functional",
        "### A3 — Fine",
        "- **must:** hold",
        "- **why:** ```not a fence```",
        "- **kind:** functional",
        "- **status:** active",
        "",
        "```text",
        "- **A9** in a fence",
        "```",
        "## Notes",
        "- **A4** Old style.",
        "### A5 — Late",
    ];
    let p = calc(&[], &test_fn("adds", &["calc/A3"]));
    p.write("contract/calc/README.md", &text.join("\n"));
    let out = p.check();
    assert_eq!(out.code, Some(1));
    let f = "contract/calc/README.md";
    let expected = [
        format!("{f}:5: `Some prose.` is not part of an obligation block"),
        format!("{f}:6: malformed obligation heading `### A1`"),
        format!("{f}:7: malformed obligation heading `### Ax - Adds`"),
        format!(
            "{f}:8: A2 has kind `fast`, not one of functional, non-functional, persisted-format, ordering, security"
        ),
        format!("{f}:8: A2 has status `done`, not active or retired"),
        format!("{f}:10: `must` is given twice"),
        format!("{f}:11: unknown field `scope`"),
        format!("{f}:14: `- **why:**` is not part of an obligation block"),
        format!("{f}:15: A2 has no `must`"),
        format!("{f}:15: A2 is declared twice"),
        format!("{f}:27: an obligation outside ## Obligations"),
        format!("{f}:28: an obligation outside ## Obligations"),
        NOT_MET.to_string(),
    ];
    assert_eq!(out.stderr, expected.join("\n") + "\n");
}

#[impish::contract("check/A5")]
fn a_readme_without_obligations_fails() {
    for text in ["# calc\n", "# calc\n\n## Obligations\n\n## Notes\n"] {
        let p = calc(&[], &test_fn("adds", &["calc/A1"]));
        p.write("contract/calc/README.md", text);
        let out = p.check();
        assert_eq!(out.code, Some(1));
        assert!(
            out.reports("contract/calc/README.md: declares no obligations"),
            "{}",
            out.stderr
        );
    }
}

#[impish::contract("check/A4")]
fn retired_obligations_need_no_test_and_take_no_tags() {
    let text = [
        "## Obligations",
        "### A1 — Adds",
        "- **must:** add",
        "### A2 — Subtracts",
        "- **must:** subtract",
        "- **kind:** non-functional",
        "- **status:** retired",
    ]
    .join("\n");
    let p = calc(&[], &test_fn("adds", &["calc/A1"]));
    p.write("contract/calc/README.md", &text);
    let out = p.check();
    assert_eq!(out.code, Some(0), "{}", out.stderr);
    assert_eq!(out.stdout.trim(), "1 primitives, 1 obligations, all tested");
    p.write(
        "tests/contract/calc.rs",
        &test_fn("adds", &["calc/A1, calc/A2"]),
    );
    let out = p.check();
    assert_eq!(out.code, Some(1));
    assert_eq!(
        out.stderr,
        format!("calc::adds names `calc/A2`, which is retired\n{NOT_MET}\n")
    );
}

#[impish::contract("check/A6")]
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

#[impish::contract("check/A7")]
fn check_finds_the_contract_above_and_fails_without_one() {
    let p = calc(&["A1"], &test_fn("adds", &["calc/A1"]));
    let out = p.check_in("tests/contract");
    assert_eq!(out.code, Some(0), "{}", out.stderr);

    let out = Project::new().check();
    assert_eq!(out.code, Some(2));
    assert!(out.stderr.contains("no contract/"), "{}", out.stderr);
}

#[impish::contract("check/A8")]
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

#[impish::contract("check/A9")]
fn ignored_tests_prove_nothing() {
    let tests = [
        "#[impish::contract(\"calc/A1\")]",
        "#[ignore]",
        "fn adds() {}",
        "",
        "#[ignore = \"slow\"]",
        "#[impish::contract(\"calc/A1\", \"calc/A2\")]",
        "fn subtracts() {}",
        "",
        "#[impish::contract(\"calc/A3\")]",
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

#[impish::contract("check/A10")]
fn a_tag_outside_a_primitive_test_file_fails() {
    let p = calc(&["A1"], &test_fn("adds", &["calc/A1"]));
    let attr = "#[impish::contract(\"calc/A1\")]";
    let lib = [
        "// Contract: calc/A1",
        "mod m {",
        &format!("    {attr}"),
        "    fn f() {}",
        &format!("    const S: &str = {attr:?};"),
        "    #[derive(Debug)]",
        "    struct T;",
        "}",
    ];
    p.write(
        "tests/contract/main.rs",
        &format!("mod calc;\n{attr}\nfn f() {{}}\n"),
    )
    .write("tests/unit/calc.rs", &test_fn("unit", &["calc/A1"]))
    .write("src/lib.rs", &lib.join("\n"))
    .write("src/broken.rs", &format!("{attr}\nfn ("))
    .write("src/notes.txt", &format!("{attr}\nfn f() {{}}\n"));
    let out = p.check();
    assert_eq!(out.code, Some(1));
    let outside = "an #[impish::contract] attribute outside tests/contract/<primitive>.rs";
    assert_eq!(
        out.stderr,
        format!(
            "src/lib.rs:3: {outside}\n\
             tests/contract/main.rs:2: {outside}\n\
             tests/unit/calc.rs:1: {outside}\n\
             {NOT_MET}\n"
        )
    );
}

#[impish::contract("check/A3")]
fn a_test_outside_a_primitive_test_file_fails() {
    let p = calc(&["A1"], &test_fn("adds", &["calc/A1"]));
    p.write(
        "tests/contract/main.rs",
        "mod calc;\nmod cite;\nmod harness;\n\n#[test]\nfn guard() {}\n",
    )
    .write(
        "tests/contract/cite.rs",
        "#[test]\nfn cites() {}\n\nfn helper() {}\n",
    )
    .write("tests/contract/harness.rs", "pub fn helper() {}\n")
    .write(
        "tests/contract/nested/deep.rs",
        "mod m {\n    #[tokio::test]\n    async fn later() {}\n}\n",
    )
    .write("tests/unit.rs", "#[test]\nfn unit() {}\n");
    let out = p.check();
    assert_eq!(out.code, Some(1));
    let outside = "a test outside tests/contract/<primitive>.rs";
    assert_eq!(
        out.stderr,
        format!(
            "tests/contract/cite.rs:1: {outside}\n\
             tests/contract/main.rs:5: {outside}\n\
             tests/contract/nested/deep.rs:2: {outside}\n\
             {NOT_MET}\n"
        )
    );
}

#[impish::contract("check/A11")]
fn a_test_file_that_isnt_rust_fails() {
    let p = calc(&["A1"], "\n#[impish::contract(\"calc/A1\")]\nfn adds( {}\n");
    let out = p.check();
    assert_eq!(out.code, Some(1));
    let lines: Vec<&str> = out.stderr.lines().collect();
    assert_eq!(lines.len(), 2, "{}", out.stderr);
    assert!(
        lines[0].starts_with("tests/contract/calc.rs:3: isn't valid Rust: "),
        "{}",
        out.stderr
    );
    assert_eq!(lines[1], NOT_MET);
}

#[impish::contract("check/A12")]
fn check_leaves_the_project_as_it_was() {
    let p = calc(&["A1"], &test_fn("adds", &["calc/A1"]));
    let before = p.files();
    assert_eq!(p.check().code, Some(0));
    assert_eq!(p.files(), before, "a passing check changed the project");
    p.write("contract/calc/README.md", &readme(&["A1", "A2"]));
    let before = p.files();
    assert_eq!(p.check().code, Some(1));
    assert_eq!(p.files(), before, "a failing check changed the project");
}
