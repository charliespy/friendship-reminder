# Notes for coding agents

Friends is a macOS menu bar app (Tauri v2: Rust + React/TypeScript) that reminds you to stay in
touch with people. Read `README.md` and `CONTRIBUTING.md` first; this file adds what an agent
needs. `CLAUDE.md` and `AGENTS.md` hold the same text: change both together.

## Rules

- **This repo is public.** Never commit personal info: real names, emails, phone numbers,
  home-folder paths (`/Users/<name>`), private host names. Use made-up people in tests and
  screenshots. `npm run privacy` checks files and commits; CI runs it.
- **No AI attribution** anywhere: no co-author trailers naming an AI tool or agent, and no
  "made with" lines, in commits, PRs, issues or comments. `npm run privacy` checks commits.
- **Conventional commits** for commit messages and PR titles.
- **Logic lives in `crates/friends-core`**, never in the UI or the Tauri shell. The UI calls
  commands through `src/api.ts`; the shell only does macOS things (tray, windows,
  notifications, open at login).
- **Dates are calendar dates** (`chrono::NaiveDate`, `"YYYY-MM-DD"` in JSON). Never store a time
  for a reminder: time zones and daylight saving would shift it.
- **Scheduling changes are test-first**: a failing test in `crates/friends-core/tests/`, then the
  fix.
- macOS file names ignore case: `src/dropdown.tsx` and `src/Dropdown.tsx` are the same file.

## Checks

```sh
npm run check     # typecheck, eslint, vitest, vite build, cargo fmt/clippy/test
npm run e2e       # Playwright (WebKit) against the browser dev mode
npm run privacy   # personal-info scan
```

`main` is protected (`.github/rulesets/main.json`, applied with `scripts/repo-settings.sh`):
changes reach it only through PRs whose required checks pass: `npm run check`, `npm run e2e`,
the privacy scan of files, commits and PR text, and a conventional-commit PR title
(`scripts/pr-title-check.sh`). Never push to `main`, and never merge with `gh pr merge --admin`:
that bypass is for the owner's emergencies; wait for the checks or use `--auto`. Dependabot's minor and patch npm/Cargo PRs
merge themselves once those checks pass (`.github/workflows/dependabot-auto-merge.yml`); keep
`scripts/test-*.sh` passing (`npm run check:scripts`) when you change these scripts.

Ports for local runs: `FRIENDS_E2E_SERVER_PORT` / `FRIENDS_E2E_WEB_PORT` 
for e2e (defaults 1440/1441, in `e2e/ports.ts`), `FRIENDS_DEVSERVER_PORT` for `npm run dev:web`.

## Debug-only overrides

Debug builds of the app read `FRIENDS_TODAY=YYYY-MM-DD` (pretend it's that day) and
`FRIENDS_DATA_FILE=<path>` (use another data file). Release builds ignore both. Turn
notifications off in a scratch data file before launching a debug build, or it will send real
notifications.
