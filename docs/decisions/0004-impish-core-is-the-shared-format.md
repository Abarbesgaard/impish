# 0004 — impish-core is the shared format

**Status:** accepted (#14)

## Decision

The README parser lives in `impish-core`, used by both the binary and the
attribute. It depends on nothing but `std` and does no I/O; callers read the
file and pass its text in.

## Why

The attribute and `check` must agree on what a README declares, or a tag could
compile and then fail `check`, or the other way round. One parser makes
disagreement impossible. Keeping it pure keeps it cheap for a proc macro to
depend on, and easy to test.

## Cost

A third crate in the workspace. `tests/architecture.rs` keeps it pure.
