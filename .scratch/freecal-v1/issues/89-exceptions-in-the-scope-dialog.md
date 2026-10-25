# 89: Exceptions in the scope dialog

**What to build:** When a change to "this and following events" or "all events" would treat Exceptions with values of their own differently from the rest, the user decides whether they change too. Spec: US-42, US-45. Decided in a grilling on 2026-10-24.

**Blocked by:** 50

**Status:** ready-for-agent

**Effort:** M

- [x] The scope dialog has radio buttons "This event", "This and following events", "All events", in that order, and a solid confirm button labelled for the change ("Change" or "Move"); "This event" is preselected and focused so Enter confirms. For a change of the repeat rule "This event" is shown disabled and "This and following events" is preselected
- [x] A checkbox "Also change the N events changed on their own" (plural forms through `t()`) applies to Exceptions in the chosen scope whose title, location, description or time differs from what the Recurring Event had. Its count follows the selected scope; it is greyed out for "This event" and when the count is 0, and hidden when both counts are 0. Unchecked by default, which is today's behaviour; it keeps its value while greyed out and is then ignored
- [x] The core gives both counts in one call before the dialog opens
- [x] Checked, Exceptions take the new title, location and description, and their times shift by the same amount as the Recurring Event's start and end, in wall time
- [ ] Core test: changing a Recurring Event's Event Time Zone with "all events" moves its Exceptions' recurrence IDs and its cancelled Occurrences with it, so none becomes an Event of its own
- [x] Core tests for the counts and both checkbox states in both scopes; frontend tests for the dialog states

Out of scope: moving Exceptions to the new time instead of shifting them; noted as a possible addition, if someone misses it.

## Comments

- 2026-10-25: Implemented except box 5, which waits on ticket 50: the core interface's `When` has no Event Time Zone yet, so no edit can change it. Meanwhile `an_all_day_recurring_event_made_timed_takes_its_exceptions_and_cancelled_days_along` covers the same moving of recurrence IDs and cancelled Occurrences (`rule::moved_id`, by the wall-time difference of the first starts) for an all-day Recurring Event that gains a zone. Ticket 50 has a comment to add the test.
  - Core: `Core::exception_counts(id, recurrence_id)` gives `ExceptionCounts { thisAndFollowing, all }`, besides the Occurrence being changed, which takes the change anyway. Counted are Exceptions of an Occurrence the rule gives, so not cancelled ones or Events of their own (`Loaded::has_values_of_its_own`). `edit_occurrence` and `reschedule_occurrence` take `exceptions_too`; with it, only the counted Exceptions take every changed field and shift.
  - Choice made: where shifting start and end would put an Exception's end before its start (the Recurring Event made shorter than the gap), the Exception moves by the start's amount and keeps its length (`rule::shifted_or_moved`), instead of the whole change failing.
  - Dialog: with "This event" selected, the greyed-out checkbox names the "all events" count.
  - The window asks for the counts before showing the dialog; a dropped Occurrence is held from the drop on, while they load.
