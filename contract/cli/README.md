# cli

How impish is run. Cargo runs `cargo impish <args>` as
`cargo-impish impish <args>`, and impish can also be run directly.

## Obligations

- **A1** `cargo impish <args>` and `cargo-impish <args>` behave the same.
- **A2** `--version` prints `cargo impish` and the crate's version, and exits 0.

```covers
src/main.rs::main
```
