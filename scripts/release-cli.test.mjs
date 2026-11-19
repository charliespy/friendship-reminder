// @vitest-environment node
// Runs scripts/release.mjs in a scratch git repo with a copy of this repo's version files.
import { execFileSync, spawnSync } from "node:child_process";
import { chmodSync, cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { setCargoLockVersion, setCargoTomlVersion, setJsonVersion } from "./release-lib.mjs";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const FILES = ["package.json", "package-lock.json", "src-tauri/tauri.conf.json", "src-tauri/Cargo.toml", "Cargo.lock"];
let dir;
// No global git settings (commit signing, hooks) in the scratch repo.
// GIT_DIR and friends (set when run from a git hook) would point git at the real repo.
const env = { ...process.env, GIT_CONFIG_GLOBAL: "/dev/null", GIT_CONFIG_NOSYSTEM: "1" };
for (const k of ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_COMMON_DIR"]) delete env[k];

function sh(cmd, args) {
  return execFileSync(cmd, args, { cwd: dir, env, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
}
/** Copies a version file, set back to 0.1.0 so the tests hold after real releases. */
function copyAt010(f) {
  const text = readFileSync(join(root, f), "utf8");
  const out =
    f === "Cargo.lock"
      ? setCargoLockVersion(text, "friends", "0.1.0")
      : f.endsWith("Cargo.toml")
        ? setCargoTomlVersion(text, "0.1.0")
        : setJsonVersion(text, "0.1.0", { lockfile: f === "package-lock.json" });
  writeFileSync(join(dir, f), out);
}
function release(...args) {
  const r = spawnSync("node", ["scripts/release.mjs", ...args], {
    cwd: dir,
    encoding: "utf8",
    // A stand-in `gh` first on PATH: it prints the release in gh.json.
    env: { ...env, INIT_CWD: dir, PATH: `${join(dir, "bin")}:${env.PATH}` },
  });
  return { code: r.status, out: r.stdout, err: r.stderr };
}
function commit(message) {
  sh("git", ["add", "-A"]);
  sh("git", ["-c", "user.name=Test", "-c", "user.email=test@example.com", "-c", "commit.gpgsign=false", "commit", "-q", "--allow-empty", "-m", message]);
}

beforeEach(() => {
  dir = mkdtempSync(join(tmpdir(), "release-cli-"));
  sh("git", ["init", "-q", "-b", "main"]);
  mkdirSync(join(dir, "scripts"));
  mkdirSync(join(dir, "src-tauri"));
  mkdirSync(join(dir, "bin"));
  writeFileSync(join(dir, "bin", "gh"), `#!/bin/sh\ncat "${join(dir, "gh.json")}"\n`);
  chmodSync(join(dir, "bin", "gh"), 0o755);
  for (const f of ["scripts/release.mjs", "scripts/release-lib.mjs"]) cpSync(join(root, f), join(dir, f));
  for (const f of FILES) copyAt010(f);
  writeFileSync(join(dir, "CHANGELOG.md"), "# Changelog\n\n## [Unreleased]\n\n### Added\n- A new look.\n");
  commit("feat: first");
});
afterEach(() => rmSync(dir, { recursive: true, force: true }));

describe("release.mjs", () => {
  it("plans, prepares and prints the first release", () => {
    const plan = release("plan", "--out", "notes.md");
    expect(plan.code).toBe(0);
    expect(plan.out).toContain("Next version:  0.1.0");
    expect(readFileSync(join(dir, "notes.md"), "utf8")).toContain("### New\n\n- A new look.");
    const prep = release("prepare", "0.1.0", "notes.md", "--date", "2026-11-19");
    expect(prep.err).toBe("");
    expect(prep.code).toBe(0);
    const notes = release("notes", "v0.1.0");
    expect(notes.code).toBe(0);
    expect(notes.out).toMatch(/^### New\n\n- A new look\.\n\n### Good to know\n/);
  });
  it("won't overwrite a notes file unless told to", () => {
    writeFileSync(join(dir, "notes.md"), "rewritten\n");
    expect(release("plan", "--out", "notes.md").err).toMatch(/already there/);
    expect(readFileSync(join(dir, "notes.md"), "utf8")).toBe("rewritten\n");
    expect(release("plan", "--out", "notes.md", "--force").code).toBe(0);
  });
  it("verifies a published release against CHANGELOG.md at the tag", () => {
    release("plan", "--out", "notes.md");
    release("prepare", "0.1.0", "notes.md", "--date", "2026-11-19");
    commit("chore(release): v0.1.0 (#9)");
    sh("git", ["-c", "tag.gpgsign=false", "tag", "v0.1.0"]);
    const body = release("notes", "0.1.0").out;
    const dmg = { name: "Friends_0.1.0_universal.dmg", size: 9_000_000 };
    const gh = (o) => writeFileSync(join(dir, "gh.json"), JSON.stringify({ tagName: "v0.1.0", isDraft: false, url: "u", body, assets: [dmg], ...o }));
    gh({ body: body.replace(/\n/g, "\r\n") });
    expect(release("verify", "v0.1.0")).toMatchObject({ code: 0, err: "" });
    gh({ body: "See CHANGELOG.md." });
    expect(release("verify", "v0.1.0").err).toMatch(/differs/);
    gh({ isDraft: true, assets: [] });
    const r = release("verify", "v0.1.0").err;
    expect(r).toMatch(/draft/);
    expect(r).toMatch(/exactly one \.dmg, found 0/);
    gh({ assets: [{ name: "Friends_0.0.9_universal.dmg", size: 10 }] });
    expect(release("verify", "v0.1.0").err).toMatch(/not named for 0\.1\.0[\s\S]*only 10 bytes/);
  });
  it("bumps from the last tag, and refuses a version that doesn't go up", () => {
    sh("git", ["-c", "tag.gpgsign=false", "tag", "v0.1.0"]);
    commit("fix: a bug (#2)");
    const plan = release("plan", "--out", "notes.md");
    expect(plan.out).toContain("Next version:  0.1.1");
    expect(release("prepare", "0.1.0", "notes.md", "--drop-unreleased").err).toMatch(/not above the last release/);
    expect(release("prepare", "0.1.1", "notes.md").code).toBe(0);
    // Prepared twice before tagging: refused.
    expect(release("prepare", "0.2.0", "notes.md").err).toMatch(/already prepared/);
    expect(JSON.parse(readFileSync(join(dir, "src-tauri/tauri.conf.json"), "utf8")).version).toBe("0.1.1");
    expect(readFileSync(join(dir, "Cargo.lock"), "utf8")).toContain('name = "friends"\nversion = "0.1.1"\n');
  });
  it("refuses notes that would lose an [Unreleased] line, and changes no file", () => {
    writeFileSync(join(dir, "notes.md"), "### New\n\n- Something else.\n");
    const before = FILES.map((f) => readFileSync(join(dir, f), "utf8"));
    const r = release("prepare", "0.1.0", "notes.md");
    expect(r.code).toBe(1);
    expect(r.err).toContain("- A new look.");
    expect(FILES.map((f) => readFileSync(join(dir, f), "utf8"))).toEqual(before);
    expect(release("prepare", "0.1.0", "notes.md", "--drop-unreleased").code).toBe(0);
  });
  it("says so when there is nothing to release, or no notes", () => {
    sh("git", ["-c", "tag.gpgsign=false", "tag", "v0.1.0"]);
    commit("docs: typo (#3)");
    expect(release("plan").code).toBe(3);
    expect(release("plan", "--patch").out).toContain("Next version:  0.1.1");
    expect(release("notes", "0.1.0").code).toBe(1);
    expect(release("notes", "0.1.0", "--output", "x.md").err).toMatch(/unknown option/);
    expect(release("prepare", "0.1.1", "missing.md").err).toMatch(/ENOENT|no such file/);
  });
});
