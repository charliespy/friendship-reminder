# Releasing Friends

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
   draft). With `npm run`, options go after `--`. If it says there is nothing to release (exit code 3), stop and say so; only
   when the maintainer still wants a release (for example, dependency updates only) run it again
   with `--patch`.

3. **Write the notes.** Edit `release-notes.md` into a short description for people who use the
   app, not for developers:
   - only the headings `### New`, `### Fixed`, `### Changed` and `### Good to know`, in that order,
     each with one plain sentence per change (leave out a heading with nothing under it);
   - no PR numbers, commit hashes, branch names, or changes users can't notice (tests, CI, docs,
     refactoring);
   - keep the "Good to know" line on opening the app the first time while releases aren't signed
     with an Apple Developer ID (the draft always adds it; delete it once releases are signed);
   - write links inline (`[text](url)`): a `[name]: url` line would end the section early;
   - no personal information (the release page is public): `scripts/privacy-scan.sh --stdin < release-notes.md`.

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

   Merge the PR once the checks and the dry run pass.

6. **Tag the release PR's merge commit** on `main`, by its hash (another PR may have merged after
   it), and push the tag:

   ```sh
   git switch main && git pull
   git log --first-parent --format='%H %s' | grep -m1 ' chore(release): vX.Y.Z (#'
   git tag -a vX.Y.Z <the hash it printed> -m "Friends vX.Y.Z"
   git push origin vX.Y.Z
   ```

   Only `vX.Y.Z` tags are pushed to this repository; no other tags.

7. **Watch the release workflow** (it builds the `.dmg` and publishes the release with the
   `CHANGELOG.md` section as its description):

   ```sh
   gh run list --workflow release.yml --limit 1
   gh run watch <run id> --exit-status
   ```

8. **Verify** the published release:

   ```sh
   npm run release:verify -- vX.Y.Z
   ```

   It checks that the release isn't a draft, has exactly one `.dmg` for that version, and shows
   the same description as `CHANGELOG.md`. Then download the `.dmg`, open it, and check that
   Friends starts.

## How the script reads the history

`plan` reads one commit per merged PR: the squash-merge commits on `main` (`feat: … (#12)`) since
the last `vX.Y.Z` tag. PRs are squash-merged, so this is every change; a PR merged with a merge
commit would show up only as "Merge pull request …", which isn't a conventional commit and is left
out. Before the first release there is no tag, so the notes come from `[Unreleased]` alone.

## When something goes wrong

- **The workflow fails before publishing** (the tag doesn't match the version, the notes are
  missing, the build breaks): nothing was published. Fix the cause on `main` with a normal PR,
  then release the next patch version (`npm run release:plan -- --patch` when the fix is not a
  `fix:`). Its notes must also carry everything in the unpublished version's section, since
  nobody has seen it: copy those lines into the new notes. Never move or re-push a tag that is
  already on GitHub.
- **The release is published but wrong** (a bad `.dmg`, a mistake in the notes): fix it on `main`
  and release the next patch version. A typo in the description alone can be edited on the release
  page; change `CHANGELOG.md` to match in the next PR.
