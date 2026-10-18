# 21: Conflicts on Recurring Events

**What to build:** When a local change to a whole Recurring Event collides with a remote change to one of its Occurrences, FreeCal raises one Conflict on the whole Recurring Event, showing both versions, so the user resolves it as one decision.

**Blocked by:** 20

**Status:** ready-for-agent

**Effort:** M

- [ ] A series edit vs a remote Occurrence edit becomes one Conflict on the whole Recurring Event
- [ ] Both versions are shown, with what each side changed against the common version from the Change Journal; resolving applies the chosen one to the whole Recurring Event and can be undone
- [ ] Written test-first against Radicale

## Comments
- 2026-10-18: Change Journal decisions from grilling tickets 58 and 59 (ADR 0009).
