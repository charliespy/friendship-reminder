// @vitest-environment node
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import {
  applyBump,
  checkNewVersion,
  bumpFor,
  draftNotes,
  insertRelease,
  latestReleaseTag,
  nextRelease,
  parseCommit,
  parseSections,
  sectionBody,
  sentence,
  setCargoLockVersion,
  setCargoTomlVersion,
  setJsonVersion,
  UNSIGNED_NOTE,
  unreleasedLinesMissing,
} from "./release-lib.mjs";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const repoFile = (p) => readFileSync(join(root, p), "utf8");
const c = (subject, body = "") => parseCommit(subject, body);
/** The lines of `after` that differ from `before` (same line count). */
function changedLines(before, after) {
  const old = before.split("\n");
  return after.split("\n").filter((l, i) => l !== old[i]);
}

describe("parseCommit", () => {
  it("reads type, scope and description, and drops the PR number", () => {
    expect(c("feat(snooze): snooze a reminder (#14)")).toEqual({
      type: "feat",
      scope: "snooze",
      breaking: false,
      description: "snooze a reminder",
    });
  });
  it("sees a breaking change from ! or the footer", () => {
    expect(c("feat!: new data file").breaking).toBe(true);
    expect(c("refactor(core)!: drop old format").breaking).toBe(true);
    expect(c("fix: x", "Some text.\n\nBREAKING CHANGE: the file moved").breaking).toBe(true);
    expect(c("fix: x", "BREAKING-CHANGE: the file moved").breaking).toBe(true);
    expect(c("fix: x", "this is not a BREAKING CHANGE: mid-line").breaking).toBe(false);
  });
  it("returns null for subjects that aren't conventional", () => {
    expect(c("Fix window resizing")).toBeNull();
    expect(c("Revert \"feat: x\"")).toBeNull();
    expect(c("feat:no space")).toBeNull();
  });
});

describe("the version rule", () => {
  it("feat -> minor, fix/perf/revert -> patch, others nothing", () => {
    expect(bumpFor([c("fix: a"), c("feat: b"), c("docs: c")])).toBe("minor");
    expect(bumpFor([c("perf: a"), c("chore: b")])).toBe("patch");
    expect(bumpFor([c("revert: feat: a")])).toBe("patch");
    expect(bumpFor([c("docs: a"), c("ci: b"), c("chore(deps): c"), c("test: d"), c("refactor: e")])).toBeNull();
    expect(bumpFor([c("feat: a"), c("chore!: b")])).toBe("major");
  });
  it("applies bumps, and a breaking change on 0.x bumps the minor", () => {
    expect(applyBump("1.2.3", "major")).toBe("2.0.0");
    expect(applyBump("1.2.3", "minor")).toBe("1.3.0");
    expect(applyBump("1.2.3", "patch")).toBe("1.2.4");
    expect(applyBump("0.4.2", "major")).toBe("0.5.0");
  });
  it("only lets a release go up", () => {
    expect(() => checkNewVersion({ version: "0.1.0", currentVersion: "0.1.0", lastTag: null })).not.toThrow();
    expect(() => checkNewVersion({ version: "0.2.0", currentVersion: "0.1.0", lastTag: null })).not.toThrow();
    expect(() => checkNewVersion({ version: "0.0.9", currentVersion: "0.1.0", lastTag: null })).toThrow(/below/);
    expect(() => checkNewVersion({ version: "0.1.0", currentVersion: "0.1.0", lastTag: "v0.1.0" })).toThrow(/not above/);
    expect(() => checkNewVersion({ version: "0.1.1", currentVersion: "0.1.0", lastTag: "v0.1.0" })).not.toThrow();
    expect(() => checkNewVersion({ version: "1.0", currentVersion: "0.1.0", lastTag: null })).toThrow(/version/);
  });
  it("finds the newest vX.Y.Z tag and ignores other tags", () => {
    expect(latestReleaseTag(["v0.9.0", "v0.10.0", "v0.2.1", "builder/NR-1/m1", "v1.0.0-rc.1", "x"])).toBe("v0.10.0");
    expect(latestReleaseTag(["builder/NR-1/m1"])).toBeNull();
  });
});

describe("nextRelease", () => {
  it("first release: the version already in the files", () => {
    const r = nextRelease({ lastTag: null, currentVersion: "0.1.0", commits: [c("feat: a")] });
    expect(r.version).toBe("0.1.0");
    expect(r.reason).toMatch(/first release/);
  });
  it("bumps from the last tag", () => {
    expect(nextRelease({ lastTag: "v0.1.0", currentVersion: "0.1.0", commits: [c("fix: a")] }).version).toBe("0.1.1");
    expect(nextRelease({ lastTag: "v0.1.0", currentVersion: "0.1.0", commits: [c("feat: a")] }).version).toBe("0.2.0");
    const breaking = nextRelease({ lastTag: "v0.1.0", currentVersion: "0.1.0", commits: [c("feat!: a")] });
    expect(breaking).toMatchObject({ version: "0.2.0", bump: "minor" });
    expect(breaking.reason).toMatch(/0\.x/);
    expect(nextRelease({ lastTag: "v1.4.0", currentVersion: "1.4.0", commits: [c("fix!: a")] }).version).toBe("2.0.0");
  });
  it("has nothing to release without a feat or fix, unless forced", () => {
    const commits = [c("docs: a"), c("chore(deps): b")];
    expect(nextRelease({ lastTag: "v0.1.0", currentVersion: "0.1.0", commits }).version).toBeNull();
    expect(nextRelease({ lastTag: "v0.1.0", currentVersion: "0.1.0", commits, forcePatch: true }).version).toBe("0.1.1");
  });
  it("refuses when the files and the last tag disagree", () => {
    expect(() => nextRelease({ lastTag: "v0.1.0", currentVersion: "0.2.0", commits: [] })).toThrow(/files say 0.2.0/);
  });
});

describe("draftNotes", () => {
  it("groups user-facing commits and leaves the rest out", () => {
    const notes = draftNotes({
      commits: [
        c("feat(snooze): snooze a reminder (#14)"),
        c("fix: the menu opened on the wrong screen (#15)"),
        c("perf: open Preferences faster"),
        c("docs: typo"),
        c("ci: pin actions"),
        c("chore(deps): bump react"),
        c("refactor!: new data file format"),
      ],
      unsigned: false,
    });
    expect(notes).toBe(
      [
        "### New",
        "",
        "- Snooze a reminder.",
        "",
        "### Fixed",
        "",
        "- The menu opened on the wrong screen.",
        "",
        "### Changed",
        "",
        "- Open Preferences faster.",
        "- **Breaking:** New data file format.",
        "- Updated the libraries Friends is built on.",
      ].join("\n"),
    );
  });
  it("keeps what is under [Unreleased], mapping old headings, and adds the unsigned note once", () => {
    const unreleased = "### Added\n- A new look.\n- Tests,\n  run on every PR.\n\n### Removed\n- An old button.\n";
    const notes = draftNotes({ unreleased, commits: [c("feat: a new look")] });
    const groups = parseSections(notes);
    expect(groups.New).toEqual(["- A new look.", "- Tests,\n  run on every PR."]);
    expect(groups.Changed).toEqual(["- Removed: An old button."]);
    expect(parseSections("### Removed\n* A star bullet.\nNot a bullet.").Changed).toEqual([
      "* Removed: A star bullet.",
      "- Removed: Not a bullet.",
    ]);
    expect(groups["Good to know"]).toEqual([`- ${UNSIGNED_NOTE}`]);
    expect(parseSections(draftNotes({ unreleased: notes }))["Good to know"]).toHaveLength(1);
  });
  it("writes sentences", () => {
    expect(sentence("add x")).toBe("Add x.");
    expect(sentence("Done!")).toBe("Done!");
  });
});

const CHANGELOG = `# Changelog

Intro.

## [Unreleased]

### New
- Something.

## [0.1.0] - 2026-11-19

### Fixed
- Old fix.
`;

describe("CHANGELOG.md", () => {
  it("reads a section's body", () => {
    expect(sectionBody(CHANGELOG, "0.1.0")).toBe("### Fixed\n- Old fix.");
    expect(sectionBody(CHANGELOG, "Unreleased")).toBe("### New\n- Something.");
    expect(sectionBody(CHANGELOG, "0.2.0")).toBeNull();
    expect(sectionBody("## [0.1.00] - x\n- no\n", "0.1.0")).toBeNull();
  });
  it("inserts a release under an emptied [Unreleased]", () => {
    const out = insertRelease(CHANGELOG, "0.2.0", "2026-12-01", "### New\n\n- Something.\n");
    expect(out).toBe(`# Changelog

Intro.

## [Unreleased]

## [0.2.0] - 2026-12-01

### New

- Something.

## [0.1.0] - 2026-11-19

### Fixed
- Old fix.
`);
    expect(sectionBody(out, "Unreleased")).toBe("");
    expect(sectionBody(out, "0.2.0")).toBe("### New\n\n- Something.");
  });
  it("inserts the first release into a file with only [Unreleased]", () => {
    const out = insertRelease("# Changelog\n\n## [Unreleased]\n\n- x\n", "0.1.0", "2026-11-19", "### New\n- x");
    expect(out).toBe("# Changelog\n\n## [Unreleased]\n\n## [0.1.0] - 2026-11-19\n\n### New\n- x\n");
  });
  it("refuses bad input", () => {
    expect(() => insertRelease(CHANGELOG, "0.1.0", "2026-12-01", "- x")).toThrow(/already has/);
    expect(() => insertRelease(CHANGELOG, "0.2.0", "2026-12-01", "  \n")).toThrow(/empty/);
    expect(() => insertRelease(CHANGELOG, "0.2.0", "12/01", "- x")).toThrow(/date/);
    expect(() => insertRelease(CHANGELOG, "0.2", "2026-12-01", "- x")).toThrow(/version/);
    expect(() => insertRelease(CHANGELOG, "0.2.0", "2026-12-01", "## [9.9.9]\n- x")).toThrow(/headings/);
    expect(() => insertRelease("# Changelog\n", "0.2.0", "2026-12-01", "- x")).toThrow(/Unreleased/);
  });
  it("leaves the rest of the file byte for byte, and stops sections at link lines", () => {
    const file =
      "# Changelog\n\n## [Unreleased]\n\n- x\n\n## [0.1.0] - 2026-11-19\n\n- a\n\n\n\n```\n\n\n```\n\n" +
      "[Unreleased]: https://example.com/compare\n[0.1.0]: https://example.com/v0.1.0\n";
    const out = insertRelease(file, "0.2.0", "2026-12-01", "- x");
    expect(out).toBe(
      "# Changelog\n\n## [Unreleased]\n\n## [0.2.0] - 2026-12-01\n\n- x\n\n## [0.1.0] - 2026-11-19\n\n- a\n\n\n\n```\n\n\n```\n\n" +
        "[Unreleased]: https://example.com/compare\n[0.1.0]: https://example.com/v0.1.0\n",
    );
    expect(sectionBody(file, "0.1.0")).toBe("- a\n\n\n\n```\n\n\n```");
  });
  it("finds [Unreleased] lines the notes would lose", () => {
    const unreleased = "### Added\n- A new look.\n- Tests,\n  run on every PR.\n\n### Removed\n- An old button.";
    expect(unreleasedLinesMissing(unreleased, "### New\n\n- A new look.\n- Tests, run\n  on every PR.")).toEqual([
      "- An old button.",
    ]);
    expect(unreleasedLinesMissing("", "- x")).toEqual([]);
  });
  it("reads this repo's CHANGELOG.md", () => {
    expect(sectionBody(repoFile("CHANGELOG.md"), "Unreleased")).not.toBeNull();
  });
});

describe("version files", () => {
  it("changes only the version in this repo's JSON files", () => {
    for (const [p, opts] of [
      ["package.json", {}],
      ["src-tauri/tauri.conf.json", {}],
      ["package-lock.json", { lockfile: true }],
    ]) {
      const text = repoFile(p);
      const version = JSON.parse(text).version;
      // Writing the same version back gives the same bytes: nothing else in the file moves.
      expect(setJsonVersion(text, version, opts)).toBe(text);
      const bumped = JSON.parse(setJsonVersion(text, "9.8.7", opts));
      expect(bumped.version).toBe("9.8.7");
      if (opts.lockfile) expect(bumped.packages[""].version).toBe("9.8.7");
    }
  });
  it("changes only the [package] version of a Cargo.toml", () => {
    const toml = '[package]\nname = "friends"\nversion = "0.1.0"\n\n[dependencies]\nfoo = { version = "1" }\n';
    expect(setCargoTomlVersion(toml, "0.2.0")).toBe(toml.replace('version = "0.1.0"', 'version = "0.2.0"'));
    expect(() => setCargoTomlVersion('[dependencies]\nversion = "1"\n', "0.2.0")).toThrow(/package/);
    const real = repoFile("src-tauri/Cargo.toml");
    expect(changedLines(real, setCargoTomlVersion(real, "9.8.7"))).toEqual([
      'version = "9.8.7"',
    ]);
  });
  it("changes only the app's entry in Cargo.lock", () => {
    const lock = repoFile("Cargo.lock");
    const out = setCargoLockVersion(lock, "friends", "9.8.7");
    expect(out).toContain('name = "friends"\nversion = "9.8.7"\n');
    expect(changedLines(lock, out)).toEqual(['version = "9.8.7"']);
    expect(() => setCargoLockVersion(lock, "nope", "1.0.0")).toThrow(/no package/);
  });
});
