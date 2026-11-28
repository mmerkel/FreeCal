# Conformance Breaches

The open Conformance Breaches (`GLOSSARY.md`, Internals; the rule is `CODING_STANDARDS.md`, "Conformance"): ways the Local Store can hold a value its readers can't read, or a writer can produce text that breaks RFC 5545 or RFC 4791. The content of properties and components FreeCal doesn't manage, kept in the Unimplemented Fields as received, is the one exception; what they keep of a managed property is not.

Every ticket's comment has a `Conformance Breaches:` line (`docs/agents/redesign-tickets.md`, "Finishing"). A ticket that adds a breach adds a line here; one that ends a breach strikes its line and names itself. Most severe first. Started 2026-11-28 from a review of the redesign plan, tickets 175 and 176.

## Open

1. **`METHOD` reaches CalDAV servers.** The reader keeps the content outside the Events under `ical:VCALENDAR`, `METHOD` included (one of the Server Texts holds `METHOD:PUBLISH`, as Google's exports do); from 187 the writer puts it into a CalDAV resource, which RFC 4791 §4.1 forbids. An import or a move from a Local Calendar brings it to a server. Ends in 216: the reader leaves it out (`DROPPED`), the writer writes none (decided 2026-11-28).
2. **A second copy of a once-only property** (`DESCRIPTION`, `RRULE`, `UID`, `DTSTART`…) is kept in the Unimplemented Fields and written back beside the managed one (176's review). Ends in 216: the reader leaves it out, the writer writes none.
3. **The Local Store doesn't enforce that its fields read.** No `STRICT` tables and few `CHECK`s; the store's row reader skips rows of the wrong type (`readable`, `core/src/store/sql/mod.rs`); unreadable values fail, default or are skipped depending on the field; managed text can hold control characters (from an edit, later from Google), left out only when written. Ends in 178 (`STRICT`, flag `CHECK`s), 216 (the Store Check reads every Event Version; edits lose control characters), 186 (the tree) and 210 (Google's reader).
5. **Google** (210, 214): a recurrence ID "as obtained" would put RFC 3339 forms beside iCalendar ones; a neutral name's value must be valid in both formats; exported `google:` entries need valid `X-` names and values. Ends in 210 (the iCalendar form, one value form per neutral name) and 214 (`X-GOOGLE-` names, escaped text).

## Accepted

- The content of properties and components FreeCal doesn't manage (an `X-` property's value, a `VALARM` without `ACTION`) is kept as received and written back unchanged.

## Not dealt with for now (agreed 2026-11-28)

- **`SEQUENCE` is never raised** (RFC 5545 §3.8.7.4); ticket 126 decides who raises it.
- **A `TZID` without its `VTIMEZONE`** in the Unimplemented Fields, when the text read lacked it, is written back so.
- **An export of a whole Calendar** (ticket 09) takes the content outside the Events from its first Event only; an `X-WR-TIMEZONE` there changes how FreeCal's own import reads the other Events' UTC times.

## Ended

- ~~**A combination can contradict itself.** A Server Event Merge, or an Undo that combines, takes the times from one side and the Unimplemented Fields from the other, though some entries belong to the times: unknown parameters of `DTSTART` or `RECURRENCE-ID` (`RANGE`), a rule FreeCal can't expand with its `EXDATE`s, a kept `VTIMEZONE`. The result can hold two `RRULE`s, or excluded dates that don't match the rule. Ends in 177: the entries of a managed property belong to its field's group, so they are taken with it; 180, 181 and 198 combine through 177 (decided 2026-11-28).~~ Ended in 177: `entry_group` (`core/src/comparison/mod.rs`) gives each entry of a managed property its field's group, so a combination takes them together.
