# 125: Recurrence scopes, Exceptions and undos reach the server

**What to build:** The last part of ticket 17 (read it for the history). End-to-end tests against Radicale, and the fixes they call for, showing that what ticket 127's Sender sends is right beyond single Events.

- Recurrence scope edits (this / this and following / all), moves and deletes, and new, changed and deleted Exceptions, reach the server as one resource per UID, and a fresh Incoming Sync from the server (or a second client) reads the same Events back.
- An undo or redo of a change in a server Calendar is sent like any other change, under the original identity (same UID, same href).
- Undoing a sync's Journal Transaction restores the previous version locally but leaves `calendar.sync_token` alone, so the server's newer version isn't fetched again until it changes there. The undo has to reach the server, or the next whole listing brings the server's version back.
- Undoing a sync with a takeover (ADR 0015) brings back Event Rows that the new Corresponding Server Object doesn't hold, and a later sync that fetches that object drops them again (ticket 91's comment). Decide what sending such an undo does, and record it in a comment.
- Unknown fields survive these edits on the server, byte-for-byte (ticket 122 tests the text; this tests the round trip).

Spec: US-46; "Change Journal"; "Testing Decisions" (Seam 1). ADR 0012.

**Blocked by:** 08, 69, 127, 128

**Status:** done

**Effort:** M

- [x] Recurrence scope edits (this / this and following / all) and Exceptions reach the server correctly
- [x] An undo or redo of a change in a server Calendar is sent to the server like any other change, under the original identity
- [x] An undone sync stays undone after the next whole listing, including one with a takeover
- [x] Unknown fields survive these edits on the server
- [x] Tested against Radicale

## Comments
- 2026-11-08 (/implement): three boxes done, two wait on the new ticket 129. Tests: `core/tests/caldav_send.rs` against Radicale, modules `recurrence` and `undo`. Each test checks that a second client's First Sync, a whole listing of the server, shows the same Occurrences, and that nothing stays unsent. Unit test: `an_exception_whose_recurring_event_moved_keeps_its_vevent` (`core/src/sync/caldav/resource/tests.rs`).
  - **Fixed: an "all events" move lost its Exceptions' unknown fields.** Moving a Recurring Event's time moves its Exceptions' recurrence IDs too (ADR 0012). Ticket 122's resource builder then paired no server `VEVENT` with them and wrote new ones, dropping what FreeCal doesn't show. Now, when the Event Rows' Recurring Event moved from the time the Server Object holds, the builder pairs every server Exception at its recurrence ID moved by as much (`moved_with`, used by `Loaded::pair_with_server`, `core/src/event/mod.rs`). It moves them all first, so a move by a whole week doesn't pair an Exception with the next one's `VEVENT`. The paired `VEVENT` gets the new `RECURRENCE-ID` as a group of its own (`Group::RecurrenceId`, `core/src/sync/caldav/resource/write.rs`), compared by Occurrence. Like the other groups, a rewritten `RECURRENCE-ID` keeps only its time and zone, not other parameters.
  - **Undoing a sync with a takeover (decision).** Sending the undo updates the resource that took over: the Event Rows the undo brought back are written into it, with `If-Match` on its ETag, as with any update. The deleted resource isn't created again. That follows the one-resource-per-UID rule: the Sender always writes a UID's Event Rows to its Corresponding Server Object. A later sync fetches what FreeCal sent, so nothing is dropped. The cost: the server version of the resource that took over is overwritten, as with any undone sync. The deleted resource's href and unknown fields don't come back (`an_undone_sync_with_a_takeover_writes_the_events_back_into_the_resource_that_took_over`).
  - **Undone syncs.** Sending the undo, with `If-Match` on the server's newer ETag that the Mirror holds, writes the earlier version over it. A whole listing then reads the undone version, as the second client's does. FreeCal's own client was checked only with its next incremental sync, not with a whole listing.
  - **Open, ticket 129 (box 2 and box 4):** a resource FreeCal *creates* is built from the Event Rows alone, as the unknown fields live only in the Mirror. Three cases lose them on the server:
    - **Undoing a sent DELETE:** the Event comes back at `<UID>.ics`, not at its original href, and without its unknown fields. The UID is kept, the href isn't.
    - **A move to another Calendar:** the Event is created in the target without its unknown fields.
    - **The new Recurring Event of "this and following events":** it doesn't take the original's unknown properties along.

    Everything else keeps both href and unknown fields: undos and redos of edits, of Exceptions and cancelled Occurrences, and of splits; the scope edits themselves. Ticket 129 asks where unknown fields live without a Server Object.
  - **Review judgement calls not taken:**
    - Bundling an Event Row with its Recurring Event's time (`row`, `recurring`) into one type for `rewrite`, `Group::differs`, `Group::write` and `new_vevent`.
    - Inlining the test helpers that only pass the call on (`OnServer::shows`, `on`, `names`).
  - **New terms:**
    - Code names: `moved_with` (an Event of a Server Object with its recurrence ID moved as its Recurring Event moved), `Group::RecurrenceId` (the `RECURRENCE-ID` line as a group of FreeCal's Properties), `normal_form` (now `pub(crate)`).
    - Test names: `OnServer::another_client_shows` and `assert_read_back` (a second client's First Sync of the Calendar shows what this one shows), `Radicale::event_names` (the resource names a Radicale Calendar holds).
- 2026-11-08: the user closed boxes 2 and 4 and the ticket. What is left of them moved to ticket 129: an undone sent DELETE keeping its href and unknown fields, moves to another Calendar, and the new Recurring Event of a split.
