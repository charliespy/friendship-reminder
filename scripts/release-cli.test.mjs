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
/** Copies a version file into the scratch repo, set to `version` (0.1.0 by default, so the tests
 * hold after real releases). */
function copyAt(f, version = "0.1.0") {
  const text = readFileSync(join(root, f), "utf8");
  const out =
    f === "Cargo.lock"
      ? setCargoLockVersion(text, "friends", version)
      : f.endsWith("Cargo.toml")
        ? setCargoTomlVersion(text, version)
        : setJsonVersion(text, version, { lockfile: f === "package-lock.json" });
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
  for (const f of FILES) copyAt(f);
  writeFileSync(join(dir, "CHANGELOG.md"), "# Changelog\n\n## [Unreleased]\n\n### Added\n- A new look.\n");
  commit("feat: first");
});
afterEach(() => rmSync(dir, { recursive: true, force: true }));

// Each test starts several node and git processes: allow for a busy machine.
describe("release.mjs", { timeout: 60_000 }, () => {
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
    expect(notes.out).toMatch(/^### New\n\n- A new look\.\n/);
    expect(notes.out).not.toContain("Open Anyway");
  });
  it("dry-run: the plan, then the prepared description, then the plan again after the tag", () => {
    let r = release("dry-run");
    expect(r).toMatchObject({ code: 0 });
    expect(r.out).toContain("Next version:  0.1.0");
    release("plan", "--out", "notes.md");
    release("prepare", "0.1.0", "notes.md", "--date", "2026-11-19");
    r = release("dry-run");
    expect(r.code).toBe(0);
    expect(r.out).toMatch(/^v0\.1\.0 is prepared .*\n\n### New\n\n- A new look\./);
    commit("chore(release): v0.1.0 (#9)");
    sh("git", ["-c", "tag.gpgsign=false", "tag", "v0.1.0"]);
    commit("docs: typo (#10)");
    r = release("dry-run");
    expect(r.code).toBe(0); // Nothing to release is fine for a dry run.
    expect(r.out).toContain("Last release:  v0.1.0");
  });
  it("dry-run fails on a hand-edited release PR: version not above the last, or files that disagree", () => {
    release("plan", "--out", "notes.md");
    release("prepare", "0.1.0", "notes.md", "--date", "2026-11-19");
    commit("chore(release): v0.1.0 (#9)");
    sh("git", ["-c", "tag.gpgsign=false", "tag", "v0.1.0"]);
    commit("fix: a bug (#10)");
    writeFileSync(join(dir, "notes.md"), "### Fixed\n\n- A bug.\n");
    expect(release("prepare", "0.1.1", "notes.md").code).toBe(0);
    expect(release("dry-run").code).toBe(0);
    // Cargo.toml left behind by hand.
    writeFileSync(join(dir, "src-tauri/Cargo.toml"), setCargoTomlVersion(readFileSync(join(dir, "src-tauri/Cargo.toml"), "utf8"), "0.1.0"));
    expect(release("dry-run").err).toMatch(/disagree with v0\.1\.1: src-tauri\/Cargo\.toml says 0\.1\.0/);
    // A section and files for a version below the last release.
    for (const f of FILES) copyAt(f, "0.0.9");
    writeFileSync(join(dir, "CHANGELOG.md"), "# Changelog\n\n## [Unreleased]\n\n## [0.0.9] - 2026-11-20\n\n- x\n");
    expect(release("dry-run").err).toMatch(/not above the last release \(v0\.1\.0\)/);
  });
  it("dry-run fails on a prepared release with an empty section", () => {
    writeFileSync(join(dir, "CHANGELOG.md"), "# Changelog\n\n## [Unreleased]\n\n## [0.1.0] - 2026-11-19\n");
    const r = release("dry-run");
    expect(r.code).toBe(1);
    expect(r.err).toMatch(/no notes for \[0\.1\.0\]/);
    expect(release("notes", "v1.0.0-rc.1").err).toMatch(/not a release version/);
    expect(release("notes", "0.2.0", "--check-files").err).toMatch(/disagree with v0\.2\.0: package\.json says 0\.1\.0/);
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
    const dmg = { name: "Friendship.Reminder_0.1.0_universal.dmg", size: 9_000_000 };
    const gh = (o) => writeFileSync(join(dir, "gh.json"), JSON.stringify({ tagName: "v0.1.0", isDraft: false, url: "u", body, assets: [dmg], ...o }));
    gh({ body: body.replace(/\n/g, "\r\n") });
    expect(release("verify", "v0.1.0")).toMatchObject({ code: 0, err: "" });
    gh({ body: "See CHANGELOG.md." });
    expect(release("verify", "v0.1.0").err).toMatch(/differs/);
    gh({ isDraft: true, assets: [] });
    const r = release("verify", "v0.1.0").err;
    expect(r).toMatch(/draft/);
    expect(r).toMatch(/exactly one \.dmg, found 0/);
    gh({ assets: [{ name: "Friendship.Reminder_0.0.9_universal.dmg", size: 10 }] });
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
