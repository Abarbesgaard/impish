# Architecture

impish is a Cargo workspace of three crates. This page, the decision records in
[contract/decisions.toml](contract/decisions.toml) and the [Contract](contract/) are enough to
regenerate it (Impatient Programming, rule 17): the Contract says what impish
promises, the records say why it is built this way, and this page says where
each part lives.

## Crates

```
core/      impish-core   the Contract's README format: obligations, kinds, statuses
impish/    impish        #[impish::contract]: the attribute users' contract tests carry
src/       cargo-impish  the `cargo impish` binary: check and mutate
```

Dependencies point inwards: `cargo-impish` and `impish` both depend on
`impish-core`, never on each other, and `impish-core` depends on nothing but
`std`. ([D4](contract/decisions.toml))

## cargo-impish

```
src/
  main.rs       parses the command line (clap) and finds the nearest contract/
  check.rs      `cargo impish check`: holds the Contract to its tests, writes coverage
  check/
    stray.rs    contract tags and tests outside the primitives' test files
  conditions.rs `cargo impish conditions`: re-checks the decision records' conditions
  decisions.rs  reads decision records from contract/, checks their shape and references
  decisions/
    evaluate.rs judges one condition: dep, env, file or rustc
  regen.rs      `cargo impish regen`: a pack to rebuild a primitive from its Contract
  regen/
    stubs.rs    the covered functions' signatures, with `todo!()` bodies
  mutate.rs     `cargo impish mutate`: a kill matrix per obligation, via cargo-mutants;
                every mutant caught or declared in mutants.md
  mutate/
    scratch.rs  the copy of the project mutate breaks, one mutant at a time
    judge.rs    the verdict on the mutants, and its JSON document
    since.rs    what changed since a git ref, for `mutate --since`
    suite.rs    witnesses and duplicates outside the Contract, for `--whole-suite`
    record.rs   the mutate result `--write` leaves in coverage.json
    pool.rs     tries mutants on several scratch copies at once, for `-j`
    list.rs     the mutants cargo-mutants makes of the covered code
  contract.rs   reads covers and surface blocks, mutants.md declarations, and
                #[impish::contract] tags with syn
```

`main` calls the commands; `check` and `mutate` both build on `contract`, never
on each other; `check` and `conditions` both build on `decisions`, which knows
neither; `regen` builds on `contract` alone; `contract` does no I/O. Only the commands read or write files,
run processes or print, and only `check` (`impish.toml`) and `decisions`
(`decisions.toml`, `Cargo.lock`) read TOML. `main` prints
one thing: the error that stopped a run, which exits 2 (undecided), not 1
(not met).

`check` is static: it reads READMEs and parses test files, and never builds or
runs a test. `cargo test` runs them, and the attribute ties the two together.
([D3](contract/decisions.toml))

## impish (the attribute)

A proc-macro crate with one export, `#[impish::contract("<primitive>/A<n>", …)]`.
It makes the function a test, or goes beside a test attribute already there
such as `#[test]` or `#[tokio::test]`, and checks each tag against
`contract/<primitive>/README.md` while the test compiles, so a wrong tag is a
compile error on that tag. It reports through `compile_error!`, never by
printing or panicking. ([D2](contract/decisions.toml), [D10](contract/decisions.toml))

## The Contract

impish holds its own Contract, in the layout it checks in other crates:

```
contract/README.md               the format
contract/<primitive>/README.md   purpose, obligations, covers and surface blocks
contract/coverage.json           published coverage, rewritten by `check --write`;
                                 `mutate --write` adds each mutate result
tests/contract/<primitive>.rs    the tests that prove the obligations
```

([D1](contract/decisions.toml),
[D5](contract/decisions.toml))

## Guardrails

Every rule here is checked by a machine (rule 19). Most fire on every PR
(rule 24); the `CI` job in `.github/workflows/ci.yml` gates them, and green
merges itself (rule 27).

| What | Where |
|---|---|
| Format, Clippy | CI `lint` |
| Tests, then `cargo impish check` on impish itself | CI `test` |
| The suite on macOS and Windows | CI `portability` |
| Architecture: the dependency directions and boundaries above, file size | `tests/architecture.rs` ([D8](contract/decisions.toml)) |
| Dependency licences, sources and advisories | CI `deny`, `deny.toml`; nightly `audit.yml` |
| Workflow security | CI `zizmor` |
| No unused dependency | CI `lint`, `cargo machete` |
| No copy-paste in scripts, workflows, Markdown or TOML | CI `lint`, jscpd (`.jscpd.json`) |
| Every changed line is mutated, and a survivor fails | CI `mutants` |
| Impish's own `mutate --since` holds each primitive a PR touches (rule 20) | CI `contract-mutants` |
| No new near-duplicate functions | CI `similar`, `scripts/similar-code.sh` |
| Existing near-duplicates become issues, one per cluster | nightly `similar-backlog.yml`, `scripts/similar-backlog.sh` |
| A change to what ships (`scripts/shipped.sh`) bumps the version | CI `version` ([D6](contract/decisions.toml)) |
| PR titles are `fix:` (with `bug`) or `refine:` (rule 9) | `pr-title.yml` |
| Issues start from obligations: Refinement and Bug forms, no blank issues (rules 1, 20, 28) | `.github/ISSUE_TEMPLATE/` ([D9](contract/decisions.toml)) |
| The gates' scripts work: each case fakes the slow tools and checks output and exit code; shellcheck; bash 3.2 | CI `lint`, `tests/pipeline/run.sh` |
| Tests that catch nothing another test doesn't | nightly `kill-matrix.yml`, `scripts/kill-matrix.sh` ([D7](contract/decisions.toml)) |

On `main`, the `Release` job publishes every new version.
