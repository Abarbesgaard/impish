# check

`cargo impish check` holds the Contract under `contract/` to its tests. A
primitive is a directory `contract/<p>/` with a `README.md`; its tests are in
`tests/contract/<p>.rs`. Problems go to stderr, one per line, and the exit code
is 1; when the Contract holds the exit code is 0.

## Obligations

### A1 — A held Contract passes
- **must:** when every obligation is proved, say how many obligations and primitives were checked, and exit 0
- **kind:** functional
- **status:** active

### A2 — An unproved obligation fails
- **must:** fail, naming each obligation no test names
- **kind:** functional
- **status:** active

### A3 — An untagged test fails
- **must:** fail, naming each test that names no obligation: a `#[test]` in a primitive's test file, where `#[impish::contract(…)]` belongs
- **kind:** functional
- **status:** active

### A4 — An unknown tag fails
- **must:** fail, naming each tag for an obligation its primitive doesn't declare
- **kind:** functional
- **status:** active

### A5 — A malformed obligation list fails
- **must:** fail, naming the file and line, when a line under `## Obligations` isn't part of a block, a heading isn't `### A<n> — <title>`, a field is unknown, given twice or missing (`must`, `kind`, `status`), a number is declared twice, or a README declares no obligations
- **kind:** functional
- **status:** active

### A6 — A missing file fails
- **must:** fail, naming what's missing, when a primitive has no README or no test file
- **kind:** functional
- **status:** active

### A7 — The Contract is found from below
- **must:** use the nearest directory, from the current one up, that has `contract/`, and fail when there is none
- **kind:** functional
- **status:** active

### A8 — Every problem is reported
- **must:** report every problem, not only the first
- **kind:** functional
- **status:** active

### A9 — Covers name real functions
- **must:** fail, naming it, on a covers line that isn't `<file>::<function>` or names a function the file doesn't define
- **kind:** functional
- **status:** active

### A10 — Ignored tests prove nothing
- **must:** not count a test marked `#[ignore]` as proof, and name the ignored tests when an obligation is left without one
- **kind:** functional
- **status:** active

### A11 — Tags stay in contract tests
- **must:** fail, naming the file and line, on an `#[impish::contract(…)]` attribute in `src/` or `tests/` outside a primitive's test file
- **kind:** functional
- **status:** active

### A12 — Surface lines are kind and name
- **must:** fail, naming it, on a surface line that isn't `<kind> <name>`
- **kind:** functional
- **status:** active

### A13 — Surface lines are real
- **must:** with `[surface] command` in `impish.toml`, fail, naming it, on a surface line the command doesn't print, and fail when the command fails
- **kind:** functional
- **status:** active

### A14 — Kind and status are known
- **must:** fail, naming the file and line, on a `kind` other than functional, non-functional, persisted-format, ordering or security, or a `status` other than active or retired
- **kind:** functional
- **status:** active

### A15 — Retired obligations need no test
- **must:** not require a test for a retired obligation, and fail, naming the tag, on a test that names one
- **kind:** functional
- **status:** active

### A16 — Obligations live under ## Obligations
- **must:** fail, naming the file and line, on an obligation heading or a `- **A<n>**` line outside `## Obligations`
- **kind:** functional
- **status:** active

### A17 — --write publishes coverage
- **must:** with `--write`, write `contract/coverage.json`: the active obligations in total, per kind and per primitive with the primitive's surface lines, and, with a surface command, how many of its lines some primitive covers, of how many, as a percentage, and which it doesn't
- **why:** Impatient Programming rule 29: the Contract's coverage is published
- **kind:** persisted-format
- **status:** active

### A18 — Stale coverage fails
- **must:** fail, saying to run `cargo impish check --write`, when `contract/coverage.json` exists and isn't what `--write` would write
- **kind:** functional
- **status:** active

### A19 — Test files are Rust
- **must:** fail, naming the file and line, when a primitive's test file isn't valid Rust
- **kind:** functional
- **status:** active

```covers
src/check.rs::run
src/check.rs::check
src/contract.rs::obligations
src/contract.rs::tagged_tests
src/contract.rs::covers
src/contract.rs::contract_attributes
src/contract.rs::surface
src/check.rs::real_surface
src/check.rs::coverage
```

```surface
cli check
```
