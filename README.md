# Friendship Reminder

A small macOS menu bar app, and an iPhone app, that remind you to stay in touch with people.

Add the people you want to see, say how often (every month, three months, six months or
year), and Friendship Reminder picks a day to aim for and reminds you before it. It skips the dates you've
blocked (trips, busy weeks), can aim only for weekends or only for weekdays, and if a day to
aim for goes by without you reaching out, it keeps them on your list and aims for the next good
days.

No tracking. Your list stays on your device unless you sign in with Apple: then it syncs with your
Friendship Reminder account, so your Mac and iPhone show the same list.

<p>
  <img src="docs/screenshots/dropdown-light.png" width="300" alt="The Friendship Reminder menu bar dropdown: three people to reach out to now, each with a Done button, and three coming up">
  <img src="docs/screenshots/dropdown-dark.png" width="300" alt="The same dropdown in dark mode">
</p>
<p>
  <img src="docs/screenshots/preferences-people.png" width="600" alt="Preferences, People: everyone you keep up with, how often, and when their next reminder comes">
</p>

<p>
  <img src="docs/screenshots/iphone-today.png" width="260" alt="The iPhone app's Today tab: three people to reach out to now, three coming up, and a tab bar with Today, People, Blocked and Settings">
  <img src="docs/screenshots/iphone-people-dark.png" width="260" alt="The iPhone app's People tab in dark mode">
</p>

The people in these screenshots are made up.

## Install

1. Download `Friendship.Reminder_<version>_universal.dmg` from the
   [latest release](https://github.com/charliespy/friendship-reminder/releases/latest).
2. Open it and drag **Friendship Reminder** to **Applications**.
3. Open **Friendship Reminder**. It is signed with an Apple Developer ID and notarized by Apple, so macOS
   only asks once whether you want to open an app downloaded from the internet. (Versions before
   0.1.2 weren't: macOS blocks their first launch until you click **Open Anyway** in
   **System Settings → Privacy & Security**.)
4. Friendship Reminder appears in the menu bar and the Dock. Click the menu bar icon to see who's due;
   click the Dock icon or open **Preferences** to add people.

Requires macOS 13 (Ventura) or newer.

## Build from source

You need [Node.js](https://nodejs.org/) 22+ and [Rust](https://rustup.rs/).

```sh
npm install
npm run tauri dev        # run the app
npm run tauri build      # build Friendship Reminder.app and a .dmg into target/release/bundle/
```

### Sign in with Apple

Signing in happens in the browser: Friendship Reminder opens Sign in with Apple there, and the browser comes
back to Friendship Reminder at `http://127.0.0.1:<port>/auth/callback` when it's done. (Apple's own sign-in
sheet only works in Mac App Store apps.) A build offers it only when it has the account server's settings,
`FRIENDS_SUPABASE_URL` and `FRIENDS_SUPABASE_PUBLISHABLE_KEY`; otherwise the Account section in
Settings says sign-in isn't available, and everything else works the same. Release builds take them
from the environment when they are built. `npm run tauri dev` also reads them from `.env.local` at
the repo root, which git ignores:

```sh
FRIENDS_SUPABASE_URL=https://<project>.supabase.co
FRIENDS_SUPABASE_PUBLISHABLE_KEY=sb_publishable_...
```

For a release build with them: `set -a; . ./.env.local; set +a; npm run tauri build`.

### Sync

While signed in, Friendship Reminder syncs the list with the account's database: at launch, right after
signing in, a few seconds after each change, and every five minutes. The tables and their privacy
rules (each account sees only its own rows) are in `supabase/migrations/`: run them once in the
project's SQL Editor. Only what you typed is synced (names, how often, the last time in touch,
blocked dates and the day setting); each device works out targets and reminders itself, and keeps
its own notification setting. When two devices change the same thing, the later change wins;
if both carry the same time, the one the account got last wins, on every device.
Signing in to a different account than the list last synced with saves the list to a copy next to
the data file (`friends.json.other-account-<n>`) and starts from that account's list. The rules
are in `crates/friends-core/src/sync.rs`, the requests in `crates/friends-sync`.

The Supabase project needs the Apple provider on (with a Services ID and its secret key) and
`http://127.0.0.1:*/auth/callback` in its allowed redirect URLs.

### iPhone

The iPhone app is the same app built for iOS: the same rules and data (`crates/`), the same screens
laid out as four tabs (`src/Phone.tsx`), the same colours. Its shell differs from the Mac's only
where the phone does:

- **Reminders are booked ahead.** iOS stops the app soon after it leaves the screen, so it can't
  check every minute like the Mac app. Whenever the list changes, it hands iOS the coming
  reminders (up to 60), each for 9:00 in the morning of its day. Reminders from earlier days count
  as sent: the person shows in Today instead.
- **Sign in with Apple is Apple's own sheet** (`ASAuthorizationController`, in
  `src-tauri/plugins/phone-sheets`). It hands the app an identity token, which Supabase trades for a
  session, so add the iPhone app's ID (`io.github.charliespy.friends`) to the Apple provider's
  Client IDs in Supabase. The app needs the Sign in with Apple capability, which the release build
  turns on by itself.
- **Sharing only some contacts** (iOS 18): whoever the user chose in Apple's sheet goes straight to
  "how often", and Import from Contacts opens Apple's sheet for sharing more (also in
  `plugins/phone-sheets`).
- Its home-screen name is **Friendship**: iOS cuts longer names short.
- No menu bar, Dock icon or open at login.

To build it you also need Xcode (with the iOS simulator), [rustup](https://rustup.rs/)'s Rust
with the iOS targets, CocoaPods and XcodeGen:

```sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim
rustup component add llvm-tools                            # for builds for a real iPhone
brew install cocoapods xcodegen
npm run tauri ios dev                                      # run it in the simulator
npm run tauri ios build -- --debug --target aarch64-sim    # or build a simulator app
```

It isn't on the App Store yet. Test builds go to TestFlight with `npm run ios:testflight`, which
needs an Apple Developer team and an App Store Connect API key: see
[docs/RELEASING.md](docs/RELEASING.md#iphone-a-testflight-build).

## Test

```sh
npm run check            # types, lint, unit tests, Rust checks
npm run check:ios        # the iPhone shell compiles (needs the iOS Rust targets)
npm run e2e              # end-to-end tests (Playwright, WebKit)
```

Every PR runs these on GitHub, plus a privacy scan and a check that the title is a conventional
commit. They all have to pass before anything reaches `main`; CONTRIBUTING.md explains how.

## How the reminders work

- Each person has a frequency. The next day to aim for is their last contact plus that many
  days, moved forward past blocked dates and to an allowed day: weekends only, weekdays only,
  or the days of the week you picked under **Custom day**.
- When the day to aim for is a Saturday or Sunday, the reminder comes on the Monday before, so
  there's time to plan. A weekday is reminded on the day itself.
- Mark someone **Done** from the menu bar when you've been in touch. Friendship Reminder asks when: on the
  day it suggested (once that day has come), today, or another day from a calendar. Their next
  day is scheduled from that day.
- If the day passes without **Done**, Friendship Reminder keeps them in the list. It counts the day as
  missed only once the rest of that weekend (under weekends only) or that week (under weekdays
  only) is over too; under Custom day, just that day. Then it aims for the next run of
  allowed days, past any blocked dates.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Please report security problems privately, as
[SECURITY.md](SECURITY.md) explains.

## License

[MIT](LICENSE)
