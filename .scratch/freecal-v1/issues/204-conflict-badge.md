# 204: The Conflict badge on Event chips

**What to build:** An Event in Conflict (190) carries the Conflict badge on its chips in every view, as the chip's only mark (spec: "Event chips stay sparse"). Old ticket 136 (`git show design/snapshot-line:.scratch/freecal-v1/issues/136-conflict-badge.md`) as written.

- **The core's one addition:** the Occurrence type the core interface returns for a range gains whether its Calendar and UID have an open Conflict. Nothing new is stored: it is read from 190's `conflict` rows without `resolved_at` (a whole-UID Conflict covers every single Event of the UID, so `conflict_event` isn't needed for it); 190 records Conflicts but doesn't put this on the Occurrence. 190 already sends "Occurrences changed" when a Conflict arises or ends, so the window reads the range again by its usual rule. Regenerate the TypeScript types and give the fake core its answer.
- **The chip:** the badge sits at the chip's right edge in the day, week and month views, takes room from the title and is never dropped, on small chips too. It wins over the Unsent Mark (202). It uses the Conflict Status Mark's drawing, so chip, sidebar and status indicator point to the same state, and has an accessible name through `t`.
- **Still editable:** an Event in Conflict moves and resizes as any other, which resolves the Conflict (191), so the grid's read-only hint (`whyReadOnly`) doesn't change.
- **No chip:** an Event the user edited and the server deleted is gone from the grid; the Conflict bar and list (206, 205) lead to it. An Event the user deleted and the server edited shows again, with the badge.

Spec: US-33, US-71; "Event chips stay sparse"; "Signals". ADR 0019 ("Conflicts end at once"). Layout: "By phase", "P5 and P6".

**Blocked by:** 190

**Status:** ready-for-agent

**Effort:** S

- [ ] Occurrences say whether their Event is in Conflict (a core test through the interface, after an Incoming Sync with a Conflict and after resolving it); the TypeScript types are regenerated
- [ ] An Event in Conflict shows the badge on its chips in every view, as the chip's only mark, on small chips too
- [ ] Component tests on the fake core; one look in the real window in light and dark (`run-freecal`)
