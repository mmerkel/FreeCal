# 31: Collapsible, resizable sidebar

**What to build:** The user can hide the sidebar completely and show it again, with a button at the left end of the toolbar or with F9, and can change its width by dragging its right edge. FreeCal remembers both across launches. Spec: user story 122, "App state and settings" in the core application interface.

**Blocked by:** 02, 04

**Status:** ready-for-agent

- [ ] A hidden sidebar is fully hidden; the grid takes the whole width
- [ ] One ☰ button at the left end of ticket 04's toolbar shows and hides it, in the same place in both states; its label ("Show sidebar" / "Hide sidebar") goes through `t()` and it carries `aria-expanded`
- [ ] F9 shows and hides it, handled with ticket 04's shortcuts
- [ ] A drag handle on the sidebar's right edge changes the width: default 14rem, minimum 10rem, maximum 40% of the window
- [ ] Dragging below the minimum hides the sidebar; showing it again restores the width it had before that drag
- [ ] Double-clicking the handle resets the width to the default
- [ ] The handle is focusable, has `role="separator"` with its current value, and the arrow keys change the width within the same limits
- [ ] A narrow window shrinks the sidebar no further than its minimum; it never hides on its own
- [ ] Showing it again after hiding restores its last width
- [ ] Shown or hidden and the width are app state in the Local Store, read and written through the core interface that ticket 04 builds for the last-used view; a drag saves once when it ends, a key press saves straight away
- [ ] On launch the sidebar opens as it was left
- [ ] Component tests against the fake core cover the button, F9, keyboard resizing with its limits and the hide below the minimum, restoring the state at launch and saving it on change
- [ ] Dragging is checked by hand in the real app (jsdom has no layout)

Out of scope: remembering the window's own size and position (possibly a separate ticket); the sidebar's contents (tickets 02, 14, 15, 18).
