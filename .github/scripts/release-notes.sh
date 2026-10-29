#!/bin/sh
# Prints Markdown release notes for the commits after the release before <ref>.
#   release-notes.sh v0.0.109          notes for HEAD, about to be tagged v0.0.109
#   release-notes.sh v0.0.108 v0.0.108 notes for a tag that already exists
set -eu

tag=$1
ref=${2:-HEAD}
previous=$(git describe --tags --abbrev=0 --match 'v[0-9]*.[0-9]*.[0-9]*' "$ref^" 2>/dev/null || true)
range=${previous:+$previous..}$ref

commits() {
  git log --reverse --no-merges --format='%H %s' "$range" | grep -E "$1" | cut -d' ' -f1 || true
}

# Subject minus its type, then the first body paragraph. The rest of the body stays out, for developers.
note() {
  git log -1 --format='%s' "$1" | sed -E 's/^[a-z]+(\([^)]*\))?!?: //' |
    awk -v sha="$(git rev-parse --short "$1")" '{ print "- " toupper(substr($0, 1, 1)) substr($0, 2) " (" sha ")" }'
  git log -1 --format='%b' "$1" | awk '
    /^(Co-authored-by|Signed-off-by):/ { next }
    /^[[:space:]]*$/ { if (seen) exit; next }
    { seen = 1; print "  " $0 }'
}

section() {
  shas=$(commits "$2")
  [ -n "$shas" ] || return 0
  printf '## %s\n\n' "$1"
  for sha in $shas; do note "$sha"; done
  echo
}

section 'New' '^[0-9a-f]+ feat(\([^)]*\))?!?: '
section 'Fixes' '^[0-9a-f]+ fix(\([^)]*\))?!?: '
section 'Faster' '^[0-9a-f]+ perf(\([^)]*\))?!?: '

others=$(git log --reverse --no-merges --format='%H %s' "$range" |
  grep -vE '^[0-9a-f]+ (feat|fix|perf)(\([^)]*\))?!?: ' | cut -d' ' -f2- || true)
if [ -n "$others" ]; then
  printf '<details><summary>Other changes</summary>\n\n'
  echo "$others" | sed 's/^/- /'
  printf '\n</details>\n\n'
fi

if [ -n "$previous" ] && [ -n "${GITHUB_REPOSITORY:-}" ]; then
  echo "**Full Changelog**: https://github.com/$GITHUB_REPOSITORY/compare/$previous...$tag"
fi
