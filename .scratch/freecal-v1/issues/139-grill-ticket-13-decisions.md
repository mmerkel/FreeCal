# 139: Grilling: ticket 13's decisions for review

**What to build:** A grilling session with the user (/grilling) on the decisions ticket 13's implementation made while the user was away, marked "decision, for review" in its comments of 2026-11-10. Each is confirmed, changed or turned into a follow-up ticket, and the outcome is recorded below and in ticket 13. Spec: US-103–US-108 and US-129; "Quitting"; `docs/research/autostart-state.md`.

**Blocked by:** none

**Status:** ready-for-human

**Effort:** S

The decisions, with the strongest con of each (details in ticket 13's comments):

- **Core layer**
  - [ ] Start at login is a stored setting (`Settings::start_at_login`), not read from the autostart entry. Confirmed by the user on 2026-11-10 after the research; listed here only so it isn't asked again. Con: the switch shows on after the entry is removed outside FreeCal.
  - [ ] A stored on/off setting that isn't 0 or 1 reads as its default (`stored_switch`). Con: the sidebar's `shown` flag uses another rule (anything but 0 is on).
- **Shell layer**
  - [x] For background running the window is destroyed, not hidden, and created again (`reopen_main_window`). **Changed by the user on 2026-11-14:** the window is hidden, and closing keeps open dialogs and unsaved edits, so nothing asks "Discard changes?" (ADR 0016). Before that, the user accepted this for now on 2026-11-10, on one condition: closing the window must act as Cancel on whatever dialog is open. Still open: does closing with unsaved changes in the event editor ask "Discard changes?" as Cancel does (the shell asks the window to cancel first and closes once it answers, or after a couple of seconds without an answer), or drop them silently? The shell now lets go of a picked import file on close (see the frontend layer).
  - [ ] Start at login: inside Flatpak only the Background portal is asked; outside Flatpak FreeCal writes its own autostart file and never asks the portal (changed after the review: there the portal named the entry after the terminal's app ID). Con: outside Flatpak the desktop never asks the user.
  - [ ] No `org.freedesktop.host.portal.Registry.Register` call. Since the review this matters only if the host build should use the portal again; outside Flatpak FreeCal now always writes its own file. Con: should a later portal version stop the host from writing `~/.config/autostart` itself, FreeCal has no app ID to offer it.
  - [ ] At login, FreeCal opens its window as at any launch, not straight into the tray. Con: the window pops up at every login.
  - [ ] Tray Quit in the background quits at once without trying to send the Unsent Changes. Con: spec "Quitting" says tray Quit tries first.
  - [ ] The tray menu's labels are English text in the shell. Con: no translations until the shell has an i18n.
  - [ ] A close goes to the background only with a tray host and FreeCal's own icon; unreadable settings quit; the tray check waits 1 s at most. Con: a hung desktop delays the close by up to that second.
  - [ ] FreeCal ends only through `exit`; an exit request without a code is refused. Con: after a webview crash FreeCal keeps running with only the tray to quit it.
- **Frontend layer**
  - [ ] Both switches are the system's checkboxes marked as switches. Con: they look like checkboxes, not sliding switches.
  - [ ] The Start at login switch is disabled while the desktop is asked. Con: while the portal's dialog waits, it is greyed out without saying why.
  - [ ] A refusal reuses "The setting couldn’t be saved: …". Con: "saved" is slightly off when the desktop said no.
  - [x] **Gone with ADR 0016 (2026-11-14):** the picked import file stays held while its dialog stays open in the hidden window. Was: a picked import file is let go when the window is destroyed, unless a preview or import of it is running then. Con: that file stays held until the next pick, as does one a file dialog still open at the close picks afterwards.
- **Left open by the review** (ticket 13's comment of 2026-11-10, "fixes")
  - [x] The tray icon and its menu's actions have no fake or test; they were checked by hand on Xvfb only, so ticket 13's box on fakeable desktop interfaces stays open. Accept the hand check, or add a fake (the menu's choices as a function of whether a window exists and startup failed, testable without a tray)? Con of accepting: a change to the tray's Quit is caught by no test. **Accepted by the user on 2026-11-14:** the hand check stands, no fake; the user checked it themselves and it works. Ticket 13's box is ticked.
  - [x] **Gone with ADR 0016 (2026-11-14):** the window is never created again, and a hidden one listens. Was: a quit asked of a window that was just created again (Open FreeCal, then Quit at once) can arrive before the window listens, and nothing happens. Fix (the window tells the shell once it listens, and the shell waits for that) or accept? Con of accepting: a Quit that seems ignored once in a while.
  - [ ] Closing twice quickly can ask the window to quit twice. Con of accepting: the quit question may show twice.
  - [ ] Judgement calls from the standards review, each fixable later: the Background section repeats layout `src/app.css` has and hard-codes WebKitGTK's 13px checkbox width for the hint; `StartupFailed` and `StartupFailure` are near-identical names for different things; the quit-asked emit and the `StartupFailed` checks are repeated in `src-tauri/src/main.rs`; `closing(…)` takes bare booleans. Con of fixing now: churn in code that works, for readability alone.

## Comments
