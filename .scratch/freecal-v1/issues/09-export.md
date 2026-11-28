# 09: Export to .ics and .zip

**What to build:** The user can export any Calendar to an `.ics` file, or export everything to a `.zip` with one `.ics` per Calendar organised by Account. While the export runs, a modal "Working…" dialog blocks the window.

- **What is written:** a Calendar's Events as the user has them (its current Event Versions), each with its managed fields and its Unimplemented Fields (ADR 0017). An exported Event keeps its alarms, its custom properties and whatever else FreeCal doesn't show. An Event with an open Conflict exports the server's version, which FreeCal shows.
- **One writer:** the text is built by ticket 176's writer, which builds a CalDAV resource from the same two parts. The export has no code of its own that builds iCalendar text. A Recurring Event and its Exceptions go through the writer together; one `.ics` holds all of a Calendar's Events with one set of time zone definitions.
- **Which entries are written:** those under `ical:` names, as they are. No other entries exist yet: neutral names are written once ticket 210 assigns them, and Google's entries (`google:`, as `X-` properties) come with ticket 214.

Spec: US-91, US-92; "Import/export"; "Untrusted content". ADR 0017.

**Blocked by:** 03, 176, 186

**Status:** ready-for-agent

**Effort:** M

- [ ] Export one Calendar to a single `.ics`
- [ ] Export everything to a `.zip`, one `.ics` per Calendar, organised by Account
- [ ] The file dialog goes through a portal-friendly API
- [ ] Long-running commands show a modal "Working…" dialog that blocks the window until they finish
- [ ] Exported data round-trips: importing the file again yields the same Events, their Unimplemented Fields included (an Event with an alarm, a custom property and a parameter FreeCal doesn't know)
- [ ] The `.ics` is written through ticket 176's writer, never built from strings; the hostile test Event round-trips without injecting properties (`\r\nATTENDEE:…` stays inside its field)
- [ ] `.zip` entry names made from Account and Calendar names are sanitized: a Calendar named `../../.bashrc` stays in its folder
- 2026-10-17 (from ticket 59): The modal "Working…" dialog exists: `src/lib/WorkingDialog.svelte`, shown by `App.svelte`'s `working` state. The file dialog goes through tauri-plugin-dialog's xdg-portal backend, opened by the shell so the window never sees a path (see `pick_import_file` in `main.rs`).

## Comments
- 2026-10-18 (from ticket 07): A kept rule can have an `UNTIL` in the other form after an Event switches between all-day and timed (the core reads both); write it in the form RFC 5545 asks for the Event as exported. Exceptions are Events with the UID of their Recurring Event and a `recurrence_id`; cancelled Occurrences are `exdates`.

- 2026-10-18 (from ticket 73): Once ticket 73 stores `RDATE`, export writes it back, in the same form as `EXDATE`.
- 2026-10-18 (from tickets 74 and 75): Once stored, export writes back every `RRULE` and `EXRULE` of an Event (ticket 74) and the `RANGE=THISANDFUTURE` of an Exception (ticket 75).
- 2026-11-16: Reworded from the triage against ADRs 0017 to 0020. Export writes through ticket 144's writer from the managed fields and the Unimplemented Fields, so it is also blocked by 144 and 149. The comments of 2026-10-18 from tickets 73, 74 and 75 ask for no code here any more: `RDATE`, further `RRULE`s, `EXRULE` and `RANGE=THISANDFUTURE` are Unimplemented Fields until those tickets manage them, and the writer writes them back either way. The one from ticket 07 (the form of `UNTIL`) is the writer's business.
- 2026-11-22: Reworded from the grilling on critique 2 (ADRs 0018 to 0020 rewritten): no User's Pointer; a Conflict no longer holds the user's side apart.
- 2026-11-28: References in the body renumbered for the redesign (new tickets 173 to 214). Comments above this one cite the old numbers; `redesign-plan.md`, "Old tickets in scope", maps them.
