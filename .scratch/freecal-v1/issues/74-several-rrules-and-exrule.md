# 74: Several `RRULE`s and `EXRULE` on one Recurring Event

**What to build:** A Recurring Event from a file can have several `RRULE` lines and `EXRULE` lines (RFC 2445 allows both; RFC 5545 deprecates them, but older files and some servers still write them). Its Occurrences are the starts every `RRULE` gives, less those any `EXRULE` gives. Today only the first `RRULE` is read and `EXRULE` is ignored, so Occurrences go missing or show although they were cancelled.

**Blocked by:** 84, 149

**Status:** ready-for-agent

**Effort:** M

- [ ] Import keeps every `RRULE` and every `EXRULE` of an Event as managed fields of the Event Version. Until this ticket the further `RRULE`s and the `EXRULE`s sit among the Unimplemented Fields (ticket 149) and are written back untouched. A migration, in Rust, moves them from the tree to their columns in every Event Version (ADR 0017); the reader and the writer (tickets 143 and 144) then read and write the columns, so CalDAV and export carry them
- [ ] Occurrences are the union of every `RRULE`'s starts with the Event's start (ticket 73), less every `EXRULE`'s starts and the `EXDATE`s; a start two rules give is one Occurrence
- [ ] `COUNT` and `UNTIL` work per rule, as RFC 2445 defines them; the derived bounds (`start_utc`/`end_utc`) are open-ended when any `RRULE` is
- [ ] Each rule is untrusted on its own: an `RRULE` FreeCal can't read is dropped and the others are kept; an `EXRULE` FreeCal can't read cancels nothing. The walk stays capped (`MAX_COUNTED`) across all rules together
- [ ] Such an Event reads as a custom rule in the details ("Repeats by a custom rule"). Choosing a repeat in the editor replaces every rule and `EXRULE` with the one chosen, and the editor's custom note says so
- [ ] Core tests for every point above, including a hostile stored rule list

Spec: "Untrusted content". ADR 0017.

## Comments

- 2026-10-18: Found while checking which parts of RFC 5545's recurrence FreeCal reads (after ticket 07). The `rrule` crate's `RRuleSet` takes several `rrule`s, and `exrule`s behind its `exrule` feature.
- 2026-10-24: Blocked by 84 after an architecture review: the rule and scope changes go into the core's Event module (tickets 83 and 84) rather than the six places that rebuild a Recurring Event today.
- 2026-11-16: Reworded from the triage against ADRs 0017 to 0020. The hash clause is gone with the Change Journal. Until this ticket is built the further rules are Unimplemented Fields; the ticket moves them to columns. Writing them on export is no longer out of scope, as the export uses the same writer (ticket 09). Blocked by 149 as well.
