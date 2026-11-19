// @vitest-environment node
// Runs scripts/release.mjs in a scratch git repo with a copy of this repo's version files.
import { execFileSync, spawnSync } from "node:child_process";
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const FILES = ["package.json", "package-lock.json", "src-tauri/tauri.conf.json", "src-tauri/Cargo.toml", "Cargo.lock"];
let dir;

function sh(cmd, args) {
  return execFileSync(cmd, args, { cwd: dir, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
}
function release(...args) {
  const r = spawnSync("node", ["scripts/release.mjs", ...args], {
    cwd: dir,
    encoding: "utf8",
    env: { ...process.env, INIT_CWD: dir },
  });
  return { code: r.status, out: r.stdout, err: r.stderr };
}
function commit(message) {
  sh("git", ["add", "-A"]);
  sh("git", ["-c", "user.name=Test", "-c", "user.email=test@example.com", "commit", "-q", "--allow-empty", "-m", message]);
}

beforeEach(() => {
  dir = mkdtempSync(join(tmpdir(), "release-cli-"));
  sh("git", ["init", "-q", "-b", "main"]);
  mkdirSync(join(dir, "scripts"));
  mkdirSync(join(dir, "src-tauri"));
  for (const f of ["scripts/release.mjs", "scripts/release-lib.mjs", ...FILES]) cpSync(join(root, f), join(dir, f));
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
  it("bumps from the last tag, and refuses a version that doesn't go up", () => {
    sh("git", ["tag", "v0.1.0"]);
    commit("fix: a bug (#2)");
    const plan = release("plan", "--out", "notes.md");
    expect(plan.out).toContain("Next version:  0.1.1");
    expect(release("prepare", "0.1.0", "notes.md", "--drop-unreleased").err).toMatch(/not above the last release/);
    expect(release("prepare", "0.1.1", "notes.md").code).toBe(0);
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
    sh("git", ["tag", "v0.1.0"]);
    commit("docs: typo (#3)");
    expect(release("plan").code).toBe(3);
    expect(release("plan", "--patch").out).toContain("Next version:  0.1.1");
    expect(release("notes", "0.1.0").code).toBe(1);
    expect(release("prepare", "0.1.1", "missing.md").err).toMatch(/ENOENT|no such file/);
  });
});
