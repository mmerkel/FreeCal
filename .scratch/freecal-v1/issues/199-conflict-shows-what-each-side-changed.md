# 199: A Conflict shows what each side changed

**What to build:** Each Conflict the core lists (190) also says, per Field Group, what each side changed against the Last Agreed Version kept in its Conflict Events (ADR 0019, "The Conflict tables"); without one, where the two sides differ. Old ticket 135 (`git show design/snapshot-line:.scratch/freecal-v1/issues/135-conflict-shows-what-each-side-changed.md`) as written, with old 21's Occurrences named per side.

- **Computed on reading, never stored** (`history/conflict.rs`): per Conflict Event, the User's Version and the Server Version are each compared with `last_agreed` by 177's one function for Field Groups.
- **What each side changed,** as values, not columns: time (start, end, all-day and time zone together), title, location, description, recurrence. Only managed fields are named; a change to the Unimplemented Fields alone is named as such ("changed details FreeCal doesn't show" is the window's wording). A side may list nothing for an Event, where the other side's change met one of its own elsewhere in the UID.
- **A Recurring Event** (old 21): the fields of the Recurring Event as for any Event, and each Occurrence whose Exception that side changed, created or removed, by its date.
- **Without a Last Agreed Version** (both sides created the UID, or 195's untrusted base), or an Event on one side only: the two versions are compared with each other, and the fields where they differ are listed without saying which side changed them. **A side without the Event** (deleted there) says only that.
- **Types:** the words are 205's. The core returns the changed Field Groups as values in the list's types; regenerate the TypeScript types and give the fake core its answer.

Spec: US-73; "Core application interface"; "Untrusted content". ADR 0019 ("The Conflict tables", "Conflicts"). Layout: "The target module tree" (`history/conflict.rs`); "By phase", P4, 199.

**Blocked by:** 190

**Status:** ready-for-agent

**Effort:** S

- [ ] Each listed Conflict says, per Event of the UID and per Field Group, what each side changed against the Last Agreed Version, computed on reading
- [ ] For a Recurring Event, each side names the Occurrences whose Exceptions it changed, created or removed
- [ ] Without a Last Agreed Version, it says where the two sides differ
- [ ] A side without the Event says it was deleted there; changes only to Unimplemented Fields are named as such
- [ ] Written test-first through the core interface
