# 19: Undo through the Change Journal

**What to build:** The user can see the changes FreeCal sent to servers, undo any one of them, or undo everything FreeCal changed in a Calendar since a given time. Undo restores the previous server version under the same identity, with all details intact. Entries are kept for 90 days.

**Blocked by:** 10, 17

**Status:** ready-for-agent

- [ ] List Change Journal entries
- [ ] Undo one entry: the previous version is written back under the same identity (original ID, all details)
- [ ] Undo everything for a Calendar since a given time
- [ ] Undo an import into a server Calendar in one step (extends the Journal unit from 10)
- [ ] Entries older than 90 days are removed (controllable clock)
- [ ] Undo sends "Occurrences changed"
