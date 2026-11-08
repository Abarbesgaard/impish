# 0003 — check is static

**Status:** accepted (#3, #13); amended by [0013](0013-check-writes-the-github-job-summary.md)

## Decision

`cargo impish check` reads READMEs and parses test files. It never builds or
runs the crate or its tests, runs no other process, and changes no file.

## Why

`cargo test` already runs the tests, and the attribute makes their tags
compile-time facts, so `check` only has to compare the Contract with the
tagged tests. That keeps it fast enough to run on every change, and safe to run
anywhere: a gate that edits what it checks can hide what it found.

## Cost

`check` can't tell whether a test passes or has teeth. `cargo test` covers the
first; cargo-mutants covers the second ([0012](0012-impish-only-checks-that-every-obligation-has-a-test.md)).
