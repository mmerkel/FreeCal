# App icons from an SVG

FreeCal's master icon is meant to be an SVG kept in the repo. The Tauri build can't use SVGs, so the PNGs it needs are generated from it.

## What Tauri accepts

- `bundle.icon` in `src-tauri/tauri.conf.json` lists the icon files. Tauri uses them for the bundles (`.deb`, `.rpm`, AppImage) and embeds one into the binary as the window icon.
- PNGs must be square and **RGBA** (with an alpha channel). An RGB-only or palette PNG fails the build with "icon … is not RGBA".
- SVG is **not** accepted in `bundle.icon`.
- `.ico` (Windows) and `.icns` (macOS) are only needed for those platforms. v1 targets Linux.
- Several resolutions are possible: list one PNG per size. The Linux bundles install each PNG under `share/icons/hicolor/<size>x<size>/apps/`, taking the size from its actual pixel dimensions.

## Preparing the SVG

- Use a square `viewBox`, for example `0 0 512 512`, and a transparent background.
- Convert any text to paths. The renderer loads system fonts for `<text>`, so the result would depend on the machine that runs the command.
- Check that it still reads at 32×32. Fine detail disappears at small sizes.

## Generating the PNGs

For Linux only, generate just the PNG sizes. With `--png`, `tauri icon` writes only those sizes, named `<size>x<size>.png`, and nothing for other platforms. It leaves the current `icon.png` alone; delete that file once the new ones are listed in `bundle.icon`.

```sh
npx tauri icon src-tauri/icons/freecal.svg -o src-tauri/icons --png 32,48,64,128,256,512
```

Then list the files in `src-tauri/tauri.conf.json`:

```json
"icon": [
  "icons/32x32.png",
  "icons/48x48.png",
  "icons/64x64.png",
  "icons/128x128.png",
  "icons/256x256.png",
  "icons/512x512.png"
]
```

Without `--png`, `npx tauri icon <svg>` generates the full set for every platform: the PNG sizes, `icon.ico`, `icon.icns`, the Windows Store logos and the `android/` and `ios/` folders. Use that once Windows or macOS builds are wanted. Delete what isn't needed, and keep `bundle.icon` in line with what's left.

Regenerate the PNGs whenever the SVG changes, and commit both.

## Flathub (ticket 28)

A Flatpak build doesn't use Tauri's bundler: the Flatpak manifest installs the icons itself. There the SVG can be used directly:

```
share/icons/hicolor/scalable/apps/<app-id>.svg
```

The file must be named after the app ID, `io.github.mmerkel.FreeCal` (ADR 0006). Check Flathub's current icon requirements when doing ticket 28.
