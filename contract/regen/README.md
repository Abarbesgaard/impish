# regen

A pack to rebuild a primitive from its Contract alone (Impatient Programming,
rule 17: keep it regenerable). `cargo impish regen <primitive>` writes the
intents, the obligations, the decision records, the contract tests and stubs
of the covered functions, with none of their code. A fresh agent rebuilds the
primitive from the pack, and every guess it has to make is a sentence the
Contract was missing. After covenant's `mix covenant.regen`. Running the agent
is out of scope: impish writes the pack, and a person or a workflow hands it
over.

## Obligations

### A1 — A primitive may state its intent
- **must:** read `contract/intent.md` and `contract/<primitive>/intent.md` when present, and fail `check`, naming the file, when an intent lacks a `## Purpose` or `## Public surface` section
- **kind:** persisted-format
- **status:** active

### A2 — regen writes a pack
- **must:** `cargo impish regen <primitive>` writes to `target/impish/regen/<primitive>/`, or `--out <dir>`, a `PROMPT.md` telling an agent to rebuild the primitive from the pack alone, the Contract format as `FORMAT.md`, the project's and the primitive's `intent.md` and `decisions.toml` when present, the primitive's README, `tests/contract/<primitive>.rs`, and under `stubs/<file>` each covered function's doc comments, attributes and signature with a `todo!()` body, a method inside its `impl` header
- **kind:** functional
- **status:** active

### A3 — A pack holds no implementation
- **must:** write no body of a covered function, no nested function, and nothing else from the covered files into the pack
- **why:** the point of regeneration is to rebuild without seeing the old code
- **kind:** security
- **status:** active

### A4 — regen only deletes what it wrote
- **must:** list in the pack's `.impish-pack` every file it wrote; on a later run delete only those, and refuse, exiting 2 and deleting nothing, a non-empty directory without the marker or one holding a file the marker doesn't list
- **kind:** security
- **status:** active

### A5 — An unknown primitive is undecided
- **must:** exit 2, writing nothing, for a primitive with no `contract/<primitive>/README.md`
- **kind:** functional
- **status:** active

```covers
src/contract.rs::intent
src/regen.rs::run
src/regen.rs::pack
src/regen.rs::clear
src/regen.rs::files
src/regen/stubs.rs::stubs
src/regen/stubs.rs::offset
src/regen/stubs.rs::head
src/regen/stubs.rs::visit_item_fn
src/regen/stubs.rs::visit_item_impl
src/regen/stubs.rs::visit_impl_item_fn
```

```surface
cli regen
```
