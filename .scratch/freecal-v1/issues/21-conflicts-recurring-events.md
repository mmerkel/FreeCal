# 21: Conflicts on Recurring Events

**What to build:** When a local change to a whole Recurring Event collides with a remote change to one of its Occurrences, FreeCal raises one Conflict on the whole Recurring Event, showing both versions, so the user resolves it as one decision.

**Blocked by:** 20

**Status:** ready-for-agent

- [ ] A series edit vs a remote Occurrence edit becomes one Conflict on the whole Recurring Event
- [ ] Both versions are shown; resolving applies the chosen one to the whole Recurring Event
- [ ] Written test-first against Radicale
