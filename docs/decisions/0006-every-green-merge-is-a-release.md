# 0006 — Every green merge is a release

**Status:** accepted (#10, #11); amended when the README was found not to count

## Decision

A PR that changes what ships must bump the workspace version; CI's `version`
job fails otherwise. What ships is what `cargo package` puts in the crates, as
listed by `scripts/shipped.sh`: each crate's sources, README, LICENSE and
manifest, and the lockfile. `cargo-impish` lists its files in `include`, so
the repository's tests, docs and workflows don't ship. On `main`, the
`Release` job tags and publishes any version without a GitHub release. Dependabot is exempt and ships with the next release.

## Why

Impatient Programming rule 3. Users install by tag, so every merged change is
installable, and a release is never a separate step someone can forget.

A hand-kept list of paths drifted: the README ships on crates.io but wasn't on
it, so #20 merged green and never reached crates.io. Asking cargo what it
packs can't drift.

## Cost

Version numbers move fast. The bump is part of every PR that ships.
