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
([D11](../decisions.toml)). Every contract test
must catch at least one mutant too.

## Obligations

### A1 — Kills are credited to obligations
- **must:** print each mutant with the active obligations whose tests caught it, or, when none did, what kind of uncaught mutant it is (A22), then each active obligation's killed and unique counts
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

### A15 — Witnesses outside the Contract are named
- **must:** with `--whole-suite`, also run every test target outside the contract target against each mutant the contract tests build and finish, and name each outside test that catches a mutant no contract test catches, as a witness of a missing obligation; in `--format json`, list the outside tests that catch each mutant
- **why:** a test the Contract doesn't hold catching what the Contract misses is evidence of an obligation nobody wrote down (rule 28, #33)
- **kind:** functional
- **status:** active

### A16 — Duplicates outside the Contract fail
- **must:** with `--whole-suite` and no primitive or `--since`, fail, naming each outside test that catches at least one mutant and whose every catch the contract tests catch too; with a primitive or `--since`, say that duplicates need a run of everything instead
- **why:** the Contract test is the spec, so a copy of its catches outside the Contract costs CPU and proves nothing new (rule 26, #33); an outside test may guard another primitive's code, so only a run of every primitive can call it a duplicate
- **kind:** functional
- **status:** active

### A17 — Mutants are tried at once
- **must:** with `-j <n>`, try up to n mutants at once, each in its own copy of the project with its own target directory, and report the same outcomes in the same order as one at a time; with no `-j`, use half the cores, at least one
- **why:** each mutant is independent, so trying them one by one leaves most cores idle (#70)
- **kind:** non-functional
- **status:** active

### A18 — The kill matrix has a row per test
- **must:** print, after the obligation rows, each contract test of the primitive with its killed and unique counts, where unique counts the mutants no other contract test caught; in `--format json`, list them under `tests`
- **kind:** functional
- **status:** active

### A19 — Tests with no unique kill are named
- **must:** when every mutant was tried, print each contract test with kills but no unique kill under `Mutation-blind tests`, as a warning that doesn't change the exit code
- **why:** a test whose every catch another test makes too costs CPU and proves nothing new (rule 26), but a contract test is also spec and may be kept on purpose, so a person decides (#59)
- **kind:** functional
- **status:** active

### A20 — A reason keeps the warning quiet
- **must:** accept `#[impish::contract("<primitive>/A<n>", blind = "<reason>")]`, and print that test with its reason under `Kept with no unique kill` in place of the warning
- **kind:** functional
- **status:** active

### A21 — --only-lines holds only the changed lines
- **must:** with `--since <ref> --only-lines`, mutate only the covered lines changed since the merge base, even of a primitive whose `contract/<primitive>/` or `tests/contract/<primitive>.rs` changed; refuse `--only-lines` without `--since`
- **why:** re-mutating all of `mutate` on a PR that changes its tests takes hours, so a PR checks the lines it changed and main re-mutates all of it after the merge (#89)
- **kind:** functional
- **status:** active

### A22 — Unreached is told from lived
- **must:** report each mutant no contract test catches as `UNREACHED` when no contract test of the primitive, run alone on the unmutated code, enters the innermost block holding it; `timed out` when the tests ran past the timeout; and `LIVED` otherwise, naming the contract tests that ran it; in `--format json`, give its outcome as `unreached`, `timed out` or `lived`, and the tests that ran it under `ran_by`
- **why:** a lived mutant needs a stronger test, an unreached one a promise that needs the code, or no code; one `SURVIVED` for both hides which (#58)
- **kind:** functional
- **status:** active

### A23 — Every uncaught mutant says how to clear it
- **must:** print under each uncaught mutant mutants.md doesn't declare one line starting `to clear:`: for a lived mutant, the tests to make fail on it, or the obligation to write; for an unreached one, write the promise that needs the code, delete the code, or declare it unpromised
- **kind:** functional
- **status:** active

### A24 — The summary counts each kind
- **must:** print one summary line with the counts of killed, lived, unreached, timed out, declared and unviable mutants, where declared counts the uncaught mutants mutants.md declares and the other counts leave them out; in `--format json`, give the same counts
- **kind:** functional
- **status:** active

### A25 — Unreached mutants aren't built
- **must:** without `--whole-suite`, skip building and testing an unreached mutant, and still report it unreached; with it, try it, since a test outside the Contract may reach it
- **why:** no contract test can catch a mutant none of them runs, so building it only costs time
- **kind:** non-functional
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
src/mutate/judge.rs::matrix
src/mutate/judge.rs::uncaught
src/mutate/judge.rs::line
src/mutate/reach.rs::ran
src/mutate/reach.rs::reach
src/mutate/reach.rs::run
src/mutate/reach.rs::probe
src/mutate/reach.rs::visit_block
src/mutate/reach.rs::visit_signature
src/mutate/reach.rs::visit_item_fn
src/mutate/reach.rs::visit_impl_item_fn
src/mutate/reach.rs::visit_item_const
src/mutate/reach.rs::visit_item_static
src/mutate/reach.rs::visit_expr_const
src/mutate/scratch.rs::offset
src/mutate/record.rs::record
src/mutate/suite.rs::tally
src/mutate/pool.rs::each
src/mutate/suite.rs::merge
src/mutate/suite.rs::duplicates
src/mutate/suite.rs::duplicate
src/mutate/suite.rs::witnessed
src/mutate/suite.rs::annotate
src/contract.rs::fingerprint
src/mutate/judge.rs::judge
src/mutate/judge.rs::json
src/mutate/judge.rs::key
src/contract.rs::declarations
src/mutate/scratch.rs::mutated
```

```surface
cli mutate
```
