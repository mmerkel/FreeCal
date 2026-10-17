# 57: Read .ics and .zip files into an import preview

**What to build:** The core reads an `.ics` file, or a `.zip` of `.ics` files such as a Google Takeout export, and previews an import into a chosen Calendar: how many Events are new, how many already exist (same UID), and how many floating times were pinned to which zone. Nothing is stored yet; running the import is ticket 58 and the window's flow is ticket 59. Spec: user stories 94–96. Split from ticket 10.

**Blocked by:** 03

**Status:** ready-for-agent

**Effort:** M

- [ ] Import accepts `.ics` and `.zip` of `.ics`; CSV is not supported; files are parsed with a library, never by hand
- [ ] `.zip` contents are read in memory and never extracted, within fixed limits on unpacked size and number of entries (choose the numbers here, sized for a Google Takeout export, and write them into the spec's "Untrusted content"); exceeding them makes a translated error entry
- [ ] The preview counts new and existing Events for the target Calendar, and the floating times pinned per zone
- [ ] Floating times are pinned by ADR 0007 (see comment), and the parsed Event carries the mark that its zone was pinned
- [ ] Time zone names that aren't IANA names are mapped to IANA names (see comment)
- [ ] An unreadable or unparseable file, or a `.zip` without `.ics` files, gives a translated error naming the file
- [ ] Previewing an `.ics` holding the hostile test Event runs nothing; its HTML description goes through the core's conversion to plain text and link pieces
- [ ] Core tests for every point above

Out of scope: storing anything (ticket 58); the dialogs (ticket 59).

## Comments

- 2026-10-13: Split from ticket 10, which noted: floating times (a time of day with no zone) are pinned by the rule in ADR 0007: a Local Calendar has no zone of its own, so they get the Display Time Zone at import. Store a mark that the zone was pinned (ticket 50 shows a note for it; add the note here if 50 isn't done yet). Time zone names that aren't IANA names (Outlook's `W. Europe Standard Time`, custom `VTIMEZONE`s) must be mapped to IANA names; decide here how (or follow ticket 54 if it comes first), and what happens when no mapping exists.
