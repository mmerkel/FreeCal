# 124: Changes are sent as they're made, and a refused write is a Sync Error

**What to build:** The fifth part of ticket 17 (read it for the history). A change in a writable server Calendar is applied to the Local Store at once, as now, and then sent straight away by an Outgoing Sync of its Calendar (ticket 127's Sender), not only at the next sync. Sends that fail are kept apart by why.

- After a change, the core asks the Sync Scheduler for an Outgoing Sync of the changed Calendars while the Account is online; offline replay is ticket 18.
- A server refusing writes to a Calendar (for example 403, or privileges lost) becomes a Sync Error on that Calendar (spec: "Sync Error"); its Unsent Changes stay.
- Decide here what an Event shows when its send fails, "Not yet synced: <reason>": what reasons there are and where the reason is kept (`unsent_change` has no reason column yet). Ticket 18 shows it.

Spec: "Sync Error"; "Event details".

**Blocked by:** 15, 127, 128

**Status:** ready-for-agent

**Effort:** M

- [ ] A change in a writable server Calendar is sent right after it is made while the Account is online
- [ ] A server refusing writes to a Calendar becomes a Sync Error on that Calendar
- [ ] The reason a send failed is kept for "Not yet synced: <reason>" (shown by ticket 18)
- [ ] Core tests with a controllable clock, and tested against Radicale

## Comments
