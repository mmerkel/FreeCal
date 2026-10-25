# 91: One server object per UID, and unsent changes

**What to build:** A Calendar's rows with one UID belong to the one server object holding it, found through the objects' keys instead of an object ID on each row; the Change Journal stops recording objects; and what FreeCal has to send becomes a list of unsent changes (ADR 0011, amending ADR 0010). Decided in a grilling on 2026-10-25.

**Blocked by:** 90

**Status:** ready-for-agent

**Effort:** L

- [ ] Schema, in the baseline (nothing is released, ADR 0008): `server_object_key (object_id, uid, recurrence_id)` and `unsent_change (calendar_id, uid, recurrence_id, since)`; `event.object_id`, `journal_entry.object_id` and `server_object.pending` go
- [ ] `Unit::mirror` writes an object's keys with its data, one per component, recurrence IDs normalised as the rows hold them; an unreadable object keeps the keys of its last readable version. Where the Store renormalises a UID's recurrence IDs (`Loaded::join`, ticket 88), it renormalises the keys in the same function
- [ ] Rows find their object through the keys: on CalDAV the object holding the row's Calendar and UID. Google's rule, by UID and recurrence ID, comes with ticket 26. `Store::events_in_object`, `original_server_data` and `sync::moved` go through the keys
- [ ] Ownership, in one function that sync and ticket 23 both use: of several objects in a Calendar holding one UID, the one holding the Recurring Event owns it, otherwise the first mirrored. The others are mirrored without rows. When the owner is removed, the next one takes over, its rows read from its mirrored data
- [ ] `event::take_object` replaces the rows of the UIDs the object owns; it never writes rows of a UID another object owns
- [ ] `EventFields.object_id` and `Unit::placed` go. Versions and their hash hold FreeCal's fields only, a new hash format, with no rehashing of existing Local Stores
- [ ] Every change but a sync adds the previous and the new key of the Event to `unsent_change` in a writable server Calendar, never in a Local Calendar; an existing row keeps its `since`. Nothing reads the table yet; ticket 17 sends from it
- [ ] Core tests: two objects with one UID (the Recurring Event's object owns it, whichever came first; the other has no rows; it takes over when the owner goes, with a sync token too); a user's new Exception and an undone delete not yet sent find their object through the keys; a create, edit, move and delete each leave the right keys unsent, and a Local Calendar none; a sync that moves a row to another object records no entry; an Exception without its Recurring Event keeps matching its keys after joining it
- [ ] Ticket 87's and 88's tests that keep rows of a UID in two objects change to the ownership rule

## Comments

- 2026-10-25: Weighed and rejected in the grilling: an object ID on each row outside the journal plus a key table (exact for objects that break RFC 4791, but three mechanisms for two edge cases); a pending mark on keys or objects (a new Event has neither, and a sync rewriting keys would lose it); recurrence IDs in keys only for Google (one rule for every Provider was preferred).
- 2026-10-25: Reporting an object that isn't shown because another owns its UID waits for ticket 23; so does the reason an unsent change failed, for ticket 17.
- 2026-10-25 (from 90): The SQL that writes `server_object_key` and `unsent_change` goes in `store/rows.rs`, behind `Unit` methods, like the other tables the Change Journal guards; add both tables to the doc comment at the top of `rows.rs`. Box 12's renormalisation of keys is a `Unit` call from `Loaded::join`, which already takes the unit; code outside `store` can't write those tables itself.
