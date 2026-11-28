# 170: A corpus of real servers' texts, run through the round-trip check in the tests

**What to build:** A collection of CalDAV resources as real servers and clients write them, run through ticket 197's round-trip check in a test, so that gaps in ticket 175's reader are found before a user loses data (ADR 0017, Consequences). Ticket 176 has a small corpus already (`core/src/sync/caldav/resource/tests.rs`); this one covers what other clients write.

- **Sources:** Nextcloud, Radicale, Fastmail, iCloud and Google's CalDAV endpoint, and Events written there by Outlook, Thunderbird, Apple Calendar, GNOME Calendar, DAVx⁵ and a booking or meeting tool. Include Recurring Events with Exceptions, alarms, attachments, attendees, non-IANA `TZID`s and custom `VTIMEZONE`s.
- **Personal data:** every text is scrubbed of names, addresses, titles and descriptions before it is committed, with the structure kept (replace values, never remove properties).
- **The test:** each text passes the check with no loss beyond ticket 175's list of deliberate drops. A failure is a bug in the reader: it is fixed there, or, if FreeCal decides to drop the part on purpose, listed.
- **Where:** a directory of `.ics` files under `core/tests/`, one per resource, named after source and case.

Collecting the texts needs accounts and clients only the user has, so this is the user's part; an agent can scrub them, add the test and fix what it finds.

Spec: "Untrusted content". ADR 0017.

**Blocked by:** 197

**Status:** ready-for-human

**Effort:** M

- [ ] Texts from at least five servers or clients, scrubbed
- [ ] A test runs every text through the round-trip check
- [ ] Every failure found is fixed in the reader or listed as a deliberate drop

## Comments
- 2026-11-22: Written from the grilling of point 3 of `critiques.md` (alternative 1 there).
