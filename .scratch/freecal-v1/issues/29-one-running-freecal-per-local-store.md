# 29: One running FreeCal per Local Store

**What to build:** Only one FreeCal runs per Local Store (ADR 0005). Starting FreeCal again brings the running one to the front and the second launch exits, passing on its command-line arguments. If the core can't open its Local Store for any reason, the window shows a translated startup error screen instead of FreeCal failing to appear. Spec: user stories 117 and 121, and "One running FreeCal" and "Startup failures" under Implementation Decisions.

**Blocked by:** 01

**Status:** done

**Effort:** S

- [x] `Core::open` takes an exclusive OS file lock (`File::try_lock`) in the data directory for the lifetime of `Core`, and fails with a new `CoreError::LocalStoreInUse` when another process holds it
- [x] Tested at the core seam: a second open of the same Local Store is refused; it opens again once the first `Core` is dropped
- [x] The shell does the hand-off (at first with `tauri-plugin-single-instance`, now over D-Bus itself; see Comments): a second launch hands its arguments to the running FreeCal, which shows and focuses its window, then exits. The forwarded arguments are received but not acted on yet (ticket 30). Re-creating a window closed to the tray belongs to ticket 13
- [x] Startup failures (`NewerLocalStore`, `LocalStoreInUse`, a damaged Local Store) open the window on an error screen; closing it quits FreeCal
- [x] Core errors cross to the frontend as codes the frontend translates through `t()`, not as English strings (this settles the matching open point in `.scratch/freecal-v1/handoff-db-and-interface-qa.md`)
- [x] The hand-off is checked by hand in the real app: start FreeCal, start it again, and see one window come to the front

## Comments

- 2026-10-17: Done. Notes for later tickets:
  - The hand-off uses a D-Bus name derived from the data directory (`local_store_name` in `src-tauri/src/handoff.rs`), so FreeCals on separate data directories run side by side (ADR 0005). It is Linux-only; Windows and macOS need their own hand-off when they are ported.
  - Failures cross the interface as `ErrorCode` (`{ code }`, generated into `types.generated.ts`), put into words by `errorText` and `startupText` in `src/i18n`. The shell's own command failures (`linkNotAllowed`, `linkNotOpened`, `windowNotShown`) share that enum.
  - Only a failed `Core::open` makes closing the window quit explicitly. A start that fails later (a read rejecting after the core opened) shows the same screen but relies on the default close behaviour; ticket 13 must keep that quitting once closing goes to the tray.
  - The hand-off was checked on Xvfb with no window manager: the second launch exits 0 and the covered window comes to the front.
- 2026-10-17: On KWin (Wayland) the plugin's hand-off only flashed FreeCal in the taskbar: it doesn't forward the second launch's activation token. The shell now does the hand-off itself over D-Bus (`src-tauri/src/handoff.rs`) and passes `XDG_ACTIVATION_TOKEN` (or `DESKTOP_STARTUP_ID` on X11), which the running FreeCal sets on its window before presenting it. A launch from a terminal usually has no token, so only a launch through a `.desktop` file shows the difference.
- 2026-10-17: Confirmed on the user's desktop: launched twice through a `.desktop` file, KWin (Wayland) brings the running FreeCal's window to the front.
