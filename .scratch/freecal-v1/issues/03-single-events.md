# 03: Create, view, edit and delete single Events

**What to build:** In a Local Calendar, the user can click an empty slot or drag across a time range to create an Event, choose which writable Calendar it goes into, open it to see its details, edit its title, time, all-day flag, place, description and Calendar, and delete it. A keyboard shortcut creates an Event.

**Blocked by:** 02

**Status:** done

- [x] The core returns the Occurrences in a date range
- [x] Clicking an empty slot or dragging across a range opens the editor prefilled with that time
- [x] The editor offers only writable Calendars
- [x] With no writable Calendar, clicking a slot, dragging or the shortcut doesn't open the editor but points to "+" next to the Local Account (a Local Store starts with Home and Work, ticket 34, but the user can delete them)
- [x] Event details show title, time, place, description and Calendar
- [x] Edit and delete work and are persisted in the Local Store
- [x] Every change sends "Occurrences changed" with the affected Calendar IDs; the frontend re-fetches the visible range and keeps the current selection
- [x] An open editor's draft survives a re-fetch
- [x] A Google-style shortcut creates an Event
- [x] Hidden Calendars' Events are not shown
- [x] Descriptions cross to the frontend as structured content (paragraphs of text and link pieces), built in the core; HTML is converted as in "Untrusted content" in the spec, and the editor edits the plain-text form. An untouched description keeps its original bytes
- [x] Only `http(s)://` and `mailto:` become links; clicking one opens the system browser through the shell, which checks the scheme again in Rust; a link whose text differs from its URL asks "Open <full URL> in your browser?" first
- [x] The hostile test Event is created at both seams and runs through the core, the grid, Event details and the editor: nothing in it runs or is rendered as HTML, and disallowed links are plain text

## Comments

- 2026-10-04: Decide here whether to generate the TypeScript types from the Rust ones. The interface QA after ticket 01 kept them hand-written because the surface was tiny (`Account`, `Provider`, an empty `Signal`). This ticket adds Occurrences, the "Occurrences changed" Signal and structured descriptions, which is where hand copies start to drift. Candidates: ts-rs (MIT, stable; can be a dev-dependency via `#[cfg_attr(test, derive(TS))]`) or tauri-specta (MIT, also generates command wrappers, release candidate at the time). Generated files need a check that they are up to date. Command names stay covered by the `mockIPC` test of `tauriCore` either way.
- 2026-10-04: Implemented. Notes for later tickets:
  - **Types are generated with ts-rs** (decided here). The core derives `ts_rs::TS` only under `cfg(test)`, so ts-rs is a dev-dependency. `core/src/typescript.rs` writes `src/core/types.generated.ts`, and `cargo test` fails while that file is out of date (`FREECAL_WRITE_TYPES=1 cargo test -p freecal-core` rewrites it). `src/core/types.ts` re-exports it. tauri-specta was left out: it was still a release candidate. Command names stay covered by the `mockIPC` test of `tauriCore`.
  - Core: `occurrences(from, to)` (shown Calendars only, Display Time Zone, ordered by start), `event`, `create_event`, `edit_event`, `delete_event`, and `link_allowed`. `When` is `{kind: 'allDay'|'timed', start, end}` with exclusive ends and wall-clock times. Tests: `core/tests/events.rs`, plus unit tests in `core/src/description.rs`.
  - Storage: timed Events are UTC instants plus the zone they were made in (`event.time_zone`); all-day Events are dates. A wall time the clocks skip is read as an hour later; an ambiguous one as the first. An edit that leaves the time as shown keeps the stored time and zone, so ticket 06's per-Event zones have something to build on.
  - **Ticket 06**: the `Clock` gained `time_zone()` (the system zone via `iana-time-zone` and `chrono-tz`), and `TestClock::set_time_zone` controls it. The core already converts Occurrences and ranges to that zone; 06 adds the override, the Signal and the editor's zone choice. FullCalendar still uses the browser's local zone, which matches only while no override is set.
  - Descriptions: `event.description` holds the original bytes, and `event.description_html` says whether they are HTML. Events created or edited in FreeCal are plain text, so markup typed by the user stays literal. **Tickets 10 and 25** must set `description_html` for Google and imported HTML descriptions; HTML conversion is tested now by arranging that flag. An edit whose plain text equals the stored plain-text form keeps the original bytes (and the flag).
  - `calendar.read_only` exists (default 0) and the core refuses changes to Events of Read-only Calendars with `CalendarReadOnly`, including moves into or out of one. **Ticket 17** adds the operation that sets it. The details hide Edit and Delete for those Events.
  - The hostile test Event is `fixtures/hostile-event.json`, shared by both seams. `expected` holds the structured description the core builds from it, as plain text and as HTML. The Rust tests check the core against it, and the frontend tests feed it to the details and the editor.
  - Links open through the shell's `open_link` command, which checks `link_allowed` again and uses `tauri-plugin-opener` from Rust. The window gets no opener permission.
  - Frontend: `App.svelte` owns the visible range, the open details or editor and the selection. It re-reads the range on "Occurrences changed" and "Calendars changed"; open details re-read their Event (and close if it is gone), and an open editor keeps its own draft. `c` creates an Event at the next full hour unless a dialog is open or focus is in a text field. A new Event goes into the first shown writable Calendar.
  - FullCalendar can't be driven through drag or select in jsdom (its hit-testing needs layout). `CalendarGrid.test.ts` covers what jsdom can do (chips, colours, clicks, the visible range, the hostile title). `App.events.test.ts` runs the App's flows against `src/test/StubGrid.svelte`, and `when.test.ts` covers the selection conversion. **Ticket 05** will hit the same limit for drags.
  - FullCalendar calls `datesSet` while it renders, inside the grid's effect. Callbacks into the owner run under `untrack`, or the owner's state becomes a dependency of the grid's effect and loops. Keep that in mind for new callbacks.
  - No confirmation before deleting an Event, as in Google Calendar; a Local Event can't be brought back until Snapshots (ticket 11).
  - Checked by hand in the real window (debug build on Xvfb): the no-Calendar hint, drag across three days, saving, the hostile Event in the grid, its details and the editor, the link confirmation and Escape, an untouched save keeping the HTML description's bytes, `c`, delete, and dark mode. The run-freecal driver gained `drag` and now types every printable ASCII character.

