# 88: Exceptions without their Recurring Event

**What to build:** An object or `.ics` file that holds Exceptions without their Recurring Event, such as an invitation to one Occurrence, is kept: each Exception is an Event of its own, as GLOSSARY has it, and joins its Recurring Event when that arrives. Today `import::assemble` leaves them out, so they never appear.

**Blocked by:** 87

**Status:** done

**Effort:** M

- [x] From a server, Exceptions without their Recurring Event are stored with their recurrence IDs and their object; they show in the grid at their own times and open as Events of their own
- [x] From an import, the same; an import that later brings the Recurring Event (or a sync whose object now holds it) pairs them with its Occurrences by recurrence ID, and an Exception that matches no Occurrence stays an Event of its own. Where the Recurring Event brings its own Exception for the same Occurrence, that one replaces the standalone one, so the Occurrence has one Exception (ticket 87's rule)
- [x] Their recurrence IDs are normalised as for any Exception; with no Recurring Event to take the zone from, a timed one keeps the zone it was given in, and is normalised again when it joins
- [x] Core tests for each point, from a server and from a file

Out of scope: editing them as Occurrences before their Recurring Event arrives (they edit as single Events).

## Comments

- 2026-10-25 (from 87): `sync::object_events` leaves out an Exception whose Recurring Event is neither in the same batch nor stored. Since 87 this also hits a first sync where an Exception object comes in a page before its Recurring Event's: it isn't mirrored, and with a sync token it isn't fetched again until it changes. A known object whose new version holds only such Exceptions loses its rows and leaves the mirror. Storing them as Events of their own fixes both; normalising against a stored Recurring Event goes through `Loaded::recurring_when` and `ImportedEvent::recurrence_id_in`.
- 2026-10-25: Implemented except one point of box 2, left open for a decision. From a file, the Recurring Event's own Exception for an Occurrence replaces the stored one; from a server, when it is in the same object as the one it replaces. When the two are in different objects, ADR 0010 doesn't let one object delete another's rows. Following ticket 87, both are kept and the first by ID shows, which is the earlier, standalone one, not the Recurring Event's. Making the Recurring Event's own one show instead would change 87's "first by ID" rule.
  - Stored format: an Exception without its Recurring Event stores its recurrence ID as given: a date, a floating wall time, or a wall time with its zone, `2026-10-12T03:00:00[America/New_York]` (`rule::id_of_original` / `original_of_id`). `Loaded::join` normalises it again when a Recurring Event with its UID arrives by import or sync. Two of them for one Occurrence, given in different zones, pair by text, not by instant, until they join.
  - Events of their own: such an Exception, and one that changes none of its Recurring Event's Occurrences (`Loaded::of_its_own`), goes to the window with no recurrence ID on its Occurrence. So it edits, moves and deletes as a single Event: deleting it leaves the Recurring Event, its details show no repeat rule, and another import of the Recurring Event keeps it. `Event.recurrence_id` stays set, so the editor still offers no repeat for it.
  - Import: `ImportPreview.events` lists the Exceptions without their Recurring Event after the other Events, each an Event of its own. `run` decides skip, update or copy once per UID. Whether a UID exists now also counts Exceptions, as the preview already did. The latest by `SEQUENCE` is kept for one Occurrence.
  - Sync: such an object is mirrored, which also fixes the page-order and "loses its rows" cases in the comment above. GLOSSARY's entry is widened to match.
- 2026-10-25: Decided: ADR 0010 stands. When the Recurring Event's own Exception and a standalone one for the same Occurrence are in different server objects, both keep their rows and the first by ID shows (ticket 87). Box 2 is done on that reading. Done.
- 2026-10-25 (grilling, ADR 0011): the decision above is reversed by ticket 91. Of several objects in a Calendar holding one UID, the one holding the Recurring Event owns it; the others are mirrored without rows. A standalone Exception in another object than its Recurring Event's is therefore not shown.
