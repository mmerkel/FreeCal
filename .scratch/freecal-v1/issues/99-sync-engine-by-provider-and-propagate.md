# 99: The Sync Engine laid out by Provider, and Propagate a Server Object

**What to build:** Mechanical changes agreed on 2026-11-06 (ADR 0013, `GLOSSARY.md`: Propagate a Server Object). Behaviour stays the same.

- The CalDAV part of the Sync Engine moves to `core/src/sync/caldav/`: `sync/incoming.rs` becomes `sync/caldav/incoming.rs`, and what only CalDAV needs from `sync/mod.rs` (handing a UID's Event Rows to its next Corresponding Server Object, `settle`, and recognising a moved Server Object, `moved`) moves to `sync/caldav/mod.rs`. `sync/mod.rs` keeps what every Provider shares. Don't define a shared interface for Providers; that waits for Google (ticket 26).
- The Mirror's SQL moves from `core/src/store/sql/mirror.rs` to `core/src/store/sql/mirror/caldav.rs`.
- Propagate a Server Object: `event::take_object` becomes `propagate_to_event_rows`. The Incoming Sync's entry point `sync::apply` becomes `incoming_sync`, and the loop over fetched Server Objects (`take`) is named after what it does, or folded into it. Identifiers, comments and test names that say "take" for this say propagate.
- `CODING_STANDARDS.md` "Local Store" names the new files.

**Blocked by:** none

**Status:** ready-for-agent

**Effort:** S

- [ ] `core/src/sync/caldav/` holds the CalDAV Incoming Sync, `settle` and `moved`; `sync/mod.rs` holds only what every Provider shares
- [ ] The Mirror's SQL is in `core/src/store/sql/mirror/caldav.rs`
- [ ] `propagate_to_event_rows` and `incoming_sync` replace `take_object`, `take` and `sync::apply`; no identifier or comment says "take" for propagating
- [ ] Moves keep git history (`git mv` before editing)
- [ ] `CODING_STANDARDS.md` "Local Store" names the new files
- [ ] Every existing test passes unchanged apart from paths and names
