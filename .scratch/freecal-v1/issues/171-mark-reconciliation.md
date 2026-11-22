# 171: Mark Reconciliation: a weekly Full Fetch, and a Conflict where the base is untrusted

**What to build:** Once a week per server Calendar, its next Incoming Sync is a Full Fetch, which checks the server's Current-Version Marks against what the server holds (ADR 0019, "Whether the server changed" and "An untrusted base"). Point 5 of `.scratch/freecal-v1/critiques.md` and point 1 of `critiques-2.md` have the reasons.

- **When:** the time of a Calendar's last Full Fetch is stored beside its sync token, and every Full Fetch sets it (First Sync, sync-token reset, Mark Reconciliation). An Incoming Sync that starts while that time is more than 7 days old is a Full Fetch, in the background like any Incoming Sync. A server without ETags already fetches everything every time; it has no Mark Reconciliation of its own.
- **An untrusted base,** on every Full Fetch: a fetched resource whose ETag equals the stored one but whose content differs from the version with the server's mark. The server's mark moves to the fetched version; an Occurrence where the user's current version is the same agrees; any other becomes a Server Event Replacement, with no Automatic Resolution. One Log line names the Calendar and UID (no values: "Untrusted content"). Nothing else: no Sync Error, toast or mark in the origin; the Conflict shows as any other.
- **Everything else** of a Full Fetch, a changed ETag, a resource gone from the server or one with no server's mark, goes through ticket 167's comparison as usual.

Spec: "Sync engine". ADR 0019.

**Blocked by:** 155, 167

**Status:** ready-for-agent

**Effort:** S

- [ ] An Incoming Sync of a Calendar whose last Full Fetch is more than 7 days old fetches every resource; one within 7 days fetches only what changed
- [ ] Every kind of Full Fetch sets the time of the last Full Fetch
- [ ] A server's mark left on the user's unsent version, with the server holding the older version under the same ETag, gives a Conflict with the user's version kept, never a Server Intake that replaces it
- [ ] Where the base is untrusted and the user's current version equals the fetched one, the server's mark moves and nothing else is written
- [ ] Where the base is untrusted, no Automatic Resolution is made, even when only the Unimplemented Fields differ
- [ ] The Log line names the Calendar and UID, no values
- [ ] A changed ETag in a Full Fetch is compared as usual

## Comments
- 2026-11-22: Written from the grilling of point 5 of `critiques.md` with point 1 of `critiques-2.md`. To run less often later: `.scratch/freecal-v2/issues/02-mark-reconciliation-less-often.md`.
