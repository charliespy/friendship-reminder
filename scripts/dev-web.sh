#!/usr/bin/env bash
# Browser dev mode: the UI in a plain browser, against the real rules in friends-devserver, over a
# scratch data file (never your real data). Open http://localhost:1420/ (the dropdown) and
# http://localhost:1420/preferences.html.
#
#   npm run dev:web                          data in target/dev-web/friends.json
#   FRIENDS_DATA_FILE=x.json npm run dev:web another scratch file
#   FRIENDS_TODAY=2026-12-05 npm run dev:web pretend it's that day
#   FRIENDS_DEVSERVER_PORT=1430              the devserver's port (default 1430)
set -euo pipefail
cd "$(dirname "$0")/.."

port="${FRIENDS_DEVSERVER_PORT:-1430}"
data="${FRIENDS_DATA_FILE:-target/dev-web/friends.json}"
mkdir -p "$(dirname "$data")"

args=(--port "$port" --data "$data")
[ -n "${FRIENDS_TODAY:-}" ] && args+=(--today "$FRIENDS_TODAY")

cargo build -q -p friends-devserver
target/debug/friends-devserver "${args[@]}" &
server=$!
trap 'kill "$server" 2>/dev/null || true' EXIT INT TERM

FRIENDS_DEVSERVER_PORT="$port" VITE_BROWSER_BRIDGE=1 npx vite
