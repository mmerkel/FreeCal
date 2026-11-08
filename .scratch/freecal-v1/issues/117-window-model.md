# 117: The Window Model takes the window's reads and state out of App.svelte

**What to build:** The second part of ticket 113 (read it for the why). Move what the window shows, and keeping it current, out of `src/App.svelte` into the **Window Model** (`src/lib/windowModel.svelte.ts`), a class, `new WindowModel(core)`, whose `start()` returns the function that stops it. No change to what the user sees or to the core interface.

- startup: the startup failure, subscribing before the first read, the settings before the window shows, then today, the last-used view, the sidebar, the Accounts and Calendars and the Sync Statuses;
- the reads of the Accounts and Calendars, the Sync Statuses, the Occurrences of the visible range and the Event whose details are open, each with one shared "newest read wins" helper in place of the four counters, and their read failure. Which Event's details are open stays App.svelte's until ticket 118; the model is told;
- which Signal re-reads what (today's `onSignal`);
- today, the navigation and the Date changed hold: `hold()` returns its release, and a held "Date changed" moves the view once the last holder releases. The grid's press and a drop's scope prompt each take one (the scope prompt is still in App.svelte until ticket 118);
- the remembered view, sidebar and settings: shown at once, written in the background, and the settings put back to what the Local Store kept when the write fails;
- the Sync Statuses as shown, with `SyncingDelay` applied, stopped with the model.

The Window Model only reads; the Signals bring writes back. The remembered view, sidebar and settings are the exception, as they are the window's own state and send no Signal. The one-second clock and the overall status's words stay with what renders them. Svelte 5 runes only; read the class fields as `model.x` in components and never destructure them, which loses their reactivity.

Spec: "Testing Decisions" (Seam 2); "Signals"; US-68.

**Blocked by:** 116

**Status:** ready-for-agent

**Effort:** M

- [ ] `App.svelte` holds no reads, no Signal handling, no failure state of the reads, no today, navigation or Date changed hold, and no remembered view, sidebar or settings state
- [ ] The Window Model's tests, on `RecordingCore` without rendering, cover: subscribing before the first read; an older read never overwriting a newer one, for each kind of read; each Signal's re-reads; a "Date changed" held by a press and by a drop's scope prompt, and followed once the last hold is released; a failed settings write putting back the stored settings
- [ ] The existing `App.*.test.ts` and `Sidebar*.test.ts` tests pass unchanged, on the old `FakeCore`
