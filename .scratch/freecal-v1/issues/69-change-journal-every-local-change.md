# 69: Change Journal for every local change

**What to build:** Every change FreeCal makes to an Event is a Change Journal entry, in any Calendar, Local Calendars included, and every change's unit can be undone and redone whole. One core module, the Change Journal module, is the only writer of Events, so no change can skip the Journal (ADR 0009). This replaces ticket 58's import-only Journal; the undo rules that go per Event are ticket 72. Spec: US-81, US-82 and US-99; "Change Journal" under Implementation Decisions.

**Blocked by:** none

**Status:** done

**Effort:** M

- [x] A new `core/src/journal.rs` is the only code that writes the `event` table and the Journal tables; the Event-write SQL moves there from `store.rs` and its four copies of the Event row become one. `Store` keeps the reads and the Calendar and settings writes
- [x] Its interface is one closure over a unit, `journal::record(&store, origin, now, |unit| …)`, returning `Recorded<T> { unit: Option<UnitId>, calendars, value }`: inside one transaction the unit offers reads and `create`, `replace` and `delete`, reads each previous version itself and records it with a hash of the new one
- [x] `journal_import` / `journal_entry` become general units and entries (migrations may change freely before the first release, ADR 0008). Each unit records its origin: user action, import or undo in this ticket; sync, Snapshot restore and Conflict resolution come with their tickets
- [x] The module decides writability from the origin, so no caller calls `writable()`: a user action or an import touching a Read-only Calendar is refused with `CalendarReadOnly` (an edit moving an Event checks both Calendars), and an undo too until ticket 72 skips instead. Incremental syncs will write into Read-only Calendars (ticket 54)
- [x] The hash is SHA-256 (`sha2`) over a length-prefixed list of the Event's Calendar, UID and fields, starting with a format version byte; a comment says how a migration re-hashes when a field such as the original server data (ADR 0004) is added
- [x] A unit with no changes (an import that skips every Event, an edit or reschedule that changes nothing, found by comparing hashes) is not stored, returns no unit and sends no Signal
- [x] `create_event`, `edit_event`, `reschedule_event` and `delete_event` return `Changed<T> { unit: Option<UnitId>, value }`; `ImportOutcome` carries its `UnitId`. `Core` sends "Occurrences changed" for `calendars` after releasing the Store lock
- [x] `Core::undo_unit(UnitId)` replaces `undo_import`: it writes every previous version of the unit back under the same identity when no Event changed since its entry, and otherwise refuses with a new `ChangedSinceUnit`. No previous version is in a deleted Calendar, because deleting a Calendar removes those entries (next point). The undo is recorded as a unit of origin undo and the original's entries are marked undone; undoing an undone unit is refused with `AlreadyUndone`. Redo is `undo_unit` on the undo's unit
- [x] Deleting a Local Calendar isn't recorded (ADR 0009): the module deletes its Events, removes their entries by Event identity and drops units left empty. It also removes each Event's entries up to and including its last entry whose previous version is in that Calendar, such as an Event moved out of it or deleted from it: undoing one would put the Event back into the deleted Calendar, and the earlier ones are undone only through it. The Event's later entries stay
- [x] Frontend: `CoreApi` gets `Changed<T>` and `undoUnit` in place of `undoImport`; the import toast offers Undo exactly when a unit came back, replacing the `imported + updated > 0` rule in `App.svelte`, `fakeCore.ts` and `lib.rs`; `errorText` words `changedSinceUnit` and `alreadyUndone` through `t()`. `fakeCore.ts` records a unit for every change and does the simple undo. Toasts for other changes stay ticket 70's
- [x] Core tests through `Core` and the `Harness` only, plus one unit test pinning the hash format: each of create, edit, reschedule, delete and import returns a unit that `undo_unit` reverses exactly; an edit moving an Event between Calendars, undone; no unit and no Signal for a change that changes nothing; undo, redo and `AlreadyUndone`; refusals in Read-only Calendars per origin; ticket 58's "moved since" and "edited since" cases refused with `ChangedSinceUnit`; deleting a Calendar makes undoing an import into it `UnitNotFound`; undoing a move out of a Calendar deleted since, a delete from it, or the Event's earlier entries is `UnitNotFound`, while its later entries still undo; the Signals each undo sends

Out of scope: undo per Event, cascading through later changes, skipping, benign and dangerous undos, the 90-day removal (ticket 72); recording incremental syncs (tickets 54 and 25); server writes of an undo (ticket 17); Ctrl+Z, toasts and confirmations (ticket 70); the list and History (ticket 19).

## Comments

- 2026-10-18: From grilling ticket 58's decisions (TODO.md). ADR 0009 records why.
- 2026-10-18: Split after an architecture review (deepening the Event write path into one Change Journal module) and its grilling. The undo rules moved to ticket 72; this ticket builds the module, records every change and undoes whole units with simple rules.
- 2026-10-18: Done. `journal::record` runs a closure over a `Unit` (`create`, `replace`, `delete`, plus `find_event`, `find_event_by_uid` and `writable`); `journal::undo` and `journal::delete_calendar` are the module's other two entry points. Decisions:
  - The Journal tables are replaced by a new migration rather than by editing ticket 58's, so development Local Stores upgrade instead of keeping a schema the code no longer describes. `journal_unit` has `origin` and `made_at`; `journal_entry` holds the previous version with its Calendar (none when created), the new version's hash (none when deleted) and `undone`.
  - Deleting a Calendar moved from `Store` to the module, since it deletes Events and their entries in one transaction.
  - 2026-10-18 (after review): deleting a Calendar also removes the entries that would put an Event back into it, with that Event's earlier entries, since none of them could be undone any more. They first stayed and were refused with `CalendarNotFound`. History (ticket 19) loses "moved from" lines for such Events. Ticket 72 no longer needs to skip Events whose Calendar was deleted.
  - An import checks its Calendar's writability before reading any Event, so an import that would skip every Event is still refused in a Read-only Calendar. Every other check is made by the write itself.
  - An Event put back after its ID went to another Event gets a new ID, as in ticket 58.
  - An import "update" that leaves an Event as it was still counts as updated, but records nothing.
