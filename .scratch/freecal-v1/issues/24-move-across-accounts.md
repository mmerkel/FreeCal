# 24: Move Events across Accounts

**What to build:** The user can move an Event to a Calendar in another Account, for example turning a local draft into a real server Event. FreeCal warns first only when the Event has Provider-specific details the target can't hold.

- **One Snapshot spanning both Calendars** records the move (ADR 0018; ticket 179 makes it for a move between any two Calendars). Each server Calendar's Outgoing Sync then sends what differs on its side, a create in the target and a delete in the source, with no code of the move's own (ticket 192).
- **What is carried:** the managed fields, the neutral entries of the Unimplemented Fields and the entries with a prefix the target's Provider holds; the rest is dropped (ADR 0017). Ticket 186 wrote that rule as one function that asks the target's Provider for its prefixes. A Recurring Event moves with its Exceptions.
- **The warning** appears exactly when that function would drop an entry, and names what is lost in the user's words ("details only Google holds"). Cancelling skips the move. Between Local and CalDAV Calendars nothing is dropped, so the warning first shows once Google's entries exist (tickets 210 and 212).
- **A pinned zone** (ADR 0007) is written to the target made explicit, since the target Calendar may read floating times differently.
- **Undo:** Ctrl+Z takes the whole move back. Undo for one of the two Calendars leaves the other as it is, so the Event is then in both or in neither, and FreeCal says so (ADR 0020; ticket 181).

Spec: US-48; "Moves across Accounts". ADRs 0017, 0018, 0020.

**Blocked by:** 186, 192

**Status:** ready-for-agent

**Effort:** S

- [ ] Changing an Event's Calendar to one in another Account is one Snapshot spanning both Calendars, sent as a create to the target and a delete to the source
- [ ] Neutral entries and those the target's Provider holds are carried; the others are dropped
- [ ] The warning appears only when entries would be dropped, and the move is skipped if the user cancels
- [ ] A pinned zone is made explicit in the target

## Comments
- 2026-10-11: An Event whose zone was pinned (ADR 0007) is written to the target with the pinned zone made explicit, since the target Calendar may read floating times differently.
- 2026-11-07: Re-sized from M to S. A Journal Transaction already adds Unsent Changes for both the previous and the new Event Version's Calendar (ADR 0015), so once ticket 17's Sender exists a move across Accounts sends a create to the target and a delete to the source without code of its own. What's left: the warning about Provider-specific details (only Google Events have them, tickets 25 and 26), the pinned zone made explicit, and tests.
- 2026-11-16: Reworded from the triage against ADRs 0017 to 0020: one Snapshot spanning both Calendars; the neutral entries and the target Provider's are carried. Blocked by 149 and 153. The comment of 2026-11-07 is outdated where it names Journal Transactions and Unsent Changes; its conclusion holds, as the two Outgoing Syncs send the move without code of its own.
- 2026-11-28: References in the body renumbered for the redesign (new tickets 173 to 214). Comments above this one cite the old numbers; `redesign-plan.md`, "Old tickets in scope", maps them.
