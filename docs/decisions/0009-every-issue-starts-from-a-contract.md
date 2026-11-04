# 0009 — Every issue starts from a Contract

**Status:** accepted

## Decision

Issues are opened through the forms in `.github/ISSUE_TEMPLATE/`, and blank
issues are off. A **Refinement** (`refine:`, label `refine`) states the intent,
the primitive and its obligations, in the same `### A<n> — …` / `must` /
`kind` / `status` blocks as `contract/<primitive>/README.md`. A **Bug**
(`fix:`, label `bug`) names the released version, and the obligation the
Contract was missing.

## Why

- [Rule 1](https://impatientprogramming.org/rules/#rule-1): before committing to
  work, write down how you'll know it worked.
- [Rule 20](https://impatientprogramming.org/rules/#rule-20): those criteria are
  the Contract's obligations, so the issue is written in the Contract's format
  and its blocks can be pasted into the README the work changes.
- [Rule 28](https://impatientprogramming.org/rules/#rule-28): a bug means the
  Contract was missing something, so a bug report proposes the missing
  obligation.
- [Rule 9](https://impatientprogramming.org/rules/#rule-9): `fix` is only for a
  defect in released code; the titles and labels match what `pr-title.yml`
  asks of the PR that closes the issue.

## Cost

The forms only bind the website: `gh issue create`, the API and agents can
still open a free-form issue. Personal repositories can't set GitHub's issue
type, so the labels carry it instead.
