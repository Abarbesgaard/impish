#!/bin/sh
# Prints impish's surface for `check`: one `cli <command>` per subcommand.
set -eu
cargo run --quiet -- --help | awk '
  /^Commands:/ { on = 1; next }
  on && /^  [a-z]/ { if ($1 != "help") print "cli " $1; next }
  on { exit }
'
