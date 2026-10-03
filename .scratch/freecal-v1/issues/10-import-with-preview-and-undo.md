# 10: Import .ics and .zip with preview and undo

**What to build:** The user can import an `.ics` file, or a `.zip` of `.ics` files such as a Google Takeout export, into any writable Calendar. A preview shows how many Events are new and how many already exist; when some exist (same UID) the user chooses once whether to update, skip or import them as copies. The whole import can be undone in one step, including imports into Local Calendars.

**Blocked by:** 03

**Status:** ready-for-agent

- [ ] Import accepts `.ics` and `.zip` of `.ics`; CSV is not supported
- [ ] The preview shows counts of new and existing Events
- [ ] One update / skip / copy choice per import for existing UIDs
- [ ] Import is not offered for Read-only Calendars
- [ ] The import is recorded as one Change Journal unit holding each Event's previous version, and can be undone in one step, restoring the Calendar exactly
- [ ] The import runs under the modal "Working…" dialog and sends "Occurrences changed" for the affected Calendars
