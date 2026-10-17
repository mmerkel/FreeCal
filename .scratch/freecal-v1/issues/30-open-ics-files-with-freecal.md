# 30: Open .ics and .zip files with FreeCal

**What to build:** The user can open an `.ics` or `.zip` file with FreeCal, from the command line or through "Open with" (for example an invitation attached to an email). Whether FreeCal was running or not, the file goes through ticket 59's import flow as one undoable import. Spec: user stories 118–120, "Opening files" under Import/export, the "Waiting imports changed" Signal and the "File hand-off" desktop interface.

**Blocked by:** 29, 59

**Status:** ready-for-agent

**Effort:** M

- [ ] Command-line arguments at first launch, arguments forwarded by a second launch, and macOS open-file events (`RunEvent::Opened`) all feed one core operation that adds waiting imports
- [ ] The core reads each file's contents on arrival (mail clients may delete their temporary copy) and keeps waiting imports in arrival order until they are imported or dismissed; each change sends "Waiting imports changed"
- [ ] A missing, unreadable or unparseable file, or a `.zip` without `.ics` files, becomes a waiting import with a translated error naming the file; the other files still import
- [ ] Several files in one launch give one import per file
- [ ] The frontend opens the import preview for the first waiting import only when no modal dialog ("Working…" or another preview) is showing; an open Event editor is not interrupted
- [ ] The `.desktop` file registers FreeCal as a handler for `text/calendar` (and `.zip` is accepted from the command line); check how files arrive through the document portal so that the Flathub build works
- [ ] Tested at the core seam (contents read on arrival, arrival order, error entries, Signals) and in the frontend against the fake core (the queue rule); the hand-off paths are checked by hand

## Comments

- 2026-10-17 (from ticket 59): The picked file is held in the shell, one at a time (`PickedImport` in `src-tauri/src/main.rs`), not in the core. Waiting imports belong in the core; move the picked file there too, so that both go through one queue and one preview / run / discard.
