# 142: An Event is identified by its Combined Event Key in the core interface and the window

**What to build:** With ADR 0018 an Event's row ID changes with every edit, so no caller may hold one. Today the core interface names an Event by the Event Row's ID (`EventId`, `core/src/event/when.rs`): the operations that read, edit, reschedule and delete an Event or an Occurrence (`Core::event`, `edit_event`, `reschedule_event`, `edit_occurrence`, `reschedule_occurrence`, `exception_counts`, `delete_event`, `delete_occurrence`, `original_server_data`, `event_history`, `core/src/lib.rs`), the Event and the Occurrence the window gets (`Event.id`, `Occurrence.eventId`), and the Change Journal's listings and undo per Event (`ListedJournalEventModification.eventId`, `UndoTarget::Event`). Replace it everywhere in the core interface by the Combined Event Key. Nothing comes out of the Local Store in this ticket: it is step 0 of the move to the Calendar History.

- **The interface type:** one type for "which Event": the Calendar, the UID and, for an Exception, the recurrence ID as obtained. It is the existing Combined Event Key (`CombinedEventKey`, `core/src/store/mod.rs`) or built from it, and is branded in TypeScript. The core finds the Event by comparing recurrence IDs by Occurrence, as everywhere (ADR 0012); the window never compares, parses or shows a key's parts. The UID is server data: see the spec's "Untrusted content".
- **An Occurrence** is named as now, by its Event and its Normal Form; only the Event's part changes from the row ID to the key.
- **One rule:** every operation and every type of the core interface uses the key, the Change Journal's listings and `UndoTarget::Event` included, although ticket 147 removes those. Trade-off, named: some of this work is thrown away two tickets later, but `EventId` then leaves the generated TypeScript types in one step and no caller is left that could hold a row ID.
- **The window:** the Window Model, Window Dialogs, the calendar grid and its drags (`src/lib/windowModel.svelte.ts`, `windowDialogs.svelte.ts`, `CalendarGrid.svelte`, `gridDrag.ts`) hold keys. Where FullCalendar needs a string ID, it is derived from the key and the Normal Form in one place. `src/test/ids.ts` makes keys for tests.
- **Inside the core** the row ID stays for now: the `event` table, the Change Journal and the Sender use it until tickets 145 and 147.
- An Event that is gone answers as today's unknown ID does. A key whose Event was deleted and created again names the new Event; that is intended, as the key is the Event's identity (ADR 0018).
- No change to what the user sees.

Spec: "Core application interface"; "Untrusted content". ADRs 0012, 0018 ("An Event's row ID changes with every edit").

**Blocked by:** none

**Status:** ready-for-agent

**Effort:** M

- [ ] No type of the core interface holds an Event Row's ID; `EventId` is gone from `src/core/types.generated.ts`
- [ ] Every operation that names an Event or an Occurrence takes the Combined Event Key, the Change Journal's listings and undo per Event included
- [ ] The window holds keys only and treats them as opaque; one function derives FullCalendar's string ID
- [ ] The core's tests through the core interface and the window's tests against the fake core pass with keys
- [ ] A test edits an Event twice through the key it got at creation

## Comments
- 2026-11-16: Written from the triage of the open tickets against ADRs 0017 to 0020. First of the tickets that build the Calendar History; the order is 142 to 154, then the tickets for the First Sync, the Write Permissions and the window.
