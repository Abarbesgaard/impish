//! Contract tests: each proves obligations in contract/; see contract/README.md.

mod attribute;
mod check;
mod cli;
mod harness;
mod mutate;

/// impish's contract tests are made by `#[impish::contract]` itself, so a
/// broken attribute could delete its own proofs and leave `cargo test` green.
/// This plain test makes sure the attribute made them.
#[test]
fn the_attribute_made_the_contract_tests() {
    let out = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--list", "--format", "terse"])
        .output()
        .unwrap();
    let list = String::from_utf8(out.stdout).unwrap();
    assert!(
        list.lines()
            .any(|l| l == "attribute::a_marked_function_runs_as_a_test: test"),
        "{list}"
    );
}
