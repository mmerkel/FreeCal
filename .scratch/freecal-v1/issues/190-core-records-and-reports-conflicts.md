# 190: The core records and reports Conflicts

**What to build:** Since 189 an Incoming Sync replaces each UID in Conflict with the server's version, a Server Event Replacement; nothing records or shows that yet. This ticket adds the `conflict` and `conflict_event` tables, written by the Incoming Sync, and makes the core report the open Conflicts: a list, their number in the Sync Status, the Signal "Conflicts changed" and the overall status (ADR 0019, "The Conflict tables", "Conflicts end at once"). Old ticket 133 (`git show design/snapshot-line:.scratch/freecal-v1/issues/133-core-reports-conflicts.md`) as written, with old 21's whole-UID Conflict and old 15's last box. Resolving is 191, what each side changed 199, the window's Conflict views 204 to 206.

- **`conflict`:** the Incoming Sync Snapshot (`ON DELETE CASCADE`), the Calendar, the UID (unique with the Snapshot and the Calendar), and `resolved_at`, empty while open. An insert trigger refuses a row whose Snapshot isn't an Incoming Sync Snapshot; an update trigger allows only setting `resolved_at`, once, from empty. How a Conflict ended isn't stored: the Calendar History shows it.
- **`conflict_event`:** one Conflict Event per single Event of the UID (the Recurring Event and each Exception) found on either side or in the Last Agreed Version (from old 21): the Conflict (`ON DELETE CASCADE`), the recurrence ID (unique per Conflict), and the Last Agreed Version, the Event Version with the server's mark before the fetch (`DEFERRABLE INITIALLY DEFERRED`), empty where there was none; 195 adds the untrusted base, which leaves it empty too. The User's Version and the Server Version aren't stored: they are the Event Versions of that Calendar and UID that end and that begin with the Incoming Sync Snapshot.
- **Written only by the `IncomingSyncWriter`,** in the Incoming Sync Plan's Conflict branch, in its Database Transaction. This is the one invariant no query can check afterwards (ADR 0019): the tests reach every path to a Conflict.
- **Kept** (185's rule): the root "the Incoming Sync Snapshot of an open Conflict", which keeps the User's Version; the edge from it, through its Conflict Events, to the Snapshots with which their Last Agreed Versions ended; a trigger refusing to delete an open Conflict, except in the removal of its Calendar, which deletes its Conflicts (ADR 0018, "Removing a Calendar"). A resolved Conflict stays as long as its Incoming Sync Snapshot.
- **The list,** through the core interface, for all Calendars or one, newest first: the Conflict's ID (191's calls take it); the Calendar, the UID and when the Incoming Sync was; the User's Version and the Server Version as the managed fields of their Events, each named by its Combined Event Key, either possibly empty; its kind, derived by one function and never stored: an edit on both sides, the user's edit against the server's delete, the user's delete against the server's edit.
- **The Sync Status:** the number of open Conflicts of a Calendar is a field of "Sync status changed" (added here; 202 builds on it), and the Account's report follows. The overall status is Conflict while any Calendar has an open one (`SyncStatusTracker::overall`, `sync_status.rs`); tick old ticket 15's last box. The window's sync display (`src/lib/syncDisplay`) shows that status; nothing else in the window.
- **"Conflicts changed"** (spec: "Signals") is sent for a Calendar after every Database Transaction that writes, resolves or drops one of its Conflicts; that Calendar is also sent "Occurrences changed" (for 204's badge), and "Sync status changed" when its number differs.
- **An Event's History** (184): the version a Server Event Replacement brought reads "replaced by the server's version" while its Conflict is open, and the replaced version before it stays in the list (moved here from old 160, as it needs the Conflict).
- **The Store Check** gains: every Conflict has its Conflict Events; each Last Agreed Version has the Conflict's Calendar, UID and recurrence ID.
- **Types:** the list's types are interface types; regenerate the TypeScript types and give the fake core its answer. Every field of a version is a stored field and untrusted.

Spec: US-33, US-71, US-76, US-77, US-148; "Core application interface"; "Signals"; the overall status under "Sync engine"; "Untrusted content". ADRs 0018 ("Keeping and dropping", "Removing a Calendar"), 0019 ("The Conflict tables", "Conflicts end at once"). Layout: "Tables (end state)" (`conflict`, `conflict_event`); "By phase", P3, 190.

**Blocked by:** 189

**Status:** ready-for-agent

**Effort:** M

- [ ] The `conflict` and `conflict_event` tables, cascading with the Incoming Sync Snapshot, with the insert trigger and `resolved_at` set once
- [ ] The Incoming Sync writes a Conflict with a Conflict Event per single Event of the UID and their Last Agreed Versions for each UID it replaces, in its Database Transaction, and nowhere else
- [ ] An open Conflict older than 90 days keeps its Incoming Sync Snapshot, the User's Version and its Last Agreed Versions; deleting an open Conflict is refused, except in its Calendar's removal
- [ ] The core lists the open Conflicts of all Calendars or of one, each with its User's Version, Server Version and kind
- [ ] "Sync status changed" carries a Calendar's number of open Conflicts; the overall status is Conflict while any Calendar has one, in the core and the sync display (old ticket 15's last box ticked)
- [ ] "Conflicts changed" and "Occurrences changed" are sent when a Conflict arises, is resolved or is dropped
- [ ] An Event's History reads "replaced by the server's version" for an open Conflict's Server Version
- [ ] The Store Check's new queries, each with a test that breaks it by hand
- [ ] Tests through the core interface against Radicale, test-first: each kind listed with its two versions and Conflict Events; a UID both sides created differently (no Last Agreed Version); a Recurring Event with Exceptions on either side and in the Last Agreed Version; the list for one Calendar and for all; a `resolved_at` set by hand takes it off the list, and setting it twice is refused; a `conflict` row at a Snapshot that isn't an Incoming Sync Snapshot is refused; a second change on the server afterwards leaves the listed Server Version as it was; the number, the overall status and the Signals for each
