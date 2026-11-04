#!/bin/sh
# Prints every repository file the published crates are built from, one path
# per line: what `cargo package` puts in each crate, so the list can't drift
# from what ships. CI's version job bumps the version on a change to any of it.
set -eu
cargo metadata --no-deps --format-version 1 |
  jq -r '.workspace_root as $root | .packages[] |
    "\(.name) \(.manifest_path | ltrimstr($root) | ltrimstr("/") | rtrimstr("Cargo.toml"))"' |
  while read -r name dir; do
    cargo package --quiet --list --allow-dirty --package "$name" |
      grep -v -x -e '.cargo_vcs_info.json' -e 'Cargo.toml.orig' |
      sed "s#^#$dir#"
  done |
  # A member's Cargo.lock is the workspace's.
  sed 's#^.*/Cargo\.lock$#Cargo.lock#' |
  # A readme from outside the crate's directory is listed under it; it ships
  # from where it really is, which another crate lists.
  while read -r path; do
    if [ -e "$path" ]; then echo "$path"; fi
  done |
  sort -u
