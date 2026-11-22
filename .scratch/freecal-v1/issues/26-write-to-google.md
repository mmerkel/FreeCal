# 26: Write to Google Calendars

**What to build:** Writable Google Calendars get the same editing, sending of unsent changes, Calendar History, Conflicts and Mass-Change Guard as CalDAV. When the Google login expires, the user is asked to log in again and sync resumes. Exports keep Google-only details, so a Google Event can be restored completely.

- **Google's writer:** builds an event resource's JSON from an Event Version's managed fields and its Unimplemented Fields, the neutral entries and the `google:` ones. It is the counterpart of ticket 163's reader, as ticket 144's writer is of ticket 143's: what the writer builds reads back as the Event Version it was built from.
- **An Outgoing Sync** sends what differs between the Current-Version Marks (ticket 153). Every update and delete is conditional on the stored ETag, and the closing Database Transaction records the outcome as for CalDAV.
- **The server's form:** Google answers a write with the resource as it stored it. If that differs from what was sent, it is the server's form of FreeCal's own write and takes the sent Event Version's place through ticket 154's function. No read-back is needed.
- **Google's Server Object table** stays at where a resource is and its ETag (ticket 25); a created resource gets its row from the answer.
- **Conflicts, undo and the Mass-Change Guard** need no Google code: they read the Event Versions, the server's marks and the Snapshots' origins (ADRs 0018 to 0020). They are tested against the fake Google server all the same.
- **The interface the two Providers' Sync Engines share** is designed here, now that both read and write.
- **An expired login** is a Sync Error on the Account that asks the user to log in again; after that, sync resumes (US-80).
- **Export and import:** the export (ticket 09) writes the `google:` entries as `X-` properties, and an import reads them back as `google:` entries. An import or a restore into a Calendar whose Provider doesn't hold `google:` drops them (ADR 0017), so the round trip is exact for a Google Calendar.
- **To decide here** (from the grilling of 2026-11-06, Q39): Google can't create an Exception without its Recurring Event's resource. An Exception without its Recurring Event in a Google Calendar, imported or moved there, is for example sent as a single Event.

Spec: US-80, US-93; "Sync engine"; "Import/export"; "Untrusted content". ADRs 0017, 0019.

**Blocked by:** 09, 25, 154

**Status:** ready-for-agent

**Effort:** L

- [ ] Conditional create, update and delete with ETags; what FreeCal doesn't show is written back from the Unimplemented Fields
- [ ] Google's answer to a write is put in place as the server's form through ticket 154's function
- [ ] Undo, Conflicts and the Mass-Change Guard work for Google Calendars, and changes made offline are sent later (tested against the fake Google server)
- [ ] An expired login is a Sync Error on the Account that asks the user to log in again; after re-authentication sync resumes
- [ ] Google-only fields are exported as `X-` properties and read back on import, round-tripping exactly
- [ ] An Exception without its Recurring Event in a Google Calendar is handled as decided here

## Comments
- 2026-10-24: Per ADR 0010, each Google event resource, a Recurring Event or an Exception, is a server object of its own, with its JSON as received.
- 2026-10-25 (grilling, ADR 0011): blocked by 91. A Google Event Row's Corresponding Server Object is the one holding its Calendar, UID and recurrence ID in `combined_event_key`; a new Exception has none until it is created. Fill the Combined Event Keys from Google's JSON (`iCalUID`, `originalStartTime`), with recurrence IDs normalised as the Event Rows hold them.
- 2026-11-06 (grilling, ADRs 0012–0014): blocked by 99 and 100. The comment of 2026-10-25 is outdated: Google gets Mirror tables of its own (ADR 0013), with one Google Server Object per Google event resource holding its UID (`iCalUID`), its recurrence ID as obtained (`originalStartTime`), Google's link to its Recurring Event (`recurringEventId`) and the JSON as received; there's no key table and no normalising (ADR 0012). Its Corresponding Server Object is the one holding its Calendar, UID and Occurrence, compared by Occurrence. The Sync Engine's Google code goes in `core/src/sync/google/`; this ticket designs the interface it shares with CalDAV's, now that there are two Providers. Conflicts need no Google code beyond reading the Server Version (ADR 0014): a changed Google Server Object is one Occurrence or the Recurring Event.
- 2026-11-06 (grilling, Q39): an Exception without its Recurring Event in a Google Calendar, whether imported (ticket 101 is wontfix, so imports keep such Exceptions) or moved there from another Calendar, needs a decision here: Google can't create an Exception without its Recurring Event's resource, so for example the Sender sends it as a single Event.
- 2026-11-16: Reworded from the triage against ADRs 0017 to 0020. Google's Server Object table holds where a resource is and its ETag only; a write's answer is the server's form, put in place by ticket 154's function. Blocked by 25, 154 and 09. The comments of 2026-10-24, 2026-10-25 and 2026-11-06 are outdated where they name Mirror tables, JSON as received, Event Rows, Corresponding Server Objects and the Server Version read from a Server Object; the Q39 comment stands and is in the body. Effort stays L: it was sized before the M cap (2026-11-16) and wasn't on the triage's list of splits. A split into "Google's writer and the Outgoing Sync", "the expired login" and "Google-only fields in exports" would bring every part to M or below; for the user to decide.
- 2026-11-22: Reworded from the grilling on critique 2 (ADRs 0018 to 0020 rewritten): no pointers or Reference Snapshot.
