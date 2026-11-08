# 125: Recurrence scopes, Exceptions and undos reach the server

**What to build:** The last part of ticket 17 (read it for the history). End-to-end tests against Radicale, and the fixes they call for, showing that what ticket 123's Sender sends is right beyond single Events.

- Recurrence scope edits (this / this and following / all), moves and deletes, and new, changed and deleted Exceptions, reach the server as one resource per UID, and a fresh Incoming Sync from the server (or a second client) reads the same Events back.
- An undo or redo of a change in a server Calendar is sent like any other change, under the original identity (same UID, same href).
- Undoing a sync's Journal Transaction restores the previous version locally but leaves `calendar.sync_token` alone, so the server's newer version isn't fetched again until it changes there. The undo has to reach the server, or the next whole listing brings the server's version back.
- Undoing a sync with a takeover (ADR 0015) brings back Event Rows that the new Corresponding Server Object doesn't hold, and a later sync that fetches that object drops them again (ticket 91's comment). Decide what sending such an undo does, and record it in a comment.
- Unknown fields survive these edits on the server, byte-for-byte (ticket 122 tests the text; this tests the round trip).

Spec: US-46; "Change Journal"; "Testing Decisions" (Seam 1). ADR 0012.

**Blocked by:** 08, 69, 123

**Status:** ready-for-agent

**Effort:** M

- [ ] Recurrence scope edits (this / this and following / all) and Exceptions reach the server correctly
- [ ] An undo or redo of a change in a server Calendar is sent to the server like any other change, under the original identity
- [ ] An undone sync stays undone after the next whole listing, including one with a takeover
- [ ] Unknown fields survive these edits on the server
- [ ] Tested against Radicale

## Comments
