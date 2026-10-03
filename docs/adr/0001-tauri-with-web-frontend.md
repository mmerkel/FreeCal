# Tauri with a Svelte web frontend instead of a native GTK UI

FreeCal's core (sync, storage, iCalendar and recurrence logic) is Rust, but the UI is a Svelte 5 web frontend rendered by Tauri in the system webview (WebKitGTK on Linux), not GTK4/relm4 or a pure-Rust toolkit. The hardest UI problem is a calendar grid that feels like Google Calendar (drag to move/resize, overlapping-event layout), and web technology has by far the most mature tooling for it, including FullCalendar. We accept two languages and WebKitGTK quirks in exchange.

## Considered Options

- **GTK4 via gtk-rs/relm4**: all-Rust with a native GNOME look, but the week/month grid with its drag gestures would have to be hand-drawn.
- **Slint / iced / egui**: all-Rust, but less mature for rich, text-heavy, accessible UIs.

## Consequences

- FullCalendar v7 (MIT standard edition) is wrapped in a single `CalendarGrid` component that only displays Occurrences and reports user actions. FreeCal owns all state, including which Event is selected, so the grid can be replaced and keyboard moves added later without FullCalendar's help.
- Svelte 5 with runes only, on plain Vite, without SvelteKit.
