# impish

The Contract gate for Rust, after [Impatient Programming](https://impatientprogramming.org).
A sibling of [flinch](https://github.com/tools4imps/flinch) (Go) and
[covenant](https://github.com/tools4imps/covenant) (Elixir).

A Contract is what a crate promises: numbered obligations, each proved by a
tagged contract test. impish fails the build when the Contract stops being true.

impish's own coverage is published in the summary of every CI run.

## Install

```bash
cargo install --git https://github.com/Abarbesgaard/impish
```

Then run it as `cargo impish`.

Every merge that changes what ships is a release; add `--tag v<version>` to
install a specific one from the [releases](https://github.com/Abarbesgaard/impish/releases).
