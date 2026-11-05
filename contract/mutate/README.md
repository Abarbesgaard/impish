# mutate

`cargo impish mutate <primitive>` breaks the code a primitive's covers block
names, one mutant at a time, and runs the primitive's contract tests against
each. cargo-mutants supplies the mutants, so it must be installed. The tests
are the `contract` test target, `tests/contract/main.rs`, filtered to
`<primitive>::`. Each kill is credited to the obligations tagged on the tests
that failed, and the kill matrix shows, per obligation, how many mutants its
tests caught and how many only its tests caught.

Every mutant must be caught. One that no contract test catches, whether it
survives or times out, fails the run unless `contract/<primitive>/mutants.md`
declares it: `equivalent` when no program could tell it from the original,
`unpromised` when its behaviour changes and the Contract leaves that open
([0011](../../docs/decisions/0011-mutate-is-strict.md)). Every contract test
must catch at least one mutant too.

## Obligations

### A1 — Kills are credited to obligations
- **must:** print each mutant with the active obligations whose tests caught it, or `SURVIVED` when none did, then each active obligation's killed and unique counts
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

### A6 — The kill matrix says what unique means
- **must:** print, under the kill matrix, that unique counts the mutants caught by no other obligation's tests, and that tests outside the Contract weren't run
- **why:** read as "caught by no other test in the suite", unique hides the duplicates Rule 26 asks us to remove (#26)
- **kind:** functional
- **status:** active

### A7 — Each run tests the code as it is
- **must:** build the project as it is now for the baseline and every mutant, never a build left by an earlier run's mutant
- **why:** a stale mutant build makes a run red, or quietly wrong (#38)
- **kind:** functional
- **status:** active

### A8 — Every mutant is caught or declared
- **must:** fail, naming each mutant of the covered code that no contract test catches, whether it survives or times out, unless `contract/<primitive>/mutants.md` declares it
- **why:** a survivor is code the Contract claims but doesn't specify (rules 20, 26; #25)
- **kind:** functional
- **status:** active

### A9 — Declarations are checked
- **must:** fail, naming each declared mutant a contract test catches, each declaration that matches no mutant, and each declaration block with no `##` heading giving its reason; and stop, undecided, when `mutants.md` exists but can't be read
- **why:** a stale or contradicted declaration hides a mutant the Contract does catch, or a reason nobody wrote down
- **kind:** functional
- **status:** active

### A10 — A blind test fails
- **must:** fail, naming each contract test of the primitive that catches none of the mutants, even when its obligations are caught by other tests
- **why:** a test that passes whatever the code does proves nothing (rule 26; #28)
- **kind:** functional
- **status:** active

### A11 — --format json reports as JSON
- **must:** with `--format json`, print one JSON document on stdout, with `version` 1, whether the Contract is `met`, every mutant's name, outcome, the obligations and tests that caught it and any declaration, the counts per outcome, the kill matrix, and every problem the text names; write nothing to stderr, and exit as the text does
- **why:** CI reads results, not prose (#30)
- **kind:** persisted-format
- **status:** active

### A12 — --write records whether mutate is met
- **must:** with `--write`, record under `primitives.<primitive>.mutate` in `contract/coverage.json` whether the primitive's mutate is met, and a fingerprint of its README, its mutants.md, its contract tests and the files its covers block names; leave the rest of the file as it was
- **why:** published coverage says which obligations have tests, not whether they bite (rule 29; #31)
- **kind:** persisted-format
- **status:** active

### A13 — With no primitive, mutate them all
- **must:** with no primitive named, mutate every primitive that covers code, one after another, skip the ones that cover none, and fail if any one's Contract isn't met; with `--format json`, print one document holding each primitive's document
- **why:** CI shouldn't have to list the primitives (#32)
- **kind:** functional
- **status:** active

### A14 — --since mutates what changed
- **must:** with `--since <ref>`, mutate only the covered lines changed since the merge base with `<ref>`, uncommitted work included, and every covered line of a primitive whose `contract/<primitive>/` or `tests/contract/<primitive>.rs` changed; on such a partial run, never call an obligation hollow, a test blind or a declaration unmatched; refuse `--write`
- **why:** a PR re-mutating everything a primitive covers is slow, and holds unchanged code the PR didn't touch (#32)
- **kind:** functional
- **status:** active

```covers
src/mutate.rs::run
src/mutate.rs::primitives
src/mutate.rs::mutate
src/mutate/since.rs::since
src/mutate/since.rs::touch_contract
src/mutate/since.rs::touch
src/mutate/since.rs::hunk
src/mutate.rs::finish
src/mutate.rs::record
src/contract.rs::fingerprint
src/mutate/judge.rs::judge
src/mutate/judge.rs::json
src/mutate.rs::key
src/contract.rs::declarations
src/mutate/scratch.rs::mutated
```

```surface
cli mutate
```
