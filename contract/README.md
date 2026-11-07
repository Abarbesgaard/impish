# The Contract

What impish promises, written down as numbered obligations, each proved by a
contract test. The layout is flinch's sibling layout:

```
contract/<primitive>/README.md   purpose and obligations
tests/contract/<primitive>.rs    the tests that prove them
```

Obligations sit under `## Obligations`, one block each, numbered `A1`, `A2`, …
and never renumbered:

```markdown
## Obligations

### A1 — A held Contract passes
- **must:** say how many obligations were checked, and exit 0
- **why:** optional; the reason the obligation exists
- **kind:** optional; functional
- **status:** optional; active
```

`must` is required. `kind`, when given, is one of functional, non-functional,
persisted-format, ordering or security; `status` is active (the default) or
retired. A retired obligation keeps its number and needs no test. Every line
under `## Obligations` must belong to a block, and an obligation outside it
fails, so a promise can't silently fall out of the Contract. Fenced code
blocks are skipped.

Every test in `tests/contract/<primitive>.rs` names the obligations it proves
with the `impish` crate's attribute. On its own it makes the function a test;
under an existing `#[test]` or `#[tokio::test]` it keeps that one:

```rust
#[impish::contract("cli/A1", "cli/A2")]
fn ...
```

The attribute checks each tag against `contract/` as the test compiles, so
a typo or a retired obligation is a compile error. A plain `#[test]` there
names no obligation and fails, and a test marked `#[ignore]` proves nothing.
The attribute belongs only in `tests/contract/<primitive>.rs`, and every test
in `tests/contract/` belongs to a primitive.

Contract tests are black-box: they run the `cargo-impish` binary.

`cargo impish check` holds the Contract to its tests; see `contract/check/`.
