# 165: Held during Undo: the server's changes wait while redo is possible

**What to build:** Since ticket 152 a Calendar is Held during Undo while its User's Pointer has a redo path and its Server's Pointer is off the Reference Snapshot: the server's changes have been fetched and wait, so that an Incoming Sync doesn't take redo away (ADR 0019, "Held during Undo"; ADR 0020). This ticket lets the user see it and end it.

- **The core reports** per Calendar whether it is Held during Undo, derived from the Calendar History as ticket 152 reads it, and signals a change of it beside "Conflicts changed". Nothing is stored for it.
- **"Continue"** is a call that ends the redo path of one Calendar: it drops the Snapshots beyond its User's Pointer, as a change of the user would (ticket 147), and runs the Incoming Sync's second step (ticket 152) in the same Database Transaction, which follows, makes a Join or finds a Conflict. A sync is asked for afterwards.
- **The Status Mark:** a Calendar Held during Undo without a Conflict carries a Status Mark of its own on its line (ticket 161's rule, after Conflict in its order). With a Conflict too, only the Conflict's mark shows.
- **The bar,** beside ticket 162's and in its style, shows while any Calendar is Held during Undo without a Conflict: "The server's changes to “Work” wait while you can still redo." with "Continue". For several Calendars: how many, and "Continue" for all. It can't be dismissed; it goes when the redo path ends.
- **With a Conflict at the same time,** ticket 162's bar adds a line for that Calendar: "Resolving them also ends what you could redo." and the Held bar doesn't show it.
- **Not in the overall status** of the tray icon and the status indicator: holding is the user's own doing in the window. For review.
- **Tests,** through the core interface against Radicale: an undo, then a change on the server, holds the Calendar and sends nothing; redo to the end, a change of the user and "Continue" each end it and take the server's changes in; an undo before any server change is sent at once, and nothing holds; a Conflict found while held shows at once, and ending it ends the hold; the Store Check passes after each. In the window, on the recording fake core (ticket 116): the mark, the bar, its texts for one and several Calendars, and the line in the Conflict bar. All text in `src/i18n/en.ts`; Calendar names are untrusted text.

Spec: "Calendar History" (undo and redo) and "Sync engine" (Tracking). ADR 0019 ("Held during Undo", "Tracking and the Reference Snapshot"), ADR 0020.

**Blocked by:** 152, 161, 162

**Status:** ready-for-agent

**Effort:** M

- [ ] The core reports and signals Held during Undo per Calendar, derived, nothing stored
- [ ] "Continue" ends one Calendar's redo path and runs the second step, in one Database Transaction
- [ ] The Calendar's line has the Held during Undo Status Mark, and only the Conflict's while there is a Conflict too
- [ ] The bar shows with "Continue" while a Calendar is Held during Undo without a Conflict, and goes when the redo path ends
- [ ] The Conflict bar adds that resolving ends the redo, for a Calendar that is also Held during Undo
- [ ] The tests listed above

## Comments
- 2026-11-21: Written from the grilling of 2026-11-21 (ADR 0019, "Held during Undo"). The rule itself (the second step waits while a redo path exists) is in ticket 152; this ticket adds what shows it and ends it. Left out of the overall status, for review.
