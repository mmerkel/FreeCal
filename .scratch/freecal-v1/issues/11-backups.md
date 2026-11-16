# 11: Backups

**What to build:** FreeCal takes a compressed Backup of each changed Calendar once a day, keeps the last 14, skips Calendars unchanged since their last Backup, and lets the user restore a Calendar from a Backup.

- **What a Backup holds:** the Calendar's Events as the user has them (the Event Versions at its User's Pointer), with their managed fields and Unimplemented Fields, as one compressed `.ics` written by ticket 144's writer, as the export's is (ticket 09). A Backup without the Unimplemented Fields would strip them from the server when it is restored.
- **Changed since the last Backup** (chosen here, for review): the Calendar's User's Pointer isn't on the Snapshot it was on when the last Backup was taken. FreeCal keeps that Snapshot's ID with the Backup and only compares it, so it doesn't matter that the Snapshot is dropped after 90 days.
- **A restore is one Snapshot** on the user's side, whose recorded origin is the restore of a Backup (ADR 0018; the origin exists since ticket 146). After it the Calendar holds the Backup's Events and no others. An Event that reads the same as in the Backup keeps its Event Version, so only what differs is a change.
- **Reading the Backup** goes through the reader an import uses (tickets 143 and 149), and every field is untrusted. Entries with a prefix the Calendar's Provider doesn't hold are dropped, as for an import (ADR 0017).
- **Before it runs,** a confirmation names how many Events the restore adds, changes and removes. It runs under the modal "Working…" dialog and sends "Occurrences changed".
- **In a server Calendar** the difference is then sent like any other change (ticket 153). The Mass-Change Guard doesn't count it while the server's side hasn't reached the restore's Snapshot, as the confirmation was the approval (ticket 22). Whichever of this ticket and 22 is built second adds the restore's origin to the ones the guard reads, with a test.
- **A Calendar with a Conflict** can be restored. The two sides are then compared again (ticket 138's rule; until 138 is built, at the next Incoming Sync).
- **Undone like any change:** Ctrl+Z takes the restore back, and the Calendar's History lists it as a restore (ticket 159).
- **Read-only Calendars** (chosen here, for review): a restore isn't offered for a Read-only Calendar, as an import isn't.

Spec: US-88, US-89, US-90; "Backups"; "Calendar History"; "Untrusted content". ADRs 0017, 0018, 0019 (the Mass-Change Guard).

**Blocked by:** 03, 144, 147, 149

**Status:** ready-for-agent

**Effort:** M

- [ ] A daily compressed `.ics` Backup per changed Calendar in the data directory, including Local Calendars, with the Unimplemented Fields (tested with the controllable clock)
- [ ] Unchanged Calendars are skipped
- [ ] Only the last 14 Backups per Calendar are kept
- [ ] The user can list Backups and restore a Calendar from one, after a confirmation with the numbers, under the modal "Working…" dialog; restore sends "Occurrences changed"
- [ ] A restore is one Snapshot with the restore as its origin; Ctrl+Z takes it back; an Event the Backup holds unchanged keeps its Event Version
- [ ] A Calendar with a Conflict can be restored
- [ ] The hostile test Event survives a Backup and its restore unchanged, with no property injected

## Comments
- 2026-11-16: Reworded from the triage against ADRs 0017 to 0020: a restore is one Snapshot, and a Calendar with a Conflict can be restored. Blocked by 147 as agreed in the triage. Added while rewording, for review: blocked by 144 and 149 too, since a Backup is written by the writer and must hold the Unimplemented Fields; "changed" is read from the User's Pointer; no restore into a Read-only Calendar; the confirmation with its numbers, which ticket 22 names as the approval.
