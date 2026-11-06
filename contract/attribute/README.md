# attribute

`#[impish::contract("<primitive>/A<n>", …)]` marks a contract test: the
obligations it proves. It comes from the `impish` crate, a dev-dependency of
the crate whose Contract it is. It makes the function a test, or goes beside
the test attribute already there, such as `#[test]` or `#[tokio::test]`. Each tag is
checked against the crate's `contract/` as the test compiles, so a wrong tag
is a compile error.

## Obligations

### A1 — It is a test
- **must:** run a function marked `#[impish::contract(…)]` as a test under `cargo test`
- **kind:** functional
- **status:** active

### A2 — It names an obligation
- **must:** fail to compile, saying what it takes, when the attribute names no obligation, or something other than string literals and one `blind = "<reason>"`
- **kind:** functional
- **status:** active

### A3 — Tags are checked as the test compiles
- **must:** fail to compile, naming the tag, when a tag isn't `<primitive>/A<n>`, or `contract/<primitive>/README.md` is missing, isn't a valid Contract, doesn't declare the obligation or declares it retired
- **kind:** functional
- **status:** active

### A4 — A changed README is checked again
- **must:** check a test's tags again when its primitive's README changes
- **kind:** functional
- **status:** active

### A5 — Tags stay inside contract/
- **must:** fail to compile, naming the tag, when a tag's primitive isn't made of letters, digits, `-` and `_`, without reading any file for it
- **why:** the tag is a path into `contract/`, and the attribute runs inside the compiler
- **kind:** security
- **status:** active

### A6 — It keeps the test attribute already there
- **must:** run a function once as a test when it also carries a test attribute, `#[test]` or one whose path ends in `test` such as `#[tokio::test]`, above or below `#[impish::contract(…)]`
- **why:** adopting impish in existing tests should take one added line, not a rewrite of each test's attributes
- **kind:** functional
- **status:** active

```covers
impish/src/lib.rs::contract
impish/src/lib.rs::is_test
impish/src/lib.rs::has_test
impish/src/lib.rs::tested_in_source
```

```surface
attribute impish::contract
```
