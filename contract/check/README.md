# check

`cargo impish check` holds the Contract under `contract/` to its tests. A
primitive is a directory `contract/<p>/` with a `README.md`; its tests are in
`tests/contract/<p>.rs`. Problems go to stderr, one per line, and the exit code
is 1; when the Contract holds the exit code is 0. When `check` can't tell, the
exit code is 2 (cli/A3). In GitHub Actions it also writes a job summary and
annotates the lines its problems name (A13–A18).

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

### A13 — A job summary in GitHub Actions
- **must:** when `GITHUB_STEP_SUMMARY` names a file, append a Markdown summary of the run to it, whether the Contract holds or not, and print and exit exactly as without it
- **why:** on a CI run, the summary is the first thing a reviewer sees
- **kind:** functional
- **status:** active

### A14 — The summary leads with the verdict
- **must:** start the summary with a `##` heading saying whether the Contract holds, then a GitHub alert giving the number of primitives and active obligations: `[!TIP]` when it holds, `[!CAUTION]` with the number of problems when it doesn't
- **kind:** functional
- **status:** active

### A15 — One row per primitive
- **must:** give a table in the summary with one row per primitive, in name order, with its active, tested and retired obligations and a ✅ or ❌, the name linking to its README: at the run's commit when `GITHUB_SERVER_URL`, `GITHUB_REPOSITORY` and `GITHUB_SHA` are set, by its path otherwise
- **kind:** functional
- **status:** active

### A16 — Problems come with a fix
- **must:** when the Contract is not met, list every problem in the summary under its primitive, or under "Elsewhere" when it has none, each with the file and line it names and a one-line fix
- **kind:** functional
- **status:** active

### A17 — Every obligation, folded away
- **must:** list each primitive's obligations in the summary inside a collapsed `<details>` block: its number, its title, and the tests that prove it, or that no test does; a retired obligation is struck through
- **kind:** functional
- **status:** active

### A18 — Problems annotate their line
- **must:** when `GITHUB_ACTIONS` is `true`, also print to stdout each problem that names a file as `::error file=<path>,line=<n>::<problem>`, the path relative to `GITHUB_WORKSPACE` when the project is inside it, and print no annotation otherwise
- **why:** GitHub shows the problem on the line in the PR's diff
- **kind:** functional
- **status:** active

### A19 — An unwritable summary is undecided
- **must:** exit 2, naming the file, when `GITHUB_STEP_SUMMARY` names a file it can't append to
- **kind:** functional
- **status:** active
