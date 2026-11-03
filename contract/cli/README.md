# cli

How impish is run. Cargo runs `cargo impish <args>` as
`cargo-impish impish <args>`, and impish can also be run directly.

## Obligations

### A1 — Both invocations agree
- **must:** make `cargo impish <args>` and `cargo-impish <args>` behave the same
- **kind:** functional
- **status:** active

### A2 — --version names the tool
- **must:** print `cargo impish` and the crate's version for `--version`, and exit 0
- **kind:** functional
- **status:** active

```covers
src/main.rs::main
```
