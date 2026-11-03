# attribute

`#[impish::contract("<primitive>/A<n>", …)]` marks a contract test: the
obligations it proves. It comes from the `impish` crate, a dev-dependency of
the crate whose Contract it is, and takes the place of `#[test]`.

## Obligations

### A1 — It is a test
- **must:** run a function marked `#[impish::contract(…)]` as a test under `cargo test`
- **kind:** functional
- **status:** active

### A2 — It names an obligation
- **must:** fail to compile, saying what it takes, when the attribute names no obligation or something other than string literals
- **kind:** functional
- **status:** active

```covers
impish/src/lib.rs::contract
```

```surface
attribute impish::contract
```
