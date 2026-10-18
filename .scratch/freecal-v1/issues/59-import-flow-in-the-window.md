# 59: Import flow in the window

**What to build:** The user picks a file and a writable Calendar, sees the preview, chooses once what to do with existing Events, confirms, and can undo the import afterwards. The import runs under the modal "Working…" dialog. Spec: US-94–US-100. Split from ticket 10.

**Blocked by:** 58

**Status:** done

**Effort:** M

- [x] "Import…" opens a file dialog through a portal-friendly API, for `.ics` and `.zip`
- [x] The user chooses the target Calendar; Read-only Calendars aren't offered
- [x] The preview dialog shows the counts of new and existing Events and the zones floating times were pinned to, and asks for the update / skip / copy choice only when some exist
- [x] Confirming runs the import under the modal "Working…" dialog (share it with ticket 09; whichever comes first builds it)
- [x] After the import, the user can undo it in one step (decide the form here, e.g. "Imported 42 Events · Undo"; ticket 19's list comes later)
- [x] File names, error texts and Event fields are shown only as text
- [x] Every string goes through `t()`
- [x] Component tests against the fake core; checked by hand in the real app with `run-freecal`, importing the hostile fixture

Out of scope: opening files with FreeCal (ticket 30).

## Comments

- 2026-10-13: Split from ticket 10.
- 2026-10-17: Done. Main menu → Import… (first item). Decisions:
  - The shell opens the file dialog (tauri-plugin-dialog, xdg-portal backend) and reads and holds the file and its latest preview (`PickedImport` in `main.rs`); the window only gets the file's name and the counts (`ImportSummary`). Commands: `pick_import_file`, `preview_import`, `run_import` (re-previews when the held preview is for another Calendar), `discard_import`, `undo_import`. A debug build takes `FREECAL_IMPORT_FILE` in place of the dialog, for the agent's Xvfb session (run-freecal skill).
  - One dialog: the Calendar select (writable Calendars only, defaulting to the first shown one, following a Calendar deleted or made Read-only meanwhile), the counts and pinned zones, and update / skip / copy only when some exist, with "Update them" chosen by default.
  - Picking and previewing don't run under "Working…" (they change nothing; the dialog says "Reading the file…"). Running and undoing do. `WorkingDialog.svelte` is ticket 09's to reuse.
  - Undo form: a message at the bottom of the window, "Imported “work.ics”: 40 added, 2 updated, 0 skipped." with Undo and ✕ (`Toast.svelte`). It stays until dismissed or replaced; Undo only when something was added or updated; a refused undo keeps Undo, to try again. Dismissed, the import can only be undone through ticket 19's list.
  - File names in messages are bidi isolates (U+2068…U+2069): the hostile name's RTL override otherwise reversed the counts after it. Written into the spec's "Untrusted content"; it also applies to the existing `error.import*` messages.
  - A `.zip` goes into one Calendar, as tickets 57 and 58 built it; one Calendar per `.ics` in a Takeout `.zip` (story 95) isn't done.
  - The preview doesn't warn that a Recurring Event imports as its first Occurrence only; noted on ticket 07.
  - Checked by hand with `run-freecal` importing an `.ics` of the hostile test Event (preview, import, re-import with the choice, Undo).
