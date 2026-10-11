# 45: Layout below the minimum window size

**What to build:** When a tiling window manager makes the window smaller than its minimum size (ticket 39), FreeCal currently keeps its 880×560 layout and the page scrolls. Instead, the layout should adapt so that FreeCal stays usable at about half of a 1366 px screen (≈ 680 px) and below. Start by brainstorming the options with the user (use the `grilling` skill), record the chosen ones here and in the spec, and then build them. Spec: user story 144, "Window size" under Frontend.

**Blocked by:** 04, 31

**Status:** ready-for-agent

- [ ] Brainstorm with the user and write the decisions down as a comment here and in the spec's "Window size". Options to weigh, among others:
  - collapse the sidebar automatically below a width, as an overlay that opens with ticket 47's tab and F9 and closes after a choice, without changing the remembered shown/hidden state (ticket 31 says the sidebar never hides on its own; this ticket changes that only for the too-narrow case)
  - switch to a narrower view: fewer days in the week view (e.g. 3 days or the day view), or hide weekends
  - a compact toolbar: icons only, the title shortened
  - smaller chips in the month view: only the dot and the title, or "+n" sooner
  - dialogs as full-window sheets when they don't fit
  - where the breakpoints sit, and whether the page still scrolls below the smallest supported size
- [ ] Build the chosen options; breakpoints use CSS container or media queries where possible, not size checks in script
- [ ] Every chosen option has a component test where jsdom allows it; layout is checked by hand in the real app with `run-freecal` at the breakpoints and at 406 px wide (a KWin tile dragged narrow), with screenshots
- [ ] Leaving the narrow layout restores the normal one as it was, including the sidebar's remembered state

Out of scope: lowering the minimum window size itself (ticket 39); touch or phone layouts.

## Comments

- 2026-10-11: Created from ticket 39. KWin on X11 ignores a window's minimum size when quick tiling, by design ([KDE bug 434615](https://bugs.kde.org/show_bug.cgi?id=434615)), and KDE's own System Settings dealt with the same problem by lowering its minimum and planning to hide its sidebar at narrow widths ([bug 395476](https://bugs.kde.org/show_bug.cgi?id=395476)).
- 2026-10-11: Ticket 47 replaces ticket 31's ☰ button with a tab on the sidebar's edge, so the overlay opens with that tab and F9.
