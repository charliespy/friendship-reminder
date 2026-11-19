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

# Ask cargo where it put the binary (CARGO_TARGET_DIR or a cargo config may move it).
bin=$(cargo build -q -p friends-devserver --message-format=json-render-diagnostics |
  node -e 'let p; require("readline").createInterface({ input: process.stdin }).on("line", (l) => {
    const m = JSON.parse(l); if (m.reason === "compiler-artifact" && m.executable && m.target.name === "friends-devserver") p = m.executable;
  }).on("close", () => { if (!p) process.exit(1); console.log(p); });')
"$bin" "${args[@]}" &
server=$!
trap 'kill "$server" 2>/dev/null || true' EXIT INT TERM

# Start Vite only once the devserver answers (it exits at once if the port is taken).
for _ in $(seq 1 50); do
  kill -0 "$server" 2>/dev/null || { echo "dev-web: friends-devserver didn't start (is port $port in use?)" >&2; exit 1; }
  if curl -fs "http://127.0.0.1:$port/__test/health" >/dev/null; then ready=1; break; fi
  sleep 0.2
done
[ -n "${ready:-}" ] || { echo "dev-web: friends-devserver isn't answering on port $port" >&2; exit 1; }

FRIENDS_DEVSERVER_PORT="$port" VITE_BROWSER_BRIDGE=1 npx vite
