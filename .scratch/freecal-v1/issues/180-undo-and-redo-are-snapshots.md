# 180: Undo and redo are Snapshots in the core

**What to build:** The first part of old ticket 147 (`git show design/snapshot-line:.scratch/freecal-v1/issues/147-undo-and-redo-are-snapshots.md`): an Undo is a Snapshot that takes back one earlier Snapshot (ADR 0020), with the core naming the next Ctrl+Z's Snapshot and the next redo's Undo. New core functions and interface types beside the Change Journal's undo, which stays until 182; the window doesn't call them until 183. Asking first is 181.

- **Where:** `core/src/history/undo.rs`; on `Core` three functions (names for this ticket to settle, e.g. `undo_snapshot`, `ctrl_z`, `redo`) and their types, added to `typescript.rs` beside the old ones.
- **An Undo** is a Snapshot of origin undo whose `undoes` names the Snapshot it takes back (`DEFERRABLE INITIALLY DEFERRED`, an edge of the drop in 185). One function takes back any Snapshot it is given, an Incoming Sync Snapshot (a First Sync's too) included. Its content is 177's `compare_event` per UID the Snapshot changed, with `Base` = the versions the Snapshot began (what it left), `Ours` = the current versions, `Theirs` = the versions it ended, never a zero-length Event Version (ADR 0018, "Zero-length Event Versions"); where nothing changed since, the Event goes back to a copy of the version before. Combinations by Field Group are taken as they come. One Database Transaction for every Calendar the Snapshot spans; until 182 it is written as every write is (through the `JournalTransaction`, so it is sent like any change).
- **Made at once** where nothing is left out. Where the comparison finds a Conflict or an Event is in a Calendar marked read-only or removed, this ticket refuses the whole Undo with that reason; 181 replaces the refusal by the report that asks first. Refused, writing nothing: an Undo that would change nothing, and one of a Snapshot that Undos in effect already take back.
- **Ctrl+Z's target:** the newest of the user's own Snapshots in effect (an action of the user, an import, a Backup restore), across all Calendars, by `made_at`; passed over: Incoming Sync Snapshots and Undos. In effect: not taken back by an Undo in effect.
- **Redo:** the newest Undo in effect that isn't itself an Undo of an Undo; none once the user made a change of their own after it. Syncs don't end it; nothing is dropped when it ends; nothing is written beside the Undo's Snapshot.
- **`UserAction`:** `Moved` becomes `Rescheduled`; `MovedBetweenCalendars` is added for a move to another Calendar (181 needs it).
- **The Store Check** gains: an Undo takes back an older Snapshot.

Spec: US-81, US-146, US-150; "Calendar History". ADRs 0018, 0020 ("An Undo", "Ctrl+Z", "Redo", "Where both changed"). Layout: "New terms" (`UserAction` values, Sides); "Tables (end state)" (`snapshot.undoes`); "By phase", P2, 180.

**Blocked by:** 177, 179

**Status:** ready-for-agent

**Effort:** M

**Switched-off tests** (on again in 181), each `#[ignore = "until ticket 181: undo becomes a Snapshot"]`: in `core/tests/caldav_sync.rs` the undo of a sync (`restores_*`, `of_a_journal_transaction_*`, `of_a_delete_*`, `brings_back_*`, `of_one_event_of_a_sync_*`, `undoing_the_sync_that_brought_*`); in `caldav_send.rs` `undoing_*` and `an_undone_sync_*`; in `caldav_first_sync.rs` the two `comes_back_with_the_undo…`; in `guests.rs` `undoing_a_sync_skips_an_event_with_guests`.

- [ ] An Undo is a Snapshot naming the one it takes back by a deferred foreign key, made by `compare_event` in one Database Transaction for every Calendar the Snapshot spans
- [ ] Any Snapshot can be taken back, Incoming Sync Snapshots included; the Undo of one brings back the user's version, not a zero-length one
- [ ] An Undo combines with a later change to another Field Group; a Conflict or a read-only or removed Calendar refuses it whole, for now; one changing nothing, or of a Snapshot already taken back, is refused
- [ ] The core names the next Ctrl+Z's Snapshot and the next redo's Undo by the rules above; redo ends with the user's next change
- [ ] Tests through the core interface: undo and redo of each kind of change, of a move and of an import into several Calendars, across a restart; Ctrl+Z passing over an Incoming Sync Snapshot and over Undos
- [ ] Rewritten on the new functions: the undo tests of `recurrence_scopes.rs`, `recurring_events.rs`, `import_run.rs` ("no Journal Transaction" becomes "no Snapshot") and the whole-Snapshot scenarios of `undo_per_event.rs` worth keeping
- [ ] `Moved` is `Rescheduled`; `MovedBetweenCalendars` exists and a move records it
- [ ] The tests above switched off with their note; the Store Check passes after each step
