# 68: Cap the Events shown per day and show "+n more"

**What to build:** A day in the month view with many Events makes its row taller, and the grid then scrolls. Instead, each day shows only as many Events as fit and then a "+n more" link that opens a popover with all of that day's Events. This is always on, with no setting. The week and day views cap their all-day area the same way, at 6 rows.

The popover is FullCalendar's own (the "more" popover of 7.1), restyled. It already lets you drag Events out of it, updates when the day's Events change, closes on Escape or a click outside and returns focus to the link. A popover of our own via `moreLinkClick` was rejected because it would have to rebuild all of that, drag-out most of all.

**Blocked by:** none

**Status:** ready-for-agent

- [ ] Month view: `dayMaxEvents: true`, so rows keep equal heights, fill the window and no longer grow; the month grid never scrolls. A day where not even one Event fits shows only "+n more"
- [ ] Week and day view: the all-day area shows at most 6 rows, with "+n more" taking the 6th (`dayMaxEventRows: 6` set for the time-grid views only)
- [ ] The link text comes from `src/i18n` ("+{n} more" in `en.ts`), via FullCalendar's `moreLinkContent`
- [ ] The popover's header date is a button of ours (through `dayHeaderContent` when `inPopover` is true; FullCalendar never gives the popover a nav link) that switches to the day view for that date
- [ ] Clicking an Event in the popover closes the popover and opens the Event editor, as a click on a grid chip does
- [ ] Dragging an Event out of the popover onto another day moves it with the grid's rules: read-only Events don't move, and unsent changes get the dashed outline as anywhere else. Check that `gridDrag.ts` handles a drag that starts in the popover
- [ ] Chips in the popover look exactly like grid chips (dot, time, title, dashed outline, Conflict badge)
- [ ] The popover closes on Escape, a click outside, or a change of view or date. Escape closes only the popover: no other Escape handler (`Menu`, `Dialog`, the Sidebar's rename) reacts to the same press
- [ ] Keyboard follows FullCalendar: "+n more" is a focusable `role="button"` that opens on Enter or Space; focus starts on the ✕; Tab past either end closes the popover; focus returns to the link on close. No arrow keys and no Tab wrapping (a later ticket can add them if this feels wrong in use)
- [ ] Look: `popoverClass` and CSS match the menus and the date picker (surface, border, shadow, radius, from `app.css` tokens) in light and dark; FullCalendar's 10px gap from the window's edges is kept. A day with more Events than fit in the window makes the popover scroll inside itself instead of growing past the window (add a max height if FullCalendar doesn't)
- [ ] `spec.md` says that days and the all-day area cap their Events with "+n more" and describes the popover, under the month view and the Event chips
- [ ] Tests in `CalendarGrid.test.ts`: a day with more Events than fit shows "+n more" with the right count; the link opens a dialog listing all the day's Events; Escape closes it and focuses the link; the header date switches to the day view; clicking an Event in it opens the editor
- [ ] Checked by hand with `run-freecal`, results noted below: a month with one day of 15 Events keeps equal rows and doesn't scroll; resizing the window changes how many show; the popover opens, looks like a menu in both themes, scrolls when tall, and an Event dragged out of it lands on the new day; a week with 10 all-day Events shows 5 and "+5 more"

## Comments

- 2026-10-17: From `TODO.md` ("Many items on a day in month view currently increase row width. Add options to setting to rather show it as "<n> more events" or so."), settled in a grilling session. The setting was dropped: keeping rows that grow defeats the month view's overview, and a setting costs a Rust type, a migration and strings. "As many as fit" was chosen over a fixed count. A click on "+n more" opens a popover rather than the day view, which stays one click away through the header date. The all-day area cap is 6 rows, not 3, since the time grid has room. FullCalendar's popover keyboard model (focus on ✕, Tab leaves) was accepted over a custom layer, which would rely on FullCalendar's DOM.
