# 162: The Conflict bar, and the dialog when a Calendar stops Tracking

**What to build:** A Calendar with a Conflict isn't syncing, as a whole and in both directions (ADR 0019), and FreeCal is insistent about it: a bar across the top of the window while any Calendar has a Conflict, and a dialog, once, when a Calendar stops Tracking. Both lead to the Conflict list of ticket 137.

- **The bar** shows while any Calendar has a Conflict, read from the Conflict counts the core reports (ticket 133) and followed through "Conflicts changed". For one Calendar: "“Work” isn't syncing until you resolve 2 Conflicts". For several: how many Calendars and the total. Its button opens the Conflict list for all Calendars. It can't be dismissed and goes when the Conflicts end. It takes its height from the content below; nothing is covered.
- **The dialog** opens when a Calendar goes from Tracking to not Tracking. It names the Calendar and its number of Conflicts and offers "Resolve now", which opens the Conflict list with that Calendar's Conflicts, and "Later". It is a confirmation (`alertdialog`), so it has no ✕.
- **Once:** further Conflicts in a Calendar that has already stopped open no dialog. After "Later" or a restart of FreeCal it doesn't come back for that stop, and the bar stays. A Calendar that was Tracking again in between and stops again opens it again.
- **It waits:** it never opens over the Event editor or another dialog, or during a drag, and opens when that ends. With the window closed it opens when the window is next shown. Calendars that stopped in the meantime share one dialog, which lists them; if their Conflicts ended in the meantime, they aren't listed, and no dialog opens when none is left.
- **Where "stopped" comes from:** the window sees a Calendar's count go from 0 to more while it runs. Nothing is stored for it, which is why a restart doesn't bring the dialog back. A Calendar that already has Conflicts when the frontend starts opens none.
- **The Window Dialogs** (`src/lib/windowDialogs.svelte.ts`) hold the waiting and the sharing, beside their rule for which dialog may open over which.
- **Names** of Calendars come from a server and are shown as text only.

Spec: US-71, US-73; "A Calendar with a Conflict isn't syncing" under the frontend; "Signals". ADR 0019 ("Tracking and the Reference Snapshot", "Conflicts").

**Blocked by:** 133, 137

**Status:** ready-for-agent

**Effort:** M

- [ ] The bar shows while any Calendar has a Conflict, with the texts for one and for several Calendars, and goes when the last Conflict ends
- [ ] The bar's button opens the Conflict list; the bar has no way to dismiss it
- [ ] The dialog opens once when a Calendar stops Tracking; "Resolve now" opens that Calendar's Conflicts
- [ ] It doesn't open over the Event editor, another dialog or a drag, and opens afterwards
- [ ] Calendars that stopped while it couldn't open share one dialog; with the window closed it opens when the window is shown
- [ ] After "Later", further Conflicts in that Calendar open nothing; after a restart nothing opens and the bar shows
- [ ] The bar and the dialog are announced to screen readers once, not on every change of the count
- [ ] Component tests on the fake core for each rule

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. Blocked by tickets 133 and 137 as the triage rewrites them (the core reports Conflicts; the Conflict list for all Calendars or one), not as they read today; their rewrite is the triage's next batch.
