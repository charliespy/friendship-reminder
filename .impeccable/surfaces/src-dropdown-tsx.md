---
version: 1
slug: "src-dropdown-tsx"
primary_target: "src/Dropdown.tsx"
related_targets: ["src/Preferences.tsx","src/PeoplePane.tsx","src/BlockedPane.tsx","src/SettingsPane.tsx","src/styles.css"]
---

# Friends: menu bar dropdown and Preferences

Scope: the dropdown (src/Dropdown.tsx) and the Preferences window (src/Preferences.tsx and the three
panes). Mode: Operate. Audience and job: see PRODUCT.md (a glance from the menu bar, Done in one
click; Preferences opened rarely). Constraints: native macOS feel pinned by the brief, light and
dark mode, WebKit only, no web fonts, test names and labels unchanged.

Decided without the owner (unattended run, no question channel): the roll's assigned direction,
code-led (Operate surface).

## Direction contract

THESIS: Letterpress stationery, two inks on good card stock: a calm Mac utility whose colour is
printed, not glowing. Refuses the category default of a grey list with one blue accent and the
incumbent cream-paper-and-terracotta.

OWN-WORLD: Cool fog stock ground (never cream), white card groups, text in deep indigo ink instead
of black, one press-blue spot ink for actions and selection, sea-green spot for done and caught
up, amber (never red) for missed; red only for delete. Flat colour, no gradients or glass. Sharp
SF Pro type with tabular figures; avatars are ink-printed discs from the two spot inks plus
plum and amber.

STORY: Open the menu bar, see who to reach out to now and who's next, press Done. Preferences is a
quiet native settings window: a source-list sidebar and inset grouped lists.

FIRST VIEWPORT: Dropdown fills its window edge to edge (square, matching the native window).
Header row: "Friends" at 13pt semibold left, today's date right in secondary ink. "Reach out now":
a white inset group, one 48px row per person (disc, name, aim line, a sea-green tinted Done; Done is sea green wherever it appears).
"Coming up": a plain list in date order, reminder line under each name. Footer: Preferences… and
Quit Friends as menu-style text buttons on a hairline.

FORM: candidate 7 of 7 (address book, wall calendar, postcards, airmail, photo booth, fridge door,
letterpress stationery). seed:38cd4d2c64d65ba5. Raises: from the developer console, delete isolated
by space at the far end of the selection bar; from the botanical folio, one fixed row scale and
baseline across dropdown and Preferences; from the zoo map, flat unmodulated colour only; from
labanotation, time runs one way (upcoming strictly by date).

Signature move: the impression. Done and every push button press into the stock (inset shadow, 1px
down) instead of glowing, and a person's disc is a printed impression (a faint inner shadow).

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
