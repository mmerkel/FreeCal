# 65: Text keeps its weight when the sidebar starts to scroll

**What to build:** With subpixel font smoothing (the user's KDE setup: `Xft.rgba: rgb`, slight hinting) and GPU rendering, the sidebar's text turns visibly heavier once it holds enough Calendars to scroll. It goes back when it no longer scrolls. Text should look the same either way. The fix is `-webkit-font-smoothing: antialiased` on `body` in `src/app.css`, so all text in the window is drawn with grey smoothing. The run-freecal driver gets an option to render as the user's machine does, so this and later text-rendering issues can be checked by hand.

**Blocked by:** none

**Status:** ready-for-agent

- [ ] `body` in `src/app.css` sets `-webkit-font-smoothing: antialiased`, with a comment saying why (the scrolling sidebar's own layer changes its smoothing)
- [ ] `.claude/skills/run-freecal/freecal.sh` gets an option (e.g. `start --subpixel`, and the same for `restart`) that runs with subpixel RGB smoothing and slight hinting through a scratch `XDG_CONFIG_HOME` (`gtk-xft-rgba=rgb`) and GPU rendering on llvmpipe (`LIBGL_ALWAYS_SOFTWARE=1`, without `WEBKIT_DISABLE_DMABUF_RENDERER`); `SKILL.md` documents it and when to use it
- [ ] Checked by hand with that option, results noted below: crops of the sidebar's heading and rows with 2 and with 32 Calendars, zoomed, are identical; the toolbar and grid text look as intended

## Comments

- 2026-10-17: From `TODO.md` ("once sidebar needs scrolling bc of many calendars, font changes to bold"), settled in a grilling session. Checked in WebKitGTK 2.52.6 (Skia): with the driver's defaults (CPU rendering, grey smoothing) 2 and 32 Calendars render pixel-identical; it only shows with subpixel smoothing and GPU rendering. Once the sidebar overflows, WebKit composites it in its own layer and draws its text differently there. Tried without effect: `background-attachment: local`, `transform: translateZ(0)`, `will-change: transform`, `contain: paint`. `-webkit-font-smoothing: antialiased` on `.sidebar` alone fixes it but leaves the sidebar drawn differently from the grid; on `body` all text matches. The user compared builds with and without it on their own screen and chose `body`. The probe used a throwaway launcher (`gl.sh` in a session scratchpad); the driver option replaces it.
