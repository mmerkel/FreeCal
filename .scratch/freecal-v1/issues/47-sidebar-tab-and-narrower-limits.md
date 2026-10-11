# 47: Sidebar tab and narrower width limits

**What to build:** A tab in the middle of the sidebar's right edge hides and shows the sidebar with a click and pulls it in or out by dragging. It replaces ticket 31's ☰ button. The width limits get narrower: default 12rem, minimum 8rem, maximum a fixed 20rem. A drag stops at the minimum, and only a pull well past it hides the sidebar. Spec: user story 122. Changes ticket 31.

**Blocked by:** none

**Status:** done

- [x] Default width 12rem, minimum 8rem, maximum 20rem whatever the window's size; a stored width outside these shows at the nearest limit, without migrating the stored value. A narrow window still never hides the sidebar on its own
- [x] The ☰ button is gone from the toolbar; the toolbar starts with Today
- [x] A half-tab with a Lucide chevron sits on the edge, centred vertically, sticking out on the grid's side of the line (about 14×40px, rounded on its outer side, in the shared base's tokens). It is always visible and gets stronger on hover, on focus and during a drag. While the sidebar is hidden it stays at the window's left edge, its chevron pointing right
- [x] A press on the tab that moves less than 4px before release is a click and hides or shows the sidebar
- [x] A drag, from the tab or anywhere else on the edge and in either state, follows one rule: the pointer's position decides. Left of 4rem from the window's left edge the sidebar is hidden (live, while dragging); right of it, it is shown at the pointer's position, kept within 8–20rem. So pulling the tab out of a hidden sidebar shows it once the pointer passes 4rem, at 8rem or wider
- [x] Letting go while hidden hides it and keeps the width it had before the drag, so showing it again restores that width. A drag saves once, when it ends, and only when the shown state or width changed (as in ticket 31)
- [x] Hiding and showing slide over about 120ms; with `prefers-reduced-motion` they are instant. A drag is never animated
- [x] One focusable element: the edge is a `role="separator"` (WAI-ARIA window splitter) with the tab as its visible grip. The arrow keys change the width within 8–20rem, Enter hides and shows the sidebar, and F9 still hides and shows it from anywhere. The tab's tooltip is "Hide sidebar (F9)" or "Show sidebar (F9)" through `t()`
- [x] Double-clicking no longer resets the width
- [x] Component tests against the fake core: the tab's click, Enter on the separator, the arrow keys with the new limits, a press that moves 4px or more not toggling, and saving on change; drop the tests for ☰ and the double-click
- [x] Checked by hand in the real app with `run-freecal`: clicking the tab, the dead zone when pulling it out, the snap-hide below 4rem, the limits, and how much of the week view's time axis the hidden tab covers. Update the coordinates and ☰ notes in `.claude/skills/run-freecal/SKILL.md`

Out of scope: the window's minimum size (ticket 39 measured it with a 14rem sidebar; it stays at 880×560); the overlay sidebar in too-narrow windows (ticket 45).

## Comments

- 2026-10-11: Grilled with the user after ticket 31 was done. The look was chosen from a throwaway mock comparing a half-tab, a round button and a slim pill on the line; the half-tab covers the least of the grid while the sidebar is hidden. The drag rule is one function of the pointer's position, so hiding and pulling out mirror each other; the ~60px between the 4px click threshold and the 4rem threshold, in which a pull from hidden shows nothing yet, is intended and guards against a sloppy click on the tab. Enter on the separator follows the WAI-ARIA window splitter pattern. Double-click reset was dropped rather than made to share the tab with clicks.

- 2026-10-11: Done. `SidebarPane` is now always rendered and takes the whole `SidebarState`; hidden, it is 0 wide and its contents are `inert`, so the tab stays at the window's edge. A drag keeps the gap between where the edge was pressed and the sidebar's line (−4 to +14px), so the line doesn't jump when a drag starts on the tab; the 4rem rule applies to the line, so a pull from the tab's middle shows the sidebar once the pointer passes about 71px rather than 64px. The separator's value is 0 while hidden (minimum 0, maximum 20, as in the window splitter pattern), and the arrow keys do nothing while hidden. Checked by hand in the real app (1200px window): the tab's click hides and shows, a pull from hidden shows nothing up to the threshold and then 8rem, the drag follows to 14.56rem and saved once, a drag to x=700 stopped at 20rem, below 4rem the sidebar hid live and letting go kept 20rem, F9 showed it at 20rem, Tab reaches the separator with the focus ring on the tab, the arrows and Enter work. Hidden, the tab covers only the grid's 12px padding and 2px of the week view's time axis border, no labels. The 120ms slide is checked in the CSS only; screenshots can't catch it. `run-freecal` gained `down` and `up` commands for screenshots mid-drag. At 8rem a Calendar's ⋮ menu is clipped past the window's left edge (already so at 10rem): ticket 48.
- 2026-10-11: The tab is 10px wide with a 10px chevron rather than 14px: at 14px it covered 2px of the grid's border, shown and hidden. Chosen by the user from screenshots of 14, 12, 10 and 8px in the real app; 10px leaves 2px of the grid's 12px gutter clear, and 8px made the chevron hard to read. The grab offset is now −4 to +10px, so a pull from the tab's middle shows the sidebar once the pointer passes about 69px.
