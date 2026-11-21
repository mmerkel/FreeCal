# 166: Writes refused: changes wait as when offline, and the read-only mark is the user's decision

**What to build:** Tickets 56, 120 and 124 made a Calendar that the server doesn't let the user change a Read-only Calendar. Since the grilling of 2026-11-21, only the read-only mark does that. A server that refuses changes is treated like being offline in one direction (ADR 0019, "When the server refuses changes").

- **Editable while writes are refused:** a Calendar that the server reports read-only but that isn't marked stays editable. Its changes wait under the Sync Error "writes refused", and undo, redo and ending Conflicts work as in any writable Calendar. The core's refusal of changes (`CalendarReadOnly`) follows the mark alone, and ticket 131's module with it, if that one is done first.
- **The mark when added:** a Calendar is marked read-only when it is added only if its server doesn't allow changes. The pre-1.0 default goes (US-18 is dropped).
- **Removing the mark asks the server:** the call that removes it first reads the server's privileges (`current-user-privilege-set` for CalDAV), and fails with an error the window shows ("Can't reach the server; try again when “Work” is online") when the server can't be reached or says no. A server that says nothing counts as allowing changes. Local Calendars are exempt.
- **Every Poll reads privileges** while writes are refused and the Calendar isn't marked; once the server allows changes again, the waiting changes are sent. Other Calendars keep the occasions in the spec ("Writability").
- **"Stop waiting and mark read-only"** in the "writes refused" bar (ticket 162's style) shows a confirmation that counts the Unsent Changes. It then makes a Snapshot on the user's side holding the server's state, with the User's Pointer's Snapshot as its only parent and the origin "Stop waiting", moves the Reference Snapshot to the Server's Pointer, which also ends a Conflict, and sets the mark, all in one Database Transaction (ADR 0019). It is one of the user's own Snapshots, so once the mark is removed Ctrl+Z can take it back and bring the waiting changes back (ticket 147's rule). A Calendar that is already marked gets "Stop waiting".
- **Tests,** through the core interface against Radicale with a user whose rights change:
  - a refused write leaves the Calendar editable and the change waiting;
  - an edit made while writes are refused waits too;
  - a Poll that finds the rights back sends the waiting changes;
  - removing the mark fails while the server says no and works once it allows;
  - "Stop waiting" drops the waiting changes, ends a Conflict, and sets the mark;
  - after "Stop waiting" and removing the mark, Ctrl+Z brings the dropped changes back;
  - a Calendar added without rights is marked; one added with rights isn't.

Spec: US-16, US-17, US-19, US-154; "Writability"; "Calendar History". ADR 0019 ("When the server refuses changes"), ADR 0020 ("Read-only Calendars").

**Blocked by:** 147, 152, 161, 162

**Status:** ready-for-agent

**Effort:** M

- [ ] A Calendar whose writes are refused but that isn't marked stays editable, and its changes wait under "writes refused"
- [ ] A Calendar is marked when added only if its server doesn't allow changes; no pre-1.0 default
- [ ] Removing the mark asks the server first and fails with a message when it can't be reached or says no
- [ ] Every Poll reads privileges while writes are refused without the mark, and sends once they are back
- [ ] "Stop waiting and mark read-only" (or "Stop waiting") with a counting confirmation, in one Database Transaction
- [ ] "Stop waiting" is a Snapshot with an origin of its own, and Ctrl+Z takes it back once the mark is gone
- [ ] The tests listed above

## Comments

- 2026-11-21: Written from the grilling on read-only Calendars. It supersedes, for the server's read-only, what tickets 56 and 120 built: a Calendar the server reports read-only no longer stays read-only whatever the user's mark.
- 2026-11-21: Reworded from the grilling on critique 1 (`.scratch/freecal-v1/critiques.md`; ADR 0020 rewritten: undo makes an Undo Snapshot, both pointers only move forwards, Held during Undo goes). "Stop waiting" is a Snapshot, not a move of the User's Pointer; marking no longer ends a redo path. No longer blocked by 165 (wontfix); blocked by 147 for the Undo.
