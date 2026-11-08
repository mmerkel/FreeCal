# 115: One Sync Engine entry per Provider

**What to build:** The core interface calls the CalDAV Sync Engine directly, whatever the Account's Provider (`sync::caldav::incoming::{mirror_and_propagate, changes, incoming_sync}`, `core/src/lib.rs`), and decides itself between a First Sync and an incremental Incoming Sync, from the Known Server Resources it reads. The CalDAV rule for which resource keeps a UID (`Holds`, ADR 0015) lives in the import module, and the CalDAV Mirror's reads sit in the shared store file while its writes are in `store/sql/mirror/caldav.rs`. Give the Sync Engine one entry that picks the Provider's Sync Engine, so that ticket 25 adds Google beside CalDAV without the core interface naming either. From the architecture review of 2026-11-07 (candidate 4).

- **The entry:** functions in `core/src/sync/mod.rs` that `match` on the Account's Provider, the Local Provider having none. One call runs a Calendar's Incoming Sync, roughly `sync::incoming(remote, calendar, sync point, record)`: the Provider's Sync Engine reads its Known Server Resources, decides between the First Sync, page by page while the Calendar holds nothing from its server yet, its unfinished rest, and an incremental Incoming Sync, and returns the Calendar's Journal Transaction, if any. The core interface keeps starting and ending the Sync Status around it.
- **Journal Transactions:** the entry opens them through a recorder the core passes in, which runs a change as one Journal Transaction of a given Transaction Origin and then sends its Signals as `Core::record` does. What follows a commit stays in one place, `Core::signal`. Trade-off: the recorder's type is a little awkward (a `dyn` closure over a change returning `()`); moving the Signals into `journal::record` was rejected because the Change Journal would have to know the Sync Status and the Sync Scheduler, and would send Signals while the Store lock is held.
- **`Holds`**, `Holds::keeps` and `read_caldav_resource` move from `core/src/import/mod.rs` to the CalDAV Sync Engine (`core/src/sync/caldav/`). The import module keeps the Event reader they call.
- **The CalDAV Mirror's reads** `known_server_resources` and `uid_at` move from `core/src/store/sql/mod.rs` to `core/src/store/sql/mirror/caldav.rs`, beside its writes. `server_object` and `server_object_of_event`, the Corresponding Server Object that the Provider-neutral Sender and Conflicts read (ADR 0014), stay shared.
- **Stays as it is:** the comparison with the Known Server Resources inside `caldav.rs::changes`, as ticket 108 decided that `Remote::changes` takes them; `Core::add_caldav_account`, which is CalDAV's by name.
- No change to what the user sees or to the core interface; the CalDAV tests pass unchanged.

Spec: "Sync engine"; "Provider interface".

**Blocked by:** none

**Status:** done

**Effort:** M

- [x] `core/src/lib.rs` names no Provider's Sync Engine; an exhaustive `match` on the Provider in `core/src/sync/mod.rs` picks it
- [x] The decision between a First Sync, its rest and an incremental Incoming Sync is the Sync Engine's, not the core interface's
- [x] `Holds` and `read_caldav_resource` are in `core/src/sync/caldav/`; the CalDAV Mirror's reads are in `store/sql/mirror/caldav.rs`
- [x] The existing CalDAV and Sync Status tests pass unchanged

## Comments

Done (2026-11-08).

- The entry is `sync::incoming` (`core/src/sync/mod.rs`): an exhaustive `match` on the Provider, `Provider::Local => Ok(None)` as the Local Account has no server. The CalDAV side, `sync::caldav::incoming::sync`, reads the Known Server Resources and decides between the First Sync page by page, its rest and an incremental Incoming Sync; `Core::first_sync` moved there. `changes`, `incoming_sync` and `mirror_and_propagate` are now private to the CalDAV Sync Engine.
- The Provider reaches the entry with the Remote: `Core::remote` returns the Account's Provider with it.
- The Sync Engine takes the Store's `Mutex`, not a `&Store`, and locks it only for its reads and the `set_synced` write: holding the Store lock across the network requests and the recorder, which locks it again, would deadlock.
- `CODING_STANDARDS.md` "Local Store" now says that `sync/mod.rs` holds the entry and the recorder, and that a Provider's own Mirror reads sit in `mirror/` beside its writes, as this ticket moved them.
- The `Holds` unit test moved with `read_caldav_resource`; no other test changed. The core suite, clippy and fmt pass.

New terms:
- **Recorder** (new term): the callback the core passes the Sync Engine, which runs a change as one Journal Transaction of a Transaction Origin and then sends its Signals as `Core::record` does (`sync::Recorder`).
- **sync point** (new term, prose for the existing `SyncPoint`): how far a Calendar whose First Sync finished is synced, such as its CalDAV sync token.
