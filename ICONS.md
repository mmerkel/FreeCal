# FreeCal's icons

FreeCal's icons are drawn as SVGs kept in the repo. Tauri's build and the tray can't use SVGs, so a script renders the PNGs they need from them. Edit only the SVGs; the PNGs are generated.

## Layout

```
src-tauri/icons/
  generate.sh             renders app/ and tray/ from src/
  generate-dev.sh         generate.sh, then installs the dev launcher's icon
  src/                    the hand-drawn SVGs, the only files to edit
    app.svg               the app icon: window, taskbar, bundles
    tray-<status>.svg     a tray icon per overall status (ticket 67)
  app/                    generated: <size>x<size>.png, listed in bundle.icon
  tray/                   generated: <status>.png, built into the binary
```

The tray icons are named after the overall status they show: `tray-synced.svg` (Online and synced), `tray-syncing.svg`, `tray-offline.svg`, `tray-sync-error.svg` and `tray-conflict.svg`. Each becomes `tray/<status>.png`, e.g. `tray/sync-error.png`.

## Generating the PNGs

After changing an SVG, run

```sh
src-tauri/icons/generate.sh
```

and commit the SVG together with the PNGs. The script renders:

- `src/app.svg` into `app/` at 32, 48, 64, 128, 256 and 512 pixels.
- every `src/tray-*.svg` into `tray/` at 64×64. The tray takes one image per icon and the panel scales it down, to about 22px or 16px. If an icon doesn't read at those sizes, change the SVG; a smaller export doesn't help.

It renders with `tauri icon` from `@tauri-apps/cli`, whose version `package-lock.json` pins, so the same SVG gives the same bytes on every machine. The pre-commit hook runs `generate.sh --check` when a commit changes `src-tauri/icons/`; it fails if a PNG doesn't match its SVG. The warnings `tauri icon` prints about fonts are hidden and don't matter, as long as the SVGs have no text (see below).

For a dev build, `src-tauri/icons/generate-dev.sh` runs `generate.sh` and then copies `app/512x512.png` to `~/.local/share/icons/hicolor/512x512/apps/freecal-dev.png`, the icon of the dev launcher (ticket 28).

To change the app icon sizes, change `APP_SIZES` in `generate.sh` and the list in `bundle.icon` in `src-tauri/tauri.conf.json` together.

## Using the PNGs

- **App icon:** `bundle.icon` in `src-tauri/tauri.conf.json` lists the `app/` PNGs. Tauri uses them for the bundles (`.deb`, `.rpm`, AppImage), which install each under `share/icons/hicolor/<size>x<size>/apps/`, and builds one into the binary as the window icon.
- **Tray icons:** the tray code builds the `tray/` PNGs into the binary (`include_bytes!`) and shows the one for the current overall status (ticket 23). Until then the tray shows the window icon (`src-tauri/src/tray.rs`).

## What Tauri accepts

- PNGs must be square and **RGBA** (with an alpha channel). An RGB-only or palette PNG fails the build with "icon … is not RGBA". `tauri icon` writes RGBA.
- SVG is **not** accepted in `bundle.icon`.
- `.ico` (Windows) and `.icns` (macOS) are only needed for those platforms. v1 targets Linux. Without `--png`, `npx tauri icon <svg>` generates the full set for every platform; add that to `generate.sh` once Windows or macOS builds are wanted.

## Drawing the SVGs

- Use a square `viewBox` and a transparent background.
- Convert any text to paths. The renderer loads system fonts for `<text>`, so the result would depend on the machine that runs the script.
- Check that the app icon still reads at 32×32, and the tray icons at 16×16. Fine detail disappears at small sizes.
- Tray icons must work on light and dark panels (ticket 67).

## Flathub (ticket 28)

A Flatpak build doesn't use Tauri's bundler: the Flatpak manifest installs the icons itself. There `src/app.svg` can be used directly:

```
share/icons/hicolor/scalable/apps/<app-id>.svg
```

The file must be named after the app ID, `io.github.mmerkel.FreeCal` (ADR 0006). Check Flathub's current icon requirements when doing ticket 28.
