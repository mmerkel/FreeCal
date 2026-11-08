# 122: The CalDAV resource to send, built from a UID's Event Rows

**What to build:** The third part of ticket 17 (read it for the history). A pure function that the Sender (ticket 123) uses: from the Event Rows of one Calendar and UID, and the text of its CalDAV Server Object if there is one, it builds the iCalendar text of the resource to send. CalDAV sends one resource per UID, so a Recurring Event and its Exceptions go in one resource.

- With a Server Object, its text is the starting point: the properties FreeCal keeps (title, times and their zones, location, description, repeat rule, excluded dates, recurrence ID) are rewritten from the Event Rows; everything else is kept byte-for-byte, including attendees, conference links, alarms, custom and `X-` properties, their parameters, and other components. An Exception the Event Rows add gets a VEVENT of its own; one they no longer hold loses its VEVENT.
- Without one (a create), it writes a new VCALENDAR with the VTIMEZONEs its times need.
- Recurrence IDs are written as obtained (ADR 0012) and matched to the Server Object's VEVENTs by Occurrence, through their Normal Form.
- Event Rows and the Server Object's text are untrusted (spec: "Untrusted content"): the text built is valid iCalendar whatever they hold, with long lines folded and values escaped.
- Export (ticket 09) may reuse the part that writes new VEVENTs; nothing more is shared now.

Spec: "Local Store" (byte-preservation); "Attendees"; "Untrusted content". ADRs 0011, 0012, 0015.

**Blocked by:** 98, 100

**Status:** ready-for-agent

**Effort:** M

- [ ] Unknown fields (attendees, conference links, custom properties) are preserved byte-for-byte across edits, tested on texts from real servers and on edge cases
- [ ] Exceptions added, changed and removed land in the right VEVENTs, matched by Occurrence
- [ ] A create writes valid iCalendar with the VTIMEZONEs it needs; the text built reads back through the import's reader to the same Events

## Comments
