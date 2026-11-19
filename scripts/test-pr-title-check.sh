#!/usr/bin/env bash
# Tests scripts/pr-title-check.sh: titles it must accept, and titles it must refuse.
#   scripts/test-pr-title-check.sh
set -euo pipefail
check="$(cd "$(dirname "$0")" && pwd)/pr-title-check.sh"
failures=0

expect() { # expect <pass|fail> <title>
  local want=$1 title=$2 got
  if bash "$check" "$title" >/dev/null 2>&1; then got=pass; else got=fail; fi
  if [ "$got" = "$want" ]; then
    echo "ok   $want: $title"
  else
    echo "FAIL wanted $want, got $got: $title"
    failures=$((failures + 1))
  fi
}

expect pass "feat: snooze a reminder"
expect pass "fix(tray): keep the dropdown under the icon"
expect pass "feat!: drop the old data format"
expect pass "refactor(core)!: rename the store"
expect pass "chore(deps): bump the npm-minor-patch group across 1 directory with 3 updates"
expect pass "chore(deps-dev): bump vitest from 3.1.0 to 3.2.0"
expect pass "ci(deps): bump actions/checkout from 4.4.0 to 4.5.0"
expect pass "docs: explain the release workflow"
expect pass "revert: feat: snooze a reminder"
expect pass "build(src-tauri/icons): new tray icon"
expect fail "Snooze a reminder"
expect fail "Feat: snooze a reminder"
expect fail "feat:snooze a reminder"
expect fail "feat: "
expect fail "feat:  "
expect fail "feature: snooze a reminder"
expect fail "feat(): snooze a reminder"
expect fail "feat(Tray): snooze a reminder"
expect pass "Revert \"feat: snooze a reminder\""
expect fail "Revert \"\""
expect fail "Revert feat: snooze a reminder"
expect fail "Bump vitest from 3.1.0 to 3.2.0"
expect fail "WIP"
expect fail ""
expect fail "$(printf 'feat: two\nlines')"

if [ "$failures" -ne 0 ]; then
  echo "test-pr-title-check: $failures failed" >&2
  exit 1
fi
echo "test-pr-title-check: all passed"
