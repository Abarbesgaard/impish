# impish

## Purpose

The Contract gate for Rust, after Impatient Programming: a project's promises
are written down as numbered obligations under `contract/`, each proved by a
contract test, and CI fails when a promise has no test, a test proves no
promise, or a test catches none of the bugs its promise is about.

## Public surface

- `cargo impish check`, `mutate`, `conditions` and `regen`: the gate, run by
  people and CI.
- The `impish` crate's `#[impish::contract("<primitive>/A<n>")]` attribute,
  which tags a contract test with the obligations it proves.
- The Contract's files: `contract/<primitive>/README.md`, `mutants.md`,
  `decisions.toml`, `intent.md`, and the published `contract/coverage.json`.
