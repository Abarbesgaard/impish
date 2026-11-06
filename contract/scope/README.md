# scope

Ceilings on what the crate may grow to (Impatient Programming, rule 17: one
person can hold it). `contract/scope.toml` lists `[[ceiling]]` tables, each
with a `name`, what it `counts` (`files`, non-blank `lines`, or `functions`
in `.rs` files), a `glob` of files relative to the root, the `ceiling` and
`why`, and an optional decision `record`. `check` counts each one, fails when
a count is over its ceiling, and says how close every count is. After
covenant's scope ceilings.

## Obligations

### A1 — Malformed ceilings fail by file and line
- **must:** fail `check`, naming `contract/scope.toml` and the line of the `[[ceiling]]`, on a missing or unknown field, a field of the wrong type, a `ceiling` that isn't a whole number above 0, `counts` other than files, lines or functions, a glob that doesn't parse or matches no file, and a `record` that names no decision record
- **kind:** persisted-format
- **status:** active

### A2 — A count over its ceiling fails
- **must:** count the files the glob matches, their non-blank lines, or the functions, methods and nested functions in the matched `.rs` files, leaving out `target/` and hidden directories, and fail `check`, naming the ceiling, its count and its ceiling, when the count is over it
- **why:** a ceiling a machine checks is one nobody has to remember
- **kind:** functional
- **status:** active

### A3 — Every count is reported
- **must:** when the Contract holds, print each ceiling as `<name>: <count> of at most <ceiling>` after the summary, and record each under `scope` in `contract/coverage.json` with what it `counts` and its `ceiling`, but not its count; with no `contract/scope.toml`, print and record nothing about scope
- **why:** a ceiling should be seen coming, not only hit; a recorded count would make `coverage.json` stale on every edit
- **kind:** persisted-format
- **status:** active

```covers
src/check/scope.rs::scope
src/check/scope.rs::ceiling
src/check/scope.rs::functions
```
