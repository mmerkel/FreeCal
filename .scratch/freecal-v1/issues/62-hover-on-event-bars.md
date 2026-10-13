# 62: Hover effect on multi-day and all-day bars

**What to build:** Bars for multi-day and all-day Events react to the pointer as timed single-day Events already do. Those show as dots in the month view, and the `classic` theme gives them a grey hover background; bars are filled with their Calendar colour and get nothing. Spec: "Event chips stay sparse" under Frontend.

**Blocked by:** none

**Status:** done

- [x] Bars change on hover in the month view and in the all-day area of day and week views, for writable and read-only Events
- [x] The effect works on any Calendar colour in light and dark mode (e.g. a slight darkening through `filter` or an overlay, not the grey `--hover`), and the text stays readable
- [x] The outline of the selected Event and, later, the dashed outline for unsent changes still show over it
- [x] Timed chips in the time grid get the same effect, if they have none yet, so every Event reacts alike
- [x] Checked by hand with `run-freecal` (vitest doesn't load component CSS)

## Comments

The bars and the time grid's chips get the class `event-block` through FullCalendar's `blockEventClass`; on hover a 15% black overlay goes over their background (`background-image`), so the text, the border colour and any outline stay as they are. The month view's dots keep the theme's grey hover.

Checked by hand with `run-freecal`: the fill darkened on hover for a multi-day bar in the month view (63,81,181 → 54,69,154), for a yellow Calendar (#f6bf26) in dark mode with its dark text still readable (→ 209,163,32), for all-day bars in the week and day views' all-day area and for a timed chip in the week view's time grid. With the Calendar set read-only in the Local Store, bars and timed chips darkened alike. The selected outline only shows while the details dialog is open, whose backdrop blocks hover, so the two never meet; the rule sets only `background-image`, so it can't hide the outline or, once it exists, the dashed outline for unsent changes (no ticket has added that yet).
