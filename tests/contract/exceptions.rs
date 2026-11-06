use crate::harness::{Crate, Project, calc, readme, test_fn};

/// An exception `A<id>` excusing `target`, or none when it's empty.
fn exception(id: &str, target: &str) -> String {
    let target = match target {
        "" => String::new(),
        t => format!("- **target:** {t}\n"),
    };
    format!(
        "### {id} — Proven elsewhere\n- **must:** hold\n- **kind:** exception\n\
         {target}- **status:** active\n\n"
    )
}

fn project(extra: &str, tests: &str) -> Project {
    let p = calc(&["A1"], &(test_fn("adds", &["calc/A1"]) + tests));
    p.write("contract/calc/README.md", &(readme(&["A1"]) + extra))
        .write("src/lib.rs", "pub fn add() {}\n");
    p
}

#[impish::contract("exceptions/A1")]
fn an_exception_names_its_target() {
    let extra = exception("A2", "")
        + &exception("A3", "src/lib.rs::add")
        + "### A4 — Targeted\n- **must:** hold\n- **kind:** functional\n\
           - **target:** src/lib.rs\n- **status:** active\n";
    let out = project(&extra, "").check();
    assert_eq!(out.code, Some(1), "{}", out.stderr);
    let f = "contract/calc/README.md";
    assert_eq!(
        out.stderr,
        format!(
            "{f}:10: A2 is an exception with no `target`\n\
             {f}:21: A4 has a `target`, which only an exception takes\n\
             The Contract is not met.\n"
        )
    );
}

#[impish::contract("exceptions/A2")]
fn an_exception_needs_no_test() {
    let extra = exception("A2", "src/lib.rs::add");
    let held = project(&extra, "").check();
    assert_eq!(held.code, Some(0), "{}", held.stderr);
    let out = project(&extra, &test_fn("excused", &["calc/A2"])).check();
    assert_eq!(out.code, Some(1));
    assert_eq!(
        out.stderr,
        "calc::excused names `calc/A2`, which is an exception\nThe Contract is not met.\n"
    );
    let compiled = Crate::with_tests("#[impish::contract(\"p/A2\")]\nfn excused() {}\n")
        .write("contract/p/README.md", &(readme(&["A1"]) + &extra))
        .test();
    let text = compiled.text();
    assert!(!compiled.status.success(), "{text}");
    assert!(
        text.contains(
            "`p/A2`: contract/p/README.md declares A2 an exception, which no test proves"
        ),
        "{text}"
    );
}

#[impish::contract("exceptions/A3")]
fn an_exception_dies_with_its_target() {
    let extra = exception("A2", "src/lib.rs::add")
        + &exception("A3", "src/lib.rs")
        + &exception("A4", "src/lib.rs::gone")
        + &exception("A5", "src/gone.rs")
        + &exception("A6", "../outside.rs")
        + &exception("A7", "/etc/passwd::root");
    let out = project(&extra, "").check();
    assert_eq!(out.code, Some(1));
    assert_eq!(
        out.stderr,
        "calc/A4 excuses `src/lib.rs::gone`, which no longer exists\n\
         calc/A5 excuses `src/gone.rs`, which no longer exists\n\
         calc/A6 excuses `../outside.rs`, which is outside the crate\n\
         calc/A7 excuses `/etc/passwd::root`, which is outside the crate\n\
         The Contract is not met.\n"
    );
    let retired = exception("A2", "src/gone.rs").replace("active", "retired");
    assert_eq!(project(&retired, "").check().code, Some(0));
}

#[impish::contract("exceptions/A4")]
fn exceptions_are_published_apart() {
    let extra = exception("A2", "src/lib.rs::add")
        + &exception("A3", "src/lib.rs")
        + &exception("A4", "src/gone.rs").replace("active", "retired");
    let p = project(&extra, "");
    let out = p.check();
    assert_eq!(
        out.stdout.lines().next(),
        Some("The Contract holds: 1 obligations across 1 primitives.")
    );
    assert_eq!(p.run(&["check", "--write"]).code, Some(0));
    let json: serde_json::Value = serde_json::from_str(&p.read("contract/coverage.json")).unwrap();
    assert_eq!(json["obligations"], 1);
    assert_eq!(json["kinds"], serde_json::json!({"functional": 1}));
    assert_eq!(json["primitives"]["calc"]["obligations"], 1);
    assert_eq!(
        json["exceptions"],
        serde_json::json!({"calc/A2": "src/lib.rs::add", "calc/A3": "src/lib.rs"})
    );
    let plain = project("", "");
    assert_eq!(plain.run(&["check", "--write"]).code, Some(0));
    assert!(!plain.read("contract/coverage.json").contains("exceptions"));
}
