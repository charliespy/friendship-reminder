# Releasing Friendship Reminder

The step-by-step for a maintainer (or a coding agent) cutting a release. The rule for the version
number is in [CONTRIBUTING.md](../CONTRIBUTING.md#releases); `scripts/release.mjs` does the
mechanical parts.

## When

Only when the maintainer asks for a release. Merging a PR never releases by itself: changes pile up
on `main` until then.

## Steps

1. **Start from an up-to-date `main`, with the tags.**

   ```sh
   git switch main && git pull && git fetch --tags
   ```

2. **Plan** (a dry run: it changes nothing).

   ```sh
   npm run release:plan -- --out release-notes.md
   ```

   It prints the last release, the next version and why, and drafts the notes into
   `release-notes.md` (it won't replace a file that is already there; add `--force` for a fresh
   draft). With `npm run`, options go after `--`. If it says there is nothing to release (exit
   code 3), stop and say so; only when the maintainer still wants a release (for example,
   dependency updates only) run it again with `--patch`.

3. **Write the notes.** Edit `release-notes.md` into a short description for people who use the
   app, not for developers:
   - only the headings `### New`, `### Fixed`, `### Changed` and `### Good to know`, in that order,
     each with one plain sentence per change (leave out a heading with nothing under it);
   - no PR numbers, commit hashes, branch names, or changes users can't notice (tests, CI, docs,
     refactoring);
   - write links inline (`[text](url)`): `prepare` refuses `[name]: url` link lines;
   - no personal information (the release page is public):
     `scripts/privacy-scan.sh --stdin < release-notes.md`.

4. **Prepare** the release on a branch:

   ```sh
   git switch -c chore/release-vX.Y.Z
   npm run release:prepare -- X.Y.Z release-notes.md
   rm release-notes.md
   npm run check
   git commit -am "chore(release): vX.Y.Z"
   git push -u origin chore/release-vX.Y.Z
   ```

   This puts the notes at the top of `CHANGELOG.md` and sets the version in `package.json`,
   `package-lock.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` and `Cargo.lock`.
   It refuses when a line under `[Unreleased]` is missing from the notes (it would be lost): put
   it in, or add `--drop-unreleased` when the notes already say it in other words.

5. **Open the release PR** titled `chore(release): vX.Y.Z`, with the notes
   (`npm run release:notes -- X.Y.Z`) in its description. The release workflow runs a dry-run
   build on it and prints the description it will publish. It starts by itself only when the PR
   changes the app version; when it doesn't (the first release, `v0.1.0`, keeps the version the
   files already have), start it by hand and check that run:

   ```sh
   gh workflow run release.yml --ref chore/release-vX.Y.Z
   ```

   Merge the PR once the checks and the dry run pass. Merge nothing else into `main` while it is
   open: a change merged before it would ship in this release without being in its notes or its
   version. If something did merge, close the release PR, delete its branch
   (`git push origin --delete chore/release-vX.Y.Z`, then `git switch main && git branch -D
   chore/release-vX.Y.Z`), and start again from step 1: `plan` reads the new change and may give
   a different version (your notes are still in the closed PR's description). One exception:
   when it was only Dependabot updating libraries (`chore(deps): …`, which merge by themselves),
   the version doesn't change: if the notes lack "Updated the libraries Friendship Reminder is built on.", add
   it under `### Changed` in the release's `CHANGELOG.md` section on the release branch (and in the
   PR description), push, and carry on.

6. **Tag the release PR's merge commit** on `main`, by its hash (another PR may have merged after
   it), and push the tag:

   ```sh
   git switch main && git pull
   git log --first-parent --format='%H %s' | grep -m1 ' chore(release): vX.Y.Z (#'
   git tag -a vX.Y.Z <the hash it printed> -m "Friendship Reminder vX.Y.Z"
   git push origin vX.Y.Z
   ```

   Only `vX.Y.Z` tags are pushed to this repository; no other tags.

7. **Watch the release workflow** (it builds the `.dmg` and publishes the release with the
   `CHANGELOG.md` section as its description):

   ```sh
   gh run list --workflow release.yml --event push --branch vX.Y.Z --limit 1
   gh run watch <run id> --exit-status
   ```

   List the tag's own run, not just the newest one (that may still be the release PR's dry run).
   If the list is empty, GitHub hasn't started it yet: wait a few seconds and list again.

8. **Verify** the published release:

   ```sh
   npm run release:verify -- vX.Y.Z
   ```

   It checks that the release isn't a draft, has exactly one `.dmg` for that version, and shows
   the same description as `CHANGELOG.md`. Then download the `.dmg`, open it, and check that
   Friendship Reminder starts.

## iPhone: a TestFlight build

The iPhone app isn't on the App Store yet; test builds go to TestFlight, by hand, whenever the
maintainer wants one. They don't need a Mac release, a tag or a version change.

**Once, on the Mac that uploads:**

- Everything README.md lists for building the iPhone app, plus `rustup component add llvm-tools`
  (without it the sign-in sheet's Swift code doesn't link for a real iPhone).
- An App Store Connect API key with **Admin** access (Users and Access → Integrations → App Store
  Connect API → Team Keys). Its `AuthKey_<key id>.p8` file can be downloaded only once: keep it
  outside the repo. Admin lets Xcode make the App Store certificate and profile by itself.
- In `.env.local` at the repo root (git ignores it), next to the Supabase settings:

  ```sh
  APPLE_DEVELOPMENT_TEAM=<team id>        # Membership details on developer.apple.com
  APPLE_API_KEY=<key id>
  APPLE_API_ISSUER=<issuer id>            # above the list of keys
  APPLE_API_KEY_PATH=<folder>/AuthKey_<key id>.p8
  ```

**Each build:**

1. On an up-to-date `main`, with nothing uncommitted: `npm run ios:testflight`. It builds the app for the App Store, signs
   it, and uploads it. The build number is the time in UTC (`YYYYMMDDHHMMSS`), so each upload is
   new to Apple; the version is the one in the files.
2. Apple processes it in 5 to 30 minutes, then it shows in App Store Connect → the app →
   **TestFlight**. Testers in an **Internal Testing** group get it in the TestFlight app on
   their iPhone. Internal testing needs no review; a build lasts 90 days.

The app uses encryption only to secure its HTTPS connections (to the account server), which is
exempt from export rules, so it says `ITSAppUsesNonExemptEncryption` is `false` and App Store
Connect asks no export questions. Revisit this if the app ever encrypts anything else.

## How the script reads the history

`plan` reads one commit per merged PR: the squash-merge commits on `main` (`feat: … (#12)`) since
the last `vX.Y.Z` tag. A commit undone by GitHub's revert button (`Revert "feat: … (#12)"`) is
left out together with its revert; reverting that revert (or `Reapply "…"`) brings it back. A
released change that was reverted and then restored since the last release counts as no change. PRs
are squash-merged, so this is every change; a PR merged with a merge commit would show up only as
"Merge pull request …", which isn't a conventional commit and is left out. Before the first
release there is no tag, so the notes come from `[Unreleased]` alone.

## When something goes wrong

- **The tag is wrong** (it isn't on the release PR's merge commit, or doesn't match the version
  in the files): the workflow checks both and stops before publishing, and the tag would block
  every later `plan`. Only the maintainer deletes it (`git push origin :refs/tags/vX.Y.Z`, then
  `git tag -d vX.Y.Z`; other clones run `git tag -d vX.Y.Z` too), and then the right commit is
  tagged as in step 6. This is the one time a tag already on GitHub is removed, and only because
  nothing was published from it.
- **The workflow fails before publishing for another reason** (the notes are missing, the build
  breaks): nothing was published. Fix the cause on `main` with a normal PR,
  then release the next patch version (`npm run release:plan -- --patch` when the fix is not a
  `fix:`). Its notes must also carry everything in the unpublished version's section, since
  nobody has seen it: copy those lines into the new notes. Never move or re-push a tag that is
  already on GitHub.
- **The release is published but wrong** (a bad `.dmg`, a mistake in the notes): fix it on `main`
  and release the next patch version. A typo in the description alone can be edited on the release
  page; change `CHANGELOG.md` to match in the next PR (`release:verify` for that version then
  reports the difference, since it reads `CHANGELOG.md` as it was at the tag).
