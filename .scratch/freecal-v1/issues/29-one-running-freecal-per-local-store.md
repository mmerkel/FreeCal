# 29: One running FreeCal per Local Store

**What to build:** Only one FreeCal runs per Local Store (ADR 0005). Starting FreeCal again brings the running one to the front and the second launch exits, passing on its command-line arguments. If the core can't open its Local Store for any reason, the window shows a translated startup error screen instead of FreeCal failing to appear. Spec: user stories 117 and 121, and "One running FreeCal" and "Startup failures" under Implementation Decisions.

**Blocked by:** 01

**Status:** ready-for-agent

- [ ] `Core::open` takes an exclusive OS file lock (`File::try_lock`) in the data directory for the lifetime of `Core`, and fails with a new `CoreError::LocalStoreInUse` when another process holds it
- [ ] Tested at the core seam: a second open of the same Local Store is refused; it opens again once the first `Core` is dropped
- [ ] The shell uses `tauri-plugin-single-instance`: a second launch hands its arguments to the running FreeCal, which shows and focuses its window, then exits. The forwarded arguments are received but not acted on yet (ticket 30). Re-creating a window closed to the tray belongs to ticket 13
- [ ] Startup failures (`NewerLocalStore`, `LocalStoreInUse`, a damaged Local Store) open the window on an error screen; closing it quits FreeCal
- [ ] Core errors cross to the frontend as codes the frontend translates through `t()`, not as English strings (this settles the matching open point in `.scratch/freecal-v1/handoff-db-and-interface-qa.md`)
- [ ] The hand-off is checked by hand in the real app: start FreeCal, start it again, and see one window come to the front
