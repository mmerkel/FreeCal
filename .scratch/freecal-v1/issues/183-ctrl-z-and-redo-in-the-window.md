# 183: Ctrl+Z and redo in the window: the toasts and the dialog asking first

**What to build:** The window's part of undo (ADR 0020), on the core functions and types of 180 and 181, which the window doesn't call yet. Old ticket 158 (`git show design/snapshot-line:.scratch/freecal-v1/issues/158-ctrl-z-and-redo-in-the-window.md`) as written. Ctrl+Z and redo work again in the window, which they haven't since 182. An Undo that can be made whole asks nothing.

- **Ctrl+Z and Ctrl+Shift+Z** call the core's Ctrl+Z and redo (180). A second Ctrl+Z while an Undo runs waits for it; a Snapshot arriving meanwhile (an Incoming Sync) doesn't change which Snapshot the running Undo takes back.
- **After an Undo:** "Undone: <what>" with "Redo", for 5 seconds. For a change on the server it names the Calendar, the time and the number of Events, and adds "also on the server" for a Calendar with a server. The window names an Incoming Sync "change on the server", never "remote" or "sync". "Redo" redoes exactly that Undo while it is the newest in effect, and is gone otherwise. **After a redo:** "Redone: <what>", without a button. **When nothing is left:** "Nothing to undo", "Nothing to redo"; an Undo that would change nothing shows its refusal as a toast.
- **The one core addition:** the number of Events the Snapshot changes in each Calendar, in what 180's functions report, for the toasts. Small, so it stays in this ticket rather than reopening 180.
- **Asking first** (ADR 0020, "Asking first"): where the core reports Events left out (181), a modal dialog names the Snapshot and lists them, each with why and "Open": “Standup was changed on the server since”, “… by "Resize Standup"” for the user's own, “Standup was replaced by the server's version” with "Restore my version" (the Undo of that Incoming Sync Snapshot limited to the Event; no replacement exists before 189, so the fake core tests it), ““Home” is read-only”, “its Calendar no longer exists”. A move with one side left out says what the other does: ““X” is read-only. Restore Standup in “Work”? It stays in “X” too.”, “The Calendar it was moved to no longer exists. Restore Standup in “Work”?”, “The Calendar it came from no longer exists. Delete Standup from “Home”?”. Buttons: "Undo the others" with their number, while any can be taken back; "Undo all over the later changes", while a Conflict leaves any out; "Cancel". A redo's dialog says "Redo" in their place. No button skips to the change before. The list scrolls for a large import.
- **What goes:** what is left of the old confirmation in `src/lib/UndoDialog.svelte`, `src/lib/undo.ts`, the undo and redo of the Window Dialogs (`src/lib/windowDialogs.svelte.ts`) and their texts in `src/i18n/`. The dialog isn't called "Undo History" (the glossary's Undo History is what Ctrl+Z steps through).
- **Untrusted content:** titles and Calendar names in the toasts and the dialog are stored fields, shown as text only.

Spec: US-146, US-150; "Calendar History"; "Toasts"; "Untrusted content". ADR 0020 ("Ctrl+Z", "Redo", "Asking first", "A move with one side that can't be written"). Layout: "By phase", P2, 183.

**Blocked by:** 182

**Status:** ready-for-agent

**Effort:** M

**Switched-off tests:** turns on and rewrites every test 182 switched off with "until ticket 183": `src/lib/UndoDialog.test.ts`'s remaining tests; `src/lib/windowDialogs.test.ts`'s Undo flow; `src/App.events.test.ts`'s undo toast and Ctrl+Z tests; `src/App.import.test.ts`'s "Undo undoes its Journal Transaction" (now its Snapshot) and "offers no Undo".

- [ ] The core reports the number of Events changed per Calendar for an Undo and a redo; core tests for the counts
- [ ] Ctrl+Z undoes at once; the toast names what was undone, a change on the server with Calendar, time and number of Events, and offers "Redo" for exactly that Undo; the redo toast has no button
- [ ] An Undo leaving Events out asks first in a dialog listing each with why (a later change, the server's or the user's own, a replacement, a read-only or removed Calendar, a half move with its consequence), with "Undo the others", "Undo all over the later changes" and "Cancel"; a redo's says "Redo"
- [ ] "Nothing to undo", "Nothing to redo" and an Undo that would change nothing show a toast
- [ ] A second Ctrl+Z during an Undo waits for it
- [ ] Nothing of the Dangerous Undo or the Change Journal's undo is left in the window or its texts
- [ ] Component tests on the fake core for each toast and each line of the dialog; the tests above on again
