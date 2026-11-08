# 0013 — check writes the GitHub job summary

**Status:** accepted (#113); amends [0003](0003-check-is-static.md)

## Decision

When `GITHUB_STEP_SUMMARY` names a file, `cargo impish check` appends a
Markdown summary of the run to it, and when `GITHUB_ACTIONS` is `true` it
prints an `::error` annotation for each problem that names a line. That file
is the only one it writes; it still changes nothing in the project, and its
output and exit code are the same either way. A summary it can't write is
undecided (exit 2).

## Why

On a CI run the result should be the first thing a reviewer sees, not a line
in a log. GitHub hands every step the summary file, so writing it needs no
flag, script or `jq` from the user, and no output format to keep stable.

## Cost

`check` is no longer read-only in the strictest sense, and it knows about one
CI system. Other systems get the plain output.
