# 69: Change Journal for every local change, undo per Event

**What to build:** Every change FreeCal makes to an Event is a Change Journal entry, in any Calendar, Local Calendars included, and undo goes per Event as ADR 0009 describes. This replaces ticket 58's import undo, which matched Events by ID and UID. Spec: user stories 81, 82, 99 and 135; "Change Journal" under Implementation Decisions.

**Blocked by:** none

**Status:** ready-for-agent

**Effort:** L

- [ ] Creating, editing, moving and deleting an Event in any Calendar records an entry with the previous version and a hash of the new one; ticket 58's `journal_import` / `journal_entry` grow into this (rename to a general unit)
- [ ] Imports, Snapshot restores (ticket 11) and Conflict resolutions (tickets 20 and 21) are units; a single change is a unit of one
- [ ] Undoing an entry writes the previous version back under the same identity, and is itself recorded as an entry, so it can be undone; undone entries are marked
- [ ] An Event that no longer matches the entry's hash has its later entries undone first, newest first; the core reports them before running, so the frontend can ask (ticket 70)
- [ ] A unit can be undone whole or one Event at a time; undoing the rest of a partly undone unit works
- [ ] Events in Read-only Calendars are skipped and their Calendars named; the rest is undone (replaces ticket 58's refusal of the whole undo)
- [ ] An Event whose current version matches no entry is skipped with "Can't undo “<title>”: its current version isn't in the Change Journal of the Local Store." (title as text in bidi isolates, through `t()`)
- [ ] The core classifies an undo as benign or dangerous (also undoes later changes, reverts a change pulled from a server, or exceeds the Mass-Change Guard's thresholds) and says why
- [ ] Undo and redo send "Occurrences changed" for the affected Calendars
- [ ] Entries older than 90 days are removed (controllable clock); moved here from ticket 19
- [ ] Core tests for every point above, including ticket 58's cases: an imported Event moved since, an updated one edited or deleted since

Out of scope: recording incremental syncs (tickets 54 and 25); server writes of an undo (ticket 17); Ctrl+Z, toasts and confirmations (ticket 70); the list and History (ticket 19).

## Comments

- 2026-10-18: From grilling ticket 58's decisions (TODO.md). ADR 0009 records why.
