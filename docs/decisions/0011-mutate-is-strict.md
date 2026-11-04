# 0011 — mutate is strict: every mutant is caught or declared

**Status:** accepted (#25); supersedes the per-obligation grain of
[0007](0007-tests-earn-their-place-at-three-grains.md)

## Decision

`cargo impish mutate <primitive>` fails on any mutant of the covered code that
no contract test catches, whether it survives or times out. A hollow
obligation still fails too, and so does a blind contract test, one that
catches no mutant (#28). The only way past a mutant is to declare it in
`contract/<primitive>/mutants.md`, under a `##` heading that gives the reason:

- `equivalent`: no program could tell the mutant from the original.
- `unpromised`: behaviour changes, and the Contract leaves it open on purpose.

A declaration names a mutant as cargo-mutants does, without the line and
column (`src/lib.rs: replace * with + in double`), so it survives edits above
it. It covers every mutant with that name. Declared mutants are still run: a
declaration a contract test contradicts fails, and so does one that matches
no mutant.

Strict is the default, with no switch to turn it off.

## Why

Under 0007, mutate passed once every obligation caught *one* mutant. Sara's
done/A1 passed with 12 survivors in code its covers block claims. A survivor
in covered code is code the Contract claims but doesn't specify (Impatient
Programming rules 20 and 26). flinch, impish's Go sibling, already blocks on
"any break the Contract's tests sleep through".

## Cost

Every covered function's mutants must be killed or explained in writing, so
covers blocks get narrower and contract tests sharper. A timeout is no longer
a pass, so a slow test suite can fail a run; declaring the mutant is the way
out when the loop it causes can't be caught. A declaration covers every
mutant with its name, so two identical changes in one function are declared
or caught together.
