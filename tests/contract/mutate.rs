use std::fs;
use std::time::{Duration, SystemTime};

use crate::harness::{NOT_MET, Project, calc_crate, readme, test_fn};

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
fn json_names_each_mutant_its_own_declaration() {
    let p = calc_crate(&format!("{ADDS}{DOUBLES}"));
    let killed = "src/lib.rs: replace + with - in add";
    p.write(
        "contract/calc/mutants.md",
        &format!(
            "{}{}",
            declaring("equivalent", &[killed]),
            declaring("unpromised", &[SURVIVOR]).replacen("# Mutants calc leaves alone\n\n", "", 1)
        ),
    );

    let out = p.mutate_with("calc", &["--format", "json"]);

    assert_eq!(out.code, Some(1), "{}", out.stdout);
    let doc: serde_json::Value = serde_json::from_str(&out.stdout).expect("one JSON document");
    let declared: Vec<_> = doc["mutants"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|m| !m["declared"].is_null())
        .map(|m| {
            (
                m["outcome"].as_str().unwrap(),
                m["declared"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(declared, [("survived", "unpromised")], "{doc}");
}

#[impish::contract("mutate/A12")]
fn write_records_whether_mutate_is_met() {
    let p = calc_crate(&format!("{ADDS}{DOUBLES_EVERY_WAY}"));
    assert_eq!(p.run(&["check", "--write"]).code, Some(0));
    let published: serde_json::Value =
        serde_json::from_str(&p.read("contract/coverage.json")).unwrap();

    let out = p.mutate_with("calc", &["--write"]);

    assert_eq!(out.code, Some(0), "{}", out.stderr);
    let mut doc: serde_json::Value =
        serde_json::from_str(&p.read("contract/coverage.json")).unwrap();
    let result = doc["primitives"]["calc"]
        .as_object_mut()
        .unwrap()
        .remove("mutate")
        .expect("a mutate result");
    assert_eq!(result["met"], true);
    let fingerprint = result["fingerprint"].as_str().unwrap().to_string();
    assert_eq!(fingerprint.len(), 16, "{result}");
    assert_eq!(doc, published, "--write changed more than the result");

    p.write("tests/contract/calc.rs", &format!("{ADDS}{DOUBLES}"));
    assert_eq!(p.mutate_with("calc", &["--write"]).code, Some(1));
    let doc: serde_json::Value = serde_json::from_str(&p.read("contract/coverage.json")).unwrap();
    let result = &doc["primitives"]["calc"]["mutate"];
    assert_eq!(result["met"], false);
    assert_ne!(result["fingerprint"], fingerprint.as_str());
}

/// calc, plus neg covering negate with `negates`, plus plain, which covers
/// no code.
fn three_primitives(negates: &str) -> Project {
    let p = calc_crate(&format!("{ADDS}{DOUBLES_EVERY_WAY}"));
    p.write(
        "contract/neg/README.md",
        &(readme(&["A1"]) + "```covers\nsrc/lib.rs::negate\n```\n"),
    )
    .write("tests/contract/neg.rs", negates)
    .write("contract/plain/README.md", &readme(&["A1"]))
    .write("tests/contract/plain.rs", &test_fn("plain", &["plain/A1"]))
    .write(
        "tests/contract/main.rs",
        "mod calc;\nmod neg;\nmod plain;\n",
    );
    p
}

const NEGATES: &str = "#[impish::contract(\"neg/A1\")]\n\
                       fn negates() {\n    assert_eq!(calc::negate(2), -2);\n}\n";

#[impish::contract("mutate/A13")]
fn no_primitive_mutates_every_one() {
    let p = three_primitives(NEGATES);

    let out = p.mutate_args(&[]);

    assert_eq!(out.code, Some(0), "{}{}", out.stdout, out.stderr);
    assert!(out.stdout.contains("calc: "), "{}", out.stdout);
    assert!(
        out.stdout
            .contains("Every mutant of calc is caught or declared.")
    );
    assert!(
        out.stdout
            .contains("Every mutant of neg is caught or declared.")
    );
    assert!(
        out.stdout
            .contains("plain: covers no code, so there is nothing to mutate\n"),
        "{}",
        out.stdout
    );

    p.write(
        "tests/contract/neg.rs",
        "#[impish::contract(\"neg/A1\")]\nfn negates() {\n    let _ = calc::negate(2);\n}\n",
    );
    let out = p.mutate_args(&["--format", "json"]);
    assert_eq!(out.code, Some(1), "{}", out.stdout);
    let doc: serde_json::Value = serde_json::from_str(&out.stdout).expect("one JSON document");
    assert_eq!(doc["version"], 1);
    assert_eq!(doc["met"], false);
    assert_eq!(doc["primitives"]["calc"]["met"], true);
    assert_eq!(doc["primitives"]["neg"]["met"], false);
    assert!(doc["primitives"]["plain"].is_null(), "{doc}");

    let out = p.mutate_args(&[]);
    assert_eq!(out.code, Some(1));
    assert!(out.stderr.contains("neg/A1 is hollow"), "{}", out.stderr);
    assert_eq!(out.stderr.matches(NOT_MET).count(), 1, "{}", out.stderr);
}

#[impish::contract("mutate/A14")]
fn since_mutates_what_changed() {
    let p = calc_crate(&format!("{ADDS}{DOUBLES}"));
    p.commit();
    let since = |r: &str| p.mutate_with("calc", &["--since", r]);
    let nothing = "calc: no covered line changed since HEAD\n\n";

    let out = since("HEAD");
    assert_eq!((out.code, out.stdout.as_str()), (Some(0), nothing));

    // Two lines ending just above double's body touch neither function.
    let lib = p.read("src/lib.rs");
    let lib = lib.replace(
        "}\n\npub fn double(a: i32) -> i32 {\n",
        "}\n// twice\npub fn double(a: i32) -> i32 { // a + a\n",
    );
    p.write("src/lib.rs", &lib);
    let out = since("HEAD");
    assert_eq!((out.code, out.stdout.as_str()), (Some(0), nothing));

    p.write("src/lib.rs", &lib.replace("a + b", "b + a"));
    let out = since("HEAD");
    assert_eq!(out.code, Some(0), "{}{}", out.stdout, out.stderr);
    assert!(
        out.stdout.contains(" in add  killed by calc/A1\n"),
        "{}",
        out.stdout
    );
    assert!(!out.stdout.contains(" in double "), "{}", out.stdout);
    assert!(!out.stderr.contains("hollow") && !out.stderr.contains("blind"));
    p.commit();

    let lib = p.read("src/lib.rs");
    p.write(
        "src/lib.rs",
        &lib.replace("    a * 2\n", "    let b = a;\n    a + b\n"),
    );
    let out = since("HEAD");
    assert!(out.stdout.contains(" in double "), "{}", out.stdout);
    assert!(!out.stdout.contains(" in add "), "{}", out.stdout);
    p.write("src/lib.rs", &lib);

    let tests = p.read("tests/contract/calc.rs");
    p.write("tests/contract/calc.rs", &format!("{tests}\n"));
    let out = since("HEAD");
    assert_eq!(out.code, Some(1), "{}", out.stdout);
    assert!(out.stdout.contains(" in add "), "{}", out.stdout);
    assert!(
        out.stderr.contains(&format!("{SURVIVOR} survived")),
        "{}",
        out.stderr
    );

    assert_eq!(
        p.mutate_with("calc", &["--since", "HEAD", "--write"]).code,
        Some(2)
    );
}

/// calc with `*` → `+` in double declared unpromised, and tests outside the
/// Contract: one the Contract duplicates, one that witnesses the survivor,
/// and one that catches nothing covered.
fn outside_tests() -> Project {
    let p = calc_crate(&format!("{ADDS}{DOUBLES}"));
    p.write(
        "contract/calc/mutants.md",
        &declaring("unpromised", &[SURVIVOR]),
    )
    .write(
        "tests/other.rs",
        "#[test]\nfn adds_again() {\n    assert_eq!(calc::add(2, 3), 5);\n}\n\n\
         #[test]\nfn doubles_three() {\n    assert_eq!(calc::double(3), 6);\n}\n\n\
         #[test]\nfn negates() {\n    assert_eq!(calc::negate(2), -2);\n}\n",
    );
    p
}

#[impish::contract("mutate/A15")]
fn whole_suite_names_witnesses() {
    let p = outside_tests();

    let out = p.mutate_with("calc", &["--whole-suite"]);

    assert_eq!(out.code, Some(0), "{}{}", out.stdout, out.stderr);
    assert!(
        out.stdout
            .contains("SURVIVED, declared unpromised, witnessed by other::doubles_three\n"),
        "{}",
        out.stdout
    );
    assert!(
        !out.stdout
            .lines()
            .any(|l| l.contains("killed by") && l.contains("witnessed")),
        "{}",
        out.stdout
    );
    assert!(
        !out.stdout
            .contains("Tests outside the Contract weren't run")
    );
    assert!(
        out.stdout.contains(
            "Duplicates outside the Contract need a run of every primitive, with no --since."
        ),
        "{}",
        out.stdout
    );

    let out = p.mutate_with("calc", &["--whole-suite", "--format", "json"]);
    assert_eq!(out.code, Some(0), "{}", out.stderr);
    let doc: serde_json::Value = serde_json::from_str(&out.stdout).expect("one JSON document");
    let outside = |name: &str| {
        doc["mutants"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["name"].as_str().unwrap().contains(name))
            .unwrap_or_else(|| panic!("no mutant {name} in {doc}"))["outside"]
            .clone()
    };
    assert_eq!(
        outside("replace * with + in double"),
        serde_json::json!(["other::doubles_three"])
    );
    assert_eq!(
        outside("replace + with - in add"),
        serde_json::json!(["other::adds_again"])
    );
}

#[impish::contract("mutate/A16")]
fn whole_suite_fails_on_duplicates() {
    let p = outside_tests();

    let out = p.mutate_args(&["--whole-suite"]);

    assert_eq!(out.code, Some(1), "{}{}", out.stdout, out.stderr);
    assert!(
        out.stderr.contains(
            "other::adds_again: every mutant it catches, the Contract catches too, \
             so it duplicates the Contract"
        ),
        "{}",
        out.stderr
    );
    assert!(!out.stderr.contains("doubles_three"), "{}", out.stderr);
    assert!(!out.stderr.contains("other::negates"), "{}", out.stderr);
    assert_eq!(out.stderr.matches(NOT_MET).count(), 1, "{}", out.stderr);

    let out = p.mutate_args(&["--whole-suite", "--format", "json"]);
    assert_eq!(out.code, Some(1), "{}", out.stderr);
    let doc: serde_json::Value = serde_json::from_str(&out.stdout).expect("one JSON document");
    assert_eq!(doc["met"], false);
    assert_eq!(doc["primitives"]["calc"]["met"], true);
    assert_eq!(doc["duplicates"], serde_json::json!(["other::adds_again"]));

    p.commit();
    let out = p.mutate_args(&["--whole-suite", "--since", "HEAD"]);
    assert_eq!(out.code, Some(0), "{}{}", out.stdout, out.stderr);
}

#[impish::contract("mutate/A17")]
fn jobs_try_mutants_at_once_in_order() {
    let p = calc_crate(&format!("{ADDS}{DOUBLES}"));
    let copies = || {
        (0..64)
            .filter(|i| {
                let name = if *i == 0 {
                    "project".to_string()
                } else {
                    format!("project-{i}")
                };
                p.path().join("target/impish/mutate").join(name).is_dir()
            })
            .count()
    };

    let serial = p.mutate_with("calc", &["-j", "1"]);
    let parallel = p.mutate_with("calc", &["-j", "3"]);

    assert_eq!(parallel.code, Some(1), "{}", parallel.stderr);
    assert_eq!(parallel.stdout, serial.stdout);
    assert_eq!(parallel.stderr, serial.stderr);
    assert_eq!(copies(), 3);

    fs::remove_dir_all(p.path().join("target/impish/mutate")).unwrap();
    let mutants = parallel
        .stdout
        .lines()
        .filter(|l| l.starts_with("  src/"))
        .count();
    let half = std::thread::available_parallelism().map_or(1, |n| n.get() / 2);
    let out = p.mutate_any_jobs(&["calc"]);
    assert_eq!(out.stdout, serial.stdout);
    assert_eq!(copies(), half.clamp(1, mutants));
}
