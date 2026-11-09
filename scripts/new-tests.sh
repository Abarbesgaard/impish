#!/usr/bin/env bash
# Does each test a PR adds catch a bug no other test catches (rule 26)? After
# Sara's.
#
#   scripts/new-tests.sh origin/main
#
# For every test function added since the base under tests/, it mutates the
# Rust files the PR changes in src/, core/src/ and impish/src/, and runs
# scripts/kill-matrix.sh --test on it. It fails when a test catches nothing,
# or nothing another test doesn't, and names the tests that share its kills.
# A Contract test is the spec, so for one those other tests are the
# duplicates: fold them into the Contract or delete them in the same PR.
# Architecture tests check structure, not behaviour, so they are skipped. It
# also fails when the PR adds more than NEW_TESTS_MAX tests (default 3): split
# the PR. NEW_TESTS_DRY=1 only lists each new test and the files it would break.
set -euo pipefail

base="${1:?give the base, e.g. origin/main}"
max="${NEW_TESTS_MAX:-3}"
dir="$(dirname "$0")"
gha="${GITHUB_ACTIONS:-}"

# "<fn>" for each test function in a Rust file on stdin: a fn after #[test]
# or #[impish::contract(...)].
test_fns() {
  awk '
    /^[[:space:]]*#\[(test|impish::contract)/ { tagged = 1; next }
    tagged && /^[[:space:]]*#\[/ { next }
    tagged && match($0, /fn [A-Za-z0-9_]+/) { print substr($0, RSTART + 3, RLENGTH - 3) }
    { tagged = 0 }
  '
}

changed_src=$(git diff --name-only --diff-filter=AM "${base}...HEAD" -- \
  'src/*.rs' 'core/src/*.rs' 'impish/src/*.rs' | tr '\n' ' ')

# "<test id>" for each added test: tests/<target>.rs is <target>/<fn>, and
# tests/<target>/<mod>.rs is <target>/<mod>::<fn>.
added=$(git diff --name-only --diff-filter=AM "${base}...HEAD" -- 'tests/*.rs' | while read -r file; do
  comm -13 <(git show "$(git merge-base "$base" HEAD):$file" 2>/dev/null | test_fns | sort -u) \
           <(test_fns < "$file" | sort -u) | while read -r fn; do
    case "$file" in
      (tests/*/*.rs)
        target="${file#tests/}"; target="${target%%/*}"
        mod="${file#tests/"$target"/}"; mod="${mod%.rs}"; mod="${mod%/mod}"; mod="${mod//\//::}"
        [ "$mod" = main ] && mod="" || mod="$mod::"
        printf '%s/%s%s\n' "$target" "$mod" "$fn" ;;
      (tests/*.rs)
        printf '%s/%s\n' "$(basename "$file" .rs)" "$fn" ;;
    esac
  done
done)

if [ -z "$added" ]; then
  echo "No tests added since ${base}."
  exit 0
fi

if [ -n "${NEW_TESTS_DRY:-}" ]; then
  printf '%s\n' "$added" | while IFS= read -r id; do printf '%s\t%s\n' "$id" "$changed_src"; done
  exit 0
fi

status=0
fail() {
  status=1
  if [ -n "$gha" ]; then echo "::error title=New test::$1"; else echo "error: $1"; fi
}

sharers() { printf '%s\n' "$1" | sed -n 's/^Shared with: //p'; }

checked=0
while IFS= read -r id; do
  case "$id" in
    architecture/*) echo "Skipping $id: an architecture test checks structure, not behaviour."; continue ;;
  esac
  if [ "$checked" -ge "$max" ]; then
    fail "$id was not checked: the PR adds more than $max tests (NEW_TESTS_MAX). Split it."
    continue
  fi
  if [ -z "${changed_src// /}" ]; then
    echo "Skipping $id: the PR changes no Rust under src/, core/src/ or impish/src/, so there is no code to break."
    continue
  fi
  checked=$((checked + 1))
  [ -z "$gha" ] || echo "::group::$id against $changed_src"
  # shellcheck disable=SC2086 # changed_src is a list of paths
  verdict=$("$dir/kill-matrix.sh" --test "$id" $changed_src 2>&1 | tee /dev/stderr | tail -n 30)
  [ -z "$gha" ] || echo "::endgroup::"
  case "$verdict" in
    *"kills nothing"*) fail "$id catches no mutant in $changed_src: it would pass whatever that code did." ;;
    *"is a Contract test"*) fail "$id is a Contract test and every bug it catches is also caught by: $(sharers "$verdict"). They duplicate the Contract: fold them into it or delete them." ;;
    *"Every kill is shared"*) fail "$id catches nothing these tests don't already catch: $(sharers "$verdict"). Delete it, or make it catch something new." ;;
    *"Keep "*) echo "$id catches a bug no other test catches." ;;
    *) fail "$id: the kill matrix gave no verdict; see the log." ;;
  esac
done <<< "$added"
exit "$status"
