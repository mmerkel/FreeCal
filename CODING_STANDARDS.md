# Coding standards

The rules a review checks a diff against. They are judgement calls; mechanical rules are enforced by the checks in `.husky/pre-commit` instead.

## Words

- The core's push messages are always called Signals, never "events" (calendar Events) or "notifications" (desktop notifications).
- Domain terms follow `GLOSSARY.md`.

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

- SQL lives only in `core/src/store/`: `schema.rs` holds the schema and its migrations, and `sql/` every query and write. The guards (the Change Journal in `core/src/journal/`, the Sync Engine in `core/src/sync/`) hold no SQL.
- In `sql/`, each write sits in the file of the guard whose Write Permission covers it: `mod.rs` holds the readers; `journal.rs` the Change Journal's writes (Event Rows, Journal Transactions and Journal Event Modifications, Unsent Changes, deleting Calendars and accounts); `mirror.rs` the Sync Engine's (a Server Object's data and its Combined Event Keys in the Mirror); `unguarded.rs` the writes no guard holds (accounts, Calendars, settings, sidebar, view, time zone rules).
- Every write is a method of `DatabaseTransaction`, so none runs outside a Database Transaction; reads are methods of `Store` (ADR 0011). Code writes through `Store::write`, which commits once its closure succeeds; only what must hold a Database Transaction, as a Journal Transaction does, calls `Store::begin`.
- Every statement that writes a guarded table (`event`, `server_object`, `combined_event_key`, `unsent_change`, `journal_transaction`, `journal_event_modification`) is in a function that takes its Guard's Write Permission: `&JournalWritePermission` in `journal.rs`, `&SyncEngineWritePermission` in `mirror.rs` (ADR 0011). Unguarded Writes in `unguarded.rs` take none.
- A Write Permission's field stays private, so that only its Guard's module can make one.

## Dependencies

- Every npm dependency is GPLv3-compatible (ADR 0003). `cargo deny check licenses` checks the Rust ones.
