# 58: Run an import as one undoable unit

**What to build:** The core runs a previewed import into a writable Calendar, applying the user's one update / skip / copy choice for existing UIDs, and records it as one Change Journal unit so the whole import can be undone in one step, including imports into Local Calendars. Spec: US-97 and US-99. Split from ticket 10.

**Blocked by:** 57

**Status:** done

**Effort:** M

- [x] One update / skip / copy choice per import for existing UIDs; copies get new UIDs
- [x] The core refuses an import into a Read-only Calendar
- [x] The import is recorded as one Change Journal unit holding each Event's previous version (start the Change Journal's storage here if it doesn't exist yet; ticket 17 adds the server entries and ticket 19 the list)
- [x] Undoing the unit restores the Calendar exactly: new Events gone, updated ones back to their previous version (core test compares the whole Calendar before and after)
- [x] Running and undoing an import each send "Occurrences changed" for the affected Calendars
- [x] The Event stores ticket 57's mark that its zone was pinned (`ImportedEvent::zone_pinned`); if ticket 50 isn't done yet, the Event's details say the original has no time zone (ADR 0007 rule 5)
- [x] An update of an existing Event that is still floating keeps the zone pinned before (ADR 0007 rule 2), not ticket 57's newly pinned one
- [x] The hostile test Event is stored without running anything
- [x] Core tests for every point above

Out of scope: undo of imports into server Calendars (ticket 19); the dialogs (ticket 59).

## Comments

- 2026-10-13: Split from ticket 10.
- 2026-10-17 (from ticket 57): `ImportPreview.events` doesn't carry `RRULE` yet and skips `RECURRENCE-ID` overrides. Until ticket 07 adds recurrence, a recurring Event would be imported as its first Occurrence only; decide here whether to refuse, warn or wait for 07.
- 2026-10-17: Done. `Core::run_import(&ImportPreview, ExistingEvents)` returns an `ImportOutcome` (counts and the `ImportId`); `Core::undo_import(ImportId)` undoes it. Neither is a Tauri command yet; ticket 59 adds them. Decisions:
  - The Change Journal starts here as `journal_import` (one unit per import, with `made_at` for ticket 19's 90 days) and `journal_entry` (one per Event: its ID, UID and previous version, none when created). Ticket 17 adds server entries.
  - Whether an Event exists is looked up again at run time, not taken from the preview. The whole import, and the whole undo, is one transaction.
  - Undo deletes the Events the import added wherever they are now, and writes back updated ones even if deleted since. It matches Events by ID and UID, so an Event that got a deleted one's ID is spared. It is refused while any Calendar it touches is Read-only, and an import is undone once (`importNotFound`).
  - The pinned mark is stored (`event.zone_pinned`) and shown as `Event.pinnedZone` in the details: "The original has no time zone, so FreeCal uses <zone>." Ticket 50 can reword it and add it to the editor (ADR 0007 rule 5). An edit that changes the time drops the mark (rule 4).
  - Rule 2 applies when both ends of the new version float. When only one does, it takes the other end's zone (rule 3).
  - An import that adds or updates nothing sends no "Occurrences changed".
  - Recurrence: wait for ticket 07, which now has a box for it. Until then a recurring Event is imported as its first Occurrence under its UID, so a later import with "update" completes it.
