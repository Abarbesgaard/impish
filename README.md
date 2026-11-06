# impish

[![crates.io](https://img.shields.io/crates/v/cargo-impish.svg)](https://crates.io/crates/cargo-impish)
[![docs.rs](https://img.shields.io/docsrs/impish)](https://docs.rs/impish)
[![CI](https://github.com/Abarbesgaard/impish/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/Abarbesgaard/impish/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/crates/l/cargo-impish.svg)](https://github.com/Abarbesgaard/impish/blob/main/LICENSE)

The Contract gate for Rust, after [Impatient Programming](https://impatientprogramming.org).
A sibling of [flinch](https://github.com/tools4imps/flinch) (Go) and
[covenant](https://github.com/tools4imps/covenant) (Elixir).

A Contract is what a crate promises: numbered obligations, each proved by a
tagged contract test. impish fails the build when the Contract stops being true:
when an obligation has no test, a test names no obligation, a tag is wrong, or
the tests catch none of the bugs put into the code they cover.

## Why a Contract

A test suite says what the code does today. It doesn't say which of those
behaviours are promises, and which are accidents that are free to change.
When a test breaks, you can't tell whether you broke a promise or just a test.

A Contract writes the promises down, one numbered obligation each, next to the
code they're about: what the code *must* do, and *why*. Every obligation is
proved by a test that names it, and every contract test names an obligation.
So the promises and their proofs can't drift apart. A reviewer reads the
Contract to see what changed in a crate's promises, without reading every
test.

This is how [Impatient Programming](https://impatientprogramming.org) keeps
work fast and safe: a change is done when the Contract holds, and a gate, not
a person, checks that it does. impish is that gate for Rust.

## Install

```bash
cargo install cargo-impish
```

Then run it as `cargo impish`. Contract tests use the `impish` crate's
attribute, so add it as a dev-dependency:

```toml
[dev-dependencies]
impish = "0.6"
```

Every merge that changes what ships is a release, published to
[crates.io](https://crates.io/crates/cargo-impish) and listed under
[releases](https://github.com/Abarbesgaard/impish/releases).

## Quick start

A Contract is split into primitives, one directory each under `contract/`.
Write down what the primitive promises in `contract/<primitive>/README.md`:

````markdown
# add

`calc::add` adds two numbers.

## Obligations

### A1 — It adds
- **must:** return the sum of its arguments
- **kind:** functional
- **status:** active

```covers
src/lib.rs::add
```
````

Prove each obligation in `tests/contract/<primitive>.rs`. The attribute names
the obligations the test proves, and on its own makes the function a test:

```rust
#[impish::contract("add/A1")]
fn adds() {
    assert_eq!(calc::add(2, 3), 5);
}
```

A test that already has `#[test]`, `#[tokio::test]` or another attribute whose
path ends in `test` keeps it. Add the line under it, and the test still runs
once:

```rust
#[tokio::test]
#[impish::contract("add/A1")]
async fn adds_later() {
    assert_eq!(calc::add(2, 3), 5);
}
```

List the primitives in `tests/contract/main.rs`, which Cargo builds as the
`contract` test target:

```rust
mod add;
```

Then:

```bash
cargo test               # runs the contract tests; a wrong tag won't compile
cargo impish check       # every obligation is proved, every test names one
cargo impish mutate add  # every obligation's tests catch a bug in the code
```

## Commands

| Command | What it does |
| --- | --- |
| `cargo impish check` | Holds the Contract to its tests. Reports every problem on stderr and exits 1, or says how many obligations held and exits 0. Reads only. Also holds the ceilings in `contract/scope.toml`, and prints how close each count is. See [contract/README.md](https://github.com/Abarbesgaard/impish/blob/main/contract/README.md#scope-ceilings). |
| `cargo impish check --outside` | Also lists the functions in `src/` that no covers line claims. Never fails on them; `check` always prints the covered share. |
| `cargo impish check --write` | Also writes `contract/coverage.json`, the published coverage. Once it exists, `check` fails when it's out of date. |
| `cargo impish mutate <primitive>` | Breaks the code the primitive's covers block names with [cargo-mutants](https://mutants.rs), on a copy of the project, and credits each kill to the obligations whose tests caught it. Fails on an obligation whose tests catch nothing. Needs `cargo install cargo-mutants`. |
| `cargo impish mutate` | With no primitive, mutates every primitive that covers code, and fails if any one's Contract isn't met. |
| `cargo impish mutate --since <ref>` | Mutates only the covered lines changed since the merge base with `<ref>`, uncommitted work included, and all of a primitive whose Contract or tests changed. A run that sees only some mutants doesn't call obligations hollow or tests blind. |
| `cargo impish mutate -j <n>` | Tries up to n mutants at once, each in its own copy of the project under `target/impish/mutate/` with its own target directory. Defaults to half the cores. Output is the same as one at a time. Each copy's first build is a full build, so more jobs means more disk. |
| `cargo impish mutate --whole-suite` | Also runs every test outside the contract target against each mutant. Names an outside test that catches a mutant the Contract misses as a witness of a missing obligation. With no primitive and no `--since`, fails on each outside test whose every catch the contract tests make too: it duplicates the Contract. Doc tests aren't run. |
| `cargo impish mutate <primitive> --write` | Also records in `contract/coverage.json` whether the primitive's mutate is met, with a fingerprint of its README, `mutants.md`, contract tests and covered files. `check` keeps the result and fails once any of those change. |
| `cargo impish conditions` | Re-checks the conditions of every decision record in `contract/decisions.toml` and `contract/<primitive>/decisions.toml`: a crate in `Cargo.lock` (optionally at a version), a set environment variable, a file in the project, or the rustc version. Prints each record as ok or suspect, lists prose conditions as not machine-checkable, and exits 1 when a record is suspect. See [contract/README.md](https://github.com/Abarbesgaard/impish/blob/main/contract/README.md#decision-records) for the format. |
| `cargo impish regen <primitive>` | Writes a pack to rebuild the primitive from its Contract alone, to `target/impish/regen/<primitive>/` or `--out <dir>`: a prompt, the Contract format, the intents, the README, the decision records, the contract tests, and stubs of the covered functions with `todo!()` bodies and none of their code. Hand it to an agent that hasn't seen the code; each guess it makes is a sentence the Contract is missing. Replaces only a previous pack. |
| `--format json` | On `check` or `mutate`: prints one JSON document on stdout instead of the text, with `version`, `met`, every problem, and the counts (for `mutate`, every mutant's outcome and the tests that caught it). Exits the same way. |

## When it fails

Both commands exit 0 when the Contract holds and 1 when it doesn't. They exit
2 when they can't tell, for example with no `contract/` directory, a broken
`impish.toml`, or contract tests that don't build or are red before
`mutate` starts. In CI, 1 means fix the Contract or its tests; 2 means fix the
run.

`check` reports every problem at once, then exits 1:

```text
$ cargo impish check
calc::subtracts names no obligation
calc/A2 has no test
calc/A3 has no test
The Contract is not met.
```

| Message | Fix |
| --- | --- |
| `<test> names no obligation` | Add `#[impish::contract("<primitive>/A<n>")]` under its `#[test]`. Every test in `tests/contract/` proves something, or it belongs elsewhere. |
| `<primitive>/A<n> has no test` | Write a test that proves it, or retire the obligation if it's no longer a promise. |
| `contract/coverage.json is out of date` | Run `cargo impish check --write` and commit the result. |
| `…: the mutate result for <primitive> is stale` | Its Contract, tests or covered code changed since `mutate --write`. Run `cargo impish mutate <primitive> --write` and commit the result. |
| `…: covers <line>, which doesn't exist` | The file, function, type or method was renamed or moved. Update the covers block. |
| `<test>: every mutant it catches, the Contract catches too, so it duplicates the Contract` | From `mutate --whole-suite`. Delete the outside test, or make it catch something the Contract doesn't. |
| `<file>:<line>: a test outside tests/contract/<primitive>.rs` | A test in `main.rs`, a helper, or a file with no `contract/<name>/README.md`. Give it a Contract, or move it out of `tests/contract/`. |

A tag that names no obligation stops at `cargo test`, before `check` runs:

```text
error: `calc/A9`: contract/calc/README.md doesn't declare A9
 --> tests/contract/calc.rs:1:20
  |
1 | #[impish::contract("calc/A9")]
  |                    ^^^^^^^^^
```

## Reading the kill matrix

`check` proves every obligation has a test. `mutate` proves the tests would
notice a bug. It breaks the covered code one change at a time (a mutant), runs
the primitive's contract tests, and credits each failure to the obligations
the failing tests name. It is strict: every mutant must be caught, or declared
in `contract/<primitive>/mutants.md` with a reason
([D11](https://github.com/Abarbesgaard/impish/blob/main/contract/decisions.toml)):

```text
$ cargo impish mutate calc
calc: 10 mutants of 2 functions, 3 tests
  src/lib.rs:2:7: replace + with - in add  killed by calc/A1
  src/lib.rs:6:5: replace sub -> i32 with 1  SURVIVED
  src/lib.rs:6:7: replace - with / in sub  SURVIVED
  …
8 killed, 2 survived, 0 timed out, 0 unviable

  obligation  killed  unique
  calc/A1          5       5
  calc/A2          3       3
  calc/A3          0       0
  unique: caught by no other obligation's tests.
  Tests outside the Contract weren't run; they may catch it too.

  test              killed  unique
  calc::adds             5       3
  calc::adds_again       2       0
  calc::subtracts        3       3
  calc::compares         0       0
  unique: caught by no other contract test.

Mutation-blind tests: another contract test catches every mutant they catch.
Remove them, or say why they stay with blind = "<reason>".
  calc::adds_again
calc/A3 is hollow: its tests catch none of the mutants
calc::compares is blind: it catches none of the mutants
src/lib.rs: replace sub -> i32 with 1 survived: catch it in a contract test, or declare it in contract/calc/mutants.md
src/lib.rs: replace - with / in sub survived: catch it in a contract test, or declare it in contract/calc/mutants.md
The Contract is not met.
```

- **killed:** mutants this obligation's tests caught.
- **unique:** mutants *only* this obligation's tests caught, among the
  Contract's tests. `mutate` runs only those unless given `--whole-suite`,
  so a test outside the Contract may catch a "unique" mutant too. An obligation with kills but no unique
  kills is proved only by bugs other obligations already catch. That's fine for a non-functional promise, but worth a look
  for a functional one.
- **hollow:** an obligation with no kills at all. Its tests pass whatever the
  code does, so they prove nothing. Assert on the result, or retire the
  obligation.
- **blind:** a contract test that catches no mutant, even if its obligation's
  other tests do. It passes whatever the code does. Make it assert something
  the code could get wrong, or delete it.
- **Mutation-blind tests:** contract tests with kills but no unique kill:
  another test catches everything they do. A warning, not a failure, since a
  contract test is also spec. Delete the test, or keep it with
  `#[impish::contract("calc/A1", blind = "<reason>")]`, and the run lists it
  under `Kept with no unique kill` with the reason instead.
- **SURVIVED:** a bug no contract test caught, so the run fails. Here the
  only test is `sub(3, 2) == 1`, and `3 / 2` and a constant `1` give 1 too. A
  second case, say `sub(5, 2) == 3`, kills both. A mutant that times out
  fails the same way: a hang is not a catch.

The run passes only when every obligation and every contract test catches
something and every mutant is caught or declared: `Every mutant of calc is
caught or declared.`

### Declaring mutants

Some mutants can't be caught, and some changes the Contract leaves open on
purpose. Declare them in `contract/<primitive>/mutants.md`, in a fenced block
under a `##` heading that says why:

````markdown
# Mutants calc leaves alone

## sub's result is only ever compared with zero

Callers check `sub(a, b) == 0`, and the Contract promises nothing more.

```unpromised
src/lib.rs: replace - with / in sub
```
````

- **`equivalent`:** no program could tell the mutant from the original.
- **`unpromised`:** behaviour changes, and the Contract leaves it open.

A line is the mutant's name as `mutate` prints it, without the line and
column, so it still matches after the code above it moves. It covers every
mutant with that name. Declared mutants still run, and the run prints them as
`SURVIVED, declared unpromised`. A declaration fails the run when a contract
test catches its mutant (`… is declared equivalent, but calc/A1 catches it`),
when it matches no mutant (`… matches no mutant`), or when its block has no
`##` heading above it.

## The Contract format

An obligation is a `### A<n> — <title>` block under `## Obligations` with
`must`, `kind` and `status` fields and an optional `why`. Numbers are never
reused; an obligation that no longer holds is marked `retired` and needs no
test. `kind` is one of functional, non-functional, persisted-format, ordering,
security or exception. An exception excuses a deliberate gap: it names a
`target` (`<file>::<function>` or `<file>`), needs no test, is counted apart,
and fails `check` once its target is gone.

A README may also list the code it is about in a `covers` block of
`<file>`, `<dir>/...`, `<file>::<item>` or `<file>::<Type>::<method>` lines
(the most specific line owns a function), and the surface its tests drive in a `surface`
block of `<kind> <name>` lines. To check surface lines against the real
surface, put a command that prints it in `impish.toml`:

```toml
[surface]
command = ["sh", "scripts/surface.sh"]
```

impish holds itself to its own Contract: [contract/README.md](https://github.com/Abarbesgaard/impish/blob/main/contract/README.md)
describes the format in full, and each `contract/<primitive>/README.md` is a
working example.

## Changing a Contract

A Contract only grows by append, so an obligation's tag means the same thing
forever, in old commits, issues and PR reviews alike.

- **Adding a promise:** give it the next number, write its test first and
  watch it fail, then make it pass. `check` won't pass until the test exists.
- **Changing a promise:** edit `must` in place when the promise is the same
  but better worded. When the behaviour itself changes, retire the old
  obligation and add a new one, so the history shows the promise changed.
- **Dropping a promise:** set `status: retired`. Keep the block and its
  number. Its tests must go too: a test naming a retired obligation fails to
  compile.
- **Never renumber.** Leave gaps. `A3` retired and `A4` active is a normal
  Contract.

Run `cargo impish check --write` after any change, and commit
`contract/coverage.json` with it.

## In CI

A GitHub Actions workflow that gates every PR on the Contract, and publishes
its coverage in the job summary:

```yaml
name: CI
on: [push, pull_request]
permissions:
  contents: read
jobs:
  contract:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install cargo-impish --locked
      - run: cargo test
      - run: cargo impish check
      - name: Contract coverage
        run: |
          jq -r '"**\(.obligations) active obligations**\n",
                 "| Primitive | Obligations |", "|---|---|",
                 (.primitives | to_entries[] | "| \(.key) | \(.value.obligations) |")' \
            contract/coverage.json >> "$GITHUB_STEP_SUMMARY"
```

`mutate` runs every test once per mutant, so a full run is slow for PRs. On a
PR, mutate only what it changed:

```yaml
      - run: cargo install cargo-mutants --locked
      - run: cargo impish mutate --since origin/${{ github.base_ref }}
```

Check out with `fetch-depth: 0` so the merge base is there. Run the full
kill matrix nightly, one job per primitive:

```yaml
name: Kill matrix
on:
  schedule: [{ cron: "0 2 * * *" }]
  workflow_dispatch:
permissions:
  contents: read
jobs:
  primitives:
    runs-on: ubuntu-latest
    outputs:
      list: ${{ steps.ls.outputs.list }}
    steps:
      - uses: actions/checkout@v7
      - id: ls
        run: echo "list=$(for d in contract/*/; do basename "$d"; done | jq -Rnc '[inputs]')" >> "$GITHUB_OUTPUT"
  mutate:
    needs: primitives
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false
      matrix:
        primitive: ${{ fromJson(needs.primitives.outputs.list) }}
    steps:
      - uses: actions/checkout@v7
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo install cargo-impish cargo-mutants --locked
      - run: cargo impish mutate "${{ matrix.primitive }}"
```

To read the results in a later step, add `--format json` and redirect stdout:
`cargo impish mutate calc --format json > mutate.json`. The document has
`met`, `mutants`, `obligations` and `tests` (the kill matrix) and `problems`.

impish's own [CI](https://github.com/Abarbesgaard/impish/blob/main/.github/workflows/ci.yml) and
[kill matrix](https://github.com/Abarbesgaard/impish/blob/main/.github/workflows/kill-matrix.yml) go further: pinned actions, sharding and a per-test matrix.

## Contributing

[ARCHITECTURE.md](https://github.com/Abarbesgaard/impish/blob/main/ARCHITECTURE.md) says where each part lives and which
guardrails fire on a change; [contract/decisions.toml](https://github.com/Abarbesgaard/impish/blob/main/contract/decisions.toml) says why.

## License

MIT; see [LICENSE](https://github.com/Abarbesgaard/impish/blob/main/LICENSE).
