# 150: The Sender builds every resource from Event Versions; the Mirror's text goes

**What to build:** The CalDAV Sender builds each resource with ticket 144's writer from the user's Event Versions alone (ADR 0017), so what FreeCal doesn't show survives the three cases of ticket 129. The Mirror's text and the patching writer come out. This is step 2 of the order in which the old model goes.

- **The Sender** (`core/src/sync/caldav/sender.rs`) loads a UID's Events with their trees and writes the resource from them, whether the resource exists on the server or not. Create, update or delete is decided by whether the UID has a kept resource in the CalDAV table and whether it has Events on the user's side. Every update and delete stays conditional on the stored ETag.
- **"Nothing to send":** the Sender's comparison of the text it would send with the Mirror's text goes with the text. Until ticket 153 derives what to send from the Current-Version Marks, a UID with an Unsent Change is always sent, also when an edit and its undo left it as the server has it. Trade-off, named: a request too many in that case for three tickets, against keeping a text only to avoid it.
- **The CalDAV table** (`caldav_server_object`) keeps the Calendar, the UID, the href and the ETag, and loses the text. The Incoming Sync reads a fetched resource into Event Versions and keeps nothing else of it.
- **What comes out:** the patching writer (`resource_to_send`, `update`, `rewrite` and the pairing of rows with `VEVENT`s in `core/src/sync/caldav/resource/`), its tests where ticket 144's round-trip tests cover the same texts, and the showing of the server's original text for an Event (`Core::original_server_data` and its part in the Event's details, ticket 81): ADR 0017 says the text is no longer in the Local Store, for writing or for inspection.
- **Ticket 129's three cases**, tested against Radicale in `core/tests/caldav_send.rs` with an Event that has an alarm, a custom property and a parameter FreeCal doesn't know:
  - moved to another CalDAV Calendar, the Event has them on the server in the target;
  - after a DELETE was sent and undone, the Event is created again with them, under the name FreeCal gives a new resource (ADR 0019: a new href, not the original one);
  - the new Recurring Event of "this and following events" has them on the server.
- **An edit of a managed field** reaches the server without that property's unknown parameters, and with every other entry unchanged (ADR 0017's consequence, now visible on the server).
- **Docs:** `GLOSSARY.md` revises Mirror, CalDAV Server Object, Server Object and Sender where they speak of the text received.

Spec: "Local Store"; "Sync engine"; "Moves across Accounts"; "Untrusted content". ADRs 0017, 0019 ("Server Objects").

**Blocked by:** 144, 149

**Status:** ready-for-agent

**Effort:** M

- [ ] The Sender writes every resource through ticket 144's writer; no code reads a stored server text
- [ ] `caldav_server_object` holds where and ETag only; the patching writer is gone
- [ ] The three cases of ticket 129, each tested against Radicale
- [ ] An Event edited in FreeCal keeps its alarm and custom properties on the server (test against Radicale, reading the resource back)
- [ ] The showing of the server's original text is gone from the core interface and the window
- [ ] The existing send and sync tests pass, changed only where they asserted the Mirror's text or a skipped request
- [ ] `GLOSSARY.md` follows

## Comments
- 2026-11-16: Written from the triage against ADRs 0017 to 0020. Second of two parts (see ticket 149). Against the order first listed, the Sender's comparison of texts goes here and not with ticket 153, since it can't outlive the text.
