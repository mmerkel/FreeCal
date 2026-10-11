# 33: Settings dialog, main menu and appearance

**What to build:** A ⋮ main menu at the right end of the toolbar offers Settings…, About FreeCal and Quit. Settings opens a modal dialog where every change takes effect at once and is kept in the Local Store. Its first setting is Appearance: System, Light or Dark. Tickets 06 and 13 add their settings to the same dialog. Spec: user stories 125, 126 and 130, "App state and settings" in the core application interface, "Startup failures" and "Settings" under Frontend.

**Blocked by:** 04, 32

**Status:** done

- [x] A ghost ⋮ "Main menu" button at the right end of ticket 04's toolbar opens a menu: Settings…, About FreeCal, Quit (no Keyboard shortcuts item until a ticket builds that overview)
- [x] Ctrl+, opens Settings, handled with ticket 04's shortcuts
- [x] Settings is a modal dialog built on ticket 32's `Dialog`, with sections in this order: Date & time (filled by ticket 06), Appearance, Background (filled by ticket 13). A section with no settings yet isn't shown
- [x] Appearance offers System, Light and Dark; System is the default and follows `prefers-color-scheme` live
- [x] Every change takes effect and is saved at once; there is no Save or Cancel button
- [x] Settings are kept in the Local Store and read and written through the core interface's "app state and settings" operations. A write is a partial update (only the fields given change), so that a later Save button would need no core change (core test)
- [x] The frontend holds its own defaults for every setting (`system` for each override) and uses them until the core has answered, and for good if the core can't open the Local Store, so that the startup error screen is drawn with them
- [x] The window is created hidden and shown once the core has sent the settings or a startup failure is known, so that it never flashes the wrong theme
- [x] About FreeCal shows the name, the version, "Licensed under the GNU GPL, version 3 or later", the copyright line and a link to the source code; links open in the system browser through the shell, never in the window, and the CSP is unchanged
- [x] Quit ends FreeCal even when background running is on (ticket 18 later adds a warning about unsent changes in front of it)
- [x] Every string goes through `t()`
- [x] Component tests against the fake core: the menu and Ctrl+,, changing Appearance writes it once and applies it, defaults are used when the fake core reports a startup failure, About's link goes through the shell
- [x] Checked by hand in the real app with `run-freecal`: no flash at startup with Dark chosen on a light system

Out of scope: settings for sync timings, retention values and the default view; a keyboard shortcuts overview.

## Comments

- 2026-10-11: From the ticket 04 review: the shortcut keys are listed twice, in the `shortcuts` map in `src/App.svelte` and as the hint letters in `src/lib/Toolbar.svelte`, kept in step by hand. Before adding Ctrl+, here, merge them into one table that both read, so a later keyboard shortcuts overview can read it too.
- 2026-10-11: Done. The shortcut keys now live in one table, `src/lib/shortcuts.ts`, which the key handling, the toolbar hints and the main menu's `Ctrl+,` hint all read. Ctrl shortcuts also act while typing, as Ctrl+, does in most apps; plain-letter ones still don't. The window starts hidden and the frontend shows it once the theme is applied (`show_window`); as a safety net the shell shows it after 5 s anyway. On the frontend, a startup failure is drawn with the default settings and then shows the window. The shell side belongs to ticket 29: today a failed `Core::open` still ends FreeCal before any window appears, so ticket 29 must open the window on its error screen. The copyright line reads "Copyright © 2026 Matthias Merkel" (`about.copyright`); change it there if it should name someone else. Checked by hand with `run-freecal`: with Dark stored on a light system, the X window first maps about 0.5 s after launch, and every frame from then on is dark.
