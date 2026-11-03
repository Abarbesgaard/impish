# The Contract

What impish promises, written down as numbered obligations, each proved by a
contract test. The layout is flinch's sibling layout:

```
contract/<primitive>/README.md   purpose, obligations, covers block
tests/contract/<primitive>.rs    the tests that prove them
```

Obligations sit under `## Obligations`, one block each, numbered `A1`, `A2`, …
and never renumbered:

```markdown
## Obligations

### A1 — A held Contract passes
- **must:** say how many obligations were checked, and exit 0
- **why:** optional; the reason the obligation exists
- **kind:** functional
- **status:** active
```

`kind` is one of functional, non-functional, persisted-format, ordering or
security; `status` is active or retired. A retired obligation keeps its number
and needs no test. Every line under `## Obligations` must belong to a block, and
an obligation outside it fails, so a promise can't silently fall out of the
Contract. Fenced code blocks are skipped.

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

A README may also list the surface its tests drive (commands, tools,
endpoints) in a surface block of `<kind> <name>` lines:

````markdown
```surface
cli check
```
````

To check those lines against the real surface, give impish a command that
prints it, one `<kind> <name>` per line, in `impish.toml`:

```toml
[surface]
command = ["sh", "scripts/surface.sh"]
```

`cargo impish check --write` writes `contract/coverage.json`, the published
coverage of the Contract. Once it exists, `check` fails when it's out of date.
impish's CI publishes it in the job summary of every run.

Contract tests are black-box: they run the `cargo-impish` binary.

`cargo impish check` holds the Contract to its tests; see `contract/check/`.
