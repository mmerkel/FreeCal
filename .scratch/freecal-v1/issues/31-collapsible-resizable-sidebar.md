# 31: Collapsible, resizable sidebar

**What to build:** The user can hide the sidebar completely and show it again, with a button at the left end of the toolbar or with F9, and can change its width by dragging its right edge. FreeCal remembers both across launches. Spec: user story 122, "App state and settings" in the core application interface.

**Blocked by:** 02, 04, 32

**Status:** done

- [x] A hidden sidebar is fully hidden; the grid takes the whole width
- [x] One ☰ button at the left end of ticket 04's toolbar shows and hides it, in the same place in both states; its label ("Show sidebar" / "Hide sidebar") goes through `t()` and it carries `aria-expanded`
- [x] F9 shows and hides it, handled with ticket 04's shortcuts
- [x] A drag handle on the sidebar's right edge changes the width: default 14rem, minimum 10rem, maximum 40% of the window
- [x] Dragging below the minimum hides the sidebar; showing it again restores the width it had before that drag
- [x] Double-clicking the handle resets the width to the default
- [x] The handle is focusable, has `role="separator"` with its current value, and the arrow keys change the width within the same limits
- [x] A narrow window shrinks the sidebar no further than its minimum; it never hides on its own
- [x] Showing it again after hiding restores its last width
- [x] Shown or hidden and the width are app state in the Local Store, read and written through the core interface that ticket 04 builds for the last-used view; a drag saves once when it ends, a key press saves straight away
- [x] On launch the sidebar opens as it was left
- [x] Component tests against the fake core cover the button, F9, keyboard resizing with its limits and the hide below the minimum, restoring the state at launch and saving it on change
- [x] Dragging is checked by hand in the real app (jsdom has no layout)

Out of scope: remembering the window's own size and position (possibly a separate ticket); the sidebar's contents (tickets 02, 14, 15, 18); collapsing the sidebar in a window below its minimum size (ticket 45).

## Comments

2026-10-04: Checked against ticket 32's look. Nothing conflicts: 32's sidebar default width is also 14rem, the drag handle sits on its 1px right border, and the ☰ button should be a ghost button from the shared base with a Lucide icon. Blocked by 32 so that it is built on the shared base.

2026-10-11: Done. The width is stored in `rem` (`null` for the 14rem default) in `app_state.sidebar_shown` and `sidebar_width`, through the core's `sidebar()` and `setSidebar()`. The ☰ button is the first item of the toolbar in both states, so on screen it moves left by the sidebar's width when the sidebar hides; it is never inside the sidebar. While a drag is below the minimum only the handle shows, and letting go there hides the sidebar. A drag that leaves the shown width unchanged saves nothing, so a stored width wider than a narrow window allows survives a click on the handle. Checked by hand in the real app (1200px window): drag to 24rem saved once, drag past the maximum stopped at 30rem, drag below the minimum hid it and kept 30rem, a restart opened it hidden, F9 showed it at 30rem, a double-click reset it to 14rem, ☰ hid it.

2026-10-11: Ticket 47 replaces parts of this: the ☰ button gives way to a tab on the sidebar's edge, the limits become 12rem default, 8rem minimum and a fixed 20rem maximum, only a pull below 4rem from the window's left edge hides the sidebar, Enter on the handle hides and shows it, and the double-click reset is dropped.
