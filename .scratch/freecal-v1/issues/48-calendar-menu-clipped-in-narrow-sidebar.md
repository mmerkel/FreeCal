# 48: Calendar menu clipped in a narrow sidebar

**What to build:** A Calendar's ⋮ menu shows in full whatever the sidebar's width. Today it opens below its row, anchored to the row's right end and inside the sidebar, whose `overflow-y: auto` (and the pane's `overflow: hidden`) clips it. The menu is about 178px wide, so below about 11.5rem its left part (the first column of swatches, the start of Rename, Show only this and Delete) runs past the window's left edge. That was already so at ticket 31's 10rem minimum; ticket 47's 8rem minimum makes it worse.

The fix goes in the shared `Menu.svelte`, so the toolbar's main menu gets it too: the menu opens in the top layer as a `popover="manual"`, anchored to the button that opened it with CSS anchor positioning (`position-area: bottom`, `position-try-fallbacks: flip-block`). It is centred under the button, shifts sideways just enough to stay inside the window, and opens upward when there is no room below. All of this is plain CSS; no script measures or places the menu. While a menu is open, scrolling outside it is blocked, as in native menus.

**Blocked by:** none

**Status:** done

**Effort:** M

- [x] `Menu` renders as `popover="manual"` and is shown with `showPopover()`; its open, close, focus-return, outside-click and key handling stay as now (ticket 32's menu)
- [x] It is anchored to its opening button with `anchor-name` / `position-anchor`, `position-area: bottom` and `position-try-fallbacks: flip-block`; the absolute `.menu-anchor` placement in `Sidebar.svelte` and `Toolbar.svelte` goes, with its "inside the sidebar, so it isn't clipped" comment
- [x] Both the Calendar ⋮ menu and the main menu use it, with no placement setting per menu; the main menu ends flush with the window's right edge
- [x] Opening by right-click, the ContextMenu key or Shift+F10 anchors to the row's ⋮ button as well
- [x] While a menu is open, `wheel` and `touchmove` outside it are cancelled (non-passive listeners, next to the events `Menu` already swallows); events over the menu pass through. Window resize needs no handling, since the anchoring follows the button
- [x] `src/test/setup.ts` stubs `showPopover` / `hidePopover` for jsdom, like `showModal`
- [x] Tests: the menu uses `popover="manual"` and `showPopover()`; a wheel outside the open menu is cancelled; the existing menu tests in `Sidebar.test.ts` and `App.settings.test.ts` still pass
- [x] Checked by hand with `run-freecal`, results noted below: at sidebar widths 8rem, 12rem and 20rem the whole Calendar menu shows, centred under ⋮, inside the window; a Calendar near the bottom of a short window opens its menu upward; the main menu shows in full; the sidebar and grid don't scroll under an open menu

Out of scope: the popovers of tickets 38 and 43, which can build on this approach.

## Comments

- 2026-10-11: Found while checking ticket 47 by hand with `run-freecal`: `update app_state set sidebar_width = 8`, restart, then the Home row's ⋮ at about 106,56. Possible ways: open the menu in the top layer (a popover) placed against the ⋮ button, or anchor it to the row's left end and let it overlap the grid, which the sidebar's `overflow` currently prevents.
- 2026-10-16: Triaged. Chose the top-layer popover over left-anchoring inside the sidebar (the sidebar is a scroll container, so it clips sideways too) and over raising the minimum width. Checked in the real webview (WebKitGTK 2.52.6): `popover="manual"`, `anchor-name`, `position-anchor`, `anchor()`, `position-area` and `position-try-fallbacks: flip-block` are all supported. A 180×150 popover with `position-area: bottom` was centred under a mid-window anchor, shifted to left 0 next to an anchor at the left edge, to the right edge next to one at the right edge, and flipped up and shifted at the bottom-left corner; `justify-self: anchor-center` and `span-all` changed nothing. Blocking scroll was preferred over closing the menu on scroll, to match native desktop menus. Placement is checked by hand, since jsdom does no layout.
- 2026-10-16: Done. Checked by hand with `run-freecal`: at sidebar widths 8rem, 12rem and 20rem the whole Calendar menu showed, centred under ⋮ (at 8rem from x≈17 to 195), inside the window. With 30 Calendars, the menu of a row near the bottom (y≈756) opened upward, centred. A right-click on a row opened the menu under that row's ⋮. The main menu showed in full, flush with the window's right edge. While a menu was open, the wheel scrolled neither the sidebar nor the week grid; with it closed both scrolled. Two findings from the check. First, WebKitGTK's overlay scrollbar of a scrolling sidebar paints over an open menu while the pointer is over the sidebar's area; it is gone once the pointer leaves. Second, the scrollbar takes clicks near x≈170, so the skill now says to click ⋮ at x≈164. Review found that a Calendar update while the menu was open reopened it and moved its focus; `Menu` now reads `anchor` once, untracked.
