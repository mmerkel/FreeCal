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

**Status:** ready-for-agent

**Effort:** M

- [ ] The tree type with serde in `unimplemented.rs`; the reader in `ical/read.rs`, used by the import too
- [ ] Canonical form: the same content in different order, folding, case and line endings gives equal forms; different content gives different forms
- [ ] One pass over a `VEVENT` gives managed fields and tree; a duplicate managed property and a value form FreeCal doesn't model go into the tree whole; what a precise rule drops is in the module's one list
- [ ] RFC violations are returned with the result
- [ ] Tests cover every kind of property value, parameter (quoted, several values, `X-`) and component (alarm, nested, unknown), unknown parameters on each managed property, and calendar-level content with a Recurring Event, a single Event and Exceptions only
- [ ] The server texts of `core/src/sync/caldav/resource/tests.rs` are read without loss: a test lists what the reader left out of each, and the list holds only what ADR 0017 leaves out
