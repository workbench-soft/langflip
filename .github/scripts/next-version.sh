#!/bin/sh
# Prints the next version: the Sydney date, then a counter for releases that day.
#   no tag today            -> 2025.10.25.1
#   v2025.10.25.2 exists    -> 2025.10.25.3
set -eu

day=$(TZ=Australia/Sydney date +%Y.%m.%d)
git tag --list "v$day.*" | awk -F. -v day="$day" '
  { if ($4 + 0 > last) last = $4 + 0 }
  END { print day "." last + 1 }'
