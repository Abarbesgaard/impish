use crate::harness::{NOT_MET, Project, calc, impish, test_fn};

#[impish::contract("cli/A1")]
fn the_cargo_form_and_the_direct_form_agree() {
    let cargo = impish(&["impish", "--version"]);
    let direct = impish(&["--version"]);
    assert!(cargo.status.success() && direct.status.success());
    assert!(!direct.stdout.is_empty(), "nothing printed");
    assert_eq!(cargo.stdout, direct.stdout);
}

#[impish::contract("cli/A2")]
fn version_names_the_tool_and_its_version() {
    let out = impish(&["--version"]);
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    assert_eq!(
        text.trim(),
        format!("cargo impish {}", env!("CARGO_PKG_VERSION"))
    );
}

#[impish::contract("cli/A3")]
fn a_run_that_cant_decide_exits_2() {
    let no_contract = Project::new().check();
    assert_eq!(no_contract.code, Some(2), "{}", no_contract.stderr);
    assert!(
        no_contract.stderr.contains("no contract/"),
        "{}",
        no_contract.stderr
    );

    let p = calc(&["A1", "A2"], &test_fn("adds", &["calc/A1"]));
    let not_met = p.check();
    assert_eq!(not_met.code, Some(1), "{}", not_met.stderr);
    assert!(not_met.stderr.ends_with(&format!("{NOT_MET}\n")));

    let nothing = p.run(&["mutate", "calc"]);
    assert_eq!(nothing.code, Some(2), "{}", nothing.stderr);
    assert!(
        nothing.stderr.contains("nothing to mutate"),
        "{}",
        nothing.stderr
    );

    p.write("impish.toml", "surface = [");
    let config = p.check();
    assert_eq!(config.code, Some(2), "{}", config.stderr);
    assert!(config.stderr.contains("impish.toml"), "{}", config.stderr);
}
