//! The Tauri shell: exposes the core application interface to the frontend as
//! commands and forwards the core's Signals as Tauri events.

use std::sync::Arc;

use freecal_core::{Account, Core, Signal, SignalSink, SystemClock};
use tauri::{AppHandle, Emitter, Manager, State};

/// The Tauri event on which Signals reach the frontend.
const SIGNAL_EVENT: &str = "freecal://signal";

struct ForwardToFrontend(AppHandle);

impl SignalSink for ForwardToFrontend {
    fn send(&self, signal: Signal) {
        // Signals are fire-and-forget: with no window open they are dropped.
        let _ = self.0.emit(SIGNAL_EVENT, signal);
    }
}

type CommandResult<T> = Result<T, String>;

#[tauri::command]
fn today(core: State<Core>) -> String {
    core.today().to_string()
}

#[tauri::command]
fn list_accounts(core: State<Core>) -> CommandResult<Vec<Account>> {
    core.list_accounts().map_err(|error| error.to_string())
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // The XDG data directory, e.g. ~/.local/share/org.freecal.FreeCal (ADR 0004).
            let data_dir = app.path().app_data_dir()?;
            let signals = Arc::new(ForwardToFrontend(app.handle().clone()));
            let core = Core::open(&data_dir, Arc::new(SystemClock), signals)?;
            app.manage(core);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![today, list_accounts])
        .run(tauri::generate_context!())
        .expect("FreeCal failed to start");
}
