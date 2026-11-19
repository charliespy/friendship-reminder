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
allowed_email='(@users\.noreply\.github\.com|@example\.(com|org|net)|^git@github\.com|@[0-9]+x\.(png|jpe?g|svg|webp|gif|avif))$'
email='[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}'
# Home folders, in the case the systems write them, so web routes (/api/users/list) don't match:
# macOS /Users/<name> anywhere (also inside local dev links like /@fs/Users/...); Linux /home/<name>
# when it starts a path or follows a dev server's /@fs (not /api/home/...); Windows C:\Users\<name>
# in any case. Accepted gaps, to keep web routes passing: a lower-case /users/<name> (macOS would
# open it) and /var/home/<name>.
mac_home='/Users/[A-Za-z0-9._-]+'
linux_home='(^|[^A-Za-z0-9._-]|/@fs)/home/[A-Za-z0-9._-]+'
windows_home='[A-Za-z]:[\\/]+Users[\\/]+[A-Za-z0-9._-]+'
# Placeholders a doc may use for a home folder, and the system's shared folders.
allowed_path='[\\/](Users|home)[\\/]+(you|me|runner|USER|\$USER|<[^>]*>|example|name|username|Shared|Public|Default|All Users)$'
# A web link to a named site (example.com): a /home/... or /Users/... in it is a page, not a folder.
# Other links do point at folders on someone's disk, and are checked: file://..., and local dev
# servers (http://localhost:1420/@fs/Users/...).
url='(https?|ftp)://([A-Za-z0-9-]+\.)+[A-Za-z]{2,}(:[0-9]+)?(/[^[:space:]<>"()]*)?'
private_host='([A-Za-z0-9-]+\.)+ts\.net'
attribution='(Co-authored-by:|Generated with \[?Claude|noreply@anthropic\.com)'

# Scans text on stdin; $1 says where it came from.
scan_text() {
  local where=$1 text hit
  text=$(cat)
  while IFS= read -r hit; do
    [ -n "$hit" ] && report "email address in $where: $hit"
  done < <(printf '%s\n' "$text" | grep -oE "$email" | grep -viE "$allowed_email" || true)
  while IFS= read -r hit; do
    [ -n "$hit" ] && report "home-folder path in $where: $hit"
  done < <(
    paths=$(printf '%s\n' "$text" | sed -E "s#$url##g")
    {
      printf '%s\n' "$paths" | grep -oE "$mac_home" || true
      printf '%s\n' "$paths" | grep -oE "$linux_home" | sed -E 's#^.*(/home/)#\1#' || true
      printf '%s\n' "$paths" | grep -oiE "$windows_home" || true
    } | grep -viE "$allowed_path" || true
  )
  while IFS= read -r hit; do
    [ -n "$hit" ] && report "private host name in $where: $hit"
  done < <(printf '%s\n' "$text" | grep -oiE "$private_host" || true)
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
case "$mode" in
  "" | --all | --range | --stdin) ;;
  *)
    echo "usage: privacy-scan.sh [--all | --range A..B | --stdin]" >&2
    exit 2
    ;;
esac
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
# Fields split by \x1f: read would merge tabs, and an empty author would shift the fields.
while IFS=$'\x1f' read -r sha author committer; do
  [ -n "$sha" ] || continue
  if ! printf '%s' "$author" | grep -qiE "$allowed_email"; then
    report "commit ${sha:0:12} has a personal author address ($author); use your GitHub noreply address"
  fi
  # GitHub itself commits web edits and PR merges as noreply@github.com.
  if [ "$committer" != "noreply@github.com" ] && ! printf '%s' "$committer" | grep -qiE "$allowed_email"; then
    report "commit ${sha:0:12} has a personal committer address ($committer)"
  fi
done < <(git log --format='%H%x1f%ae%x1f%ce' "$range")
while IFS= read -r -d '' record; do
  sha=${record%%$'\n'*}
  [ -n "$sha" ] || continue
  scan_text "the message of commit ${sha:0:12}" < <(printf '%s\n' "${record#*$'\n'}")
done < <(git log -z --format='%H%n%B' "$range")

# 3. What the commits added: something added in one commit and removed in the next is gone from
#    the files but stays public in the history. Same files skipped as above. The diff goes to a file
#    first, so a failing git log fails the scan instead of scanning nothing.
diff_file=$(mktemp)
trap 'rm -f "$diff_file"' EXIT
git log -p --no-ext-diff --no-color --format= "$range" -- . \
  ':(glob,exclude)**/package-lock.json' ':(glob,exclude)**/Cargo.lock' \
  ':(exclude)scripts/privacy-scan.sh' ':(exclude)scripts/test-privacy-scan.sh' >"$diff_file"
# Added lines start with "+"; git's own "+++ b/<file>" (or "+++ /dev/null") header lines don't count,
# but an added line whose text starts with "++" does.
scan_text "the changes in $range" < <(grep -E '^\+' "$diff_file" | grep -vE '^\+\+\+ (b/|/dev/null)' || true)

finish
