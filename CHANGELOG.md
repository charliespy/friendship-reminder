# Changelog

What changed in each version of Friends, for the people who use it. Each version's section is also
its description on [GitHub Releases](https://github.com/charliespy/friendship-reminder/releases),
word for word, so it is short and in plain words, under **New**, **Fixed**, **Changed** and
**Good to know**. Versions follow [Semantic Versioning](https://semver.org/):
[CONTRIBUTING.md](CONTRIBUTING.md#releases) has the rule, and
[docs/RELEASING.md](docs/RELEASING.md) the steps.

## [Unreleased]

### New

- You can sign in with Apple in Preferences → Settings. While you're signed in, your list syncs
  with your Friends account, ready for a later iPhone app: the people, blocked dates and the day
  setting. Signed out, everything stays on this Mac as before.

### Changed

- Friends now also has a Dock icon. Click it to open Preferences; the menu bar icon works as
  before.

## [0.1.2] - 2026-11-21

### Changed

- The download itself is now also checked by Apple, so macOS opens the disk image without a
  warning too.

## [0.1.1] - 2026-11-21

### Fixed

- After you delete people, click Deselect all, or click Done or Remove in Preferences, the keyboard
  stays where you were instead of jumping back to the top of the window.

### Changed

- Friends is now signed with an Apple Developer ID and checked by Apple, so macOS opens it
  without the "unidentified developer" warning.

## [0.1.0] - 2026-11-19

### New

- A new look: calm, native macOS styling in light and dark mode.
- If Friends can't read its data folder when it starts, it tells you in an alert instead of
  quitting without a word.

### Changed

- Friends is now a new app. It keeps its own list of people and sends its own notifications,
  instead of writing to Apple Reminders.
- Removed the "Add Reminders for Today" and "Delete All Reminders" buttons. They only worked with
  Apple Reminders, which Friends no longer uses.

### Good to know

- Friends isn't signed with an Apple Developer ID yet, so macOS blocks the first launch. Open it once and click **Done**, then go to **System Settings → Privacy & Security** and click **Open Anyway** (the README has the details). On macOS 13 or 14, Control-click the app
  and choose **Open** instead.
- If you used the old version, your people, blocked dates and settings don't carry over: add your
  friends again. You can delete the old "Friends" list in Reminders.
