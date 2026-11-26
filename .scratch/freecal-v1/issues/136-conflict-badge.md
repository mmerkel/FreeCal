# 136: The Conflict badge on Event chips

**What to build:** An Event in Conflict (ticket 133) carries a Conflict badge on its chips in every view. A chip carries at most one mark, and the badge is it (spec: "Event chips stay sparse").

- **Core:** each Occurrence the core returns for a range says whether its Event is in Conflict: whether its Calendar and UID have an open Conflict (ticket 133), read from the rows of `conflict` without a resolved time. Ticket 133 sends "Occurrences changed" for a Calendar whose Conflicts changed, so the frontend reads the range again by its usual rule. Regenerate the TypeScript types (`FREECAL_WRITE_TYPES=1 cargo test -p freecal-core`).
- **Chip:** the badge sits at the chip's right edge in the day, week and month views, takes room from the title and is never dropped, on small chips too. An Occurrence in Conflict shows the Server Version, which has nothing unsent; if it has an Unsent Change for another reason, the badge wins over the Unsent Mark (ticket 161). Use the Conflict Status Mark's drawing, so the chip, the sidebar and the status indicator point to the same state.
- **Still editable:** an Event in Conflict moves and resizes as any other, which resolves the Conflict (ticket 134), so the grid's read-only hint (`whyReadOnly`) doesn't change.
- **Not on a chip:** an Event the user edited and the server deleted is gone from the grid, so there is no chip to badge. (An Event the user deleted and the server edited is shown again with the server's edit and carries the badge.) The Conflict bar and the Conflict list (tickets 162, 137) lead to it.

Spec: US-33, US-71; "Event chips stay sparse"; "Signals".

**Blocked by:** 133

**Status:** ready-for-agent

**Effort:** S

- [ ] Occurrences say whether their Event is in Conflict
- [ ] An Event in Conflict shows the Conflict badge on its chips in every view, as the chip's only mark
- [ ] Tested on the recording fake core and in the real window

## Comments
- 2026-11-09: Split from ticket 20.
- 2026-11-09 (grilling): "Occurrences changed" for a Conflict now comes from ticket 133. An Event in Conflict stays editable.
- 2026-11-16: Reworded from the triage against ADRs 0017 to 0020: the badge is the chip's only mark (the dashed outline became the Unsent Mark, which gives way to it), and "in Conflict" is read from ticket 133's list.
- 2026-11-22: Reworded from the grilling on critique 2 (ADR 0019 rewritten): an Event in Conflict shows the Server Version; the badge stays until the Conflict is resolved.
- 2026-11-26: Reworded from the discussion of point 7 of `.scratch/freecal-v1/critiques-2.md` (ADRs 0018 to 0020): a Conflict is a row of `conflict`.
