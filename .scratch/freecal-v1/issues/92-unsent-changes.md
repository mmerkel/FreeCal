# 92: Unsent changes

**What to build:** What FreeCal has to send becomes a list of unsent changes: every change but a sync records the keys of the Events it touched in `unsent_change` (ADR 0011). Split from ticket 91; decided in a grilling on 2026-10-25.

**Blocked by:** 91

**Status:** done

**Effort:** S

- [x] Schema, in the baseline (nothing is released, ADR 0008): `unsent_change (calendar_id, uid, recurrence_id, since)`
- [x] Every change but a sync adds the previous and the new key of the Event to `unsent_change` in a writable server Calendar, never in a Local Calendar; an existing row keeps its `since`. Nothing reads the table yet; ticket 17 sends from it
- [x] Core tests: a create, edit, move and delete each leave the right keys unsent, and a Local Calendar none; a sync that moves a row to another object records no entry

## Comments

- 2026-11-02: Split from ticket 91. As for `server_object_key` there (comment from 90): the SQL that writes `unsent_change` goes in `store/sql.rs`, behind a `Unit` method that takes a `Journaled`; add the table to the doc comment at the top of `sql.rs`. The reason an unsent change failed waits for ticket 17.
- 2026-11-02: Done. `Unit::entry`, which every write of an Event goes through, adds the previous and the new version's key for every origin but a sync, through `Store::add_unsent_change`; the SQL adds it only in a Calendar with a `url`, a server Calendar. It doesn't check writability itself: `Unit::writable` already refuses every origin but a sync in a Read-only Calendar. Undo records like any change, so undoing a delete not sent yet leaves its key unsent; the sender finds rows and object alike and sends nothing. Two unique indexes keep a key there once, with and without a recurrence ID, and `ON CONFLICT DO NOTHING` keeps its `since`.
  - Beyond the boxes: `Unit::rejoin_keys` (`Loaded::join`) renormalises unsent keys as well as the objects' keys, merging into an existing key with the earlier `since`, as ADR 0011 says a key holds the recurrence ID as the rows do. Without it, a sync that joins an Exception the user changed before its Recurring Event arrived would leave a key matching no rows, which ticket 17 would send as a delete.
  - For ticket 17: an unsent key can match neither rows nor an object, e.g. an Event's previous key after its recurrence ID changed, or after an import's `join` (its `replace` records the zoned key after `rejoin_keys` renamed it). The sender should drop such keys.
