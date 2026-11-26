# Security

## Reporting a problem

Please don't open a public issue for a security problem. Report it privately through
GitHub: on this repository, go to **Security → Report a vulnerability**. You'll get a reply
within a week.

## What Friendship Reminder keeps

Friendship Reminder stores your list of people, blocked dates and settings in one JSON file on
each device: on the Mac in `~/Library/Application Support/io.github.charliespy.friends/`, on
iPhone in the app's own data folder. Notifications are local notifications. On the Mac the app
sends each one when it is due; on iPhone it hands iOS the coming ones ahead of time (up to 60,
each with the person's name in it), since iOS stops the app soon after it leaves the screen. iOS
keeps them until the app next opens and books them again, so a change made on another device
reaches them only then. There are no analytics. Unless you sign in, your people, blocked dates
and settings never leave the device, apart from the backups you make of it (Time Machine on the
Mac, iCloud or computer backups on iPhone).

Signing in is optional. You can sign in with Apple to a Friendship Reminder account, which keeps
your list in sync between the Mac and iPhone apps. Only then does Friendship Reminder use the
network. On the Mac it opens Sign in with Apple in your browser and listens on a one-off
`127.0.0.1` address until the browser comes back (for up to 10 minutes); on iPhone the same page
opens in a sheet over the app and comes back to `io.github.charliespy.friends://auth/callback`.
Either way, it talks to the Friendship Reminder account server (run on Supabase) to sign you in,
to check and renew your sign-in (at launch, for example) and to sign you out. The account server
gets what Apple shares when you sign in: an account ID and your email address, or Apple's private
relay address if you chose to hide it. Friendship Reminder keeps the session in the device's
Keychain.

While you're signed in, Friendship Reminder syncs your list with the account: each person's name, how often to
reach out and when you were last in touch; each blocked range's dates and label; and the "which
days count" setting. It sends them when they change and checks for changes from your other devices
every few minutes (on iPhone, while the app is open). Deleting something keeps an empty row on the
server saying it was deleted, so your other devices delete it too. The database's rules let each
account read and write only its own rows; signed-out requests get nothing. Reminder dates, which
reminders were sent and the other settings stay on the device. Signing out stops syncing; your
list stays on the device. If you never sign in, Friendship Reminder sends nothing over the network.

### Contacts

Add person › Import from Contacts (and the same step when you first open the app) lists the people
in your Contacts so you can pick who to add. macOS or iOS asks you first, and you can say no; you
can still type names in. Friendship Reminder reads only names (and, to spot people already in your
list, the web addresses on a card), never phone numbers, email addresses, addresses, birthdays or
photos. For each person you add, it keeps:

- their name, like any name you type in (synced while you're signed in);
- this device's id for their card, so the app can find the card again. It stays on the device and
  is never synced;
- a link written onto their card in Contacts, labelled "Friendship Reminder", for example
  `io.github.charliespy.friends://person/3f2a…`. The code at the end is the app's random id for the
  person, which says nothing about them. Contacts syncs the card (with iCloud), so your other
  devices find the person's card by this link, and tapping it opens Friendship Reminder at that
  person (the app registers this kind of link for that). It's the only change the app makes to your contacts,
  and only on the cards of people you add; you can delete it from the card at any time.

## Supported versions

Only the latest release gets fixes.
