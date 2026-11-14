# One running FreeCal per user session

Only one FreeCal process may run in a user session, whatever Local Store it opens. A second launch hands its command-line arguments (files to open) to the running FreeCal, which comes to the front, and then exits. Sync, reminders, the tray and Signals all assume they are alone: Signals reach only one process, two syncs would send the same queued change twice, and every reminder would fire twice. FreeCal runs in the background, so starting it again from the launcher while it sits in the tray is the normal way to reach a second launch, not an edge case.

The rule is enforced twice. The core takes an exclusive OS file lock in the data directory for the lifetime of `Core` and refuses to open an already-locked Local Store, which is the real guarantee for the Local Store and is testable at the core interface. The shell does the hand-off, which provides the user experience and enforces the per-session rule: the first launch owns the D-Bus name `io.github.mmerkel.FreeCal` (ADR 0006), and a second launch passes its arguments, working directory and activation token to it in one call. A FreeCal that should run next to another one, such as an agent's test run, needs its own D-Bus session.

## Considered Options

- **Several processes sharing one Local Store**: every feature would need cross-process coordination (Signals between processes, a sync lease, deduplicated reminders and tray notifications), for no benefit to a single-user desktop app.
- **One running FreeCal per Local Store** (decided first): a development build with its own data directory could run next to the real FreeCal, because the hand-off's D-Bus name carried a hash of the data directory. Rejected because two FreeCals in one session still share everything outside the Local Store: the tray, start at login, notifications and the app ID the desktop knows them by. With the app ID set on the window (ticket 28), both windows would also be one app to the desktop.
- **`tauri-plugin-single-instance` for the hand-off** (used at first): it forwards only the arguments and the working directory. Without the second launch's activation token, Wayland compositors won't raise the running FreeCal's window; KWin only flashes it in the taskbar.
- **SQLite's exclusive locking mode, or a PID file, instead of an OS file lock**: SQLite's mode only blocks others once the right lock is taken, which is hard to reason about. A PID file can be left behind after a crash. An OS file lock is released by the OS when the process dies, and Rust's `File::try_lock` works on Linux, Windows and macOS.

## Consequences

- "Files to open" is one input with two sources: forwarded command-line arguments on Linux and Windows, and the system's open-file events (`RunEvent::Opened`) on macOS, where the OS itself keeps one app running.
- A second launch on another data directory hands off to the running FreeCal too, which keeps its own Local Store.
- When a second launch can't hand off (a race before the hand-off is in place), it fails with `LocalStoreInUse`, which is shown on the startup error screen like any other startup failure.
