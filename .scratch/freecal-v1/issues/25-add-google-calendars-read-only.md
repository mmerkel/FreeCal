# 25: Add a Google Account's Calendars and pull them, read-only

**What to build:** The user chooses which of a Google Account's Available Calendars to add (ticket 164 lists them) and sees them in FreeCal, kept up to date by Incoming Syncs. "Manage Calendars…" on the Account line works as for CalDAV. Third of three parts: ticket 163 is Google's reader, ticket 164 the Account.

- **Adding Calendars** runs their First Syncs through ticket 155's operation and its "First Sync progress" Signal, shown by ticket 156's dialog. A Calendar enters the Local Store together with its first Snapshot (ADR 0018). Google gives no total, so the progress counts the Events loaded.
- **An Incoming Sync** lists what changed since the Calendar's sync token, reads each event resource through ticket 163's reader, and hands the result to the operation that records what the server now holds (ticket 151): a Snapshot on the server's side, which the user's side follows. Nothing here moves a pointer itself.
- **A 410 "token expired"** makes the next Incoming Sync list everything. It is compared with what is stored and records only what differs, as one Snapshot like any other Incoming Sync, or none when nothing differs.
- **Google's Server Object table** holds, for each event resource, where it is and its ETag, and nothing of its content (ADR 0019, as for CalDAV); the sync token is kept with the Calendar. A resource whose UID and Occurrence a kept resource already holds is ignored and written to the Log, as CalDAV's Ambiguous UID is (chosen here, for review).
- **The Sync Engine:** Google's code goes in `core/src/sync/google/` and joins the Provider `match` of the Sync Engine entry (`core/src/sync/mod.rs`, ticket 115) beside CalDAV's; the core interface names neither. One sync activity per Calendar, as ticket 154 has it.
- **Read-only:** the server's permissions decide whether a Calendar is read-only; before 1.0 a newly added Google Calendar is marked read-only. Until ticket 26 FreeCal sends nothing to Google, so a Google Calendar can't be made writable.
- **Removing** a Calendar or the Account deletes the local copy only; a removed Calendar is an Available Calendar again.
- **Tests** against the fake Google server of ticket 164, extended with events, sync tokens, ETags, a 410 and permissions, through the core interface.

Spec: US-6, US-12, US-16, US-18, US-152; "Choosing server Calendars"; "Adding server Calendars"; "Sync engine"; "Untrusted content". ADRs 0002, 0018, 0019.

**Blocked by:** 154, 155, 156, 163, 164

**Status:** ready-for-agent

**Effort:** M

- [ ] The user chooses which Available Calendars to add; each First Sync runs through ticket 155's operation and shows in ticket 156's dialog
- [ ] Incoming Syncs through the Google Calendar REST API (ADR 0002) using sync tokens; each is one Snapshot through ticket 151's operation
- [ ] A 410 "token expired" lists everything and records only what differs
- [ ] Google's Server Object table holds where a resource is and its ETag only
- [ ] Server permissions decide read-only; before 1.0 newly added Google Calendars default to read-only, and none can be made writable until ticket 26
- [ ] Removing the Account removes its Calendars
- [ ] The fake Google server serving the hostile test Event shows it is pulled, stored and shown without running or rendering anything

## Comments
- 2026-10-18: Change Journal decisions from grilling tickets 58 and 59 (ADR 0009).
- 2026-10-24: Blocked by 87: Google events are stored as server objects that mirror the server (ADR 0010), each resource, a Recurring Event or an Exception, an object of its own with its JSON as received.
- 2026-11-07: Blocked by 115: Google's Sync Engine joins the Provider `match` of the Sync Engine entry (`core/src/sync/mod.rs`) beside CalDAV's; the core interface names neither.
- 2026-11-16: Reworded and split from the triage against ADRs 0017 to 0020. Re-sized from L to XL, then split under the M cap: the reader is ticket 163, the login and the Available Calendars ticket 164, and this ticket keeps its number for the last part, so tickets 26 and 27 stay blocked by it. The First Sync goes through tickets 155 and 156, an Incoming Sync is a Snapshot, and the first sync is recorded as the Calendar's first Snapshot. The comments above are outdated where they name the Change Journal and server objects holding JSON as received. Blocked by 154 and 155 as agreed; also by 156 (the dialog), 163 and 164. Its former blocker 15 is dropped: what this ticket needs of the Sync Status is built, and 15 now waits on the Conflict tickets.
