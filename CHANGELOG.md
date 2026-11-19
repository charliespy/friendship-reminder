# Changelog

What changed in each version of Friends, for the people who use it. Each version's section is also
its description on [GitHub Releases](https://github.com/charliespy/friendship-reminder/releases),
word for word, so it is short and in plain words, under **New**, **Fixed**, **Changed** and
**Good to know**. Versions follow [Semantic Versioning](https://semver.org/):
[CONTRIBUTING.md](CONTRIBUTING.md#releases) has the rule, and
[docs/RELEASING.md](docs/RELEASING.md) the steps.

## [Unreleased]

### New
- A new look: calm, native macOS styling in light and dark mode.
- If Friends can't read its data folder when it starts, it tells you in an alert instead of
  quitting without a word.

### Changed
- Friends is rebuilt as a new app (Tauri instead of Swift). It keeps its own list of people and
  sends its own notifications, instead of writing to Apple Reminders.
- Removed the "Add Reminders for Today" and "Delete All Reminders" buttons. They only worked with
  Apple Reminders, which Friends no longer uses.
