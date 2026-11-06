# exceptions

A deliberate gap, said in the Contract next to the obligations: code proven
somewhere other than the contract tests, such as an integration test against
a real service or a property test. An obligation of kind `exception` names the
`target` it excuses, needs no test, is counted apart from the obligations, and
fails `check` once its target is gone, so it can't outlive the code it
excuses. After covenant's `kind: exception`. `mutate` treats an exception's
target like any other code: a mutant there no contract test catches is still
declared in `mutants.md`, the one place mutants are declared.

## Obligations

### A1 — An exception names its target
- **must:** accept `kind: exception` with a `target:` of `<file>::<function>` or `<file>`, and fail `check`, naming the file and line, on an exception with no `target` or a `target` on an obligation that isn't an exception
- **kind:** persisted-format
- **status:** active

### A2 — An exception needs no test
- **must:** never report an exception as having no test, and fail `check`, naming the test, and fail the test's compilation, naming the tag, when a contract test names one
- **why:** an exception says no contract test proves this; a test that claims to is one of the two lying
- **kind:** functional
- **status:** active

### A3 — An exception dies with its target
- **must:** fail `check`, naming the exception and its target, when the target's file or function no longer exists, and, without reading it, when the target's file is outside the crate
- **why:** an excuse that outlives its code hides the next gap behind it
- **kind:** functional
- **status:** active

### A4 — Exceptions are published apart from coverage
- **must:** leave active exceptions out of the obligations and kinds `check` counts and `coverage.json` records, and list each under `exceptions` in `coverage.json` with its target; with none, record no `exceptions`
- **kind:** persisted-format
- **status:** active

```covers
src/check/exceptions.rs::excused
core/src/lib.rs::needs_test
core/src/lib.rs::target_problem
```
