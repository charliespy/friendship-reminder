# Security

## Reporting a problem

Please don't open a public issue for a security problem. Report it privately through
GitHub: on this repository, go to **Security → Report a vulnerability**. You'll get a reply
within a week.

## What Friends keeps

Friends stores your list of people, blocked dates and settings in one JSON file in
`~/Library/Application Support/io.github.charliespy.friends/`. Notifications are local macOS
notifications. There are no analytics and no sync: your people, blocked dates and settings never
leave the Mac.

Signing in is optional. You can sign in with Apple to set up a Friends account, ready for a
Friends app on iPhone; nothing syncs yet. Only then does Friends use the network: it opens Sign in
with Apple in your browser, listens on a one-off `127.0.0.1` address until the browser comes back
(for up to 10 minutes), and talks to the Friends account server (run on Supabase) to sign you in,
to check and renew your sign-in (at launch, for example) and to sign you out. The account server
gets what Apple shares when you sign in: an account ID and your email address, or Apple's private
relay address if you chose to hide it. Friends keeps the session in the macOS Keychain. If you
never sign in, Friends sends nothing over the network.

## Supported versions

Only the latest release gets fixes.
