#!/usr/bin/env bash
# Decides whether a Dependabot PR may merge by itself, and if so turns on GitHub's auto-merge for it:
# GitHub then squash-merges it once every required check on main passes, and never before.
# .github/workflows/dependabot-auto-merge.yml runs it with what dependabot/fetch-metadata found.
#
#   UPDATE_TYPE   the highest update in the PR: version-update:semver-major|minor|patch
#   UPDATED_DEPENDENCIES  fetch-metadata's updated-dependencies-json: each package's old and new
#                 version
#   ECOSYSTEM     npm_and_yarn, cargo or github_actions (fetch-metadata's package-ecosystem)
#   PR_URL        the PR
#   HEAD_SHA      the commit the decision is about; auto-merge is tied to it
#   DRY_RUN=1     print what it would do, change nothing
#   --disable     only turn auto-merge off (the workflow's fallback when a step fails)
#
# Merges: minor and patch updates of npm and Cargo packages.
# Waits for a maintainer: major updates; a 0.x package whose middle number changes (0.4 -> 0.5),
# or a 0.0.x one whose last number does, because npm and Cargo treat those as breaking although
# they're reported as minor or patch; anything it can't read or classify; and GitHub Actions
# updates (they change .github/workflows/, which the workflow's own token isn't allowed to merge).
# Whenever a PR waits, auto-merge is turned off: Dependabot pushes newer versions onto an open PR,
# so a PR cleared for one push may need to wait after the next.
set -euo pipefail

update_type=${UPDATE_TYPE-}
ecosystem=${ECOSYSTEM-}
: "${PR_URL:?PR_URL is required}"
: "${HEAD_SHA:?HEAD_SHA is required}"

say() {
  echo "dependabot-auto-merge: $1"
  if [ -n "${GITHUB_STEP_SUMMARY-}" ]; then echo "$1" >>"$GITHUB_STEP_SUMMARY"; fi
}

# Turns auto-merge off if an earlier push turned it on. A failure here fails the job, so the PR
# can't merge by itself without anyone noticing.
disable() {
  if [ "${DRY_RUN-}" = 1 ]; then
    say "Dry run: would make sure auto-merge is off for $PR_URL."
    return
  fi
  local on
  # A plain assignment, so a failing `gh pr view` stops the script (inside `[ "$(…)" ]` it wouldn't).
  on=$(gh pr view "$PR_URL" --json autoMergeRequest --jq '.autoMergeRequest != null')
  if [ "$on" = true ]; then
    gh pr merge "$PR_URL" --disable-auto
    say "Auto-merge turned off for $PR_URL."
  fi
}

wait_for_maintainer() {
  say "Waits for a maintainer: $1"
  disable
  exit 0
}

if [ "${1-}" = --disable ]; then
  disable
  exit 0
fi

case "$ecosystem" in
  npm_and_yarn | npm | cargo) ;;
  github_actions | github-actions)
    wait_for_maintainer "GitHub Actions updates change workflow files, which this workflow's token can't merge."
    ;;
  *) wait_for_maintainer "unknown ecosystem '$ecosystem'." ;;
esac

case "$update_type" in
  version-update:semver-minor | version-update:semver-patch) ;;
  version-update:semver-major) wait_for_maintainer "a major update." ;;
  *) wait_for_maintainer "unknown update type '$update_type'." ;;
esac

# Each package's old and new version, as "<name> <old> <new>" lines; a package without both versions
# can't be checked, so it makes the whole PR wait.
if ! deps=$(printf '%s' "${UPDATED_DEPENDENCIES-}" | jq -er '
    if type == "array" and length > 0 then .[] else error("no packages") end
    | "\(.dependencyName // "?") \(.prevVersion // "") \(.newVersion // "")"' 2>/dev/null); then
  wait_for_maintainer "can't read which packages are updated."
fi
while read -r name old new; do
  old=${old#v}
  new=${new#v}
  # Plain version numbers only: a range (^0.4.1), a commit hash or nothing at all can't be judged.
  if ! [[ $old =~ ^[0-9]+\.[0-9]+ ]] || ! [[ $new =~ ^[0-9]+\.[0-9]+ ]]; then
    wait_for_maintainer "can't read the versions of $name ('$old' -> '$new')."
  fi
  IFS=. read -r old_major old_minor old_patch _ <<<"$old"
  IFS=. read -r _ new_minor new_patch _ <<<"$new"
  if [ "$old_major" = 0 ] && { [ "$old_minor" != "$new_minor" ] ||
    { [ "$old_minor" = 0 ] && [ "$old_patch" != "$new_patch" ]; }; }; then
    wait_for_maintainer "$name $old -> $new is a breaking change for a 0.x package."
  fi
done <<<"$deps"

if [ "${DRY_RUN-}" = 1 ]; then
  say "Dry run: would turn on auto-merge (squash) for $PR_URL at $HEAD_SHA ($ecosystem, $update_type)."
  exit 0
fi
gh pr merge "$PR_URL" --auto --squash --match-head-commit "$HEAD_SHA"
say "Auto-merge is on for $PR_URL ($ecosystem, $update_type): it merges once every required check passes."
