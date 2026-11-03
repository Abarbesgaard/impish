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
