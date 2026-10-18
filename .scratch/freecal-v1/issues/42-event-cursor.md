# 42: Hand cursor over Events

**What to build:** The pointer is a hand over the whole of an Event in the grid, for every Event, since all of them open their details. Text in Event chips can't be selected, so the text cursor never shows. Spec: US-141, "Event chips stay sparse" under Frontend.

**Blocked by:** none

**Status:** done

**Effort:** S

- [x] `cursor: pointer` over the whole chip (dot, time, title, padding), for writable and read-only Events, in every view
- [x] `user-select: none` on chips
- [x] Empty slots keep the normal arrow
- [x] Checked by hand with `run-freecal` (jsdom has no cursors)

Ticket 05 adds `grabbing` while dragging and resize cursors on the edges of movable Events.

## Comments

- 2026-10-13: Chips get the class `event-chip` through FullCalendar's `eventClass`, styled in `CalendarGrid.svelte`. There is no automated test, because vitest doesn't load component CSS, so `getComputedStyle` can't see the rule. Checked by hand with `run-freecal`, reading the X cursor image through XFixes (`xfixes_get_cursor_image` in python-xlib, after `xfixes_query_version`). The hand matches a toolbar button's over the dot, time, title and padding of timed chips in month, week and day views, over all-day chips in month view, and over a chip in a Calendar set to `read_only = 1`. Empty slots keep the arrow. Before the change, time and title showed the text cursor. A drag across a chip's text highlighted it before the change and doesn't after it. The built CSS includes `-webkit-user-select`.
