# 59: Import flow in the window

**What to build:** The user picks a file and a writable Calendar, sees the preview, chooses once what to do with existing Events, confirms, and can undo the import afterwards. The import runs under the modal "Working…" dialog. Spec: user stories 94–100. Split from ticket 10.

**Blocked by:** 58

**Status:** ready-for-agent

- [ ] "Import…" opens a file dialog through a portal-friendly API, for `.ics` and `.zip`
- [ ] The user chooses the target Calendar; Read-only Calendars aren't offered
- [ ] The preview dialog shows the counts of new and existing Events and the zones floating times were pinned to, and asks for the update / skip / copy choice only when some exist
- [ ] Confirming runs the import under the modal "Working…" dialog (share it with ticket 09; whichever comes first builds it)
- [ ] After the import, the user can undo it in one step (decide the form here, e.g. "Imported 42 Events · Undo"; ticket 19's list comes later)
- [ ] File names, error texts and Event fields are shown only as text
- [ ] Every string goes through `t()`
- [ ] Component tests against the fake core; checked by hand in the real app with `run-freecal`, importing the hostile fixture

Out of scope: opening files with FreeCal (ticket 30).

## Comments

- 2026-10-13: Split from ticket 10.
