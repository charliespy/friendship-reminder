#!/usr/bin/env bash
# Tests scripts/privacy-scan.sh in a scratch git repo: what it must catch, and what it must let through.
#   scripts/test-privacy-scan.sh
set -euo pipefail
scan="$(cd "$(dirname "$0")" && pwd)/privacy-scan.sh"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
failures=0

expect() { # expect <pass|fail> <name> <command...>
  local want=$1 name=$2
  shift 2
  if "$@" >"$work/out" 2>&1; then got=pass; else got=fail; fi
  if [ "$got" = "$want" ]; then
    echo "ok   $name"
  else
    echo "FAIL $name (wanted $want, got $got)"
    sed 's/^/     /' "$work/out"
    failures=$((failures + 1))
  fi
}
text() { printf '%s\n' "$1" | bash "$scan" --stdin; }

# Text (a PR title and description).
expect pass "plain text" text "feat: snooze a reminder"
expect pass "noreply and example addresses" text "42754940+someone@users.noreply.github.com, ada@example.com"
expect pass "git remote and retina image" text "git@github.com:owner/repo.git, icon@2x.png"
expect pass "placeholder and system folders" text "/Users/you, /home/<name>, /Users/Shared, C:\\Users\\Public"
expect pass "a link with /home/ in it" text "https://example.com/home/page and https://example.com/Users/list"
expect fail "personal email" text "write to jane.doe@gmail.com"
expect fail "home folder" text "it lives in /Users/jane/Code"
expect fail "linux home folder" text "/home/jane/.config"
expect fail "windows home folder" text "C:\\Users\\jane\\Desktop"
expect fail "tailnet host" text "https://my-mac.tail1234.ts.net:4100/"
expect fail "co-author trailer" text "Co-authored-by: Someone <someone@example.com>"

# A repo: files and commits.
repo="$work/repo"
git init -q "$repo"
cd "$repo"
git config user.name tester
git config user.email "1+tester@users.noreply.github.com"
mkdir scripts
cp "$scan" scripts/privacy-scan.sh
printf 'ok\n' >README.md
git add -A
git commit -qm "chore: start"
base=$(git rev-parse HEAD)

expect pass "a clean repo" bash scripts/privacy-scan.sh --range "$base"
expect fail "a range git can't read" bash scripts/privacy-scan.sh --range "deadbeef..HEAD"

printf 'mail jane@gmail.com\n' >./-n.txt
git add -- ./-n.txt
git commit -qm "test: odd file name"
expect fail "email in a file whose name starts with -" bash scripts/privacy-scan.sh --range "$base..HEAD"
git rm -q -- ./-n.txt
git commit -qm "test: remove it"

printf 'see /Users/jane\n' >"a b:c.md"
git add -- "a b:c.md"
git commit -qm "test: spaces and a colon"
expect fail "home folder in a file named with a space and a colon" bash scripts/privacy-scan.sh --range "HEAD~1..HEAD"
git rm -q -- "a b:c.md"
git commit -qm "test: remove it"

printf '\x00\x01 jane@gmail.com' >blob.bin
git add blob.bin
git commit -qm "test: binary"
expect pass "binary files are skipped" bash scripts/privacy-scan.sh --range "HEAD~1..HEAD"
git rm -q blob.bin
git commit -qm "test: remove it"

git commit -q --allow-empty -m "chore: by someone" --author="Jane <jane@gmail.com>"
expect fail "personal author" bash scripts/privacy-scan.sh --range "HEAD~1..HEAD"
GIT_COMMITTER_EMAIL=noreply@github.com git commit -q --allow-empty -m "chore: web edit" \
  --author="Web <web@gmail.com>"
expect fail "personal author of a commit GitHub made" bash scripts/privacy-scan.sh --range "HEAD~1..HEAD"
GIT_COMMITTER_EMAIL=noreply@github.com git commit -q --allow-empty -m "chore: merged on GitHub"
expect pass "GitHub as committer" bash scripts/privacy-scan.sh --range "HEAD~1..HEAD"
git commit -q --allow-empty -m $'feat: y\n\nCo-authored-by: Bot <bot@example.com>'
expect fail "co-author trailer in a message" bash scripts/privacy-scan.sh --range "HEAD~1..HEAD"

if [ "$failures" -ne 0 ]; then
  echo "$failures privacy-scan test(s) failed"
  exit 1
fi
echo "privacy-scan tests passed"
