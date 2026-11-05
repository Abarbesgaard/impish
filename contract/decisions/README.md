# decisions

Decision records: why impish, or a crate it gates, is built the way it is.
They live in the Contract, `contract/decisions.toml` for the project and
`contract/<primitive>/decisions.toml` for one primitive, so a Contract is
enough to rebuild the crate (rule 17). A record's conditions are the premises
it rests on; `cargo impish conditions` re-checks them, so a decision whose
ground has moved shows up as suspect before anyone builds on it. After
covenant's decision records. See `contract/README.md` for the format.

## Obligations

### A1 — Decision records are read from the Contract
- **must:** read `contract/decisions.toml` and `contract/<primitive>/decisions.toml`, each a list of `[[record]]` tables with required `id`, `date`, `decision`, `driver` and `approved_by`, and optional `alternatives`, `consequences`, `depends_on`, `superseded_by`, `conditions` and `expires`; fail `check`, naming the file and line, on a missing required field, a field of the wrong type, a repeated id, an unknown field or a malformed condition, and exit 2 from `conditions`
- **kind:** persisted-format
- **status:** active

### A2 — References resolve
- **must:** fail `check`, naming the file and line, on an obligation's `record:` or a record's `depends_on` or `superseded_by` that names no record
- **kind:** functional
- **status:** active

### A3 — Conditions are re-checked
- **must:** `cargo impish conditions` evaluates every condition with a check, of kind `dep` (a crate in Cargo.lock, optionally at a version matching `version`), `env` (a variable is set), `file` (a path under the project root exists) or `rustc` (the active toolchain matches a version requirement), prints each record with `ok` or `suspect` and each condition with its verdict and why, and exits 1 when any record is suspect
- **kind:** functional
- **status:** active

### A4 — Prose conditions are counted, not skipped silently
- **must:** list a condition with no check as not machine-checkable, and count it in the summary line
- **kind:** functional
- **status:** active

### A5 — A condition can't leave the project
- **must:** treat a `file` path that is absolute or resolves outside the project root (`..` or a symlink) as suspect, without reading it
- **kind:** security
- **status:** active

### A6 — A version requirement that doesn't parse is suspect
- **must:** report a `dep` or `rustc` version requirement that doesn't parse as suspect, never as skipped
- **kind:** functional
- **status:** active

```covers
src/decisions.rs::read
src/decisions.rs::dangling
src/decisions.rs::parse
src/decisions.rs::record
src/decisions.rs::condition
src/decisions/evaluate.rs::evaluate
src/decisions/evaluate.rs::requirement
src/decisions/evaluate.rs::dep
src/decisions/evaluate.rs::file
src/decisions/evaluate.rs::rustc
src/conditions.rs::run
```

```surface
cli conditions
```
