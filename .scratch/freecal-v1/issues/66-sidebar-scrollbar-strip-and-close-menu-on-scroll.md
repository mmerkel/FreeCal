# 66: The sidebar's scrollbar gets its own strip, and scrolling closes an open menu

**What to build:** Two changes around the sidebar's scrollbar.

The sidebar's scrollbar is a GTK overlay bar today, drawn over the rows' right end: near x≈170 it takes clicks meant for a row's ⋮ button. It becomes a thin bar in its own strip, styled with `::-webkit-scrollbar` on `.sidebar`. It shows only while the sidebar overflows, and then all the time. In WebKitGTK `scrollbar-width` and `scrollbar-gutter` don't do this; `::-webkit-scrollbar` does.

Ticket 48 blocks the wheel and touch scrolling outside an open menu, but dragging a scrollbar still scrolls, and that can flip the menu or scroll its anchor out of view. A `preventDefault` on the press doesn't stop a scrollbar drag in WebKitGTK. So instead of blocking, scrolling outside an open menu closes it, and the scroll goes ahead. This covers the wheel, touch, scrollbar drags and keyboard scrolling, in the sidebar and the time grid alike.

**Blocked by:** none

**Status:** done

**Effort:** M

- [x] `.sidebar` has a `::-webkit-scrollbar` about 6–8px wide, a rounded thumb in colours from `app.css` tokens (add tokens if none fit), and no visible track; it reserves its own strip, shows only while the sidebar overflows, and the rows narrow to make room
- [x] With the strip, a click on a row's ⋮ opens its menu, and the bar no longer covers an open menu
- [x] `Menu` no longer cancels `wheel` and `touchmove` outside it; instead any `scroll` event outside the menu, while it is open, calls `onclose` (one capture-phase listener on `window`, next to the events `Menu` already swallows). The scroll is not undone, and focus returns as on any close
- [x] A wheel over something that can't scroll (the month grid, a sidebar that doesn't overflow) leaves the menu open, since no `scroll` event fires
- [x] A scroll the app makes itself while a menu is open (e.g. the sidebar shortening while scrolled to the end) also closes it; that is accepted
- [x] `Menu`'s header comment says what scrolling outside it does now
- [x] Tests: ticket 48's wheel and touch tests in `Sidebar.test.ts` are replaced by: a `scroll` outside the open menu closes it; a `scroll` inside it does not
- [x] `.claude/skills/run-freecal/SKILL.md` is updated: the ⋮ coordinates with the strip, the overlay-bar gotcha removed, the line on scrolling under an open menu rewritten, and a wheel helper documented if one is added to the driver
- [x] The sidebar's right padding is `--space-1` (2px), with or without the strip, so ⋮ sits close to the strip or the splitter without unused space; checked by hand
- [x] Checked by hand with `run-freecal` and 30 or more Calendars, results noted below: the strip shows only when the sidebar overflows; ⋮ opens its menu wherever its row is; a wheel over the sidebar or the week grid closes an open menu and scrolls; dragging the sidebar's scrollbar closes it and scrolls; a wheel over the month grid leaves it open

## Comments

- 2026-10-17: From `TODO.md` ("side bar scroll bar on left?" and "side bar can still be scrolled when menu open: by dragging scroll bar, and that can make the menu flip"), settled in a grilling session. Kept on the right in its own strip, since a left-hand bar is unusual on Linux. Considered and rejected for the menu: undoing the scroll by restoring `scrollTop` (works, but the thumb twitches), `overflow: hidden` on scrollers (the bar vanishes) and `pointer-events: none` on scrollers (hover dies). Closing on gestures only (`wheel`, `touchmove`, a press hit-tested onto a scrollbar) was rejected for the WebKit-specific hit-testing; closing on any `scroll` event was chosen, and the wheel closes too, so all scrolling behaves alike. Checked in WebKitGTK 2.52.6: the overlay bar did not paint over the menu in GPU rendering; the run-freecal note that it does came from CPU rendering.
- 2026-10-17: Done. Checked by hand with `run-freecal --subpixel` and 32 Calendars: with 2 Calendars there is no strip; with 32 the strip is at x≈184–191, and ⋮ (now x≈162) opens its menu on the top row and on a low row (y≈756, flipped above), and the menu lies over the strip. A wheel over the sidebar or the week grid closes an open menu and scrolls; dragging the sidebar's scrollbar closes it and scrolls; a wheel over the month grid leaves it open; arrow keys inside the menu leave it open. The low row first failed: focusing the first item before the menu was placed made WebKit scroll the sidebar, which now closes the menu. The first item is now focused with `preventScroll`; jsdom doesn't scroll on focus, so only the hand check covers this. `run-freecal` gained a `wheel` command. Keyboard scrolling outside an open menu can't happen while the menu holds focus; a scroll the app makes itself closes it, as accepted.
- 2026-10-17: Reopened. With the strip, ⋮ sits slightly too far left: the sidebar's 8px right padding plus the strip leave unused space.
- 2026-10-17: Done again. A `ResizeObserver` on the sidebar marks it `.scrolls` while it overflows (the strip appearing shrinks its content box, so the observer fires; CSS can't tell overflow in WebKitGTK), and its right padding is then `--space-2`. Checked by hand with `--subpixel`: with the strip, ⋮ is at x≈166 and "+" at x≈164 (was 162 and 160); deleting Calendars through the menu until the sidebar fits removes the strip and restores the 8px padding ("+" at x≈168) live, and adding Calendars brings both back.
- 2026-10-17: Changed on request: the right padding is now 2px (`--space-1`) whether or not the strip shows, and the `ResizeObserver` is gone. Checked by hand with `--subpixel`: ⋮ at x≈175 without the strip and x≈168 with it; the focus rings of ⋮ and "+" show in full; ⋮ stays clear of the splitter's grab area (from x≈188; the cursor at x≈182 is `pointer` and at x≈187 is `default`). With the strip, the splitter's grab area covers the strip's rightmost few px, as it did before; drags on the thumb at x≈187 work.
- 2026-10-17: The strip showed about 24px early: under the last Account's list sat its 12px margin as well as the sidebar's 12px padding. The margin is now only between Accounts. Checked by hand: scrolled to the end, 12px are left under the last row, as above the first. In the 800px window the strip still shows from 27 Calendars, since the 27th row ends 2px above the window's edge; at other heights this saves a row.
