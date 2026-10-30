#!/bin/sh
# Prints the next version: the Sydney date, then a counter for releases that day.
#   no tag today            -> 2025.10.25.1
#   v2025.10.25.2 exists    -> 2025.10.25.3
set -eu

# Git Bash on the Windows runner has no zone data and would give the UTC date, so Windows asks itself.
if [ -e /usr/share/zoneinfo/Australia/Sydney ]; then
  day=$(TZ=Australia/Sydney date +%Y.%m.%d)
else
  day=$(powershell.exe -NoProfile -Command "[TimeZoneInfo]::ConvertTimeBySystemTimeZoneId([DateTime]::UtcNow, 'AUS Eastern Standard Time').ToString('yyyy.MM.dd')" | tr -d '\r')
fi
git tag --list "v$day.*" | awk -F. -v day="$day" '
  { if ($4 + 0 > last) last = $4 + 0 }
  END { print day "." last + 1 }'
