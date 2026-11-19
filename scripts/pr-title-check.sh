#!/usr/bin/env bash
# Fails unless a PR title is a conventional commit, because the title becomes the squash commit on
# main: "<type>: <what>" or "<type>(<scope>): <what>", with "!" before the colon for a breaking
# change. GitHub's Revert button titles (Revert "feat: ...") are accepted too. CI runs it on every
# PR, and it's a required check.
#
#   scripts/pr-title-check.sh "feat: snooze a reminder"
#   TITLE="fix(tray): ..." scripts/pr-title-check.sh
set -euo pipefail

title=${1-${TITLE-}}
types='feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert'
pattern="^($types)(\\([a-z0-9][a-z0-9._/-]*\\))?!?: [^[:space:]]"

revert='^Revert "[^"].*"$'
if { printf '%s' "$title" | grep -qE "$pattern" || printf '%s' "$title" | grep -qE "$revert"; } &&
  [ "$(printf '%s' "$title" | wc -l)" -eq 0 ]; then
  echo "pr-title-check: ok: $title"
  exit 0
fi
cat >&2 <<EOF
pr-title-check: the PR title isn't a conventional commit:
  $title
Write it as "<type>: <what>" or "<type>(<scope>): <what>", for example "feat: snooze a reminder"
or "fix(tray): keep the dropdown under the icon". Types: ${types//|/, }.
Add "!" before the colon for a breaking change. Edit the title and the check runs again.
EOF
exit 1
