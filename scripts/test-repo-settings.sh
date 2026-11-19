#!/usr/bin/env bash
# Tests `scripts/repo-settings.sh check` against a stand-in `gh` that answers with made-up GitHub
# responses: it must pass when GitHub matches .github/rulesets/main.json, and fail when it doesn't.
#   scripts/test-repo-settings.sh
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir "$work/bin" "$work/api"
# The stand-in: `gh api <path> [--jq <expr>]` prints $work/api/<path with / as _>.json.
cat >"$work/bin/gh" <<'EOF'
#!/usr/bin/env bash
[ "$1" = api ] || { echo "stand-in gh: only 'api' is supported" >&2; exit 1; }
path=$2; shift 2
file="$GH_API/$(printf '%s' "$path" | tr '/' '_').json"
[ -f "$file" ] || { echo "stand-in gh: no answer for $path" >&2; exit 1; }
if [ "${1-}" = --jq ]; then jq -r "$2" "$file"; else cat "$file"; fi
EOF
chmod +x "$work/bin/gh"
failures=0
a=$work/api
r=repos_owner_repo

# A GitHub that matches: the file as GitHub returns it, with fields of its own added.
good() {
  echo '{"allow_auto_merge":true,"allow_squash_merge":true,"delete_branch_on_merge":true}' >"$a/$r.json"
  echo '[{"id":7,"name":"main","source_type":"Repository"},{"id":8,"name":"main","source_type":"Organization"}]' >"$a/${r}_rulesets.json"
  jq '. + {id: 7, source_type: "Repository", current_user_can_bypass: "pull_requests_only"}
      | .rules |= map(if .type == "pull_request" then .parameters += {automatic_copilot_code_review_enabled: false} else . end)
      | .rules |= reverse' "$root/.github/rulesets/main.json" >"$a/${r}_rulesets_7.json"
  jq '[.rules[] | {type, ruleset_id: 7, ruleset_source_type: "Repository"}]' "$root/.github/rulesets/main.json" >"$a/${r}_rules_branches_main.json"
}
edit() { # edit <file> <jq filter>
  jq "$2" "$a/$1" >"$work/tmp" && mv "$work/tmp" "$a/$1"
}
expect() { # expect <pass|fail> <name>
  local got
  if (cd "$root" && PATH="$work/bin:$PATH" GH_API="$a" REPO=owner/repo bash scripts/repo-settings.sh check) >"$work/out" 2>&1; then got=pass; else got=fail; fi
  if [ "$got" = "$1" ]; then
    echo "ok   $2"
  else
    echo "FAIL $2 (wanted $1, got $got)"
    sed 's/^/     /' "$work/out"
    failures=$((failures + 1))
  fi
}

good
expect pass "GitHub matches the file"
good; edit "$r.json" '.allow_auto_merge = false'
expect fail "auto-merge off"
good; echo '[]' >"$a/${r}_rulesets.json"
expect fail "no ruleset"
good; edit "${r}_rulesets_7.json" '.enforcement = "evaluate"'
expect fail "ruleset only evaluated"
good; edit "${r}_rulesets_7.json" '.bypass_actors[0].bypass_mode = "always"'
expect fail "admins may bypass on a direct push"
good; edit "${r}_rulesets_7.json" '.bypass_actors += [{actor_id: 1, actor_type: "Integration", bypass_mode: "always"}]'
expect fail "an extra bypass actor"
good; edit "${r}_rulesets_7.json" '.rules |= map(select(.type != "non_fast_forward"))'
expect fail "force-pushes allowed"
good; edit "${r}_rulesets_7.json" '.rules |= map(if .type == "pull_request" then .parameters.required_approving_review_count = 1 else . end)'
expect fail "a review required"
good; edit "${r}_rulesets_7.json" '.rules |= map(if .type == "required_status_checks" then .parameters.required_status_checks |= .[1:] else . end)'
expect fail "a required check missing"
good; edit "${r}_rulesets_7.json" '.rules |= map(if .type == "required_status_checks" then .parameters.required_status_checks += [{context: "Other"}] else . end)'
expect fail "an extra required check"
good; edit "${r}_rules_branches_main.json" 'map(select(.type != "pull_request"))'
expect fail "main doesn't get every rule"

if [ "$failures" -ne 0 ]; then
  echo "test-repo-settings: $failures failed" >&2
  exit 1
fi
echo "test-repo-settings: all passed"
