# 145: Event Versions are rows that never change; `event` becomes a view

**What to build:** The first part of the Calendar History (ADR 0018): the table of Event Versions and the user's Current-Version Mark. A write to an Event makes a new row instead of changing one. Snapshots and pointers come with ticket 146; the Change Journal keeps recording and undo stays per Event until ticket 147.

- **The table** (`event_version`) holds what the `event` table holds today (`core/src/store/schema.rs`): the managed fields, the Calendar, the UID and the recurrence ID as obtained. A column for the Unimplemented Fields comes with ticket 149.
- **The user's Current-Version Mark** is a column. A unique index allows one marked row per Calendar, UID and recurrence ID; today's rule of one row per Calendar and UID that is no Exception (`event_calendar_uid_not_exception`) is kept among marked rows. Ticket 146 replaces this column by the Event Version's `end` (ADR 0018, rewritten 2026-11-22), keeping the index as a partial one where `end` is empty. The server's Current-Version Mark comes with ticket 151.
- **A row never changes:** a trigger refuses an update of every column but the mark. The schema carries a comment naming the rule and ADR 0018 at each constraint.
- **`event` is a view** over the rows with the user's mark, with a partial index over Calendar and start time, so that reading the Occurrences of a window costs what it did. A test asserts the query plan uses the index. Every reader keeps reading `event`; only the code that writes Event Versions reads the table.
- **A write** (create, edit, delete, a Propagate of a Server Object, an import, an undo) inserts the new row with the mark, also when its content equals an earlier row's (no lookup, ADR 0018), and takes the mark from the old one, in its Database Transaction. Until ticket 146 gives Snapshots that refer to old rows, a row without a mark is deleted in that same Database Transaction: that is ADR 0018's rule for when an Event Version goes, with no Snapshot kept yet.
- **Glue until ticket 147, named:** the Change Journal identifies an Event by its row ID (`journal_event_modification.event_id`, and ticket 85's rule that an ID is never reused), and so does undo per Event. The table gets a column holding that ID, carried from each row to its successor and shown by the view as `id`, so the Change Journal, undo per Event and the Sent Event Rows work unchanged. Ticket 147 removes the column with the Change Journal. Trade-off: a column that lives for two tickets, against rewriting the identity inside code that goes.
- The hash of an Event Version, the Change Journal's tables and the Write Permissions stay; the Change Journal's Write Permission covers the new table as it covered `event`.
- No migration of data: no real Local Store exists. The schema changes in place.
- No change to the core interface or to what the user sees.

Spec: "Local Store"; "Calendar History". ADR 0018 ("Event Versions", "Who may write").

**Blocked by:** 142

**Status:** ready-for-agent

**Effort:** M

- [ ] `event_version` with the user's Current-Version Mark and its unique index; `event` is a view over the marked rows
- [ ] The trigger refuses an update of any column but the mark; a test tries each column
- [ ] Every write makes a new row; an edit changes the row ID behind an unchanged Combined Event Key (test through the core interface)
- [ ] No unmarked row is left after any test scenario
- [ ] Reading a window's Occurrences uses the partial index (query-plan test)
- [ ] The Change Journal's, undo's and the send tests pass unchanged

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. First of four parts of what was proposed as one ticket (XL): 145 the table, 146 Snapshots and the User's Pointer, 147 undo and redo, 148 undo for one Calendar and dropping.
- 2026-11-21: Decided in a grilling: every write makes a new row, also for content equal to an earlier one; only undo and redo bring a row back. Event Versions are compared same row first, then by canonical form; a side that takes over the other's version takes its row (ADR 0018, "Event Versions").
