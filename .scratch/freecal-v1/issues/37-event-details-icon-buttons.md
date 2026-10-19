# 37: Event details with icon buttons, ✕ on dialogs, safer delete and close

**What to build:** The Event details get Google-style icon buttons at the top right: Edit, Delete and ✕. Every plain dialog gets a ✕. Deleting an Event asks first, and closing the editor after changing something asks "Discard changes?". Spec: US-132–US-134, "Dialog close button", "Event details" and "Event editor" under Frontend.

**Blocked by:** none

**Status:** done

**Effort:** M

- [x] `Dialog` shows a ✕ ghost icon button at the top right for `role="dialog"` that calls `onclose`, like Escape; `alertdialog` shows none
- [x] `Dialog` has a maximum height within the window and its body scrolls
- [x] The details stay a centred modal `Dialog`. Above the title, a row of icon buttons at the top right: Edit (pencil), Delete (trash), Close (✕), Lucide icons; for Events of Read-only Calendars only ✕. The text button row is gone
- [x] Each icon button has a tooltip and an `aria-label` through `t`
- [x] A click on the backdrop closes the details (not the editor or confirmations)
- [x] Delete opens an `alertdialog` "Delete this Event?" naming the Event, with Cancel focused and a danger Delete; only Delete calls the core
- [x] The editor keeps its text Cancel and Save buttons. ✕, Cancel and Escape close it straight away when the draft is unchanged, and otherwise ask "Discard changes?" (Keep editing focused, Discard)
- [x] Focus on opening the details goes to ✕; focus returns afterwards as before
- [x] Component tests: icons per writable/read-only Event, ✕ and backdrop close, delete only after confirming, cancel keeps the Event, discard prompt only for a changed draft
- [x] Checked by hand in the real app with `run-freecal`

Out of scope: a popover anchored to the Event chip; an "Event deleted · Undo" toast instead of the confirmation, which needs undo for deletes (ticket 19 or later).

## Comments

- 2026-10-11: Ticket 33 added the Settings and About dialogs (`src/lib/SettingsDialog.svelte`, `src/lib/AboutDialog.svelte`), each with a single text Close button. Once `Dialog` shows the ✕, remove those Close buttons so both dialogs show only the ✕. That means `Dialog`'s `actions` snippet becomes optional, and a dialog without actions has no empty actions row. About's initial focus moves from Close to the ✕, like the details. Settings keeps its focus on the checked Appearance choice. Update the tests in `src/App.settings.test.ts` that click Close.
- 2026-10-19: Done. `Dialog` takes a `tools` snippet (icon buttons left of the ✕, in a row above the title) and `backdropCloses`; `onclose` became optional, and a dialog without it shows no ✕ and ignores Escape: only the "Working…" dialog, which can't be closed. Recurring Events ask "This event" / "All events" inside the new "Delete this event?" confirmation. The recurrence scope prompt (`ScopeDialog`) stays a plain `dialog`, so it now has a ✕ that does the same as its Cancel; making it an `alertdialog` without ✕ is open for a later decision. Checked by hand: details icons with ✕ focused, the delete question (Escape keeps the Event), backdrop close, the discard prompt, Settings with only ✕.
