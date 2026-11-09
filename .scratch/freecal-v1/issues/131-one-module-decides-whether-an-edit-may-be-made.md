# 131: One module decides whether an edit may be made

**What to build:** The rule for whether the user may change an Event lives in several places. The Local Store's Calendar reader combines the user's Read-only mark with Server read-only (`read_only: row.get(5)? || row.get(6)?`, `core/src/store/sql/mod.rs`). The Change Journal adds the Transaction Origin and guests (`JournalTransaction::writable` and `event_writable`, `core/src/journal/mod.rs`). Three frontend files each combine `readOnly` and `hasGuests` on their own: `src/lib/EventDetails.svelte`, `src/lib/CalendarGrid.svelte` and `src/lib/windowDialogs.svelte.ts`. Give the rule one module on each side of the core interface. From the architecture review of 2026-11-09 (candidate 1).

- **The `read_only` module** (`core/src/read_only.rs`, named after Read-only Calendar, no new term). It holds a plain value built from a stored Calendar's user mark and Server read-only, and, for an Event, from whether its UID has guests. It answers one question: may an edit of this Transaction Origin be made? The answer is either yes or why not: a Read-only Calendar (`CoreError::CalendarReadOnly`) or an Event with guests (`CoreError::EventHasGuests`). An Incoming Sync and a First Sync may always edit, as now. The module knows nothing of the Local Store and is tested in-process.
- **The Change Journal stays the only enforcer** (it is the Guard of Event Rows). `writable` and `event_writable` build the value inside their Database Transaction and ask it. Nothing outside the Change Journal builds the value to decide on an edit.
- **"The user's mark or Server read-only"** moves out of the Local Store reader into the `read_only` module. The reader reads both stored columns as they are and asks the module for the Calendar's `read_only`. `Calendar::read_only` and `Calendar::server_read_only` keep their meaning in the core interface.
- **Not in this ticket:** whether an Outgoing Sync may send, and when the server's privileges are read again. Both go to the Sync Engine in ticket 132.
- **The window:** the core stays the only authority. One Window Model helper combines a Calendar's `readOnly` and an Occurrence's or Event's `hasGuests` into whether it can be edited, and why not. The Event details, the calendar grid and the drag toast in the Window Dialogs use it, and so does the list of Calendars offered for a new Event or an import. The window uses it only to disable or hide actions. When it is wrong, the core refuses with its reason, which the window already shows (`errorText`, `src/i18n/index.ts`). Trade-off, named: the rule is written once in Rust and once in TypeScript. The TypeScript copy is only a hint, and ticket 126 (editing Events with guests) changes both. Shipping a ready-made reason per Occurrence was rejected because marking a Calendar would then need "Occurrences changed" and a re-fetch. Only trying and showing the refusal was rejected because a drag would visibly move and snap back, edits in the editor would be lost on save, and the spec asks for disabled editing.
- No change to what the user sees or to the core interface types.

Spec: US-17, US-19, US-30; "Writability"; "Attendees"; "Event details".

**Blocked by:** none

**Status:** ready-for-agent

**Effort:** M

- [ ] `core/src/read_only.rs` decides whether an edit may be made, from the user's mark, Server read-only, guests and the Transaction Origin, with in-process tests for each case
- [ ] The Change Journal's `writable` and `event_writable` ask it; the Local Store's Calendar reader asks it for `read_only` and no longer combines the columns itself
- [ ] One Window Model helper says whether an Occurrence or Event can be edited and why not; `EventDetails.svelte`, `CalendarGrid.svelte` and `windowDialogs.svelte.ts` use it and combine no flags themselves; frontend tests cover the helper
- [ ] The existing core and frontend tests pass unchanged, apart from tests moved to the helper

## Comments
- 2026-11-09 (/improve-codebase-architecture): the user chose: the module answers only "may this edit be made?" (user's mark, Server read-only, guests, Transaction Origin); a plain value the Change Journal asks; combining the mark with Server read-only moves into it; the window keeps the flags as hints through one Window Model helper; the module is named `read_only`, with no new term.
- Google (ticket 26) reuses the module unchanged; only how Server read-only is read differs per Provider (ticket 132).
