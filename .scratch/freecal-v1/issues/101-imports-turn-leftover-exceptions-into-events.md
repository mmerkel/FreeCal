# 101: Imports turn Exceptions without their Recurring Event into single Events

**What to build:** Agreed on 2026-11-04 (grilling of the altitude findings of 90–92, Q12). An import pairs each Exception with its Recurring Event, from the same import or already stored in the Calendar, compared by Occurrence (ADR 0012). An Exception left with neither becomes a plain single Event: no recurrence ID, and a UID of its own derived from its UID and recurrence ID, so that importing the same file again finds it, and a Recurring Event with its UID imported later doesn't replace it. An Incoming Sync keeps such Exceptions as they are (ticket 88), so that an invitation to one Occurrence stays visible and finds its Recurring Event if it arrives. Imports stop doing that, as nothing would ever bring the Recurring Event.

**Blocked by:** 98

**Status:** wontfix

**Effort:** S

- [ ] An Exception imported together with its Recurring Event, or into a Calendar that holds it, becomes its Exception
- [ ] An Exception without either becomes a single Event with no recurrence ID and the derived UID; its times and fields are the Exception's
- [ ] Importing the same file again with "update" or "skip" finds that Event by the derived UID
- [ ] An Incoming Sync still keeps an Exception without its Recurring Event as ticket 88 decided

## Comments

- 2026-11-06: The derived UID is a proposal made while writing this ticket (the grilling said "possibly"); confirm or change it before implementing.
- 2026-11-06 (grilling, Q39): wontfix. Since recurrence IDs are stored as given (ADR 0012), an Exception joining its Recurring Event writes nothing, so keeping one without its Recurring Event costs imports nothing. Imports keep it as ticket 88 decided, as Incoming Syncs do: one rule instead of two. It shows as an Event of its own, exports with its recurrence ID unchanged, and pairs with its Recurring Event if a later import brings it. The import half of Q12 (2026-11-04) is reversed. The Google case, where an Exception can't exist without its Recurring Event, goes to ticket 26.
