# 08: Recurrence scopes when editing and dragging

**What to build:** When the user edits or drags an Occurrence of a Recurring Event, FreeCal asks "this event", "this and following events" or "all events" and changes exactly that. Cancelling the prompt after a drag puts the Occurrence back. Exceptions are kept when unrelated parts of the Recurring Event change, where possible.

**Blocked by:** 05, 07

**Status:** ready-for-agent

- [ ] Editing an Occurrence asks for the scope: this / this and following / all
- [ ] Dropping a dragged or resized Occurrence asks for the scope; cancel reverts the drag (component test)
- [ ] "This event" creates an Exception; "this and following" splits the Recurring Event; "all" changes the whole
- [ ] Exceptions are preserved when unrelated parts of the Recurring Event change, where possible
