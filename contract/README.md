# The Contract

What impish promises, written down as numbered obligations, each proved by a
contract test. The layout is flinch's sibling layout:

```
contract/<primitive>/README.md   purpose, obligations, covers block
tests/contract/<primitive>.rs    the tests that prove them
```

An obligation is one line, numbered `A1`, `A2`, … and never renumbered:

```markdown
- **A1** One sentence a test can prove.
```

Every `#[test]` in `tests/contract/<primitive>.rs` names the obligations it
proves in a comment above it:

```rust
// Contract: cli/A1
#[test]
fn ...
```

A test marked `#[ignore]` proves nothing. Tags belong only in
`tests/contract/<primitive>.rs`.

A README may list the code its obligations are about in a covers block of
`<file>::<function>` lines; each must exist:

````markdown
```covers
src/check.rs::run
```
````

Contract tests are black-box: they run the `cargo-impish` binary.

`cargo impish check` holds the Contract to its tests; see `contract/check/`.
