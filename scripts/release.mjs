#!/usr/bin/env node
// Releasing Friends, step by step (docs/RELEASING.md has the whole procedure):
//
//   node scripts/release.mjs plan [--patch] [--out <file>]
//       Dry run: changes nothing. Prints the last release, the next version and why, and a draft
//       of the release notes (also written to <file> with --out). Exits 3 when there is nothing to
//       release (no feat, fix, perf, revert or breaking change since the last tag; --patch
//       releases a patch anyway, e.g. for dependency updates only).
//   node scripts/release.mjs prepare <version> <notes file> [--date YYYY-MM-DD] [--drop-unreleased]
//       Writes the notes into CHANGELOG.md as "## [<version>] - <date>" (under an emptied
//       [Unreleased]) and sets <version> in package.json, package-lock.json,
//       src-tauri/tauri.conf.json, src-tauri/Cargo.toml and Cargo.lock. Refuses a version that
//       isn't above the last release, and, without --drop-unreleased, refuses when a line under
//       [Unreleased] isn't in the notes (it would be lost).
//   node scripts/release.mjs notes <version> [--out <file>]
//       Prints the CHANGELOG.md section for <version> (the release description). Exits 1 when it
//       is missing or empty. The release workflow publishes exactly this text.
//   node scripts/release.mjs verify <vX.Y.Z>
//       Checks the published GitHub Release: not a draft, exactly one .dmg for that version, and
//       a description equal to the CHANGELOG.md section at that tag. Needs `gh`.
//
// Run from anywhere inside the repo; file arguments are relative to the folder you ran it from.
// The rules themselves are in scripts/release-lib.mjs.
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import {
  checkNewVersion,
  draftNotes,
  insertRelease,
  latestReleaseTag,
  nextRelease,
  parseCommit,
  sectionBody,
  setCargoLockVersion,
  setCargoTomlVersion,
  setJsonVersion,
  unreleasedLinesMissing,
} from "./release-lib.mjs";

/** The Rust package of the app itself (src-tauri); the other crates keep their own versions. */
const APP_CRATE = "friends";

function git(args, opts = {}) {
  return execFileSync("git", args, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"], ...opts });
}

const root = git(["rev-parse", "--show-toplevel"]).trim();
const file = (p) => join(root, p);
const read = (p) => readFileSync(file(p), "utf8");
/** A path the user typed: relative to where they ran the command (npm run moves to the repo root). */
const userPath = (p) => resolve(process.env.INIT_CWD ?? process.cwd(), p);

function fail(message, code = 1) {
  process.stderr.write(`release: ${message}\n`);
  process.exit(code);
}

/** Splits argv into positional arguments and --flags (a flag in `valued` takes the next argument). */
function parseArgs(argv, valued) {
  const pos = [];
  const flags = {};
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (!a.startsWith("--")) {
      pos.push(a);
    } else if (valued.includes(a)) {
      if (i + 1 >= argv.length) fail(`${a} needs a value`);
      flags[a] = argv[++i];
    } else {
      flags[a] = true;
    }
  }
  return { pos, flags };
}

function currentVersion() {
  return JSON.parse(read("src-tauri/tauri.conf.json")).version;
}

/** Commits on main's first-parent line since `since` (or all of them), oldest first. */
function commitsSince(since) {
  const range = since ? [`${since}..HEAD`] : ["HEAD"];
  const out = git(["log", "--first-parent", "--reverse", "--format=%H%x1f%s%x1f%b%x1e", ...range]);
  return out
    .split("\x1e")
    .map((r) => r.replace(/^\n/, ""))
    .filter(Boolean)
    .map((r) => {
      const [sha, subject, body] = r.split("\x1f");
      return { sha, subject, body: body ?? "", parsed: parseCommit(subject, body ?? "") };
    });
}

function lastReleaseTag() {
  return latestReleaseTag(git(["tag", "--list", "v*", "--merged", "HEAD"]).split("\n").filter(Boolean));
}

function checkFlags(flags, allowed) {
  for (const f of Object.keys(flags)) if (!allowed.includes(f)) fail(`unknown option ${f}`);
}

function plan(argv) {
  const { flags } = parseArgs(argv, ["--out"]);
  checkFlags(flags, ["--out", "--patch"]);
  const lastTag = lastReleaseTag();
  const version = currentVersion();
  const all = commitsSince(lastTag);
  const conventional = all.filter((c) => c.parsed);
  let next;
  try {
    next = nextRelease({
      lastTag,
      currentVersion: version,
      commits: conventional.map((c) => c.parsed),
      forcePatch: Boolean(flags["--patch"]),
    });
  } catch (e) {
    fail(e.message);
  }
  const unreleased = sectionBody(read("CHANGELOG.md"), "Unreleased");
  if (unreleased === null) fail("CHANGELOG.md has no ## [Unreleased] section");

  const out = [];
  out.push(`Last release:  ${lastTag ?? "none"}`);
  out.push(`Version now:   ${version}`);
  out.push(`Next version:  ${next.version ?? "-"}`);
  out.push(`Why:           ${next.reason}`);
  out.push("");
  out.push(`Commits on main since ${lastTag ?? "the start"}: ${all.length} (${all.length - conventional.length} not conventional, left out)`);
  if (lastTag) {
    for (const c of conventional) out.push(`  ${c.sha.slice(0, 7)} ${c.subject}`);
  } else {
    out.push("  (first release: the notes come from CHANGELOG.md's [Unreleased] section alone)");
  }
  process.stdout.write(`${out.join("\n")}\n`);
  if (!next.version) process.exit(3);

  const notes = draftNotes({ unreleased, commits: lastTag ? conventional.map((c) => c.parsed) : [] });
  process.stdout.write(`\nDraft notes for v${next.version} (rewrite into plain words before "prepare"):\n\n${notes}\n`);
  if (flags["--out"]) writeFileSync(userPath(flags["--out"]), `${notes}\n`);
}

function prepare(argv) {
  const { pos, flags } = parseArgs(argv, ["--date"]);
  checkFlags(flags, ["--date", "--drop-unreleased"]);
  if (pos.length !== 2) fail("usage: release.mjs prepare <version> <notes file> [--date YYYY-MM-DD] [--drop-unreleased]");
  const [version, notesFile] = pos;
  const date = flags["--date"] ?? localDate();
  // Work everything out first, then write: a bad input changes no file.
  const writes = [];
  try {
    checkNewVersion({ version, currentVersion: currentVersion(), lastTag: lastReleaseTag() });
    const notes = readFileSync(userPath(notesFile), "utf8");
    const missing = unreleasedLinesMissing(sectionBody(read("CHANGELOG.md"), "Unreleased") ?? "", notes);
    if (missing.length && !flags["--drop-unreleased"]) {
      throw new Error(
        `these lines under [Unreleased] aren't in the notes, and would be lost:\n  ${missing.join("\n  ")}\n` +
          "Put them in the notes, or pass --drop-unreleased if the notes already cover them in other words.",
      );
    }
    writes.push(["CHANGELOG.md", insertRelease(read("CHANGELOG.md"), version, date, notes)]);
    writes.push(["package.json", setJsonVersion(read("package.json"), version)]);
    writes.push(["package-lock.json", setJsonVersion(read("package-lock.json"), version, { lockfile: true })]);
    writes.push(["src-tauri/tauri.conf.json", setJsonVersion(read("src-tauri/tauri.conf.json"), version)]);
    writes.push(["src-tauri/Cargo.toml", setCargoTomlVersion(read("src-tauri/Cargo.toml"), version)]);
    writes.push(["Cargo.lock", setCargoLockVersion(read("Cargo.lock"), APP_CRATE, version)]);
  } catch (e) {
    fail(e.message);
  }
  for (const [p, text] of writes) writeFileSync(file(p), text);
  process.stdout.write(`Prepared v${version} (${date}): ${writes.map(([p]) => p).join(", ")}.\n`);
}

function notes(argv) {
  const { pos, flags } = parseArgs(argv, ["--out"]);
  if (pos.length !== 1) fail("usage: release.mjs notes <version> [--out <file>]");
  const version = pos[0].replace(/^v/, "");
  const body = sectionBody(read("CHANGELOG.md"), version);
  if (!body) fail(`CHANGELOG.md has no notes for [${version}]: run "release.mjs prepare" in the release PR`);
  if (flags["--out"]) writeFileSync(userPath(flags["--out"]), `${body}\n`);
  else process.stdout.write(`${body}\n`);
}

function verify(argv) {
  const { pos } = parseArgs(argv, []);
  if (pos.length !== 1 || !/^v\d+\.\d+\.\d+$/.test(pos[0])) fail("usage: release.mjs verify <vX.Y.Z>");
  const tag = pos[0];
  const version = tag.slice(1);
  let release;
  try {
    release = JSON.parse(
      execFileSync("gh", ["release", "view", tag, "--json", "tagName,isDraft,body,assets,url"], {
        cwd: root,
        encoding: "utf8",
        stdio: ["ignore", "pipe", "pipe"],
      }),
    );
  } catch (e) {
    fail(`no GitHub Release for ${tag} (${String(e.stderr ?? e.message).trim()})`);
  }
  const problems = [];
  if (release.isDraft) problems.push("the release is still a draft");
  const dmgs = release.assets.filter((a) => a.name.endsWith(".dmg"));
  if (dmgs.length !== 1) problems.push(`want exactly one .dmg, found ${dmgs.length}`);
  for (const d of dmgs) {
    if (!d.name.includes(`_${version}_`)) problems.push(`${d.name} is not named for ${version}`);
    if (!(d.size > 1_000_000)) problems.push(`${d.name} is only ${d.size} bytes`);
  }
  let changelog = null;
  try {
    changelog = git(["show", `${tag}:CHANGELOG.md`]);
  } catch {
    problems.push(`can't read CHANGELOG.md at ${tag} (fetch the tags first: git fetch --tags)`);
  }
  const expected = changelog === null ? null : sectionBody(changelog, version);
  if (changelog !== null && expected === null) problems.push(`CHANGELOG.md at ${tag} has no [${version}] section`);
  if (expected === "") problems.push(`the [${version}] section of CHANGELOG.md at ${tag} is empty`);
  if (expected && release.body.trim() !== expected) {
    problems.push("the release description differs from the CHANGELOG.md section");
  }
  if (problems.length) fail(`${tag} at ${release.url}:\n  - ${problems.join("\n  - ")}`);
  process.stdout.write(`${tag} is published at ${release.url}: ${dmgs[0].name} (${dmgs[0].size} bytes), and the description matches CHANGELOG.md.\n`);
}

function localDate() {
  const d = new Date();
  const pad = (n) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

const [command, ...rest] = process.argv.slice(2);
const commands = { plan, prepare, notes, verify };
if (!commands[command]) fail("usage: release.mjs plan | prepare | notes | verify (see the top of scripts/release.mjs)");
commands[command](rest);
