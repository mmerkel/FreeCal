# 01: Keep the text of a resource that fails the round-trip check, for a bug report

**Idea:** When v1's round-trip check (ADR 0017, ticket 197 of `.scratch/freecal-v1`) finds that FreeCal couldn't read everything of a CalDAV resource, keep that resource's text, and offer "Copy details" in the Event's details, so a bug report holds what the reader missed. v1's Log line names only the parts, not their values.

**Where it came from:** alternative 2 of the grilling of point 3 of `.scratch/freecal-v1/critiques.md` (2026-11-22). Deferred because it stores a second format again, if only for failures, and copies untrusted content (another client's data, perhaps personal) to the clipboard; v1's Log line is expected to be enough to reproduce most bugs.

**Questions for the triage:** Kept until the next fetch passes, or with a limit? Scrubbed of values before copying, or copied whole after a warning? Does Google's JSON get the same?

**Status:** needs-triage

**Effort:** S

**Blocked by:** none
