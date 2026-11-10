# Coding standards

The rules a review checks a diff against. They are judgement calls; mechanical rules are enforced by the checks in `.husky/pre-commit` instead.

## Words

- The core's push messages are always called Signals, never "events" (calendar Events) or "notifications" (desktop notifications).
- Domain terms follow `GLOSSARY.md`.

## Signals

- A write that changes what a core read returns sends the Signal that makes the window read it again, also when it records no Journal Transaction, as a send or a stored Send Failure does. A test checks the Signal goes out.

## Frontend

- Every UI string goes through the i18n layer (`t` in `src/i18n`).
- Colours, spacing and controls that more than one component uses live once in `src/app.css` as tokens or classes (spec: "Visual style"). A component that copies another component's control styles is a finding.
- A side effect that relies on the DOM showing new `$state`, such as showing the window once the theme is applied, runs after `await tick()`. Its test checks what is visible at the moment of the effect, not only the order of the calls.

## Untrusted content

Every stored field is untrusted, whatever its origin (spec: "Untrusted content" in `.scratch/freecal-v1/spec.md`):

- `.ics`, XML and SQL are produced only through a library's writer or bound parameters.
- The CSP never gets looser for scripts or remote content.
- Links open in the system browser through the shell (`openLink`), never in the window.
- A field that gains a new way of being shown, such as links or markup pieces, gets a matching payload and `expected` entry in `fixtures/hostile-event.json`.

## Local Store

- SQL lives only in `core/src/store/`: `schema.rs` holds the schema and its migrations, and `sql/` every query and write. The guards (the Change Journal in `core/src/journal/`, the Sync Engine in `core/src/sync/`) hold no SQL. The Sync Engine keeps each Provider's code in a module of its own (CalDAV's in `sync/caldav/`, with CalDAV's Incoming Sync in `sync/caldav/incoming.rs`), and what every Provider shares in `sync/mod.rs`: the entry that picks the Provider's Sync Engine by an exhaustive `match` on the Provider, the recorder it opens Journal Transactions through, and the Write Permission (ADR 0015). The core interface names no Provider's Sync Engine.
- In `sql/`, each write sits in the file of the guard whose Write Permission covers it: `mod.rs` holds the readers, but those of a Provider's own Mirror tables (CalDAV's Known Server Resources), which sit beside that Provider's writes in `mirror/`; the Corresponding Server Object, which the Provider-neutral Sender and Conflicts read (ADR 0014), stays in `mod.rs`; `journal.rs` the Change Journal's writes (Event Rows, Journal Transactions and Journal Event Modifications, Unsent Changes, deleting Calendars and accounts); `mirror/` the Sync Engine's (a Server Object in its Provider's Mirror tables, and CalDAV's Ignored Resources), one file per Provider (`mirror/caldav.rs`, ADR 0015); `unguarded.rs` the writes no guard holds (accounts, Calendars, settings, sidebar, view, time zone rules).
- Every write is a method of `DatabaseTransaction`, so none runs outside a Database Transaction; reads are methods of `Store` (ADR 0015). Code writes through `Store::write`, which commits once its closure succeeds; only what must hold a Database Transaction, as a Journal Transaction does, calls `Store::begin`.
- Every statement that writes a guarded table (`event`, `caldav_server_object`, `caldav_ignored_resource`, `unsent_change`, `journal_transaction`, `journal_event_modification`) is in a function that takes its Guard's Write Permission: `&JournalWritePermission` in `journal.rs`, `&SyncEngineWritePermission` in `mirror/` (ADR 0015). Unguarded Writes in `unguarded.rs` take none.
- A Write Permission's field stays private, so that only its Guard's module can make one.

## Dependencies

- Every npm dependency is GPLv3-compatible (ADR 0003). `cargo deny check licenses` checks the Rust ones.
