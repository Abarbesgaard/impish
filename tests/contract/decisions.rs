use crate::harness::{Project, calc, test_fn};

/// A record `id` with `extra` fields after the required ones.
fn record(id: &str, extra: &str) -> String {
    format!(
        "[[record]]\nid = \"{id}\"\ndate = 2026-01-01\ndecision = \"d\"\n\
         driver = \"why\"\napproved_by = \"me\"\n{extra}\n"
    )
}

fn project(records: &str) -> Project {
    let p = calc(&["A1"], &test_fn("adds", &["calc/A1"]));
    p.write("contract/decisions.toml", records);
    p
}

#[impish::contract("decisions/A1")]
fn malformed_records_fail_check_naming_the_file_and_line() {
    let p = project(&format!(
        "{}{}",
        record("D1", ""),
        "[[record]]\nid = \"D2\"\ndate = \"soon\"\ncolour = 1\n\
         depends_on = \"D1\"\nconditions = \"none\"\n"
    ));
    p.write(
        "contract/calc/decisions.toml",
        &format!(
            "{}{}",
            record("D1", ""),
            record(
                "D3",
                "conditions = [{ text = \"t\", env = \"A\", file = \"b\" }]"
            )
        ),
    );
    let out = p.check();
    assert_eq!(out.code, Some(1), "{}", out.stderr);
    for problem in [
        "contract/decisions.toml:8: no `decision`",
        "contract/decisions.toml:8: `date` must be a date",
        "contract/decisions.toml:8: unknown field `colour`",
        "contract/decisions.toml:8: `depends_on` must be a list of strings",
        "contract/decisions.toml:8: `conditions` must be a list of [[record.conditions]] tables",
        "contract/calc/decisions.toml:1: `D1` is declared twice (first at contract/decisions.toml:1)",
    ] {
        assert!(out.reports(problem), "no `{problem}` in\n{}", out.stderr);
    }
    assert!(
        out.stderr.contains("contract/calc/decisions.toml:8: "),
        "{}",
        out.stderr
    );
    let broken = p.run(&["conditions"]);
    assert_eq!(broken.code, Some(2), "{}", broken.stderr);

    let p = project(&record(
        "D1",
        "alternatives = [\"a\"]\nconsequences = [\"c\"]\nexpires = 2030-01-01",
    ));
    let out = p.check();
    assert!(!out.stderr.contains("decisions.toml"), "{}", out.stderr);
    p.write("contract/decisions.toml", "[[record]\n");
    assert!(p.check().stderr.contains("contract/decisions.toml:1: "));
}

#[impish::contract("decisions/A2")]
fn references_that_name_no_record_fail_check() {
    let p = project(&format!(
        "{}{}",
        record("D1", "depends_on = [\"D9\"]"),
        record("D2", "superseded_by = \"D8\"\ndepends_on = [\"D1\"]")
    ));
    p.write(
        "contract/calc/README.md",
        "# A primitive\n\n## Obligations\n\n### A1 — A promise\n- **must:** hold\n- **kind:** functional\n\
         - **record:** D1, D7\n- **status:** active\n",
    );
    let out = p.check();
    assert_eq!(out.code, Some(1), "{}", out.stderr);
    for problem in [
        "contract/decisions.toml:1: depends_on `D9` names no decision record",
        "contract/decisions.toml:8: superseded_by `D8` names no decision record",
        "contract/calc/README.md:8: record `D7` names no decision record",
    ] {
        assert!(out.reports(problem), "no `{problem}` in\n{}", out.stderr);
    }
    assert!(!out.stderr.contains("`D1` names"), "{}", out.stderr);
}

#[impish::contract("decisions/A3")]
fn conditions_are_rechecked_and_a_suspect_record_fails() {
    let lock = "version = 4\n\n[[package]]\nname = \"syn\"\nversion = \"2.0.1\"\n";
    let ok = project(&record(
        "D1",
        "conditions = [\
         { text = \"has syn\", dep = \"syn\" },\
         { text = \"syn 2\", dep = \"syn\", version = \"^2\" },\
         { text = \"path set\", env = \"PATH\" },\
         { text = \"readme\", file = \"contract/calc/README.md\" },\
         { text = \"any rustc\", rustc = \">=1.0\" }]",
    ));
    ok.write("Cargo.lock", lock);
    let out = ok.run(&["conditions"]);
    assert_eq!(out.code, Some(0), "{}{}", out.stdout, out.stderr);
    assert!(
        out.stdout.starts_with("D1 ok\n  ok: has syn\n"),
        "{}",
        out.stdout
    );
    assert!(
        out.stdout
            .ends_with("1 decision records: 1 ok, 0 suspect; 0 conditions not machine-checkable\n"),
        "{}",
        out.stdout
    );

    let bad = project(&format!(
        "{}{}",
        record("D1", "conditions = [{ text = \"fine\", env = \"PATH\" }]"),
        record(
            "D2",
            "conditions = [\
             { text = \"syn 3\", dep = \"syn\", version = \"^3\" },\
             { text = \"serde\", dep = \"serde\" },\
             { text = \"unset\", env = \"IMPISH_SURELY_UNSET_VAR\" },\
             { text = \"gone\", file = \"nope.txt\" },\
             { text = \"old rustc\", rustc = \"<1.0\" }]"
        )
    ));
    let none = bad.run(&["conditions"]);
    assert!(
        none.stdout.contains("(no readable Cargo.lock)"),
        "{}",
        none.stdout
    );
    bad.write("Cargo.lock", lock);
    let out = bad.run(&["conditions"]);
    assert_eq!(out.code, Some(1), "{}{}", out.stdout, out.stderr);
    for line in [
        "D1 ok",
        "D2 suspect",
        "  suspect: syn 3 (Cargo.lock has syn 2.0.1, not ^3)",
        "  suspect: serde (Cargo.lock has no serde)",
        "  suspect: unset ($IMPISH_SURELY_UNSET_VAR is not set)",
        "  suspect: gone (`nope.txt` doesn't exist)",
        "2 decision records: 1 ok, 1 suspect; 0 conditions not machine-checkable",
    ] {
        assert!(
            out.stdout.lines().any(|l| l == line),
            "no `{line}` in\n{}",
            out.stdout
        );
    }
    assert!(
        out.stdout
            .lines()
            .any(|l| l.starts_with("  suspect: old rustc (rustc is ") && l.ends_with(", not <1.0)")),
        "{}",
        out.stdout
    );
}

#[impish::contract("decisions/A4")]
fn prose_conditions_are_listed_and_counted() {
    let p = project(&record(
        "D1",
        "conditions = [{ text = \"people agree\" }, { text = \"so do I\" }]",
    ));
    let out = p.run(&["conditions"]);
    assert_eq!(out.code, Some(0), "{}{}", out.stdout, out.stderr);
    assert_eq!(
        out.stdout,
        "D1 ok\n  not machine-checkable: people agree\n  not machine-checkable: so do I\n\
         1 decision records: 1 ok, 0 suspect; 2 conditions not machine-checkable\n"
    );
}

#[impish::contract("decisions/A5")]
fn a_file_condition_cant_leave_the_project() {
    let p = project(&record(
        "D1",
        "conditions = [\
         { text = \"up\", file = \"../outside\" },\
         { text = \"abs\", file = \"/etc/hosts\" },\
         { text = \"inside\", file = \"contract/../contract/calc/README.md\" }]",
    ));
    let out = p.run(&["conditions"]);
    assert_eq!(out.code, Some(1), "{}{}", out.stdout, out.stderr);
    for line in [
        "  suspect: up (`../outside` leaves the project)",
        "  suspect: abs (`/etc/hosts` leaves the project)",
        "  suspect: inside (`contract/../contract/calc/README.md` leaves the project)",
    ] {
        assert!(
            out.stdout.lines().any(|l| l == line),
            "no `{line}` in\n{}",
            out.stdout
        );
    }

    #[cfg(unix)]
    {
        let outside = tempfile::TempDir::new().unwrap();
        std::fs::write(outside.path().join("secret"), "x").unwrap();
        let p = project(&record(
            "D1",
            "conditions = [{ text = \"link\", file = \"link/secret\" }]",
        ));
        std::os::unix::fs::symlink(outside.path(), p.path().join("link")).unwrap();
        let out = p.run(&["conditions"]);
        assert_eq!(out.code, Some(1), "{}{}", out.stdout, out.stderr);
        assert!(
            out.stdout
                .lines()
                .any(|l| l == "  suspect: link (`link/secret` leaves the project)"),
            "{}",
            out.stdout
        );
    }
}

#[impish::contract("decisions/A6")]
fn a_version_requirement_that_doesnt_parse_is_suspect() {
    let p = project(&record(
        "D1",
        "conditions = [\
         { text = \"dep\", dep = \"syn\", version = \"not a version\" },\
         { text = \"rustc\", rustc = \"also not\" }]",
    ));
    p.write(
        "Cargo.lock",
        "version = 4\n\n[[package]]\nname = \"syn\"\nversion = \"2.0.1\"\n",
    );
    let out = p.run(&["conditions"]);
    assert_eq!(out.code, Some(1), "{}{}", out.stdout, out.stderr);
    for prefix in [
        "  suspect: dep (version requirement `not a version` doesn't parse",
        "  suspect: rustc (version requirement `also not` doesn't parse",
    ] {
        assert!(
            out.stdout.lines().any(|l| l.starts_with(prefix)),
            "no `{prefix}` in\n{}",
            out.stdout
        );
    }
}
