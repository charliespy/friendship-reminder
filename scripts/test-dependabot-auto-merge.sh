#!/usr/bin/env bash
# Tests scripts/dependabot-auto-merge.sh with a stand-in `gh` that records how it was called: which
# Dependabot PRs it turns auto-merge on for, and which it leaves for a maintainer (turning off
# auto-merge that an earlier push turned on).
#   scripts/test-dependabot-auto-merge.sh
set -euo pipefail
script="$(cd "$(dirname "$0")" && pwd)/dependabot-auto-merge.sh"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/bin"
# `gh pr view` answers whether auto-merge is on ($GH_AUTO); every other call is recorded.
cat >"$work/bin/gh" <<'EOF'
#!/usr/bin/env bash
if [ "$1 $2" = "pr view" ]; then echo "${GH_AUTO:-false}"; exit 0; fi
printf '%s\n' "$*" >>"$GH_CALLS"
EOF
chmod +x "$work/bin/gh"
failures=0
url=https://github.com/owner/repo/pull/7
sha=0123456789abcdef0123456789abcdef01234567

one='[{"dependencyName":"vite","prevVersion":"7.1.2","newVersion":"7.2.0"}]'

# expect <name> <expected gh call, or "none"> <ecosystem> <update type> [packages json] [DRY_RUN]
# Auto-merge is already on (GH_AUTO=true, as after an earlier push) unless the caller says otherwise,
# so every PR that waits must turn it off.
expect() {
  local name=$1 want=$2 got
  : >"$work/calls"
  if ! PATH="$work/bin:$PATH" GH_CALLS="$work/calls" GH_AUTO=${GH_AUTO-true} GITHUB_STEP_SUMMARY="" \
    PR_URL=$url HEAD_SHA=$sha ECOSYSTEM=$3 UPDATE_TYPE=$4 UPDATED_DEPENDENCIES=${5-$one} DRY_RUN=${6-} \
    bash "$script" ${ARGS:+"$ARGS"} >"$work/out" 2>&1; then
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
off="pr merge $url --disable-auto"
expect "npm patch merges" "$on" npm_and_yarn version-update:semver-patch
expect "npm minor merges" "$on" npm_and_yarn version-update:semver-minor
expect "cargo patch merges" "$on" cargo version-update:semver-patch
expect "cargo minor merges" "$on" cargo version-update:semver-minor
expect "npm major waits" "$off" npm_and_yarn version-update:semver-major
expect "cargo major waits" "$off" cargo version-update:semver-major
expect "GitHub Actions patch waits" "$off" github_actions version-update:semver-patch
expect "unknown update type waits" "$off" npm_and_yarn ""
expect "unknown ecosystem waits" "$off" pip version-update:semver-patch
expect "dry run changes nothing" none cargo version-update:semver-minor "$one" 1
group='[{"dependencyName":"serde","prevVersion":"1.0.219","newVersion":"1.0.228"},{"dependencyName":"tauri","prevVersion":"2.8.5","newVersion":"2.9.0"}]'
expect "a group of safe updates merges" "$on" cargo version-update:semver-minor "$group"
expect "0.x middle number changes waits" "$off" cargo version-update:semver-minor \
  '[{"dependencyName":"serde","prevVersion":"1.0.219","newVersion":"1.0.228"},{"dependencyName":"chrono","prevVersion":"0.4.41","newVersion":"0.5.0"}]'
expect "0.x patch merges" "$on" cargo version-update:semver-patch \
  '[{"dependencyName":"chrono","prevVersion":"0.4.41","newVersion":"0.4.42"}]'
expect "0.0.x patch waits" "$off" npm_and_yarn version-update:semver-patch \
  '[{"dependencyName":"tiny","prevVersion":"0.0.3","newVersion":"0.0.4"}]'
expect "a v-prefixed 0.x version waits" "$off" npm_and_yarn version-update:semver-minor \
  '[{"dependencyName":"x","prevVersion":"v0.4.1","newVersion":"v0.5.0"}]'
expect "a package without its old version waits" "$off" cargo version-update:semver-patch \
  '[{"dependencyName":"serde","newVersion":"1.0.228"}]'
expect "no package list waits" "$off" cargo version-update:semver-patch ""
expect "an empty package list waits" "$off" cargo version-update:semver-patch "[]"
expect "unreadable package list waits" "$off" cargo version-update:semver-patch "not json"
expect "a version range waits" "$off" cargo version-update:semver-minor \
  '[{"dependencyName":"chrono","prevVersion":"^0.4.1","newVersion":"^0.5.0"}]'
expect "a commit-hash version waits" "$off" cargo version-update:semver-minor \
  '[{"dependencyName":"git-dep","prevVersion":"abc1234","newVersion":"def5678"}]'
GH_AUTO=false expect "a waiting PR with auto-merge off: no call" none cargo version-update:semver-major
ARGS=--disable expect "--disable turns auto-merge off" "$off" cargo version-update:semver-patch
GH_AUTO=false ARGS=--disable expect "--disable with auto-merge off: no call" none "" ""

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
