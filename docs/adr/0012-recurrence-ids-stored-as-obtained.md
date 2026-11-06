# Recurrence IDs are stored as obtained and compared by Occurrence

An Exception can arrive without its Recurring Event, with a recurrence ID in another zone (`2026-10-12T04:00:00[America/New_York]`), as a floating wall time, or as a date. Every Event Row, Unsent Change and Server Object keeps its recurrence ID as it was obtained. Whether two recurrence IDs mean the same Occurrence is worked out when they are compared, against the Recurring Event the Event Rows hold: through a `RecurrenceId` type whose only comparison takes that Recurring Event, so that no code compares recurrence IDs as text. The Normal Form (the Occurrence's wall time in the Recurring Event's Event Time Zone, or its date for an all-day Recurring Event) exists only in memory, computed once per Loaded Event. FreeCal writes the recurrence IDs of the Exceptions it makes in the Normal Form, and moving a Recurring Event's time still rewrites its Exceptions' recurrence IDs, because that changes them. One rule, "compare by Occurrence", replaces an invariant that `event`, `combined_event_key`, `unsent_change` and every undo had to keep in step, and it follows ADR 0007: store what was obtained, derive the rest.

## Considered Options

- **The Normal Form stored, renormalised when the Recurring Event arrives** (ADR 0011's first form): the Event Rows, the Combined Event Keys of every Server Object holding the UID and the Unsent Changes had to change together, an undo had to know which recurrence ID changes kept their Occurrence (a flag on each Journal Event Modification), and an import that joined stored Exceptions already left an Unsent Change in the old form. Turning a recurrence ID obtained in another zone into the Normal Form also used the time zone rules at the time of joining, and the result stayed when they changed.
- **A UTC form independent of the Recurring Event**: it can't express floating recurrence IDs, a date for a timed Recurring Event or a time for an all-day one, and ADR 0007 rejects stored UTC, since time zone rule changes would break the match.
- **The Normal Form cached in a column of `event`, recomputed like `start_utc`**: it would be refreshed on exactly the old triggers (the Recurring Event arriving or going, its zone or all-day changing, new time zone rules), and a text column invites plain comparison, for lookups in SQL that nothing needs.

## Consequences

- Comparing recurrence IDs by Occurrence happens in Rust, with the UID's Event Rows loaded, never in SQL. The unique index on `unsent_change` merges only identical text; the Sender merges Unsent Changes of one Occurrence written differently.
- An Occurrence's ID in the core interface is the computed Normal Form; an edit finds the stored Exception for it by comparing by Occurrence.
- An Exception joining its Recurring Event changes no stored recurrence ID, so it neither writes the Change Journal nor gives an undo anything to put back.
