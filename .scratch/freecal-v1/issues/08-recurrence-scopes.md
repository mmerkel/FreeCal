# 08: Recurrence scopes when editing and dragging

**What to build:** When the user edits or drags an Occurrence of a Recurring Event, FreeCal asks "this event", "this and following events" or "all events" and changes exactly that. Cancelling the prompt after a drag puts the Occurrence back. Exceptions are kept when unrelated parts of the Recurring Event change, where possible.

**Blocked by:** 05, 07

**Status:** done

**Effort:** L

- [x] Editing an Occurrence asks for the scope: this / this and following / all
- [x] Dropping a dragged or resized Occurrence asks for the scope; cancel reverts the drag (component test)
- [x] While the scope prompt for a drop is open, a "Date changed" waits as it does during a drag (ticket 05): the grid holds back the new today and the App the jump while tracking today (component test)
- [x] "This event" creates an Exception; "this and following" splits the Recurring Event; "all" changes the whole
- [x] Exceptions are preserved when unrelated parts of the Recurring Event change, where possible

## Comments
- 2026-10-18 (from ticket 07): The grid offers no drag or resize for any Occurrence of a Recurring Event yet (`CalendarGrid.svelte`, `editable`); enable it with the scope prompt. Editing an Occurrence edits the whole Recurring Event from its first date. Cancelled Occurrences (`exdates`) and Exceptions' `recurrence_id` are wall starts in the start's Event Time Zone, so an edit that moves the start must move them too, or cancelled Occurrences come back and changed ones show twice. Removing the repeat clears `exdates` but leaves Exceptions behind as Events of their own.

- 2026-10-18 (from ticket 73): Once ticket 73 stores `RDATE`, splitting a Recurring Event at an Occurrence splits its extra dates too: those before the Occurrence stay, the rest go to the new Recurring Event.
- 2026-10-18 (from ticket 75): Ticket 75, blocked by this one, keeps an imported Exception with `RANGE=THISANDFUTURE` and applies it to every later Occurrence; it extends these scopes to Occurrences such a range changed. Keep the scope code open to that.
- 2026-10-18 (done): The core takes `edit_occurrence` and `reschedule_occurrence` with a `Scope` (`core/src/scope.rs`); `edit_event` and `reschedule_event` on a Recurring Event or an Exception go through it as "all" and "this", so a Calendar move or a removed repeat no longer leaves Exceptions behind. A change is what differs between the Occurrence as it was and what was saved: Exception fields shared with the Recurring Event follow it, ones changed on their own stay, and an Exception at its Occurrence's time moves with a time change. Cancelled Occurrences and Exceptions move with the first start and are dropped when the new rule no longer has their date; removing the repeat removes the Exceptions. A move to another day moves `BYDAY` and `BYMONTHDAY` along. "This event" into another Calendar takes the Occurrence out as an Event of its own; "this and following" from the first Occurrence is "all"; the first part of a split ends with `UNTIL` at its last Occurrence before the split. The scope is asked on Save and on a drop, without "this event" when the repeat changed; the editor starts from the Occurrence's date, and an Exception shows its Recurring Event's repeat. Left as they are: `BYHOUR`/`BYMINUTE` don't move with a time change; a split whose first part has only cancelled Occurrences left keeps it; Exceptions past the 50,000th Occurrence are dropped by "all"; the warning for moves across Accounts comes with server Accounts.
