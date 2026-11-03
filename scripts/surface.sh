#!/bin/sh
# Prints impish's surface for `check`: one `cli <command>` per subcommand and
# one `attribute impish::<name>` per attribute the impish crate exports.
set -eu
cargo run --quiet -- --help | awk '
  /^Commands:/ { on = 1; next }
  on && /^  [a-z]/ { if ($1 != "help") print "cli " $1; next }
  on { exit }
'
awk '
  /^#\[proc_macro_attribute\]/ { attr = 1; next }
  attr && match($0, /fn [a-z_]+/) { print "attribute impish::" substr($0, RSTART + 3, RLENGTH - 3); attr = 0 }
' impish/src/lib.rs
