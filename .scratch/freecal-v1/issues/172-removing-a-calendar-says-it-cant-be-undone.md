# 172: Removing a Calendar says it can't be undone

**What to build:** The dialogs that remove a Calendar, a Local Calendar's "Delete" in the sidebar (`src/lib/Sidebar.svelte`) and "Remove" in a server Account's "Manage Calendars…", and the one that removes an Account, say that it can't be undone and what is lost with it (ADR 0018, "Removing a Calendar"). The core's part, deleting the Calendar's Event Versions, Conflicts and so its Unsent Changes, is tickets 146, 133 and 151's.

- **Local Calendar:** the dialog names how many Events go and the date of the last Backup that holds the Calendar (ticket 11): “Delete “Home” and its 212 Events? This can't be undone. The last Backup is from 27 Nov.” Without a Backup: “There is no Backup of it.”
- **Server Calendar:** “Remove “Work” from FreeCal? Its Events stay on the server. This can't be undone, but you can add it again.” With Unsent Changes: “3 changes not yet sent to the server will be lost.”
- **Account:** the same, for its Calendars together, with the Unsent Changes counted over all of them.
- **What the core reports:** for a Calendar, its number of Events, its number of Unsent Changes (read from the marks, ticket 153) and the date of its last Backup.

Spec: US-3, US-12, US-15; "Accounts and Calendars", "Sidebar". ADR 0018.

**Blocked by:** 11, 146, 153

**Status:** ready-for-agent

**Effort:** S

- [ ] The core reports a Calendar's number of Events, Unsent Changes and last Backup's date
- [ ] Deleting a Local Calendar, removing a server Calendar and removing an Account each say that it can't be undone and what is lost
- [ ] Component tests on the fake core for each dialog; core tests for the counts

## Comments
- 2026-11-28: Written from the grilling on point 5 of `.scratch/freecal-v1/critiques-2.md` (ADR 0018): removing a Calendar deletes its part of the Calendar History and can't be undone, so its dialog says so.
