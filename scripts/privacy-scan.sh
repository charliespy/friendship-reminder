#!/usr/bin/env bash
# Fails when tracked files or commit metadata hold personal info: email addresses, home-folder
# paths, private host names, or AI attribution lines. This repo is public, so CI runs it on every PR.
#
#   scripts/privacy-scan.sh            scan tracked files and the commits of this branch
#   scripts/privacy-scan.sh --all      also scan every commit in history
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

fail=0
report() { echo "privacy-scan: $1" >&2; fail=1; }

# Addresses that are fine to publish: GitHub noreply addresses and the reserved example domains.
allowed_email='(users\.noreply\.github\.com|@example\.(com|org|net)|@localhost|@[0-9]+x\.(png|jpg|svg))'
email='[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}'
home_path='(/Users/[A-Za-z0-9._-]+|/home/[A-Za-z0-9._-]+|C:\\\\Users\\\\[A-Za-z0-9._-]+)'
private_host='[A-Za-z0-9-]+\.[A-Za-z0-9-]+\.ts\.net'
attribution='(Co-authored-by:|Generated with \[?Claude|noreply@anthropic\.com)'
# Placeholders a doc may use for a home folder.
allowed_path='/(Users|home)/(you|me|runner|USER|\$USER|<[^>]*>|example|name|username)\b'

# 1. Tracked files (lock files are generated and full of package metadata; skip them).
files=$(git ls-files -z | xargs -0 -n1 echo | grep -vE '(^|/)(package-lock\.json|Cargo\.lock)$|\.(png|icns|ico|jpg|gif|pdf)$' | grep -v '^scripts/privacy-scan\.sh$' || true)
if [ -n "$files" ]; then
  while IFS= read -r hit; do
    report "email address in $hit"
  done < <(echo "$files" | tr '\n' '\0' | xargs -0 grep -nHoE "$email" 2>/dev/null | grep -vE "$allowed_email" || true)
  while IFS= read -r hit; do
    report "home-folder path in $hit"
  done < <(echo "$files" | tr '\n' '\0' | xargs -0 grep -nHoE "$home_path" 2>/dev/null | grep -vE "$allowed_path" || true)
  while IFS= read -r hit; do
    report "private host name in $hit"
  done < <(echo "$files" | tr '\n' '\0' | xargs -0 grep -nHoE "$private_host" 2>/dev/null || true)
  while IFS= read -r hit; do
    report "AI attribution in $hit"
  done < <(echo "$files" | tr '\n' '\0' | xargs -0 grep -nHiE "$attribution" 2>/dev/null || true)
fi

# 2. Commit authors, committers and messages.
range="HEAD"
if [ "${1:-}" != "--all" ]; then
  base=$(git merge-base HEAD origin/main 2>/dev/null || true)
  [ -n "$base" ] && range="$base..HEAD"
fi
while IFS=$'\t' read -r sha ae ce; do
  for addr in "$ae" "$ce"; do
    if ! echo "$addr" | grep -qE "$allowed_email"; then
      report "commit ${sha:0:12} has a personal address ($addr); use your GitHub noreply address"
    fi
  done
done < <(git log --format='%H%x09%ae%x09%ce' "$range" | grep -v $'\tnoreply@github.com$' || true)
for c in $(git log --format='%H' "$range"); do
  msg=$(git log -1 --format='%B' "$c")
  if echo "$msg" | grep -qiE "$attribution"; then
    report "commit ${c:0:12} message has an AI attribution line"
  fi
  if echo "$msg" | grep -oE "$email" | grep -qvE "$allowed_email"; then
    report "commit ${c:0:12} message has an email address"
  fi
done

if [ "$fail" -ne 0 ]; then
  echo "privacy-scan: failed. Remove the personal info above (this repo is public)." >&2
  exit 1
fi
echo "privacy-scan: clean"
