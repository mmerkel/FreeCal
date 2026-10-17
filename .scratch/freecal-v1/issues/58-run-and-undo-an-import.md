# 58: Run an import as one undoable unit

**What to build:** The core runs a previewed import into a writable Calendar, applying the user's one update / skip / copy choice for existing UIDs, and records it as one Change Journal unit so the whole import can be undone in one step, including imports into Local Calendars. Spec: user stories 97 and 99. Split from ticket 10.

**Blocked by:** 57

**Status:** ready-for-agent

**Effort:** M

- [ ] One update / skip / copy choice per import for existing UIDs; copies get new UIDs
- [ ] The core refuses an import into a Read-only Calendar
- [ ] The import is recorded as one Change Journal unit holding each Event's previous version (start the Change Journal's storage here if it doesn't exist yet; ticket 17 adds the server entries and ticket 19 the list)
- [ ] Undoing the unit restores the Calendar exactly: new Events gone, updated ones back to their previous version (core test compares the whole Calendar before and after)
- [ ] Running and undoing an import each send "Occurrences changed" for the affected Calendars
- [ ] The Event stores ticket 57's mark that its zone was pinned (`ImportedEvent::zone_pinned`); if ticket 50 isn't done yet, the Event's details say the original has no time zone (ADR 0007 rule 5)
- [ ] An update of an existing Event that is still floating keeps the zone pinned before (ADR 0007 rule 2), not ticket 57's newly pinned one
- [ ] The hostile test Event is stored without running anything
- [ ] Core tests for every point above

Out of scope: undo of imports into server Calendars (ticket 19); the dialogs (ticket 59).

## Comments

- 2026-10-13: Split from ticket 10.
- 2026-10-17 (from ticket 57): `ImportPreview.events` doesn't carry `RRULE` yet and skips `RECURRENCE-ID` overrides. Until ticket 07 adds recurrence, a recurring Event would be imported as its first Occurrence only; decide here whether to refuse, warn or wait for 07.
