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

Releases happen only when a maintainer decides to release; merging a PR doesn't release anything.
[docs/RELEASING.md](docs/RELEASING.md) has the step-by-step, and `npm run release:plan` shows what
the next release would be without changing anything.

**Version numbers** follow [Semantic Versioning](https://semver.org/) and come from the
conventional commits (one per merged PR) since the last release:

| Since the last release | Next version |
| --- | --- |
| a breaking change (`feat!: …`, `fix!: …`, or a `BREAKING CHANGE:` line) | major: 1.4.2 → 2.0.0 |
| a `feat` | minor: 1.4.2 → 1.5.0 |
| only `fix` or `perf` | patch: 1.4.2 → 1.4.3 |
| only `docs`, `ci`, `chore`, `test`, `refactor`, `build`, `style`, `revert` | no release needed |

While the version is `0.x`, a breaking change bumps the minor (0.4.2 → 0.5.0). The first release
is `v0.1.0`.

**Release notes**: each release's section in [CHANGELOG.md](CHANGELOG.md) is also its description
on GitHub Releases, word for word. It is short and in plain words, under **New**, **Fixed**,
**Changed** and **Good to know**. You may add a line under `## [Unreleased]` in your PR for a change
users will notice; the person releasing turns it, and the PR titles since the last release, into the
notes.

**Tags**: only `vX.Y.Z` release tags are pushed to this repository. Pushing one runs the release
workflow, which builds the app and publishes the release.

## Code of conduct

Everyone taking part agrees to the [Code of Conduct](CODE_OF_CONDUCT.md).
