# 169: The round-trip check at fetch: the Log names what couldn't be carried, and the Event's details warn

**What to build:** When an Incoming Sync or a First Sync fetches a CalDAV resource, FreeCal writes it back in memory, from what ticket 143's reader took, with ticket 144's writer, and compares the two texts (ADR 0017, "The round-trip check"). What would be lost goes to the Log and into a mark the Event's details show. Nothing else changes: the Event can be edited, deleted, moved and sent as any other. Point 3 of `.scratch/freecal-v1/critiques.md` has the reasons.

- **The comparison:** both texts parsed only into components, properties, parameters and values, with the `icalendar` crate the core already uses, never with FreeCal's reader on both sides (it would share the reader's blind spots). It ignores folding, line endings, the letter case of names, order where the tree ignores it (ticket 143's canonical form says where), and what the writer sets fresh: `DTSTAMP`, `LAST-MODIFIED`, `PRODID`, `VERSION`, a default `CALSCALE`, and the time zone definitions. A managed property's value may differ when the reader reads the same managed field from both texts (an Outlook `TZID` written as an IANA name, `RRULE` parts in another order). What ticket 143's list of deliberate drops names may be missing. Anything else missing from the written text was lost.
- **The Log:** one line per fetched resource that lost something, naming its Calendar, href and the names of the parts that couldn't be carried (property, parameter and component names, never their values: "Untrusted content"). The RFC violations ticket 143's reader returns are written to the Log in the same place, one line per resource.
- **The mark:** a column of the CalDAV table (`caldav_server_object`) beside the ETag, written in the same statement as the ETag after every fetch, so a later version of the resource that passes clears it. It goes with the row. Nothing reads it to decide on an edit, a send or a Conflict.
- **The details:** the Event type of the core interface carries whether its UID's resource has the mark, as it carries `hasGuests`. The Event's details say: "FreeCal couldn't read everything in this event. Changing it here may remove data that other apps stored in it." No toast, notification or chip mark.
- **Only CalDAV:** imports and ICS feeds have no check; Google's reader gets its own in ticket 163.

Spec: "Event details"; "Untrusted content". ADR 0017.

**Blocked by:** 144, 149

**Status:** ready-for-agent

**Effort:** M

- [ ] A resource that reads and writes back whole gives no Log line and no mark
- [ ] Folding, line endings, letter case, order, `DTSTAMP`, `LAST-MODIFIED`, `PRODID`, `VERSION` and a renamed `TZID` don't count as lost
- [ ] A part the reader leaves out (tested by giving the check a reader stub that drops a property) gives a Log line naming it and sets the mark
- [ ] What the list of deliberate drops names isn't reported
- [ ] A later fetch of a version that passes clears the mark
- [ ] The details show the note for an Event whose resource has the mark; the Event can still be edited and deleted
- [ ] The Log line holds names only, no values

## Comments
- 2026-11-22: Written from the grilling of point 3 of `critiques.md`. The detector, not a guard: making such resources read-only was rejected (ADR 0017, Considered Options).
