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
  - [ ] For background running the window is destroyed, not hidden, and created again (`reopen_main_window`). The user accepted this for now on 2026-11-10, on one condition: closing the window must act as Cancel on whatever dialog is open. Still open: does closing with unsaved changes in the event editor ask "Discard changes?" as Cancel does (the shell asks the window to cancel first and closes once it answers, or after a couple of seconds without an answer), or drop them silently? The shell also has to discard a picked import file on close.
  - [ ] Start at login asks the Background portal first and, outside Flatpak, writes FreeCal's own autostart file where the portal didn't. Con: two writers own one file.
  - [ ] No `org.freedesktop.host.portal.Registry.Register` call. Con: a build started from a terminal always takes the file path.
  - [ ] At login, FreeCal opens its window as at any launch, not straight into the tray. Con: the window pops up at every login.
  - [ ] Tray Quit in the background quits at once without trying to send the Unsent Changes. Con: spec "Quitting" says tray Quit tries first.
  - [ ] The tray menu's labels are English text in the shell. Con: no translations until the shell has an i18n.
  - [ ] A close goes to the background only with a tray host and FreeCal's own icon; unreadable settings quit; the tray check waits 1 s at most. Con: a hung desktop delays the close by up to that second.
  - [ ] FreeCal ends only through `exit`; an exit request without a code is refused. Con: after a webview crash FreeCal keeps running with only the tray to quit it.
- **Frontend layer**
  - [ ] Both switches are the system's checkboxes marked as switches. Con: they look like checkboxes, not sliding switches.
  - [ ] The Start at login switch is disabled while the desktop is asked. Con: while the portal's dialog waits, it is greyed out without saying why.
  - [ ] A refusal reuses "The setting couldn’t be saved: …". Con: "saved" is slightly off when the desktop said no.
  - [ ] A picked import file is let go when the window is destroyed, unless its import is running then. Con: that file stays held until the next pick.

## Comments
