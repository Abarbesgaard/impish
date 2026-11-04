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

### A3 — A run that can't decide exits 2
- **must:** exit 2, saying why, when impish can't tell whether the Contract holds, such as no `contract/` directory, an unreadable `impish.toml`, contract tests that don't build or are red before mutating, or nothing to mutate; and exit 1 only when the Contract isn't met
- **why:** CI must tell "the tests are weak" from "the run is broken", as flinch does (#29)
- **kind:** functional
- **status:** active

```covers
src/main.rs::main
```
