// The rules behind `scripts/release.mjs`: how the next version is worked out, how release notes
// are drafted, and how a release is written into CHANGELOG.md and the version files.
// Pure functions (no git, no files, no network), so they are unit-tested in release-lib.test.mjs.
//
// The version rule (semantic versioning, from the conventional commits since the last release):
//   feat -> minor; fix, perf, revert -> patch; a breaking change ("!" or "BREAKING CHANGE") -> major,
//   except that while the version is 0.x a breaking change bumps the minor.
//   Other types (docs, ci, chore, test, refactor, build, style) don't release on their own.

/** Headings of a release section, in the order they are written. */
export const SECTIONS = ["New", "Fixed", "Changed", "Good to know"];

/** Older Keep a Changelog headings, and the section each one is written under. "Removed" and
 * "Deprecated" lines keep that word, as "Removed: ...", so the meaning survives the move. */
const HEADING_ALIASES = {
  new: "New",
  added: "New",
  fixed: "Fixed",
  changed: "Changed",
  removed: "Changed",
  deprecated: "Changed",
  security: "Fixed",
  "good to know": "Good to know",
};

/** What a user of an unsigned build needs to open it the first time. */
export const UNSIGNED_NOTE =
  "Friends isn't signed with an Apple Developer ID yet, so macOS blocks the first launch. " +
  "Open it once and click **Done**, then go to **System Settings → Privacy & Security** and " +
  "click **Open Anyway** (the README has the details).";

const VERSION_RE = /^(\d+)\.(\d+)\.(\d+)$/;

/** "1.2.3" -> [1, 2, 3]; throws on anything else (no pre-releases: this project doesn't use them). */
export function parseVersion(version) {
  const m = VERSION_RE.exec(String(version).trim());
  if (!m) throw new Error(`not a version (want X.Y.Z): ${version}`);
  return [Number(m[1]), Number(m[2]), Number(m[3])];
}

/** Windows line endings (a file or a description edited on github.com) -> "\n". */
export function lf(text) {
  return text.replace(/\r\n?/g, "\n");
}

/** The newest vX.Y.Z tag among `tags`, or null. Other tags are ignored. */
export function latestReleaseTag(tags) {
  let best = null;
  for (const tag of tags) {
    const m = /^v(\d+\.\d+\.\d+)$/.exec(tag.trim());
    if (!m) continue;
    if (!best || compareVersions(m[1], best.slice(1)) > 0) best = `v${m[1]}`;
  }
  return best;
}

export function compareVersions(a, b) {
  const [x, y] = [parseVersion(a), parseVersion(b)];
  for (let i = 0; i < 3; i++) if (x[i] !== y[i]) return x[i] - y[i];
  return 0;
}

const SUBJECT_RE = /^(\w+)(?:\(([^()]*)\))?(!)?: (.+)$/;

/**
 * A commit's subject and body -> { type, scope, breaking, description }, or null when the subject
 * isn't a conventional commit. A squash merge's trailing " (#12)" is dropped from the description.
 */
export function parseCommit(subject, body = "") {
  const m = SUBJECT_RE.exec(subject.trim());
  if (!m) return null;
  const description = m[4].replace(/\s*\(#\d+\)\s*$/, "").trim();
  return {
    type: m[1].toLowerCase(),
    scope: m[2] ?? null,
    breaking: m[3] === "!" || /^BREAKING[ -]CHANGE: /m.test(body),
    description,
  };
}

/** The bump one parsed commit asks for: "major", "minor", "patch" or null. */
export function commitBump(commit) {
  if (commit.breaking) return "major";
  if (commit.type === "feat") return "minor";
  if (commit.type === "fix" || commit.type === "perf" || commit.type === "revert") return "patch";
  return null;
}

const RANK = { patch: 1, minor: 2, major: 3 };

/** The biggest bump the commits ask for, or null when none of them releases anything. */
export function bumpFor(commits) {
  let bump = null;
  for (const c of commits) {
    const b = commitBump(c);
    if (b && (!bump || RANK[b] > RANK[bump])) bump = b;
  }
  return bump;
}

/** The bump that really applies to `version`: on 0.x a major bump becomes a minor one. */
export function effectiveBump(version, bump) {
  return bump === "major" && parseVersion(version)[0] === 0 ? "minor" : bump;
}

/** Applies a bump (after effectiveBump). */
export function applyBump(version, bump) {
  const [major, minor, patch] = parseVersion(version);
  bump = effectiveBump(version, bump);
  if (bump === "major") return `${major + 1}.0.0`;
  if (bump === "minor") return `${major}.${minor + 1}.0`;
  if (bump === "patch") return `${major}.${minor}.${patch + 1}`;
  throw new Error(`unknown bump: ${bump}`);
}

/**
 * Works out the next release.
 *   lastTag: the newest vX.Y.Z tag, or null before the first release.
 *   currentVersion: the version in the app's files.
 *   commits: parsed commits since lastTag (non-conventional ones already left out).
 *   forcePatch: release a patch even when no commit asks for one (dependency updates only).
 * Returns { version, bump, reason } or { version: null, reason } when there is nothing to release.
 */
export function nextRelease({ lastTag, currentVersion, commits, forcePatch = false }) {
  parseVersion(currentVersion);
  if (!lastTag) {
    return {
      version: currentVersion,
      bump: null,
      reason: `first release: no vX.Y.Z tag yet, so it is the version already in the files (${currentVersion})`,
    };
  }
  const base = lastTag.slice(1);
  if (compareVersions(currentVersion, base) !== 0) {
    throw new Error(
      `the files say ${currentVersion} but the last release is ${lastTag}: fix that first (a release PR merged without its tag?)`,
    );
  }
  let bump = bumpFor(commits);
  let why = bump && describeBump(commits, bump);
  if (!bump && forcePatch) {
    bump = "patch";
    why = "--patch: no feat or fix since the last release, released as a patch on request";
  }
  if (!bump) {
    return {
      version: null,
      reason: `nothing to release since ${lastTag}: no feat, fix, perf, revert or breaking change (use --patch to release anyway)`,
    };
  }
  const applied = effectiveBump(base, bump);
  return {
    version: applyBump(base, applied),
    bump: applied,
    reason: applied === bump ? why : `${why}; on 0.x a breaking change bumps the ${applied}`,
  };
}

function describeBump(commits, bump) {
  const hits = commits.filter((c) => commitBump(c) === bump).length;
  const what = { major: "breaking change", minor: "feat", patch: "fix/perf/revert" }[bump];
  return `${hits} ${what} commit${hits === 1 ? "" : "s"} since the last release -> ${bump}`;
}

/** The section a commit's line goes under in the notes, or null when users don't need it. */
export function sectionFor(commit) {
  if (commit.type === "feat") return "New";
  if (commit.type === "fix") return "Fixed";
  if (commit.type === "perf" || commit.type === "revert") return "Changed";
  if (commit.breaking) return "Changed";
  return null;
}

/** "add a snooze button" -> "Add a snooze button." */
export function sentence(text) {
  const t = text.trim();
  if (!t) return t;
  const s = t[0].toUpperCase() + t.slice(1);
  return /[.!?)]$/.test(s) ? s : `${s}.`;
}

/**
 * Drafts the notes for a release: the lines already under [Unreleased] in CHANGELOG.md, then one
 * line per user-facing commit, grouped under New / Fixed / Changed, then "Good to know".
 * The draft is a starting point: the person releasing rewrites it into plain words.
 *   unreleased: the body of the [Unreleased] section (may be empty).
 *   commits: parsed commits since the last release ([] before the first release, whose notes
 *     come from [Unreleased] alone).
 *   unsigned: add the note on opening an unsigned app.
 */
export function draftNotes({ unreleased = "", commits = [], unsigned = true }) {
  const groups = parseSections(unreleased);
  for (const c of commits) {
    const section = sectionFor(c);
    if (!section) continue;
    const line = `- ${c.breaking ? "**Breaking:** " : ""}${sentence(c.description)}`;
    if (!groups[section].includes(line)) groups[section].push(line);
  }
  if (commits.some((c) => c.scope === "deps" && (c.type === "chore" || c.type === "build"))) {
    const line = "- Updated the libraries Friends is built on.";
    if (!groups.Changed.includes(line)) groups.Changed.push(line);
  }
  if (unsigned && !groups["Good to know"].some((l) => l.includes("Open Anyway"))) {
    groups["Good to know"].push(`- ${UNSIGNED_NOTE}`);
  }
  return renderSections(groups);
}

/** A release body -> { New: [...lines], Fixed: [...], Changed: [...], "Good to know": [...] }. */
export function parseSections(body) {
  const groups = Object.fromEntries(SECTIONS.map((s) => [s, []]));
  let current = "Changed";
  let prefix = "";
  let last = null;
  for (const raw of body.split("\n")) {
    const line = raw.replace(/\s+$/, "");
    const h = /^###\s+(.+)$/.exec(line);
    if (h) {
      const name = h[1].trim().toLowerCase();
      current = HEADING_ALIASES[name] ?? "Changed";
      prefix = name === "removed" || name === "deprecated" ? `${name[0].toUpperCase()}${name.slice(1)}: ` : "";
      last = null;
      continue;
    }
    if (!line.trim()) {
      last = null;
      continue;
    }
    if (/^\s+\S/.test(line) && last !== null) {
      // A wrapped line of the bullet above: keep it with that bullet.
      groups[current][last] += `\n${line}`;
      continue;
    }
    groups[current].push(prefix ? withPrefix(line, prefix) : line);
    last = groups[current].length - 1;
  }
  return groups;
}

/** "- An old button." -> "- Removed: An old button."; a line that isn't a bullet becomes one. */
function withPrefix(line, prefix) {
  const m = /^([-*+]) (.*)$/.exec(line);
  return m ? `${m[1]} ${prefix}${m[2]}` : `- ${prefix}${line.trim()}`;
}

export function renderSections(groups) {
  return SECTIONS.filter((s) => groups[s]?.length)
    .map((s) => `### ${s}\n\n${groups[s].join("\n")}`)
    .join("\n\n");
}

/** A Keep a Changelog link line at the end of the file, like "[0.1.0]: https://...". */
const LINK_REF_RE = /^\[[^\]]+\]:\s/;

/** Finds `## [<name>]` in a changelog: { start, bodyStart, end } line indexes, or null. A section
 * ends at the next "## " heading or link line. */
function findSection(lines, name) {
  const head = `## [${name}]`;
  const start = lines.findIndex((l) => l === head || l.startsWith(`${head} `));
  if (start < 0) return null;
  let end = lines.length;
  for (let i = start + 1; i < lines.length; i++) {
    if (lines[i].startsWith("## ") || LINK_REF_RE.test(lines[i])) {
      end = i;
      break;
    }
  }
  return { start, bodyStart: start + 1, end };
}

/** The body of `## [<name>]` (e.g. "0.1.0" or "Unreleased"), trimmed; "" when it has none, null when missing. */
export function sectionBody(changelog, name) {
  const lines = changelog.split("\n");
  const s = findSection(lines, name);
  if (!s) return null;
  return lines.slice(s.bodyStart, s.end).join("\n").trim();
}

/**
 * Writes a release into CHANGELOG.md: an empty `## [Unreleased]` stays at the top, and
 * `## [<version>] - <date>` with the notes goes right under it. Throws when that version is
 * already there or the file has no [Unreleased] section.
 */
export function insertRelease(changelog, version, date, notes) {
  parseVersion(version);
  const day = new Date(`${date}T00:00:00Z`);
  if (!/^\d{4}-\d{2}-\d{2}$/.test(date) || Number.isNaN(day.getTime()) || day.toISOString().slice(0, 10) !== date) {
    throw new Error(`not a date (want YYYY-MM-DD): ${date}`);
  }
  const body = notes.trim();
  if (!body) throw new Error("the release notes are empty");
  if (/^## /m.test(body)) throw new Error('the notes may not contain "## " headings; use "### New" etc.');
  if (body.split("\n").some((l) => LINK_REF_RE.test(l))) {
    throw new Error('the notes may not contain "[name]: url" link lines (a section ends there); write links inline');
  }
  const lines = changelog.split("\n");
  if (findSection(lines, version)) throw new Error(`CHANGELOG.md already has a [${version}] section`);
  const u = findSection(lines, "Unreleased");
  if (!u) throw new Error("CHANGELOG.md has no ## [Unreleased] section");
  // Only the lines around the new section are tidied; the rest of the file stays byte for byte.
  const before = lines.slice(0, u.start).join("\n").replace(/\n*$/, "");
  const after = lines.slice(u.end).join("\n").replace(/^\n*/, "").replace(/\n*$/, "");
  const inserted = `## [Unreleased]\n\n## [${version}] - ${date}\n\n${body}\n`;
  return `${before ? `${before}\n\n` : ""}${inserted}${after ? `\n${after}\n` : ""}`;
}

/** The versions that have a `## [X.Y.Z]` section in a changelog. */
export function changelogVersions(changelog) {
  return [...changelog.matchAll(/^## \[(\d+\.\d+\.\d+)\]/gm)].map((m) => m[1]);
}

/**
 * Checks the version given to "prepare": a first release (no tag) may keep the version in the
 * files or raise it; a later one must be above the last release, from files still at the last
 * release. A release already prepared but not tagged (a CHANGELOG.md section newer than the last
 * tag) must be undone first. Throws otherwise.
 */
export function checkNewVersion({ version, currentVersion, lastTag, changelog = "" }) {
  parseVersion(version);
  const base = lastTag ? lastTag.slice(1) : null;
  const pending = changelogVersions(changelog).filter((v) => !base || compareVersions(v, base) > 0);
  if (pending.length) {
    throw new Error(
      `CHANGELOG.md already has [${pending[0]}], which isn't tagged yet: a release is already prepared (undo it with git first)`,
    );
  }
  if (!lastTag) {
    if (compareVersions(version, currentVersion) < 0) {
      throw new Error(`${version} is below the version in the files (${currentVersion})`);
    }
    return;
  }
  if (compareVersions(currentVersion, base) !== 0) {
    throw new Error(`the files say ${currentVersion} but the last release is ${lastTag}: fix that first`);
  }
  if (compareVersions(version, base) <= 0) {
    throw new Error(`${version} is not above the last release (${lastTag})`);
  }
}

/** The entries of a release body: each bullet with its wrapped lines, or a lone line. */
function entries(body) {
  const out = [];
  let open = false; // An indented line continues only the entry right above it.
  for (const line of body.split("\n")) {
    if (!line.trim() || /^#{1,6}\s/.test(line)) {
      open = false;
      continue;
    }
    if (/^\s+\S/.test(line) && open) out[out.length - 1] += `\n${line}`;
    else out.push(line);
    open = true;
  }
  return out;
}

const squash = (t) => t.replace(/^[-*+]\s+/, "").replace(/\s+/g, " ").trim().toLowerCase();

/**
 * The entries under [Unreleased] that aren't in the notes: "prepare" empties [Unreleased], so these
 * would be lost from CHANGELOG.md. An entry counts as kept when a notes entry has the same words,
 * maybe after the "Removed: " or "Deprecated: " the draft puts in front.
 */
export function unreleasedLinesMissing(unreleased, notes) {
  const have = entries(notes).map(squash);
  return entries(unreleased).filter((e) => {
    const want = squash(e);
    return !have.some((h) => h === want || h === `removed: ${want}` || h === `deprecated: ${want}`);
  });
}

/** Sets the top-level "version" of a JSON file's text (package.json, tauri.conf.json, package-lock.json). */
export function setJsonVersion(text, version, { lockfile = false } = {}) {
  const data = JSON.parse(text);
  if (typeof data.version !== "string") throw new Error('no top-level "version"');
  data.version = version;
  if (lockfile) {
    const root = data.packages?.[""];
    if (!root || typeof root.version !== "string") throw new Error('package-lock.json has no packages[""].version');
    root.version = version;
  }
  const indent = /^\{\n(\s+)"/.exec(text)?.[1] ?? "  ";
  return `${JSON.stringify(data, null, indent)}\n`;
}

/** Sets `version = "..."` in the [package] table of a Cargo.toml. */
export function setCargoTomlVersion(text, version) {
  const lines = text.split("\n");
  let inPackage = false;
  for (let i = 0; i < lines.length; i++) {
    const table = /^\s*\[([^\]]+)\]\s*$/.exec(lines[i]);
    if (table) {
      inPackage = table[1].trim() === "package";
      continue;
    }
    if (inPackage && /^version\s*=\s*"[^"]*"/.test(lines[i])) {
      lines[i] = lines[i].replace(/"[^"]*"/, `"${version}"`);
      return lines.join("\n");
    }
  }
  throw new Error("no version in the [package] table");
}

/** Sets the version of package `name` in a Cargo.lock. */
export function setCargoLockVersion(text, name, version) {
  const re = new RegExp(`(\\[\\[package\\]\\]\\nname = "${name.replace(/[-.]/g, "\\$&")}"\\nversion = ")[^"]*(")`);
  if (!re.test(text)) throw new Error(`Cargo.lock has no package "${name}"`);
  return text.replace(re, `$1${version}$2`);
}
