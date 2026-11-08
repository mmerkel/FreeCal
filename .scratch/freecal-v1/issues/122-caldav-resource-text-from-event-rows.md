# 122: The CalDAV resource to send, built from a UID's Event Rows

**What to build:** The third part of ticket 17 (read it for the history). A pure function that the Sender (ticket 127) uses: from the Event Rows of one Calendar and UID, and the text of its CalDAV Server Object if there is one, it builds the iCalendar text of the resource to send. CalDAV sends one resource per UID, so a Recurring Event and its Exceptions go in one resource.

- With a Server Object, its text is the starting point: the properties FreeCal keeps (title, times and their zones, location, description, repeat rule, excluded dates, recurrence ID) are rewritten from the Event Rows; everything else is kept byte-for-byte, including attendees, conference links, alarms, custom and `X-` properties, their parameters, and other components. An Exception the Event Rows add gets a VEVENT of its own; one they no longer hold loses its VEVENT.
- Without one (a create), it writes a new VCALENDAR with the VTIMEZONEs its times need.
- Recurrence IDs are written as obtained (ADR 0012) and matched to the Server Object's VEVENTs by Occurrence, through their Normal Form.
- Event Rows and the Server Object's text are untrusted (spec: "Untrusted content"): the text built is valid iCalendar whatever they hold, with long lines folded and values escaped.
- Export (ticket 09) may reuse the part that writes new VEVENTs; nothing more is shared now.

Spec: "Local Store" (byte-preservation); "Attendees"; "Untrusted content". ADRs 0011, 0012, 0015.

**Blocked by:** 98, 100

**Status:** done

**Effort:** M

- [x] Unknown fields (attendees, conference links, custom properties) are preserved byte-for-byte across edits, tested on texts from real servers and on edge cases
- [x] Exceptions added, changed and removed land in the right VEVENTs, matched by Occurrence
- [x] A create writes valid iCalendar with the VTIMEZONEs it needs; the text built reads back through the import's reader to the same Events

## Comments
- 2026-11-08 (/implement): done. `resource_to_send` in `core/src/sync/caldav/resource/` builds the text from a `Loaded` and the Server Object's text, stamping new VEVENTs with a `now` the Sender passes.
  - **What gets rewritten.** In each VEVENT, a group of FreeCal's Properties is written anew only when the Event Row differs from what Propagate would read from the VEVENT. Every other line is kept byte for byte, its folding included; only line ends become CRLF. An unchanged Event therefore sends the Server Object's text unchanged.
  - **What a rewritten line loses.** A rewritten line drops its parameters (`SUMMARY;LANGUAGE=de-DE` becomes `SUMMARY`). An edited description also drops `X-ALT-DESC`, so clients don't keep showing the old HTML.
  - **How VEVENTs are matched.** They are paired with Event Rows by Occurrence, through the Event module's pairing (`Loaded::pair_with_server`), against the Recurring Event the Event Rows hold, or else the server's. A paired VEVENT keeps its RECURRENCE-ID line.
  - **New VTIMEZONEs.** They are made from chrono-tz's rules. They list every offset change from the year of the earliest time written through 2099, as DTSTART plus RDATEs, and are added only when the VCALENDAR has no VTIMEZONE with that IANA TZID.
  - **Test texts.** Two are as a local Radicale 3.8.3 returned them after a PUT; Radicale reorders properties and puts some after the VALARM. The Nextcloud, Google, iCloud, Exchange and Evolution texts are written to match what each produces, not captured from those servers. Every resource built was also PUT to that Radicale by hand: all were accepted, and the hostile one was stored without an attendee. That check isn't automated.
  - **New error.** `CoreError::UnreadableServerObject` (error code `unreadableServerObject`) covers Server Object text FreeCal can't read any more, including text with a control character other than a tab inside a line. That rule keeps a kept line from carrying in a property.
  - **For 125: moves.** Moving a Recurring Event rewrites its Exceptions' recurrence IDs (ADR 0012), so their old VEVENTs no longer pair. They are replaced by new ones, and any attendees, alarms or `X-` properties on those Exceptions are lost.
  - **For 127: one unreadable Event Row stops the whole UID.** An Event Row whose stored recurrence ID or time can't be read gives `UnreadableEventTime`, so nothing is built for the UID.
  - **Revision properties (user decision, 2026-11-08).** A VEVENT whose FreeCal's Properties are rewritten also gets `DTSTAMP` and `LAST-MODIFIED` set to now, and its `SEQUENCE` raised by one. Without a METHOD, RFC 5545 §3.8.7 makes `DTSTAMP` mean the time of the last revision, and RFC 5546 §2.1.4 requires a higher `SEQUENCE` when times, the repeat rule or excluded dates change. One rule covers every change, since other changes may raise it too. A new VEVENT gets `DTSTAMP` and `LAST-MODIFIED`, and keeps the default `SEQUENCE` of 0, as each Exception has a sequence of its own.
  - **New terms:**
    - **FreeCal's Properties** (new term): the properties of a VEVENT that an Event Row holds: title, times and their zones, location, description, repeat rule, excluded dates and recurrence ID.
    - Code names: `Revision` (the properties saying when and how often a VEVENT was revised), `ServerPairing` (a Server Object's Events paired with the Event Rows by Occurrence), `Group` (FreeCal's Properties kept or written as one), `Written` (content lines written, with the zones they need), and `ContentLine`, `Block` and `Item` (the Server Object's text as received).
