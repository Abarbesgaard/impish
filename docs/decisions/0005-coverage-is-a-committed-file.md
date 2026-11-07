# 0005 — Coverage is a committed file

**Status:** superseded by [0012](0012-impish-only-checks-that-every-obligation-has-a-test.md)

## Decision

`cargo impish check --write` writes `contract/coverage.json`: active
obligations in total, per kind and per primitive, and, with a surface command,
how much of the surface the Contract covers. The file is committed, and once it
exists `check` fails when it's stale. CI publishes it in the run summary.

## Why

Impatient Programming rule 29: the Contract is met or not, and its coverage is
published. A committed file makes a change in coverage show up in the PR that
caused it.

## Cost

A PR that changes the Contract also runs `check --write`.
