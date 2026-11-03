# mutate

`cargo impish mutate <primitive>` breaks the code a primitive's covers block
names, one mutant at a time, and runs the primitive's contract tests against
each. cargo-mutants supplies the mutants, so it must be installed. The tests
are the `contract` test target, `tests/contract/main.rs`, filtered to
`<primitive>::`. Each kill is credited to the obligations tagged on the tests
that failed, and the kill matrix shows, per obligation, how many mutants its
tests caught and how many only its tests caught.

## Obligations

### A1 — Kills are credited to obligations
- **must:** print each mutant with the obligations whose tests caught it, or `SURVIVED`, then each active obligation's killed and unique counts, and exit 0 when every active obligation catches a mutant
- **kind:** functional
- **status:** active

### A2 — A hollow obligation fails
- **must:** fail, naming each active obligation whose tests catch none of the mutants
- **kind:** functional
- **status:** active

### A3 — A red baseline fails
- **must:** fail before mutating, naming the contract tests that fail on the unmutated code
- **kind:** functional
- **status:** active

### A4 — Nothing to mutate fails
- **must:** fail when the primitive's README covers no code
- **kind:** functional
- **status:** active

### A5 — The project is never touched
- **must:** mutate a copy, leaving every file of the project as it was
- **kind:** functional
- **status:** active

```covers
src/mutate.rs::run
src/mutate.rs::report
src/mutate.rs::mutated
```

```surface
cli mutate
```
