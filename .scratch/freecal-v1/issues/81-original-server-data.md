# 81: Store each Event's original server data

**What to build:** Each Event pulled from a server keeps its original server data exactly as received, next to the fields FreeCal uses (ADR 0004), so later writes (ticket 17) can preserve fields FreeCal doesn't understand and Change Journal entries can restore exact previous versions. Split from ticket 54.

**Blocked by:** 80

**Status:** ready-for-agent

**Effort:** S

- [ ] The Local Store holds each server Event's original data as received, byte for byte, with its ETag; Local Calendars' Events have none
- [ ] The first sync stores it
- [ ] The Change Journal's previous versions include it and the Event hash covers it: a new hash format version, with the re-hash `journal.rs` describes for added fields (`journal::rehash`)
- [ ] An undo writes the previous original data back with the rest of the version
- [ ] Tests: the hostile test Event's data is stored unchanged; the hash format test is updated for the new version

Out of scope: writing it back to the server (ticket 17).

## Comments

- 2026-10-18: Split from ticket 54 so each part fits one session.
