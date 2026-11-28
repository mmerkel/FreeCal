# 175: Unimplemented Fields: the tree, its canonical form and the CalDAV reader

**What to build:** Old ticket 143 as written (`git show design/snapshot-line:.scratch/freecal-v1/issues/143-unimplemented-fields-tree-and-caldav-reader.md`): the tree of an Event Version's Unimplemented Fields with its canonical form, and the reader that fills it and the managed fields from a CalDAV resource in one pass (ADR 0017). Pure code: nothing stores the tree yet (186), and the Sender keeps the patching writer until 187.

- **Where:** `core/src/unimplemented.rs` (`UnimplementedFields`, serde, canonical form, prefixes per Provider) and `core/src/ical/read.rs` (the reader, built from the import's `read_events`, returning `ImportedEvent`s with their tree). The import reads through it too, so `.ics` files keep their `ical:` entries.
- **Canonical form:** two trees are equal exactly when their serialised forms are, whatever order, folding, letter case of names or layout the server sent; where order carries meaning in iCalendar it is kept. The module's doc comment says which is which.
- **Names:** every entry gets the prefix `ical:`; `caldav:` is reserved and unused; neutral names come with 210.
- **What goes into the tree, what the reader can't represent exactly, what doesn't:** ADR 0017 ("What is in the tree", "What the reader can't represent exactly") and old 143's lists: unmanaged properties and components whole; unknown parameters of a managed property under its name without its value; `SEQUENCE`; what today's fields don't cover of the recurrence; calendar-level content on the single Event or Recurring Event, or, with Exceptions only, on the Exception with the earliest Occurrence. Not: managed values, `DTSTAMP`, `LAST-MODIFIED`, time zone definitions, `VERSION`, `PRODID`, a default `CALSCALE`. What a precise rule drops on purpose is listed in one place in the module, for the round-trip check (197).
- **RFC violations** the reader notices are returned with its result for the caller to log.
- **Untrusted content:** the tree is server data, never shown and never across the core interface; the reader keeps today's limits and refuses an unreadable resource whole.

Spec: "Local Store"; "Untrusted content". ADR 0017. Layout: "The target module tree" (`unimplemented.rs`, `ical/`); "Seams for `/tdd`", 1; "By phase", P1, 175.

**Blocked by:** none

**Status:** done

**Effort:** M

- [x] The tree type with serde in `unimplemented.rs`; the reader in `ical/read.rs`, used by the import too
- [x] Canonical form: the same content in different order, folding, case and line endings gives equal forms; different content gives different forms
- [x] One pass over a `VEVENT` gives managed fields and tree; a duplicate managed property and a value form FreeCal doesn't model go into the tree whole; what a precise rule drops is in the module's one list
- [x] RFC violations are returned with the result
- [x] Tests cover every kind of property value, parameter (quoted, several values, `X-`) and component (alarm, nested, unknown), unknown parameters on each managed property, and calendar-level content with a Recurring Event, a single Event and Exceptions only
- [x] The server texts of `core/src/sync/caldav/resource/tests.rs` are read without loss: a test lists what the reader left out of each, and the list holds only what ADR 0017 leaves out

## Comments

- 2026-11-28: Started from commit 50b4cf8.
- 2026-11-28: Implemented. The tree is `UnimplementedFields` (`core/src/unimplemented.rs`); the reader is `core/src/ical/read/mod.rs` (`read/mod.rs`, not `read.rs`: clippy's module rule wants it beside `read/tests.rs`), filling `ImportedEvent::unimplemented` and returning `ReadFile::violations`, which the import and the Incoming Sync write to the Log. What the precise rules drop is `DROPPED` there. The servers' texts moved from `sync/caldav/resource/tests.rs` to `core/src/ical/server_texts.rs`, so that 187 can delete the former. Layout plan amended.
  - **Decisions, for review** (each with its strongest con):
    1. A text holding several UIDs keeps no calendar-level content: no one Event holds it (listed in `DROPPED`). Con: importing a whole calendar's `.ics` drops its `X-WR-CALNAME` and the like; the import is a copy, so nothing on a server is lost.
    2. A parameter value without quotes is one value, commas and all; only a quoted list (`MEMBER="a","b"`) is split. The `icalendar` parser unquotes a single quoted value, so `CN="Doe, Jane"` and an unquoted list `X-P=a,b` look alike, and the first is far more common. Con: an unquoted list is written back as one quoted value, and `X-P=a,b` and `X-P="a","b"` give different trees.
    3. "The earliest Exception" is the one whose recurrence ID's normal form sorts first. Con: Exceptions with recurrence IDs in different zones compare by wall time, not instant.
    4. `SEQUENCE` is an entry, and the reader also looks at it to keep the latest of several revisions of one Occurrence, as today. Con: one property feeds both the tree and that choice; 126 must keep them in step.
    5. A rule FreeCal can't expand goes into the tree with every `EXDATE`; an `EXDATE` with a value that doesn't read goes into the tree whole, the others are read; an `EXDATE` or `RRULE` on an Event that can't hold it (no rule, an Exception) goes into the tree. Con: the Event shows as its first Occurrence while its tree carries the rule.
    6. A `TZID` FreeCal doesn't know is left out (in `DROPPED`) and the time pinned, as today (ADR 0007). Con: the server's zone name is lost on the next write.
    7. RFC violations are logged at info level, one line per text with each kind once. Con: the Incoming Sync logs a resource again each time it changes.
  - **New terms:** **RFC Violation** (`RfcViolation`, new term): a breach of RFC 5545 the reader notices in a text, returned for the caller's Log. **Server Texts** (`ical/server_texts.rs`, new term): real servers' and clients' iCalendar texts the tests of the reader and writer read.
  - Switched-off tests: none off, none on (the plan names none for 175).
- 2026-11-28: The user approved the new terms (RFC Violation, Server Texts) and decisions 1 to 7.
