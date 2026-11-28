# 217: An Exception created on both sides is compared with its Occurrence

**What to build:** Where both sides created an Exception for one Occurrence that what both are compared with has none of (177's decision 6: today every Field Group counts as changed on both, so it is always a Conflict), the Event Comparison compares both with the Occurrence as the Recurring Event in the base gives it, by Field Group like any edit. So one side's new title and the other's new Location for the same Occurrence of a series combine in a Server Event Merge (198) or an Undo, and only real clashes are Conflicts. Found in 177's build (2026-11-28, with the user).

- **The stand-in base:** the fields an Exception for that Occurrence starts with, built by the code that makes them for an edit of one Occurrence (`Changing::occurrence`, `core/src/event/scope.rs:282`), shared, never copied: the Recurring Event's managed fields with the Occurrence's times, its recurrence ID in the Normal Form, no rule and no excluded dates. Built inside `compare_event` (`core/src/comparison/mod.rs`) from the base's Recurring Event; none where the base has no Recurring Event, or its rule doesn't give the Occurrence (an excluded date, a rule FreeCal can't expand): then it stays a Conflict as today.
- **The tree:** the stand-in's Unimplemented Fields are what a new Exception made by FreeCal holds (check what 186 decided: if a new Exception copies the Recurring Event's alarms, so does the stand-in). Otherwise every new Exception looks like a change to the Unimplemented Fields on both sides and clashes there.
- **Nothing else changes:** the Recurring Event changed against an Exception on the other side stays a Conflict of the UID; one side's creation alone stays that side's version.

Spec: "Sync engine". ADRs 0019 ("Whether the server changed"), 0020 ("Where both changed"). Layout: "The comparison: `compare_event`".

**Blocked by:** 198

**Status:** ready-for-agent

**Effort:** S

- [ ] An Exception's new title on one side against the same Occurrence's new Location on the other, neither in the base: combined
- [ ] The same Occurrence's new title on both sides, differently: a Conflict; its new start against the other's new title: a Conflict
- [ ] An Occurrence the base's rule doesn't give, or a base without a Recurring Event: a Conflict, as before
- [ ] A new Exception made by FreeCal through each scope compares unchanged against its stand-in in every Field Group but those the edit changed (test through the scope edit and the comparison)
- [ ] 177's decision 6 in its ticket comment says 217 replaced it
