# 0007 — Tests earn their place, per line, per obligation and per test

**Status:** accepted (#16 and the PR that added this record); the per-obligation grain is superseded by [0011](0011-mutate-is-strict.md)

## Decision

Mutation testing runs at three grains:

- **Per changed line:** CI mutates every line a PR changes; a mutant no test
  catches fails the PR.
- **Per obligation:** `cargo impish mutate <primitive>` credits every kill to
  the obligations whose tests caught it, and fails on an obligation that
  catches nothing.
- **Per test:** the nightly kill matrix (`scripts/kill-matrix.sh`) runs every
  mutant against the whole suite, and files an issue for each test whose kills
  other tests already make. Closing the issue keeps the test.

## Why

Impatient Programming rule 26: a test is kept only if it catches something no
other test catches. A green obligation can be hollow, and agents write
redundant tests cheaply; each grain catches what the others can't.

## Cost

The full per-test sweep takes hours, so it runs one shard a night and only
suggests; contract tests are spec and may be kept on purpose.
