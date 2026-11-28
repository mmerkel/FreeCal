# 191: The user resolves a Conflict

**What to build:** Through the core interface, the user resolves an open Conflict (190) in one of three ways (ADR 0019, "Conflicts end at once"): keeping the server's version, restoring their own, or changing the Event again. Each sets the Conflict's `resolved_at`, once, in the Database Transaction that ends it. How it ended isn't stored; the Calendar History shows it. Old ticket 134 (`git show design/snapshot-line:.scratch/freecal-v1/issues/134-user-ends-conflicts.md`) as written, with old 21's restore of a whole Recurring Event's UID.

- **Where:** `history/conflict.rs`.
- **Keep the server's version:** a call naming the Conflict sets `resolved_at` with no Snapshot. Nothing else changes and nothing is sent. "Keep all" for a Calendar resolves each of its open Conflicts in one Database Transaction.
- **Restore my version:** 181's Undo of the Conflict's Incoming Sync Snapshot limited to its UID, which brings back the user's whole UID (the Recurring Event with every Exception) and overwrites the server's changes to it; asked from the Conflict, from 183's dialog ("Restore my version") or, later, from the Calendar History window (207). A whole Undo of that Incoming Sync Snapshot restores and resolves all its Conflicts. It is written as any Undo, so it is sent like any change and counted by the Mass-Change Guard (201). Where the Event changed again since, 181's comparison reports it as left out and asks first.
- **One rule for restoring and changing again:** any Snapshot of the user's own or Undo that changes an Event with an open Conflict (an edit, a move, a delete, an import or Backup restore over it, an Undo reaching it, the restore above) resolves it in the same Database Transaction. The `SnapshotWriter` does it, so no caller can leave it out.
- **Redo** of a restore puts the server's version back and the Conflict stays resolved: `resolved_at` is set once and nothing reopens a Conflict.
- **A stale call:** "keep" on a Conflict already resolved fails without changing anything, with an error of its own; the window reads the list again.
- **Writability** (ticket 131's module): keeping is allowed in a Read-only Calendar, as it changes no Event. A restore is an Undo, and so left out in a Calendar marked read-only (ADR 0020). While the server refuses writes, a restore is made and waits like any change (200).
- **The Store Check** gains: no open Conflict's Event was changed by the user or an Undo since its Incoming Sync Snapshot.

Spec: US-73 to US-77, US-153; "Core application interface"; "Sync engine"; "Calendar History"; "Writability"; "Untrusted content". ADRs 0019 ("Conflicts end at once", "The Conflict tables"), 0020 ("Limited to one Event", "Read-only Calendars"). Layout: "By phase", P3, 191.

**Blocked by:** 190

**Status:** ready-for-agent

**Effort:** M

- [ ] "Keep" for one Conflict and "Keep all" for a Calendar set `resolved_at` without a Snapshot
- [ ] An Undo of the Incoming Sync Snapshot, limited to the UID or whole, restores the User's Version, a Recurring Event's whole UID included, and the restore reaches the server at the next sync
- [ ] Every Snapshot of the user's own or Undo that changes an Event with an open Conflict resolves it in its Database Transaction; a change to another Event doesn't
- [ ] A redo of a restore leaves the Conflict resolved
- [ ] A "keep" on a resolved Conflict fails without changing anything
- [ ] Keeping works in a Read-only Calendar; a restore is left out there
- [ ] The Store Check's new query, with a test that breaks it by hand
- [ ] Tests through the core interface against Radicale, test-first: each kind kept and restored, with the server's state after the next sync; "Keep all"; an edit, a delete and an import over an Event in Conflict each resolve it; a whole Undo of the Incoming Sync Snapshot restores and resolves all its Conflicts and takes back the Events it followed; an Undo limited to another Event of that Snapshot leaves the Conflict open; a restore that asks first because of a later change
