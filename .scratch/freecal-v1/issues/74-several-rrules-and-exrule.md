# 74: Several `RRULE`s and `EXRULE` on one Recurring Event

**What to build:** A Recurring Event from a file can have several `RRULE` lines and `EXRULE` lines (RFC 2445 allows both; RFC 5545 deprecates them, but older files and some servers still write them). Its Occurrences are the starts every `RRULE` gives, less those any `EXRULE` gives. Today only the first `RRULE` is read and `EXRULE` is ignored, so Occurrences go missing or show although they were cancelled.

**Blocked by:** none

**Status:** ready-for-agent

**Effort:** M

- [ ] Import keeps every `RRULE` and every `EXRULE` of an Event; the Change Journal's hash covers them (a new format, re-hashed by the migration as ticket 07 did)
- [ ] Occurrences are the union of every `RRULE`'s starts with the Event's start (ticket 73), less every `EXRULE`'s starts and the `EXDATE`s; a start two rules give is one Occurrence
- [ ] `COUNT` and `UNTIL` work per rule, as RFC 2445 defines them; the derived bounds (`start_utc`/`end_utc`) are open-ended when any `RRULE` is
- [ ] Each rule is untrusted on its own: an `RRULE` FreeCal can't read is dropped and the others are kept; an `EXRULE` FreeCal can't read cancels nothing. The walk stays capped (`MAX_COUNTED`) across all rules together
- [ ] Such an Event reads as a custom rule in the details ("Repeats by a custom rule"). Choosing a repeat in the editor replaces every rule and `EXRULE` with the one chosen, and the editor's custom note says so
- [ ] Core tests for every point above, including a hostile stored rule list

Out of scope: writing them on export (ticket 09).

## Comments

- 2026-10-18: Found while checking which parts of RFC 5545's recurrence FreeCal reads (after ticket 07). The `rrule` crate's `RRuleSet` takes several `rrule`s, and `exrule`s behind its `exrule` feature.
