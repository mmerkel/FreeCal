# 08: Recurrence scopes when editing and dragging

**What to build:** When the user edits or drags an Occurrence of a Recurring Event, FreeCal asks "this event", "this and following events" or "all events" and changes exactly that. Cancelling the prompt after a drag puts the Occurrence back. Exceptions are kept when unrelated parts of the Recurring Event change, where possible.

**Blocked by:** 05, 07

**Status:** ready-for-agent

**Effort:** L

- [ ] Editing an Occurrence asks for the scope: this / this and following / all
- [ ] Dropping a dragged or resized Occurrence asks for the scope; cancel reverts the drag (component test)
- [ ] While the scope prompt for a drop is open, a "Date changed" waits as it does during a drag (ticket 05): the grid holds back the new today and the App the jump while tracking today (component test)
- [ ] "This event" creates an Exception; "this and following" splits the Recurring Event; "all" changes the whole
- [ ] Exceptions are preserved when unrelated parts of the Recurring Event change, where possible

## Comments
- 2026-10-18 (from ticket 07): The grid offers no drag or resize for any Occurrence of a Recurring Event yet (`CalendarGrid.svelte`, `editable`); enable it with the scope prompt. Editing an Occurrence edits the whole Recurring Event from its first date. Cancelled Occurrences (`exdates`) and Exceptions' `recurrence_id` are wall starts in the start's Event Time Zone, so an edit that moves the start must move them too, or cancelled Occurrences come back and changed ones show twice. Removing the repeat clears `exdates` but leaves Exceptions behind as Events of their own.

- 2026-10-18 (from ticket 73): Once ticket 73 stores `RDATE`, splitting a Recurring Event at an Occurrence splits its extra dates too: those before the Occurrence stay, the rest go to the new Recurring Event.
- 2026-10-18 (from ticket 75): Ticket 75, blocked by this one, keeps an imported Exception with `RANGE=THISANDFUTURE` and applies it to every later Occurrence; it extends these scopes to Occurrences such a range changed. Keep the scope code open to that.
