use crate::harness::calc_crate;

const ADDS: &str = "#[impish::contract(\"calc/A1\")]\n\
                    fn adds() {\n    assert_eq!(calc::add(2, 3), 5);\n}\n\n";
// double(2) is also 2 + 2, so `*` → `+` survives.
const DOUBLES: &str = "#[impish::contract(\"calc/A2\")]\n\
                       fn doubles() {\n    assert_eq!(calc::double(2), 4);\n}\n";
const DOUBLES_NOTHING: &str = "#[impish::contract(\"calc/A2\")]\n\
                               fn doubles() {\n    let _ = calc::double(3);\n}\n";

#[impish::contract("mutate/A1", "mutate/A5")]
fn kills_are_credited_to_the_obligations_that_caught_them() {
    let p = calc_crate(&format!("{ADDS}{DOUBLES}"));
    let lib = p.read("src/lib.rs");

    let out = p.mutate("calc");

    assert_eq!(out.code, Some(0), "{}{}", out.stdout, out.stderr);
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

#[impish::contract("mutate/A2")]
fn a_hollow_obligation_fails_by_name() {
    let out = calc_crate(&format!("{ADDS}{DOUBLES_NOTHING}")).mutate("calc");

    assert_eq!(out.code, Some(1), "{}", out.stdout);
    assert_eq!(
        out.stderr,
        "calc/A2 is hollow: its tests catch none of the mutants\nThe Contract is not met.\n"
    );
}

#[impish::contract("mutate/A3")]
fn a_red_baseline_fails_before_mutating() {
    let red = ADDS.replace("5)", "6)");
    let out = calc_crate(&format!("{red}{DOUBLES}")).mutate("calc");

    assert_eq!(out.code, Some(1));
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

    assert_eq!(out.code, Some(1));
    assert!(
        out.stderr
            .contains("contract/calc/README.md covers no code, so there is nothing to mutate"),
        "{}",
        out.stderr
    );
}
