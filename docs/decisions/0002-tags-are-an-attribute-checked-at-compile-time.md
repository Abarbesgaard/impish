# 0002 — Tags are an attribute, checked as the test compiles

**Status:** accepted (#12, #13, #14); supersedes `// Contract:` comments

## Decision

A contract test is a function marked `#[impish::contract("<primitive>/A<n>", …)]`
instead of, or beside, `#[test]` ([0010](0010-the-attribute-goes-beside-the-test-attribute.md)). The attribute checks each tag against the primitive's
README while the test compiles, and includes the README in the build so a
change to it rechecks the tags. `check` finds tests by parsing test files with
`syn`, not by scanning lines.

Tags must name a plain primitive (letters, digits, `-`, `_`), since a tag is a
path into `contract/` and the attribute runs inside the compiler.

## Why

Comment tags were only checked when `check` ran, so a typo or a retired
obligation went unnoticed while the test was written. Line scanning also needed
workarounds, for example for test-looking text inside strings.

## Cost

Users add `impish` as a dev-dependency, and the attribute reads files at
compile time. A plain `#[test]` in `tests/contract/` names no obligation and
fails `check`.
