# impish

`#[impish::contract]`, the attribute that marks a contract test for
[cargo-impish](https://crates.io/crates/cargo-impish), the Contract gate for
Rust.

```toml
[dev-dependencies]
impish = "0.6"
```

```rust
#[impish::contract("add/A1", "add/A2")]
fn adds() {
    assert_eq!(calc::add(2, 3), 5);
}
```

The attribute takes the place of `#[test]`. As the test compiles, it checks
each tag against `contract/<primitive>/README.md` in the crate, so a typo, a
missing obligation or a retired one is a compile error.

See the [impish README](https://github.com/Abarbesgaard/impish#readme) for the
Contract format and the `cargo impish` commands.

## License

MIT
