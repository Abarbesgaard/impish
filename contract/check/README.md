# check

`cargo impish check` holds the Contract under `contract/` to its tests. A
primitive is a directory `contract/<p>/` with a `README.md`; its tests are in
`tests/contract/<p>.rs`. Problems go to stderr, one per line, and the exit code
is 1; when the Contract holds the exit code is 0. When `check` can't tell, the
exit code is 2 (cli/A3).

## Obligations

### A1 — A held Contract passes
- **must:** when every active obligation is proved, print `<N> primitives, <M> obligations, all tested` and exit 0
- **kind:** functional
- **status:** active

### A2 — An unproved obligation fails
- **must:** fail, naming each active obligation no test names
- **kind:** functional
- **status:** active

### A3 — An untagged test fails
- **must:** fail, naming each test in `tests/contract/` that names no obligation: a `#[test]` in a primitive's test file, or any test in `main.rs`, a helper module, or a file with no primitive
- **why:** a test there that names no obligation looks like a promise and proves none
- **kind:** functional
- **status:** active

### A4 — An unknown or retired tag fails
- **must:** fail, naming the tag, on a tag for an obligation its primitive doesn't declare, or declares retired
- **kind:** functional
- **status:** active

### A5 — A malformed obligation list fails
- **must:** fail, naming the file and line, when a line under `## Obligations` isn't part of a block, a heading isn't `### A<n> — <title>`, a block has no `must`, a field is unknown or given twice, a `status` isn't active or retired, a `kind` isn't functional, non-functional, persisted-format, ordering or security, a number is declared twice, an obligation sits outside `## Obligations`, or a README declares no obligations
- **kind:** functional
- **status:** active

### A6 — A missing file fails
- **must:** fail, naming what's missing, when a primitive has no README or no test file
- **kind:** functional
- **status:** active

### A7 — The Contract is found from below
- **must:** use the nearest directory, from the current one up, that has `contract/`
- **kind:** functional
- **status:** active

### A8 — Every problem is reported
- **must:** report every problem, not only the first
- **kind:** functional
- **status:** active

### A9 — Ignored tests prove nothing
- **must:** not count a test marked `#[ignore]` as proof, and name the ignored tests when an obligation is left without one
- **kind:** functional
- **status:** active

### A10 — Tags stay in contract tests
- **must:** fail, naming the file and line, on an `#[impish::contract(…)]` attribute in `src/` or `tests/` outside a primitive's test file
- **why:** `check` reads only the primitives' test files, so a tag elsewhere would prove nothing it can see
- **kind:** functional
- **status:** active

### A11 — Test files are Rust
- **must:** fail, naming the file and line, when a primitive's test file isn't valid Rust
- **kind:** functional
- **status:** active

### A12 — check only reads
- **must:** leave every file in the project as it was, whether the Contract holds or not
- **why:** a gate that edits the tree it checks can hide what it found
- **kind:** non-functional
- **status:** active
