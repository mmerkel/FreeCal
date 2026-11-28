# 212: Add a Google Account's Calendars and pull them, read-only

**What to build:** The user chooses which of a Google Account's Available Calendars (211) to add, and sees them in FreeCal, kept up to date by Incoming Syncs through the same Incoming Sync Plan as CalDAV. Old ticket 25 (`git show design/snapshot-line:.scratch/freecal-v1/issues/25-add-google-calendars-read-only.md`), reworded to the marks: it named a pointer and a Join, which ADR 0019 replaced.

- **Adding Calendars** goes through 208's dialog and 194's `Core::add_calendars`: each Calendar is stored empty, and its First Sync is a Full Fetch, as is the sync after Google's 410 "token expired" (ADR 0019). Google gives no total, so 202's progress counts the Events loaded.
- **An Incoming Sync** lists what changed since the Calendar's sync token, reads each event resource through 210's reader, and hands the Event Version Contents to 189's `IncomingSyncPlan` and `IncomingSyncWriter`: Conflicts, Server Event Merges and Mark Reconciliation are the shared code's, no Google code. One sync activity per Calendar (193).
- **A 410** makes the next Incoming Sync a Full Fetch; the plan records only what differs, as one Incoming Sync Snapshot or none.
- **`google_server_object`,** keyed per event resource, holds where it is and its ETag, nothing of its content (ADR 0019, as `caldav_server_object` after 187); the sync token is kept with the Calendar. A resource whose UID and Occurrence a kept resource already holds is ignored and written to the Log, as CalDAV's Ambiguous UID is. Add the table to `redesign-layout.md`'s "Tables (end state)".
- **The Sync Engine:** Google's code in `core/src/sync/google/` joins the Provider `match` of the Sync Engine entry (`core/src/sync/mod.rs`) beside CalDAV's; the core interface names neither.
- **Read-only:** until 213 FreeCal sends nothing to Google, so every Google Calendar is marked read-only when added, and removing the mark is refused without asking the server (200's call, a Provider that can't write yet).
- **Removing** a Calendar or the Account deletes the local copy only (179's removal); a removed Calendar is an Available Calendar again.

Spec: US-6, US-12, US-16, US-152; "Choosing server Calendars"; "Adding server Calendars"; "Sync engine"; "Untrusted content". ADRs 0002, 0018, 0019. Layout: "Interfaces designed twice", "The Incoming Sync"; "By phase", "P5 and P6".

**Blocked by:** 194, 208, 210, 211

**Status:** ready-for-agent

**Effort:** M

- [ ] The user chooses which Available Calendars to add; each is stored at once and its First Sync is a Full Fetch, with progress on its line
- [ ] Incoming Syncs through the Google Calendar REST API (ADR 0002) with sync tokens, through the shared Incoming Sync Plan; each is one Incoming Sync Snapshot or none
- [ ] A 410 "token expired" fetches everything and records only what differs
- [ ] `google_server_object` holds where a resource is and its ETag only
- [ ] Every Google Calendar is marked read-only until 213, and the mark can't be removed
- [ ] Removing the Account removes its Calendars
- [ ] `GLOSSARY.md` follows: Google Server Object (where and ETag, no JSON)
- [ ] Tests through the core interface against the fake Google server, extended with events, sync tokens, ETags, a 410 and permissions; the hostile test Event is pulled, stored and shown without running or rendering anything; the Store Check after each
