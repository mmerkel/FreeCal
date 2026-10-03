# Handoff: FreeCal — fix Local Store migration issues, then QA the interface

Repo: `/home/mmpi/Code/FreeCal`, branch `main`. Ticket 01 (app skeleton) is implemented and committed as `a8d0683`.

Read first:
- Ticket and its leftover notes (under `## Comments`): `.scratch/freecal-v1/issues/01-app-skeleton-local-account.md`
- Spec, especially "Implementation Decisions" and "Testing Decisions": `.scratch/freecal-v1/spec.md`
- `GLOSSARY.md`, `docs/adr/0001`–`0004`, `CLAUDE.md` (code conventions and check commands)
- The code: `git show a8d0683 --stat`

## Task 1: fix two issues in `migrate` (`core/src/store.rs`)

The user asked how `migrate` behaves when no database file exists yet. That case is fine: `Connection::open` creates the file, `user_version` is 0 and every migration runs in one transaction. Two real issues came up, and the user ran this handoff to get them fixed:

1. **A database from a newer FreeCal opens silently.** If `user_version` is higher than `MIGRATIONS.len()`, `.skip()` skips every migration and the store opens anyway. It should refuse with a new `CoreError` variant, for example "Local Store was written by a newer FreeCal (schema N, this build knows M)".
2. **Race between two instances.** `user_version` is read *before* the transaction starts, so two processes can both read 0, and the second then fails with "table already exists". Fix: start the transaction with `conn.transaction_with_behavior(TransactionBehavior::Immediate)` and read `user_version` inside it.

Work test-first, at the agreed seam: the core application interface, through the harness in `core/tests/support/mod.rs`, with real SQLite in a temporary directory.
- For issue 1, a test has to put the file into a "newer schema" state. The cleanest way is test-only setup that sets `PRAGMA user_version` on `freecal.sqlite3` in the harness's data directory before calling `harness.open()`. That means the harness needs a way to expose the data directory path or run setup SQL. Keep it explicit that this is *arranging* state, not *verifying* it through a side channel.
- Issue 2 is hard to test deterministically. A code change with a comment is fine; don't write a flaky concurrency test.

## Task 2: QA of the interface

Review and harden the core application interface and its mirror in the frontend, as they stand after ticket 01:
- Rust: `core/src/lib.rs` (`Core`, `CoreError`), `account.rs`, `clock.rs`, `signal.rs`; the shell's commands in `src-tauri/src/main.rs`.
- TypeScript: `src/core/CoreApi.ts`, `src/core/types.ts`, `src/core/tauriCore.ts`, the fake in `src/test/fakeCore.ts`.

Open points from the code review (judgement calls; nothing decided yet):
- The types are copied by hand between Rust and TypeScript (`Account`, `Provider`, `Signal`, and the `'freecal://signal'` channel name). Consider generating them (ts-rs or specta; check that the licence is GPLv3-compatible) or at least add a check that they match.
- Dates cross between the two sides as `"YYYY-MM-DD"` strings (`today`). `temporal-polyfill` is loaded anyway, because FullCalendar v7 needs it, so `Temporal.PlainDate` would fit on the TypeScript side.
- Commands turn errors into `String` (`map_err(|e| e.to_string())`), so the frontend shows English text from the core outside `t()`. Consider serialising `CoreError` as a code the frontend can translate.
- `Core::today` uses the system offset through `Clock::now() -> DateTime<FixedOffset>`. The Display Time Zone belongs to ticket 06; don't build it now.
- The main window has never been seen in the real app: the screen was locked during the session. The launch itself worked (no errors, database created with the Local Account). To check visually, build with `npx tauri build --debug --no-bundle` and run `XDG_DATA_HOME=<scratch dir> ./target/debug/freecal`. The user's system locale is French, but the grid shows FullCalendar's English labels; that is expected until ticket 06.

Deliberately deferred; don't flag these as bugs:
- `Signal` is an empty enum, and the frontend doesn't subscribe yet. Ticket 02 adds both.
- `CalendarGrid` takes only `today`. Ticket 03 adds the Occurrences prop and the user-action callbacks.

Decision still open with the user: the app identifier `org.freecal.FreeCal`, which sets the data directory and matters for Flathub. Don't change it without asking.

## Checks

`cargo test --workspace`, `cargo clippy --workspace --all-targets`, `cargo fmt --all --check`, `npm run check`, `npm test`. All passed at `a8d0683`.

## Suggested skills

- `tdd`: for the migration fixes (red → green at the core interface seam).
- `codebase-design`: vocabulary for the interface QA (depth, seam, adapter).
- `code-review`: review the work against `a8d0683` when done.
- `domain-modeling`: "Local Store" is used throughout but is missing from `GLOSSARY.md`.
- `run`: to see the real Tauri window, if the screen is available.
