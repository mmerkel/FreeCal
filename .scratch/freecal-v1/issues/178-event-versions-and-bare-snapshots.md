# 178: Event Versions are rows that never change; `event` becomes a view

**What to build:** The first part of the Calendar History (ADR 0018, "Event Versions"): a write to an Event inserts a new Event Version instead of changing a row, and each Event Version begins and ends with a Snapshot. Old ticket 145 (`git show design/snapshot-line:.scratch/freecal-v1/issues/145-event-versions-table.md`), with `begin` and `end` brought forward from old 146, so there is no user's Current-Version Mark column. The Change Journal keeps recording and undo stays per Event until 182.

- **`core/src/history/`** with the Snapshot Writer (`SnapshotWriter`, `history/mod.rs`, SQL in `history/versions.rs`, `pub(super)`): it opens with the Database Transaction, writes the `snapshot` row with its first change and none for a step that changes nothing, and has `put`, `remove` and the reads the edits need. Every write (create, edit, delete, reschedule, the scope edits, Propagate, import, the Change Journal's undo) goes through it: the `JournalTransaction` calls it for its row writes.
- **Tables:** `snapshot` bare (`id`, the line's order, and `made_at`; origin is 179's); `event_version` with what `event` holds today, `begin` → `snapshot` `ON DELETE SET NULL`, `end` → `snapshot` `ON DELETE CASCADE`; the partial unique index on (Calendar, UID, recurrence ID) where `end IS NULL`, today's rule of one Recurring Event or single Event per Calendar and UID kept among those rows; a partial index on (Calendar, start) where `end IS NULL`. `event` is a view over the rows where `end IS NULL`; the readers in `store/sql/mod.rs` read it unchanged. Each constraint carries a comment naming its rule and ADR. The schema changes in place (no real Local Store).
- **A write** inserts the new row, even when its content equals an earlier row's (no lookup), and sets `end` on the row it replaces. Old rows stay; nothing is dropped until 185.
- **Row IDs:** `EventId` becomes the Event Version's row ID and changes with every write; callers in `event/` reload by key. **Glue until 182, named:** the Change Journal, undo per Event and the Sent Event Rows identify an Event by a never-reused ID (ticket 85). A column carries that ID from each row to its successor, and the view shows it where they read it, so they work unchanged; 182 removes it with the Change Journal. Trade-off: a column that lives four tickets, against rewriting the identity inside code that goes.
- **Conformance** (`CODING_STANDARDS.md`, "Conformance"): `snapshot` and `event_version` are `STRICT` tables, so a value of the wrong type is refused when written, not skipped when read; `event_version`'s flags (`all_day`, `zone_pinned`, `description_html`) carry `CHECK (… IN (0, 1))`. What SQL can't check (times, zones, rules, recurrence IDs read) is 216's Store Check.
- The update triggers on `event_version` and the Store Check are 179's.
- No change to the core interface or to what the user sees.

Spec: "Local Store"; "Calendar History". ADR 0018 ("Event Versions", "Snapshots", "Who may write"). Layout: "New terms" (Snapshot Writer); "Interfaces designed twice", "The store of Event Versions"; "Tables (end state)" (`snapshot`, `event_version`, `event`); "By phase", P2, 178.

**Blocked by:** 174

**Status:** ready-for-agent

**Effort:** M

- [ ] `history/` with `SnapshotWriter`; nothing outside `history` writes `event_version` or `snapshot`
- [ ] `snapshot` and `event_version` with `begin`, `end`, their foreign keys and indexes; `event` a view over the rows where `end IS NULL`
- [ ] Every write makes one Snapshot and new rows; a step that changes nothing makes none; an edit changes the row ID behind an unchanged key (test through the core interface)
- [ ] A test reads a Calendar's Events as they were at each Snapshot (`begin <= N` and `end` empty or `> N`)
- [ ] Reading a window's Occurrences uses the partial index (query-plan test)
- [ ] The Change Journal's, undo's and the send tests pass unchanged
- [ ] `snapshot` and `event_version` are `STRICT`, the flags carry their `CHECK`s; a test writes a value of the wrong type and a flag of 2 and sees each refused
- [ ] `GLOSSARY.md` follows: Event Row (`event` is a view), Event Version (no longer the Change Journal's), Server Object and Corresponding Server Object (Event Versions), Recurrence ID as obtained
