# 89: Exceptions in the scope dialog

**What to build:** When a change to "this and following events" or "all events" would treat Exceptions with values of their own differently from the rest, the user decides whether they change too. Spec: US-42, US-45. Decided in a grilling on 2026-10-24.

**Blocked by:** none

**Status:** ready-for-agent

**Effort:** M

- [ ] The scope dialog has radio buttons "This event", "This and following events", "All events", in that order, and a solid confirm button labelled for the change ("Change" or "Move"); "This event" is preselected and focused so Enter confirms. For a change of the repeat rule "This event" is shown disabled and "This and following events" is preselected
- [ ] A checkbox "Also change the N events changed on their own" (plural forms through `t()`) applies to Exceptions in the chosen scope whose title, location, description or time differs from what the Recurring Event had. Its count follows the selected scope; it is greyed out for "This event" and when the count is 0, and hidden when both counts are 0. Unchecked by default, which is today's behaviour; it keeps its value while greyed out and is then ignored
- [ ] The core gives both counts in one call before the dialog opens
- [ ] Checked, Exceptions take the new title, location and description, and their times shift by the same amount as the Recurring Event's start and end, in wall time
- [ ] Core test: changing a Recurring Event's Event Time Zone with "all events" moves its Exceptions' recurrence IDs and its cancelled Occurrences with it, so none becomes an Event of its own
- [ ] Core tests for the counts and both checkbox states in both scopes; frontend tests for the dialog states

Out of scope: moving Exceptions to the new time instead of shifting them; noted as a possible addition, if someone misses it.
