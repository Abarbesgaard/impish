//! Architecture rules for impish, enforced with ArchUnitRust (`archunit`).
//!
//! archunit builds a dependency graph from every source file in the workspace
//! (`src/`, `core/src/`, `impish/src/`), syntax-based: `use`, `mod`, and
//! qualified paths like `crate::contract::f()`.
//!
//!   core/src     impish-core: the Contract's README format, pure
//!   impish/src   #[impish::contract]: the attribute, on top of core
//!   src          cargo-impish: main → {check, mutate} → contract, on top of core

use archunit::{
    Checkable, FileInfo, ResultFactory, Violation, assert_passes, pattern, project_files,
    project_layers,
};

/// Largest file, in non-blank lines, before it must be split by concern.
const MAX_FILE_LINES: usize = 400;

/// The one line main may print: why a run couldn't decide (exit 2).
const UNDECIDED: &str = r#"eprintln!("Error: {e:?}");"#;

/// Runs several rules and fails once with every violation, so one broken rule
/// does not hide the others.
fn assert_all_pass(rules: &[&dyn Checkable]) {
    let violations: Vec<Violation> = rules
        .iter()
        .flat_map(|rule| rule.check().expect("architecture check should run"))
        .collect();
    let result = ResultFactory::from_violations(&violations);
    assert!(result.passed, "{}", result.message);
}

/// The file's lines with `//` comments dropped, so prose can name what code may not use.
fn code_lines(f: &FileInfo) -> impl Iterator<Item = &str> {
    f.content
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with("//"))
}

fn uses_any(f: &FileInfo, needles: &[&str]) -> bool {
    code_lines(f).any(|l| needles.iter().any(|n| l.contains(n)))
}

#[test]
fn binary_layers_depend_inwards() {
    // check and mutate are the commands; they share the Contract's reading, not each other.
    let rule = project_layers()
        .layer("main")
        .defined_by("src/main.rs")
        .layer("check")
        .defined_by("src/check.rs")
        .layer("mutate")
        .defined_by("src/mutate.rs")
        .layer("contract")
        .defined_by("src/contract.rs")
        .where_layer("contract")
        .may_only_depend_on_layers(&[])
        .where_layer("check")
        .may_only_depend_on_layers(&["contract"])
        .where_layer("mutate")
        .may_only_depend_on_layers(&["contract"]);

    assert_passes!(rule);
}

#[test]
fn crates_depend_inwards() {
    // core is shared by the binary and the attribute, so it knows neither.
    let core_knows_no_workspace_crate = project_files()
        .in_path("core/src/**")
        .should_not()
        .depend_on_external_modules()
        .matching("impish")
        .matching("cargo_impish");
    // The attribute is a dev-dependency for users' tests, never part of the gate.
    let binary_never_uses_the_attribute = project_files()
        .in_path("src/**")
        .should_not()
        .depend_on_external_modules()
        .matching("impish")
        .matching("proc_macro");

    assert_all_pass(&[
        &core_knows_no_workspace_crate,
        &binary_never_uses_the_attribute,
    ]);
}

#[test]
fn core_is_pure() {
    let std_only = project_files()
        .in_path("core/src/**")
        .should()
        .depend_on_external_modules()
        .matching("std");
    let no_io = project_files().in_path("core/src/**").should().adhere_to(
        |f: &FileInfo| {
            !uses_any(
                f,
                &[
                    "fs::",
                    "process::",
                    "env::",
                    "net::",
                    "print!",
                    "println!",
                    "eprint",
                ],
            )
        },
        "do no I/O (callers read the README and pass its text in)",
    );

    assert_all_pass(&[&std_only, &no_io]);
}

#[test]
fn contract_parsing_does_no_io() {
    let rule = project_files()
        .in_file("src/contract.rs")
        .should()
        .adhere_to(
            |f: &FileInfo| !uses_any(f, &["fs::", "Command", "print!", "println!", "eprint"]),
            "do no I/O (src/check.rs reads files and reports)",
        );

    assert_passes!(rule);
}

#[test]
fn only_main_parses_the_command_line() {
    let rule = project_files()
        .in_path(pattern("src/**").except_in_path("src/main.rs"))
        .should_not()
        .depend_on_external_modules()
        .matching("clap");

    assert_passes!(rule);
}

#[test]
fn only_check_reads_config() {
    let rule = project_files()
        .in_path(pattern("src/**").except_in_path("src/check.rs"))
        .should_not()
        .depend_on_external_modules()
        .matching("toml");

    assert_passes!(rule);
}

#[test]
fn only_commands_serialise_and_report() {
    let rule = project_files()
        .in_path(
            pattern("src/**")
                .except_in_path("src/check.rs")
                .except_in_path("src/mutate.rs"),
        )
        .should_not()
        .depend_on_external_modules()
        .matching("serde")
        .matching("serde_json");
    let no_print = project_files()
        .in_path(
            pattern("src/**")
                .except_in_path("src/check.rs")
                .except_in_path("src/mutate.rs"),
        )
        .should()
        .adhere_to(
            |f: &FileInfo| {
                code_lines(f)
                    .filter(|l| {
                        ["print!", "println!", "eprint"]
                            .iter()
                            .any(|n| l.contains(n))
                    })
                    .all(|l| f.path.ends_with("src/main.rs") && l == UNDECIDED)
            },
            "not print (the commands, src/check.rs and src/mutate.rs, report; \
             main prints only the error that stopped a run)",
        );

    assert_all_pass(&[&rule, &no_print]);
}

#[test]
fn the_attribute_reports_through_compile_errors() {
    // Printing from a proc macro lands in the user's build log, not as a
    // diagnostic on the tag that caused it.
    let rule = project_files().in_path("impish/src/**").should().adhere_to(
        |f: &FileInfo| !uses_any(f, &["print!", "println!", "eprint", "panic!"]),
        "not print or panic (return a compile_error! on the tag's span)",
    );

    assert_passes!(rule);
}

#[test]
fn files_stay_small() {
    let rule = project_files().in_path("**").should().adhere_to(
        |f: &FileInfo| f.content.lines().filter(|l| !l.trim().is_empty()).count() <= MAX_FILE_LINES,
        "stay within MAX_FILE_LINES non-blank lines (split by concern)",
    );

    assert_passes!(rule);
}
