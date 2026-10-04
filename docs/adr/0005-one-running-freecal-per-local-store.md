# One running FreeCal per Local Store

Only one FreeCal process may have a given Local Store open at a time. A second launch hands its command-line arguments (files to open) to the running FreeCal, which comes to the front, and then exits. Sync, reminders, the tray and Signals all assume they are alone: Signals reach only one process, two syncs would send the same queued change twice, and every reminder would fire twice. FreeCal runs in the background, so starting it again from the launcher while it sits in the tray is the normal way to reach a second launch, not an edge case.

The rule is enforced twice. The core takes an exclusive OS file lock in the data directory for the lifetime of `Core` and refuses to open an already-locked Local Store, which is the real guarantee and is testable at the core interface. The shell uses `tauri-plugin-single-instance` for the hand-off, which provides the user experience. Separate Local Stores (a development build with its own data directory) may run side by side.

## Considered Options

- **Several processes sharing one Local Store**: every feature would need cross-process coordination (Signals between processes, a sync lease, deduplicated reminders and tray notifications), for no benefit to a single-user desktop app.
- **One running FreeCal per user session, whatever the data directory**: stricter than needed, and it would block test and development builds next to the real FreeCal.
- **SQLite's exclusive locking mode, or a PID file, instead of an OS file lock**: SQLite's mode only blocks others once the right lock is taken, which is hard to reason about. A PID file can be left behind after a crash. An OS file lock is released by the OS when the process dies, and Rust's `File::try_lock` works on Linux, Windows and macOS.

## Consequences

- "Files to open" is one input with two sources: forwarded command-line arguments on Linux and Windows, and the system's open-file events (`RunEvent::Opened`) on macOS, where the OS itself keeps one app running.
- When a second launch can't hand off (a race before the hand-off is in place), it fails with `LocalStoreInUse`, which is shown on the startup error screen like any other startup failure.
