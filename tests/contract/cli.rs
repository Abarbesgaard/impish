use crate::harness::impish;

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
