# Product

<!-- impeccable:product-schema 1 -->

<!-- Inferred from the issue, README and code; correct anything that is wrong. -->

## Platform

web

(A Tauri app: the UI is HTML/CSS/React rendered by macOS's WebKit inside native windows. It ships
on macOS only, so its design language is macOS, not the web.)

## Users

One person on their own Mac who wants to keep up with friends, family and old colleagues, and
knows they drift apart when nothing prompts them. They glance at Friends from the menu bar a few
times a week, act in seconds ("I texted Maya: Done"), and open Preferences rarely: to add someone,
block a trip, or change which days count.

## Product Purpose

Friends reminds you to reach out to people at the rhythm you pick (every month, three months, six
months, or year). It picks a day to aim for, avoids blocked dates, can keep to weekends or
weekdays, and if a day to aim for goes by without contact, aims for the next good days. Success:
the people you care about hear from you about as often as you meant, and the app takes almost no
attention.

## Positioning

A personal reminder that lives in the menu bar, keeps everything on the Mac (no account, no sync,
no tracking), and is open source. It is not a contacts manager or a CRM: there are no notes,
fields, pipelines or scores, only a name, a rhythm, and the last time you were in touch.

## Operating Context

- The menu bar dropdown (360 x 520 pt, opens under the tray icon, closes when it loses focus or on
  Escape): who to reach out to now, with a Done button each, and the next few coming up.
- The Preferences window: People (add, edit, mark done, select and delete), Blocked dates (add,
  remove), Settings (which days count, notifications, open at login).
- macOS notifications on the reminder day; the menu bar shows the count of people due.
- Light and dark mode both, following the system.

## Capabilities and Constraints

- Data is one JSON file in the app's data folder; a damaged file is moved aside and reported.
- Dates are calendar days (no times). "Due" starts on the reminder day. "Missed" means the
  aimed-for day went by without Done, along with the rest of that weekend (under weekends only) or
  that week (under weekdays only); with every day allowed, just that day. A missed person stays due
  while the app aims for the next run of allowed days, past any blocked dates.
- macOS 13 or newer. WebKit only (Safari's engine), so CSS must work in Safari 16.
- Tests address the UI by accessible names, labels and a few `data-testid`s
  (`due-person`, `upcoming-person`, `person-row`, `blocked-row`); copy changes must keep them.
- The web page loads no web fonts and makes no network requests: the content security policy
  allows only the app's own files. Signing in to a Friends account (optional) is the only network
  use: the sign-in page opens in the browser, and the app's own requests go from the Rust side.

## Brand Commitments

- The name is **Friends**. Plain, warm, short sentences ("Reach out now", "You're all caught up.").
- No guilt or gamification: no streaks, scores, or red alarm states for a missed day.

## Evidence on Hand

No real people may appear anywhere (the repo is public). Screenshots and tests use made-up names
(Maya Chen, Theo Marsh, Ada). There are no testimonials, user counts or press.

## Product Principles

1. Glanceable first: the dropdown answers "who now?" in one look.
2. Quiet: it's a nudge, not a nag. Missed is information, not failure.
3. Native: it should feel like it came with the Mac.
4. Local and private by default.

## Accessibility & Inclusion

Full keyboard use, visible focus, labels on every control, and contrast that holds in light and
dark mode. Respect reduced motion.
