#!/usr/bin/env bash
# Fails when tracked files, commits or a PR's text hold personal info: email addresses, home-folder
# paths, private host names, or AI attribution lines. This repo is public, so CI runs it on every PR.
#
#   scripts/privacy-scan.sh                   tracked files, and the commits not on origin/main yet
#   scripts/privacy-scan.sh --range A..B      tracked files, and the commits in A..B (CI passes this)
#   scripts/privacy-scan.sh --all             tracked files, and every commit in history
#   ... | scripts/privacy-scan.sh --stdin     only the text piped in (CI: the PR title and description)
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

fail=0
report() { echo "privacy-scan: $1" >&2; fail=1; }

# Addresses that are fine to publish: GitHub noreply addresses, the reserved example domains, git
# remotes, and retina image names (icon@2x.png).
allowed_email='(users\.noreply\.github\.com|@example\.(com|org|net)|@localhost|^git@github\.com|@[0-9]+x\.(png|jpg|svg))'
email='[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}'
home_path='(/Users/[A-Za-z0-9._-]+|/home/[A-Za-z0-9._-]+|[A-Za-z]:[\\/]+Users[\\/]+[A-Za-z0-9._-]+)'
# Placeholders a doc may use for a home folder, and the system's shared folders.
allowed_path='[\\/](Users|home)[\\/]+(you|me|runner|USER|\$USER|<[^>]*>|example|name|username|Shared|Public|Default|All Users)$'
# Web links: a /home/... or /Users/... inside one is a page, not a folder on someone's disk.
url='[A-Za-z][A-Za-z0-9+.-]*://[^[:space:]<>"()]+'
private_host='[A-Za-z0-9-]+\.[A-Za-z0-9-]+\.ts\.net'
attribution='(Co-authored-by:|Generated with \[?Claude|noreply@anthropic\.com)'

# Scans text on stdin; $1 says where it came from.
scan_text() {
  local where=$1 text hit
  text=$(cat)
  while IFS= read -r hit; do
    [ -n "$hit" ] && report "email address in $where: $hit"
  done < <(printf '%s\n' "$text" | grep -oE "$email" | grep -vE "$allowed_email" || true)
  while IFS= read -r hit; do
    [ -n "$hit" ] && report "home-folder path in $where: $hit"
  done < <(printf '%s\n' "$text" | sed -E "s#$url##g" | grep -oE "$home_path" | grep -vE "$allowed_path" || true)
  while IFS= read -r hit; do
    [ -n "$hit" ] && report "private host name in $where: $hit"
  done < <(printf '%s\n' "$text" | grep -oE "$private_host" || true)
  while IFS= read -r hit; do
    [ -n "$hit" ] && report "AI attribution in $where: $hit"
  done < <(printf '%s\n' "$text" | grep -iE "$attribution" || true)
}

finish() {
  if [ "$fail" -ne 0 ]; then
    echo "privacy-scan: failed. Remove the personal info above (this repo is public)." >&2
    exit 1
  fi
  echo "privacy-scan: clean"
  exit 0
}

mode=${1:-}
if [ "$mode" = "--stdin" ]; then
  scan_text "the text"
  finish
fi

# 1. Tracked text files, one at a time (grep -I skips binary ones). Lock files are generated package
#    metadata; this script and its test name the patterns.
skip='(^|/)(package-lock\.json|Cargo\.lock)$|^scripts/(test-)?privacy-scan\.sh$'
while IFS= read -r -d '' f; do
  printf '%s' "$f" | grep -qE "$skip" && continue
  [ -f "$f" ] && grep -Iq . -- "$f" 2>/dev/null || continue
  scan_text "$f" < "$f"
done < <(git ls-files -z)

# 2. Commits: authors, committers and messages.
case "$mode" in
  --all) range=HEAD ;;
  --range) range=${2:?--range needs A..B} ;;
  *)
    base=$(git merge-base HEAD origin/main 2>/dev/null || true)
    range=${base:+$base..}HEAD
    ;;
esac
# A range git can't resolve (a typo, or a push's old tip after a force-push) must fail, not pass.
if ! git rev-list --quiet "$range" -- 2>/dev/null; then
  echo "privacy-scan: can't read the commits in $range" >&2
  exit 1
fi
while IFS=$'\t' read -r sha author committer; do
  [ -n "$sha" ] || continue
  if ! printf '%s' "$author" | grep -qE "$allowed_email"; then
    report "commit ${sha:0:12} has a personal author address ($author); use your GitHub noreply address"
  fi
  # GitHub itself commits web edits and PR merges as noreply@github.com.
  if [ "$committer" != "noreply@github.com" ] && ! printf '%s' "$committer" | grep -qE "$allowed_email"; then
    report "commit ${sha:0:12} has a personal committer address ($committer)"
  fi
done < <(git log --format='%H%x09%ae%x09%ce' "$range")
while IFS= read -r -d '' record; do
  sha=${record%%$'\n'*}
  [ -n "$sha" ] || continue
  scan_text "the message of commit ${sha:0:12}" < <(printf '%s\n' "${record#*$'\n'}")
done < <(git log -z --format='%H%n%B' "$range")

finish
