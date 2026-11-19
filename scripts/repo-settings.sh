#!/usr/bin/env bash
# The repo's GitHub settings, kept as code. A maintainer with admin rights runs it with their own
# `gh` login; CI never does.
#
#   scripts/repo-settings.sh apply        protect main with .github/rulesets/main.json (create or
#                                         update the "main" ruleset), turn on auto-merge, squash
#                                         merges and deleting merged branches; then `check`
#   scripts/repo-settings.sh check        read-only: fails unless GitHub matches this repo's files
#   scripts/repo-settings.sh verify-push  try a direct push of an empty commit to main; passes only
#                                         if GitHub refuses it
#
# What the ruleset does on main: every change comes through a PR, the required checks must pass,
# no force-pushes, no deleting the branch, and no approving review needed. Admins may bypass it,
# but only when merging a PR (an emergency fix whose checks can't pass); a direct push is refused
# for everyone.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

ruleset_file=.github/rulesets/main.json
repo=${REPO:-$(gh repo view --json nameWithOwner -q .nameWithOwner)}
name=$(jq -r .name "$ruleset_file")

ruleset_id() {
  gh api "repos/$repo/rulesets" --jq ".[] | select(.name == \"$name\" and .source_type == \"Repository\") | .id" | head -n1
}

apply() {
  # Auto-merge (Dependabot's minor and patch updates use it), squash merges, delete merged branches.
  gh api -X PATCH "repos/$repo" -F allow_auto_merge=true -F allow_squash_merge=true \
    -F delete_branch_on_merge=true --silent
  local id
  id=$(ruleset_id)
  if [ -n "$id" ]; then
    gh api -X PUT "repos/$repo/rulesets/$id" --input "$ruleset_file" --silent
    echo "repo-settings: updated ruleset \"$name\" ($id)"
  else
    gh api -X POST "repos/$repo/rulesets" --input "$ruleset_file" --silent
    echo "repo-settings: created ruleset \"$name\""
  fi
  check
}

check() {
  local fail=0 id live want
  settings=$(gh api "repos/$repo" --jq '[.allow_auto_merge, .allow_squash_merge, .delete_branch_on_merge] | map(tostring) | join(" ")')
  if [ "$settings" != "true true true" ]; then
    echo "repo-settings: auto-merge, squash merges and deleting merged branches must all be on (got: $settings)" >&2
    fail=1
  fi
  id=$(ruleset_id)
  if [ -z "$id" ]; then
    echo "repo-settings: no ruleset named \"$name\"; run: scripts/repo-settings.sh apply" >&2
    exit 1
  fi
  live=$(gh api "repos/$repo/rulesets/$id")
  # Every field and rule in the file must be there, with the same values. GitHub may add fields of
  # its own with default values; those don't count. Arrays are compared as sets.
  want=$(jq -c '{target, enforcement, conditions, bypass_actors, rules}' "$ruleset_file")
  if ! jq -e --argjson want "$want" '
      def norm: walk(if type == "array" then sort else . end);
      ({target, enforcement, conditions, bypass_actors} | norm) as $l
      | ($want | {target, enforcement, conditions, bypass_actors} | norm) as $w
      | ($l | contains($w)) and ($w.bypass_actors | length) == ($l.bypass_actors | length)
        and ([$want.rules[] as $r | any(.rules[]; (. | norm) | contains($r | norm))] | all)
        and ((.rules | map(.type) | sort) == ($want.rules | map(.type) | sort))
        and ([.rules[] | select(.type == "required_status_checks") | .parameters.required_status_checks[].context] | sort)
          == ([$want.rules[] | select(.type == "required_status_checks") | .parameters.required_status_checks[].context] | sort)
    ' <<<"$live" >/dev/null; then
    echo "repo-settings: ruleset \"$name\" ($id) differs from $ruleset_file; run: scripts/repo-settings.sh apply" >&2
    fail=1
  fi
  # And GitHub applies it to main.
  types=$(gh api "repos/$repo/rules/branches/main" --jq '[.[] | select(.ruleset_id == '"$id"') | .type] | sort | join(" ")')
  if [ "$types" != "deletion non_fast_forward pull_request required_status_checks" ]; then
    echo "repo-settings: main doesn't get the ruleset's rules (got: ${types:-none})" >&2
    fail=1
  fi
  if [ "$fail" -ne 0 ]; then exit 1; fi
  echo "repo-settings: GitHub matches $ruleset_file and the merge settings"
}

verify_push() {
  git fetch --quiet origin main
  local sha out
  # An empty commit on top of main: if GitHub ever accepted it, main would only gain a no-op commit.
  sha=$(git commit-tree "origin/main^{tree}" -p origin/main -m "test: direct push to main (must be refused)")
  if out=$(git push origin "$sha:refs/heads/main" 2>&1); then
    echo "$out" >&2
    echo "repo-settings: GitHub ACCEPTED a direct push to main ($sha): main is not protected" >&2
    exit 1
  fi
  # shellcheck disable=SC2001 # indent every line of git's output
  echo "$out" | sed 's/^/  /'
  if echo "$out" | grep -qiE 'rule violations|protected branch|GH013'; then
    echo "repo-settings: a direct push to main was refused by the ruleset, as it should be"
  else
    echo "repo-settings: the push failed, but not because of the ruleset (see above)" >&2
    exit 1
  fi
}

case "${1-}" in
  apply) apply ;;
  check) check ;;
  verify-push) verify_push ;;
  *)
    echo "usage: scripts/repo-settings.sh apply | check | verify-push" >&2
    exit 2
    ;;
esac
