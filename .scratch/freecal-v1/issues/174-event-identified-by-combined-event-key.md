# 174: An Event is identified by its Combined Event Key

**What to build:** Old ticket 142 as written (`git show design/snapshot-line:.scratch/freecal-v1/issues/142-event-identified-by-combined-event-key.md`): the core interface and the window name an Event by its Combined Event Key, never by the Event Row's ID (ADR 0018, "Event Versions": a row ID changes with every edit). Nothing in the Local Store changes.

- Every operation of `Core` that names an Event or an Occurrence (reading, editing, rescheduling and deleting them, exception counts, the original server data, an Event's History, the Change Journal's listings and undo per Event) takes the key; `Event` and `Occurrence` carry it. One rule for every operation, the ones 182 deletes included, so that `EventId` leaves the generated types in one step.
- The key is branded in TypeScript and opaque to the window: no part of it is compared, parsed or shown there. One function derives FullCalendar's string ID from the key and the Occurrence's Normal Form. `src/test/ids.ts` makes keys.
- Inside the core the row ID stays (`pub(crate)`) until 178 makes it the Event Version's row ID.
- An Event that is gone answers as an unknown ID does today. A key whose Event was deleted and created again names the new Event.
- No change to what the user sees.

Spec: "Core application interface"; "Untrusted content". ADRs 0012, 0018. Layout: "Today's code, in brief" (`EventId`, Core interface); "By phase", P1, 174.

**Blocked by:** none

**Status:** ready-for-agent

**Effort:** M

- [ ] No type of the core interface holds an Event Row's ID; `EventId` is gone from `src/core/types.generated.ts`
- [ ] Every operation that names an Event or an Occurrence takes the key
- [ ] The window holds keys only; one function derives FullCalendar's string ID
- [ ] `core/tests/support/` helpers (`yoga()` among them) return keys; the tests through the core interface and the window's tests pass with keys, changed only mechanically
- [ ] A test edits an Event twice through the key it got at creation
