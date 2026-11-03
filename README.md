# impish

The Contract gate for Rust, after [Impatient Programming](https://impatientprogramming.org).
A sibling of [flinch](https://github.com/tools4imps/flinch) (Go) and
[covenant](https://github.com/tools4imps/covenant) (Elixir).

A Contract is what a crate promises: numbered obligations, each proved by a
tagged contract test. impish fails the build when the Contract stops being true:
when an obligation has no test, a test names no obligation, a tag is wrong, or
the tests catch none of the bugs put into the code they cover.

## Install

```bash
cargo install cargo-impish
```

Then run it as `cargo impish`. Contract tests use the `impish` crate's
attribute, so add it as a dev-dependency:

```toml
[dev-dependencies]
impish = "0.6"
```

Every merge that changes what ships is a release, published to
[crates.io](https://crates.io/crates/cargo-impish) and listed under
[releases](https://github.com/Abarbesgaard/impish/releases).

## Quick start

A Contract is split into primitives, one directory each under `contract/`.
Write down what the primitive promises in `contract/<primitive>/README.md`:

````markdown
# add

`calc::add` adds two numbers.

## Obligations

### A1 — It adds
- **must:** return the sum of its arguments
- **kind:** functional
- **status:** active

```covers
src/lib.rs::add
```
````

Prove each obligation in `tests/contract/<primitive>.rs`. The attribute takes
the place of `#[test]` and names the obligations the test proves:

```rust
#[impish::contract("add/A1")]
fn adds() {
    assert_eq!(calc::add(2, 3), 5);
}
```

List the primitives in `tests/contract/main.rs`, which Cargo builds as the
`contract` test target:

```rust
mod add;
```

Then:

```bash
cargo test               # runs the contract tests; a wrong tag won't compile
cargo impish check       # every obligation is proved, every test names one
cargo impish mutate add  # every obligation's tests catch a bug in the code
```

## Commands

| Command | What it does |
| --- | --- |
| `cargo impish check` | Holds the Contract to its tests. Reports every problem on stderr and exits 1, or says how many obligations held and exits 0. Reads only. |
| `cargo impish check --write` | Also writes `contract/coverage.json`, the published coverage. Once it exists, `check` fails when it's out of date. |
| `cargo impish mutate <primitive>` | Breaks the code the primitive's covers block names with [cargo-mutants](https://mutants.rs), on a copy of the project, and credits each kill to the obligations whose tests caught it. Fails on an obligation whose tests catch nothing. Needs `cargo install cargo-mutants`. |

## The Contract format

An obligation is a `### A<n> — <title>` block under `## Obligations` with
`must`, `kind` and `status` fields and an optional `why`. Numbers are never
reused; an obligation that no longer holds is marked `retired` and needs no
test. `kind` is one of functional, non-functional, persisted-format, ordering
or security.

A README may also list the code it is about in a `covers` block of
`<file>::<function>` lines, and the surface its tests drive in a `surface`
block of `<kind> <name>` lines. To check surface lines against the real
surface, put a command that prints it in `impish.toml`:

```toml
[surface]
command = ["sh", "scripts/surface.sh"]
```

impish holds itself to its own Contract: [contract/README.md](contract/README.md)
describes the format in full, and each `contract/<primitive>/README.md` is a
working example.

## In CI

```yaml
- run: cargo install cargo-impish --locked
- run: cargo test
- run: cargo impish check
```

impish's own CI also publishes `contract/coverage.json` in the summary of every
run, and a nightly job runs `mutate` on every primitive.

## Contributing

[ARCHITECTURE.md](ARCHITECTURE.md) says where each part lives and which
guardrails fire on a change; [docs/decisions](docs/decisions/) says why.

## License

MIT; see [LICENSE](LICENSE).
