#!/usr/bin/env bash
# Tests for the scripts and workflows that make up impish's CI pipeline. A gate
# that can't fail passes every PR it should stop (Impatient Programming, rule
# 24), so each case builds a throwaway git repo, fakes the slow tools
# (similarity-rs, cargo-mutants, gh) and checks a script's
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
  needs=$(awk '$0 == "  mutants:" { m = 1; next } m && /^  [a-z]/ { m = 0 } m && /^    needs:/ { print; exit }' "$root/.github/workflows/ci.yml")
  for want in lint deny zizmor test similar version; do
    case "$needs" in *"$want"*) ;; *) missing="$missing mutants:$want" ;; esac
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

# A sweep is hours of runner time: nightly, in shards, never on a PR.
workflows_kill_matrix_runs_nightly() {
  local f="$root/.github/workflows/kill-matrix.yml"
  grep -q '^  schedule:' "$f" || { bad "kill-matrix.yml runs on no schedule"; return; }
  ! grep -q 'pull_request' "$f" || { bad "kill-matrix.yml runs on PRs"; return; }
  grep -qF 'scripts/kill-matrix.sh --merge --file-issues' "$f" || { bad "kill-matrix.yml files no issues"; return; }
  ok
}

# m1 is killed by a Contract test and another test, m2 by a third. nextest
# names a failed test again in its summary, so a kill can be listed twice.
kill_matrix_report_keeps_contract_tests() {
  repo km
  printf '#[impish::contract("add/A1")]\nfn spec() {}\n' > tests/contract/add.rs
  printf 'm1\tcontract/add::spec\nm1\tsuite/dup\nm2\tsuite/own\nm2\tsuite/own\n' > "$work/matrix.tsv"
  printf 'contract/add::spec\nsuite/dup\nsuite/own\nsuite/idle\narchitecture/layers\n' > "$work/tests.txt"
  sed '/^case /,$d' "$root/scripts/kill-matrix.sh" > "$work/km-lib.sh"
  local out rc suspects
  # shellcheck disable=SC2016 # expands in the inner shell
  out=$(KILLMX_OUT="$work/kmout" "$sh" -c 'source "$1"; report "$2" "$3" "$4"' _ \
        "$work/km-lib.sh" "$work/matrix.tsv" "$work/tests.txt" "$work/suspects.tsv" 2>&1); rc=$?
  expect_exit $rc 0 "$out" && expect_has "$out" "contract/add::spec   (Contract)" && expect_has "$out" "suite/dup   <- removable" &&
    expect_lacks "$out" "own   <- removable" && expect_has "$out" "   1    1  suite/own" || return
  suspects=$(cut -f1 "$work/suspects.tsv" | sort | tr '\n' ' ')
  [ "$suspects" = "suite/dup suite/idle " ] || { bad "suspects: '$suspects'"; return; }
  ok
}

# Shard a alone ties dup and own, so it would keep dup; merged with shard b,
# own covers both mutants and dup is the one that can go.
kill_matrix_merges_shards_before_judging() {
  repo kmmerge
  printf 'fn spec() {}\n' > tests/contract/add.rs
  mkdir -p "$work/shard-a" "$work/shard-b"
  printf 'm1\tsuite/dup\nm1\tsuite/own\n' > "$work/shard-a/matrix.tsv"
  printf 'm2\tsuite/own\n' > "$work/shard-b/matrix.tsv"
  printf 'suite/dup\nsuite/own\nsuite/idle\n' | tee "$work/shard-a/tests.txt" > "$work/shard-b/tests.txt"
  local out rc
  out=$(KILLMX_OUT="$work/kmmerge-out" "$sh" "$root/scripts/kill-matrix.sh" --merge "$work/shard-a" "$work/shard-b" 2>&1); rc=$?
  expect_exit $rc 0 "$out" && expect_has "$out" "2 caught mutant(s), killed by 2 of 3 test(s)." &&
    expect_has "$out" "suite/dup   <- removable" && expect_lacks "$out" "own   <- removable" && ok
}

# A fake `cargo mutants` for kill-matrix.sh --test: on its own the test kills
# m1 and m2; without it the suite kills m1 and misses m2, the last mutant.
fake_mutants() {
  mkdir -p "$work/bin"
  cat > "$work/bin/cargo" <<'FAKE'
#!/usr/bin/env bash
printf '%s\n' "$*" >> "$(dirname "$0")/cargo.args"
while [ $# -gt 0 ]; do [ "$1" = --output ] && out="$2"; shift; done
mkdir -p "$out/mutants.out/log"
m() { printf '{"scenario":{"Mutant":{"name":"%s"}},"summary":"%s","log_path":"log/%s.log"}' "$1" "$2" "$1"; }
case "$out" in
  */one) rest="$(m m1 CaughtMutant),$(m m2 CaughtMutant)" ;;
  *) rest="$(m m1 CaughtMutant),$(m m2 MissedMutant)"
     printf '        PASS [   0.010s] (  1/41) cargo-impish::contract check::t\n        FAIL [   0.020s] (  2/41) cargo-impish::suite other\n' > "$out/mutants.out/log/m1.log" ;;
esac
printf '{"outcomes":[{"scenario":"Baseline","summary":"Success","log_path":"log/b.log"},%s]}' "$rest" > "$out/mutants.out/outcomes.json"
FAKE
  chmod +x "$work/bin/cargo"
}

kill_matrix_keeps_a_test_whose_last_mutant_only_it_catches() {
  repo kmlast
  printf 'fn spec() {}\n' > tests/contract/add.rs
  cp "$root/scripts/kill-matrix.sh" scripts/
  fake_mutants
  local out rc args
  out=$(PATH="$work/bin:$PATH" KILLMX_OUT="$work/kmlast-out" "$sh" scripts/kill-matrix.sh --test contract/check::t src/lib.rs 2>&1); rc=$?
  expect_exit $rc 0 "$out" && expect_has "$out" "Keep contract/check::t: it alone catches 1 mutant(s):" || return
  args=$(cat "$work/bin/cargo.args")
  expect_has "$args" "mutants --workspace" &&
    expect_has "$args" "--test contract -E binary_id(*::contract) & test(=check::t)" &&
    expect_has "$args" "-E not (binary_id(*::contract) & test(=check::t))" && ok
}

# A crate's own unit tests are its library's binary, or the binary's.
kill_matrix_finds_unit_tests_in_each_crate() {
  sed '/^case /,$d' "$root/scripts/kill-matrix.sh" > "$work/km-lib.sh"
  local got
  # shellcheck disable=SC2016 # expands in the inner shell
  got=$("$sh" -c 'source "$1"; for t in impish-core/a::t cargo-impish/b::t; do echo "$(target_args "$t") $(test_filter "$t")"; done' _ "$work/km-lib.sh")
  expect_has "$got" "-p impish-core --lib binary_id(impish-core) & test(=a::t)" &&
    expect_has "$got" "--bin cargo-impish binary_id(cargo-impish::bin/cargo-impish) & test(=b::t)" && ok
}

# Kills and passes come from nextest status lines; a timeout is a kill.
kill_matrix_reads_tests_from_a_nextest_log() {
  sed '/^case /,$d' "$root/scripts/kill-matrix.sh" > "$work/km-lib.sh"
  printf '%s\n' \
    '    Starting 4 tests across 3 binaries' \
    '       START [         ] (1/4) cargo-impish::contract check::spec' \
    '        SLOW [> 60.000s] (3/4) cargo-impish::suite slow' \
    '        PASS [   0.010s] (1/4) cargo-impish::contract check::spec' \
    $'\033[31m        FAIL\033[0m [   0.020s] (2/4) impish-core parse::own' \
    '     TIMEOUT [  60.000s] (3/4) cargo-impish::suite slow' \
    '        PASS [   0.030s] (4/4) cargo-impish::bin/cargo-impish x::fine' \
    '     Summary [   0.100s] 4 tests run: 2 passed, 2 failed' > "$work/nx.log"
  local failed passed
  # shellcheck disable=SC2016 # expands in the inner shell
  failed=$("$sh" -c 'source "$1"; tests_in_log "$2" FAILED' _ "$work/km-lib.sh" "$work/nx.log" | sort -u | tr '\n' ' ')
  # shellcheck disable=SC2016 # expands in the inner shell
  passed=$("$sh" -c 'source "$1"; tests_in_log "$2" ok' _ "$work/km-lib.sh" "$work/nx.log" | tr '\n' ' ')
  [ "$failed" = "impish-core/parse::own suite/slow " ] || { bad "failed: '$failed'"; return; }
  [ "$passed" = "contract/check::spec cargo-impish/x::fine " ] || { bad "passed: '$passed'"; return; }
  ok
}

cases=$(declare -F | awk '{ print $3 }' | grep -E '^(workflows|scripts|shipped|similar_code|kill_matrix)_')
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
