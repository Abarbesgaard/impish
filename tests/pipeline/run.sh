#!/usr/bin/env bash
# Tests for the scripts and workflows that make up impish's CI pipeline. A gate
# that can't fail passes every PR it should stop (Impatient Programming, rule
# 24), so each case builds a throwaway git repo, fakes the slow tools
# (similarity-rs, the kill matrix's cargo-mutants runs) and checks a script's
# output and exit code. After Sara's tests/pipeline.
#
#   tests/pipeline/run.sh            # every case
#   tests/pipeline/run.sh similar    # cases whose name contains "similar"
#
# BASH_UNDER_TEST (default bash) runs the scripts, so macOS can check them
# with its bash 3.2: BASH_UNDER_TEST=/bin/bash tests/pipeline/run.sh
# SHELLCHECK=0 skips shellcheck where it isn't installed.
# JSCPD=0 skips the case that runs jscpd through npx, where npx isn't installed.
set -uo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
sh="${BASH_UNDER_TEST:-bash}"
only="${1:-}"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
export GITHUB_ACTIONS=''

ok() { echo "PASS  $case"; }
bad() { echo "FAIL  $case: $1"; [ -z "${2:-}" ] || printf '%s\n' "$2" | sed 's/^/      /'; return 1; }
expect_exit() { [ "$1" -eq "$2" ] || bad "exit $1, want $2" "$3"; }
expect_has() { printf '%s' "$1" | grep -qF -- "$2" || bad "output lacks: $2" "$1"; }
expect_lacks() { ! printf '%s' "$1" | grep -qF -- "$2" || bad "output has: $2" "$1"; }

# A fresh repo at $work/<name> with impish's layout and one commit on main.
repo() {
  local d="$work/$1"
  rm -rf "$d"; mkdir -p "$d/scripts" "$d/src" "$d/core/src" "$d/impish/src" "$d/tests/contract" "$d/contract"
  cd "$d" || exit 1
  git init -q -b main . && git config user.email t@t && git config user.name t
  echo 'pub fn a() {}' > src/lib.rs
  git add -A && git commit -qm base
}
commit() { git add -A && git commit -qm "$1"; }

# ── workflows ──────────────────────────────────────────────────────────────

# Without pipefail, `script | tee` takes tee's exit code and a failing gate
# passes; `shell: bash` turns pipefail on.
workflows_run_with_pipefail() {
  local f missing=""
  for f in "$root"/.github/workflows/*.yml; do
    awk '/^defaults:/ { d = 1; next } d && /^[^ #]/ { d = 0 } d && /^    shell: bash *$/ { found = 1 } END { exit !found }' "$f" ||
      missing="$missing $(basename "$f")"
  done
  [ -z "$missing" ] || { bad "no top-level defaults.run.shell: bash in:$missing"; return; }
  ok
}

# A mutation run takes minutes; a format slip shouldn't wait for one to be found.
workflows_mutate_after_the_cheap_checks() {
  local needs want missing=""
  needs=$(awk '$0 == "  contract-mutants:" { m = 1; next } m && /^  [a-z]/ { m = 0 } m && /^    needs:/ { print; exit }' "$root/.github/workflows/ci.yml")
  for want in lint deny zizmor test similar version; do
    case "$needs" in *"$want"*) ;; *) missing="$missing contract-mutants:$want" ;; esac
  done
  [ -z "$missing" ] || { bad "a mutation job doesn't need:$missing"; return; }
  ok
}

# Skipped jobs count as passed, so a job the gate doesn't need can fail and
# still let the PR merge.
workflows_gate_needs_every_job() {
  local f="$root/.github/workflows/ci.yml" jobs needs job missing=""
  jobs=$(awk '/^jobs:/ { j = 1; next } j && /^  [a-z_-]+:/ { sub(/^  /, ""); sub(/:.*/, ""); print }' "$f")
  needs=$(awk '/^  ci:/ { c = 1; next } c && /^  [a-z]/ { c = 0 } c && /^    needs:/ { print; exit }' "$f")
  for job in $jobs; do
    case "$job" in ci|release) continue ;; esac
    case "$needs" in *"$job"*) ;; *) missing="$missing $job" ;; esac
  done
  [ -z "$missing" ] || { bad "the CI gate doesn't need:$missing"; return; }
  ok
}

workflows_lint_fails_on_unused_dependencies() {
  local lint
  lint=$(awk '/^  lint:/ { l = 1; next } l && /^  [a-z]/ { exit } l' "$root/.github/workflows/ci.yml")
  case "$lint" in *"run: cargo machete"*) ok ;; *) bad "Lint doesn't run cargo machete" ;; esac
}

workflows_cache_saves_from_main_only() {
  local f bad_steps=""
  for f in "$root"/.github/workflows/*.yml; do
    bad_steps="$bad_steps$(awk -v f="${f##*/}" '
      /uses: Swatinem\/rust-cache@/ { if (open) print f ":" at; open = 1; at = NR; next }
      open && /save-if: (false|\$\{\{ github.ref == .refs\/heads\/main. \}\})$/ { open = 0; next }
      open && /^      - / { print f ":" at; open = 0 }
      END { if (open) print f ":" at }' "$f")"
  done
  [ -z "$bad_steps" ] || { bad "rust-cache saves off main at:" "$bad_steps"; return; }
  ok
}

workflows_lint_fails_on_copy_paste() {
  local lint
  lint=$(awk '/^  lint:/ { l = 1; next } l && /^  [a-z]/ { exit } l' "$root/.github/workflows/ci.yml")
  case "$lint" in *"run: npx --yes jscpd@"*" ."*) ok ;; *) bad "Lint doesn't run jscpd" ;; esac
}

# The real jscpd, with impish's .jscpd.json, on a block copied between two
# scripts. JSCPD=0 skips it where npx isn't installed.
copy_paste() { (cd "$work/copy" && npx --yes jscpd@4.3.0 --config "$root/.jscpd.json" . 2>&1); }

scripts_copy_paste_fails_between_two_scripts() {
  [ "${JSCPD:-1}" = 0 ] && { ok; return; }
  rm -rf "$work/copy"; mkdir -p "$work/copy/scripts"
  local i out rc
  for i in 1 2 3 4 5 6 7 8 9 10 11 12; do
    echo "echo \"step $i\" | sed 's/a/b/g' | awk '{ print \$1, \$2 }'"
  done > "$work/copy/scripts/a.sh"
  out=$(copy_paste); rc=$?
  expect_exit $rc 0 "$out" || return
  cp "$work/copy/scripts/a.sh" "$work/copy/scripts/b.sh"
  out=$(copy_paste); rc=$?
  expect_exit $rc 1 "$out" && expect_has "$out" "scripts/b.sh" && ok
}

# shellcheck disable=SC2016 # the workflow's ${GITHUB_STEP_SUMMARY}, literally
workflows_similar_backlog_runs_nightly() {
  local f="$root/.github/workflows/similar-backlog.yml"
  grep -q '^  schedule:' "$f" || { bad "similar-backlog.yml runs on no schedule"; return; }
  grep -qF 'scripts/similar-backlog.sh --file-issues | tee -a "${GITHUB_STEP_SUMMARY}"' "$f" ||
    { bad "similar-backlog.yml doesn't put the report in the job summary"; return; }
  ok
}

# ── scripts ────────────────────────────────────────────────────────────────

scripts_parse() {
  local f out=""
  for f in "$root"/scripts/*.sh "$root"/tests/pipeline/*.sh; do out="$out$("$sh" -n "$f" 2>&1)"; done
  [ -z "$out" ] || { bad "$sh -n" "$out"; return; }
  ok
}

scripts_pass_shellcheck() {
  [ "${SHELLCHECK:-1}" = 1 ] || { echo "PASS  $case (skipped: SHELLCHECK=0)"; return; }
  command -v shellcheck >/dev/null || { bad "shellcheck is not installed"; return; }
  local out
  out=$(shellcheck "$root"/scripts/*.sh "$root"/tests/pipeline/*.sh 2>&1) || { bad shellcheck "$out"; return; }
  ok
}

# ── shipped.sh ─────────────────────────────────────────────────────────────

# The Version job bumps on a change to any of these, so a source file or the
# README missing from the list would ship unreleased.
shipped_lists_what_the_crates_are_built_from() {
  cd "$root" || return
  local out rc
  out=$(sh scripts/shipped.sh 2>&1); rc=$?
  expect_exit $rc 0 "$out" || return
  for f in Cargo.toml Cargo.lock README.md src/main.rs core/src/lib.rs impish/src/lib.rs impish/Cargo.toml; do
    printf '%s\n' "$out" | grep -qx -- "$f" || { bad "lacks $f" "$out"; return; }
  done
  expect_lacks "$out" "tests/contract/" && expect_lacks "$out" ".github/" && ok
}

# ── similar-code.sh ────────────────────────────────────────────────────────

# A similarity-rs that prints the pairs in .pairs ("locA nameA locB nameB pct"
# per line) under any directory it is asked to scan.
fake_similarity() {
  mkdir -p "$work/bin"
  cat > "$work/bin/similarity-rs" <<'FAKE'
#!/usr/bin/env bash
[ -f .pairs ] || exit 0
while read -r la na lb nb pct; do
  hit=""
  for dir in "$@"; do case "$la" in "$dir"/*) hit=1 ;; esac; done
  [ -n "$hit" ] || continue
  printf '  %s function %s <-> %s function %s\n  Similarity: %s\n\n' "$la" "$na" "$lb" "$nb" "$pct"
done < .pairs
FAKE
  chmod +x "$work/bin/similarity-rs"
}

# main has a and b alike; the PR branch is checked out.
similar_repo() {
  repo sim
  fake_similarity
  cp "$root/scripts/similar-code.sh" scripts/
  : > scripts/similar-code.allow
  printf 'pub fn a() {}\npub fn b() {}\n' > src/lib.rs
  echo 'src/lib.rs:1-1 a src/lib.rs:2-2 b 95.00%' > .pairs
  commit setup
  git checkout -q -b pr
}

# The PR adds c, in the attribute crate, which looks like a.
add_c_like_a() {
  printf 'pub fn c() {}\n' > impish/src/lib.rs
  echo 'impish/src/lib.rs:1-1 c src/lib.rs:1-1 a 90.00%' >> .pairs
}

similar() { PATH="$work/bin:$PATH" "$sh" scripts/similar-code.sh main 2>&1; }

similar_code_fails_on_a_new_pair_only() {
  similar_repo && add_c_like_a && commit pr
  local out rc
  out=$(similar); rc=$?
  expect_exit $rc 1 "$out" && expect_has "$out" "1 new pair(s)" &&
    expect_has "$out" "error: function \`c\` is 90.00% similar to \`a\`" && expect_lacks "$out" "\`b\`" && ok
}

similar_code_passes_a_pair_already_on_the_base() {
  similar_repo
  printf 'pub fn a() {}\npub fn b() { }\n' > src/lib.rs
  commit pr
  local out rc
  out=$(similar); rc=$?
  expect_exit $rc 0 "$out" && expect_has "$out" "(already on the base)" && ok
}

similar_code_passes_a_declared_pair() {
  similar_repo && add_c_like_a
  echo 'src/lib.rs::a impish/src/lib.rs::c  # intended' > scripts/similar-code.allow
  commit pr
  local out rc
  out=$(similar); rc=$?
  expect_exit $rc 0 "$out" && expect_has "$out" "(declared)" && ok
}

similar_code_passes_without_rust_changes() {
  similar_repo
  echo docs > README.md && commit pr
  local out rc
  out=$(similar); rc=$?
  expect_exit $rc 0 "$out" && expect_has "$out" "no Rust source changed" && ok
}

# similar-backlog.sh: a~b and b~c make one cluster of three, d~e one of two,
# f~g is declared, h~i, j~k and l~m make three more. gh files into .created and lists
# what .filed holds.
backlog_repo() {
  repo backlog
  fake_similarity
  cp "$root/scripts/similar-backlog.sh" scripts/
  echo 'src/lib.rs::g src/lib.rs::f  # intended' > scripts/similar-code.allow
  local x y
  while read -r x y; do
    echo "src/lib.rs:1-1 $x src/lib.rs:2-2 $y 95.00%" >> .pairs
  done <<'PAIRS'
b a
b c
d e
f g
h i
j k
l m
PAIRS
  cat > "$work/bin/gh" <<'FAKE'
#!/usr/bin/env bash
case "$1 $2" in
  "issue list") cat .filed 2>/dev/null ;;
  "issue create") while [ $# -gt 0 ]; do [ "$1" = --title ] && echo "$2" >> .created; shift; done ;;
esac
FAKE
  chmod +x "$work/bin/gh"
}

backlog() { PATH="$work/bin:$PATH" "$sh" scripts/similar-backlog.sh "$@" 2>&1; }

similar_code_backlog_prints_clusters_largest_first() {
  backlog_repo
  local out rc
  out=$(backlog); rc=$?
  expect_exit $rc 0 "$out" && expect_lacks "$out" "::f" &&
    expect_has "$out" "5 cluster(s)" || return
  [ "$(printf '%s\n' "$out" | grep 'similar functions:' | head -2)" = "3 similar functions: src/lib.rs::a, src/lib.rs::b, src/lib.rs::c
2 similar functions: src/lib.rs::d, src/lib.rs::e" ] || { bad "clusters out of order" "$out"; return; }
  expect_has "$out" "  src/lib.rs:1-1 \`b\` ~ src/lib.rs:2-2 \`c\`: 95.00%" && ok
}

similar_code_backlog_files_three_new_clusters() {
  backlog_repo
  echo 'Similar code: src/lib.rs::d, src/lib.rs::e' > .filed
  local out rc
  out=$(backlog --file-issues); rc=$?
  expect_exit $rc 0 "$out" || return
  [ "$(cat .created)" = "Similar code: src/lib.rs::a, src/lib.rs::b, src/lib.rs::c
Similar code: src/lib.rs::h, src/lib.rs::i
Similar code: src/lib.rs::j, src/lib.rs::k" ] || { bad "filed the wrong issues" "$(cat .created)"; return; }
  ok
}

# ── kill-matrix.sh ─────────────────────────────────────────────────────────

# kill-matrix.sh's functions, without running its command line.
kill_matrix_lib() { sed '/^case /,$d' "$root/scripts/kill-matrix.sh" > "$work/km-lib.sh"; }

# m1 is killed by two tests, m2 by one of them; idle kills nothing.
kill_matrix_report_marks_what_can_go() {
  kill_matrix_lib
  printf 'm1\tcontract/a\nm1\tcontract/b\nm2\tcontract/b\n' > "$work/matrix.tsv"
  printf 'contract/a\ncontract/b\ncontract/idle\narchitecture/layers\n' > "$work/tests.txt"
  local out rc suspects
  # shellcheck disable=SC2016 # expands in the inner shell
  out=$(KILLMX_OUT="$work/kmout" "$sh" -c 'source "$1"; report "$2" "$3" "$4"' _ \
        "$work/km-lib.sh" "$work/matrix.tsv" "$work/tests.txt" "$work/suspects.tsv" 2>&1); rc=$?
  expect_exit $rc 0 "$out" && expect_has "$out" "contract/a   <- removable" &&
    expect_lacks "$out" "contract/b   <- removable" || return
  suspects=$(cut -f1 "$work/suspects.tsv" | sort | tr '\n' ' ')
  [ "$suspects" = "contract/a contract/idle " ] || { bad "suspects: '$suspects'"; return; }
  ok
}

# Each test the kill matrix expects to kill nothing must still exist, or a
# renamed one is filed as removable every night.
kill_matrix_quiet_tests_exist() {
  local re name target missing=""
  re=$(sed -n "s/^no_kills_expected='\(.*\)'$/\1/p" "$root/scripts/kill-matrix.sh")
  [ -n "$re" ] || { bad "no no_kills_expected in kill-matrix.sh"; return; }
  for name in $(printf '%s' "$re" | tr -d '^()' | tr '|' '\n' | grep '\$$' | tr -d '$'); do
    target=${name%%/*}
    grep -rqE "fn ${name#*/}\(" "$root/tests/$target.rs" "$root/tests/$target" 2>/dev/null ||
      missing="$missing $name"
  done
  [ -z "$missing" ] || { bad "no such test:$missing"; return; }
  ok
}

cases=$(declare -F | awk '{ print $3 }' | grep -E '^(workflows|scripts|shipped|similar_code|kill_matrix)_' |
  grep -vE '^(kill_matrix_lib)$')
passed=0 failed=0
for case in $cases; do
  case "$case" in *"$only"*) ;; *) continue ;; esac
  out=$( ("$case") 2>&1 )
  case "$out" in
    *"PASS  $case"*) passed=$((passed + 1)); echo "PASS  $case" ;;
    *) failed=$((failed + 1)); printf '%s\n' "${out:-FAIL  $case: no verdict}" | grep -E '^(FAIL|      )' || echo "FAIL  $case" ;;
  esac
done
echo "$passed passed, $failed failed"
[ "$failed" -eq 0 ] && [ "$passed" -gt 0 ]
