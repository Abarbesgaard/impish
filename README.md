# impish

The Contract gate for Rust, after [Impatient Programming](https://impatientprogramming.org).
A sibling of [flinch](https://github.com/tools4imps/flinch) (Go) and
[covenant](https://github.com/tools4imps/covenant) (Elixir).

A Contract is what a crate promises: numbered obligations, each proved by a
tagged contract test. impish fails the build when the Contract stops being true.

impish's own coverage is published in the summary of every CI run.

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

`cargo impish check` holds the Contract to its tests. `cargo impish mutate
<primitive>` checks that those tests have teeth: it breaks the code the
primitive covers with [cargo-mutants](https://mutants.rs) and fails on an
obligation whose tests catch none of the breaks.

## Contributing

[ARCHITECTURE.md](ARCHITECTURE.md) says where each part lives and which
guardrails fire on a change; [docs/decisions](docs/decisions/) says why.
