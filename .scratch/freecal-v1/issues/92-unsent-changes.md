# 92: Unsent changes

**What to build:** What FreeCal has to send becomes a list of unsent changes: every change but a sync records the keys of the Events it touched in `unsent_change` (ADR 0011). Split from ticket 91; decided in a grilling on 2026-10-25.

**Blocked by:** 91

**Status:** ready-for-agent

**Effort:** S

- [ ] Schema, in the baseline (nothing is released, ADR 0008): `unsent_change (calendar_id, uid, recurrence_id, since)`
- [ ] Every change but a sync adds the previous and the new key of the Event to `unsent_change` in a writable server Calendar, never in a Local Calendar; an existing row keeps its `since`. Nothing reads the table yet; ticket 17 sends from it
- [ ] Core tests: a create, edit, move and delete each leave the right keys unsent, and a Local Calendar none; a sync that moves a row to another object records no entry

## Comments

- 2026-11-02: Split from ticket 91. As for `server_object_key` there (comment from 90): the SQL that writes `unsent_change` goes in `store/sql.rs`, behind a `Unit` method that takes a `Journaled`; add the table to the doc comment at the top of `sql.rs`. The reason an unsent change failed waits for ticket 17.
