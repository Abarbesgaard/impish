# 0008 — Architecture rules are tests, not obligations

**Status:** accepted

## Decision

The dependency directions and boundaries in [ARCHITECTURE.md](../../ARCHITECTURE.md)
are enforced by `tests/architecture.rs` with archunit, as plain `#[test]`s,
outside the Contract.

## Why

The Contract is what impish promises its users; how its code is arranged isn't
one of those promises, and could change without any user noticing. Putting it
in the Contract would mix the two and inflate coverage. The rules still fire on
every change (rule 24), as tests that catch structure, not behaviour.

## Cost

A rule change is a code change to the test file, not a Contract change.
