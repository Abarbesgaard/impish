use std::fs;

use crate::harness::{Checked, NOT_MET, Project, calc, readme, test_fn};

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

/// calc: A1 proved, A2 not, A3 retired; sum: both proved, but a test that
/// names nothing; mul holds; and a tag in src/. Three problems, each at a
/// file and line.
fn ledger() -> Project {
    let calc = [
        "# calc",
        "",
        "## Obligations",
        "",
        "### A1 — Adds",
        "- **must:** add",
        "",
        "### A2 — Pipes | splits",
        "- **must:** split",
        "",
        "### A3 — Divides",
        "- **must:** divide",
        "- **status:** retired",
    ];
    let p = Project::new();
    p.write("contract/calc/README.md", &calc.join("\n"))
        .write("tests/contract/calc.rs", &test_fn("adds", &["calc/A1"]))
        .write("contract/mul/README.md", &readme(&["A1"]))
        .write("tests/contract/mul.rs", &test_fn("muls", &["mul/A1"]))
        .write("contract/sum/README.md", &readme(&["A1", "A2"]))
        .write(
            "tests/contract/sum.rs",
            &(test_fn("sums", &["sum/A1, sum/A2"]) + &test_fn("loose", &[])),
        )
        .write(
            "src/lib.rs",
            "#[impish::contract(\"calc/A1\")]\nfn stray() {}\n",
        );
    p
}

/// A held Contract: sum alone.
fn held() -> Project {
    let p = Project::new();
    p.write("contract/sum/README.md", &readme(&["A1", "A2"]))
        .write(
            "tests/contract/sum.rs",
            &test_fn("sums", &["sum/A1, sum/A2"]),
        );
    p
}

/// `check` with a job summary file, and what it wrote there.
fn summarised(p: &Project, env: &[(&str, &str)]) -> (Checked, String) {
    let dir = tempfile::TempDir::new().unwrap();
    let file = dir.path().join("summary.md");
    fs::write(&file, "before\n").unwrap();
    let file = file.to_str().unwrap();
    let mut env = env.to_vec();
    env.push(("GITHUB_STEP_SUMMARY", file));
    let out = p.check_env(&env);
    let text = fs::read_to_string(file).unwrap();
    let summary = text
        .strip_prefix("before\n")
        .expect("appended, not replaced");
    (out, summary.to_string())
}

#[impish::contract("check/A13")]
fn a_job_summary_is_appended_and_output_is_unchanged() {
    for p in [held(), ledger()] {
        let plain = p.check();
        let (out, summary) = summarised(&p, &[]);
        assert_eq!(
            (out.code, &out.stdout, &out.stderr),
            (plain.code, &plain.stdout, &plain.stderr)
        );
        assert!(summary.starts_with("## "), "{summary}");
    }
}

#[impish::contract("check/A14")]
fn the_summary_leads_with_the_verdict() {
    let (_, summary) = summarised(&held(), &[]);
    assert!(
        summary.starts_with(
            "## ✅ The Contract holds\n\n> [!TIP]\n> **1 primitives, 2 obligations, all tested.**\n"
        ),
        "{summary}"
    );
    let (_, summary) = summarised(&ledger(), &[]);
    assert!(
        summary.starts_with(
            "## ❌ The Contract is not met\n\n> [!CAUTION]\n> **3 primitives, 5 obligations, 3 problems.**\n"
        ),
        "{summary}"
    );
}

#[impish::contract("check/A15")]
fn the_summary_has_a_row_per_primitive() {
    let (_, summary) = summarised(&ledger(), &[]);
    let rows = "| Primitive | Active | Tested | Retired | |\n\
                | :-- | --: | --: | --: | :-: |\n\
                | [calc](contract/calc/README.md) | 2 | 1 | 1 | ❌ |\n\
                | [mul](contract/mul/README.md) | 1 | 1 | 0 | ✅ |\n\
                | [sum](contract/sum/README.md) | 2 | 2 | 0 | ❌ |\n";
    assert!(summary.contains(rows), "{summary}");
    let (_, summary) = summarised(&held(), &[]);
    let row = "| [sum](contract/sum/README.md) | 2 | 2 | 0 | ✅ |\n";
    assert!(summary.contains(row), "{summary}");
    let github = [
        ("GITHUB_SERVER_URL", "https://github.example"),
        ("GITHUB_REPOSITORY", "o/r"),
        ("GITHUB_SHA", "abc123"),
    ];
    let (_, summary) = summarised(&ledger(), &github);
    let row = "| [sum](https://github.example/o/r/blob/abc123/contract/sum/README.md) | 2 | 2 | 0 | ❌ |\n";
    assert!(summary.contains(row), "{summary}");
    let (_, summary) = summarised(&ledger(), &github[..2]);
    assert!(
        summary.contains("| [sum](contract/sum/README.md) |"),
        "{summary}"
    );
}

#[impish::contract("check/A16")]
fn the_summary_lists_each_problem_with_where_and_a_fix() {
    let (_, summary) = summarised(&ledger(), &[]);
    let problems = &summary[summary.find("### Problems\n").expect(&summary)..];
    let expected = [
        "**calc**",
        "- calc/A2 has no test  ",
        "  `contract/calc/README.md:8` · ",
        "**sum**",
        "- sum::loose names no obligation  ",
        "  `tests/contract/sum.rs:5` · ",
        "**Elsewhere**",
        "- src/lib.rs:1: an #[impish::contract] attribute outside tests/contract/\\<primitive>.rs  ",
        "  `src/lib.rs:1` · ",
    ];
    let mut rest = problems;
    for want in expected {
        let at = rest
            .find(want)
            .unwrap_or_else(|| panic!("{want:?} in order in:\n{problems}"));
        rest = &rest[at + want.len()..];
        if want.ends_with(" · ") {
            let fix = rest.lines().next().unwrap();
            assert!(
                fix.len() > 10 && fix.ends_with('.'),
                "a fix after {want:?}: {fix:?}"
            );
        }
    }
    assert!(!problems.contains("**mul**"), "{problems}");
    let (_, summary) = summarised(&held(), &[]);
    assert!(!summary.contains("### Problems"), "{summary}");
}

#[impish::contract("check/A17")]
fn the_summary_folds_away_every_obligation() {
    let (_, summary) = summarised(&ledger(), &[]);
    let calc = "<details>\n<summary><b>calc</b> · 2 active, 1 tested</summary>\n\n\
                | | Obligation | Proved by |\n\
                | :-- | :-- | :-- |\n\
                | A1 | Adds | `calc::adds` |\n\
                | A2 | Pipes \\| splits | no test |\n\
                | ~~A3~~ | ~~Divides~~ | retired |\n\n\
                </details>\n";
    assert!(summary.contains(calc), "{summary}");
    assert!(summary.contains("<summary><b>sum</b> · 2 active, 2 tested</summary>"));
    assert!(!summary.contains("<details open"));
}

#[impish::contract("check/A18")]
fn problems_annotate_their_line_in_github_actions() {
    let p = ledger();
    let out = p.check_env(&[("GITHUB_ACTIONS", "true")]);
    let annotations: Vec<&str> = out.stdout.lines().filter(|l| l.starts_with("::")).collect();
    assert_eq!(
        annotations,
        [
            "::error file=contract/calc/README.md,line=8::calc/A2 has no test",
            "::error file=tests/contract/sum.rs,line=5::sum::loose names no obligation",
            "::error file=src/lib.rs,line=1::src/lib.rs:1: an #[impish::contract] attribute outside tests/contract/<primitive>.rs",
        ]
    );
    assert_eq!(out.code, Some(1));

    let parent = p.path().parent().unwrap().to_str().unwrap();
    let name = p.path().file_name().unwrap().to_str().unwrap();
    let out = p.check_env(&[("GITHUB_ACTIONS", "true"), ("GITHUB_WORKSPACE", parent)]);
    let want = format!("::error file={name}/src/lib.rs,line=1::");
    assert!(out.stdout.contains(&want), "{want} in:\n{}", out.stdout);

    let out = p.check_env(&[("GITHUB_ACTIONS", "false")]);
    assert!(!out.stdout.contains("::"), "{}", out.stdout);

    let odd = calc(&["A1"], &test_fn("adds", &["calc/A1", "calc/A%1\r\nx"]));
    let out = odd.check_env(&[("GITHUB_ACTIONS", "true")]);
    let want = "::calc::adds names `calc/A%251%0D%0Ax`, which contract/calc doesn't declare";
    assert!(out.stdout.contains(want), "{want} in:\n{}", out.stdout);
}

#[impish::contract("check/A19")]
fn an_unwritable_summary_is_undecided() {
    let dir = tempfile::TempDir::new().unwrap();
    let file = dir.path().to_str().unwrap();
    let out = held().check_env(&[("GITHUB_STEP_SUMMARY", file)]);
    assert_eq!(out.code, Some(2));
    assert!(out.stderr.contains(file), "{}", out.stderr);
}
