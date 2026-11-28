# 206: The Conflict bar, and the toasts when the server's version replaces or combines a change

**What to build:** A change of the user's that the server's version replaced is never missed (ADR 0019, "Conflicts end at once"), and a Server Event Merge is announced once. Old ticket 162 (`git show design/snapshot-line:.scratch/freecal-v1/issues/162-conflict-bar-and-toast.md`) as written, plus old 168's toast (`git show design/snapshot-line:.scratch/freecal-v1/issues/168-server-event-merge.md`), on 198's Signal.

- **The Conflict bar** shows while any Calendar has an open Conflict, from the counts in "Sync status changed" (190): “2 changes in “Work” were replaced by the server's”, or for several Calendars how many and the total. "Review" opens the Conflict list for all Calendars (205). It can't be dismissed and goes when the last is resolved. It reuses 203's bar component; with both bars, the Conflict bar comes first.
- **The Conflict toast** when a Calendar's count rises while the window is shown: “Standup was replaced by the server's version”, or “3 changes in “Work” were replaced by the server's”, with "Review". The window compares with the count it last saw while it runs; nothing is stored, so a count already open at start shows the bar and no toast. With the window closed, the shell's notification is ticket 23's.
- **The merge toast,** once per Incoming Sync whose Signal reports Server Event Merges (198): for one, “Standup was combined with a change on the server” with "Undo" (181's Undo of that Incoming Sync Snapshot limited to the Event, with 183's toasts and dialog) and "Open"; for several, “3 Events were combined with changes on the server” with "Show", which opens the Calendar History at that Incoming Sync once 207 exists (until then "Show" is absent). No desktop notification.
- **Toasts' rules** (spec: "Toasts"); the bar and the toasts are announced to screen readers once, not on every change of a count. Names of Calendars and Events are untrusted text.

Spec: US-71, US-73, US-153; the Conflict bar under the frontend; "Toasts"; "Signals"; "Untrusted content". ADR 0019 ("Conflicts end at once", "A Server Event Merge"). Layout: "By phase", "P5 and P6".

**Blocked by:** 198, 205

**Status:** ready-for-agent

**Effort:** S

- [ ] The bar shows while any Calendar has an open Conflict, with the texts for one and several Calendars, goes with the last, and can't be dismissed
- [ ] "Review" opens the Conflict list
- [ ] A toast with "Review" when a count rises while the window is shown; none at start
- [ ] A toast after an Incoming Sync with Server Event Merges: "Undo" and "Open" for one, a count for several
- [ ] Announced to screen readers once
- [ ] Component tests on the fake core for each rule
