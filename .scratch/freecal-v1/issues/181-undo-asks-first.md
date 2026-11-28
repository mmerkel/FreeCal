# 181: An Undo asks first about what it leaves out

**What to build:** The second part of old ticket 147 (`git show design/snapshot-line:.scratch/freecal-v1/issues/147-undo-and-redo-are-snapshots.md`): an Undo that can't take back every Event reports what it would leave out and why, writing nothing, and the caller then asks for the rest or for all over the later changes (ADR 0020, "Asking first"). Also an Undo limited to one UID, and moves with one side that can't be written. The core interface reports; the window shows it in plain words in 183.

- **Left out** (`LeftOut { key, reason }`, replacing 180's refusal): an Event in a Conflict the comparison can't combine, with its Occurrences and, for each, the first Snapshot after the taken-back one that changed it (its origin and, for the user's own, what the user did), so the window can name it and, for an Incoming Sync Snapshot that replaced the Event, offer its restore (191); an Event in a Calendar marked read-only or removed, naming the Calendar and which. Then the caller asks for the Undo of the rest, or "… all over the later changes" (`OnConflict::Win(Theirs)`): the Undo records that it was made over a later change. Where nothing is left out, it is made at once, as in 180.
- **Limited to one UID** (ADR 0020, "Limited to one Event"): `snapshot.undo_uid`, `CHECK (undo_uid IS NULL OR origin = 'undo')`; the Undo takes back that UID's Events in every Calendar the Snapshot spans and otherwise behaves as any Undo. Refused where the Snapshot didn't change that UID. A Snapshot partly taken back stays in effect for Ctrl+Z, which takes back the rest. Ctrl+Z is never limited. The Store Check gains: a limited Undo's UID was changed by the Snapshot it takes back.
- **A move with one side that can't be written** (ADR 0020, the section of that name): a Snapshot of `MovedBetweenCalendars`, or an Undo of one, with one side read-only or removed is reported as a move with that side left out, with what taking back the other side does (restore the Event in the origin, or delete it from the target); never as a create or a delete, also when the UID has Event Versions in one Calendar only because the other was removed.
- **Ctrl+Z and redo** pass over a Snapshot, or an Undo, of which every Event still in effect is in a Calendar marked read-only or removed, checked at each call.
- **Events with guests** are undone and redone like any Event (ADR 0020, "Events with guests").

Spec: US-81, US-146, US-150; "Calendar History". ADR 0020 ("Asking first", "Limited to one Event", "A move with one side that can't be written", "Read-only Calendars", "Events with guests"). Layout: "Tables (end state)" (`snapshot.undo_uid`); "By phase", P2, 181.

**Blocked by:** 180

**Status:** ready-for-agent

**Effort:** M

**Switched-off tests:** turns on and rewrites, to the new Undo, every test 180 switched off with "until ticket 181" (the undo of a sync in `caldav_sync.rs`, `caldav_send.rs`, `caldav_first_sync.rs`, `guests.rs`); the guests test now asserts that the Event with guests is taken back.

- [ ] What an Undo leaves out is reported with why before anything is written; the rest, or everything over the later changes, is taken back on request, and the Undo records which
- [ ] An Undo limited to one UID takes back that UID's Events in every Calendar the Snapshot spans; refused for a UID the Snapshot didn't change; Ctrl+Z then takes back the rest
- [ ] A move into a read-only Calendar is restored in its origin only; a move whose origin, and one whose target, was removed is reported as a move
- [ ] Ctrl+Z passes over a Snapshot only in a read-only Calendar, and reaches it again after the mark is removed
- [ ] Tests through the core interface: an Undo leaving out an Event changed later by the user, then the rest, then over the later changes; an Undo limited to one Event of a move and of an import
- [ ] The tests 180 switched off are on again, against Radicale where they were; an undo and a redo in a server Calendar reach the server
