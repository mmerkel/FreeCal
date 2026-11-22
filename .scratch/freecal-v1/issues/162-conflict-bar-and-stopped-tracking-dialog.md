# 162: The Conflict bar and the toast when the server's version replaces a change

**What to build:** A change of the user's that the server's version replaced is never missed (ADR 0019, "Conflicts end at once"): a bar across the top of the window while any Calendar has an open Conflict, and a toast when one arises while the window is open. Both lead to the Conflict list of ticket 137. There is no dialog.

- **The bar** shows while any Calendar has an open Conflict, read from the counts the core reports (ticket 133) and followed through "Conflicts changed". For one Calendar: "2 changes in “Work” were replaced by the server's". For several: how many Calendars and the total. Its button "Review" opens the Conflict list for all Calendars. It can't be dismissed and goes when the Conflicts are resolved. It takes its height from the content below; nothing is covered.
- **The toast** appears when a Calendar's count rises while the window is shown: "Standup was replaced by the server's version", or for several "3 changes in “Work” were replaced by the server's", with "Review". It follows the toasts' rules (spec: "Toasts"). With the window closed, the shell's notification takes its place (ticket 23).
- **Where "rises" comes from:** the window compares a Calendar's count with the one it last saw while it runs. Nothing is stored for it; a Calendar that already has open Conflicts when the frontend starts shows the bar and no toast.
- **Names** of Calendars and Events come from a server and are shown as text only.

Spec: US-71, US-73, US-153; the Conflict bar under the frontend; "Toasts"; "Signals". ADR 0019 ("Conflicts end at once").

**Blocked by:** 133, 137

**Status:** ready-for-agent

**Effort:** S

- [ ] The bar shows while any Calendar has an open Conflict, with the texts for one and for several Calendars, and goes when the last is resolved
- [ ] "Review" opens the Conflict list; the bar has no way to dismiss it
- [ ] A toast with "Review" when a count rises while the window is shown; none at start
- [ ] The bar and the toast are announced to screen readers once, not on every change of the count
- [ ] Component tests on the fake core for each rule

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. Blocked by tickets 133 and 137 as the triage rewrites them (the core reports Conflicts; the Conflict list for all Calendars or one), not as they read today; their rewrite is the triage's next batch.
- 2026-11-21: Not Tracking now also covers Held during Undo, so the dialog is worded by Conflicts. Ticket 165 adds a line to the bar while the Calendar is also Held during Undo (grilling, ADR 0019).
- 2026-11-21: Held during Undo goes (ticket 165 `wontfix`, grilling on critique 1, ADR 0020 rewritten): Tracking means no Conflict again, and the bar has no line about redo.
- 2026-11-22: Rewritten from the grilling on critique 2 (ADR 0019 rewritten, spec US-153): no Calendar stops syncing, so the dialog when a Calendar stops Tracking goes, with its waiting and sharing; a toast takes its place while the window is open. Re-sized from M to S. The file name keeps the old title.
