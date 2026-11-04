use std::time::{Duration, SystemTime};

use crate::harness::{NOT_MET, calc_crate};

const ADDS: &str = "#[impish::contract(\"calc/A1\")]\n\
                    fn adds() {\n    assert_eq!(calc::add(2, 3), 5);\n}\n\n";
// double(2) is also 2 + 2, so `*` → `+` survives.
const DOUBLES: &str = "#[impish::contract(\"calc/A2\")]\n\
                       fn doubles() {\n    assert_eq!(calc::double(2), 4);\n}\n";
const DOUBLES_EVERY_WAY: &str = "#[impish::contract(\"calc/A2\")]\n\
                                 fn doubles() {\n    assert_eq!(calc::double(3), 6);\n}\n";
const DOUBLES_NOTHING: &str = "#[impish::contract(\"calc/A2\")]\n\
                               fn doubles() {\n    let _ = calc::double(3);\n}\n";

#[impish::contract("mutate/A1", "mutate/A5")]
fn kills_are_credited_to_the_obligations_that_caught_them() {
    let p = calc_crate(&format!("{ADDS}{DOUBLES}"));
    let lib = p.read("src/lib.rs");

    let out = p.mutate("calc");

    assert!(
        out.stdout
            .contains("src/lib.rs:2:7: replace + with - in add  killed by calc/A1\n"),
        "{}",
        out.stdout
    );
    assert!(
        out.stdout
            .contains("src/lib.rs:6:7: replace * with / in double  killed by calc/A2\n"),
        "{}",
        out.stdout
    );
    assert!(
        out.stdout
            .contains("src/lib.rs:6:7: replace * with + in double  SURVIVED\n"),
        "{}",
        out.stdout
    );
    assert!(
        !out.stdout.contains("in negate"),
        "only covered functions are mutated: {}",
        out.stdout
    );
    let row = |id: &str| {
        out.stdout
            .lines()
            .find(|l| l.trim_start().starts_with(id))
            .map(|l| l.split_whitespace().skip(1).collect::<Vec<_>>())
    };
    let a1 = row("calc/A1").expect("a row for calc/A1");
    assert_eq!(a1[0], a1[1], "every add mutant is caught only by calc/A1");
    assert_ne!(a1[0], "0");
    assert_eq!(p.read("src/lib.rs"), lib, "the project is untouched");
}

#[impish::contract("mutate/A6")]
fn the_kill_matrix_says_what_unique_means() {
    let out = calc_crate(&format!("{ADDS}{DOUBLES}")).mutate("calc");
    let table = out
        .stdout
        .find("  obligation  killed  unique")
        .expect("a matrix");
    let legend = out.stdout[table..].lines().skip(3).collect::<Vec<_>>();
    assert_eq!(
        legend[..2],
        [
            "  unique: caught by no other obligation's tests.",
            "  Tests outside the Contract weren't run; they may catch it too.",
        ],
        "{}",
        out.stdout
    );
}

#[impish::contract("mutate/A7")]
fn each_run_tests_the_code_as_it_is() {
    let p = calc_crate(&format!("{ADDS}{DOUBLES_EVERY_WAY}"));
    // Files older than any build, as a checkout often is, so only their
    // content can tell Cargo the code changed back.
    let age = || p.age(SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000));

    age();
    let first = p.mutate("calc");
    age();
    let second = p.mutate("calc");

    assert_eq!(first.code, Some(0), "{}", first.stderr);
    assert_eq!(second.code, first.code, "{}", second.stderr);
    assert_eq!(second.stdout, first.stdout);
}

#[impish::contract("mutate/A2")]
fn a_hollow_obligation_fails_by_name() {
    let out = calc_crate(&format!("{ADDS}{DOUBLES_NOTHING}")).mutate("calc");

    assert_eq!(out.code, Some(1), "{}", out.stdout);
    assert!(
        out.stderr
            .starts_with("calc/A2 is hollow: its tests catch none of the mutants\n"),
        "{}",
        out.stderr
    );
}

#[impish::contract("mutate/A10")]
fn a_blind_test_fails_by_name() {
    let blind = "#[impish::contract(\"calc/A1\")]\n\
                 fn adds_quietly() {\n    let _ = calc::add(2, 3);\n}\n\n";
    let out = calc_crate(&format!("{ADDS}{blind}{DOUBLES_EVERY_WAY}")).mutate("calc");

    assert_eq!(out.code, Some(1), "{}", out.stdout);
    assert_eq!(
        out.stderr,
        format!("calc::adds_quietly is blind: it catches none of the mutants\n{NOT_MET}\n")
    );
}

#[impish::contract("mutate/A3")]
fn a_red_baseline_fails_before_mutating() {
    let red = ADDS.replace("5)", "6)");
    let out = calc_crate(&format!("{red}{DOUBLES}")).mutate("calc");

    assert_eq!(out.code, Some(2));
    assert!(
        out.stderr
            .contains("the baseline is red: calc::adds fail before any mutation"),
        "{}",
        out.stderr
    );
    assert!(out.stdout.is_empty(), "nothing is mutated: {}", out.stdout);
}

#[impish::contract("mutate/A4")]
fn nothing_to_mutate_fails() {
    let p = calc_crate(&format!("{ADDS}{DOUBLES}"));
    let readme = p.read("contract/calc/README.md");
    p.write(
        "contract/calc/README.md",
        readme.split("```covers").next().unwrap(),
    );

    let out = p.mutate("calc");

    assert_eq!(out.code, Some(2));
    assert!(
        out.stderr
            .contains("contract/calc/README.md covers no code, so there is nothing to mutate"),
        "{}",
        out.stderr
    );
}

const SURVIVOR: &str = "src/lib.rs: replace * with + in double";

/// A mutants.md declaring `names` as `kind`, under a heading giving the
/// reason. The blank line in the block declares nothing.
fn declaring(kind: &str, names: &[&str]) -> String {
    format!(
        "# Mutants calc leaves alone\n\n## Doubling two is adding two\n\n```{kind}\n{}\n\n```\n",
        names.join("\n")
    )
}

#[impish::contract("mutate/A8")]
fn a_survivor_fails_by_name() {
    let out = calc_crate(&format!("{ADDS}{DOUBLES}")).mutate("calc");

    assert_eq!(out.code, Some(1), "{}", out.stdout);
    assert_eq!(
        out.stderr,
        format!(
            "{SURVIVOR} survived: catch it in a contract test, \
             or declare it in contract/calc/mutants.md\n{NOT_MET}\n"
        )
    );
}

#[impish::contract("mutate/A8")]
fn a_declared_survivor_passes() {
    let p = calc_crate(&format!("{ADDS}{DOUBLES}"));
    p.write(
        "contract/calc/mutants.md",
        &declaring("unpromised", &[SURVIVOR]),
    );

    let out = p.mutate("calc");

    assert_eq!(out.code, Some(0), "{}{}", out.stdout, out.stderr);
    assert!(
        out.stdout.contains(
            "src/lib.rs:6:7: replace * with + in double  SURVIVED, declared unpromised\n"
        ),
        "{}",
        out.stdout
    );
}

#[impish::contract("mutate/A8")]
fn every_mutant_caught_passes() {
    let out = calc_crate(&format!("{ADDS}{DOUBLES_EVERY_WAY}")).mutate("calc");

    assert_eq!(out.code, Some(0), "{}{}", out.stdout, out.stderr);
    assert!(!out.stdout.contains("SURVIVED"), "{}", out.stdout);
    assert!(
        out.stdout
            .ends_with("\nEvery mutant of calc is caught or declared.\n"),
        "{}",
        out.stdout
    );
}

#[impish::contract("mutate/A8")]
fn a_timed_out_mutant_fails_by_name() {
    let p = calc_crate(
        "#[impish::contract(\"calc/A1\", \"calc/A2\")]\n\
         fn counts() {\n    assert_eq!(calc::count(3), 3);\n}\n",
    );
    // `i *= 1` never reaches n, so that mutant loops until it is stopped.
    p.write(
        "src/lib.rs",
        "pub fn count(n: u32) -> u32 {\n    let mut i = 0;\n    while i < n {\n        i += 1;\n    }\n    i\n}\n",
    );
    let readme = p.read("contract/calc/README.md");
    let covers = readme.find("```covers").expect("a covers block");
    p.write(
        "contract/calc/README.md",
        &format!("{}```covers\nsrc/lib.rs::count\n```\n", &readme[..covers]),
    );

    let out = p.mutate("calc");

    assert_eq!(out.code, Some(1), "{}{}", out.stdout, out.stderr);
    assert_eq!(
        out.stderr,
        format!(
            "src/lib.rs: replace += with *= in count timed out: catch it in a contract test, \
             or declare it in contract/calc/mutants.md\n{NOT_MET}\n"
        )
    );
}

#[impish::contract("mutate/A9")]
fn a_caught_declaration_fails() {
    let p = calc_crate(&format!("{ADDS}{DOUBLES_EVERY_WAY}"));
    p.write(
        "contract/calc/mutants.md",
        &declaring("equivalent", &["src/lib.rs: replace + with - in add"]),
    );

    let out = p.mutate("calc");

    assert_eq!(out.code, Some(1), "{}", out.stdout);
    assert_eq!(
        out.stderr,
        format!(
            "contract/calc/mutants.md:6: src/lib.rs: replace + with - in add is declared \
             equivalent, but calc/A1 catches it\n{NOT_MET}\n"
        )
    );
}

#[impish::contract("mutate/A9")]
fn a_declaration_matching_no_mutant_fails() {
    let p = calc_crate(&format!("{ADDS}{DOUBLES_EVERY_WAY}"));
    p.write(
        "contract/calc/mutants.md",
        &declaring("unpromised", &["src/lib.rs: replace / with * in add"]),
    );

    let out = p.mutate("calc");

    assert_eq!(out.code, Some(1), "{}", out.stdout);
    assert_eq!(
        out.stderr,
        format!(
            "contract/calc/mutants.md:6: src/lib.rs: replace / with * in add matches no mutant\n{NOT_MET}\n"
        )
    );
}

#[impish::contract("mutate/A9")]
fn a_declaration_without_a_reason_fails() {
    let p = calc_crate(&format!("{ADDS}{DOUBLES}"));
    p.write(
        "contract/calc/mutants.md",
        &format!("# Mutants calc leaves alone\n\n```unpromised\n{SURVIVOR}\n```\n"),
    );

    let out = p.mutate("calc");

    assert_eq!(out.code, Some(1), "{}", out.stdout);
    assert_eq!(
        out.stderr,
        format!(
            "contract/calc/mutants.md:3: an unpromised block with no ## heading giving its reason\n{NOT_MET}\n"
        )
    );
    assert!(out.stdout.is_empty(), "nothing is mutated: {}", out.stdout);
}

#[impish::contract("mutate/A9")]
fn an_unreadable_declaration_file_stops_the_run() {
    let p = calc_crate(&format!("{ADDS}{DOUBLES}"));
    p.write("contract/calc/mutants.md/keep", "");

    let out = p.mutate("calc");

    assert_eq!(out.code, Some(2), "{}{}", out.stdout, out.stderr);
    assert!(
        out.stderr.contains("read contract/calc/mutants.md"),
        "{}",
        out.stderr
    );
}

#[impish::contract("mutate/A11")]
fn json_reports_what_the_text_does() {
    let out = calc_crate(&format!("{ADDS}{DOUBLES}")).mutate_with("calc", &["--format", "json"]);

    assert_eq!(out.code, Some(1), "{}", out.stdout);
    assert_eq!(out.stderr, "");
    let doc: serde_json::Value = serde_json::from_str(&out.stdout).expect("one JSON document");
    assert_eq!(doc["version"], 1);
    assert_eq!(doc["primitive"], "calc");
    assert_eq!(doc["met"], false);
    let mutant = |name: &str| {
        doc["mutants"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["name"] == name)
            .unwrap_or_else(|| panic!("no {name} in {doc}"))
            .clone()
    };
    assert_eq!(
        mutant("src/lib.rs:2:7: replace + with - in add"),
        serde_json::json!({
            "name": "src/lib.rs:2:7: replace + with - in add",
            "outcome": "killed",
            "caught_by": ["calc/A1"],
            "tests": ["calc::adds"],
            "declared": null,
        })
    );
    assert_eq!(
        mutant("src/lib.rs:6:7: replace * with + in double")["outcome"],
        "survived"
    );
    let mutants = doc["mutants"].as_array().unwrap();
    let count = |o: &str| mutants.iter().filter(|m| m["outcome"] == o).count();
    assert_eq!(doc["killed"], count("killed"));
    assert_eq!(doc["survived"], 1);
    assert_eq!(doc["timed_out"], 0);
    assert_eq!(doc["unviable"], count("unviable"));
    let rows = doc["obligations"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["id"], "calc/A1");
    assert_ne!(rows[0]["killed"], 0);
    assert_eq!(rows[0]["killed"], rows[0]["unique"]);
    assert_eq!(
        doc["problems"],
        serde_json::json!([format!(
            "{SURVIVOR} survived: catch it in a contract test, \
             or declare it in contract/calc/mutants.md"
        )])
    );
}

#[impish::contract("mutate/A11")]
fn json_names_a_declaration() {
    let p = calc_crate(&format!("{ADDS}{DOUBLES}"));
    p.write(
        "contract/calc/mutants.md",
        &declaring("unpromised", &[SURVIVOR]),
    );

    let out = p.mutate_with("calc", &["--format", "json"]);

    assert_eq!(out.code, Some(0), "{}", out.stdout);
    let doc: serde_json::Value = serde_json::from_str(&out.stdout).expect("one JSON document");
    assert_eq!(doc["met"], true);
    let declared: Vec<_> = doc["mutants"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|m| !m["declared"].is_null())
        .collect();
    assert_eq!(declared.len(), 1, "{doc}");
    assert_eq!(declared[0]["declared"], "unpromised");
    assert_eq!(declared[0]["outcome"], "survived");
}
