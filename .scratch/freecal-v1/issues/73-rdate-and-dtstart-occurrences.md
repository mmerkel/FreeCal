# 73: Extra dates of Recurring Events: `RDATE`, and a first start off the rule

**What to build:** A Recurring Event's Occurrences are its whole RFC 5545 recurrence set: `DTSTART`, the starts its `RRULE` gives and its `RDATE` dates, less its `EXDATE` dates. Import carries `RDATE`, and the start of the Event is always its first Occurrence, even when the rule wouldn't give it.

**Blocked by:** 84, 149

**Status:** ready-for-agent

**Effort:** M

- [ ] Import keeps `RDATE` (dates, date-times in any zone, and comma-separated lists, as `EXDATE` does) as extra Occurrences, as a managed field of the Event Version, in its own column like `exdates`. Until this ticket an `RDATE` sits among the Unimplemented Fields (ticket 149) and is written back untouched. A migration, in Rust, moves it from the tree to the column in every Event Version (ADR 0017), from its `ical:` name or from its neutral name if ticket 163 gave it one; the reader and the writer (tickets 143 and 144) then read and write the column, so CalDAV and export carry it
- [ ] An Event with `RDATE` and no `RRULE` is a Recurring Event: today it imports as its first Occurrence only
- [ ] An `RDATE` with `VALUE=PERIOD` gives an Occurrence with that period's own length; every other extra Occurrence lasts as long as the Event, as rule Occurrences do
- [ ] The start of a Recurring Event is always its first Occurrence (RFC 5545, 3.3.10: "The DTSTART property value always counts as the first occurrence"), and it counts toward `COUNT`. Today the `rrule` crate leaves out a start its rule wouldn't give, so `DTSTART` on a Tuesday with `FREQ=WEEKLY;BYDAY=MO;COUNT=3` shows three Mondays and no Tuesday; Google Calendar, Apple Calendar and Thunderbird show the Tuesday
- [ ] Extra dates are cancelled and changed like other Occurrences: deleting one adds it to `exdates`, and an imported `RECURRENCE-ID` can name one
- [ ] The derived bounds (`start_utc`/`end_utc`) include the extra dates, so a date after the rule's last start still shows in its range
- [ ] An Event with extra dates reads as a custom rule in the editor and details ("by further rules"); editing its repeat keeps them, and removing its repeat drops them
- [ ] Every stored date is untrusted: an unreadable one is skipped, and the hostile test Event's fields still go through unchanged
- [ ] Core tests for every point above, and a frontend test that an Occurrence on an extra date opens and deletes like any other

Out of scope: splitting extra dates when a Recurring Event is split at an Occurrence (ticket 08). The editor never sets extra dates.

## Comments

- 2026-10-18: Ticket 07 dropped `RDATE` on import, and found the first-start gap while checking which parts of RFC 5545's recurrence FreeCal reads. Both belong together: the recurrence set is built from the same parts, and the `rrule` crate's `RRuleSet` takes `rdate`s.
- 2026-10-18 (from ticket 08): Splitting a Recurring Event lives in `following` in `core/src/scope.rs`: it ends the rule with `recurrence::rule_until` and splits `exdates` with `recurrence::exdates_where`. Extra dates split the same way there, and "all" moves them with `recurrence::moved_id` as it moves `exdates`.
- 2026-10-24: Blocked by 84 after an architecture review: the rule and scope changes go into the core's Event module (tickets 83 and 84) rather than the six places that rebuild a Recurring Event today.
- 2026-11-16: Reworded from the triage against ADRs 0017 to 0020. The hash clause is gone with the Change Journal. Until this ticket is built `RDATE` is an Unimplemented Field; the ticket moves it to a column. Writing it on export is no longer out of scope, as the export uses the same writer (ticket 09). Blocked by 149 as well.
