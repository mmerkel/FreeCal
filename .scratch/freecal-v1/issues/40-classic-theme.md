# 40: The `classic` FullCalendar theme, with FreeCal's colours

**What to build:** The grid switches from FullCalendar's `breezy` theme to `classic`. In the month view, `classic` draws each timed Event as a dot in its Calendar's exact colour, then the time, then the title, as in Google Calendar; `breezy` draws no dot and puts the time at the end. FreeCal adapts `classic` to its own look: readable text on every Calendar colour, the window's light and dark colours instead of the theme's, and "…" on titles that don't fit. Spec: user story 139, "Visual style" and "Event chips stay sparse" under Frontend.

**Blocked by:** none

**Status:** done

**Effort:** M

- [x] The grid uses `classic` (theme and `palette.css`); `breezy` and its `indigo` palette are no longer imported in `CalendarGrid.svelte`
- [x] Timed Events in the month view show the dot in the Calendar colour, then the time, then the title; all-day Events stay filled bars in the Calendar colour
- [x] Text on a filled Event is near-black or white, whichever contrasts more with the Calendar's colour (WCAG contrast ratio), worked out from the colour itself, so that colours from CalDAV or Google also work. Yellow, mustard and lime get dark text, for example
- [x] Dot and fill follow a Calendar's colour change without a reload
- [x] `classic`'s palette variables are pointed at FreeCal's tokens in `app.css`, for light and dark: background, borders and foregrounds match the rest of the window (no blue-tinted black next to the sidebar in dark mode)
- [x] `classic`'s own accent colours are replaced by neutral ones from the tokens: the primary blue (`#3788d8`) and the cyan tint when selecting a range of days. Calendar colours stay the only strong colours
- [x] Titles that don't fit end in "…" instead of being cut off mid-letter, for timed and all-day Events
- [x] The dot is decorative: the chip's accessible name is unchanged
- [x] Component test: a timed Event's chip has a dot in its Calendar's colour and an all-day Event's has none; a yellow Calendar's all-day Event gets dark text, a blue one's white
- [x] Checked by hand with `run-freecal` in light and dark mode, with a light (yellow) and a dark Calendar colour and a long title

Out of scope:

- Today's marker: `classic` only tints today's cell faintly yellow, which is hard to see in dark mode. Left as it is for now; a later ticket can draw a circle around the number as `breezy` did.
- Day and week views (ticket 04). `classic`'s look there hasn't been checked yet; ticket 04 checks it by hand.
- Bold titles, day numbers at the top right and the dot's size in px: `classic`'s defaults, kept.

## Comments

Prototype comparing breezy, classic, forma, monarch and pulse in the real month view, light and dark: branch `prototype/ticket-40-themes` (commit `e9968b6`). Verdict: `classic`. It has the dot followed by the time, as in Google Calendar, and needs only palette overrides to fit. pulse draws no dot. forma and monarch bring more of their own look (all-day style, today marker). Keeping `breezy` would have meant adding the dot and moving the time by hand.
