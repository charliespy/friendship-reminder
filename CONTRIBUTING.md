# Contributing to Friendship Reminder

Thanks for helping. Friendship Reminder is a small macOS menu bar app, so changes should stay small and
focused too.

## Set up

You need macOS, [Node.js](https://nodejs.org/) 22 or newer, [Rust](https://rustup.rs/)
(stable), and [jq](https://jqlang.org/) 1.6 or newer (built into macOS 15 and later; otherwise
`brew install jq`) for the script tests in `npm run check` and for `scripts/repo-settings.sh`.

```sh
npm install
npm run tauri dev      # the real app, in the menu bar
npm run dev:web        # the UI in a browser, against a scratch data file
                       # (FRIENDS_DEV_ALLOWED_HOSTS=<host> to view it through a tunnel)
```

## How the code is laid out

- `crates/friends-core/`: all the rules (scheduling, the data file, the commands, and what to
  sync). Plain Rust, no macOS code, fully unit-tested.
- `crates/friends-account/`: signing in to the Friendship Reminder account (Supabase Auth): the requests,
  the session and its rules. Plain Rust, tested without a network.
- `crates/friends-sync/`: syncing the list with the Friendship Reminder account: the requests that upload
  and download it (Supabase's REST API). Plain Rust, tested without a network.
- `supabase/migrations/`: the account's database tables and their privacy rules (each account
  sees only its own rows).
- `src-tauri/`: the shell. `src/desktop.rs` is the Mac's (menu bar icon, windows, notifications,
  open at login), `src/mobile.rs` the iPhone's (booking reminders ahead with iOS), and the rest is
  shared: the account (browser sign-in on the Mac, a sign-in sheet on iPhone from
  `plugins/web-auth`), the Keychain and when to sync. `tauri.ios.conf.json` and `gen/apple/` are
  the iPhone app's settings and Xcode project.
- `src/`: the React UI. It only shows data and calls commands. `Phone.tsx` lays the same panes
  out as the iPhone app's tabs.
- `crates/friends-devserver/` and `src/dev/`: the browser dev mode and the end-to-end tests run
  the real rules through a small local server.
- `e2e/`: Playwright end-to-end tests.

## Before you open a PR

```sh
npm run check          # types, lint, unit tests, Rust format, clippy and tests
npm run e2e            # end-to-end tests in WebKit
npm run privacy        # no personal info in files or commits
```

CI runs all three on every PR, along with a check of the PR title (see below).

- **Tests first** for a change to the scheduling rules: add a failing test in
  `crates/friends-core/tests/`, then make it pass.
- **Conventional commits** for commit messages and PR titles: `feat: …`, `fix: …`,
  `docs: …`, `test: …`, `ci: …`, `refactor: …`, `chore: …`.
- **No personal info.** This repo is public. Use your GitHub noreply address as your commit
  email, and keep names, emails and home-folder paths out of code, tests and screenshots (use
  made-up people).
- **One change per PR**, with a short description of what and why, and a screenshot for
  anything you can see.

## How a PR reaches `main`

`main` is protected by a branch ruleset, kept in [`.github/rulesets/main.json`](.github/rulesets/main.json):

- Every change comes through a pull request, and it is squash-merged, so each PR is one commit
  on `main` titled with the PR's title. Nobody can push to `main` directly, force-push it or
  delete it.
- These checks must pass before a PR can merge:
  - **Check (types, lint, unit tests, Rust)**: `npm run check`.
  - **End-to-end (Playwright, WebKit)**: `npm run e2e`.
  - **Privacy scan**: `npm run privacy` on the PR's files and commits.
  - **PR title and description**: the same scan on the PR's text, which is public too.
  - **PR title (conventional commit)**: `scripts/pr-title-check.sh`. The title becomes the commit
    on `main` when the PR is squash-merged, so it must read like `feat: snooze a reminder` or
    `fix(tray): keep the dropdown under the icon`. Titles from GitHub's Revert button
    (`Revert "feat: …"`) pass too.
- No approving review is required: the project has one maintainer. Admins can bypass the rules
  only when merging a PR, for an emergency fix whose checks can't pass.
- A merged PR's branch is deleted automatically.

Maintainers set this up, or check it, with `scripts/repo-settings.sh` (`apply` creates or
updates the ruleset and turns on auto-merge; `check` compares GitHub with the files and changes
nothing; `verify-push` checks that a direct push to `main` is refused). To change the
protection, edit the JSON in a PR, then run `scripts/repo-settings.sh apply` after it merges.

## Dependency updates

[Dependabot](.github/dependabot.yml) looks for new versions every Monday, for npm packages,
Rust crates and GitHub Actions. It proposes a release only once it is 3 days old.

- Minor and patch updates come as **one PR per ecosystem** (the `minor-and-patch` group). For
  npm and Cargo, [a workflow](.github/workflows/dependabot-auto-merge.yml) turns on auto-merge, so
  the PR squash-merges by itself once every required check passes. A failing check stops it.
  These merges don't start CI's run on `main` (GitHub starts no workflows for them); the PR's
  own checks are what tested them.
- A package still at 0.x whose middle number changes (0.4 → 0.5), or a 0.0.x one whose last
  number does, is a breaking change under npm and Cargo rules, so that PR waits for a maintainer
  too.
- Tauri's CLI refuses to build when the `tauri` crate and the `@tauri-apps/api` npm package (or a
  `tauri-plugin-*` crate and its `@tauri-apps/plugin-*` package) are on different major.minor
  versions, and Dependabot updates the two sides in separate PRs. So a PR that moves one of these
  to a new minor version waits for a maintainer, who updates both sides together. Their patch
  updates, and `tauri-build` and `@tauri-apps/cli`, which have no partner, still merge by
  themselves.
- Dependabot pushes newer versions onto a PR that is still open. When a push turns a PR into one
  that has to wait, the workflow turns its auto-merge off again, even auto-merge a maintainer
  turned on by hand: turn it on again after checking the new push. (Between a push and that job,
  a few seconds, the PR could still merge if every check on the new push had already passed.)
- GitHub Actions updates change the workflow files, which that workflow's token can't merge, so
  they wait for a maintainer.
- Major updates come as a PR each and always wait for a maintainer.

`scripts/dependabot-auto-merge.sh` makes that decision; `npm run check:scripts` tests it, along
with the PR title check and `repo-settings.sh check`.

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
