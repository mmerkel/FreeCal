# 81: Store each Event's original server data

**What to build:** Each Event pulled from a server keeps its original server data exactly as received, next to the fields FreeCal uses (ADR 0004), so later writes (ticket 17) can preserve fields FreeCal doesn't understand and Change Journal entries can restore exact previous versions. Split from ticket 54.

**Blocked by:** 80

**Status:** done

**Effort:** S

- [x] The Local Store holds each server Event's original data as received, byte for byte, with its ETag; Local Calendars' Events have none
- [x] The first sync stores it
- [x] The Change Journal's previous versions include it and the Event hash covers it: a new hash format version, with the re-hash `journal.rs` describes for added fields (`journal::rehash`)
- [x] An undo writes the previous original data back with the rest of the version
- [x] Tests: the hostile test Event's data is stored unchanged; the hash format test is updated for the new version

Out of scope: writing it back to the server (ticket 17).

## Comments

- 2026-10-18: Split from ticket 54 so each part fits one session.
- 2026-10-18: Done. Original server data is `EventFields.server_data`, an `OriginalServerData` (the object's `href` as the server gave it, its ETag, and its iCalendar text as received), in the columns `server_href`, `server_etag`, `server_data` of `event` and `journal_entry`. It lives on the Recurring Event's row; its Exceptions have none, as the object holds them. `Unit` decides it (`with_server_data`): a first sync and an undo write it as given, user actions and imports keep the previous version's, and a move to another Calendar drops it. Hash format 3 covers it; `journal::rehash` now recognises any earlier format and runs once after the migrations in `store::migrate`, since a migration's own re-hash read columns later migrations add. The multiget asks for `getetag`; the XML reader keeps CR LF (unit test in `caldav.rs`). `Core::original_server_data` reads it.
