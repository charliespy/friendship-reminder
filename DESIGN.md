---
name: Friends
description: A quiet macOS menu bar reminder, printed in two inks on cool card stock.
colors:
  stock: "#eef0f4"
  sheet: "#ffffff"
  well: "#e4e7ee"
  sidebar: "#e6e9f0"
  ink: "#1d2233"
  ink-2: "#4f5568"
  ink-3: "#5c6275"
  rule: "rgb(29 34 51 / 11%)"
  rule-strong: "rgb(29 34 51 / 20%)"
  press: "#3346c4"
  press-down: "#2a3aa6"
  press-tint: "rgb(51 70 196 / 10%)"
  on-press: "#ffffff"
  sea: "#17705a"
  sea-tint: "rgb(23 112 90 / 11%)"
  on-sea: "#ffffff"
  amber: "#96600c"
  amber-tint: "rgb(150 96 12 / 11%)"
  plum: "#7c4a86"
  danger: "#c4312b"
  danger-down: "#a72822"
  focus: "rgb(51 70 196 / 42%)"
  selection: "rgb(51 70 196 / 20%)"
  disc-indigo: "#3346c4"
  disc-green: "#1c7d66"
  disc-slate: "#4d5a73"
  disc-plum: "#7c4a86"
typography:
  headline:
    fontFamily: "-apple-system, BlinkMacSystemFont, SF Pro Text, Helvetica Neue, sans-serif"
    fontSize: "20px"
    fontWeight: 700
    lineHeight: 1.38
    letterSpacing: "-0.015em"
  title:
    fontFamily: "-apple-system, BlinkMacSystemFont, SF Pro Text, Helvetica Neue, sans-serif"
    fontSize: "15px"
    fontWeight: 600
    lineHeight: 1.38
  wordmark:
    fontFamily: "-apple-system, BlinkMacSystemFont, SF Pro Text, Helvetica Neue, sans-serif"
    fontSize: "13px"
    fontWeight: 700
    lineHeight: 1.38
    letterSpacing: "-0.005em"
  body:
    fontFamily: "-apple-system, BlinkMacSystemFont, SF Pro Text, Helvetica Neue, sans-serif"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.38
    fontFeature: "tnum"
  body-strong:
    fontFamily: "-apple-system, BlinkMacSystemFont, SF Pro Text, Helvetica Neue, sans-serif"
    fontSize: "13px"
    fontWeight: 600
    lineHeight: 1.38
  meta:
    fontFamily: "-apple-system, BlinkMacSystemFont, SF Pro Text, Helvetica Neue, sans-serif"
    fontSize: "12px"
    fontWeight: 400
    lineHeight: 1.38
  label:
    fontFamily: "-apple-system, BlinkMacSystemFont, SF Pro Text, Helvetica Neue, sans-serif"
    fontSize: "11px"
    fontWeight: 600
    lineHeight: 1.38
    letterSpacing: "0.01em"
rounded:
  sm: "6px"
  md: "8px"
  lg: "10px"
  pill: "999px"
spacing:
  "1": "4px"
  "2": "8px"
  "3": "12px"
  "4": "16px"
  "5": "24px"
  "6": "32px"
components:
  button-primary:
    backgroundColor: "{colors.press}"
    textColor: "{colors.on-press}"
    typography: "{typography.body}"
    rounded: "{rounded.sm}"
    padding: "0 12px"
    height: "26px"
  button-primary-hover:
    backgroundColor: "{colors.press-down}"
  button-outline:
    backgroundColor: "{colors.sheet}"
    textColor: "{colors.ink}"
    rounded: "{rounded.sm}"
    padding: "0 12px"
    height: "26px"
  button-outline-active:
    backgroundColor: "{colors.well}"
  button-quiet:
    backgroundColor: "transparent"
    textColor: "{colors.ink-2}"
    rounded: "{rounded.sm}"
    padding: "0 12px"
    height: "26px"
  button-quiet-hover:
    backgroundColor: "{colors.press-tint}"
    textColor: "{colors.ink}"
  button-danger:
    backgroundColor: "{colors.danger}"
    textColor: "{colors.on-press}"
    rounded: "{rounded.sm}"
    padding: "0 8px"
    height: "22px"
  button-danger-hover:
    backgroundColor: "{colors.danger-down}"
  button-done:
    backgroundColor: "{colors.sea-tint}"
    textColor: "{colors.sea}"
    typography: "{typography.body-strong}"
    rounded: "{rounded.sm}"
    padding: "0 8px"
    height: "22px"
  button-done-hover:
    backgroundColor: "{colors.sea}"
    textColor: "{colors.on-sea}"
  input:
    backgroundColor: "{colors.sheet}"
    textColor: "{colors.ink}"
    rounded: "{rounded.sm}"
    padding: "0 8px"
    height: "26px"
  card:
    backgroundColor: "{colors.sheet}"
    rounded: "{rounded.lg}"
  list-row:
    padding: "8px 12px"
    height: "48px"
  tab:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
    rounded: "{rounded.sm}"
    padding: "0 8px"
    height: "28px"
  tab-selected:
    backgroundColor: "{colors.press}"
    textColor: "{colors.on-press}"
  tag-count:
    backgroundColor: "{colors.press}"
    textColor: "{colors.on-press}"
    typography: "{typography.label}"
    rounded: "{rounded.pill}"
    padding: "0 6px"
    height: "17px"
  tag-due:
    backgroundColor: "{colors.amber-tint}"
    textColor: "{colors.amber}"
    rounded: "{rounded.pill}"
  avatar:
    textColor: "#ffffff"
    typography: "{typography.body-strong}"
    size: "28px"
  selection-bar:
    backgroundColor: "{colors.press-tint}"
    rounded: "{rounded.lg}"
    padding: "8px 8px 8px 12px"
  banner:
    backgroundColor: "{colors.amber-tint}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    padding: "8px 12px"
  caught-up:
    backgroundColor: "{colors.sea-tint}"
    textColor: "{colors.sea}"
    rounded: "{rounded.lg}"
    padding: "12px"
---

# Design System: Friends

## Overview

**Creative North Star: "Letterpress Stationery"**

Friends is two inks printed on good card stock. Text is deep indigo ink rather than black, actions are one press-blue spot ink, "done" is sea green, and "missed" is amber. The ground is a cool fog stock (never cream), and content sits on white sheets laid on it. Colour is flat: no gradients, no glass, no glow.

It is a native Mac utility first. Type is the system face (SF Pro) at macOS control sizes, figures are tabular, and the layouts follow the Mac's own grammar: a menu bar dropdown that fills its window edge to edge, and a settings window with a source-list sidebar and inset grouped lists. Density is calm and fixed; one 48px row scale is shared by the dropdown and Preferences.

The signature is the impression: push buttons press into the stock when clicked (an inset shadow and half a pixel down) instead of lighting up.

**Key Characteristics:**
- Cool fog stock, white sheets, indigo ink; dark mode keeps the same roles on charcoal stock.
- One action ink (press blue); sea green only for done and caught up; amber for missed.
- Flat colour, hairline (0.5px) dividers, one soft lift under sheets.
- Buttons press in on click; nothing glows.
- System type only, 11 to 20px, tabular figures everywhere.

## Colors

Two working inks on a cool neutral stock, plus three small spot inks with fixed meanings. Every role has a dark-mode value in `src/styles.css`; components only use the role names.

### Primary
- **Press Blue** (press): the one action ink. Primary buttons, the selected sidebar tab, the count tag, checkbox and radio accents, the text caret, the focus ring and selection tint. **Press Blue Down** (press-down) is its hover and pressed state. **Press Tint** (press-tint) is the quiet hover wash and the selection bar ground.

### Secondary
- **Sea Green** (sea): done, and only done. The Done button (tint at rest, solid on hover and press) and the "You're all caught up" note. Done is sea green wherever it appears.

### Tertiary
- **Amber** (amber): missed. The meta line of an overdue person, the Due tag on the People list, and (as amber-tint) the warning banner ground.
- **Plum** (plum): a fourth ink, used only as an avatar tone.

### Neutral
- **Fog Stock** (stock): the ground of the dropdown and every Preferences pane.
- **Sheet White** (sheet): grouped lists, form cards, inputs, outline buttons.
- **Well** (well): the pressed state of outline and quiet buttons.
- **Sidebar Fog** (sidebar): the Preferences source list, one step darker than the stock.
- **Indigo Ink** (ink): all primary text.
- **Secondary Ink** (ink-2): meta lines, descriptions, quiet buttons, field labels.
- **Tertiary Ink** (ink-3): section labels, the sidebar wordmark, placeholders.
- **Rule / Rule Strong** (rule, rule-strong): 0.5px row dividers; the stronger rule for the dropdown footer, sidebar edge, and input and outline-button strokes.

### Avatar discs
Four fixed tones (disc-indigo, disc-green, disc-slate, disc-plum), picked from a hash of the name so a person keeps their colour. White initial on top. These stay the same in light and dark mode.

### Dark mode
The same roles, re-inked: charcoal stock `#181a21`, sheet `#22252e`, well `#13151b`, sidebar `#1d2028`; ink `#e9ebf2` / `#b4b8c6` / `#9196a6`; press `#4a5bd4`; sea `#5fcbae` with dark `#0d2a23` text on it; amber `#e3b25c`; plum `#c99ad3`; danger `#ff6b61`. Tints and rules switch to light-on-dark alpha. Full set in `src/styles.css` and the sidecar.

### Named Rules
**The Missed Is Amber Rule.** A missed day is information, not failure: it is printed in amber, never red. Red (danger) is for destructive actions (Delete) and error messages only.

**The One Action Ink Rule.** Press blue is the only colour that means "act here" or "selected". Do not introduce a second action colour.

**The Done Is Sea Rule.** Sea green marks completion and nothing else; every Done button uses it.

## Typography

**Body Font:** the system face (`-apple-system`, SF Pro Text, with Helvetica Neue and sans-serif fallback). No web fonts; the app's security policy forbids them.

**Character:** Sharp, compact Mac UI type with tabular figures, so dates and counts line up. Hierarchy comes from weight and ink, not size jumps.

### Hierarchy
- **Headline** (700, 20px, -0.015em): Preferences pane titles ("People", "Blocked dates", "Settings").
- **Title** (600, 15px): the bold first line of an empty state.
- **Wordmark** (700, 13px, -0.005em): "Friends" in the dropdown header. In the sidebar the same wordmark drops to 11px 600 in tertiary ink.
- **Body** (400, 13px, line-height 1.38): the base size for everything, including buttons (500) and tabs. Pane descriptions cap at 60ch.
- **Body Strong** (600, 13px): person names, the selection count, Done.
- **Meta** (400, 12px): the line under a name, field labels (500), hints, choice descriptions, small buttons, errors.
- **Label** (600, 11px, 0.01em, sentence case): section labels over lists ("Reach out now", "Coming up", "Reminders") and tags.

### Named Rules
**The Tabular Rule.** Figures are tabular across the whole app (`font-variant-numeric: tabular-nums` on the root).

## Layout

Fixed native windows, no responsive breakpoints. The dropdown is 360 x 520pt and fills its window edge to edge: a header row (wordmark left, today's date right in secondary ink), a scrolling body, and a footer of menu-style text buttons on a strong hairline. Preferences is a two-column grid: a 168px sidebar and a scrolling pane padded 24px (32px at the bottom).

Spacing runs on a 4px base: 4, 8, 12, 16, 24, 32. Rows are 48px tall with 8px by 12px padding and a 12px gap between avatar, text and actions. Sections in the dropdown sit 16px apart; setting groups 32px apart; a pane header sits 24px above its content.

**The One Row Rule.** Every list row in the dropdown and in Preferences uses the same 48px row, 28px disc and 12px gap.

## Elevation & Depth

Mostly flat, with two shadows that both read as paper. Sheets get a hairline **lift** so they sit just above the stock. Push buttons get the **press-in** impression while held. Everything else (the "Coming up" list, the sidebar, tints) is printed directly on the stock with no shadow.

### Shadow Vocabulary
- **Lift** (`0 0.5px 0 rgb(29 34 51 / 6%), 0 1px 2px rgb(29 34 51 / 8%)`; dark adds a 0.5px white hairline): cards, grouped lists, choice lists, primary, outline and danger buttons.
- **Press-in** (`inset 0 1px 2px rgb(10 14 30 / 28%)`, with `translateY(0.5px)`): any push button while pressed.
- **Field inset** (`inset 0 0 0 0.5px rule-strong, inset 0 1px 1px rgb(10 14 30 / 6%)`): text fields and selects at rest.

### Named Rules
**The Impression Rule.** Buttons respond to a press by sinking into the stock, never by glowing, scaling up, or casting a bigger shadow.

## Shapes

Gently rounded Mac corners: 6px on controls (buttons, inputs, tabs), 8px on the banner, 10px on sheets (cards, grouped lists, the selection bar, the caught-up note). Tags and scrollbar thumbs are full pills; avatars are full circles. Dividers are 0.5px hairlines. The dropdown itself has no radius: the native window supplies the square frame.

## Components

### Buttons
Mac push buttons that press into the stock.
- **Shape:** 6px corners, 26px tall with 12px side padding; the small size is 22px tall, 8px padding, 12px text.
- **Primary:** press blue with white text and the lift; press-down on hover and press.
- **Outline:** white sheet with a 0.5px strong-rule stroke and the lift; the well colour when pressed.
- **Quiet:** no ground, secondary ink; press-tint wash on hover, well when pressed. In the dropdown footer, quiet buttons use full ink at weight 400 to read like menu items.
- **Danger:** red with white text, used only for the confirming Delete.
- **Done:** sea-tint ground, sea text, weight 600; solid sea with on-sea text on hover and press.
- **States:** pressed buttons get the press-in shadow and move down 0.5px (120ms, `cubic-bezier(0.16, 1, 0.3, 1)`); disabled is 40% opacity. Focus is the shared 3px focus ring.

### Tags
- **Count tag:** an 11px pill in press blue with white text (the due count on the People tab). Inverts to white with blue text on the selected tab.
- **Due tag:** the same pill in amber-tint with amber text, after a person's name in the People list.

### Cards / Containers
- **Corner Style:** 10px.
- **Background:** sheet white on the stock.
- **Shadow Strategy:** the lift only.
- **Border:** none; rows inside are split by 0.5px rule hairlines.
- **Internal Padding:** 16px for form cards; rows carry their own padding.

### Inputs / Fields
- **Style:** 26px tall, sheet ground, 6px corners, field inset stroke; the label sits above in 12px secondary ink at weight 500. Placeholders in tertiary ink. Checkboxes and radios are native, tinted press blue.
- **Focus:** a 1px press-blue inner stroke plus a 3px focus halo.
- **Error:** a 12px danger-coloured line under the form or row.

### Navigation
The Preferences sidebar is a Mac source list on sidebar fog: 28px tabs with 6px corners in ink, a press-tint wash on hover, and solid press blue with white text (weight 500) when selected. The wordmark sits above it in small tertiary ink.

### Person Row
The core component: a 28px avatar disc, a name in body strong (ellipsised), a 12px meta line in secondary ink (amber when missed), and trailing small buttons. In the dropdown, "Reach out now" rows sit on a white sheet; "Coming up" rows are printed straight on the stock.

### Selection Bar
A press-tint strip with 10px corners above the People list: the count and the way out on the left, the destructive action alone at the far right. Delete is two steps: an outline "Delete selected…" first, then a red Delete beside Keep.

### Status Notes
- **Banner:** amber-tint ground with ink text and 8px corners, for a damaged data file.
- **Caught up:** sea-tint ground with sea text, centred, 600 weight.
- **Empty state:** centred secondary ink with a 15px title line, on the stock or in a card.

## Do's and Don'ts

### Do:
- **Do** use the role tokens in `src/styles.css` for every colour, size, radius and gap, so light and dark mode both follow.
- **Do** keep every list row at 48px with a 28px disc, in both windows.
- **Do** use press blue for every primary action and selected state, and nothing else.
- **Do** print Done in sea green wherever it appears.
- **Do** show pressed buttons with the press-in shadow and a 0.5px drop.
- **Do** use 0.5px hairlines between rows instead of borders around them.
- **Do** keep the 3px focus ring visible on every control, and let reduced motion turn transitions off.

### Don't:
- **Don't** use red for a missed day; missed is amber.
- **Don't** use red for anything but deleting and error messages.
- **Don't** add gradients, glass, blur or glows; colour is flat.
- **Don't** use cream or warm paper for the stock; it is cool fog.
- **Don't** load web fonts; the system face is the only face.
- **Don't** add a second action colour beside press blue.
