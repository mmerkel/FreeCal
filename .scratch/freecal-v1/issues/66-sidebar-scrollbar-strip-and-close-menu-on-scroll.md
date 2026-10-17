# 66: The sidebar's scrollbar gets its own strip, and scrolling closes an open menu

**What to build:** Two changes around the sidebar's scrollbar.

The sidebar's scrollbar is a GTK overlay bar today, drawn over the rows' right end: near x≈170 it takes clicks meant for a row's ⋮ button. It becomes a thin bar in its own strip, styled with `::-webkit-scrollbar` on `.sidebar`. It shows only while the sidebar overflows, and then all the time. In WebKitGTK `scrollbar-width` and `scrollbar-gutter` don't do this; `::-webkit-scrollbar` does.

Ticket 48 blocks the wheel and touch scrolling outside an open menu, but dragging a scrollbar still scrolls, and that can flip the menu or scroll its anchor out of view. A `preventDefault` on the press doesn't stop a scrollbar drag in WebKitGTK. So instead of blocking, scrolling outside an open menu closes it, and the scroll goes ahead. This covers the wheel, touch, scrollbar drags and keyboard scrolling, in the sidebar and the time grid alike.

**Blocked by:** none

**Status:** ready-for-agent

**Effort:** M

- [ ] `.sidebar` has a `::-webkit-scrollbar` about 6–8px wide, a rounded thumb in colours from `app.css` tokens (add tokens if none fit), and no visible track; it reserves its own strip, shows only while the sidebar overflows, and the rows narrow to make room
- [ ] With the strip, a click on a row's ⋮ opens its menu, and the bar no longer covers an open menu
- [ ] `Menu` no longer cancels `wheel` and `touchmove` outside it; instead any `scroll` event outside the menu, while it is open, calls `onclose` (one capture-phase listener on `window`, next to the events `Menu` already swallows). The scroll is not undone, and focus returns as on any close
- [ ] A wheel over something that can't scroll (the month grid, a sidebar that doesn't overflow) leaves the menu open, since no `scroll` event fires
- [ ] A scroll the app makes itself while a menu is open (e.g. the sidebar shortening while scrolled to the end) also closes it; that is accepted
- [ ] `Menu`'s header comment says what scrolling outside it does now
- [ ] Tests: ticket 48's wheel and touch tests in `Sidebar.test.ts` are replaced by: a `scroll` outside the open menu closes it; a `scroll` inside it does not
- [ ] `.claude/skills/run-freecal/SKILL.md` is updated: the ⋮ coordinates with the strip, the overlay-bar gotcha removed, the line on scrolling under an open menu rewritten, and a wheel helper documented if one is added to the driver
- [ ] Checked by hand with `run-freecal` and 30 or more Calendars, results noted below: the strip shows only when the sidebar overflows; ⋮ opens its menu wherever its row is; a wheel over the sidebar or the week grid closes an open menu and scrolls; dragging the sidebar's scrollbar closes it and scrolls; a wheel over the month grid leaves it open

## Comments

- 2026-10-17: From `TODO.md` ("side bar scroll bar on left?" and "side bar can still be scrolled when menu open: by dragging scroll bar, and that can make the menu flip"), settled in a grilling session. Kept on the right in its own strip, since a left-hand bar is unusual on Linux. Considered and rejected for the menu: undoing the scroll by restoring `scrollTop` (works, but the thumb twitches), `overflow: hidden` on scrollers (the bar vanishes) and `pointer-events: none` on scrollers (hover dies). Closing on gestures only (`wheel`, `touchmove`, a press hit-tested onto a scrollbar) was rejected for the WebKit-specific hit-testing; closing on any `scroll` event was chosen, and the wheel closes too, so all scrolling behaves alike. Checked in WebKitGTK 2.52.6: the overlay bar did not paint over the menu in GPU rendering; the run-freecal note that it does came from CPU rendering.
