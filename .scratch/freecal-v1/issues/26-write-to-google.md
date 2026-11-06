# 26: Write to Google Calendars

**What to build:** Writable Google Calendars get the same editing, offline replay, Change Journal, Conflict and Mass-Change Guard behaviour as CalDAV. When the Google login expires, the user is asked to log in again and sync resumes. Exports keep Google-only details, so a Google Event can be restored completely.

**Blocked by:** 09, 18, 25, 91, 97, 99, 100

**Status:** ready-for-agent

**Effort:** L

- [ ] Conditional create, update and delete with ETags; unknown fields preserved byte-for-byte
- [ ] Change Journal, Conflicts, the Mass-Change Guard and offline replay work for Google Calendars (tested against the fake Google server)
- [ ] An expired login is a Sync Error on the Account that asks the user to log in again; after re-authentication sync resumes
- [ ] Google-only fields are exported as `X-` properties and read back on import, round-tripping exactly

## Comments
- 2026-10-24: Per ADR 0010, each Google event resource, a Recurring Event or an Exception, is a server object of its own, with its JSON as received.
- 2026-10-25 (grilling, ADR 0011): blocked by 91. A Google Event Row's Corresponding Server Object is the one holding its Calendar, UID and recurrence ID in `combined_event_key`; a new Exception has none until it is created. Fill the Combined Event Keys from Google's JSON (`iCalUID`, `originalStartTime`), with recurrence IDs normalised as the Event Rows hold them.
- 2026-11-06 (grilling, ADRs 0012–0014): blocked by 99 and 100. The comment of 2026-10-25 is outdated: Google gets Mirror tables of its own (ADR 0013), with one Google Server Object per Google event resource holding its UID (`iCalUID`), its recurrence ID as given (`originalStartTime`), Google's link to its Recurring Event (`recurringEventId`) and the JSON as received; there's no key table and no normalising (ADR 0012). Its Corresponding Server Object is the one holding its Calendar, UID and Occurrence, compared by Occurrence. The Sync Engine's Google code goes in `core/src/sync/google/`; this ticket designs the interface it shares with CalDAV's, now that there are two Providers. Conflicts need no Google code beyond reading the Server Version (ADR 0014): a changed Google Server Object is one Occurrence or the Recurring Event.
- 2026-11-06 (grilling, Q39): an Exception without its Recurring Event in a Google Calendar, whether imported (ticket 101 is wontfix, so imports keep such Exceptions) or moved there from another Calendar, needs a decision here: Google can't create an Exception without its Recurring Event's resource, so for example the Sender sends it as a single Event.
