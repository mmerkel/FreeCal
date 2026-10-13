# 62: Hover effect on multi-day and all-day bars

**What to build:** Bars for multi-day and all-day Events react to the pointer as timed single-day Events already do. Those show as dots in the month view, and the `classic` theme gives them a grey hover background; bars are filled with their Calendar colour and get nothing. Spec: "Event chips stay sparse" under Frontend.

**Blocked by:** none

**Status:** ready-for-agent

- [ ] Bars change on hover in the month view and in the all-day area of day and week views, for writable and read-only Events
- [ ] The effect works on any Calendar colour in light and dark mode (e.g. a slight darkening through `filter` or an overlay, not the grey `--hover`), and the text stays readable
- [ ] The outline of the selected Event and, later, the dashed outline for unsent changes still show over it
- [ ] Timed chips in the time grid get the same effect, if they have none yet, so every Event reacts alike
- [ ] Checked by hand with `run-freecal` (vitest doesn't load component CSS)
