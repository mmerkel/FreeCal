# 42: Hand cursor over Events

**What to build:** The pointer is a hand over the whole of an Event in the grid, for every Event, since all of them open their details. Text in Event chips can't be selected, so the text cursor never shows. Spec: user story 141, "Event chips stay sparse" under Frontend.

**Blocked by:** none

**Status:** ready-for-agent

- [ ] `cursor: pointer` over the whole chip (dot, time, title, padding), for writable and read-only Events, in every view
- [ ] `user-select: none` on chips
- [ ] Empty slots keep the normal arrow
- [ ] Checked by hand with `run-freecal` (jsdom has no cursors)

Ticket 05 adds `grabbing` while dragging and resize cursors on the edges of movable Events.
