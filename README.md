# impish

[![crates.io](https://img.shields.io/crates/v/cargo-impish.svg)](https://crates.io/crates/cargo-impish)
[![docs.rs](https://img.shields.io/docsrs/impish)](https://docs.rs/impish)
[![CI](https://github.com/Abarbesgaard/impish/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/Abarbesgaard/impish/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/crates/l/cargo-impish.svg)](https://github.com/Abarbesgaard/impish/blob/main/LICENSE)

The Contract gate for Rust, after [Impatient Programming](https://impatientprogramming.org).
A sibling of [flinch](https://github.com/tools4imps/flinch) (Go) and
[covenant](https://github.com/tools4imps/covenant) (Elixir).

A Contract is what a crate promises: numbered obligations, each proved by a
tagged contract test. impish answers one question: does every obligation have a test? It fails the
build when an obligation has no test, a test names no obligation, or a tag is
wrong.

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
impish = "0.7"
```

Every merge that changes what ships is a release, published to
[crates.io](https://crates.io/crates/cargo-impish) and listed under
[releases](https://github.com/Abarbesgaard/impish/releases).

## Quick start

A Contract is split into primitives, one directory each under `contract/`.
Write down what the primitive promises in `contract/<primitive>/README.md`:

```markdown
# add

`calc::add` adds two numbers.

## Obligations

### A1 — It adds
- **must:** return the sum of its arguments
```

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
cargo test           # runs the contract tests; a wrong tag won't compile
cargo impish check   # every obligation has a test, every test names one
```

```text
$ cargo impish check
1 primitives, 1 obligations, all tested
```

`check` is the only command. It reads the READMEs and parses the test files;
it never builds or runs anything, and writes nothing. Run it from anywhere in
the project: it uses the nearest `contract/` above.

## When it fails

`check` exits 0 when the Contract holds and 1 when it doesn't. It exits 2 when
it can't tell, for example with no `contract/` directory or a file it can't
read. In CI, 1 means fix the Contract or its tests; 2 means fix the run.

It reports every problem at once, then exits 1:

```text
$ cargo impish check
calc::subtracts names no obligation
calc/A2 has no test
calc/A3 has no test (ignored: calc::divides)
The Contract is not met.
```

| Message | Fix |
| --- | --- |
| `<primitive>/A<n> has no test` | Write a test that proves it, or retire the obligation if it's no longer a promise. A test marked `#[ignore]` proves nothing. |
| `<test> names no obligation` | Add `#[impish::contract("<primitive>/A<n>")]` under its `#[test]`. Every test in `tests/contract/` proves something, or it belongs elsewhere. |
| ``<test> names `<tag>`, which contract/<p> doesn't declare`` | Fix the tag, or add the obligation to the Contract first. |
| ``<test> names `<tag>`, which is retired`` | Delete the test, or name the obligation that replaced it. |
| `<file>:<line>: a test outside tests/contract/<primitive>.rs` | A test in `main.rs`, a helper, or a file with no `contract/<name>/README.md`. Give it a Contract, or move it out of `tests/contract/`. |
| `<file>:<line>: an #[impish::contract] attribute outside tests/contract/<primitive>.rs` | Move the test into its primitive's test file. |
| `contract/<p>/README.md:<line>: …` | The obligation list is malformed; the message says how. |
| `contract/<p>: no README.md` or `no tests/contract/<p>.rs` | Every primitive needs both. |

A tag that names no obligation stops at `cargo test`, before `check` runs:

```text
error: `calc/A9`: contract/calc/README.md doesn't declare A9
 --> tests/contract/calc.rs:1:20
  |
1 | #[impish::contract("calc/A9")]
  |                    ^^^^^^^^^
```

## The Contract format

An obligation is a `### A<n> — <title>` block under `## Obligations` with a
`must` field, and optional `why`, `kind` and `status` fields. `status` is
active (the default) or retired; a retired obligation keeps its number and
needs no test. `kind` is one of functional, non-functional, persisted-format,
ordering or security. Every line under `## Obligations` must belong to a
block, and an obligation outside it fails, so a promise can't silently fall
out of the Contract.

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

## In CI

A GitHub Actions job that gates every PR on the Contract:

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
```

impish checks that each promise has a test, not that the test is any good.
For that, run [cargo-mutants](https://mutants.rs) on the lines a PR changes,
as impish's own [CI](https://github.com/Abarbesgaard/impish/blob/main/.github/workflows/ci.yml) does.

## Upgrading from 0.6

0.7 cuts impish back to `check` and the attribute (#110). Removed:
`cargo impish mutate`, `conditions` and `regen`; `check --write`, `--outside`
and `--format json`; covers and surface blocks, `impish.toml`,
`contract/coverage.json`, `scope.toml`, `decisions.toml`, `intent.md` and
`mutants.md`; exception obligations and the `record` and `target` fields; and
the attribute's `blind = "…"` key. `kind` and `status` are now optional. Delete
the files and fields, and run cargo-mutants directly for mutation testing.

## Contributing

[ARCHITECTURE.md](https://github.com/Abarbesgaard/impish/blob/main/ARCHITECTURE.md) says where each part lives and which
guardrails fire on a change; [docs/decisions/](https://github.com/Abarbesgaard/impish/tree/main/docs/decisions) says why.

## License

MIT; see [LICENSE](https://github.com/Abarbesgaard/impish/blob/main/LICENSE).
