# 143: Unimplemented Fields: the tree, its canonical form and the CalDAV reader

**What to build:** The type that holds one Event Version's Unimplemented Fields, and the reader that fills it from a CalDAV resource (ADR 0017). Pure code: nothing is stored and nothing calls it yet. Ticket 144 adds the writer, ticket 149 stores the tree, ticket 150 sends from it.

- **The tree:** a Rust type of named entries, in a module named after the term (for example `core/src/event/unimplemented_fields.rs`), serialised through serde to the text one column will hold. An entry holds a value, parameters, or further entries (a component such as an alarm, with its own properties and components).
- **Canonical form:** two trees are equal exactly when their serialised forms are, whatever order, folding, letter case of names or layout the server sent. Where order carries meaning in iCalendar it is kept (several properties of one name keep their order among themselves); where it doesn't, the form sorts. State in the module's doc comment which is which.
- **Names:** every entry this ticket makes has a name with the prefix `ical:`, as everything read from iCalendar text has; `caldav:` is reserved and assigned to nothing (ADR 0017). No neutral name is assigned; those come with Google's reader (ticket 25).
- **The reader:** from a resource's iCalendar text to, for each `VEVENT`, the managed fields as read today (`read_vevent`, `core/src/sync/caldav/resource/mod.rs`, and the import's reading in `core/src/import/mod.rs`) and its tree. One reading of a `VEVENT` gives both, so a property can't be in both or in neither.
- **What goes into the tree:**
  - every property FreeCal doesn't manage, with its parameters and value;
  - every component inside the `VEVENT` (alarms, and ones FreeCal has never heard of), whole;
  - for a managed property, the parameters FreeCal doesn't know (`LANGUAGE`, `ALTREP`, `X-` parameters), as one entry under the property's name, without its value;
  - `SEQUENCE`, as an entry like any other (ADR 0017: written back unchanged until ticket 126);
  - what today's fields don't cover of the recurrence (`RDATE`, a second `RRULE`, `EXRULE`, `RANGE=THISANDFUTURE` on a recurrence ID), until tickets 73 to 75 make them managed fields;
  - what the resource holds at the calendar level, outside its Events, under one `ical:` name on the resource's single Event or Recurring Event, or, when it holds only Exceptions, on the Exception with the earliest Occurrence (ADR 0017 leaves "the first" open; this ticket fixes it).
- **What doesn't:** the values of managed properties; `DTSTAMP` and `LAST-MODIFIED`; time zone definitions; at the calendar level, what the writer sets itself (`VERSION`, `PRODID`, `CALSCALE` when it is the default).
- **Untrusted content:** the tree is server data. It is never shown and never crosses the core interface. The reader keeps the limits the reading of a resource has today, and a resource it can't read is refused whole, as now.
- The patching writer and the Mirror's text are untouched by this ticket.

Spec: "Local Store"; "Untrusted content". ADR 0017.

**Blocked by:** none

**Status:** ready-for-agent

**Effort:** M

- [ ] The tree type with serde, in a module named after Unimplemented Fields
- [ ] Canonical form: tests feed the same content in different order, folding, case and line endings and get equal serialised forms; content that differs gives different forms
- [ ] The reader gives managed fields and tree from one pass over a `VEVENT`
- [ ] Tests cover every kind of property value, parameter (quoted, several values, `X-`) and component (alarm, nested, unknown), unknown parameters on each managed property, and calendar-level content in a resource with a Recurring Event, with a single Event, and with Exceptions only
- [ ] The texts in `core/src/sync/caldav/resource/tests.rs` that the patching writer is tested with are read without loss: a test lists, for each, what the reader left out, and the list holds only what "What doesn't" names

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. First half of what was proposed as one ticket (L); the writer is ticket 144.
- 2026-11-16: prefix decided by the user: `ical:` for the format, not `caldav:` for the Provider.
