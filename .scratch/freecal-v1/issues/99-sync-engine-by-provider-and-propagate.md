# 99: The Sync Engine laid out by Provider, and Propagate a Server Object

**What to build:** Mechanical changes agreed on 2026-11-06 (ADR 0013, `GLOSSARY.md`: Propagate a Server Object). Behaviour stays the same.

- The CalDAV part of the Sync Engine moves to `core/src/sync/caldav/`: `sync/incoming.rs` becomes `sync/caldav/incoming.rs`, and what only CalDAV needs from `sync/mod.rs` (handing a UID's Event Rows to its next Corresponding Server Object, `settle`, and recognising a moved Server Object, `moved`) moves to `sync/caldav/mod.rs`. `sync/mod.rs` keeps what every Provider shares. Don't define a shared interface for Providers; that waits for Google (ticket 26).
- The Mirror's SQL moves from `core/src/store/sql/mirror.rs` to `core/src/store/sql/mirror/caldav.rs`.
- Propagate a Server Object: `event::take_object` becomes `propagate_to_event_rows`. The Incoming Sync's entry point `sync::apply` becomes `incoming_sync`, and the loop over fetched Server Objects (`take`) is named after what it does, or folded into it. Identifiers, comments and test names that say "take" for this say propagate.
- `CODING_STANDARDS.md` "Local Store" names the new files.

**Blocked by:** none

**Status:** done

**Effort:** S

- [x] `core/src/sync/caldav/` holds the CalDAV Incoming Sync, `settle` and `moved`; `sync/mod.rs` holds only what every Provider shares
- [x] The Mirror's SQL is in `core/src/store/sql/mirror/caldav.rs`
- [x] `propagate_to_event_rows` and `incoming_sync` replace `take_object`, `take` and `sync::apply`; no identifier or comment says "take" for propagating
- [x] Moves keep git history (`git mv` before editing)
- [x] `CODING_STANDARDS.md` "Local Store" names the new files
- [x] Every existing test passes unchanged apart from paths and names

## Comments

- 2026-11-06: Done.
  - **Layout.** `core/src/sync/caldav/mod.rs` holds `settle`, `moved` and the Sync Engine's Mirror writes (`mirror`, `unreadable`, `remove_object`, `event_keys_in_calendar`, and the `testing` module the Change Journal's tests use). `sync/caldav/incoming.rs` holds CalDAV's Incoming Sync. `sync/mod.rs` keeps only the Write Permission.
  - **Beyond the ticket's list.** The ticket named only `settle` and `moved`. Both reviewers pointed out that the Mirror writes wrap SQL that is now in `mirror/caldav.rs`, and `unreadable` works by href. So they moved too: one rule, that whatever reads or writes CalDAV's Mirror is in `sync/caldav/`. Trade-off: Google (ticket 26) will need writes of its own rather than reusing these. That is what ADR 0013 expects anyway.
  - **Store side.** `store/sql/mod.rs` declares the Mirror as `mod mirror { mod caldav; }`, so there is no near-empty `mirror/mod.rs`.
  - **Renames.**
    - `event::take_object` → `propagate_to_event_rows`.
    - `sync::incoming::apply` → `sync::caldav::incoming::incoming_sync`.
    - The loop `take` → `mirror_and_propagate`. It is not folded into `incoming_sync`, because the First Sync calls it directly, one page at a time. Its name says mirroring, since it also mirrors each Server Object, removes empty ones and recognises moved ones.
    - The test helper `Alone::take` → `Alone::mirror_and_propagate`.
  - **What still says "take".** `Loaded::take` and `take_exception` are the import's writers, not Propagate, so they keep their names. `TODO.md` lines 49 and 55 still say `take_object`; they were left for the user to edit.
  - **No new terms.**

