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
#
# Merges: minor and patch updates of npm and Cargo packages.
# Waits for a maintainer: major updates; a 0.x package whose middle number changes (0.4 -> 0.5),
# or a 0.0.x one whose last number does, because npm and Cargo treat those as breaking although
# they're reported as minor or patch; anything it can't read or classify; and GitHub Actions
# updates (they change .github/workflows/, which the workflow's own token isn't allowed to merge).
set -euo pipefail

update_type=${UPDATE_TYPE-}
ecosystem=${ECOSYSTEM-}
: "${PR_URL:?PR_URL is required}"
: "${HEAD_SHA:?HEAD_SHA is required}"

say() {
  echo "dependabot-auto-merge: $1"
  if [ -n "${GITHUB_STEP_SUMMARY-}" ]; then echo "$1" >>"$GITHUB_STEP_SUMMARY"; fi
}

case "$ecosystem" in
  npm_and_yarn | npm | cargo) ;;
  github_actions | github-actions)
    say "Waits for a maintainer: GitHub Actions updates change workflow files, which this workflow's token can't merge."
    exit 0
    ;;
  *)
    say "Waits for a maintainer: unknown ecosystem '$ecosystem'."
    exit 0
    ;;
esac

case "$update_type" in
  version-update:semver-minor | version-update:semver-patch) ;;
  version-update:semver-major)
    say "Waits for a maintainer: a major update."
    exit 0
    ;;
  *)
    say "Waits for a maintainer: unknown update type '$update_type'."
    exit 0
    ;;
esac

# Each package's old and new version, as "<name> <old> <new>" lines; a package without both versions
# can't be checked, so it makes the whole PR wait.
if ! deps=$(printf '%s' "${UPDATED_DEPENDENCIES-}" | jq -er '
    if type == "array" and length > 0 then .[] else error("no packages") end
    | "\(.dependencyName // "?") \(.prevVersion // "") \(.newVersion // "")"' 2>/dev/null); then
  say "Waits for a maintainer: can't read which packages are updated."
  exit 0
fi
while read -r name old new; do
  if [ -z "$old" ] || [ -z "$new" ]; then
    say "Waits for a maintainer: no old and new version for $name."
    exit 0
  fi
  IFS=. read -r old_major old_minor old_patch _ <<<"${old#v}"
  IFS=. read -r _ new_minor new_patch _ <<<"${new#v}"
  if [ "$old_major" = 0 ] && { [ "$old_minor" != "$new_minor" ] ||
    { [ "$old_minor" = 0 ] && [ "$old_patch" != "$new_patch" ]; }; }; then
    say "Waits for a maintainer: $name $old -> $new is a breaking change for a 0.x package."
    exit 0
  fi
done <<<"$deps"

if [ "${DRY_RUN-}" = 1 ]; then
  say "Dry run: would turn on auto-merge (squash) for $PR_URL at $HEAD_SHA ($ecosystem, $update_type)."
  exit 0
fi
gh pr merge "$PR_URL" --auto --squash --match-head-commit "$HEAD_SHA"
say "Auto-merge is on for $PR_URL ($ecosystem, $update_type): it merges once every required check passes."
