# 126: Attendee features

**What to build:** Attendee features before v1 (US-151): showing an Event's attendees, answering invitations (RSVP), inviting people, and editing Events with guests. Ticket 121 made such Events read-only for now (spec: "Attendees", US-31, US-46). The first step is a grilling session with the user; the decisions go into this ticket and into ADRs where they are lasting, and the ticket is then split into implementation tickets.

Questions for the grilling, among others:

- Which features, in which order: showing attendees, RSVP, editing Events with guests, inviting, and whether all of them come before v1.
- How changes reach the attendees: through the server's scheduling (CalDAV RFC 6638, Google's `sendUpdates`), or never by email from FreeCal. What a server without scheduling support, such as Radicale, means for this.
- The user's role per Event: organiser or attendee, and what each may change. What `SEQUENCE`, `PARTSTAT` and the other iTIP rules ask of FreeCal's writes (ticket 176's writer builds the CalDAV resource).
- Where attendees live: today they are among an Event's Unimplemented Fields (ADR 0017), written back unchanged. Which of their parts become managed fields with columns, which get neutral names (ticket 210), and what the migration moves.
- Whether the user is asked before a change notifies other people, and how this fits the Mass-Change Guard and the Calendar History: Ctrl+Z, an Undo from the History and a Backup restore each send what differs like any other change (ADR 0020), so each can notify attendees. Whether a change in attendees alone can be a Conflict, or the server's wins as for Unimplemented Fields.
- Own Addresses (`core/src/guests.rs`): whether they are read only when an Account is added, as now, or kept up to date, and how the user corrects them.
- Untrusted content: attendee names and addresses shown in the window, and the hostile test Event.

Spec: US-151, US-31, US-46, US-47; "Attendees"; "Untrusted content".

**Blocked by:** none

**Status:** ready-for-human

**Effort:** XL

- [ ] Grilling session with the user; decisions recorded here and in ADRs
- [ ] Split into implementation tickets with Effort and Blocked by lines

## Comments
- 2026-11-16: To decide here: who raises `SEQUENCE`. Until then FreeCal keeps a server's `SEQUENCE` among the Unimplemented Fields and writes it back unchanged (ADR 0017, ticket 144), which is only right while FreeCal changes no Event with guests.
- 2026-11-16: Reworded from the triage against ADRs 0017 to 0020: the questions name the Calendar History and the Unimplemented Fields.
- 2026-11-21: From the grilling on read-only Calendars: until this ticket, undo, redo and a Backup restore change Events with guests like any Event (ADR 0020, "Events with guests"). Its design must cover them: what an undo of a change to a meeting sends, and whether attendees are notified (on CalDAV servers that schedule, RFC 6638, a write by the organiser can notify them).
- 2026-11-28: Reworded from point 6 of `.scratch/freecal-v1/critiques-2.md`: "Go back to before this" is gone (ADR 0020); an Undo from the History takes its place.
- 2026-11-28: References in the body renumbered for the redesign (new tickets 173 to 214). Comments above this one cite the old numbers; `redesign-plan.md`, "Old tickets in scope", maps them.
