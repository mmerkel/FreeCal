# 28: .deb, .rpm and AppImage packaging

**What to build:** FreeCal can be installed from a `.deb`, an `.rpm` or run as an AppImage, all built by Tauri. All OS integration goes through portal-friendly APIs so a later Flathub release needs no code changes.

**Blocked by:** 01

**Status:** ready-for-agent

**Effort:** M

- [ ] The build produces `.deb`, `.rpm` and AppImage
- [ ] Each package installs or runs and launches FreeCal on a clean system
- [ ] Autostart, keyring, file dialogs and network status use portal-friendly APIs (checked as features land)
- [ ] A manual release checklist against a real Google test account is documented
- [ ] FreeCal's window reports the app ID `io.github.mmerkel.FreeCal` (Wayland app_id, X11 `WM_CLASS`), so the desktop finds its `.desktop` file and shows FreeCal's icon in the panel and the window decorations, not a generic one
- [ ] The packages install `io.github.mmerkel.FreeCal.desktop` (with `MimeType=text/calendar;`, spec "Opening files") and the icon in the `hicolor` sizes under that name; the sizes are made from `src-tauri/icons/base.svg`

## Comments

- 2026-11-14: On KDE Plasma (Wayland) FreeCal's panel entry and title bar show KWin's placeholder (a white W on yellow), while the tray icon is right. Under Wayland the window can't hand its icon to the desktop: KWin looks up the `.desktop` file named after the window's app_id. Tauri leaves the GTK application ID unset unless `app.enableGTKAppId` is on (`tauri-2.12.1/src/app.rs:2446`), so GTK takes the binary name and the app_id is `freecal`. For development, `~/.local/share/applications/freecal-dev.desktop` now has `StartupWMClass=freecal` and an `Icon=` path to `src-tauri/icons/icon.png`.
  - **decision:** the shell sets the GLib program name to the app ID before Tauri builds its event loop; GTK3 uses it for the Wayland app_id and the X11 `WM_CLASS` when no application ID is set, and claims no bus name. Strongest con: it relies on GTK3's fallback from the application ID to the program name, not on a Tauri setting.
  - Rejected: `app.enableGTKAppId: true`. Tested on a private session bus and Xvfb, with a debug build built with that setting and two launches on one data directory: tao registers a unique `GApplication` named after the app ID, and a second launch only forwards "activate" to it and exits with no output (`tao-0.37.1/src/platform_impl/linux/event_loop.rs:1062-1069`), before the hand-off of ticket 29 runs, so its files and activation token are lost. The running FreeCal takes the forwarded "activate" as a second start, reruns Tauri's setup, and panics: `Failed to setup app: a webview with label `main` already exists` (`tauri-2.12.1/src/app.rs:1444`).
- 2026-11-14: Plasma's panel finds a window's `.desktop` file by its `StartupWMClass` too, but KWin's title bar only by the file name `<app_id>.desktop`. So the dev launcher is now `~/.local/share/applications/freecal.desktop` (`StartupWMClass=freecal`, `Icon=freecal-dev`, the icon in `~/.local/share/icons/hicolor/512x512/apps/freecal-dev.png`), and the panel and the title bar both show FreeCal's icon. With the app ID decision above, the app_id becomes `io.github.mmerkel.FreeCal`, the name the packaged `.desktop` file takes; a dev launcher of that name in `~/.local/share/applications` would hide the installed one. How a dev build gets its icon then is part of this ticket: e.g. the dev launcher takes the packaged name on a machine without the package, or dev builds keep the program name `freecal`.
