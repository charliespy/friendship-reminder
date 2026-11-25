---
name: Friends
description: A macOS menu bar reminder, and its iPhone app, that look like they came with the Mac and the iPhone.
colors:
  window: "#ececec"
  sidebar: "#e3e3e3"
  group: "#ffffff"
  control: "#ffffff"
  control-down: "#e3e3e3"
  hover: "rgb(0 0 0 / 5%)"
  label: "#1d1d1f"
  label-2: "#636366"
  label-3: "#6e6e73"
  separator: "rgb(0 0 0 / 10%)"
  separator-strong: "rgb(0 0 0 / 18%)"
  accent: "#007aff"
  on-accent: "#ffffff"
  accent-tint: "rgb(0 122 255 / 12%)"
  focus: "rgb(0 122 255 / 45%)"
  selection: "rgb(0 122 255 / 22%)"
  orange: "#b25000"
  orange-tint: "rgb(255 149 0 / 16%)"
  danger: "#d70015"
  danger-fill: "#e0352b"
  monogram-top: "#8e8e93"
  monogram-bottom: "#6f6f74"
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
  button:
    fontFamily: "-apple-system, BlinkMacSystemFont, SF Pro Text, Helvetica Neue, sans-serif"
    fontSize: "13px"
    fontWeight: 500
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
    backgroundColor: "{colors.accent}"
    textColor: "{colors.on-accent}"
    typography: "{typography.button}"
    rounded: "{rounded.sm}"
    padding: "0 12px"
    height: "26px"
  button-push:
    backgroundColor: "{colors.control}"
    textColor: "{colors.label}"
    typography: "{typography.button}"
    rounded: "{rounded.sm}"
    padding: "0 12px"
    height: "26px"
  button-push-active:
    backgroundColor: "{colors.control-down}"
  button-done:
    backgroundColor: "{colors.control}"
    textColor: "{colors.label}"
    rounded: "{rounded.sm}"
    padding: "0 8px"
    height: "22px"
  button-done-active:
    backgroundColor: "{colors.control-down}"
  button-quiet:
    backgroundColor: "transparent"
    textColor: "{colors.label-2}"
    typography: "{typography.button}"
    rounded: "{rounded.sm}"
    padding: "0 12px"
    height: "26px"
  button-quiet-hover:
    backgroundColor: "{colors.hover}"
    textColor: "{colors.label}"
  button-quiet-active:
    backgroundColor: "{colors.control-down}"
  button-danger:
    backgroundColor: "{colors.danger-fill}"
    textColor: "#ffffff"
    typography: "{typography.button}"
    rounded: "{rounded.sm}"
    padding: "0 12px"
    height: "26px"
  button-sm:
    padding: "0 8px"
    height: "22px"
  group:
    backgroundColor: "{colors.group}"
    rounded: "{rounded.lg}"
  input:
    backgroundColor: "{colors.group}"
    textColor: "{colors.label}"
    rounded: "{rounded.sm}"
    padding: "0 8px"
    height: "26px"
  tag-count:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.on-accent}"
    typography: "{typography.label}"
    rounded: "{rounded.pill}"
    padding: "0 6px"
    height: "17px"
  tag-due:
    backgroundColor: "{colors.orange-tint}"
    textColor: "{colors.orange}"
    typography: "{typography.label}"
    rounded: "{rounded.pill}"
    padding: "0 6px"
    height: "17px"
  sidebar-row:
    backgroundColor: "transparent"
    textColor: "{colors.label}"
    typography: "{typography.body}"
    rounded: "{rounded.sm}"
    padding: "0 8px"
    height: "28px"
  sidebar-row-selected:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.on-accent}"
  person-row:
    padding: "8px 12px"
    height: "48px"
  avatar:
    textColor: "#ffffff"
    typography: "{typography.button}"
    rounded: "{rounded.pill}"
    size: "28px"
  selection-bar:
    backgroundColor: "{colors.accent-tint}"
    rounded: "{rounded.lg}"
    padding: "8px 8px 8px 12px"
  banner:
    backgroundColor: "{colors.orange-tint}"
    textColor: "{colors.label}"
    typography: "{typography.meta}"
    rounded: "{rounded.md}"
    padding: "8px 12px"
  caught-up:
    backgroundColor: "{colors.group}"
    textColor: "{colors.label-2}"
    rounded: "{rounded.lg}"
    padding: "12px"
---

# Design System: Friends

## Overview

**Creative North Star: "Came With The Mac"**

Friends looks like an app Apple shipped. It borrows System Settings: window grey, white inset groups, the system's label greys, and standard push buttons. Nothing is branded. A user should not be able to tell where macOS ends and Friends begins.

There is one colour: the Mac's own accent, the one the user picked in System Settings > Appearance. It is blue unless they chose another. It marks the main action and what is selected. Orange marks a missed day. Red is kept for deleting and errors. Everything else is grey.

The app is small and dense, at Mac control sizes. Type is the system face, 11 to 20px, with tabular figures. One 48px row and one 28px disc are shared by the dropdown and Preferences. Light and dark mode follow the Mac, with the same roles in both.

**Key Characteristics:**
- Window grey, white groups with a hairline edge, label greys; dark mode keeps the same roles.
- The Mac's accent is the only colour; orange for missed, red for delete and errors.
- Push buttons darken when pressed, like the Mac's.
- Contacts-style grey monogram discs, the same for everyone.
- System type only, 11 to 20px, tabular figures.

## Colors

Neutral system greys, with the user's own accent as the one colour.

### Primary
- **Mac Accent** (`-apple-system-control-accent`, fallback system blue): primary buttons, the selected sidebar row, the count tag, checkbox and radio fills, the caret, the focus ring and text selection. In WebKit it follows the Mac's accent setting, light and dark. Elsewhere it falls back to system blue (#007aff, dark #0a84ff).
- **Accent Tint, Focus, Selection** (accent at 12%, 45% and 22%): the selection bar's fill, the 3px focus ring, and selected text. When `color-mix` is supported they are mixed from the live accent, so they follow the user's colour too.
- **On Accent** (white): text on accent fills. On the selected sidebar row the count tag flips to white with accent text.

### Secondary
- **Missed Orange** (light #b25000 / dark #ffa337): the meta line of an overdue person, and the text of the Due tag. Its tint (orange at 16-18%) fills the Due tag and the alert banner.

### Tertiary
- **Delete Red** (text light #d70015 / dark #ff6961; fill light #e0352b / dark #d93a30): the delete button fill and error messages. Nothing else.

### Neutral
- **Window Grey** (dark #1e1e1e): the ground of the dropdown and the Preferences pane.
- **Sidebar Grey** (dark #282828): the Preferences source list.
- **Group White** (dark #2a2a2a): inset groups, inputs and the caught-up note.
- **Control White / Control Down** (dark #4a4a4c / #5c5c5f): push buttons at rest and pressed.
- **Hover** (black 5% / white 7%): hover fill for quiet buttons.
- **Label, Label 2, Label 3** (dark #e8e8ea / #a1a1a6 / #98989d): primary text; secondary text (meta lines, section labels, hints, quiet buttons); placeholders.
- **Separator / Separator Strong** (black 10% / 18%; white in dark mode): hairlines between rows; the footer rule, sidebar edge, input stroke and scrollbar thumb.
- **Monogram Grey** (gradient #8e8e93 to #6f6f74; dark #78787d to #5a5a5e): the avatar disc.

### Named Rules
**The One Colour Rule.** The accent is the only colour. It means "act here" or "selected". Do not add a brand colour or a second action colour.

**The Missed Is Orange Rule.** A missed day is shown in orange, never red. It is information, not failure.

**The Red Means Delete Rule.** Red is only for deleting and for error messages.

**The Follow The Mac Rule.** Colour comes from the Mac. Use the accent token, never a fixed blue, so the user's own accent shows through.

## Typography

**Display Font:** none
**Body Font:** the system face (-apple-system, BlinkMacSystemFont, SF Pro Text, with Helvetica Neue)

**Character:** One face at Mac control sizes. Hierarchy comes from weight and grey, not from size jumps.

### Hierarchy
- **Headline** (700, 20px, -0.015em): Preferences pane titles.
- **Title** (600, 15px): the bold first line of an empty state.
- **Wordmark** (700, 13px, -0.005em): "Friends" in the dropdown header. In the sidebar it is 11px, 600, in Label 2.
- **Body** (400, 13px, 1.38): base text, sidebar rows and footer buttons. Pane intros stop at 60ch.
- **Body Strong** (600, 13px): person names, the selection count.
- **Button** (500, 13px): push buttons, choice titles, the selected sidebar row.
- **Meta** (400, 12px): meta lines, field labels (500), hints, errors, small buttons, the dropdown date.
- **Label** (600, 11px, 0.01em, sentence case): section labels above groups, and tags.

### Named Rules
**The Tabular Rule.** Figures are tabular everywhere (`font-variant-numeric: tabular-nums` on the root).

**The System Face Rule.** The system face is the only face. Load no web fonts.

## Layout

A 4px base: 4, 8, 12, 16, 24 and 32px. The dropdown fills its window edge to edge, so the native window's corners and shadow are the only frame. Its header is 12px 16px 8px, its body 4px 12px 12px, with 16px between sections, and a footer of quiet text buttons above a strong hairline. "Reach out now" sits in a white group. "Coming up" is a plain list on the window grey.

Preferences is a two-column grid: a 168px source-list sidebar and a pane padded 24px (32px at the bottom). Pane headers sit 24px above the content. Groups are 24px apart; setting groups are 32px apart. Form cards are padded 16px.

Every list row is at least 48px tall, padded 8px 12px, with a 12px gap and a 28px disc. Rows are split by 0.5px hairlines. Sidebar rows are 28px tall with 1px between them.

### Named Rules
**The One Row Rule.** Every list row in the dropdown and in Preferences uses the same 48px row, 28px disc and 12px gap.

## Elevation & Depth

Flat, like System Settings. Groups are not lifted. They are edged by a 0.5px hairline ring. Push buttons carry the Mac's small edge and drop. Nothing glows or floats.

### Shadow Vocabulary
- **Group Edge** (`box-shadow: 0 0 0 0.5px rgb(0 0 0 / 7%)`; dark `rgb(255 255 255 / 6%)`): the hairline around every inset group.
- **Raise** (`box-shadow: 0 0 0 0.5px rgb(0 0 0 / 12%), 0 0.5px 1px rgb(0 0 0 / 14%)`; dark `0 0 0 0.5px rgb(255 255 255 / 8%), 0 0.5px 1px rgb(0 0 0 / 40%)`): push buttons only.
- **Focus** (`outline: 3px solid` focus, offset 1px; inputs use a 1px accent ring inside plus a 3px focus ring outside): every focusable control.

### Named Rules
**The Darken On Press Rule.** Pressed buttons darken, as the Mac's do. Grey buttons switch to Control Down. Accent and red buttons use `filter: brightness(0.86)`. They do not sink, move, scale or glow.

## Shapes

Gently rounded Mac corners: 6px for buttons, inputs and sidebar rows; 8px for the banner; 10px for groups and the selection bar. Discs and tags are fully round (999px). Lines are 0.5px hairlines. Groups have no borders; their edge is the hairline ring.

## Components

### Buttons
Standard macOS push buttons.
- **Shape:** 6px corners, 26px tall, 12px sides; small buttons are 22px tall, 8px sides, 12px text.
- **Primary:** accent fill, white text, with Raise. Pressed: brightness 0.86.
- **Push (outline):** Control White, Label text, with Raise. Pressed: Control Down.
- **Done:** a plain small push button. No colour of its own; it is the list's only action.
- **Quiet:** no fill, Label 2 text. Hover: Hover fill and Label text. Pressed: Control Down. Used in the footer (Label, 400) and for cancel and edit.
- **Danger:** red fill, white text, with Raise. Pressed: brightness 0.86. Only for deleting.
- **Save** (Days to aim for): the day setting reschedules everyone, so it's a draft until Save. Under the group: a Label 2 status on the left ("Not saved yet." / "Saved."), a quiet Discard while there are changes, and a primary Save, off until something changed. On iPhone the buttons share the row and the status sits above them.
- **Day buttons** (Custom day): seven 26px circles, M to S, Control White with Raise; a picked day fills with the accent and white text.

### Done's question
Done first asks "When were you in touch with …?" in a sheet over the window: the Window grey box (10px corners, a soft drop shadow, a dim backdrop), the answers as a grouped list (title, then the date in Label 2), and a quiet Cancel. The answers are the day suggested (once it has come), Today, and Another day…, which opens a date field and a primary Done.
- **Sign in with Apple:** black on light, white on dark, per Apple's rules.
- **Disabled:** 40% opacity. State changes are instant, as on the Mac: no fades.

### Tags
- **Count:** pill, 17px line, 11px 600, accent fill with white text. Flips to white with accent text on the selected sidebar row.
- **Due:** the same pill in Orange Tint with orange text, beside a person's name.

### Cards / Containers
- **Corner Style:** 10px.
- **Background:** Group White on the window grey.
- **Shadow Strategy:** Group Edge only (see Elevation & Depth).
- **Border:** none; rows inside are split by 0.5px separators.
- **Internal Padding:** rows 8px 12px; form cards 16px; choices 12px 16px.

### Inputs / Fields
- **Style:** Group White, 26px tall, 6px corners, a 0.5px inset Separator Strong stroke. Placeholders in Label 3. Field labels sit 4px above in 12px 500 Label 2.
- **Focus:** a 1px accent ring inside and a 3px focus ring outside.
- **Checkboxes and radios:** native and untinted, so the Mac draws them in its own accent.
- **Error:** 12px red text 4px below.

### Navigation
The Preferences source list: Sidebar Grey, a strong hairline on its right edge. Rows are 28px, 13px Label, with no hover fill (like System Settings). Selected: accent fill, white text, 500.

### Person Row
A 48px row: grey disc, name in Body Strong with an ellipsis, a 12px meta line in Label 2 (orange when overdue), and a Done button.

### Avatar
A Contacts-style monogram: a 28px round disc in the Monogram gradient, the person's initial in white, 13px 500. The same grey for everyone.

### Selection Bar
An Accent Tint strip with 10px corners above the People list. The count and the way out sit on the left; delete sits alone at the far right.

### Status Notes
- **Banner:** Orange Tint with Label text, 12px, 8px corners, for alerts.
- **Caught up:** a plain group in Label 2, 500, centred, padded 12px.
- **Empty state:** centred Label 2 text with a 15px 600 title in Label.

### iPhone
The same tokens and components, at iOS sizes (`:root.phone-app` in `src/styles.css`), so it looks like it came with the iPhone:
- **Colours:** iOS's grouped background (`#f2f2f7`, black in dark mode) with white groups (`#1c1c1e` dark), no group edge, and buttons as flat grey fills (`rgb(118 118 128 / 12%)`) instead of raised push buttons. iOS has no user accent, so the accent is the system blue.
- **Type:** 17px body, 15px secondary, 13px section labels, a 34px bold large title per tab.
- **Rows:** 56px with a 36px disc, inset hairlines that start at the text. People rows leave the disc out to give names room.
- **Touch:** 44px buttons and fields, 32px pill buttons in rows. Nothing typed into is under 16px, or iOS zooms in. Fields show the accent edge when focused, with no Mac glow.
- **iOS controls:** on/off settings are switches (white knob, accent track when on); a pick-one list shows an accent tick on the right of the chosen row; selecting people uses round circles that fill with the accent. The People header's Add is plain accent text at the top right, as in iOS.
- **People rows:** the name with Done and Edit beside it, and the detail line underneath at full width.
- **Done's question:** an iOS action sheet at the bottom: the question in small grey type on top of one rounded group of answers in accent text, and a separate bold Cancel under it.
- **Day buttons:** 38px circles spread across the row under Custom day's text.
- **Tab bar:** four tabs at the bottom (Today, People, Blocked, Settings), 26px line symbols, the selected one in the accent, over a translucent blurred bar. Today carries an accent badge with the number due.

## Do's and Don'ts

### Do:
- **Do** use the role tokens in `src/styles.css` for every colour, size, radius and gap, so light and dark mode both follow.
- **Do** use the accent token for every primary action and selected state, so the user's Mac accent shows.
- **Do** keep every list row at 48px with a 28px disc, in both Mac windows (56px and 36px on iPhone).
- **Do** show pressed buttons by darkening them: Control Down for grey buttons, brightness 0.86 for accent and red.
- **Do** edge groups with the 0.5px Group Edge, and split rows with 0.5px hairlines.
- **Do** keep the 3px focus ring visible on every control, and keep state changes instant.

### Don't:
- **Don't** add a brand colour or a second action colour beside the accent.
- **Don't** hard-code a blue; use the accent token.
- **Don't** use red for a missed day; missed is orange.
- **Don't** use red for anything but deleting and error messages.
- **Don't** give Done, avatars or the caught-up note a colour of their own.
- **Don't** make buttons sink, move or glow when pressed.
- **Don't** load web fonts; the system face is the only face.
