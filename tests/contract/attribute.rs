use crate::harness::{Crate, readme};

#[impish::contract("attribute/A1")]
fn a_marked_function_runs_as_a_test() {
    let out = Crate::with_tests(
        r#"
#[impish::contract("p/A1")]
fn proves() {
    panic!("the marked function ran");
}
"#,
    )
    .write("contract/p/README.md", &readme(&["A1"]))
    .test();
    assert!(!out.status.success(), "{}", out.text());
    assert!(
        out.text().contains("the marked function ran"),
        "{}",
        out.text()
    );
    assert!(out.text().contains("1 failed"), "{}", out.text());
}

#[impish::contract("attribute/A2")]
fn naming_no_obligation_fails_to_compile() {
    let out = Crate::with_tests(
        r#"
#[impish::contract()]
fn names_nothing() {}

#[impish::contract(A1)]
fn names_an_ident() {}

#[impish::contract(blind = "kept")]
fn gives_only_a_reason() {}

#[impish::contract("p/A1", colour = "red")]
fn gives_another_key() {}

#[impish::contract("p/A1", blind = 3)]
fn gives_no_reason() {}
"#,
    )
    .test();
    assert!(!out.status.success(), "{}", out.text());
    let says = "name the obligations this test proves: #[impish::contract(\"<primitive>/A<n>\")]";
    assert_eq!(out.text().matches(says).count(), 2, "{}", out.text());
    assert_eq!(
        out.text().matches("expected string literal").count(),
        1,
        "{}",
        out.text()
    );
    let key = "the one key is `blind = \"<reason>\"`: why a test with no unique kill is kept";
    assert_eq!(out.text().matches(key).count(), 2, "{}", out.text());
}

#[impish::contract("attribute/A2")]
fn a_reason_beside_the_tags_compiles() {
    let out = Crate::with_tests("#[impish::contract(\"p/A1\", blind = \"kept\")]\nfn kept() {}\n")
        .write("contract/p/README.md", &readme(&["A1"]))
        .test();
    assert!(out.status.success(), "{}", out.text());
}

#[impish::contract("attribute/A3")]
fn a_wrong_tag_fails_to_compile() {
    let retired = readme(&["A1"])
        + "### A2 — Gone\n- **must:** hold\n- **kind:** functional\n- **status:** retired\n";
    let out = Crate::with_tests(
        r#"
#[impish::contract("p/A1")]
fn holds() {}

#[impish::contract("p/A1", "p/A9")]
fn undeclared() {}

#[impish::contract("p/A2")]
fn retired() {}

#[impish::contract("p1")]
fn malformed() {}

#[impish::contract("r/A1")]
fn missing() {}

#[impish::contract("q/A1")]
fn invalid() {}
"#,
    )
    .write("contract/p/README.md", &retired)
    .write(
        "contract/q/README.md",
        "# q\n\n## Obligations\n\nnot a block\n",
    )
    .test();
    let text = out.text();
    assert!(!out.status.success(), "{text}");
    for says in [
        "`p/A9`: contract/p/README.md doesn't declare A9",
        "`p/A2`: contract/p/README.md declares A2 retired",
        "`p1` isn't <primitive>/A<n>",
        "`r/A1`: contract/r/README.md can't be read",
        "`q/A1`: contract/q/README.md isn't a valid Contract; `cargo impish check` names its problems",
    ] {
        assert_eq!(text.matches(says).count(), 1, "{says} not once in:\n{text}");
    }
    assert!(!text.contains("`p/A1`"), "{text}");
}

#[impish::contract("attribute/A4")]
fn a_changed_readme_is_checked_again() {
    let fixture = Crate::with_tests("#[impish::contract(\"p/A1\")]\nfn holds() {}\n");
    let first = fixture
        .write("contract/p/README.md", &readme(&["A1"]))
        .test();
    assert!(first.status.success(), "{}", first.text());
    let second = fixture
        .write("contract/p/README.md", &readme(&["A2"]))
        .test();
    assert!(!second.status.success(), "{}", second.text());
    let says = "`p/A1`: contract/p/README.md doesn't declare A1";
    assert!(second.text().contains(says), "{}", second.text());
}

#[impish::contract("attribute/A5")]
fn a_tag_that_leaves_contract_fails_to_compile() {
    let out = Crate::with_tests(
        r#"
#[impish::contract("p-1_x/A1")]
fn plain() {}

#[impish::contract("../contract/p-1_x/A1")]
fn climbs() {}

#[impish::contract("p-1_x/../p-1_x/A1")]
fn climbs_inside() {}

#[impish::contract("/A1")]
fn unnamed() {}
"#,
    )
    .write("contract/p-1_x/README.md", &readme(&["A1"]))
    .test();
    let text = out.text();
    assert!(!out.status.success(), "{text}");
    for tag in ["../contract/p-1_x/A1", "p-1_x/../p-1_x/A1", "/A1"] {
        let says = format!("`{tag}`: the primitive must be letters, digits, `-` and `_`");
        assert_eq!(
            text.matches(&says).count(),
            1,
            "{says} not once in:\n{text}"
        );
    }
    assert!(!text.contains("`p-1_x/A1`"), "{text}");
}

#[impish::contract("attribute/A6")]
fn a_test_attribute_already_there_is_kept() {
    let out = Crate::with_tests(
        r#"
#[test]
#[impish::contract("p/A1")]
fn below_test() {}

#[impish::contract("p/A1")]
#[test]
fn above_test() {}

#[::core::prelude::v1::test]
#[impish::contract("p/A1")]
fn below_a_path() {}

#[impish::contract("p/A1")]
#[::core::prelude::v1::test]
fn above_a_path() {}

#[impish::contract("p/A1")]
fn alone() {}

mod shadow {
    #[impish::contract("p/A1")]
    fn below_test() {
        assert!(true);
    }

    #[test]
    #[impish::contract("p/A1")]
    fn nested() {}
}
"#,
    )
    .write("contract/p/README.md", &readme(&["A1"]))
    .test();
    let text = out.text();
    assert!(out.status.success(), "{text}");
    assert!(text.contains("running 7 tests"), "{text}");
    assert!(!text.contains("duplicated attribute"), "{text}");
}
