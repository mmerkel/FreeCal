# Handoff: FreeCal: QA of the core application interface

Repo: `/home/mmpi/Code/FreeCal`, branch `main`, at `879e713`. Ticket 01 (app skeleton) is implemented in `a8d0683`; the Local Store migration fixes are done in `8cd8b53` (see Task 1).

Read first:
- Ticket 01 and its leftover notes (under `## Comments`): `.scratch/freecal-v1/issues/01-app-skeleton-local-account.md`
- Spec, especially "Implementation Decisions" (including the new "Untrusted content") and "Testing Decisions": `.scratch/freecal-v1/spec.md`
- `GLOSSARY.md`, `docs/adr/0001`–`0005`, `CLAUDE.md` (code conventions and check commands)
- Ticket 29, which depends on part of this QA: `.scratch/freecal-v1/issues/29-one-running-freecal-per-local-store.md`
- The code: `git show a8d0683 --stat`, `git show 8cd8b53`

## Task 1: fix two issues in `migrate` (done)

Done in `8cd8b53`: `Core::open` refuses a Local Store with a newer schema (`CoreError::NewerLocalStore`), and `migrate` reads `user_version` inside an IMMEDIATE transaction. The test is `core/tests/local_store.rs`. The harness gained `try_open()` and `arrange_local_store(sql)` (test-only setup, never used for verifying).

Two FreeCal processes on one Local Store were discussed afterwards and settled in ADR 0005 and tickets 29 and 30; don't revisit that here. In prose, say "second launch" or "another running FreeCal", never a bare "instance" (the glossary reserves that word against Occurrences).

## Task 2: QA of the interface (done, see the last comment)

Review and harden the core application interface and its mirror in the frontend:
- Rust: `core/src/lib.rs` (`Core`, `CoreError`), `account.rs`, `clock.rs`, `signal.rs`; the shell's commands in `src-tauri/src/main.rs`.
- TypeScript: `src/core/CoreApi.ts`, `src/core/types.ts`, `src/core/tauriCore.ts`, the fake in `src/test/fakeCore.ts`.

Open points from the code review. They are judgement calls, so bring recommendations to the user before making broad changes. Recommendations were made on 2026-10-04 (see Comments) but the user hasn't answered yet; put them to the user again:
- The types are copied by hand between Rust and TypeScript (`Account`, `Provider`, `Signal`, and the `'freecal://signal'` channel name). Consider generating them (ts-rs or specta; check that the licence is GPLv3-compatible) or at least add a check that they match.
- Dates cross between the two sides as `"YYYY-MM-DD"` strings (`today`). `temporal-polyfill` is loaded anyway, because FullCalendar v7 needs it, so `Temporal.PlainDate` would fit on the TypeScript side.
- `Core::today` uses the system offset through `Clock::now() -> DateTime<FixedOffset>`. The Display Time Zone belongs to ticket 06; don't build it now.
- The main window has never been seen in the real app: the screen was locked during the earlier session. The launch itself worked (no errors, database created with the Local Account). To check visually, build with `npx tauri build --debug --no-bundle` and run `XDG_DATA_HOME=<scratch dir> ./target/debug/freecal`. The user's system locale is French, but the grid shows FullCalendar's English labels; that is expected until ticket 06.

Decided, no longer a judgement call:
- Commands turn errors into `String` (`map_err(|e| e.to_string())`), so the frontend shows English text from the core outside `t()`. `CoreError` must cross to the frontend as a code the frontend translates: ticket 29's startup error screen needs it. Ticket 29 has since taken this on (its acceptance criteria say so), so leave it to 29 unless the user decides otherwise.

Deliberately deferred; don't flag these as bugs:
- `Signal` is an empty enum, and the frontend doesn't subscribe yet. Ticket 02 adds both.
- `CalendarGrid` takes only `today`. Ticket 03 adds the Occurrences prop and the user-action callbacks.
- `main.rs` aborts with "FreeCal failed to start" when `Core::open` fails. Ticket 29 replaces this with the startup error screen.

Decision still open with the user: the app identifier `org.freecal.FreeCal`, which sets the data directory and matters for Flathub. Don't change it without asking.

## Checks

`cargo test --workspace`, `cargo clippy --workspace --all-targets`, `cargo fmt --all --check`, `npm run check`, `npm test`. The Rust checks passed at `8cd8b53`; the frontend is unchanged since `a8d0683`, where all checks passed.

## Suggested skills

- `codebase-design`: vocabulary for the interface QA (depth, seam, adapter).
- `tdd`: for any behaviour change, test-first at the core interface seam or against the fake core.
- `code-review`: review the work against `879e713` when done.
- `run`: to see the real Tauri window, if the screen is available.

## Comments

- 2026-10-04: Task 1 is done in `8cd8b53`. The open point "serialise `CoreError` as a code the frontend can translate" is no longer optional: ticket 29's startup error screen needs it (see `.scratch/freecal-v1/issues/29-one-running-freecal-per-local-store.md` and ADR 0005).
- 2026-10-04: Recommendations put to the user, not yet answered: (1) generate the TypeScript types from Rust with ts-rs (MIT), with command names and the Signal channel name staying hand-written; (2) keep `"YYYY-MM-DD"` on the wire and convert to `Temporal.PlainDate` in the `tauriCore` adapter, with `CoreApi.today()` and the fake returning `PlainDate`; (3) `CoreError` codes stay with ticket 29; (4) check the window visually with the `run` skill. Work test-first once agreed.
- 2026-10-04: Unrelated to this QA, the same session added `ICONS.md` (`8b90ca5`) and settled untrusted content (`879e713`: spec section "Untrusted content", a `CLAUDE.md` convention, and criteria in tickets 03, 09, 10, 12, 14, 23 and 25). For this QA it means descriptions will cross the interface as structured content from ticket 03 on, which argues for generated types.
- 2026-10-04: Task 2 is done. Decisions: (1) the types stay hand-written for now; whether to generate them is decided in ticket 03 (see its Comments); (2) dates stay `"YYYY-MM-DD"` on the wire, `tauriCore` turns them into `Temporal.PlainDate`, and `CoreApi`, the fake and `CalendarGrid` use `PlainDate`; (3) `CoreError` codes stay with ticket 29. Hardening: `mockIPC` tests for `tauriCore` (`src/core/tauriCore.test.ts`); an unknown Provider in the Local Store is `CoreError::UnknownProvider`, not a panic; the window's capability is narrowed to event listen/unlisten. The real window was checked: Local Account in the sidebar, month grid on today, no errors. A page scrollbar (default body margin) was fixed with `src/app.css`; only the sidebar scrolls now. The collapsible, resizable sidebar became ticket 31 and user story 122. Still open with the user: the app identifier `org.freecal.FreeCal`.
