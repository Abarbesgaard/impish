# 0006 — Every green merge is a release

**Status:** accepted (#10, #11)

## Decision

A PR that changes what ships (`src/`, `core/`, `impish/`, `Cargo.toml` or
`Cargo.lock`) must bump the workspace version; CI's `version` job fails
otherwise. On `main`, the `Release` job tags and publishes any version without
a GitHub release. Dependabot is exempt and ships with the next release.

## Why

Impatient Programming rule 3. Users install by tag, so every merged change is
installable, and a release is never a separate step someone can forget.

## Cost

Version numbers move fast. The bump is part of every PR that ships.
