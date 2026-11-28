# 187: The Sender builds every resource from Event Versions; the Mirror's text goes

**What to build:** The CalDAV Sender builds each resource with 176's writer from the user's Event Versions alone (ADR 0017), so what FreeCal doesn't show survives a move, an undone delete and "this and following events". The Mirror's text and the patching writer come out. Old ticket 150 (`git show design/snapshot-line:.scratch/freecal-v1/issues/150-sender-builds-from-event-versions.md`) as written.

- **The Sender** (`sync/caldav/sender.rs`) loads a UID's Event Versions with their trees and writes the resource from them with `ical/write.rs`, whether the resource exists on the server or not. Create, update or delete is decided by whether the UID has a kept resource in `caldav_server_object` and whether it has Events on the user's side. Every update and delete stays conditional on the stored ETag.
- **"Nothing to send":** the Sender's comparison with the Mirror's text goes with the text. Until 192 derives what to send from the marks, a UID with an Unsent Change is always sent, also when an edit and its Undo left it as the server has it. Trade-off, named: a request too many in that case for five tickets, against keeping a text only to avoid it.
- **`caldav_server_object`** keeps the Calendar, the UID, the href and the ETag, and loses the text. The Incoming Sync reads a fetched resource into Event Versions and keeps nothing else of it; Propagate reads the fetched resources, not stored text, and stays until 189. Ignored Resources are kept as before, their triggers reworded from "mirrored" to "kept".
- **What comes out:** the patching writer (`sync/caldav/resource/`), and the showing of the server's original text: `Core::original_server_data`, its command and its part in the Event's details (ADR 0017: the text is no longer in the Local Store, for writing or inspection). The generated types are rewritten.
- **The three cases** (from old ticket 129), tested against Radicale in `core/tests/caldav_send.rs` with an Event that has an alarm, a custom property and a parameter FreeCal doesn't know: moved to another CalDAV Calendar, the Event has them on the server in the target; after a DELETE was sent and undone, the Event is created again with them under a new href (ADR 0019); the new Recurring Event of "this and following events" has them on the server.
- **An edit of a managed field** reaches the server without that property's unknown parameters, with every other entry unchanged.

Spec: "Local Store"; "Sync engine"; "Moves across Accounts"; "Untrusted content". ADRs 0017, 0019 ("Server Objects"). Layout: "Tables (end state)" (`caldav_server_object`, `caldav_ignored_resource`); "By phase", P2, 187.

**Blocked by:** 176, 186

**Status:** ready-for-agent

**Effort:** M

**Deleted tests:** `core/src/sync/caldav/resource/tests.rs` (176 took over the cases worth keeping); `store/schema.rs`'s `an_href_is_never_both_mirrored_and_ignored`, or rewritten for the kept and Ignored Resources. Rewritten: `caldav_send.rs`'s "send results" (the patching writer's assertions; unknown fields kept, now read back from Radicale); `caldav_sync.rs`'s harness where it asserts the Mirror's text.

- [ ] The Sender writes every resource through `ical/write.rs`; no code reads a stored server text
- [ ] `caldav_server_object` holds where and ETag only; the patching writer is gone
- [ ] The three cases above, each tested against Radicale, reading the resource back
- [ ] An Event edited in FreeCal keeps its alarm and custom properties on the server
- [ ] The showing of the server's original text is gone from the core interface and the window
- [ ] The tests above deleted or rewritten; the other send and sync tests pass, changed only where they asserted the Mirror's text or a skipped request
- [ ] `GLOSSARY.md` follows: Mirror goes; CalDAV Server Object (no text), Known Server Resources, Ignored Resource ("kept in the CalDAV table"), Database Transaction (the Mirror's writes), Sync Engine ("the only code that writes the Mirror"), Server Object and Sender where they speak of the text received

## Comments

- 2026-11-28: From the review of 176: the writer (`write_resource`) returns, with the text, what of the fields and trees it left out (`Written::left_out`); the sender writes each distinct one to the Log with the href, as `log_violations` does for the reader. It is empty unless the Local Store holds what the reader never makes.
