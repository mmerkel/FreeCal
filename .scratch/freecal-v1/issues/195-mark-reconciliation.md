# 195: Mark Reconciliation and the untrusted base

**What to build:** Once a week per server Calendar its next Incoming Sync is a Full Fetch, which checks the server's Current-Version Marks against what the server holds (ADR 0019, "Whether the server changed", "An untrusted base"). Old ticket 171 (`git show design/snapshot-line:.scratch/freecal-v1/issues/171-mark-reconciliation.md`) as written.

- **When:** `last_full_fetch` beside the Calendar's sync token, set by every Full Fetch (194's First Sync, a sync-token reset, Mark Reconciliation). An Incoming Sync that starts while it is more than 7 days old (by the core's Clock) is a Full Fetch, in the background like any. A server without ETags already fetches everything; it has no Mark Reconciliation of its own.
- **The untrusted base,** a case of the Incoming Sync Plan (`sync/plan.rs`), on every Full Fetch: a fetched resource whose ETag equals the stored one but whose content differs from the versions with the server's mark. The server's mark moves to the fetched version; an Occurrence whose current user's version equals it agrees; any other UID is a Server Event Replacement whose Conflict Events name no Last Agreed Version (190). The plan marks such a UID so that 198's Server Event Merge never combines it. One Log line names the Calendar and UID, no values ("Untrusted content"). No Sync Error, toast or origin of its own.
- **Everything else** of a Full Fetch (a changed ETag, a resource gone, one with no server's mark) goes through the plan as usual.

Spec: "Sync engine". ADR 0019 ("Whether the server changed", "An untrusted base"). Layout: "Tables (end state)" (`calendar`); "Seams for `/tdd`", 1; "By phase", P3, 195.

**Blocked by:** 190, 194

**Status:** ready-for-agent

**Effort:** S

- [ ] An Incoming Sync of a Calendar whose last Full Fetch is more than 7 days old fetches every resource; one within 7 days fetches only what changed
- [ ] Every kind of Full Fetch sets the time of the last Full Fetch
- [ ] A server's mark left on the user's unsent version, with the server holding the older version under the same ETag, gives a Conflict with the user's version kept as the User's Version, never an Incoming Sync Snapshot that silently replaces it
- [ ] Where the base is untrusted and the user's current version equals the fetched one, the server's mark moves and nothing else is written
- [ ] An untrusted base leaves the Conflict Events' Last Agreed Version empty (old 133's test, moved here)
- [ ] The untrusted base is a unit-tested case of `sync/plan.rs`, flagged for 198
- [ ] The Log line names the Calendar and UID, no values
- [ ] A changed ETag in a Full Fetch is compared as usual
