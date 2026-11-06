# covers

Which code each primitive is about. A README's `covers` lines name a file
(`src/lib.rs`), a tree (`src/check/...`), a function or type
(`src/lib.rs::parse`, `src/lib.rs::Parser`) or a method
(`src/lib.rs::Parser::next`). Each line resolves to functions, so a primitive
owns code, not files. When several lines claim one function, the most
specific wins; a tie goes to the primitive whose name sorts first. After
covenant's and flinch's covers.

## Obligations

### A1 — Covers lines name real code
- **must:** fail `check`, naming the README and the line, on a covers line that isn't `<file>`, `<dir>/...`, `<file>::<item>` or `<file>::<Type>::<method>`, or that names no function: a missing file or directory, or an item, type or method the file doesn't define
- **kind:** functional
- **status:** active

### A2 — The most specific claim owns a function
- **must:** give each function to the primitive whose covers line names it most specifically, a method before a function or type, before a file, before a tree, and on a tie to the primitive whose name sorts first
- **why:** a file can hold two primitives' code without either owning the other's
- **kind:** functional
- **status:** active

### A3 — Code outside the Contract is listed
- **must:** with `check --outside`, list as `<file>::<function>` the functions in `src/` that no covers line claims, leaving out `#[cfg(test)]` code and files marked `@generated`, and never fail on them
- **why:** uncovered code is a fact to see, not an error
- **kind:** functional
- **status:** active

### A4 — The covered share is reported
- **must:** print `code: <covered> of <total> functions in the Contract (<percent>%)` after the summary, and record `covered`, `total` and `percent` under `code` in `contract/coverage.json`; with no functions in `src/`, print and record nothing about code
- **kind:** persisted-format
- **status:** active

### A5 — Mutate breaks what a primitive owns
- **must:** have `cargo impish mutate <primitive>` break exactly the functions the primitive owns, nested functions included, and none another primitive owns in the same file
- **kind:** functional
- **status:** active

```covers
src/code.rs
src/check.rs::percent
```
