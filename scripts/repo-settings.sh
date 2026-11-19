#!/usr/bin/env bash
# The repo's GitHub settings, kept as code. A maintainer with admin rights runs it with their own
# `gh` login; CI never does.
#
#   scripts/repo-settings.sh apply        protect main with .github/rulesets/main.json (create or
#                                         update the "main" ruleset), turn on auto-merge, squash
#                                         merges titled by the PR title with the commits as the
#                                         message, and deleting merged branches; then `check`
#   scripts/repo-settings.sh check        read-only: fails unless GitHub matches this repo's files
#   scripts/repo-settings.sh verify-push  try a direct push of an empty commit to main; passes only
#                                         if GitHub refuses it
#
# What the ruleset does on main: every change comes through a PR, squash-merged (so the PR title,
# which must be a conventional commit, is the commit on main), the required checks must pass, no
# force-pushes, no deleting the branch, and no approving review needed. Merge commits stay allowed
# repo-wide, for PRs into other branches. Admins may bypass it,
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
  # Auto-merge (Dependabot's minor and patch updates use it), squash merges named after the PR
  # title, delete merged branches.
  gh api -X PATCH "repos/$repo" -F allow_auto_merge=true -F allow_squash_merge=true \
    -F delete_branch_on_merge=true -f squash_merge_commit_title=PR_TITLE \
    -f squash_merge_commit_message=COMMIT_MESSAGES --silent
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
  settings=$(gh api "repos/$repo" --jq '[.allow_auto_merge, .allow_squash_merge, .delete_branch_on_merge, .squash_merge_commit_title, .squash_merge_commit_message] | map(tostring) | join(" ")')
  if [ "$settings" != "true true true PR_TITLE COMMIT_MESSAGES" ]; then
    echo "repo-settings: auto-merge, squash merges, deleting merged branches, squash titles from the PR title and squash messages from the commits must all be on (got: $settings)" >&2
    fail=1
  fi
  id=$(ruleset_id)
  if [ -z "$id" ]; then
    echo "repo-settings: no ruleset named \"$name\"; run: scripts/repo-settings.sh apply" >&2
    exit 1
  fi
  live=$(gh api "repos/$repo/rulesets/$id")
  # GitHub's copy must have exactly the file's fields and rules, with the same values. Fields GitHub
  # adds on its own (ids, links, new parameters with defaults) are ignored; every field the file
  # names is compared exactly, with arrays compared in any order.
  want=$(jq -c '{target, enforcement, conditions, bypass_actors, rules}' "$ruleset_file")
  if ! jq -e --argjson want "$want" '
      def norm: walk(if type == "array" then sort else . end);
      # $live restricted to the keys $want has, at every level of objects.
      def like($w):
        if ($w | type) == "object" and type == "object"
        then with_entries(select(.key as $k | $w | has($k))) | with_entries(.key as $k | .value |= like($w[$k]))
        elif ($w | type) == "array" and type == "array" and ($w | length) > 0 and ($w[0] | type) == "object"
        then map(like($w[0]))
        else . end;
      (.rules | map({(.type): .}) | add) as $lr
      | ({target, enforcement, conditions, bypass_actors} | like($want) | norm)
          == ($want | {target, enforcement, conditions, bypass_actors} | norm)
        and ((.rules | map(.type) | sort) == ($want.rules | map(.type) | sort))
        and ([$want.rules[] as $r | ($lr[$r.type] | like($r) | norm) == ($r | norm)] | all)
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
  # Made by a neutral noreply identity, so even an accepted push would publish no one's address.
  sha=$(GIT_AUTHOR_NAME=repo-settings GIT_AUTHOR_EMAIL=noreply@github.com \
    GIT_COMMITTER_NAME=repo-settings GIT_COMMITTER_EMAIL=noreply@github.com \
    git commit-tree "origin/main^{tree}" -p origin/main -m "test: direct push to main (must be refused)")
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
