#!/usr/bin/env bash
# The similar functions already in the crates, as a backlog (Impatient
# Programming, rule 12, the exhale). The Similar code gate stops new pairs;
# this reports the ones already here, grouped into clusters of functions that
# are alike, largest first.
#
#   scripts/similar-backlog.sh                 # print every cluster
#   scripts/similar-backlog.sh --file-issues   # and file an issue per cluster
#
# An issue (label similar-code) is titled by its cluster's functions, so each
# cluster is filed once, at most 3 a run; closing one means "keep it". Pairs
# declared in scripts/similar-code.allow are left out. SIMILARITY_THRESHOLD
# (default 0.9) sets the minimum similarity, as in similar-code.sh.
set -euo pipefail

file_issues=0
case "${1:-}" in
  "") ;;
  --file-issues) file_issues=1 ;;
  *) echo "usage: $0 [--file-issues]" >&2; exit 2 ;;
esac
threshold="${SIMILARITY_THRESHOLD:-0.9}"
allow="$(dirname "$0")/similar-code.allow"
max_new_issues=3
# The workspace's crates: cargo-impish, impish-core and the impish attribute.
roots=(src core/src impish/src)

# One cluster per line: "<size>\t<function>, <function>, …\t<pair>;<pair>;…",
# largest first. A function is "file::name"; clusters join pairs that share one.
clusters=$({
  if [ -f "$allow" ]; then sed -e 's/#.*//' -e '/^[[:space:]]*$/d' -e 's/^/ALLOW /' "$allow"; fi
  similarity-rs "${roots[@]}" --skip-test --threshold "$threshold"
} | awk '
    function path(loc) { sub(/:.*/, "", loc); return loc }
    function key(x, y) { return x < y ? x " " y : y " " x }
    function find(x) { while (up[x] != x) x = up[x]; return x }
    /^ALLOW / { allowed[key($2, $3)] = 1; next }
    /<->/ { pair = $0; next }
    /Similarity:/ && pair != "" {
      split(pair, side, " <-> "); sub(/^ +/, "", side[1])
      split(side[1], a, " "); split(side[2], b, " ")
      x = path(a[1]) "::" a[3]; y = path(b[1]) "::" b[3]
      pair = ""
      if (key(x, y) in allowed || seen[key(x, y)]++) next
      if (!(x in up)) up[x] = x
      if (!(y in up)) up[y] = y
      up[find(x)] = find(y)
      line[++n] = sprintf("%s `%s` ~ %s `%s`: %s", a[1], a[3], b[1], b[3], $2)
      first[n] = x
    }
    END {
      for (f in up) { r = find(f); k = ++size[r]; member[r, k] = f }
      for (i = 1; i <= n; i++) { r = find(first[i]); pairs[r] = pairs[r] (pairs[r] == "" ? "" : ";") line[i] }
      for (r in size) {
        # Insertion sort: awk has no portable sort.
        for (i = 2; i <= size[r]; i++)
          for (j = i; j > 1 && member[r, j - 1] > member[r, j]; j--) {
            t = member[r, j]; member[r, j] = member[r, j - 1]; member[r, j - 1] = t
          }
        m = member[r, 1]
        for (i = 2; i <= size[r]; i++) m = m ", " member[r, i]
        printf "%d\t%s\t%s\n", size[r], m, pairs[r]
      }
    }' | sort -t "$(printf '\t')" -k1,1nr -k2,2)

count=0
while IFS=$'\t' read -r size members pairs; do
  [ -n "$size" ] || continue
  count=$((count + 1))
  echo "$size similar functions: $members"
  printf '%s\n' "$pairs" | tr ';' '\n' | sed 's/^/  /'
done <<< "$clusters"
echo "$count cluster(s) of similar functions"

[ "$file_issues" = 1 ] || exit 0
gh label create similar-code --color FBCA04 \
  --description "Functions alike enough to merge" 2>/dev/null || true
filed_before=$(gh issue list --label similar-code --state all --limit 1000 --json title -q '.[].title')
filed=0
while IFS=$'\t' read -r size members pairs; do
  [ -n "$size" ] || continue
  [ "$filed" -lt "$max_new_issues" ] || break
  title="Similar code: $members"
  printf '%s\n' "$filed_before" | grep -qxF -- "$title" && continue
  body="These $size functions are alike:"$'\n\n'"$(printf '%s\n' "$pairs" | tr ';' '\n' | sed 's/^/- /')"
  body+=$'\n\n'"Merge them, or extract what they share. To keep them, list the pairs in \`scripts/similar-code.allow\` and close this issue. Found by \`scripts/similar-backlog.sh\`."
  gh issue create --label similar-code --title "$title" --body "$body" >/dev/null
  echo "Filed an issue for $members"
  filed=$((filed + 1))
done <<< "$clusters"
