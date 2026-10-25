# 21: Conflicts on Recurring Events

**What to build:** When a local change to a whole Recurring Event collides with a remote change to one of its Occurrences, FreeCal raises one Conflict on the whole Recurring Event, showing both versions, so the user resolves it as one decision.

**Blocked by:** 20, 91

**Status:** ready-for-agent

**Effort:** M

- [ ] A series edit vs a remote Occurrence edit becomes one Conflict on the whole Recurring Event
- [ ] Both versions are shown, with what each side changed against the common version from the Change Journal; resolving applies the chosen one to the whole Recurring Event and can be undone
- [ ] Written test-first against Radicale

## Comments
- 2026-10-18: Change Journal decisions from grilling tickets 58 and 59 (ADR 0009).
- 2026-10-25 (grilling, ADR 0011): blocked by 91, as ticket 20 is. A Recurring Event's rows are one object on CalDAV, but several on Google; a Conflict there is on the objects whose keys are unsent.
