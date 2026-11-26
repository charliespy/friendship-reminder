# Changelog

What changed in each version of Friendship Reminder (called Friends until v0.1.2), for the
people who use it. Each version's section is also its description on [GitHub Releases](https://github.com/charliespy/friendship-reminder/releases),
word for word, so it is short and in plain words, under **New**, **Fixed**, **Changed** and
**Good to know**. Versions follow [Semantic Versioning](https://semver.org/):
[CONTRIBUTING.md](CONTRIBUTING.md#releases) has the rule, and
[docs/RELEASING.md](docs/RELEASING.md) the steps.

## [Unreleased]

### New

- **Import from Contacts**: Add person can now add one or several people from Contacts at once.
  Search, tick who you want, pick how often for each, and add them. Friendship Reminder reads only
  their names, and adds a link back to the app on their cards: tap it in Contacts to open
  Friendship Reminder at that person.
- A short welcome when you first open Friendship Reminder: sign in with Apple (or choose Not now),
  then pick people from Contacts (or Skip).

## [0.2.0] - 2026-11-25

### New

- You can sign in with Apple in Preferences → Settings. While you're signed in, your people,
  blocked dates and day setting sync with your Friendship Reminder account. Signed out, everything
  stays on this Mac as before.
- **Custom day** replaces "Any day" in Settings: pick the days of the week you're free, and
  Friendship Reminder only aims for those. All seven start picked, which works the way "Any day"
  did. Changes to these days now wait for **Save** (or Discard), since saving reschedules everyone.
- **Done** now asks when you were in touch: on the suggested day, today, or another day from a
  calendar. The next reminder counts from that day.
- An iPhone version is on the way: sign in with Apple on both to keep them in sync.

### Changed

- Friends is now called **Friendship Reminder**. Your list, your sign-in and "Open at login" carry
  over. It installs next to the old Friends app: quit Friends, open Friendship Reminder from
  Applications once, then move Friends to the Bin.
- Friendship Reminder now also has a Dock icon. Click it to open Preferences; the menu bar icon
  works as before.
- A calmer look that matches the Mac's own apps, like System Settings: neutral greys and one
  colour, your Mac's accent colour. Missed days are still orange.

### Good to know

- After you update, the older version of the app can't read a list that uses Custom day, so update
  every Mac you use.

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
