# 01: App skeleton showing the Local Account in an empty month view

**What to build:** FreeCal launches as a Tauri desktop app (Rust core, Tauri shell, Svelte 5 frontend, per ADR 0001). On first launch the Local Account exists, the sidebar shows it, and an empty month view opens on today. Everything the frontend does goes through the core application interface. This ticket also sets up the test seams and the conventions every later ticket relies on.

**Blocked by:** None (can start immediately)

**Status:** done

- [x] A Rust core library, a Tauri shell exposing the core as commands and forwarding Signals as Tauri events, and a Svelte 5 frontend (runes only, plain Vite, no SvelteKit); the shipped binary embeds the compiled frontend
- [x] The Local Store is a SQLite database in the XDG data directory (ADR 0004)
- [x] The Local Account exists from the first launch and cannot be removed (the core refuses)
- [x] `CalendarGrid` wraps FullCalendar v7 (standard edition), only displays the Occurrences it's given and reports user actions; it shows an empty month view on today
- [x] All UI strings go through an i18n layer (English only)
- [x] Core test harness: real SQLite in a temporary directory, a controllable clock, a recording Signal subscriber
- [x] Frontend test harness: Vitest component tests against a fake core interface that can also emit Signals
- [x] `CLAUDE.md` gains the rule that Svelte code uses Svelte 5 runes only
- [x] All dependencies are GPLv3-compatible (ADR 0003)

## Comments

Implemented. Notes for later tickets:

- The core's `Signal` enum is empty; each ticket adds the Signal it first sends. The frontend does not subscribe yet: ticket 02 adds subscribe-before-read.
- `CalendarGrid` takes only `today` for now. The Occurrences prop and the user-action callbacks arrive with ticket 03.
- FullCalendar's own labels (month title, weekday names) come from its default locale, not from `t`. Ticket 06 should wire FullCalendar's locale.
- The app identifier `org.freecal.FreeCal` sets the data directory (`~/.local/share/org.freecal.FreeCal`). Settle it before the first release (ticket 28, Flathub).
