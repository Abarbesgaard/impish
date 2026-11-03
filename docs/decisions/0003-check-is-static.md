# 0003 — check is static

**Status:** accepted (#3, #13)

## Decision

`cargo impish check` reads READMEs and parses test files. It never builds or
runs the crate or its tests, and it changes no file unless `--write` is given.
The only process it runs is the surface command a crate configures in
`impish.toml`.

## Why

`cargo test` already runs the tests, and the attribute makes their tags
compile-time facts, so `check` only has to compare the Contract with the
tagged tests. That keeps it fast enough to run on every change, and safe to run
anywhere: a gate that edits what it checks can hide what it found.

## Cost

`check` can't tell whether a test passes or has teeth. `cargo test` covers the
first; `cargo impish mutate` covers the second ([0007](0007-tests-earn-their-place-at-three-grains.md)).
