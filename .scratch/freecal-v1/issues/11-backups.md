# 11: Backups

**What to build:** FreeCal takes a compressed Backup of each changed Calendar once a day, keeps the last 14, skips Calendars unchanged since their last Backup, and lets the user restore Calendars from their Backups, each into a Target Calendar.

- **What a Backup holds:** the Calendar's Events as the user has them (its current Event Versions), with their managed fields and Unimplemented Fields, as one compressed `.ics` written by ticket 144's writer, as the export's is (ticket 09). A Backup without the Unimplemented Fields would strip them from the server when it is restored.
- **Changed since the last Backup** (agreed 2026-11-22): an Event Version of the Calendar began or ended with a Snapshot newer than the newest Snapshot when the last Backup was taken. FreeCal keeps that Snapshot's ID with the Backup and only compares it, so it doesn't matter that the Snapshot is dropped after 90 days.
- **Choosing what to restore:** the user picks a date, and the dialog lists, for each Calendar with a Backup, its newest Backup from that date or before, Calendars that are gone included. Each has a Target Calendar select over the writable Calendars, in ticket 71's list: by default its own Calendar while it exists, otherwise the writable Calendar with the same name, otherwise none. Opened from a Calendar's menu, only that Calendar's Backup has a Target Calendar at first; from the app menu, every Calendar that still exists has its own. A Calendar can be the Target Calendar of one Backup only: the select disables Calendars another line uses.
- **A restore never creates a Calendar** (ADR 0018, decided 2026-11-21). For a Backup whose Calendar is gone, the user cancels, creates a Calendar and restores again.
- **A restore is one Snapshot** on the user's side spanning its Target Calendars, whose recorded origin is the restore of a Backup (ADR 0018; the origin exists since ticket 146). After it each Target Calendar holds its Backup's Events and no others. An Event that reads the same as in the Backup keeps its Event Version, so only what differs is a change.
- **Reading the Backup** goes through the reader an import uses (tickets 143 and 149), and every field is untrusted. Entries with a prefix the Target Calendar's Provider doesn't hold are dropped, as for an import (ADR 0017).
- **Before it runs,** a confirmation names, for each Target Calendar, how many Events the restore adds, changes and removes. It runs under the modal "Working…" dialog and sends "Occurrences changed".
- **In a server Calendar** the difference is then sent like any other change (ticket 153). The Mass-Change Guard doesn't count it while the server's side hasn't reached the restore's Snapshot, as the confirmation was the approval (ticket 22). Whichever of this ticket and 22 is built second adds the restore's origin to the ones the guard reads, with a test.
- **A Calendar with a Conflict** can be restored. The two sides are then compared again (ticket 138's rule; until 138 is built, at the next Incoming Sync).
- **Undone like any change:** Ctrl+Z takes the restore back whole, and the Calendar's History lists it as a restore (ticket 159).
- **Read-only Calendars** can't be Target Calendars, as for an import.

Spec: US-88, US-89, US-90; "Backups"; "Calendar History"; "Untrusted content". ADRs 0017, 0018, 0019 (the Mass-Change Guard).

**Blocked by:** 03, 71, 144, 147, 149

**Status:** ready-for-agent

**Effort:** M

- [ ] A daily compressed `.ics` Backup per changed Calendar in the data directory, including Local Calendars, with the Unimplemented Fields (tested with the controllable clock)
- [ ] Unchanged Calendars are skipped
- [ ] Only the last 14 Backups per Calendar are kept
- [ ] The restore dialog lists, for a chosen date, each Calendar's newest Backup from then or before, gone Calendars included, with Target Calendars proposed by Calendar, then by name, and only the menu's Calendar preselected when opened from a Calendar's menu
- [ ] A Calendar can't be the Target Calendar of two Backups; Read-only Calendars aren't offered; no Calendar is created
- [ ] The restore runs after a confirmation with the numbers per Target Calendar, under the modal "Working…" dialog, and sends "Occurrences changed"
- [ ] A restore is one Snapshot spanning its Target Calendars with the restore as its origin; Ctrl+Z takes it back whole; an Event the Backup holds unchanged keeps its Event Version
- [ ] A Calendar with a Conflict can be restored
- [ ] The hostile test Event survives a Backup and its restore unchanged, with no property injected

## Comments
- 2026-11-16: Reworded from the triage against ADRs 0017 to 0020: a restore is one Snapshot, and a Calendar with a Conflict can be restored. Blocked by 147 as agreed in the triage. Added while rewording, for review: blocked by 144 and 149 too, since a Backup is written by the writer and must hold the Unimplemented Fields; "changed" is read from the User's Pointer; no restore into a Read-only Calendar; the confirmation with its numbers, which ticket 22 names as the approval.
- 2026-11-21: Decided in a grilling: a restore chooses Backups by date, each with a Target Calendar (its own Calendar, else one of the same name, else none; one Backup per Calendar), and never creates a Calendar, so a Backup of a Calendar that is gone is restored into one the user creates first. Several Calendars restore as one Snapshot. Blocked by 71 too, for the list with Target Calendar selects. Effort left at M, for review: the multi-Calendar dialog adds to it.
- 2026-11-21: Decided in a grilling: a restore replaces Events with guests like any other, although the user can't edit them, until the attendee features (ticket 126). Its confirmation counts them with the rest.
- 2026-11-22: Reworded from the grilling on critique 2 (ADRs 0018 to 0020 rewritten): no User's Pointer; "changed since" compares Snapshot IDs on the one line.
- 2026-11-22: Agreed by the user: "changed since the last Backup" compares Snapshot IDs on the one line.
