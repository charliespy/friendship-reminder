#!/usr/bin/env bash
# Tests scripts/dependabot-auto-merge.sh with a stand-in `gh` that only records how it was called:
# which Dependabot PRs it turns auto-merge on for, and which it leaves for a maintainer.
#   scripts/test-dependabot-auto-merge.sh
set -euo pipefail
script="$(cd "$(dirname "$0")" && pwd)/dependabot-auto-merge.sh"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/bin"
cat >"$work/bin/gh" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$*" >>"$GH_CALLS"
EOF
chmod +x "$work/bin/gh"
failures=0
url=https://github.com/owner/repo/pull/7
sha=0123456789abcdef0123456789abcdef01234567

# expect <name> <expected gh call, or "none"> <ecosystem> <update type> [DRY_RUN]
expect() {
  local name=$1 want=$2 got
  : >"$work/calls"
  if ! PATH="$work/bin:$PATH" GH_CALLS="$work/calls" GITHUB_STEP_SUMMARY="" PR_URL=$url HEAD_SHA=$sha \
    ECOSYSTEM=$3 UPDATE_TYPE=$4 DRY_RUN=${5-} bash "$script" >"$work/out" 2>&1; then
    echo "FAIL $name: the script failed"
    sed 's/^/     /' "$work/out"
    failures=$((failures + 1))
    return
  fi
  got=$(cat "$work/calls")
  [ -n "$got" ] || got=none
  if [ "$got" = "$want" ]; then
    echo "ok   $name"
  else
    echo "FAIL $name: wanted gh call '$want', got '$got'"
    failures=$((failures + 1))
  fi
}

on="pr merge $url --auto --squash --match-head-commit $sha"
expect "npm patch merges" "$on" npm_and_yarn version-update:semver-patch
expect "npm minor merges" "$on" npm_and_yarn version-update:semver-minor
expect "cargo patch merges" "$on" cargo version-update:semver-patch
expect "cargo minor merges" "$on" cargo version-update:semver-minor
expect "npm major waits" none npm_and_yarn version-update:semver-major
expect "cargo major waits" none cargo version-update:semver-major
expect "GitHub Actions patch waits" none github_actions version-update:semver-patch
expect "unknown update type waits" none npm_and_yarn ""
expect "unknown ecosystem waits" none pip version-update:semver-patch
expect "dry run changes nothing" none cargo version-update:semver-minor 1

# Without the PR or its head commit it must fail, not guess.
if PATH="$work/bin:$PATH" GH_CALLS="$work/calls" ECOSYSTEM=cargo UPDATE_TYPE=version-update:semver-patch \
  PR_URL=$url HEAD_SHA="" bash "$script" >/dev/null 2>&1; then
  echo "FAIL a missing HEAD_SHA must fail"
  failures=$((failures + 1))
else
  echo "ok   a missing HEAD_SHA fails"
fi

if [ "$failures" -ne 0 ]; then
  echo "test-dependabot-auto-merge: $failures failed" >&2
  exit 1
fi
echo "test-dependabot-auto-merge: all passed"
