# The Contract

What impish promises, written down as numbered obligations, each proved by a
contract test. The layout is flinch's sibling layout:

```
contract/<primitive>/README.md   purpose, obligations, covers block
tests/contract/<primitive>.rs    the tests that prove them
```

Obligations sit under `## Obligations`, one block each, numbered `A1`, `A2`, …
and never renumbered:

```markdown
## Obligations

### A1 — A held Contract passes
- **must:** say how many obligations were checked, and exit 0
- **why:** optional; the reason the obligation exists
- **record:** optional; the decision records behind it, such as D3, D7
- **kind:** functional
- **status:** active
```

`kind` is one of functional, non-functional, persisted-format, ordering or
security; `status` is active or retired. A retired obligation keeps its number
and needs no test. Every line under `## Obligations` must belong to a block, and
an obligation outside it fails, so a promise can't silently fall out of the
Contract. Fenced code blocks are skipped.

Every test in `tests/contract/<primitive>.rs` names the obligations it proves
with the `impish` crate's attribute. On its own it makes the function a test;
under an existing `#[test]` or `#[tokio::test]` it keeps that one:

```rust
#[impish::contract("cli/A1", "cli/A2")]
fn ...
```

The attribute checks each tag against `contract/` as the test compiles, so
a typo or a retired obligation is a compile error. A plain `#[test]` there
names no obligation and fails, and a test marked
`#[ignore]` proves nothing. The attribute belongs only in
`tests/contract/<primitive>.rs`.

A README may list the code its obligations are about in a covers block of
`<file>::<function>` lines; each must exist:

````markdown
```covers
src/check.rs::run
```
````

A README may also list the surface its tests drive (commands, tools,
endpoints) in a surface block of `<kind> <name>` lines:

````markdown
```surface
cli check
```
````

A README may have several covers or surface blocks, for example one beside
each obligation; every one is read.

To check those lines against the real surface, give impish a command that
prints it, one `<kind> <name>` per line, in `impish.toml`:

```toml
[surface]
command = ["sh", "scripts/surface.sh"]
```

`cargo impish check --write` writes `contract/coverage.json`, the published
coverage of the Contract. Once it exists, `check` fails when it's out of date.
impish's CI publishes it in the job summary of every run.
`cargo impish mutate <primitive> --write` adds whether the primitive's mutate
is met; `check` fails once that result is stale.

Contract tests are black-box: they run the `cargo-impish` binary.

`cargo impish check` holds the Contract to its tests; see `contract/check/`.
`cargo impish mutate <primitive>` breaks the code a primitive covers with
cargo-mutants and credits each kill to the obligations whose tests caught it.
It is strict: an obligation whose tests catch nothing is hollow and fails, a
contract test that catches nothing is blind and fails, and so does any mutant
no contract test catches, unless `mutants.md` declares it.
See `contract/mutate/`.

## Decision records

Why the project is built the way it is lives in `contract/decisions.toml`;
the decisions of one primitive in `contract/<primitive>/decisions.toml`. One
`[[record]]` each:

```toml
[[record]]
id = "D3"                      # required, never reused
date = 2026-11-03              # required
decision = "check never builds or runs the crate"   # required
driver = "a gate that runs code is slow and unsafe" # required
approved_by = "Abarbesgaard"   # required
alternatives = ["run the tests: duplicates cargo test"]
consequences = ["check can't tell whether a test passes"]
depends_on = ["D2"]
superseded_by = "D12"
expires = 2027-01-01

[[record.conditions]]
text = "syn parses the test files"
dep = "syn"
version = "^3"
```

Everything after `approved_by` is optional. A condition is a premise that, if
it stops holding, makes the decision suspect. It has a `text` and at most one
check: `dep` (a crate in `Cargo.lock`, with an optional semver `version`),
`env` (a variable is set), `file` (a path under the project root exists) or
`rustc` (the active rustc matches a semver requirement). A condition with only
`text` is prose.

An obligation points at the records behind it with a `- **record:** D3` field;
several ids are comma-separated. `check` fails on a missing required field, a
repeated id, an unknown field, or a `record:`, `depends_on` or
`superseded_by` that names no record.

`cargo impish conditions` re-checks every condition and prints ok or suspect
per record. Prose conditions count as not machine-checkable. It exits 1 when
any record is suspect.
