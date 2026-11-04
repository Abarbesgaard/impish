# 0010 — The attribute goes beside the test attribute

**Status:** accepted; refines [0002](0002-tags-are-an-attribute-checked-at-compile-time.md)

## Decision

`#[impish::contract(…)]` adds `#[test]` only when the function has no test
attribute of its own: `#[test]`, or one whose path ends in `test`, such as
`#[tokio::test]`. Adopting impish in an existing test is one added line, above
or below the attribute already there
([attribute/A6](../../contract/attribute/README.md)).

An attribute below the contract attribute is in the tokens the macro gets. One
above it has already been expanded by rustc, which passes on the bare
function, so the macro reads its own source file (`Span::local_file`) and
looks there for the same function: name, signature and body.

## Why

Replacing `#[test]` rewrote every test a crate brought to impish, and keeping
it registered the test twice. With `#[tokio::test]` below the contract
attribute, the test didn't compile at all. As [IMP rule
13](https://impatientprogramming.org/rules/#rule-13) asks, the change started
as an obligation, attribute/A6, and a test that failed against it.

## Cost

The `impish` crate needs Rust 1.88, and each contract attribute parses its
file once more as the test compiles. A function written twice in one file,
identically, with a test attribute on only one copy, loses the other copy's
`#[test]`. A test
attribute whose path doesn't end in `test` (`#[rstest]`, `#[test_case]`) isn't
recognised.
