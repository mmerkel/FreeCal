# 209: Removing a Calendar says it can't be undone

**What to build:** The dialogs that remove a Calendar (a Local Calendar's "Delete" in `src/lib/Sidebar.svelte`, "Remove" in a server Account's "Manage Calendars…") and the one that removes an Account say that it can't be undone and what is lost (ADR 0018, "Removing a Calendar"). Old ticket 172 (`git show design/snapshot-line:.scratch/freecal-v1/issues/172-removing-a-calendar-says-it-cant-be-undone.md`) as written; the core's removal itself is 179's, 190's (its Conflicts) and 192's (what differs goes with its Event Versions).

- **The core reports,** for a Calendar: its number of Events, its number of Events not yet sent (those whose current version differs from the one with the server's mark, as 192 derives them) and the date of the last Backup holding it (ticket 11). A reader in `history/`; interface types; regenerate the TypeScript types and give the fake core its answer.
- **Local Calendar:** “Delete “Home” and its 212 Events? This can't be undone. The last Backup is from 27 Nov.” Without one: “There is no Backup of it.”
- **Server Calendar:** “Remove “Work” from FreeCal? Its Events stay on the server. This can't be undone, but you can add it again.” With unsent Events: “3 changes not yet sent to the server will be lost.”
- **Account:** the same for its Calendars together, the unsent Events counted over all of them.
- Calendar names are untrusted text; all text in `src/i18n/en.ts`.

Spec: US-3, US-12, US-15; "Accounts and Calendars"; "Sidebar". ADR 0018 ("Removing a Calendar"). Layout: "By phase", "P5 and P6" (counts for 209).

**Blocked by:** 11, 179, 192

**Status:** ready-for-agent

**Effort:** S

- [ ] The core reports a Calendar's number of Events, of unsent Events and its last Backup's date (core tests through the interface)
- [ ] Deleting a Local Calendar, removing a server Calendar and removing an Account each say that it can't be undone and what is lost
- [ ] Component tests on the fake core for each dialog
