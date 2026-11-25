---
version: 1
slug: "src-dropdown-tsx"
primary_target: "src/Dropdown.tsx"
related_targets: ["src/Preferences.tsx","src/PeoplePane.tsx","src/BlockedPane.tsx","src/SettingsPane.tsx","src/styles.css"]
---

# Friends: menu bar dropdown and Preferences

Scope: the dropdown (src/Dropdown.tsx) and the Preferences window (src/Preferences.tsx and the three
panes). Mode: Operate. Audience and job: see PRODUCT.md (a glance from the menu bar, Done in one
click; Preferences opened rarely). Constraints: looks like it came with the Mac, light and dark mode,
WebKit only, no web fonts, test names and labels unchanged.

Direction: the category standard (stock macOS), chosen by the user over the roll in November 2026
because the previous letterpress world felt cold, corporate and too colourful. Reference app: System
Settings. Code-led (Operate surface).

## Direction contract

THESIS: Friends looks like an app Apple shipped: System Settings' greys, groups and controls, with
the Mac's own accent as the only colour. Refuses a custom palette of spot inks.

OWN-WORLD: Window grey #ececec (dark #1e1e1e), white inset groups with a hairline edge, the system
label greys, standard push buttons, Contacts-style grey monogram discs. One colour: the user's
accent (-apple-system-control-accent, system blue fallback) for the primary button, selected
sidebar row, count tag, checkboxes and focus. Orange for missed only, red for delete and errors only.

STORY: Open the menu bar, see who to reach out to now and who's next, press Done. Preferences reads
as one more System Settings window.

FIRST VIEWPORT: Dropdown edge to edge: "Friends" left, date right; "Reach out now" in a white group,
48px rows (grey disc, name, meta line, plain Done push button); "Coming up" as a plain list on the
window grey; footer of menu-style text buttons. Preferences: 168px source list with an accent-filled
selected row, pane with a 20pt title and inset groups.

FORM: the category standard played straight (canon), reference System Settings. seed:e23ec5dabcd89b44

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
