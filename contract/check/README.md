# check

`cargo impish check` holds the Contract under `contract/` to its tests. A
primitive is a directory `contract/<p>/` with a `README.md`; its tests are in
`tests/contract/<p>.rs`. Problems go to stderr, one per line, and the exit code
is 1; when the Contract holds the exit code is 0.

## Obligations

- **A1** When every obligation is proved, `check` says how many obligations and primitives it checked, and exits 0.
- **A2** An obligation no test names fails, naming the obligation.
- **A3** A test that names no obligation fails, naming the test.
- **A4** A tag naming an obligation its primitive doesn't declare fails, naming the tag.
- **A5** A malformed obligation line, a number declared twice, or a README with no obligations fails, naming the file and line.
- **A6** A primitive missing its README or its test file fails, naming what's missing.
- **A7** `check` uses the nearest directory, from the current one up, that has `contract/`; with none, it fails.
- **A8** Every problem is reported, not only the first.

```covers
src/check.rs::run
src/check.rs::check
src/contract.rs::obligations
src/contract.rs::tagged_tests
```
