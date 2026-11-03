use crate::harness::Crate;

// Contract: attribute/A1
#[test]
fn a_marked_function_runs_as_a_test() {
    let out = Crate::with_tests(
        r#"
#[impish::contract("p/A1")]
fn proves() {
    panic!("the marked function ran");
}
"#,
    )
    .test();
    assert!(!out.status.success(), "{}", out.text());
    assert!(
        out.text().contains("the marked function ran"),
        "{}",
        out.text()
    );
    assert!(out.text().contains("1 failed"), "{}", out.text());
}

// Contract: attribute/A2
#[test]
fn naming_no_obligation_fails_to_compile() {
    let out = Crate::with_tests(
        r#"
#[impish::contract()]
fn names_nothing() {}

#[impish::contract(A1)]
fn names_an_ident() {}
"#,
    )
    .test();
    assert!(!out.status.success(), "{}", out.text());
    let says = "name the obligations this test proves: #[impish::contract(\"<primitive>/A<n>\")]";
    assert_eq!(out.text().matches(says).count(), 1, "{}", out.text());
    assert!(
        out.text().contains("expected string literal"),
        "{}",
        out.text()
    );
}
