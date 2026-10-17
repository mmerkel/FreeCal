# 57: Read .ics and .zip files into an import preview

**What to build:** The core reads an `.ics` file, or a `.zip` of `.ics` files such as a Google Takeout export, and previews an import into a chosen Calendar: how many Events are new, how many already exist (same UID), and how many floating times were pinned to which zone. Nothing is stored yet; running the import is ticket 58 and the window's flow is ticket 59. Spec: user stories 94–96. Split from ticket 10.

**Blocked by:** 03

**Status:** done

**Effort:** M

- [x] Import accepts `.ics` and `.zip` of `.ics`; CSV is not supported; files are parsed with a library, never by hand
- [x] `.zip` contents are read in memory and never extracted, within fixed limits on unpacked size and number of entries (choose the numbers here, sized for a Google Takeout export, and write them into the spec's "Untrusted content"); exceeding them makes a translated error entry
- [x] The preview counts new and existing Events for the target Calendar, and the floating times pinned per zone
- [x] Floating times are pinned by ADR 0007 (see comment), and the parsed Event carries the mark that its zone was pinned
- [x] Time zone names that aren't IANA names are mapped to IANA names (see comment)
- [x] An unreadable or unparseable file, or a `.zip` without `.ics` files, gives a translated error naming the file
- [x] Previewing an `.ics` holding the hostile test Event runs nothing; its HTML description goes through the core's conversion to plain text and link pieces
- [x] Core tests for every point above

Out of scope: storing anything (ticket 58); the dialogs (ticket 59).

## Comments

- 2026-10-13: Split from ticket 10, which noted: floating times (a time of day with no zone) are pinned by the rule in ADR 0007: a Local Calendar has no zone of its own, so they get the Display Time Zone at import. Store a mark that the zone was pinned (ticket 50 shows a note for it; add the note here if 50 isn't done yet). Time zone names that aren't IANA names (Outlook's `W. Europe Standard Time`, custom `VTIMEZONE`s) must be mapped to IANA names; decide here how (or follow ticket 54 if it comes first), and what happens when no mapping exists.
- 2026-10-17: Done. `Core::preview_import` takes an `ImportFile` (read with `ImportFile::read`, or built from bytes) and returns an `ImportPreview` with the parsed `ImportedEvent`s, each with `zone_pinned` and `exists`. Decisions:
  - Limits: 10,000 entries, 128 MiB per file and for a `.zip`'s `.ics` files unpacked together, counted while unpacking (written into the spec's "Untrusted content").
  - Time zone names: an IANA name, a path ending in one, a Windows name through CLDR's `windowsZones` table (`core/src/import/windows_zones.rs`), or a custom `VTIMEZONE`'s `X-LIC-LOCATION`. A zone that can't be mapped is pinned like a floating time and counted in the preview. UTC times keep their instant and get the file's `X-WR-TIMEZONE` (Google writes it) when it names one, otherwise UTC. Written into the spec's "Import/export"; ticket 54 can reuse the mapping.
  - Errors are `ErrorCode`s carrying the file's name (`takeout.zip/Calendar/Work.ics` for an entry); `errorText` fills `{file}`.
  - A description is read as HTML when it has a common HTML tag, otherwise as plain text.
  - An Event without `DTSTART` makes the file unreadable; a UID that appears twice is kept once (first wins).
  - Not done here, handed to ticket 58: storing the pinned mark and the note for it (ticket 50 isn't done); ADR 0007 rule 2 on update; recurrence (`RRULE` isn't carried yet and `RECURRENCE-ID` overrides are skipped, see ticket 07).
