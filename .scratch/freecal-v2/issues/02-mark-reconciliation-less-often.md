# 02: Run Mark Reconciliation less often

**Idea:** v1 runs a Full Fetch of every server Calendar once a week as Mark Reconciliation (ADR 0019, ticket 171 of `.scratch/freecal-v1`), to find a server's Current-Version Mark that doesn't match what the server holds. If that fault never shows, make the interval longer (monthly, say), or run it only after a sync-token reset.

**Where it came from:** the grilling of point 5 of `.scratch/freecal-v1/critiques.md` (2026-11-22). Weekly was chosen while it is unknown whether the fault happens at all; it costs a few MB a week for a large Calendar.

**Condition:** no Log line for an untrusted base ("ETag unchanged, content differs from the version with the server's mark") in the Logs of the v1 users we hear from. FreeCal has no telemetry, so this rests on bug reports and testers' Logs.

**Questions for the triage:** How many Logs are enough? A longer fixed interval, or only on a sync-token reset? Does Google need the same interval as CalDAV?

**Status:** needs-triage

**Effort:** S

**Blocked by:** none
