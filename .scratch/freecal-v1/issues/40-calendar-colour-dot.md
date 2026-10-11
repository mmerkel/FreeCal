# 40: Calendar colour dot on timed Events in the month view

**What to build:** In the month view, each timed Event shows a small dot in its Calendar's exact colour before its time, as in Google Calendar. The text keeps the normal colour. The `breezy` theme draws no dot there, so FreeCal adds it. Spec: user story 139, "Event chips stay sparse" under Frontend.

**Blocked by:** none

**Status:** ready-for-agent

- [ ] Timed Events in the month view show a round dot (about 0.5rem) in the Calendar colour, vertically centred before the time; all-day Events stay filled chips
- [ ] The dot follows a Calendar's colour change without a reload
- [ ] The dot is decorative (`aria-hidden`); the chip's accessible name is unchanged
- [ ] Narrow cells still show the title, cut off with "…"
- [ ] Component test: a timed Event's chip has a dot with its Calendar's colour, an all-day Event's has none
- [ ] Checked by hand in light and dark mode with `run-freecal`

Out of scope: day and week views, where timed Events are filled blocks (ticket 04).
