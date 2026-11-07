# 0012 — impish only checks that every obligation has a test

**Status:** accepted (#110); supersedes [0005](0005-coverage-is-a-committed-file.md),
[0007](0007-tests-earn-their-place-at-three-grains.md) and
[0011](0011-mutate-is-strict.md)

## Decision

impish is `cargo impish check` and the `#[impish::contract]` attribute, and
nothing else. `check` answers one question: does every active obligation have
a tagged test, and does every contract test name one? Mutation testing,
coverage files, covers and surface blocks, scope ceilings, exceptions,
decision records as data, `regen` and JSON output are gone. Mutation testing
is cargo-mutants' job, run directly in CI on the lines a PR changes.

impish's own Contract was renumbered from A1 as part of this cut, rather than
carried over as a long list of retired obligations.

## Why

By 0.6.31 impish had grown to nine primitives and about ninety obligations,
most of them about mutation. It was slow to run, hard to adopt and hard to
explain, and the one thing other crates needed, a gate that every promise has
a test, was buried under it. cargo-mutants already does mutation testing well.

## Cost

impish no longer says whether a test has teeth, only that it exists. A crate
that wants that runs cargo-mutants itself.
