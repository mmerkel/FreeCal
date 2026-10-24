# 87: Sync matches server objects by href

**What to build:** Incoming changes are matched to the server objects FreeCal holds by href, and each object updates only its own rows (ADR 0010). The UID still groups a Calendar's rows into a Recurring Event with its Exceptions.

**Blocked by:** 86

**Status:** ready-for-agent

**Effort:** M

- [ ] A changed object replaces its own rows with what it now holds, whatever their UIDs; a new object adds its rows; a removed object takes its rows with it. `Store::event_at` and the UID-change special case in `sync::apply` go
- [ ] Replacing an object's rows touches only that object's rows: `Loaded::take`, which replaces the whole UID group and deletes every stored Exception the incoming version lacks, isn't used as it is, since in Google each Exception is an object of its own and would be deleted with every change to its Recurring Event
- [ ] At most one Exception per recurrence ID per object, enforced where `Loaded` writes Exceptions, not by the schema: iCalendar allows several components with one UID and RECURRENCE-ID (revisions told apart by SEQUENCE), and a unique index would turn a server's harmless duplicate into a Sync Error. Of several in one object, the one with the highest SEQUENCE is kept (today `import::assemble` keeps the first). Where two objects with one UID each have one, both keep their rows and the first by ID is shown; reading stays tolerant of duplicates from another program
- [ ] An object FreeCal can no longer read keeps its last readable version in the mirror and its rows as they were, and is recorded as unreadable, so that it can be shown as a problem later; it is fetched again on each sync until it reads
- [ ] An object moved to another href on the server is still changed, not deleted and added again
- [ ] Core tests: two objects with one UID on a server without sync tokens no longer take each other's rows on every sync (each sync recorded no unit); a Google-like Recurring Event whose Exceptions are objects of their own keeps them when only its object changes; two components with one recurrence ID in an object; an object whose UID changes; an unreadable object

Out of scope: Exceptions without their Recurring Event (ticket 88); a server change arriving while an object is pending (ticket 20).
