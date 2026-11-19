# Contributing to Friends

Thanks for helping. Friends is a small macOS menu bar app, so changes should stay small and
focused too.

## Set up

You need macOS, [Node.js](https://nodejs.org/) 22 or newer, and [Rust](https://rustup.rs/)
(stable).

```sh
npm install
npm run tauri dev      # the real app, in the menu bar
npm run dev:web        # the UI in a browser, against a scratch data file
                       # (FRIENDS_DEV_ALLOWED_HOSTS=<host> to view it through a tunnel)
```

## How the code is laid out

- `crates/friends-core/`: all the rules (scheduling, the data file, the commands). Plain Rust,
  no macOS code, fully unit-tested.
- `src-tauri/`: the macOS shell: menu bar icon, windows, notifications, open at login.
- `src/`: the React UI. It only shows data and calls commands.
- `crates/friends-devserver/` and `src/dev/`: the browser dev mode and the end-to-end tests run
  the real rules through a small local server.
- `e2e/`: Playwright end-to-end tests.

## Before you open a PR

```sh
npm run check          # types, lint, unit tests, Rust format, clippy and tests
npm run e2e            # end-to-end tests in WebKit
npm run privacy        # no personal info in files or commits
```

CI runs all three on every PR.

- **Tests first** for a change to the scheduling rules: add a failing test in
  `crates/friends-core/tests/`, then make it pass.
- **Conventional commits** for commit messages and PR titles: `feat: …`, `fix: …`,
  `docs: …`, `test: …`, `ci: …`, `refactor: …`, `chore: …`.
- **No personal info.** This repo is public. Use your GitHub noreply address as your commit
  email, and keep names, emails and home-folder paths out of code, tests and screenshots (use
  made-up people).
- **One change per PR**, with a short description of what and why, and a screenshot for
  anything you can see.

## Releases

Maintainers release by bumping the version in `package.json`, `src-tauri/tauri.conf.json` and
`src-tauri/Cargo.toml`, moving the notes under a new heading in `CHANGELOG.md`, and pushing a
`vX.Y.Z` tag. The release workflow builds the app and publishes it.

## Code of conduct

Everyone taking part agrees to the [Code of Conduct](CODE_OF_CONDUCT.md).
