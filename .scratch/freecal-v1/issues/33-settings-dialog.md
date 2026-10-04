# 33: Settings dialog, main menu and appearance

**What to build:** A ⋮ main menu at the right end of the toolbar offers Settings…, About FreeCal and Quit. Settings opens a modal dialog where every change takes effect at once and is kept in the Local Store. Its first setting is Appearance: System, Light or Dark. Tickets 06 and 13 add their settings to the same dialog. Spec: user stories 125, 126 and 130, "App state and settings" in the core application interface, "Startup failures" and "Settings" under Frontend.

**Blocked by:** 04, 32

**Status:** ready-for-agent

- [ ] A ghost ⋮ "Main menu" button at the right end of ticket 04's toolbar opens a menu: Settings…, About FreeCal, Quit (no Keyboard shortcuts item until a ticket builds that overview)
- [ ] Ctrl+, opens Settings, handled with ticket 04's shortcuts
- [ ] Settings is a modal dialog built on ticket 32's `Dialog`, with sections in this order: Date & time (filled by ticket 06), Appearance, Background (filled by ticket 13). A section with no settings yet isn't shown
- [ ] Appearance offers System, Light and Dark; System is the default and follows `prefers-color-scheme` live
- [ ] Every change takes effect and is saved at once; there is no Save or Cancel button
- [ ] Settings are kept in the Local Store and read and written through the core interface's "app state and settings" operations. A write is a partial update (only the fields given change), so that a later Save button would need no core change (core test)
- [ ] The frontend holds its own defaults for every setting (`system` for each override) and uses them until the core has answered, and for good if the core can't open the Local Store, so that the startup error screen is drawn with them
- [ ] The window is created hidden and shown once the core has sent the settings or a startup failure is known, so that it never flashes the wrong theme
- [ ] About FreeCal shows the name, the version, "Licensed under the GNU GPL, version 3 or later", the copyright line and a link to the source code; links open in the system browser through the shell, never in the window, and the CSP is unchanged
- [ ] Quit ends FreeCal even when background running is on (ticket 18 later adds a warning about unsent changes in front of it)
- [ ] Every string goes through `t()`
- [ ] Component tests against the fake core: the menu and Ctrl+,, changing Appearance writes it once and applies it, defaults are used when the fake core reports a startup failure, About's link goes through the shell
- [ ] Checked by hand in the real app with `run-freecal`: no flash at startup with Dark chosen on a light system

Out of scope: settings for sync timings, retention values and the default view; a keyboard shortcuts overview.
