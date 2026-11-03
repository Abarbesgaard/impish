# attribute

`#[impish::contract("<primitive>/A<n>", …)]` marks a contract test: the
obligations it proves. It comes from the `impish` crate, a dev-dependency of
the crate whose Contract it is, and takes the place of `#[test]`. Each tag is
checked against the crate's `contract/` as the test compiles, so a wrong tag
is a compile error.

## Obligations

### A1 — It is a test
- **must:** run a function marked `#[impish::contract(…)]` as a test under `cargo test`
- **kind:** functional
- **status:** active

### A2 — It names an obligation
- **must:** fail to compile, saying what it takes, when the attribute names no obligation or something other than string literals
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

```covers
impish/src/lib.rs::contract
```

```surface
attribute impish::contract
```
