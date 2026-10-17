# 67: Design the app icon and the tray icons

**What to build:** FreeCal's own icons, drawn by hand. The app icon replaces the placeholder `src-tauri/icons/icon.png` (512×512) and is used for the window, the taskbar and the bundles. The tray icons cover the tray icon of ticket 13 and one per overall status of ticket 23: Online and synced, Syncing, Offline, Sync Error and Conflict. They follow the visual style in the spec: neutral and flat.

**Blocked by:** none

**Status:** ready-for-human

- [ ] An app icon drawn as SVG, kept in the repo as the source, that still reads at 16×16 and 32×32
- [ ] PNGs exported from it in the sizes the Tauri bundles need (at least 32, 128, 256 and 512, plus `icon.ico`/`icon.icns` if Windows or macOS builds are kept possible), and `bundle.icon` in `tauri.conf.json` listing them
- [ ] A tray icon per overall status: Online and synced is the plain icon; Syncing, Offline, Sync Error and Conflict add a badge or mark. Conflict and Sync Error must be told apart at tray size (about 22px, and 16px on some panels), and not by colour alone
- [ ] Tray icons that work on light and dark panels: either monochrome symbolic icons (GNOME AppIndicator, KDE Plasma) or a variant per panel shade; the choice is noted below
- [ ] The tray icons are kept as SVG sources with PNGs exported at the sizes the tray code loads, in a place tickets 13 and 23 can use

## Comments

- 2026-10-17: Added on request. No tray code exists yet (tickets 13 and 23 are open), so the tray icons can be delivered before or alongside them.
