# Security

## Reporting a problem

Please don't open a public issue for a security problem. Report it privately through
GitHub: on this repository, go to **Security → Report a vulnerability**. You'll get a reply
within a week.

## What Friendship Reminder keeps

Friendship Reminder stores your list of people, blocked dates and settings in one JSON file in
`~/Library/Application Support/io.github.charliespy.friends/`. Notifications are local macOS
notifications. There are no analytics. Unless you sign in, your people, blocked dates and
settings never leave the Mac.

Signing in is optional. You can sign in with Apple to a Friendship Reminder account, which keeps your list in
sync, ready for a Friendship Reminder app on iPhone. Only then does Friendship Reminder use the network: it opens Sign in
with Apple in your browser, listens on a one-off `127.0.0.1` address until the browser comes back
(for up to 10 minutes), and talks to the Friendship Reminder account server (run on Supabase) to sign you in,
to check and renew your sign-in (at launch, for example) and to sign you out. The account server
gets what Apple shares when you sign in: an account ID and your email address, or Apple's private
relay address if you chose to hide it. Friendship Reminder keeps the session in the macOS Keychain.

While you're signed in, Friendship Reminder syncs your list with the account: each person's name, how often to
reach out and when you were last in touch; each blocked range's dates and label; and the "which
days count" setting. It sends them when they change and checks for changes from your other devices
every few minutes. Deleting something keeps an empty row on the server saying it was deleted, so
your other devices delete it too. The database's rules let each account read and write only its
own rows; signed-out requests get nothing. Reminder dates, which reminders were sent and the other
settings stay on the Mac. Signing out stops syncing; your list stays on the Mac. If you never sign
in, Friendship Reminder sends nothing over the network.

## Supported versions

Only the latest release gets fixes.
