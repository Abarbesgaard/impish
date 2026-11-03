# 0001 — The Contract is Markdown blocks

**Status:** accepted (#3, #6)

## Decision

A Contract lives in `contract/<primitive>/README.md`, the layout of impish's
siblings flinch (Go) and covenant (Elixir). Obligations are blocks under
`## Obligations`: `### A<n> — <title>` and the fields `must`, `why`, `kind`
and `status`. Numbers are never reused, and a retired obligation keeps its
number.

The parser is total: every line under `## Obligations` must belong to a block,
and an obligation outside the section fails.

## Why

The Contract is read by people first, so it is prose a reviewer can read in a
PR. One-line obligations were tried first; a typo like `* **A3**` was skipped
silently, so a promise could fall out of the Contract without an error. A total
parser can't lose one.

## Cost

The format is strict, and a README that reads fine to a person can fail
`check`. Every failure names the file and line.
