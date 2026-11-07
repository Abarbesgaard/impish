# Architecture

impish is a Cargo workspace of three crates. This page, the decision records in
[docs/decisions/](docs/decisions/) and the [Contract](contract/) are enough to
regenerate it (Impatient Programming, rule 17): the Contract says what impish
promises, the records say why it is built this way, and this page says where
each part lives.

## Crates

```
core/      impish-core   the Contract's README format: obligations and their fields
impish/    impish        #[impish::contract]: the attribute users' contract tests carry
src/       cargo-impish  the `cargo impish` binary: check
```

Dependencies point inwards: `cargo-impish` and `impish` both depend on
`impish-core`, never on each other, and `impish-core` depends on nothing but
`std`. ([0004](docs/decisions/0004-impish-core-is-the-shared-format.md))

## cargo-impish

```
src/
  main.rs       parses the command line (clap) and finds the nearest contract/
  check.rs      `cargo impish check`: every obligation has a test
  check/
    stray.rs    contract tags and tests outside the primitives' test files
  contract.rs   reads the tests and #[impish::contract] tags in a file with syn
```

`main` calls `check`, which builds on `contract`; `contract` does no I/O. Only
`check` reads files or prints. `main` prints one thing: the error that stopped
a run, which exits 2 (undecided), not 1 (not met).

`check` is static: it reads READMEs and parses test files, and never builds or
runs a test. `cargo test` runs them, and the attribute ties the two together.
([0003](docs/decisions/0003-check-is-static.md))

## impish (the attribute)

A proc-macro crate with one export, `#[impish::contract("<primitive>/A<n>", …)]`.
It makes the function a test, or goes beside a test attribute already there
such as `#[test]` or `#[tokio::test]`, and checks each tag against
`contract/<primitive>/README.md` while the test compiles, so a wrong tag is a
compile error on that tag. It reports through `compile_error!`, never by
printing or panicking. ([0002](docs/decisions/0002-tags-are-an-attribute-checked-at-compile-time.md), [0010](docs/decisions/0010-the-attribute-goes-beside-the-test-attribute.md))

## The Contract

impish holds its own Contract, in the layout it checks in other crates:

```
contract/README.md               the format
contract/<primitive>/README.md   purpose and obligations
tests/contract/<primitive>.rs    the tests that prove the obligations
```

([0001](docs/decisions/0001-the-contract-is-markdown-blocks.md), [0012](docs/decisions/0012-impish-only-checks-that-every-obligation-has-a-test.md))

## Guardrails

Every rule here is checked by a machine (rule 19). Most fire on every PR
(rule 24); the `CI` job in `.github/workflows/ci.yml` gates them, and green
merges itself (rule 27).

| What | Where |
|---|---|
| Format, Clippy | CI `lint` |
| Tests, then `cargo impish check` on impish itself | CI `test` |
| The released `cargo-impish` checks impish's Contract, as users run it | CI `contract` |
| The suite on macOS and Windows | CI `portability` |
| Architecture: the dependency directions and boundaries above, file size | `tests/architecture.rs` ([0008](docs/decisions/0008-architecture-rules-are-tests-not-obligations.md)) |
| Dependency licences, sources and advisories | CI `deny`, `deny.toml`; nightly `audit.yml` |
| Workflow security | CI `zizmor` |
| No unused dependency | CI `lint`, `cargo machete` |
| No copy-paste in scripts, workflows, Markdown or TOML | CI `lint`, jscpd (`.jscpd.json`) |
| Every changed line is mutated by cargo-mutants, in four shards, and a survivor fails | CI `mutants` |
| No new near-duplicate functions | CI `similar`, `scripts/similar-code.sh` |
| Existing near-duplicates become issues, one per cluster | nightly `similar-backlog.yml`, `scripts/similar-backlog.sh` |
| A change to what ships (`scripts/shipped.sh`) bumps the version | CI `version` ([0006](docs/decisions/0006-every-green-merge-is-a-release.md)) |
| PR titles are `fix:` (with `bug`) or `refine:` (rule 9) | `pr-title.yml` |
| Issues start from obligations: Refinement and Bug forms, no blank issues (rules 1, 20, 28) | `.github/ISSUE_TEMPLATE/` ([0009](docs/decisions/0009-every-issue-starts-from-a-contract.md)) |
| The gates' scripts work: each case fakes the slow tools and checks output and exit code; shellcheck; bash 3.2 | CI `lint`, `tests/pipeline/run.sh` |

On `main`, the `Release` job publishes every new version.
