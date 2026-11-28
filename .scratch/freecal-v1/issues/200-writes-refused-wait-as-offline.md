# 200: Writes refused: changes wait as when offline, and the read-only mark is the user's decision

**What to build:** A server that refuses changes is treated like being offline in one direction (ADR 0019, "When the server refuses changes"); only the user's read-only mark makes a Read-only Calendar. The core part of old ticket 166 (`git show design/snapshot-line:.scratch/freecal-v1/issues/166-writes-refused-waits-like-offline.md`); the window's bar, confirmation and error message are 203's.

- **Editable while writes are refused:** a Calendar the server reports read-only but that isn't marked stays editable. A refused write leaves what differs as it is (192's closing Database Transaction records nothing for it) and gives the Calendar the Sync Error "writes refused", a kind of its own in `sync_status.rs`. Undo, redo and resolving Conflicts work as in any writable Calendar. The core's refusal of changes (`CalendarReadOnly`, ticket 131's module `read_only.rs`) follows the mark alone: `server_read_only` no longer counts there.
- **Every Poll reads privileges** (`server_read_only`, `remote.rs`) while writes are refused and the Calendar isn't marked; once the server allows changes again, the waiting changes are sent and the Sync Error goes. Other Calendars keep the spec's occasions ("Writability").
- **The mark when added:** set only if the server doesn't allow changes (US-18 is dropped; remove any pre-1.0 default left in the code).
- **Removing the mark asks the server:** the call first reads the server's privileges (`current-user-privilege-set` for CalDAV) and fails with an error of its own when the server can't be reached or says no; a server that says nothing counts as allowing changes. Local Calendars are exempt.
- **"Stop waiting":** a call for one Calendar, with or without setting the mark: one Snapshot of a new origin, "stop waiting" (`snapshot.origin`), that puts the versions with the server's mark in place of the user's for every UID that differs, and sets the mark if asked, in one Database Transaction. It is one of the user's own Snapshots, so once the mark is removed Ctrl+Z takes it back and brings the waiting changes back (180). The core also gives the number of Events waiting in a Calendar, for 203's confirmation.
- **Types:** the new Sync Error kind, the calls and the error are interface types; regenerate the TypeScript types. The window's part stays what the type change forces (a plain fallback text); 203 writes the bar and the words.

Spec: US-16, US-17, US-19, US-154; "Writability"; "Calendar History". ADRs 0019 ("When the server refuses changes"), 0020 ("Read-only Calendars"). Layout: "Tables (end state)" (`snapshot.origin`, `calendar`'s "writes refused" state); "Seams for `/tdd`", 3; "By phase", P4, 200.

**Blocked by:** 180, 192

**Status:** ready-for-agent

**Effort:** M

**Rewritten:** `core/tests/caldav_sync.rs`'s read-only and permission group; `caldav_send.rs`'s `a_write_the_server_refuses_is_a_sync_error_of_the_calendar`; `caldav.rs`'s `a_write_refused_for_its_content_is_no_refusal_of_writes`. The privilege read once when adding survives.

- [ ] A Calendar whose writes are refused but that isn't marked stays editable, and its changes wait under "writes refused"
- [ ] A Calendar is marked when added only if its server doesn't allow changes
- [ ] Removing the mark asks the server first and fails with an error when it can't be reached or says no; a Local Calendar's mark is removed without asking
- [ ] Every Poll reads privileges while writes are refused without the mark, and sends once they are back
- [ ] "Stop waiting", with or without the mark, is one Snapshot of origin "stop waiting" in one Database Transaction; Ctrl+Z takes it back once the mark is gone
- [ ] The core counts a Calendar's waiting Events
- [ ] Tests through the core interface against Radicale with a user whose rights change, test-first: a refused write leaves the Calendar editable and the change waiting; an edit made while writes are refused waits too; a Poll that finds the rights back sends the waiting changes; removing the mark fails while the server says no and works once it allows; "Stop waiting" drops the waiting changes and sets the mark; after it and removing the mark, Ctrl+Z brings the dropped changes back; a Calendar added without rights is marked, one with rights isn't
- [ ] The tests above rewritten; the Store Check passes after every scenario
