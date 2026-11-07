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
- **must:** exit 2, saying why, when impish can't tell whether the Contract holds, such as no `contract/` directory in the current one or above it, or a README or test file that exists but can't be read; and exit 1 only when the Contract isn't met
- **why:** CI must tell "the Contract isn't met" from "the run is broken"
- **kind:** functional
- **status:** active

### A4 — check is the one command
- **must:** list `check` as the only command in `--help`
- **why:** impish answers one question; mutation is cargo-mutants' job (#110)
- **kind:** functional
- **status:** active
