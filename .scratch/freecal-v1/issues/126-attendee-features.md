# 126: Attendee features

**What to build:** Attendee features before v1 (US-151): showing an Event's attendees, answering invitations (RSVP), inviting people, and editing Events with guests. Ticket 121 made such Events read-only for now (spec: "Attendees", US-31, US-46). The first step is a grilling session with the user; the decisions go into this ticket and into ADRs where they are lasting, and the ticket is then split into implementation tickets.

Questions for the grilling, among others:

- Which features, in which order: showing attendees, RSVP, editing Events with guests, inviting, and whether all of them come before v1.
- How changes reach the attendees: through the server's scheduling (CalDAV RFC 6638, Google's `sendUpdates`), or never by email from FreeCal. What a server without scheduling support, such as Radicale, means for this.
- The user's role per Event: organiser or attendee, and what each may change. What `SEQUENCE`, `PARTSTAT` and the other iTIP rules ask of FreeCal's writes (ticket 122 builds the CalDAV resource text).
- Whether the user is asked before a change notifies other people, and how this fits the Mass-Change Guard and the Change Journal (an undo that would notify attendees).
- Own Addresses (`core/src/guests.rs`): whether they are read only when an Account is added, as now, or kept up to date, and how the user corrects them.
- Untrusted content: attendee names and addresses shown in the window, and the hostile test Event.

Spec: US-151, US-31, US-46, US-47; "Attendees"; "Untrusted content".

**Blocked by:** none

**Status:** ready-for-human

**Effort:** XL

- [ ] Grilling session with the user; decisions recorded here and in ADRs
- [ ] Split into implementation tickets with Effort and Blocked by lines

## Comments
