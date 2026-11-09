# 136: The Conflict badge on Event chips

**What to build:** The fourth part of ticket 20 (read it for the history). An Event in Conflict (ticket 133) carries a Conflict badge on its chips in every view. It is the only badge on a chip (spec: "Event chips stay sparse").

- **Core:** each Occurrence the core returns for a range says whether its Event is in Conflict. Ticket 133 already sends "Occurrences changed" for a Calendar whose Conflict was raised, replaced or resolved, so the frontend reads the range again by its usual rule. Regenerate the TypeScript types (`FREECAL_WRITE_TYPES=1 cargo test -p freecal-core`).
- **Chip:** the badge sits on the chip in the day, week and month views, next to the dashed outline for unsent changes, which an Event in Conflict also has. Use the toolbar's Conflict icon (`TriangleAlert`, `src/lib/Toolbar.svelte`), so both point to the same state.
- **Still editable:** an Event in Conflict moves and resizes as any other (ticket 138), so the grid's read-only hint (`whyReadOnly`) doesn't change.
- **Not on a chip:** an Event deleted locally whose server version was edited has no Event Rows, so there is no chip to badge. The overall status and the Conflicts list (ticket 137) lead to it.

Spec: US-33, US-71; "Event chips stay sparse"; "Signals".

**Blocked by:** 133

**Status:** ready-for-agent

**Effort:** S

- [ ] Occurrences say whether their Event is in Conflict
- [ ] An Event in Conflict shows the Conflict badge on its chips in every view, as the only badge
- [ ] Tested on the recording fake core and in the real window

## Comments
- 2026-11-09: Split from ticket 20.
- 2026-11-09 (grilling): "Occurrences changed" for a Conflict now comes from ticket 133. An Event in Conflict stays editable.
